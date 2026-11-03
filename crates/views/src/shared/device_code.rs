use gpui::prelude::*;
use gpui::{App, ClipboardItem, FontWeight, SharedString, div};
use i18n::t;
use service::{Outcome, Toasts};
use ui::ActiveTheme as _;
use ui::{Button, Qr, Text};

/// The code a device sign-in asks the listener to type on the provider's site, centred in
/// whatever holds it, with a button that copies it and says so in a toast. The login screen
/// and the settings dialog both show it.
pub(crate) fn device_code(code: String, cx: &App) -> impl IntoElement {
    let theme = *cx.theme();
    div()
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .w_full()
        .child(
            div()
                .text_size(theme.text(Text::Title))
                .font_weight(FontWeight::BOLD)
                .child(SharedString::from(code.clone())),
        )
        .child(
            Button::new("copy-code")
                .icon("icons/copy.svg")
                .ghost()
                .small()
                .on_click(move |_, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(code.clone()));
                    Toasts::show(Outcome::Done, "toast-code-copied", cx);
                }),
        )
}

/// The native scan-to-authorize prompt shared by login and account settings.
pub(crate) fn qr_code(url: String, scanned: bool, cx: &App) -> impl IntoElement {
    let theme = *cx.theme();
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap_2()
        .child(
            Qr::new(url)
                .size(theme.metrics.cover * 1.5)
                .rounded(theme.radius),
        )
        .child(
            div()
                .text_size(theme.text(Text::Small))
                .text_color(theme.muted_foreground)
                .child(match scanned {
                    true => t!("login-qr-confirm"),
                    false => t!("login-qr-scan"),
                }),
        )
}
