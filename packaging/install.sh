#!/usr/bin/env bash
set -euo pipefail

if [[ $# != 1 || ! -f $1 ]]; then
  printf 'Usage: %s <Unzeit .pkg.tar.zst, .rpm or .flatpak file>\n' "$0" >&2
  exit 1
fi

package=$(realpath -- "$1")
case "$package" in
  *.pkg.tar.zst) manager=pacman; command=(pacman -U "$package") ;;
  *.rpm) manager=dnf; command=(dnf install "$package") ;;
  *.flatpak) manager=flatpak; command=(flatpak install --user "$package") ;;
  *) printf 'Unsupported package: %s\n' "$package" >&2; exit 1 ;;
esac

if ! command -v "$manager" >/dev/null; then
  printf 'This package requires %s. Choose the package for your distribution.\n' "$manager" >&2
  exit 1
fi

if [[ $manager != flatpak && $EUID != 0 ]]; then
  if ! command -v sudo >/dev/null; then
    printf 'Run this script as an administrator to install with %s.\n' "$manager" >&2
    exit 1
  fi
  command=(sudo "${command[@]}")
fi

printf 'The package manager will show the changes and ask for confirmation.\n'
exec "${command[@]}"
