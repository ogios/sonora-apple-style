//! GPUI adaptation for the shared presentation models.

pub use gpui::*;

/// Creates a presentation entity through the shared service context vocabulary.
pub(crate) trait StateContext {
    fn create<T: 'static>(&mut self, build: impl FnOnce(&mut Context<T>) -> T) -> Entity<T>;
}

impl StateContext for App {
    fn create<T: 'static>(&mut self, build: impl FnOnce(&mut Context<T>) -> T) -> Entity<T> {
        AppContext::new(self, build)
    }
}
