# Arch Linux local package

Adapted from the [AUR sonora PKGBUILD](https://aur.archlinux.org/cgit/aur.git/tree/PKGBUILD?h=sonora),
commit `e70a504972140b494d2484c74a3b8af552ba183a` (version 0.44.1).

This recipe compiles this checkout, including uncommitted and untracked source changes.
It never downloads an upstream Sonora release or resets the checkout. Keep it inside this
repository; it is not a standalone AUR recipe. Do not edit sources during a build.

Install Arch's build tools once:

```sh
sudo pacman -S --needed base-devel rust git
```

From the repository root, build and install (run as your regular user):

```sh
./packaging/arch/build.sh -fsi
```

`-s` installs missing build/runtime dependencies through pacman, `-i` installs the resulting
package, and `-f` permits rebuilding the same version after local edits. To build without
installing the resulting package, use `-fs`. All makepkg options are forwarded; add `-L`
to save build logs.

The package is named `sonora-local` and provides `sonora`. It conflicts with other packages
providing Sonora (including `sonora` and `sonora-bin`); pacman will ask about replacing an
installed version. The executable and desktop entry remain `sonora`. Existing Sonora settings
and data are reused.

The version is `r<commit-count>.g<12-character-commit-hash>`, with `.dirty` appended when Git
reports uncommitted changes (including untracked files, excluding ignored build output).
For example: `r123.g0123456789ab.dirty`. Different edits on the same commit share a version;
`-f` still rebuilds them. Use a full Git checkout for meaningful commit counts; counts can
change after rebasing or switching branches. The recipe never rewrites itself to update its
version.

All build output is ignored by Git and stays below `packaging/arch/.build/`:

| Path | Contents |
| --- | --- |
| `sonora-local/src/`, `sonora-local/pkg/` | makepkg working and staging directories |
| `target/` | Cargo compilation output, separate from the checkout's `target/` |
| `packages/` | Installable `.pkg.tar.zst` packages |
| `logs/` | Logs when `-L` is used |
| `sources/`, `source-packages/` | makepkg source downloads and source archives, if requested |

Use `build.sh` to enforce these paths even if your makepkg configuration sets other output
directories. Cargo's usual shared download cache and Rust toolchains remain in your user
directories. The build uses the pinned `rust-toolchain.toml` when rustup is active; otherwise
the installed Rust compiler must support this checkout. Rust code is built with
`-C target-cpu=native`, retaining the workspace's ThinLTO and single codegen unit. The package
is intended for the build machine; another CPU may lack instructions used by the executable.
The mold linker is installed as a build dependency to shorten linking time. No Widevine CDM
is bundled.

Browser sign-in (including NetEase) needs `webkit2gtk-4.1`. Install `pipewire-alsa` or
`pulseaudio-alsa` for your sound server and the appropriate Vulkan driver for your GPU.

The entire `.build/` directory is disposable after building. Removing it also discards the
separate Cargo compilation cache and any saved packages. This recipe intentionally reads
the local tree directly; `makepkg --source` does not produce a self-contained source release.
