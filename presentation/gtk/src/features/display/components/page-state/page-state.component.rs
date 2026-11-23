//! Display page widget shell, view state, and layout/profile actions.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use adw::prelude::*;
use gtk::glib;
use gtk4 as gtk;
use libadwaita as adw;
use settings_application::dto::{ApplyOutcome, DisplayView, TickOutcome};
use settings_application::{AppError, SettingsApi};
use settings_domain::controls::Flag;
use settings_domain::{
    ConnectorName, LayoutPatch, ModeRequest, Position, ProfileName, Resolution, Scale, Transform,
};

use crate::app::blocking;
use crate::features::display::components::arrangement_canvas::ArrangementCanvas;
use crate::features::display::components::profile_detail;
use crate::features::display::components::trial_pill::{TrialPill, TrialPillProps};
use crate::features::display::constants::{
    APPLIED_DIRECTLY_MESSAGE, EXPIRED_MESSAGE, TICK_SECS, UNVERIFIED_MESSAGE,
};
use crate::features::display::html;
use crate::features::display::interfaces::DisplayActions;
use crate::features::display::utils::{
    bind, page_props, profile_detail_props, resolve_selection, trial_message,
};
use crate::features::settings_shell::interfaces::{Section, ShellContext};

pub(crate) struct Page {
    this: Weak<Page>,
    ctx: ShellContext,
    root: gtk::Overlay,
    stack: gtk::Stack,
    scroller: gtk::ScrolledWindow,
    body: adw::Bin,
    content: gtk::Widget,
    notice: gtk::Label,
    rest: adw::Bin,
    detail: adw::Bin,
    canvas: ArrangementCanvas,
    pill: TrialPill,
    state: RefCell<State>,
}

#[derive(Default)]
pub(super) struct State {
    view: Option<DisplayView>,
    selected: Option<String>,
    /// The profile whose detail page is open.
    profile: Option<String>,
    ticker: Option<glib::SourceId>,
    /// Bumped per reload; an answer to an older reload is dropped.
    generation: u64,
    /// A change is being applied. Further changes, ticks and event reloads wait for it.
    busy: bool,
    /// A render was held back while a tile is being dragged.
    render_pending: bool,
    hw: hardware::HardwareState,
}

impl Section for Page {
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }

    fn refresh(&self) {
        // Topology events caused by our own apply arrive mid-change; the change reloads
        // once it finishes, so they are dropped here.
        if self.state.borrow().busy {
            return;
        }
        if let Some(page) = self.this.upgrade() {
            page.reload();
        }
    }

    fn on_close(&self, proceed: Box<dyn FnOnce()>) {
        let Some(page) = self.this.upgrade() else {
            return proceed();
        };
        if page
            .state
            .borrow()
            .view
            .as_ref()
            .is_none_or(|v| v.trial.is_none())
        {
            return proceed();
        }
        page.call(
            |api| api.revert_trial(),
            move |page, result| {
                if let Err(err) = result {
                    page.toast(&format!("Could not restore the previous layout: {err}"));
                }
                proceed();
            },
        );
    }
}

impl Page {
    pub(crate) fn new(ctx: ShellContext, this: &Weak<Page>) -> Self {
        let pill = TrialPill::new(&TrialPillProps {
            on_keep: Self::action(this, Page::keep),
            on_revert: Self::action(this, Page::revert),
        });
        let canvas = ArrangementCanvas::new({
            let this = this.clone();
            Rc::new(move || {
                if let Some(page) = this.upgrade() {
                    page.settled();
                }
            })
        });
        let frame = html::build_frame(&pill.widget(), &canvas.widget());
        Self {
            this: this.clone(),
            ctx,
            root: frame.root,
            stack: frame.stack,
            scroller: frame.scroller,
            body: frame.body,
            content: frame.content,
            notice: frame.notice,
            rest: frame.rest,
            detail: frame.detail,
            canvas,
            pill,
            state: RefCell::default(),
        }
    }

    fn action(this: &Weak<Page>, run: fn(&Rc<Page>)) -> Rc<dyn Fn()> {
        let this = this.clone();
        Rc::new(move || {
            if let Some(page) = this.upgrade() {
                run(&page);
            }
        })
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

    /// Runs a change: one at a time, with the page held still until it is done and reloaded.
    fn mutate<T: Send + 'static>(
        &self,
        work: impl FnOnce(&dyn SettingsApi) -> T + Send + 'static,
        done: impl FnOnce(&Rc<Page>, T) + 'static,
    ) {
        if self.state.borrow().busy {
            // Re-render so a canvas waiting on this change returns to the real layout.
            if let Some(page) = self.this.upgrade() {
                page.render();
            }
            return;
        }
        self.state.borrow_mut().busy = true;
        self.sync_busy();
        self.call(work, move |page, value| {
            page.state.borrow_mut().busy = false;
            done(page, value);
            page.reload();
        });
    }

    fn sync_busy(&self) {
        let state = self.state.borrow();
        let trial = state.view.as_ref().is_some_and(|v| v.trial.is_some());
        self.pill.set_busy(state.busy);
        self.content.set_sensitive(!state.busy && !trial);
    }

    pub(crate) fn reload(self: &Rc<Self>) {
        let generation = {
            let mut state = self.state.borrow_mut();
            state.generation += 1;
            state.generation
        };
        self.call(
            |api| api.view(),
            move |page, result| {
                if page.state.borrow().generation != generation {
                    return;
                }
                match result {
                    Ok(view) => page.apply(view),
                    Err(err) => page.toast(&format!("Could not read displays: {err}")),
                }
            },
        );
    }

    /// A canvas interaction ended without a drop: catch up on anything held back.
    fn settled(self: &Rc<Self>) {
        let pending = std::mem::take(&mut self.state.borrow_mut().render_pending);
        if pending {
            self.render();
        }
    }

    fn apply(self: &Rc<Self>, view: DisplayView) {
        {
            let mut state = self.state.borrow_mut();
            state.selected = resolve_selection(&view, state.selected.as_deref());
            state.view = Some(view);
        }
        self.render();
        self.fetch_hardware();
    }

    fn render(self: &Rc<Self>) {
        // Rebuilding under a drag in progress would yank the tiles from under the pointer.
        if self.canvas.is_dragging() || self.state.borrow().hw.is_sliding() {
            self.state.borrow_mut().render_pending = true;
            return;
        }
        self.state.borrow_mut().render_pending = false;
        let (props, detail, trial) = {
            let state = self.state.borrow();
            let Some(view) = state.view.as_ref() else {
                return;
            };
            let actions = self.actions();
            let props = state
                .selected
                .as_deref()
                .and_then(|selected| page_props(view, selected, state.hw.view(), &actions));
            let detail = state
                .profile
                .as_deref()
                .and_then(|name| profile_detail_props(view, name, &actions));
            (props, detail, view.trial.clone())
        };

        self.pill
            .set_message(trial.as_ref().map(trial_message).as_deref());
        self.sync_busy();

        let offset = self.scroller.vadjustment().value();
        match &props {
            Some(props) => {
                if self.body.child().as_ref() != Some(&self.content) {
                    self.body.set_child(Some(&self.content));
                }
                self.canvas.set_props(&props.arrangement);
                self.notice
                    .set_label(props.notice.as_deref().unwrap_or_default());
                self.notice.set_visible(props.notice.is_some());
                self.rest.set_child(Some(&html::build_rest(props)));
            }
            None => self.body.set_child(Some(&html::build_empty())),
        }
        let scroller = self.scroller.clone();
        glib::idle_add_local_once(move || scroller.vadjustment().set_value(offset));

        match &detail {
            Some(detail) => {
                self.detail.set_child(Some(&profile_detail::build(detail)));
                self.stack.set_visible_child_name("profile");
            }
            None => {
                self.state.borrow_mut().profile = None;
                self.stack.set_visible_child_name("main");
                if self.detail.child().is_some() {
                    // Dropped after the slide so the page does not vanish mid-transition.
                    let slot = self.detail.clone();
                    glib::timeout_add_local_once(
                        std::time::Duration::from_millis(500),
                        move || {
                            slot.set_child(gtk::Widget::NONE);
                        },
                    );
                }
            }
        }

        self.sync_ticker(trial.is_some());
    }

    fn sync_ticker(self: &Rc<Self>, active: bool) {
        let mut state = self.state.borrow_mut();
        match (active, state.ticker.is_some()) {
            (true, false) => {
                let this = Rc::downgrade(self);
                state.ticker = Some(glib::timeout_add_seconds_local(
                    TICK_SECS,
                    move || match this.upgrade() {
                        Some(page) => {
                            page.tick();
                            glib::ControlFlow::Continue
                        }
                        None => glib::ControlFlow::Break,
                    },
                ));
            }
            (false, true) => {
                if let Some(id) = state.ticker.take() {
                    id.remove();
                }
            }
            _ => {}
        }
    }

    fn tick(self: &Rc<Self>) {
        // Keep or Revert is in flight; its reload settles the pill.
        if self.state.borrow().busy {
            return;
        }
        self.call(
            |api| api.tick(),
            |page, result| match result {
                Ok(TickOutcome::Active(trial)) => {
                    page.pill.set_message(Some(&trial_message(&trial)))
                }
                Ok(TickOutcome::Idle) => page.reload(),
                Ok(TickOutcome::Expired) => {
                    page.toast(EXPIRED_MESSAGE);
                    page.reload();
                }
                Err(err) => page.toast(&err.to_string()),
            },
        );
    }

    fn actions(self: &Rc<Self>) -> DisplayActions {
        let this = Rc::downgrade(self);
        DisplayActions {
            select_output: bind(&this, Page::select),
            move_output: {
                let this = this.clone();
                Rc::new(move |connector, x, y| {
                    if let Some(page) = this.upgrade() {
                        page.move_output(&connector, x, y);
                    }
                })
            },
            set_resolution: {
                let this = this.clone();
                Rc::new(move |w, h| {
                    if let Some(page) = this.upgrade() {
                        page.set_resolution(w, h);
                    }
                })
            },
            set_refresh: bind(&this, Page::set_refresh),
            set_scale: bind(&this, Page::set_scale),
            set_transform: bind(&this, Page::set_transform),
            set_enabled: bind(&this, Page::set_enabled),
            open_profile: bind(&this, Page::open_profile),
            close_profile: {
                let this = this.clone();
                Rc::new(move || {
                    if let Some(page) = this.upgrade() {
                        page.close_profile();
                    }
                })
            },
            apply_profile: bind(&this, Page::switch_profile),
            delete_profile: bind(&this, Page::delete_profile),
            save_profile_as: bind(&this, Page::save_profile_as),
            save_active: {
                let this = this.clone();
                Rc::new(move || {
                    if let Some(page) = this.upgrade() {
                        page.save_active();
                    }
                })
            },
            set_main: {
                let this = this.clone();
                Rc::new(move || {
                    if let Some(page) = this.upgrade() {
                        page.set_main();
                    }
                })
            },
            add_custom_hz: bind(&this, Page::add_custom_hz),
            remove_custom_hz: bind(&this, Page::remove_custom_hz),
            set_brightness: bind(&this, Page::set_brightness),
            set_sync: bind(&this, Page::set_sync),
            set_contrast: bind(&this, Page::set_contrast),
            set_overdrive: bind(&this, Page::set_overdrive),
            set_input: bind(&this, Page::set_input),
            set_flag: {
                let this = this.clone();
                Rc::new(move |flag: Flag, enable| {
                    if let Some(page) = this.upgrade() {
                        page.set_flag(flag, enable);
                    }
                })
            },
            set_night: bind(&this, Page::set_night),
            set_kelvin: bind(&this, Page::set_kelvin),
        }
    }

    fn selected_connector(&self) -> Option<String> {
        self.state.borrow().selected.clone()
    }

    fn select(self: &Rc<Self>, connector: String) {
        if self.selected_connector().as_deref() == Some(connector.as_str()) {
            return;
        }
        self.state.borrow_mut().selected = Some(connector);
        self.reset_hardware();
        self.render();
        self.fetch_hardware();
    }

    fn move_output(self: &Rc<Self>, connector: &str, x: i32, y: i32) {
        let connector = match ConnectorName::try_from(connector) {
            Ok(connector) => connector,
            Err(err) => return self.toast(&err.to_string()),
        };
        self.start_trial(LayoutPatch::new(connector).position(Position::new(x, y)));
    }

    fn open_profile(self: &Rc<Self>, name: String) {
        self.state.borrow_mut().profile = Some(name);
        self.render();
    }

    fn close_profile(self: &Rc<Self>) {
        self.state.borrow_mut().profile = None;
        self.render();
    }

    fn delete_profile(self: &Rc<Self>, name: String) {
        let profile = match ProfileName::from_stored(&name) {
            Ok(profile) => profile,
            Err(err) => return self.toast(&err.to_string()),
        };
        self.mutate(
            move |api| api.delete_profile(profile),
            move |page, result| match result {
                Ok(()) => {
                    page.state.borrow_mut().profile = None;
                    page.toast(&format!("Deleted profile {name}"));
                }
                Err(err) => page.toast(&err.to_string()),
            },
        );
    }

    fn set_resolution(self: &Rc<Self>, width: u32, height: u32) {
        self.begin_trial(|patch| {
            Ok(patch.mode(ModeRequest::Resolution(Resolution::new(width, height))))
        });
    }

    fn set_refresh(self: &Rc<Self>, hz: u32) {
        self.begin_trial(|patch| Ok(patch.mode(ModeRequest::Refresh(hz))));
    }

    fn set_scale(self: &Rc<Self>, hundredths: u16) {
        self.begin_trial(|patch| {
            let scale = Scale::new(f64::from(hundredths) / 100.0).map_err(|e| e.to_string())?;
            Ok(patch.scale(scale))
        });
    }

    fn set_transform(self: &Rc<Self>, hypr: i32) {
        self.begin_trial(|patch| Ok(patch.transform(Transform::from_hypr(hypr))));
    }

    fn begin_trial(
        self: &Rc<Self>,
        build: impl FnOnce(LayoutPatch) -> Result<LayoutPatch, String>,
    ) {
        let Some(patch) = self.patch_for_selected(build) else {
            return;
        };
        self.start_trial(patch);
    }

    fn start_trial(self: &Rc<Self>, patch: LayoutPatch) {
        self.mutate(
            move |api| api.begin_trial(patch),
            |page, result| match result {
                Ok(trial) => page.note_outcome(trial.outcome),
                Err(err) => page.toast(&err.to_string()),
            },
        );
    }

    fn set_enabled(self: &Rc<Self>, enable: bool) {
        let Some(connector) = self.connector_for_selected() else {
            return;
        };
        self.mutate(
            move |api| api.set_enabled(connector, enable),
            |page, result| match result {
                Ok(outcome) => page.note_outcome(outcome),
                Err(err) => page.toast(&err.to_string()),
            },
        );
    }

    fn keep(self: &Rc<Self>) {
        self.mutate(
            |api| api.keep_trial(),
            |page, result| {
                match result {
                    Ok(()) => page.toast("Layout kept"),
                    // The trial already ended (expired or reverted); the reload shows that.
                    Err(AppError::Rejected(_)) => {}
                    Err(err) => page.toast(&err.to_string()),
                }
            },
        );
    }

    fn revert(self: &Rc<Self>) {
        self.mutate(
            |api| api.revert_trial(),
            |page, result| match result {
                Ok(()) => page.toast("Previous layout restored"),
                Err(AppError::Rejected(_)) => {}
                Err(err) => page.toast(&err.to_string()),
            },
        );
    }

    fn switch_profile(self: &Rc<Self>, name: String) {
        let profile = match ProfileName::from_stored(&name) {
            Ok(profile) => profile,
            Err(err) => return self.toast(&err.to_string()),
        };
        self.mutate(
            move |api| api.switch_profile(profile),
            move |page, result| match result {
                Ok(()) => page.toast(&format!("Switched to {name}")),
                Err(err) => page.toast(&err.to_string()),
            },
        );
    }

    fn save_profile_as(self: &Rc<Self>, name: String) {
        let profile = match ProfileName::parse(&name) {
            Ok(profile) => profile,
            Err(err) => return self.toast(&err.to_string()),
        };
        self.mutate(
            move |api| api.save_snapshot_as(profile),
            move |page, result| match result {
                Ok(()) => page.toast(&format!("Saved profile {name}")),
                Err(err) => page.toast(&err.to_string()),
            },
        );
    }

    fn connector_for_selected(&self) -> Option<ConnectorName> {
        let selected = self.selected_connector()?;
        match ConnectorName::try_from(selected.as_str()) {
            Ok(connector) => Some(connector),
            Err(err) => {
                self.toast(&err.to_string());
                None
            }
        }
    }

    fn patch_for_selected(
        &self,
        build: impl FnOnce(LayoutPatch) -> Result<LayoutPatch, String>,
    ) -> Option<LayoutPatch> {
        let connector = self.connector_for_selected()?;
        match build(LayoutPatch::new(connector)) {
            Ok(patch) => Some(patch),
            Err(message) => {
                self.toast(&message);
                None
            }
        }
    }

    fn note_outcome(&self, outcome: ApplyOutcome) {
        match outcome {
            ApplyOutcome::Verified => {}
            ApplyOutcome::Unverified => self.toast(UNVERIFIED_MESSAGE),
            ApplyOutcome::Degraded => self.toast(APPLIED_DIRECTLY_MESSAGE),
        }
    }
}

#[path = "page-hardware.component.rs"]
mod hardware;
