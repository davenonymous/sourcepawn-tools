#!/bin/sh
# Copyright 2026 davenonymous
#
# Runs the tests and builds pawncritic and pawntidy for Linux and Windows
# (x86_64), writing dist/ anew:
#   dist/linux-x86_64/pawncritic, dist/linux-x86_64/pawntidy
#   dist/windows-x86_64/pawncritic.exe, dist/windows-x86_64/pawntidy.exe
#
# The Windows build needs the Rust target and the MinGW-w64 C compiler:
#   rustup target add x86_64-pc-windows-gnu
#   apt install gcc-mingw-w64-x86-64
set -eu
here="$(cd "$(dirname "$0")" && pwd)"
cd "${here}"

if ! rustup target list --installed | grep -qx x86_64-pc-windows-gnu; then
	echo "the Rust target x86_64-pc-windows-gnu is missing: rustup target add x86_64-pc-windows-gnu" >&2
	exit 1
fi
if ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
	echo "the MinGW-w64 C compiler is missing: apt install gcc-mingw-w64-x86-64" >&2
	exit 1
fi

cargo test --workspace --quiet
cargo build --workspace --release --quiet
cargo build --workspace --release --quiet --target x86_64-pc-windows-gnu

rm -rf "${here}/dist"
mkdir -p "${here}/dist/linux-x86_64" "${here}/dist/windows-x86_64"
for tool in pawncritic pawntidy; do
	cp "target/release/${tool}" "${here}/dist/linux-x86_64/"
	cp "target/x86_64-pc-windows-gnu/release/${tool}.exe" "${here}/dist/windows-x86_64/"
done
echo
echo "dist/ holds:"
(cd "${here}/dist" && find . -type f | sort | sed 's|^\./|  |')
