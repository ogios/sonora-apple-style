# sonora

A native music streaming client, built with Rust and [GPUI](https://github.com/zed-industries/zed)
(Zed's UI framework). Cargo workspace, edition 2024, resolver 3.

## Crate layout

```
crates/
  sonora/     GUI binary: main, window, actions, asset registry, HTTP client shim, tray, dock
  views/      screens, shells (Workspace, FullscreenView) and app chrome
  service/    sonora-server binary and authoritative music state, without GUI dependencies
  service-client/ GPUI read models and IPC client, built from the shared service model sources
  service-runtime/ ordered service entities, subscriptions and tasks on Tokio
  locale/     localization for processes without GPUI, sharing the i18n sources
  protocol/   versioned local IPC framing, commands and ordered subscriptions
  cli/        sonora-cli terminal client over protocol, with no GPUI dependency
  music/      provider traits and models in the root, one submodule per provider (no GPUI)
  storage/    the state.sqlite and cache.sqlite paths, schema and connection setup
  ui/         design system: theme, metrics, layout ladder and reusable elements
  router/     Destination enum, navigation history, Link trait
  input/      text input element, global actions and keybindings
  i18n/       Fluent localization: the `t!` macro, locale selection, embedded .ftl
  icons/      the icon packs: registry, active pack, path resolution, AssetSource
  embed/      build-script helper that walks a folder and writes include_bytes! literals
  webview/    a native browser window over a throwaway session, for cookie sign-ins and PO tokens
  widevine/   the Widevine CDM host, pssh and CENC fMP4 parsing, for any DRM'd provider
```

Dependency direction is strict. Do not create a back edge:

```
sonora → views → service-client → music
         service → music, protocol, service-runtime, locale
sonora, service-client → protocol
         cli → protocol, i18n (without GPUI)
         service, music → storage
         service-client → webview
         music → widevine
         all ui-side crates → ui, router, input → ui → gpui
         every ui-side crate → i18n, icons → gpui
```

- Only `service/src/bootstrap.rs` constructs concrete music and lyrics providers. Views and the
  rest of the service use the traits and models in the root of `music`.
- `sonora` connects to `sonora-server` through `protocol`; it never starts an embedded music
  service. Server-owned entities in `service-runtime` manage providers, playback, queues, lyrics
  and persistence. The server and its transitive dependencies must not include GPUI, windowing,
  fonts, icons, rendering or native WebView code.
  GUI entities mirror presentation data and forward commands. Their constructors must not open
  databases, create playback engines, restore accounts or start provider tasks.
- Native browser windows for sign-in and PO tokens belong to the GUI. The server sends browser
  requests over `protocol` and accepts acknowledged results only from the assigned connection.
  Browser credentials are excluded from snapshots and logs. Presentation preferences are
  persisted as data by the server; only the GUI loads themes, resolves fonts and applies effects.
- Linux MPRIS, Discord, download jobs and audio analysis belong to the server. GUI subscriptions
  are released on page drop or disconnect. Accepted mutations are never replayed on reconnect.
  The service never depends on `sonora` or `views`.
- `music` never depends on `gpui`. `ui` never knows about `music`, `service` or playback. Widgets
  that need app state live in `views/src/chrome/`.
- `music::engine` is the one playback engine and `music::stream` the one progressive download. A
  provider implements `engine::Fetch` and `stream::Body` rather than writing its own. The engine
  applies loudness normalisation, so a provider only reports a track's level through
  `Fetch::loudness`. Spotify is the one exception, because librespot decodes and normalises
  inside its own player.
- `music::drm` is the only way `service` and `views` reach protected playback. The CDM is never
  shipped in any artefact or package.

## Rules

- **Reuse before you build.** Check `crates/ui/src/lib.rs`, `crates/views/src/shared/` and
  `crates/views/src/chrome/` first, and extend an element with a builder method rather than
  writing a sibling. New elements copy the shape of `ui/src/button.rs`, including the
  `mem::take` and `refine` of caller styles.
- **Never call the Spotify Web API.** No `reqwest` to `api.spotify.com`, no client secret, no
  `rspotify`. Spotify data comes from librespot's `spclient`, and auth uses Spotify's own client id
  in `auth.rs`.
- **Never hardcode a color, radius or size.** Read them from `cx.theme()` and `theme.metrics`. A
  literal `px(…)` is allowed only for a local, non-scaling detail declared as a module `const`. A
  new color token goes into `Theme`, every `Theme::*()` constructor, `ThemeOverrides` and
  `apply_color!`.
- **Never write a bare breakpoint.** Use the `ui::Room` ladder, and classify width with
  `Chrome::room` or `Chrome::content`, never the raw viewport. A view whose layout depends on
  width observes `Chrome::entity(cx)`.
- **Every user-facing string comes from Fluent.** Add the key to `assets/i18n/en-US/main.ftl`. Other
  locales may lag. Never call `t!` in a constructor: store the key and resolve it in `render`.
  Counts use Fluent selectors. Log messages and `.context(…)` stay in English.
- **Network work runs on tokio.** Anything touching `MusicApi`, librespot or sockets goes inside
  `Io::global(cx).spawn`, awaited from `cx.spawn` and applied in `this.update` ending with
  `cx.notify()`. Store the returned `Task` in a field and never detach a data load. Network-backed
  features belong in a `service` entity, not a view. An entity that caches provider data clears on
  `SessionEvent::SignedOut`.
- **Never drive a player from a view.** Go through `service::Playback`.
- **Assets are picked up from their folder.** Drop an icon into `assets/icons/<pack>/` or a face
  into `assets/fonts/`. Call sites spell `"icons/<name>.svg"` and resolve it with `icons::path` at
  render.
- **Generated files are regenerated, never edited.** App icons come from
  `scripts/generate-icons.py`, `THIRD-PARTY.md` from `scripts/generate-notices.py`, the README
  translation table from `scripts/i18n-coverage.py`.
- **Do not add tests unless asked.** Offering is fine. Existing tests are `#[cfg(test)] mod tests`
  at the bottom of the file they cover.

## Building and checks

```sh
nix develop                    # or direnv allow
./run.sh                       # build and supervise server + GUI; Ctrl+C stops both
# or start sonora-server and sonora separately in two terminals
cargo fmt
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
```

The devShell has no `cargo` or `cargo-clippy`; those come from the system profile. The flake's
default package is the released binary, so `nix build` never sees the working tree, while
`nix build .#sonora-src` compiles it.
`.cargo/config.toml` links with mold on `x86_64-unknown-linux-gnu`, so mold must be on PATH, or
build with `RUSTFLAGS=""`. `crates/widevine` compiles C++, so a C++ compiler is needed too.

Clippy is clean, `--all-targets` included, and stays that way. Boxed callback fields get a
module-local `type` alias rather than a `type_complexity` warning. The one `#[allow]` is
`reversed_empty_ranges` on the `clamp_range("abc", &(2..1))` test in `crates/ui/src/input/mod.rs`.

## Code style

- Every type and every function whose behaviour is not obvious from its name carries a `///`
  comment saying what it is for and what a caller has to know. One or two plain sentences.
- `use gpui::prelude::*;` then explicit imports. Traits are imported anonymously
  (`use ui::ActiveTheme as _;`).
- Module order: `use` block, `const`s, types, impls, private free helpers at the bottom.
- In render, prefer `.when()`, `.when_some()` and `.map()` over branching. Two-arm boolean choices
  use `match flag { true => …, false => … }`. Use let-else and return early.
- `anyhow::Result` at boundaries with lowercase `.context("cannot …")`. Logs are prefixed by
  subsystem (`"playback: …"`).
- Dependencies go in the root `[workspace.dependencies]`. `gpui` and `gpui_platform` are pinned to
  one git rev and move together.

## Commits

Conventional Commits: `type(scope): description`, imperative, lowercase, no trailing period, no
body. Scopes in use: `views`, `ui`, `music`, `state`, `playback`, `player`, `settings`, `router`,
`local`, `sonora`, `nix`.

## Pull requests

One heading, one list, nothing else:

```markdown
## Summary

- add playlist create, rename, delete, visibility, and track mutation support
- add album and playlist queue actions
```

Bullets are lowercase, imperative, one line each, and describe behaviour rather than files.

## Changelog

`CHANGELOG.md` follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Add to
`## [Unreleased]` as features land, under `Added`, `Changed` or `Fixed`. Entries say what someone
using Sonora can now do, in one or two short sentences, since the notes are posted to Discord.
Leave out work no user can observe. Cutting a release is the `release-sonora` skill.
