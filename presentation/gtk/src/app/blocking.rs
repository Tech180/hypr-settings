//! Runs a blocking `SettingsApi` call off the main thread and delivers the result back on it.

use std::sync::Arc;

use gtk4::{gio, glib};
use settings_application::SettingsApi;

pub fn call<T: Send + 'static>(
    api: &Arc<dyn SettingsApi>,
    work: impl FnOnce(&dyn SettingsApi) -> T + Send + 'static,
    done: impl FnOnce(T) + 'static,
) {
    let api = Arc::clone(api);
    glib::spawn_future_local(async move {
        match gio::spawn_blocking(move || work(api.as_ref())).await {
            Ok(value) => done(value),
            Err(_) => tracing::error!("display worker panicked"),
        }
    });
}
