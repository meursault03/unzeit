#!/usr/bin/env python3
"""Stage local source packages without copying videos, models or build products."""
import argparse
import base64
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tomllib
import xml.etree.ElementTree as ET

REPO = Path(__file__).resolve().parent.parent
INPUTS = (
    "src", "public", "scripts", "tests", "packaging", "src-tauri",
    "LICENSE", "icon.svg", "package.json", "package-lock.json", "tsconfig.json",
    "vite.config.ts", "index.html",
)
IGNORED = shutil.ignore_patterns("target", "gen", "node_modules", "pdfjs", "__pycache__",
                                "*.mp4", "*.wav", "*.bin", "*.gguf", "*.sqlite*")


def run(*args, cwd):
    subprocess.run(args, cwd=cwd, check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("format", choices=("arch", "fedora", "flatpak"))
    parser.add_argument("--output", type=Path, help="new output directory (default: artifacts/<format>)")
    parser.add_argument("--offline", action="store_true", help="use cached dependencies only")
    args = parser.parse_args()
    version = json.loads((REPO / "package.json").read_text())["version"]
    cargo = tomllib.loads((REPO / "src-tauri/Cargo.toml").read_text())
    config = json.loads((REPO / "src-tauri/tauri.conf.json").read_text())
    if cargo["package"]["version"] != version or config["version"] != version:
        parser.error("package.json, Cargo.toml and tauri.conf.json must use the same version")
    output = (args.output or REPO / "artifacts" / args.format).resolve()
    if output == REPO or REPO.is_relative_to(output) or any(
        output.is_relative_to(REPO / name) for name in INPUTS
    ):
        parser.error("output must be outside the source directories")
    output.mkdir(parents=True, exist_ok=False)
    source = output / "source"
    source.mkdir()
    for name in INPUTS:
        path = REPO / name
        if path.is_dir():
            shutil.copytree(path, source / name, ignore=IGNORED)
        else:
            shutil.copy2(path, source / name)
    epoch = subprocess.check_output(
        ["git", "log", "-1", "--format=%ct"], cwd=REPO, text=True
    ).strip()
    metainfo = source / "packaging/local.unzeit.desktop.metainfo.xml"
    component = ET.parse(metainfo)
    releases = ET.SubElement(component.getroot(), "releases")
    ET.SubElement(releases, "release", version=version,
                  date=datetime.fromtimestamp(int(epoch), timezone.utc).date().isoformat())
    ET.indent(component, space="  ")
    component.write(metainfo, encoding="UTF-8", xml_declaration=True)

    if args.format == "arch":
        archive = output / f"unzeit-{version}.tar.gz"
        with tarfile.open(archive, "w:gz") as tar:
            tar.add(source, arcname="unzeit")
        with archive.open("rb") as handle:
            checksum = hashlib.file_digest(handle, "sha256").hexdigest()
        template = (REPO / "packaging/arch/PKGBUILD.in").read_text()
        (output / "PKGBUILD").write_text(template.replace("@VERSION@", version)
                                       .replace("@SHA256@", checksum).replace("@EPOCH@", epoch))
        shutil.rmtree(source)
    elif args.format == "fedora":
        shutil.copy2(REPO / "packaging/fedora/Containerfile", output / "Containerfile")
    else:
        # Populate a portable npm cache; scripts run only during the SDK build.
        npm = ["npm", "ci", "--ignore-scripts", "--cache", "npm-cache", "--no-audit", "--no-fund"]
        if args.offline:
            # npm can load tarballs by lockfile integrity without its URL index.
            cache = Path(subprocess.check_output(["npm", "config", "get", "cache"], text=True).strip())
            packages = json.loads((source / "package-lock.json").read_text())["packages"]
            for package in packages.values():
                integrity = package.get("integrity")
                if not integrity:
                    continue
                algorithm, digest = integrity.split()[0].split("-", 1)
                digest = base64.b64decode(digest).hex()
                relative = Path("_cacache/content-v2") / algorithm / digest[:2] / digest[2:4] / digest[4:]
                if (cache / relative).is_file():
                    target = source / "npm-cache" / relative
                    target.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copy2(cache / relative, target)
        run(*npm, *(["--offline"] if args.offline else []), cwd=source)
        shutil.rmtree(source / "node_modules")
        command = ["cargo", "vendor", "--locked", "--versioned-dirs", "--manifest-path",
                   "src-tauri/Cargo.toml", "cargo/vendor"]
        if args.offline:
            command.insert(2, "--offline")
        with (output / "cargo-vendor.log").open("w") as log:
            configuration = subprocess.check_output(command, cwd=source, text=True, stderr=log)
        # Cargo's --quiet suppresses the configuration as well as its progress output.
        if not tomllib.loads(configuration).get("source"):
            sys.exit("Cargo did not produce a vendor configuration; see cargo-vendor.log.")
        (source / ".cargo").mkdir()
        # Relative to the repository root in the SDK, not this staging directory.
        (source / ".cargo/config.toml").write_text(
            configuration.replace(str(source / "cargo/vendor"), "cargo/vendor")
        )
        manifest = json.loads((REPO / "packaging/flatpak/local.unzeit.desktop.json").read_text())
        manifest["build-options"]["env"]["SOURCE_DATE_EPOCH"] = epoch
        (output / "local.unzeit.desktop.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(output)


if __name__ == "__main__":
    try:
        main()
    except (OSError, subprocess.CalledProcessError) as error:
        sys.exit(f"Source preparation failed: {error}. Use a new --output directory to retry.")
