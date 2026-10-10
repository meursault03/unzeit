"""Check installer routing without invoking a real package manager."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

installer = Path(__file__).with_name("install.sh")
with tempfile.TemporaryDirectory(prefix="unzeit-installer-") as temporary:
    root = Path(temporary)
    tools = root / "bin"
    tools.mkdir()
    executable = "#!/usr/bin/python3\nimport json, sys\nprint(json.dumps(sys.argv))\n"
    for name in ("pacman", "dnf", "flatpak", "sudo"):
        tool = tools / name
        tool.write_text(executable)
        tool.chmod(0o755)
    (tools / "realpath").symlink_to("/usr/bin/realpath")
    environment = dict(os.environ, PATH=str(tools))
    for suffix, manager, arguments in (
        (".pkg.tar.zst", "pacman", ["-U"]),
        (".rpm", "dnf", ["install"]),
        (".flatpak", "flatpak", ["install", "--user"]),
    ):
        package = root / ("Unzeit course" + suffix)
        package.touch()
        result = subprocess.run(["/usr/bin/bash", str(installer), str(package)],
                                env=environment, text=True, capture_output=True, check=True)
        called = json.loads(result.stdout.splitlines()[-1])
        expected = [manager, *arguments, str(package)]
        assert called[1:] == (expected if manager != "flatpak" and os.geteuid() != 0
                             else expected[1:]), called
    package = root / "Unzeit.txt"
    package.touch()
    for arguments in ([], [str(root / "missing.rpm")], [str(package)], [str(package), "extra"]):
        result = subprocess.run(["/usr/bin/bash", str(installer), *arguments],
                                env=environment, text=True, capture_output=True)
        assert result.returncode != 0
    (tools / "flatpak").unlink()
    result = subprocess.run(["/usr/bin/bash", str(installer), str(root / "Unzeit course.flatpak")],
                            env=environment, text=True, capture_output=True)
    assert result.returncode != 0 and "requires flatpak" in result.stderr
print("Installer routing and invalid-input checks passed.")
