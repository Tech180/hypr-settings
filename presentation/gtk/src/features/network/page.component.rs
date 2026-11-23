//! Network section: NetworkManager state behind a home page and five sliding subpages
//! (Wi-Fi, Ethernet, VPN, Diagnostics, Connection Properties).

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use super::constants::RATE_POLL_SECS;
use super::html;
use super::interfaces::{
    CardMemory, ConfirmProps, DiagnosticsProps, EthernetProps, HomeProps, NetPage, NetworkActions,
    PropertiesProps, QrProps, VpnDetailProps, VpnFormProps, VpnProps, WifiPagination, WifiProps,
};
use crate::shared::pagination::DEFAULT_PAGE_SIZE;
use crate::app::blocking;
use crate::features::settings_shell::interfaces::{Section, ShellContext};
use adw::prelude::*;
use gtk::glib;
use gtk4 as gtk;
use libadwaita as adw;
use settings_application::network_dto::{
    DiagnosticsView, EthernetView, NetworkOverview, ScanResult, VpnView, WifiProfileView, WifiView,
};
use settings_application::{AppError, SettingsApi};
use settings_domain::network::{
    IpConfig, IpVersion, NetworkPreferences, ScanPolicy, ScanReason, VpnDraft,
};

use super::components::wifi_page::{refresh_wifi_lists, WifiListSurfaces};
use super::utils::available_networks;
use super::utils::choose_vpn_config_file;

/// How often the page checks whether a periodic scan is due.
const SCAN_TICK_SECS: u32 = 5;

/// What the visible page needs besides the overview.
enum RouteData {
    None,
    Wifi(Result<(WifiView, NetworkPreferences), AppError>),
    Ethernet(Result<EthernetView, AppError>),
    Vpn(Result<VpnView, AppError>),
    Diagnostics(Result<DiagnosticsView, AppError>),
    Properties(Result<WifiProfileView, AppError>),
}

struct Snapshot {
    overview: Result<NetworkOverview, AppError>,
    data: RouteData,
}

fn fetch(api: &dyn SettingsApi, route: NetPage) -> Snapshot {
    let data = match route {
        NetPage::Home => RouteData::None,
        NetPage::Wifi => RouteData::Wifi(
            api.wifi_view()
                .and_then(|view| api.network_preferences().map(|prefs| (view, prefs))),
        ),
        NetPage::Ethernet => RouteData::Ethernet(api.ethernet_view()),
        NetPage::Vpn | NetPage::VpnDetail | NetPage::VpnForm => RouteData::Vpn(api.vpn_view()),
        NetPage::Diagnostics => RouteData::Diagnostics(api.network_diagnostics()),
        NetPage::Properties => RouteData::Properties(api.wifi_profile()),
    };
    Snapshot {
        overview: api.network_overview(),
        data,
    }
}

struct Page {
    this: Weak<Page>,
    ctx: ShellContext,
    frame: super::interfaces::PageFrame,
    state: RefCell<State>,
}

struct State {
    route: NetPage,
    /// Where Back from Connection Properties returns to.
    properties_origin: NetPage,
    depth: u8,
    overview: Option<NetworkOverview>,
    wifi: Option<(WifiView, NetworkPreferences)>,
    ethernet: Option<EthernetView>,
    vpn: Option<VpnView>,
    vpn_detail_id: Option<String>,
    vpn_form_draft: Option<VpnDraft>,
    /// Where Back from the WireGuard editor returns.
    vpn_form_origin: NetPage,
    diagnostics: Option<DiagnosticsView>,
    properties: Option<WifiProfileView>,
    /// What each page was last built from; an unchanged key skips a live rebuild.
    keys: std::collections::HashMap<&'static str, String>,
    query: String,
    wifi_saved_page: usize,
    wifi_saved_page_size: usize,
    wifi_available_page: usize,
    wifi_available_page_size: usize,
    wifi_lists: Option<WifiListSurfaces>,
    memory: CardMemory,
    scanning: bool,
    /// Monotonic time of the last scan request, in microseconds.
    last_scan_us: i64,
    home_rates: Option<(gtk::Label, gtk::Label)>,
    busy: bool,
    generation: u64,
    subscription: Option<settings_domain::ports::Subscription>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            route: NetPage::Home,
            properties_origin: NetPage::Wifi,
            depth: 0,
            overview: None,
            wifi: None,
            ethernet: None,
            vpn: None,
            vpn_detail_id: None,
            vpn_form_draft: None,
            vpn_form_origin: NetPage::Vpn,
            diagnostics: None,
            properties: None,
            keys: std::collections::HashMap::new(),
            query: String::new(),
            wifi_saved_page: 1,
            wifi_saved_page_size: DEFAULT_PAGE_SIZE,
            wifi_available_page: 1,
            wifi_available_page_size: DEFAULT_PAGE_SIZE,
            wifi_lists: None,
            memory: CardMemory::default(),
            scanning: false,
            last_scan_us: 0,
            home_rates: None,
            busy: false,
            generation: 0,
            subscription: None,
        }
    }
}

pub fn build_section(ctx: &ShellContext) -> Rc<dyn Section> {
    let page = Rc::new_cyclic(|weak: &Weak<Page>| Page::new(ctx.clone(), weak));
    page.install_keys();
    page.start_timers();
    page.reload();
    page
}

impl Section for Page {
    fn widget(&self) -> gtk::Widget {
        self.frame.root.clone().upcast()
    }

    fn refresh(&self) {
        if let Some(page) = self.this.upgrade() {
            page.reload();
        }
    }

    fn on_close(&self, proceed: Box<dyn FnOnce()>) {
        proceed();
    }
}

impl Page {
    fn new(ctx: ShellContext, this: &Weak<Page>) -> Self {
        Self {
            this: this.clone(),
            ctx,
            frame: html::build_frame(),
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

    /// Runs a command; afterwards every page redraws from fresh data.
    fn mutate(
        &self,
        work: impl FnOnce(&dyn SettingsApi) -> Result<(), AppError> + Send + 'static,
        ok_message: Option<String>,
    ) {
        self.mutate_then(work, ok_message, |_| {});
    }

    fn mutate_then(
        &self,
        work: impl FnOnce(&dyn SettingsApi) -> Result<(), AppError> + Send + 'static,
        ok_message: Option<String>,
        after: impl FnOnce(&Rc<Page>) + 'static,
    ) {
        if self.state.borrow().busy {
            self.toast("Another network action is still running");
            return;
        }
        self.state.borrow_mut().busy = true;
        self.call(work, move |page, result| {
            {
                let mut state = page.state.borrow_mut();
                state.busy = false;
                // A switch flipped while the command ran may not match its outcome.
                state.keys.clear();
            }
            match result {
                Ok(()) => {
                    if let Some(message) = ok_message {
                        page.toast(&message);
                    }
                    after(page);
                }
                Err(err) => page.toast(&format!("Network action failed: {err}")),
            }
            page.reload();
        });
    }

    fn reload(self: &Rc<Self>) {
        let (route, generation) = {
            let mut state = self.state.borrow_mut();
            state.generation += 1;
            (state.route, state.generation)
        };
        self.call(
            move |api| fetch(api, route),
            move |page, snapshot| {
                if page.state.borrow().generation != generation {
                    return;
                }
                page.apply(route, snapshot);
            },
        );
    }

    fn apply(self: &Rc<Self>, route: NetPage, snapshot: Snapshot) {
        match snapshot.overview {
            Ok(overview) => self.state.borrow_mut().overview = Some(overview),
            Err(err) => self.toast(&format!("Could not read network state: {err}")),
        }
        {
            let mut state = self.state.borrow_mut();
            match snapshot.data {
                RouteData::None => {}
                RouteData::Wifi(Ok(data)) => state.wifi = Some(data),
                RouteData::Ethernet(Ok(view)) => state.ethernet = Some(view),
                RouteData::Vpn(Ok(view)) => state.vpn = Some(view),
                RouteData::Diagnostics(Ok(view)) => state.diagnostics = Some(view),
                RouteData::Properties(Ok(view)) => state.properties = Some(view),
                RouteData::Wifi(Err(e))
                | RouteData::Ethernet(Err(e))
                | RouteData::Vpn(Err(e))
                | RouteData::Diagnostics(Err(e))
                | RouteData::Properties(Err(e)) => {
                    drop(state);
                    self.toast(&format!("Could not read network state: {e}"));
                }
            }
        }
        if self.state.borrow().route == route {
            self.render(route, true);
        }
        self.ensure_subscription();
    }

    fn ensure_subscription(self: &Rc<Self>) {
        if self.state.borrow().subscription.is_some() {
            return;
        }
        let (tx, rx) = async_channel::unbounded::<()>();
        match self.ctx.api.subscribe_network(Box::new(move || {
            let _ = tx.try_send(());
        })) {
            Ok(sub) => self.state.borrow_mut().subscription = Some(sub),
            Err(err) => {
                tracing::warn!("network live updates unavailable: {err}");
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

    // ---- timers --------------------------------------------------------------------

    fn is_showing(&self) -> bool {
        self.frame.root.is_mapped()
    }

    fn start_timers(self: &Rc<Self>) {
        let weak = self.this.clone();
        glib::timeout_add_seconds_local(RATE_POLL_SECS, move || {
            let Some(page) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            if page.is_showing() && page.state.borrow().route == NetPage::Home {
                page.poll_rates();
            }
            glib::ControlFlow::Continue
        });
        let weak = self.this.clone();
        glib::timeout_add_seconds_local(SCAN_TICK_SECS, move || {
            let Some(page) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            page.periodic_scan();
            glib::ControlFlow::Continue
        });
    }

    fn poll_rates(&self) {
        self.call(
            |api| api.network_rates(),
            |page, rates| {
                if let Some((tx, rx)) = &page.state.borrow().home_rates {
                    tx.set_label(&super::utils::format_rate(rates.tx_per_sec));
                    rx.set_label(&super::utils::format_rate(rates.rx_per_sec));
                }
            },
        );
    }

    fn periodic_scan(self: &Rc<Self>) {
        let due = {
            let state = self.state.borrow();
            let interval_us = state
                .wifi
                .as_ref()
                .filter(|(_, prefs)| prefs.policy == ScanPolicy::Periodic)
                .map(|(_, prefs)| i64::from(prefs.interval_ms) * 1000);
            state.route == NetPage::Wifi
                && interval_us
                    .is_some_and(|interval| glib::monotonic_time() - state.last_scan_us >= interval)
        };
        if due && self.is_showing() {
            self.request_scan(ScanReason::Periodic, false);
        }
    }

    fn request_scan(self: &Rc<Self>, reason: ScanReason, force: bool) {
        {
            let mut state = self.state.borrow_mut();
            if state.scanning {
                return;
            }
            state.scanning = true;
            state.last_scan_us = glib::monotonic_time();
        }
        self.render(NetPage::Wifi, true);
        self.call(
            move |api| api.request_wifi_scan(reason, force, true),
            move |page, result| {
                page.state.borrow_mut().scanning = false;
                match result {
                    Ok(ScanResult::Throttled) if force => {
                        page.toast("NetworkManager throttled the scan; showing known networks");
                    }
                    Ok(_) => {}
                    Err(err) => page.toast(&format!("Scan failed: {err}")),
                }
                page.reload();
            },
        );
    }

    // ---- navigation ----------------------------------------------------------------

    fn go(self: &Rc<Self>, page: NetPage) {
        {
            let mut state = self.state.borrow_mut();
            if page == NetPage::Properties && state.route != NetPage::Properties {
                state.properties_origin = state.route;
            }
            state.route = page;
            state.keys.remove(page.stack_name());
        }
        self.render(page, false);
        self.reload();
        if page == NetPage::Wifi {
            let policy = self
                .state
                .borrow()
                .wifi
                .as_ref()
                .map(|(_, prefs)| prefs.policy);
            if policy != Some(ScanPolicy::Manual) {
                self.request_scan(ScanReason::Open, false);
            }
        }
    }

    fn go_back(self: &Rc<Self>) {
        let target = {
            let mut state = self.state.borrow_mut();
            match state.route {
                NetPage::Home => return,
                NetPage::Properties => state.properties_origin,
                NetPage::VpnDetail => NetPage::Vpn,
                NetPage::VpnForm => {
                    let origin = state.vpn_form_origin;
                    state.vpn_form_draft = None;
                    origin
                }
                _ => NetPage::Home,
            }
        };
        self.state.borrow_mut().route = target;
        self.render(target, false);
        self.reload();
    }

    fn depth_of(state: &State, page: NetPage) -> u8 {
        match page {
            NetPage::Home => 0,
            NetPage::Properties | NetPage::VpnDetail => 2,
            NetPage::VpnForm => {
                if state.vpn_form_origin == NetPage::VpnDetail {
                    3
                } else {
                    2
                }
            }
            _ => 1,
        }
    }

    fn slide_to(&self, page: NetPage) {
        let depth = Self::depth_of(&self.state.borrow(), page);
        let previous = std::mem::replace(&mut self.state.borrow_mut().depth, depth);
        let transition = match depth.cmp(&previous) {
            std::cmp::Ordering::Greater => gtk::StackTransitionType::SlideLeft,
            std::cmp::Ordering::Less => gtk::StackTransitionType::SlideRight,
            std::cmp::Ordering::Equal => gtk::StackTransitionType::None,
        };
        self.frame
            .root
            .set_visible_child_full(page.stack_name(), transition);
    }

    // ---- rendering -----------------------------------------------------------------

    /// Builds `page` from what is known. A live render (`live`) rebuilds only when the
    /// page's own data changed, keeps the scroll offset and never interrupts typing;
    /// navigation builds fresh from the top.
    fn render(self: &Rc<Self>, page: NetPage, live: bool) {
        let actions = self.actions();
        let (bin, widget, key) = {
            let state = self.state.borrow();
            let name = page.stack_name();
            let built = match page {
                NetPage::Home => state.overview.clone().map(|overview| {
                    let key = format!("{overview:?}");
                    (
                        &self.frame.home,
                        Built::Home(Box::new(HomeProps { overview, actions })),
                        key,
                    )
                }),
                NetPage::Wifi => state.wifi.clone().map(|(view, prefs)| {
                    let scanning = state.scanning || view.scanning;
                    let key = format!(
                        "{:?}{:?}{scanning}{}",
                        view.signature,
                        (
                            view.enabled,
                            &view.selected,
                            &view.devices,
                            &view.saved,
                            &prefs
                        ),
                        state.query,
                    );
                    let props = WifiProps {
                        view,
                        prefs,
                        scanning,
                        query: state.query.clone(),
                        pagination: WifiPagination {
                            saved_page: state.wifi_saved_page,
                            saved_page_size: state.wifi_saved_page_size,
                            available_page: state.wifi_available_page,
                            available_page_size: state.wifi_available_page_size,
                        },
                        memory: state.memory.clone(),
                        actions,
                    };
                    let (widget, lists) = html::build_wifi(&props);
                    (
                        &self.frame.wifi,
                        Built::Wifi {
                            widget,
                            lists,
                        },
                        key,
                    )
                }),
                NetPage::Ethernet => state.ethernet.clone().map(|view| {
                    let key = format!("{view:?}");
                    let this = self.this.clone();
                    let props = EthernetProps {
                        view,
                        memory: state.memory.clone(),
                        actions,
                        apply_ip: Rc::new({
                            let this = this.clone();
                            move |version, config| {
                                if let Some(page) = this.upgrade() {
                                    page.apply_connection_ip(NetPage::Ethernet, version, config);
                                }
                            }
                        }),
                    };
                    (
                        &self.frame.ethernet,
                        Built::Page(html::build_ethernet(&props)),
                        key,
                    )
                }),
                NetPage::Vpn => state.vpn.clone().map(|view| {
                    let key = format!("{view:?}");
                    let props = VpnProps { view, actions };
                    (&self.frame.vpn, Built::Page(html::build_vpn(&props)), key)
                }),
                NetPage::VpnDetail => state.vpn_detail_id.clone().and_then(|id| {
                    state.vpn.clone().and_then(|view| {
                        view.profiles
                            .iter()
                            .find(|p| p.id == id)
                            .cloned()
                            .map(|profile| {
                                let key = format!("{profile:?}");
                                let props = VpnDetailProps { profile, actions };
                                (
                                    &self.frame.vpn_detail,
                                    Built::Page(html::build_vpn_detail(&props)),
                                    key,
                                )
                            })
                    })
                }),
                NetPage::VpnForm => state.vpn_form_draft.clone().map(|draft| {
                    let key = format!("{draft:?}");
                    let this = self.this.clone();
                    let props = VpnFormProps {
                        draft,
                        actions: actions.clone(),
                        on_save: Rc::new(move |draft| {
                            if let Some(page) = this.upgrade() {
                                page.save_vpn_form(draft);
                            }
                        }),
                    };
                    (
                        &self.frame.vpn_form,
                        Built::Page(html::build_vpn_form(&props)),
                        key,
                    )
                }),
                NetPage::Diagnostics => state.diagnostics.clone().map(|view| {
                    let key = format!("{view:?}");
                    let props = DiagnosticsProps { view, actions };
                    (
                        &self.frame.diagnostics,
                        Built::Page(html::build_diagnostics(&props)),
                        key,
                    )
                }),
                NetPage::Properties => state.properties.clone().map(|view| {
                    let key = format!("{view:?}");
                    let this = self.this.clone();
                    let props = PropertiesProps {
                        view,
                        actions,
                        apply_ip: Rc::new({
                            let this = this.clone();
                            move |version, config| {
                                if let Some(page) = this.upgrade() {
                                    page.apply_connection_ip(NetPage::Properties, version, config);
                                }
                            }
                        }),
                    };
                    (
                        &self.frame.properties,
                        Built::Page(html::build_properties(&props)),
                        key,
                    )
                }),
            };
            let Some((bin, built, key)) = built else {
                drop(state);
                if let Some(bin) = self.bin_of(page)
                    && bin.child().is_none()
                {
                    bin.set_child(Some(&html::build_loading(page, &self.actions())));
                }
                self.slide_to(page);
                return;
            };
            if live && state.keys.get(name) == Some(&key) {
                return;
            }
            if live && focus_in_editable(bin.upcast_ref()) {
                // Rebuilding would drop the cursor and the half-typed text.
                return;
            }
            let widget = match built {
                Built::Home(props) => {
                    let widgets = html::build_home(&props);
                    let root = widgets.root.clone();
                    drop(state);
                    self.state.borrow_mut().home_rates = Some((widgets.tx, widgets.rx));
                    self.poll_rates();
                    (bin.clone(), root, key)
                }
                Built::Page(widget) => (bin.clone(), widget, key),
                Built::Wifi { widget, lists } => {
                    drop(state);
                    self.state.borrow_mut().wifi_lists = Some(lists);
                    (bin.clone(), widget, key)
                }
            };
            (widget.0, widget.1, widget.2)
        };
        self.state.borrow_mut().keys.insert(page.stack_name(), key);
        replace_keeping_scroll(&bin, &widget, live);
        if !live || self.frame.root.visible_child_name().as_deref() != Some(page.stack_name()) {
            self.slide_to(page);
        }
    }

    fn refresh_wifi_lists(self: &Rc<Self>) {
        let (lists, props, available, query) = {
            let state = self.state.borrow();
            let Some(lists) = state.wifi_lists.clone() else {
                return;
            };
            let Some((view, prefs)) = state.wifi.clone() else {
                return;
            };
            let scanning = state.scanning || view.scanning;
            let props = WifiProps {
                view: view.clone(),
                prefs,
                scanning,
                query: state.query.clone(),
                pagination: WifiPagination {
                    saved_page: state.wifi_saved_page,
                    saved_page_size: state.wifi_saved_page_size,
                    available_page: state.wifi_available_page,
                    available_page_size: state.wifi_available_page_size,
                },
                memory: state.memory.clone(),
                actions: self.actions(),
            };
            let available = available_networks(&view);
            let query = state.query.clone();
            (lists, props, available, query)
        };
        refresh_wifi_lists(
            &lists.saved_slot,
            &lists.saved_pager,
            &lists.results,
            &lists.available_pager,
            &available,
            &props,
            &query,
        );
    }

    fn bin_of(&self, page: NetPage) -> Option<&adw::Bin> {
        match page {
            NetPage::Home => None,
            NetPage::Wifi => Some(&self.frame.wifi),
            NetPage::Ethernet => Some(&self.frame.ethernet),
            NetPage::Vpn => Some(&self.frame.vpn),
            NetPage::VpnDetail => Some(&self.frame.vpn_detail),
            NetPage::VpnForm => Some(&self.frame.vpn_form),
            NetPage::Diagnostics => Some(&self.frame.diagnostics),
            NetPage::Properties => Some(&self.frame.properties),
        }
    }

    // ---- dialogs -------------------------------------------------------------------

    fn anchor(&self) -> gtk::Widget {
        self.frame.root.clone().upcast()
    }

    fn confirm(&self, heading: &str, body: &str, confirm_label: &str, on_confirm: Rc<dyn Fn()>) {
        html::confirm(
            &self.anchor(),
            &ConfirmProps {
                heading: heading.to_owned(),
                body: body.to_owned(),
                confirm_label: confirm_label.to_owned(),
                on_confirm,
            },
        );
    }

    fn apply_connection_ip(self: &Rc<Self>, page: NetPage, version: IpVersion, config: IpConfig) {
        let id = match page {
            NetPage::Properties => self
                .state
                .borrow()
                .properties
                .as_ref()
                .and_then(|view| view.profile.as_ref().map(|profile| profile.id.clone())),
            NetPage::Ethernet => self
                .state
                .borrow()
                .ethernet
                .as_ref()
                .and_then(|view| view.profile.as_ref().map(|profile| profile.id.clone())),
            _ => None,
        };
        let Some(id) = id else { return };
        self.mutate(
            move |api| api.apply_ip_settings(id, version, config),
            Some("IP settings applied".into()),
        );
    }

    fn share(self: &Rc<Self>, id: String) {
        let ssid = self
            .state
            .borrow()
            .properties
            .as_ref()
            .and_then(|view| view.profile.as_ref())
            .map(|profile| profile.ssid.clone())
            .unwrap_or_default();
        self.call(
            move |api| api.wifi_qr(id),
            move |page, result| match result {
                Ok(matrix) => html::show_qr(&page.anchor(), &QrProps { ssid, matrix }),
                Err(err) => page.toast(&format!("Could not create the QR code: {err}")),
            },
        );
    }

    fn forget(self: &Rc<Self>, id: String) {
        let this = self.this.clone();
        self.confirm(
            "Forget this network?",
            "Its saved password is removed and it will not be joined automatically again.",
            "Forget",
            Rc::new(move || {
                let Some(page) = this.upgrade() else { return };
                let id = id.clone();
                page.mutate_then(
                    move |api| api.forget_connection(id),
                    Some("Network forgotten".into()),
                    |page| {
                        page.state.borrow_mut().route = NetPage::Wifi;
                        page.render(NetPage::Wifi, false);
                    },
                );
            }),
        );
    }

    fn open_vpn_form(self: &Rc<Self>, draft: VpnDraft) {
        {
            let mut state = self.state.borrow_mut();
            state.vpn_form_origin = state.route;
            state.vpn_form_draft = Some(draft);
        }
        self.go(NetPage::VpnForm);
    }

    fn new_vpn(self: &Rc<Self>) {
        self.open_vpn_form(VpnDraft::default());
    }

    fn edit_vpn(self: &Rc<Self>, id: String) {
        self.call(
            move |api| api.load_vpn_draft(id),
            |page, result| match result {
                Ok(draft) => page.open_vpn_form(draft),
                Err(err) => page.toast(&format!("Could not open the tunnel: {err}")),
            },
        );
    }

    fn save_vpn_form(self: &Rc<Self>, draft: VpnDraft) {
        let origin = self.state.borrow().vpn_form_origin;
        let message = if draft.id.is_empty() {
            "Tunnel created"
        } else {
            "Tunnel saved"
        };
        self.mutate(
            move |api| api.save_vpn_draft(draft),
            Some(message.into()),
        );
        self.state.borrow_mut().vpn_form_draft = None;
        self.go(origin);
    }

    fn delete_vpn(self: &Rc<Self>, id: String) {
        let name = self
            .state
            .borrow()
            .vpn
            .as_ref()
            .and_then(|view| view.profiles.iter().find(|p| p.id == id))
            .map_or_else(|| id.clone(), |p| p.name.clone());
        let this = self.this.clone();
        self.confirm(
            &format!("Delete “{name}”?"),
            "The tunnel and its keys are removed from this computer.",
            "Delete",
            Rc::new(move || {
                if let Some(page) = this.upgrade() {
                    let id = id.clone();
                    page.mutate(move |api| api.delete_vpn(id), Some("Tunnel deleted".into()));
                }
            }),
        );
    }

    fn import_vpn(self: &Rc<Self>) {
        let this = self.this.clone();
        choose_vpn_config_file(&self.anchor(), Rc::new(move |path| {
            if let Some(page) = this.upgrade() {
                page.import_path(path);
            }
        }));
    }

    fn import_path(self: &Rc<Self>, path: String) {
        if self.state.borrow().busy {
            self.toast("Another network action is still running");
            return;
        }
        self.state.borrow_mut().busy = true;
        self.call(
            move |api| api.import_vpn(path),
            |page, result| {
                {
                    let mut state = page.state.borrow_mut();
                    state.busy = false;
                    state.keys.clear();
                }
                match result {
                    Ok(name) => page.toast(&format!("Imported {name}")),
                    Err(err) => page.toast(&format!("Import failed: {err}")),
                }
                page.reload();
            },
        );
    }

    fn connect_wifi(self: &Rc<Self>, ssid: String, password: Option<String>) {
        self.toast(&format!("Connecting to {ssid}…"));
        let joined = ssid.clone();
        let name = ssid.clone();
        self.mutate_then(
            move |api| api.connect_wifi(ssid, password),
            Some(format!("Connected to {joined}")),
            move |page| {
                page.state.borrow_mut().memory.drafts.remove(&name);
            },
        );
    }

    // ---- actions -------------------------------------------------------------------

    fn actions(self: &Rc<Self>) -> NetworkActions {
        let this = Rc::downgrade(self);
        NetworkActions {
            open: with1(&this, |page, target: NetPage| page.go(target)),
            back: with0(&this, |page| page.go_back()),
            open_properties: with0(&this, |page| page.go(NetPage::Properties)),

            set_wifi_enabled: with1(&this, |page, on: bool| {
                page.mutate_then(
                    move |api| api.set_wifi_enabled(on),
                    None,
                    move |page| {
                        if on {
                            page.state.borrow_mut().last_scan_us = 0;
                            page.request_scan(ScanReason::RadioEnabled, false);
                        }
                    },
                );
            }),
            select_wifi_adapter: with1(&this, |page, iface: String| {
                page.mutate_then(
                    move |api| api.select_wifi_adapter(iface),
                    None,
                    |page| page.request_scan(ScanReason::AdapterChange, false),
                );
            }),
            scan: with0(&this, |page| page.request_scan(ScanReason::Manual, true)),
            set_scan_policy: with2(&this, |page, policy: ScanPolicy, interval_ms: u32| {
                page.mutate(move |api| api.set_scan_policy(policy, interval_ms), None);
            }),
            connect_wifi: with2(&this, |page, ssid: String, password: Option<String>| {
                page.connect_wifi(ssid, password);
            }),
            activate_wifi: with1(&this, |page, id: String| {
                page.mutate(
                    move |api| api.activate_wifi_connection(id),
                    Some("Connected".into()),
                );
            }),
            disconnect_wifi: with0(&this, |page| {
                page.mutate(|api| api.disconnect_wifi(), Some("Disconnected".into()));
            }),
            set_query: with1(&this, |page, query: String| {
                {
                    let mut state = page.state.borrow_mut();
                    state.query = query;
                    state.wifi_saved_page = 1;
                    state.wifi_available_page = 1;
                }
                page.refresh_wifi_lists();
            }),
            set_wifi_saved_page: with1(&this, |page, p: usize| {
                page.state.borrow_mut().wifi_saved_page = p;
                page.refresh_wifi_lists();
            }),
            set_wifi_saved_page_size: with1(&this, |page, size: usize| {
                let mut state = page.state.borrow_mut();
                state.wifi_saved_page_size = size;
                state.wifi_saved_page = 1;
                drop(state);
                page.refresh_wifi_lists();
            }),
            set_wifi_available_page: with1(&this, |page, p: usize| {
                page.state.borrow_mut().wifi_available_page = p;
                page.refresh_wifi_lists();
            }),
            set_wifi_available_page_size: with1(&this, |page, size: usize| {
                let mut state = page.state.borrow_mut();
                state.wifi_available_page_size = size;
                state.wifi_available_page = 1;
                drop(state);
                page.refresh_wifi_lists();
            }),
            card_toggled: with2(&this, |page, key: String, open: bool| {
                let mut state = page.state.borrow_mut();
                if open {
                    state.memory.expanded.insert(key);
                } else {
                    state.memory.expanded.remove(&key);
                }
            }),
            set_draft: with2(&this, |page, ssid: String, text: String| {
                let mut state = page.state.borrow_mut();
                if text.is_empty() {
                    state.memory.drafts.remove(&ssid);
                } else {
                    state.memory.drafts.insert(ssid, text);
                }
            }),

            set_ethernet_enabled: with1(&this, |page, on: bool| {
                page.mutate(move |api| api.set_ethernet_enabled(on), None);
            }),
            select_ethernet_adapter: with1(&this, |page, iface: String| {
                page.mutate(move |api| api.select_ethernet_adapter(iface), None);
            }),
            set_autoconnect: with2(&this, |page, id: String, on: bool| {
                page.mutate(move |api| api.set_connection_autoconnect(id, on), None);
            }),
            set_metered: with2(&this, |page, id: String, on: bool| {
                page.mutate(move |api| api.set_connection_metered(id, on), None);
            }),
            set_mac_random: with2(&this, |page, id: String, on: bool| {
                page.mutate(move |api| api.set_connection_mac_random(id, on), None);
            }),
            forget: with1(&this, |page, id: String| page.forget(id)),
            share: with1(&this, |page, id: String| page.share(id)),

            open_vpn: with1(&this, |page, id: String| {
                page.state.borrow_mut().vpn_detail_id = Some(id);
                page.go(NetPage::VpnDetail);
            }),
            activate_vpn: with1(&this, |page, id: String| {
                page.mutate(
                    move |api| api.activate_vpn(id),
                    Some("VPN connected".into()),
                );
            }),
            deactivate_vpn: with1(&this, |page, id: String| {
                page.mutate(
                    move |api| api.deactivate_vpn(id),
                    Some("VPN disconnected".into()),
                );
            }),
            new_vpn: with0(&this, |page| page.new_vpn()),
            edit_vpn: with1(&this, |page, id: String| page.edit_vpn(id)),
            delete_vpn: with1(&this, |page, id: String| page.delete_vpn(id)),
            import_vpn: with0(&this, |page| page.import_vpn()),

            clear_log: with0(&this, |page| {
                page.call(
                    |api| api.clear_network_log(),
                    |page, ()| {
                        page.state.borrow_mut().keys.clear();
                        page.reload();
                    },
                );
            }),
        }
    }

    // ---- keyboard ------------------------------------------------------------------

    /// Escape goes back one page; Ctrl+F jumps to the Wi-Fi search.
    fn install_keys(self: &Rc<Self>) {
        let keys = gtk::EventControllerKey::new();
        let weak = self.this.clone();
        keys.connect_key_pressed(move |_, key, _, modifiers| {
            let Some(page) = weak.upgrade() else {
                return glib::Propagation::Proceed;
            };
            if key == gtk::gdk::Key::Escape && page.state.borrow().route != NetPage::Home {
                page.go_back();
                return glib::Propagation::Stop;
            }
            if key == gtk::gdk::Key::f
                && modifiers.contains(gtk::gdk::ModifierType::CONTROL_MASK)
                && page.state.borrow().route == NetPage::Wifi
                && let Some(entry) = find_search_entry(page.frame.wifi.upcast_ref())
            {
                entry.grab_focus();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        self.frame.root.add_controller(keys);
    }
}

enum Built {
    Home(Box<HomeProps>),
    Page(gtk::Widget),
    Wifi {
        widget: gtk::Widget,
        lists: WifiListSurfaces,
    },
}

fn with0(this: &Weak<Page>, run: fn(&Rc<Page>)) -> Rc<dyn Fn()> {
    let this = this.clone();
    Rc::new(move || {
        if let Some(page) = this.upgrade() {
            run(&page);
        }
    })
}

fn with1<A: 'static>(this: &Weak<Page>, run: fn(&Rc<Page>, A)) -> Rc<dyn Fn(A)> {
    let this = this.clone();
    Rc::new(move |a| {
        if let Some(page) = this.upgrade() {
            run(&page, a);
        }
    })
}

fn with2<A: 'static, B: 'static>(this: &Weak<Page>, run: fn(&Rc<Page>, A, B)) -> Rc<dyn Fn(A, B)> {
    let this = this.clone();
    Rc::new(move |a, b| {
        if let Some(page) = this.upgrade() {
            run(&page, a, b);
        }
    })
}

/// True while the user is typing in a field inside `root`.
fn focus_in_editable(root: &gtk::Widget) -> bool {
    root.root()
        .and_then(|window| gtk::prelude::RootExt::focus(&window))
        .is_some_and(|focus| {
            focus.is_ancestor(root) && focus.dynamic_cast_ref::<gtk::Editable>().is_some()
        })
}

/// Swaps a page's content; for a live update the scroll offset and the search field's
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

/// The page's scroller: the home page *is* the scroller's child, subpages end in one.
fn scroller_of(page: &gtk::Widget) -> Option<gtk::ScrolledWindow> {
    if let Some(scroller) = page.ancestor(gtk::ScrolledWindow::static_type()) {
        return scroller.downcast::<gtk::ScrolledWindow>().ok();
    }
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
