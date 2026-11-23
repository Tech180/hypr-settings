//! `AdwApplication` with single-instance semantics: a second launch forwards its
//! request to the running window through application actions and exits.

use std::cell::RefCell;
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::Arc;

use adw::prelude::*;
use gtk4::{gio, glib};
use libadwaita as adw;
use settings_application::SettingsApi;

use super::interfaces::Launch;
use crate::features::settings_shell::ShellWindow;
use crate::style;

const APP_ID: &str = "dev.tech180.HyprSettings";

type Shell = Rc<RefCell<Option<ShellWindow>>>;

pub fn run(api: Arc<dyn SettingsApi>, launch: Launch) -> ExitCode {
    super::log_filter::install();
    let app = adw::Application::builder().application_id(APP_ID).build();
    let shell = Shell::default();
    install_actions(&app, &api, &shell);
    app.connect_startup(|_| style::install());

    if let Err(err) = app.register(gio::Cancellable::NONE) {
        tracing::warn!("could not register on the session bus: {err}");
    } else if app.is_remote() {
        forward(&app, &launch);
        return ExitCode::SUCCESS;
    }

    let section = match launch {
        Launch::Open { section } => section,
        Launch::Toggle => None,
        Launch::Close | Launch::Refresh => return ExitCode::SUCCESS,
    };

    let first_section = RefCell::new(section);
    {
        let (api, shell) = (Arc::clone(&api), Rc::clone(&shell));
        app.connect_activate(move |app| open(app, &api, &shell, first_section.take()));
    }
    ExitCode::from(app.run_with_args::<&str>(&[]).get())
}

fn forward(app: &adw::Application, launch: &Launch) {
    let (action, parameter) = match launch {
        Launch::Open { section } => (
            "open-section",
            Some(section.clone().unwrap_or_default().to_variant()),
        ),
        Launch::Toggle => ("toggle", None),
        Launch::Close => ("close", None),
        Launch::Refresh => ("refresh", None),
    };
    app.activate_action(action, parameter.as_ref());
    if let Some(connection) = app.dbus_connection() {
        let _ = connection.flush_sync(gio::Cancellable::NONE);
    }
}

fn open(
    app: &adw::Application,
    api: &Arc<dyn SettingsApi>,
    shell: &Shell,
    section: Option<String>,
) {
    let existing = shell.borrow().clone();
    let window = existing.unwrap_or_else(|| {
        let window = ShellWindow::new(app, Arc::clone(api));
        *shell.borrow_mut() = Some(window.clone());
        window
    });
    if let Some(section) = section.filter(|s| !s.is_empty()) {
        window.select(&section);
    }
    window.present();
}

fn install_actions(app: &adw::Application, api: &Arc<dyn SettingsApi>, shell: &Shell) {
    let open_section = gio::SimpleAction::new("open-section", Some(glib::VariantTy::STRING));
    {
        let (app, api, shell) = (app.clone(), Arc::clone(api), Rc::clone(shell));
        open_section.connect_activate(move |_, parameter| {
            let section = parameter.and_then(|p| p.get::<String>());
            open(&app, &api, &shell, section);
        });
    }

    let toggle = gio::SimpleAction::new("toggle", None);
    {
        let (app, api, shell) = (app.clone(), Arc::clone(api), Rc::clone(shell));
        toggle.connect_activate(move |_, _| {
            let existing = shell.borrow().clone();
            match existing {
                Some(window) if window.is_active() => window.close(),
                _ => open(&app, &api, &shell, None),
            }
        });
    }

    let close = gio::SimpleAction::new("close", None);
    {
        let shell = Rc::clone(shell);
        close.connect_activate(move |_, _| {
            let existing = shell.borrow().clone();
            if let Some(window) = existing {
                window.close();
            }
        });
    }

    let refresh = gio::SimpleAction::new("refresh", None);
    {
        let shell = Rc::clone(shell);
        refresh.connect_activate(move |_, _| {
            let existing = shell.borrow().clone();
            if let Some(window) = existing {
                window.refresh();
            }
        });
    }

    for action in [&open_section, &toggle, &close, &refresh] {
        app.add_action(action);
    }
}
