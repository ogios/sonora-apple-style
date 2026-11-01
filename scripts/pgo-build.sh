#!/usr/bin/env bash
# Build, train, merge and use a profile without changing the default Cargo target directory.
set -euo pipefail
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
phase=${1:?usage: scripts/pgo-build.sh generate|train|merge|use /absolute/output [app arguments]}
out=${2:?provide a fresh absolute output directory}
shift 2
[[ $out = /* && $out != *' '* ]] || { echo 'use an absolute output path without spaces' >&2; exit 2; }
cd "$repo"
target=${PGO_TARGET:-$(rustc -vV | sed -n 's/^host: //p')}
flags=${PGO_RUSTFLAGS:--C link-arg=-fuse-ld=mold}
profdata=${LLVM_PROFDATA:-$(rustc --print sysroot)/lib/rustlib/$target/bin/llvm-profdata}
# Hash the actual build inputs so documentation commits do not invalidate a trained profile.
fingerprint() {
    { rustc -vV; cargo -V; printf '%s\n' "$target" "$flags"; git ls-files -z --cached --others --exclude-standard -- Cargo.toml Cargo.lock .cargo crates vendor assets | LC_ALL=C sort -zu | xargs -0 sha256sum; } | sha256sum
}
check_inputs() {
    fingerprint > "$out/current-inputs"
    cmp "$out/inputs" "$out/current-inputs" || { echo 'build inputs changed; generate a fresh profile' >&2; exit 1; }
}
build() {
    CARGO_TARGET_DIR="$out/target" RUSTFLAGS="$flags $1" cargo build --release --locked --target "$target" -j "${CARGO_BUILD_JOBS:-8}"
}
case "$phase" in
    generate)
        [[ ! -e $out ]] || { echo 'generate requires a new output directory' >&2; exit 1; }
        mkdir -p "$out/profiles"
        fingerprint > "$out/inputs"
        git rev-parse HEAD > "$out/source-commit"
        build "-Cprofile-generate=$out/profiles"
        cp "$out/target/$target/release/sonora" "$out/sonora-generate"
        ;;
    train)
        check_inputs
        [[ ! -e $out/merged.profdata ]] || { echo 'profile is already frozen; start a fresh run to retrain' >&2; exit 1; }
        exec env LLVM_PROFILE_FILE="$out/profiles/training-%p-%m.profraw" "$out/sonora-generate" "$@"
        ;;
    merge)
        check_inputs
        [[ ! -e $out/merged.profdata ]] || { echo 'profile is already frozen' >&2; exit 1; }
        "$profdata" merge -o "$out/merged.profdata" "$out/profiles"
        "$profdata" show "$out/merged.profdata" > "$out/profile-summary"
        sha256sum "$out/merged.profdata" > "$out/profile-sha256"
        ;;
    use)
        check_inputs
        sha256sum --check "$out/profile-sha256"
        build "-Cprofile-use=$out/merged.profdata -Cllvm-args=-pgo-warn-missing-function"
        cp "$out/target/$target/release/sonora" "$out/sonora-pgo"
        ;;
    *) echo 'expected generate, train, merge or use' >&2; exit 2 ;;
esac
