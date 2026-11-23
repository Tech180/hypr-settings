//! The hardware half of the display page: DDC/CI levels, brightness, HDR / 10-bit / VRR,
//! the night light, the main display and custom refresh rates. Probing the displays is
//! slow, so it runs after the page is shown and fills the rows in when it is done.

use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use gtk::glib;
use gtk4 as gtk;
use libadwaita as adw;
use settings_application::dto::{BrightnessView, NightLightView, OutputControls};
use settings_domain::ConnectorName;
use settings_domain::controls::{Flag, clamp_percent};
use settings_domain::night_light::Kelvin;

use super::Page;
use crate::features::display::constants::SLIDER_DEBOUNCE_MS;
use crate::features::display::utils::Hardware;

#[derive(Default)]
pub(super) struct HardwareState {
    pub controls: Option<OutputControls>,
    pub brightness: Option<BrightnessView>,
    pub night: Option<NightLightView>,
    /// Bumped per probe; an answer to an older probe is dropped.
    generation: u64,
    /// Sliders with a change that has not reached the hardware yet, and the latest ticket of
    /// each. The page is held still while any exist so a rebuild cannot snatch the knob.
    sliders: HashMap<&'static str, u64>,
}

impl HardwareState {
    pub fn view(&self) -> Hardware<'_> {
        Hardware {
            controls: self.controls.as_ref(),
            brightness: self.brightness.as_ref(),
            night: self.night.as_ref(),
        }
    }

    pub fn is_sliding(&self) -> bool {
        !self.sliders.is_empty()
    }
}

impl Page {
    /// Asks the hardware about the selected output and the global controls.
    pub(super) fn fetch_hardware(self: &Rc<Self>) {
        let Some(connector) = self.selected_connector() else {
            return;
        };
        let Ok(connector) = ConnectorName::try_from(connector.as_str()) else {
            return;
        };
        let generation = {
            let mut state = self.state.borrow_mut();
            state.hw.generation += 1;
            state.hw.generation
        };
        self.call(
            move |api| {
                (
                    api.output_controls(connector),
                    api.brightness(),
                    api.night_light(),
                )
            },
            move |page, (controls, brightness, night)| {
                let changed = {
                    let mut state = page.state.borrow_mut();
                    if state.hw.generation != generation {
                        return;
                    }
                    let hw = &mut state.hw;
                    let (controls, brightness, night) =
                        (controls.ok(), brightness.ok(), night.ok());
                    let changed =
                        hw.controls != controls || hw.brightness != brightness || hw.night != night;
                    hw.controls = controls;
                    hw.brightness = brightness;
                    hw.night = night;
                    changed
                };
                if changed {
                    page.render();
                }
            },
        );
    }

    /// Forgets what was learned about the previous selection.
    pub(super) fn reset_hardware(&self) {
        let mut state = self.state.borrow_mut();
        state.hw.controls = None;
        state.hw.generation += 1;
    }

    /// Runs `apply` once the slider has been still for a moment.
    fn debounced(self: &Rc<Self>, key: &'static str, apply: impl FnOnce(&Rc<Page>, u64) + 'static) {
        let ticket = {
            let mut state = self.state.borrow_mut();
            let ticket = state.hw.sliders.get(key).map_or(1, |t| t + 1);
            state.hw.sliders.insert(key, ticket);
            ticket
        };
        let this = Rc::downgrade(self);
        glib::timeout_add_local_once(Duration::from_millis(SLIDER_DEBOUNCE_MS), move || {
            let Some(page) = this.upgrade() else {
                return;
            };
            if page.state.borrow().hw.sliders.get(key) == Some(&ticket) {
                apply(&page, ticket);
            }
        });
    }

    /// The change reached the hardware; the page may be rebuilt again once nothing else moves.
    fn slider_done(self: &Rc<Self>, key: &'static str, ticket: u64) {
        let idle = {
            let mut state = self.state.borrow_mut();
            if state.hw.sliders.get(key) == Some(&ticket) {
                state.hw.sliders.remove(key);
            }
            state.hw.sliders.is_empty()
        };
        if idle {
            self.settled();
        }
    }

    pub(super) fn set_brightness(self: &Rc<Self>, value: f64) {
        let percent = clamp_percent(value.round() as i64);
        self.debounced("brightness", move |page, ticket| {
            page.call(
                move |api| api.set_brightness(percent),
                move |page, result| {
                    match result {
                        Ok(()) => {
                            if let Some(view) = page.state.borrow_mut().hw.brightness.as_mut() {
                                view.percent = Some(percent);
                            }
                        }
                        Err(err) => page.toast(&err.to_string()),
                    }
                    page.slider_done("brightness", ticket);
                },
            );
        });
    }

    pub(super) fn set_sync(self: &Rc<Self>, sync: bool) {
        self.call(
            move |api| api.set_sync_brightness(sync),
            move |page, result| {
                match result {
                    Ok(()) => {
                        if let Some(view) = page.state.borrow_mut().hw.brightness.as_mut() {
                            view.sync = sync;
                        }
                    }
                    Err(err) => page.toast(&err.to_string()),
                }
                page.render();
            },
        );
    }

    pub(super) fn set_contrast(self: &Rc<Self>, value: f64) {
        self.set_level("contrast", value, |api, connector, percent| {
            api.set_contrast(connector, percent)
        });
    }

    pub(super) fn set_overdrive(self: &Rc<Self>, value: f64) {
        self.set_level("overdrive", value, |api, connector, percent| {
            api.set_overdrive(connector, percent)
        });
    }

    fn set_level(
        self: &Rc<Self>,
        key: &'static str,
        value: f64,
        write: fn(
            &dyn settings_application::SettingsApi,
            ConnectorName,
            u8,
        ) -> Result<(), settings_application::AppError>,
    ) {
        let Some(connector) = self.connector_for_selected() else {
            return;
        };
        let percent = clamp_percent(value.round() as i64);
        self.debounced(key, move |page, ticket| {
            page.call(
                move |api| write(api, connector, percent),
                move |page, result| {
                    match result {
                        Ok(()) => {
                            if let Some(controls) = page.state.borrow_mut().hw.controls.as_mut() {
                                let level = match key {
                                    "contrast" => &mut controls.contrast,
                                    _ => &mut controls.overdrive,
                                };
                                level.percent = Some(percent);
                            }
                        }
                        Err(err) => page.toast(&err.to_string()),
                    }
                    page.slider_done(key, ticket);
                },
            );
        });
    }

    /// Switching input can blank the picture, so it is confirmed first.
    pub(super) fn set_input(self: &Rc<Self>, code: u8) {
        let Some(connector) = self.connector_for_selected() else {
            return;
        };
        let label = self
            .state
            .borrow()
            .hw
            .controls
            .as_ref()
            .and_then(|c| c.input.options.iter().find(|o| o.code == code))
            .map_or_else(|| format!("input 0x{code:02x}"), |o| o.label.clone());
        let dialog = adw::AlertDialog::builder()
            .heading(format!("Switch to {label}?"))
            .body("The display may stay blank until you switch back with its own buttons.")
            .close_response("cancel")
            .default_response("cancel")
            .build();
        dialog.add_responses(&[("cancel", "Cancel"), ("switch", "Switch")]);
        dialog.set_response_appearance("switch", adw::ResponseAppearance::Destructive);
        let this = Rc::downgrade(self);
        dialog.connect_response(None, move |_, response| {
            let Some(page) = this.upgrade() else {
                return;
            };
            if response != "switch" {
                // Put the dropdown back on the input that is still selected.
                return page.render();
            }
            let connector = connector.clone();
            page.call(
                move |api| api.set_input_source(connector, code),
                |page, result| {
                    if let Err(err) = result {
                        page.toast(&err.to_string());
                    }
                    page.fetch_hardware();
                },
            );
        });
        dialog.present(Some(&self.root));
    }

    pub(super) fn set_flag(self: &Rc<Self>, flag: Flag, enable: bool) {
        let Some(connector) = self.connector_for_selected() else {
            return;
        };
        self.mutate(
            move |api| api.set_flag(connector, flag, enable),
            move |page, result| match result {
                Ok(outcome) => page.note_outcome(outcome),
                Err(err) => page.toast(&err.to_string()),
            },
        );
    }

    pub(super) fn set_night(self: &Rc<Self>, on: bool) {
        self.call(
            move |api| api.set_night_light(on),
            move |page, result| {
                match result {
                    Ok(()) => {
                        if let Some(night) = page.state.borrow_mut().hw.night.as_mut() {
                            night.on = on;
                        }
                    }
                    Err(err) => page.toast(&err.to_string()),
                }
                page.render();
            },
        );
    }

    pub(super) fn set_kelvin(self: &Rc<Self>, value: f64) {
        let kelvin = Kelvin::clamped(value.round() as u32).get();
        self.debounced("kelvin", move |page, ticket| {
            page.call(
                move |api| api.set_night_kelvin(kelvin),
                move |page, result| {
                    match result {
                        Ok(()) => {
                            if let Some(night) = page.state.borrow_mut().hw.night.as_mut() {
                                night.kelvin = kelvin;
                            }
                        }
                        Err(err) => page.toast(&err.to_string()),
                    }
                    page.slider_done("kelvin", ticket);
                },
            );
        });
    }

    pub(super) fn set_main(self: &Rc<Self>) {
        let Some(connector) = self.connector_for_selected() else {
            return;
        };
        let name = connector.to_string();
        self.mutate(
            move |api| api.set_bar_host(connector),
            move |page, result| match result {
                Ok(()) => page.toast(&format!("{name} is the main display now")),
                Err(err) => page.toast(&err.to_string()),
            },
        );
    }

    pub(super) fn add_custom_hz(self: &Rc<Self>, hz: u32) {
        let Some(connector) = self.connector_for_selected() else {
            return;
        };
        self.mutate(
            move |api| api.add_custom_hz(connector, hz),
            |page, result| {
                if let Err(err) = result {
                    page.toast(&err.to_string());
                }
            },
        );
    }

    pub(super) fn remove_custom_hz(self: &Rc<Self>, hz: u32) {
        let Some(connector) = self.connector_for_selected() else {
            return;
        };
        self.mutate(
            move |api| api.remove_custom_hz(connector, hz),
            |page, result| {
                if let Err(err) = result {
                    page.toast(&err.to_string());
                }
            },
        );
    }

    pub(super) fn save_active(self: &Rc<Self>) {
        self.mutate(
            |api| api.save_to_active_profile(),
            |page, result| match result {
                Ok(name) => page.toast(&format!("Layout saved to {name}")),
                Err(err) => page.toast(&err.to_string()),
            },
        );
    }
}
