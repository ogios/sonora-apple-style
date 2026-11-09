use gpui::{App, ElementId, PathPromptOptions};
use i18n::t;
use state::Sonora;
use ui::{Button, Vacancy};

pub(crate) fn choose_button(id: impl Into<ElementId>) -> Button {
    Button::new(id)
        .label(t!("settings-choose-folder"))
        .on_click(|_, _, cx| choose_folder(cx))
}

pub(crate) fn unconfigured(id: impl Into<ElementId>) -> Vacancy {
    Vacancy::new(t!("library-local-unconfigured"))
        .icon("icons/folder-plus.svg")
        .action(choose_button(id).outline())
}

pub(crate) fn choose_folder(cx: &mut App) {
    let receiver = cx.prompt_for_paths(PathPromptOptions {
        files: false,
        directories: true,
        multiple: true,
        prompt: None,
    });
    let library = Sonora::global(cx).library.clone();
    cx.spawn(async move |cx| {
        let Ok(Ok(Some(paths))) = receiver.await else {
            return;
        };
        if paths.is_empty() {
            return;
        }
        library.update(cx, |library, cx| library.add_local_folders(paths, cx));
    })
    .detach();
}
