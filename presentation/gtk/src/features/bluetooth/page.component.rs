//! Bluetooth section: BlueZ snapshot, three sliding subviews (main, device, advanced).

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use super::constants::SCAN_TIMEOUT_SECS;
use super::html;
use super::interfaces::BluetoothActions;
use super::utils::{advanced_props, device_detail, main_props, scan_props};
use crate::app::blocking;
use crate::features::settings_shell::interfaces::{Section, ShellContext};
use adw::prelude::*;
use gtk::glib;
use gtk4 as gtk;
use libadwaita as adw;
use settings_application::dto::{BluetoothView, DeviceId};
use settings_application::{AppError, SettingsApi};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Route {
    #[default]
    Main,
    Device,
    Advanced,
    Scan,
}

struct Page {
    this: Weak<Page>,
    ctx: ShellContext,
    stack: gtk::Stack,
    main_scroller: gtk::ScrolledWindow,
    main_body: adw::Bin,
    device: adw::Bin,
    advanced: adw::Bin,
    scan: adw::Bin,
    state: RefCell<State>,
}

#[derive(Default)]
struct State {
    view: Option<BluetoothView>,
    selected: Option<DeviceId>,
    route: Route,
    /// Where Back from a device page returns to: the list or the scan results.
    device_origin: Route,
    advanced_tab: usize,
    /// What the visible subpage was last built from; an unchanged key skips the rebuild.
    subpage_key: Option<String>,
    /// How deep the visible page is (list 0, scan or advanced 1, a device above its origin);
    /// going deeper slides forward, going shallower slides back.
    depth: u8,
    /// The scan page's search text; it survives rebuilds and leaving the page.
    scan_query: String,
    /// The user switched scanning off, so entering the page must not switch it on again.
    scan_paused: bool,
    adapter_fallback_notified: bool,
    busy: bool,
    generation: u64,
    subscription: RefCell<Option<settings_domain::ports::Subscription>>,
}

pub fn build_section(ctx: &ShellContext) -> Rc<dyn Section> {
    let page = Rc::new_cyclic(|weak: &Weak<Page>| Page::new(ctx.clone(), weak));
    page.reload();
    page
}

impl Section for Page {
    fn widget(&self) -> gtk::Widget {
        self.stack.clone().upcast()
    }

    fn refresh(&self) {
        if let Some(page) = self.this.upgrade() {
            page.reload();
        }
    }

    fn on_close(&self, proceed: Box<dyn FnOnce()>) {
        self.stop_scan_quietly();
        proceed();
    }
}

impl Page {
    fn new(ctx: ShellContext, this: &Weak<Page>) -> Self {
        let frame = html::build_frame();
        Self {
            this: this.clone(),
            ctx,
            stack: frame.root,
            main_scroller: frame.main_scroller,
            main_body: frame.main_body,
            device: frame.device,
            advanced: frame.advanced,
            scan: frame.scan,
            state: RefCell::default(),
        }
    }

    fn toast(&self, message: &str) {
        (self.ctx.toast)(message);
    }

    fn call<T: Send + 'static>(
        &self,
        work: impl FnOnce(&dyn SettingsApi) -> T + Send + 'static,
        done: impl FnOnce(&Rc<Page>, T) + 'static,
    ) {
        let this = self.this.clone();
        blocking::call(&self.ctx.api, work, move |value| {
            if let Some(page) = this.upgrade() {
                done(&page, value);
            }
        });
    }

    fn mutate(
        &self,
        work: impl FnOnce(&dyn SettingsApi) -> Result<(), AppError> + Send + 'static,
        ok_message: Option<&str>,
    ) {
        if self.state.borrow().busy {
            return;
        }
        self.state.borrow_mut().busy = true;
        let toast = ok_message.map(str::to_owned);
        self.call(work, move |page, result| {
            {
                let mut state = page.state.borrow_mut();
                state.busy = false;
                // A switch the user flipped while the action ran may not match the
                // outcome; forget the cached view so the reload redraws it.
                state.view = None;
                state.subpage_key = None;
            }
            match result {
                Ok(()) => {
                    if let Some(msg) = toast {
                        page.toast(&msg);
                    }
                }
                Err(err) => page.toast(&format!("Bluetooth action failed: {err}")),
            }
            page.reload();
        });
    }

    fn reload(self: &Rc<Self>) {
        let generation = {
            let mut state = self.state.borrow_mut();
            state.generation += 1;
            state.generation
        };
        self.call(
            |api| api.bluetooth_view(),
            move |page, result| {
                if page.state.borrow().generation != generation {
                    return;
                }
                match result {
                    Ok(view) => page.apply(view),
                    Err(err) => page.toast(&format!("Could not read Bluetooth state: {err}")),
                }
            },
        );
    }

    fn apply(self: &Rc<Self>, view: BluetoothView) {
        let unchanged = self.state.borrow().view.as_ref() == Some(&view);
        if !unchanged {
            let toast = (!self.state.borrow().adapter_fallback_notified)
                .then(|| {
                    view.missing_saved_adapter.as_ref().map(|missing| {
                        let using = view
                            .adapter
                            .as_ref()
                            .map(|a| a.name.as_str())
                            .unwrap_or("default adapter");
                        format!("Adapter {missing} not found, using {using}")
                    })
                })
                .flatten();
            if toast.is_some() {
                self.state.borrow_mut().adapter_fallback_notified = true;
            }
            if let Some(message) = toast {
                self.toast(&message);
            }
            self.state.borrow_mut().view = Some(view);
            self.render_main();
            self.show_route(true);
        }
        self.ensure_subscription();
    }

    fn ensure_subscription(self: &Rc<Self>) {
        if self.state.borrow().subscription.borrow().is_some() {
            return;
        }
        let (tx, rx) = async_channel::unbounded::<()>();
        match self.ctx.api.subscribe_bluetooth(Box::new(move || {
            let _ = tx.try_send(());
        })) {
            Ok(sub) => *self.state.borrow().subscription.borrow_mut() = Some(sub),
            Err(err) => {
                tracing::warn!("bluetooth live updates unavailable: {err}");
                return;
            }
        }
        let weak = self.this.clone();
        glib::spawn_future_local(async move {
            while rx.recv().await.is_ok() {
                if let Some(page) = weak.upgrade() {
                    page.reload();
                }
            }
        });
    }

    fn actions(self: &Rc<Self>) -> BluetoothActions {
        let this = Rc::downgrade(self);
        BluetoothActions {
            set_powered: {
                let this = this.clone();
                Rc::new(move |on| {
                    if let Some(page) = this.upgrade() {
                        page.mutate(move |api| api.set_bluetooth_powered(on), None);
                    }
                })
            },
            connect: id_action(&this, |page, id| {
                page.mutate(move |api| api.connect_device(id), None);
            }),
            pair: id_action(&this, |page, id| {
                page.mutate(move |api| api.pair_device(id), None);
            }),
            open_device: {
                let this = this.clone();
                Rc::new(move |id| {
                    if let Some(page) = this.upgrade() {
                        let mut state = page.state.borrow_mut();
                        state.selected = Some(id);
                        if state.route != Route::Device {
                            state.device_origin = state.route;
                        }
                        state.route = Route::Device;
                        drop(state);
                        page.show_route(false);
                    }
                })
            },
            open_advanced: {
                let this = this.clone();
                Rc::new(move || {
                    if let Some(page) = this.upgrade() {
                        page.state.borrow_mut().route = Route::Advanced;
                        page.show_route(false);
                    }
                })
            },
            open_scan: {
                let this = this.clone();
                Rc::new(move || {
                    if let Some(page) = this.upgrade() {
                        page.state.borrow_mut().route = Route::Scan;
                        page.show_route(false);
                        if !page.state.borrow().scan_paused {
                            page.begin_scan();
                        }
                    }
                })
            },
            start_scan: {
                let this = this.clone();
                Rc::new(move || {
                    if let Some(page) = this.upgrade() {
                        page.state.borrow_mut().scan_paused = false;
                        page.begin_scan();
                    }
                })
            },
            stop_scan: {
                let this = this.clone();
                Rc::new(move || {
                    if let Some(page) = this.upgrade() {
                        page.state.borrow_mut().scan_paused = true;
                        page.mutate(|api| api.stop_bluetooth_discovery(), None);
                    }
                })
            },
            close_subpage: {
                let this = this.clone();
                Rc::new(move || {
                    if let Some(page) = this.upgrade() {
                        page.go_back();
                    }
                })
            },
            select_adapter: {
                let this = this.clone();
                Rc::new(move |address| {
                    if let Some(page) = this.upgrade() {
                        page.stop_scan_quietly();
                        {
                            let mut state = page.state.borrow_mut();
                            state.route = Route::Main;
                            state.selected = None;
                            state.device_origin = Route::Main;
                            state.scan_query.clear();
                            state.scan_paused = false;
                            state.subpage_key = None;
                        }
                        page.mutate(move |api| api.select_bluetooth_adapter(address), None);
                    }
                })
            },
            select_advanced_tab: {
                let this = this.clone();
                Rc::new(move |tab| {
                    if let Some(page) = this.upgrade() {
                        page.state.borrow_mut().advanced_tab = tab;
                        page.show_route(false);
                    }
                })
            },
            forget: id_action(&this, |page, id| {
                page.state.borrow_mut().route = Route::Main;
                page.mutate(move |api| api.forget_device(id), Some("Device removed"));
            }),
            set_connected: {
                let this = this.clone();
                Rc::new(move |id, on| {
                    if let Some(page) = this.upgrade() {
                        if on {
                            page.mutate(move |api| api.connect_device(id), None);
                        } else {
                            page.mutate(move |api| api.disconnect_device(id), None);
                        }
                    }
                })
            },
            set_class: {
                let this = this.clone();
                Rc::new(move |id, class| {
                    if let Some(page) = this.upgrade() {
                        page.mutate(move |api| api.set_device_class(id, class), None);
                    }
                })
            },
            set_profile: {
                let this = this.clone();
                Rc::new(move |id, profile, on| {
                    if let Some(page) = this.upgrade() {
                        page.mutate(move |api| api.set_device_profile(id, profile, on), None);
                    }
                })
            },
            set_codec: {
                let this = this.clone();
                Rc::new(move |id, codec| {
                    if let Some(page) = this.upgrade() {
                        page.mutate(move |api| api.set_device_codec(id, codec), None);
                    }
                })
            },
            set_absolute_volume: {
                let this = this.clone();
                Rc::new(move |id, on| {
                    if let Some(page) = this.upgrade() {
                        page.mutate(move |api| api.set_absolute_volume(id, on), None);
                    }
                })
            },
            set_pan: {
                let this = this.clone();
                Rc::new(move |on| {
                    if let Some(page) = this.upgrade() {
                        page.mutate(move |api| api.set_pan_enabled(on), None);
                    }
                })
            },
            set_avrcp: {
                let this = this.clone();
                Rc::new(move |version| {
                    if let Some(page) = this.upgrade() {
                        page.mutate(move |api| api.set_avrcp_version(version), None);
                    }
                })
            },
            set_sample_rate: {
                let this = this.clone();
                Rc::new(move |hz| {
                    if let Some(page) = this.upgrade() {
                        page.mutate(move |api| api.set_sample_rate_hz(hz), None);
                    }
                })
            },
            set_offload_disabled: {
                let this = this.clone();
                Rc::new(move |disabled| {
                    if let Some(page) = this.upgrade() {
                        page.mutate(move |api| api.set_a2dp_offload_disabled(disabled), None);
                    }
                })
            },
            set_hci_snoop: {
                let this = this.clone();
                Rc::new(move |on| {
                    if let Some(page) = this.upgrade() {
                        page.mutate(move |api| api.set_hci_snoop_enabled(on), None);
                    }
                })
            },
            clear_gatt: {
                let this = this.clone();
                Rc::new(move || {
                    if let Some(page) = this.upgrade() {
                        page.mutate(move |api| api.clear_gatt_cache(), None);
                    }
                })
            },
        }
    }

    /// Rebuilds the device list page, keeping the scroll position so a live update does
    /// not throw the user back to the top.
    fn render_main(self: &Rc<Self>) {
        let state = self.state.borrow();
        let Some(view) = state.view.as_ref() else {
            return;
        };
        let adjustment = self.main_scroller.vadjustment();
        let offset = adjustment.value();
        self.main_body
            .set_child(Some(&html::build_main(&main_props(view, &self.actions()))));
        restore_scroll(&adjustment, offset);
    }

    /// Shows a page with a slide that follows the navigation, not the order the pages were
    /// added to the stack in.
    fn slide_to(&self, name: &str, depth: u8) {
        let previous = std::mem::replace(&mut self.state.borrow_mut().depth, depth);
        let transition = match depth.cmp(&previous) {
            std::cmp::Ordering::Greater => gtk::StackTransitionType::SlideLeft,
            std::cmp::Ordering::Less => gtk::StackTransitionType::SlideRight,
            std::cmp::Ordering::Equal => gtk::StackTransitionType::None,
        };
        self.stack.set_visible_child_full(name, transition);
    }

    /// Starts a scan and stops it again after `SCAN_TIMEOUT_SECS`, or sooner when the user
    /// leaves the page.
    fn begin_scan(self: &Rc<Self>) {
        self.mutate(|api| api.start_bluetooth_discovery(), None);
        let weak = self.this.clone();
        glib::timeout_add_seconds_local_once(SCAN_TIMEOUT_SECS, move || {
            if let Some(page) = weak.upgrade() {
                let still_scanning = {
                    let state = page.state.borrow();
                    state.route == Route::Scan && state.view.as_ref().is_some_and(|v| v.discovering)
                };
                if still_scanning {
                    page.mutate(|api| api.stop_bluetooth_discovery(), None);
                }
            }
        });
    }

    /// Best-effort stop for when the page is left; nothing waits on the result.
    fn stop_scan_quietly(&self) {
        let discovering = self
            .state
            .borrow()
            .view
            .as_ref()
            .is_some_and(|v| v.discovering);
        if discovering {
            blocking::call(&self.ctx.api, |api| api.stop_bluetooth_discovery(), |_| {});
        }
    }

    fn go_back(self: &Rc<Self>) {
        let route = self.state.borrow().route;
        if route == Route::Scan {
            self.stop_scan_quietly();
        }
        {
            let mut state = self.state.borrow_mut();
            state.route = match route {
                Route::Device => state.device_origin,
                _ => Route::Main,
            };
            state.subpage_key = None;
        }
        self.show_route(false);
    }

    /// Shows the subpage the route names, building it from the current view. A live update
    /// (`live`) rebuilds only when the page's own data changed and keeps the scroll
    /// position; navigation always builds fresh from the top.
    fn show_route(self: &Rc<Self>, live: bool) {
        let state = self.state.borrow();
        let Some(view) = state.view.as_ref() else {
            return;
        };
        let actions = self.actions();
        let route = state.route;
        let key = match route {
            Route::Main => None,
            Route::Device => state.selected.as_ref().map(|id| {
                format!(
                    "device:{:?}:{:?}",
                    view.paired
                        .iter()
                        .chain(&view.discovered)
                        .find(|d| d.id == *id),
                    view.capabilities
                )
            }),
            Route::Advanced => Some(format!(
                "advanced:{}:{:?}",
                state.advanced_tab,
                (
                    view.powered,
                    &view.adapter,
                    &view.advanced,
                    &view.capabilities
                )
            )),
            Route::Scan => Some(format!("scan:{:?}", (view.discovering, &view.discovered))),
        };
        if live && key.is_some() && state.subpage_key == key {
            return;
        }

        let (bin, widget, name) = match route {
            Route::Main => {
                drop(state);
                self.slide_to("main", 0);
                return;
            }
            Route::Device => {
                if let Some(id) = state.selected.as_ref()
                    && let Some(props) = device_detail(view, id, &actions)
                {
                    (&self.device, html::build_device_detail(&props), "device")
                } else {
                    drop(state);
                    self.state.borrow_mut().route = Route::Main;
                    self.show_route(false);
                    return;
                }
            }
            Route::Advanced => (
                &self.advanced,
                html::build_advanced(&advanced_props(view, state.advanced_tab, &actions)),
                "advanced",
            ),
            Route::Scan => {
                let weak = self.this.clone();
                let on_query: Rc<dyn Fn(String)> = Rc::new(move |query| {
                    if let Some(page) = weak.upgrade() {
                        page.state.borrow_mut().scan_query = query;
                    }
                });
                (
                    &self.scan,
                    html::build_scan(&scan_props(view, &state.scan_query, on_query, &actions)),
                    "scan",
                )
            }
        };
        replace_keeping_scroll(bin, &widget, live);
        let depth = match route {
            Route::Device if state.device_origin == Route::Scan => 2,
            Route::Device => 1,
            _ => 1,
        };
        drop(state);
        self.slide_to(name, depth);
        self.state.borrow_mut().subpage_key = key;
    }
}

/// Swaps a subpage's content; for a live update the scroll offset and the search field's
/// focus carry over to the rebuilt page.
fn replace_keeping_scroll(bin: &adw::Bin, widget: &gtk::Widget, keep: bool) {
    let old = keep.then(|| bin.child()).flatten();
    let offset = old
        .as_ref()
        .and_then(scroller_of)
        .map_or(0.0, |scroller| scroller.vadjustment().value());
    let focused_entry = old.as_ref().and_then(focused_search_entry);

    bin.set_child(Some(widget));

    if keep {
        if let Some(scroller) = scroller_of(widget) {
            restore_scroll(&scroller.vadjustment(), offset);
        }
        if let Some((cursor, _)) = focused_entry
            && let Some(entry) = find_search_entry(widget)
        {
            entry.grab_focus();
            entry.select_region(cursor, cursor);
        }
    }
}

fn scroller_of(page: &gtk::Widget) -> Option<gtk::ScrolledWindow> {
    page.last_child()?.downcast::<gtk::ScrolledWindow>().ok()
}

fn find_search_entry(widget: &gtk::Widget) -> Option<gtk::SearchEntry> {
    if let Some(entry) = widget.downcast_ref::<gtk::SearchEntry>() {
        return Some(entry.clone());
    }
    let mut child = widget.first_child();
    while let Some(current) = child {
        if let Some(found) = find_search_entry(&current) {
            return Some(found);
        }
        child = current.next_sibling();
    }
    None
}

/// The search field under `widget`, with its cursor, when the user is typing in it.
fn focused_search_entry(widget: &gtk::Widget) -> Option<(i32, gtk::SearchEntry)> {
    let entry = find_search_entry(widget)?;
    let focus = entry.root()?.focus()?;
    (focus == *entry.upcast_ref::<gtk::Widget>() || focus.is_ancestor(&entry))
        .then(|| (entry.position(), entry))
}

/// A rebuilt page starts empty and grows into its content, so the offset can only be
/// applied once the adjustment is tall enough; give up if it never gets there.
fn restore_scroll(adjustment: &gtk::Adjustment, offset: f64) {
    if offset <= 0.0 {
        return;
    }
    let handler: Rc<RefCell<Option<glib::SignalHandlerId>>> = Rc::default();
    let id = adjustment.connect_changed({
        let handler = handler.clone();
        move |adjustment| {
            if adjustment.upper() - adjustment.page_size() >= offset {
                adjustment.set_value(offset);
                if let Some(id) = handler.borrow_mut().take() {
                    adjustment.disconnect(id);
                }
            }
        }
    });
    *handler.borrow_mut() = Some(id);
    let adjustment = adjustment.clone();
    glib::timeout_add_local_once(std::time::Duration::from_millis(600), move || {
        if let Some(id) = handler.borrow_mut().take() {
            adjustment.disconnect(id);
        }
    });
}

fn id_action(this: &Weak<Page>, run: fn(&Rc<Page>, DeviceId)) -> Rc<dyn Fn(DeviceId)> {
    let this = this.clone();
    Rc::new(move |id| {
        if let Some(page) = this.upgrade() {
            run(&page, id);
        }
    })
}
