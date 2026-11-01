# Optional profile-guided Linux builds

The normal `cargo build --release --locked` remains the default build. PGO is an
explicit, local workflow: instrument a release binary, exercise representative
workloads, merge its counters, then build a separate optimized executable.
See the [rustc PGO guide](https://doc.rust-lang.org/rustc/profile-guided-optimization.html).

Run from the repository root with the usual native build dependencies and a
compatible `llvm-profdata`. `rustup component add llvm-tools-preview` supplies
that tool for the active Rust toolchain. If it lives elsewhere, set
`LLVM_PROFDATA=/absolute/path/to/llvm-profdata` for the merge command.
The script defaults to the host target and the Linux mold linker flags used by
this workspace. `PGO_TARGET` and `PGO_RUSTFLAGS` override these values; keep them
identical throughout a training run. Cargo features and release settings come
from the workspace, as in the default release build.

```bash
pgo_out="$PWD/target/local-pgo-$(date +%Y%m%d-%H%M%S)"
scripts/pgo-build.sh generate "$pgo_out"
# Quit the running Sonora before starting an instrumented copy.
scripts/pgo-build.sh train "$pgo_out"
# Quit normally with Ctrl+Q so LLVM flushes the counters, then train again.
scripts/pgo-build.sh train "$pgo_out"
scripts/pgo-build.sh merge "$pgo_out"
scripts/pgo-build.sh use "$pgo_out"
"$pgo_out/sonora-pgo"
```

Training should cover the UI and playback paths used in daily work. For the
NetEase Vulkan client, use this sequence on the same displays and scales used
for normal playback:

1. Start paused; open a large Chinese playlist and scroll its tracks.
2. Browse an album and artists, search in Chinese, and switch between Home and
   the playlist fifteen times.
3. Resume playback, open fullscreen lyrics, and allow lyrics and spectrum to
   animate for at least twenty seconds; pause and leave fullscreen.
4. On the 4K display, open an artist's full releases grid and scroll from top
   to bottom and back six times. Return Home and wait thirty seconds.
5. In a second process, repeat browsing and play a Chinese song with lyrics
   and pinyin visible for at least twenty seconds. Quit normally.

Save the playback position, queue and window layout before training and restore
them afterward. The training command runs the real application with the normal
account and library; it does not restore state or drive the desktop itself.
Instrumentation overhead is substantial, so measure the final PGO executable,
not the training executable. Compare repeated new processes including cold
starts, memory peaks, frame tails and screenshots before adopting a new profile.

Each output directory is a single training session. Generation refuses an
existing directory, and merging freezes the profile. The script checks a hash
of Rust/Cargo versions, target, flags and tracked or untracked workspace build
inputs before training, merging and using counters. It also verifies the merged
profile's checksum before compilation. Source, compiler or flag changes require
a fresh output directory and new training; documentation-only changes do not.
Changing native tools, libraries, Cargo profile environment overrides or display
and driver settings also calls for fresh training and validation.

Artifacts live under the chosen output directory: `sonora-generate`, `profiles/`,
`merged.profdata`, provenance files, a separate Cargo target directory, and
`sonora-pgo`. Profiles are machine/workload/toolchain dependent and are not
checked into the repository. Keep them with the matching source revision if a
specific binary must be reproduced.

To try the result, quit the running instance and launch `sonora-pgo` directly.
An existing launcher can point to that explicit path once the build has been
validated. The script does not install an executable, change desktop launchers,
or replace `target/release/sonora`. Launch the original executable again to
switch back. Normal Cargo release builds continue to work independently.
