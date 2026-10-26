#!/usr/bin/env bash
# Keep makepkg output here regardless of the caller's working directory or config.
set -euo pipefail

cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
packaging_dir=$(pwd -P)
export BUILDDIR="${packaging_dir}/.build"
export PKGDEST="${packaging_dir}/.build/packages"
export SRCDEST="${packaging_dir}/.build/sources"
export SRCPKGDEST="${packaging_dir}/.build/source-packages"
export LOGDEST="${packaging_dir}/.build/logs"

mkdir -p -- "$BUILDDIR" "$PKGDEST" "$SRCDEST" "$SRCPKGDEST" "$LOGDEST"
exec makepkg "$@"
