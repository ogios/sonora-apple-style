//! The Windows and macOS hook that lets [`crate::AppSettings::window_rounding`] reach the real
//! window. The `DwmSetWindowAttribute` and AppKit toolbar calls are platform code only `sonora`
//! links against, so it installs the closure here once at startup. `views` calls
//! [`apply_window_rounding`] from `Root`'s render whenever the setting changes, so neither crate
//! depends on the other's platform bindings. Linux and FreeBSD round their own chrome in `views`
//! instead, since there it is ordinary element styling rather than a platform call.

use gpui::{App, Global, Window};
use ui::Rounding;

type Apply = Box<dyn Fn(&Window, Rounding)>;

struct RoundedWindowHook(Apply);

impl Global for RoundedWindowHook {}

/// Registers the platform-specific corner-rounding hook. A no-op until this is called, so
/// Linux and FreeBSD simply never install one.
pub fn install_rounded_window_hook(apply: impl Fn(&Window, Rounding) + 'static, cx: &mut App) {
    cx.set_global(RoundedWindowHook(Box::new(apply)));
}

/// Applies the window-rounding preference to the live window, if a hook is installed.
pub fn apply_window_rounding(window: &Window, rounding: Rounding, cx: &App) {
    if let Some(hook) = cx.try_global::<RoundedWindowHook>() {
        (hook.0)(window, rounding);
    }
}
