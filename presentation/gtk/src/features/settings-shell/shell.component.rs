//! The settings window: sidebar with search, section stack, toasts, keyboard shortcuts,
//! live topology refresh and a close guard that lets sections finish pending work.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;

use adw::prelude::*;
use gtk::glib;
use gtk4 as gtk;
use libadwaita as adw;
use settings_application::SettingsApi;
use settings_application::dto::RecoverOutcome;
use settings_domain::ports::Subscription;

use super::components::{section_search, sidebar};
use super::constants::{PLACEHOLDERS, SECTIONS};
use super::html;
use super::interfaces::{Section, ShellContext, ShellProps};
use super::utils::section_matches;
use crate::app::blocking;

struct Nav {
    stack: gtk::Stack,
    split: adw::NavigationSplitView,
    content_page: adw::NavigationPage,
}

struct Inner {
    window: adw::ApplicationWindow,
    list: gtk::ListBox,
    sections: Vec<(&'static str, Rc<dyn Section>)>,
    closing: Cell<bool>,
    subscription: RefCell<Option<Subscription>>,
}

#[derive(Clone)]
pub struct ShellWindow {
    inner: Rc<Inner>,
}

impl ShellWindow {
    pub fn new(app: &adw::Application, api: Arc<dyn SettingsApi>) -> Self {
        let toasts = adw::ToastOverlay::new();
        let ctx = ShellContext {
            api: Arc::clone(&api),
            toast: {
                let toasts = toasts.clone();
                Rc::new(move |message: &str| toasts.add_toast(adw::Toast::new(message)))
            },
        };
        let sections: Vec<(&'static str, Rc<dyn Section>)> = SECTIONS
            .iter()
            .map(|descriptor| (descriptor.id, (descriptor.build)(&ctx)))
            .collect();

        let nav: Rc<RefCell<Option<Nav>>> = Rc::default();
        let query = Rc::new(RefCell::new(String::new()));
        let list_slot: Rc<RefCell<Option<gtk::ListBox>>> = Rc::default();

        let search = section_search::build(&section_search::SectionSearchProps {
            on_change: {
                let (query, list_slot) = (Rc::clone(&query), Rc::clone(&list_slot));
                Rc::new(move |text| {
                    *query.borrow_mut() = text;
                    if let Some(list) = list_slot.borrow().as_ref() {
                        list.invalidate_filter();
                    }
                })
            },
            on_activate: {
                let list_slot = Rc::clone(&list_slot);
                Rc::new(move || {
                    if let Some(list) = list_slot.borrow().as_ref() {
                        select_first_visible(list);
                    }
                })
            },
            on_dismiss: {
                let list_slot = Rc::clone(&list_slot);
                Rc::new(move || {
                    if let Some(window) = list_slot
                        .borrow()
                        .as_ref()
                        .and_then(|l| l.root())
                        .and_downcast::<gtk::Window>()
                    {
                        window.close();
                    }
                })
            },
        });

        let sidebar = sidebar::build(&sidebar::SidebarProps {
            title: "Settings".into(),
            entries: SECTIONS
                .iter()
                .map(|d| sidebar::SidebarEntry {
                    id: d.id.into(),
                    title: d.title.into(),
                    icon: d.icon,
                    enabled: true,
                })
                .chain(PLACEHOLDERS.iter().map(|d| sidebar::SidebarEntry {
                    id: d.id.into(),
                    title: d.title.into(),
                    icon: d.icon,
                    enabled: false,
                }))
                .collect(),
            search: search.clone().upcast(),
            on_select: {
                let nav = Rc::clone(&nav);
                Rc::new(move |id| {
                    if let Some(nav) = nav.borrow().as_ref() {
                        show_section(nav, &id);
                    }
                })
            },
        });
        sidebar.list.set_filter_func({
            let query = Rc::clone(&query);
            move |row| {
                let id = row.widget_name();
                let query = query.borrow();
                SECTIONS
                    .iter()
                    .find(|d| d.id == id.as_str())
                    .map(|d| (d.title, d.keywords))
                    .or_else(|| {
                        PLACEHOLDERS
                            .iter()
                            .find(|d| d.id == id.as_str())
                            .map(|d| (d.title, &[][..]))
                    })
                    .is_some_and(|(title, keywords)| section_matches(title, keywords, &query))
            }
        });
        *list_slot.borrow_mut() = Some(sidebar.list.clone());

        let widgets = html::build(&ShellProps {
            app: app.clone(),
            toasts,
            sidebar: sidebar.page,
            pages: sections
                .iter()
                .map(|(id, section)| ((*id).to_owned(), section.widget()))
                .collect(),
        });
        *nav.borrow_mut() = Some(Nav {
            stack: widgets.stack,
            split: widgets.split,
            content_page: widgets.content_page,
        });

        let inner = Rc::new(Inner {
            window: widgets.window,
            list: sidebar.list,
            sections,
            closing: Cell::new(false),
            subscription: RefCell::default(),
        });
        inner.install_close_guard();
        inner.install_shortcuts(&search);
        inner.follow_topology(&api, &ctx);
        inner.recover_interrupted_trial(&api, &ctx);
        if let Some(first) = inner.list.row_at_index(0) {
            inner.list.select_row(Some(&first));
        }
        Self { inner }
    }

    pub fn present(&self) {
        self.inner.window.present();
    }

    pub fn is_active(&self) -> bool {
        self.inner.window.is_active()
    }

    pub fn close(&self) {
        self.inner.window.close();
    }

    pub fn select(&self, id: &str) {
        let mut index = 0;
        while let Some(row) = self.inner.list.row_at_index(index) {
            if row.widget_name() == id {
                self.inner.list.select_row(Some(&row));
                return;
            }
            index += 1;
        }
    }

    pub fn refresh(&self) {
        self.inner.refresh_sections();
    }
}

impl Inner {
    fn refresh_sections(&self) {
        for (_, section) in &self.sections {
            section.refresh();
        }
    }

    fn install_close_guard(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.window.connect_close_request(move |window| {
            let Some(inner) = weak.upgrade() else {
                return glib::Propagation::Proceed;
            };
            if inner.closing.replace(true) {
                return glib::Propagation::Stop;
            }
            let remaining = Rc::new(Cell::new(inner.sections.len()));
            if remaining.get() == 0 {
                return glib::Propagation::Proceed;
            }
            for (_, section) in &inner.sections {
                let (remaining, window) = (Rc::clone(&remaining), window.clone());
                section.on_close(Box::new(move || {
                    remaining.set(remaining.get() - 1);
                    if remaining.get() == 0 {
                        window.destroy();
                    }
                }));
            }
            glib::Propagation::Stop
        });
    }

    fn install_shortcuts(&self, search: &gtk::SearchEntry) {
        let shortcuts = gtk::ShortcutController::new();
        let window = self.window.clone();
        shortcuts.add_shortcut(gtk::Shortcut::new(
            gtk::ShortcutTrigger::parse_string("Escape"),
            Some(gtk::CallbackAction::new(move |_, _| {
                window.close();
                glib::Propagation::Stop
            })),
        ));
        let search = search.clone();
        shortcuts.add_shortcut(gtk::Shortcut::new(
            gtk::ShortcutTrigger::parse_string("<Control>f"),
            Some(gtk::CallbackAction::new(move |_, _| {
                search.grab_focus();
                glib::Propagation::Stop
            })),
        ));
        self.window.add_controller(shortcuts);
    }

    fn follow_topology(self: &Rc<Self>, api: &Arc<dyn SettingsApi>, ctx: &ShellContext) {
        let (tx, rx) = async_channel::unbounded::<()>();
        match api.subscribe(Box::new(move |_| {
            let _ = tx.try_send(());
        })) {
            Ok(subscription) => *self.subscription.borrow_mut() = Some(subscription),
            Err(err) => {
                tracing::warn!("live display updates unavailable: {err}");
                (ctx.toast)("Live display updates are unavailable");
                return;
            }
        }
        let weak: Weak<Self> = Rc::downgrade(self);
        glib::spawn_future_local(async move {
            while rx.recv().await.is_ok() {
                let Some(inner) = weak.upgrade() else { break };
                inner.refresh_sections();
            }
        });
    }

    fn recover_interrupted_trial(self: &Rc<Self>, api: &Arc<dyn SettingsApi>, ctx: &ShellContext) {
        let (weak, toast) = (Rc::downgrade(self), Rc::clone(&ctx.toast));
        blocking::call(
            api,
            |api| api.recover_pending_trial(),
            move |result| match result {
                Ok(RecoverOutcome::Restored { connector }) => {
                    toast(&format!(
                        "Restored the previous layout of {connector} after an interrupted change"
                    ));
                    if let Some(inner) = weak.upgrade() {
                        inner.refresh_sections();
                    }
                }
                Ok(RecoverOutcome::Nothing) => {}
                Err(err) => toast(&format!("Could not recover a pending change: {err}")),
            },
        );
    }
}

fn show_section(nav: &Nav, id: &str) {
    nav.stack.set_visible_child_name(id);
    if let Some(descriptor) = SECTIONS.iter().find(|d| d.id == id) {
        nav.content_page.set_title(descriptor.title);
    }
    nav.split.set_show_content(true);
}

fn select_first_visible(list: &gtk::ListBox) {
    let mut index = 0;
    while let Some(row) = list.row_at_index(index) {
        if row.is_child_visible() && row.is_selectable() {
            list.select_row(Some(&row));
            return;
        }
        index += 1;
    }
}
