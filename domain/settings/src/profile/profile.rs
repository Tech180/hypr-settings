//! Kanshi profile model. The text format is an adapter concern; this module owns the
//! rules for matching, updating and snapshotting profiles.

use std::fmt;

use crate::error::DomainError;
use crate::layout::ResolvedPatch;
use crate::output::{Output, OutputIdentity, Topology};
use crate::profile::constants::{DEFAULT_PROFILE, HOME_PROFILE};
use crate::profile::interfaces::{
    Criteria, Profile, ProfileName, ProfileOutput, ProfileSet, UpsertOutcome,
};
use crate::profile::utils::{criteria_for, glob_match};
use crate::value::is_identifier;

impl ProfileName {
    pub fn parse(value: &str) -> Result<Self, DomainError> {
        if is_identifier(value) {
            Ok(Self(value.to_owned()))
        } else {
            Err(DomainError::InvalidProfileName(value.to_owned()))
        }
    }

    pub fn from_stored(value: &str) -> Result<Self, DomainError> {
        let ok = !value.is_empty()
            && !value
                .chars()
                .any(|c| c.is_whitespace() || matches!(c, '{' | '}' | '"'));
        if ok {
            Ok(Self(value.to_owned()))
        } else {
            Err(DomainError::InvalidProfileName(value.to_owned()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_default(&self) -> bool {
        self.0 == DEFAULT_PROFILE
    }
}

impl TryFrom<String> for ProfileName {
    type Error = DomainError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_stored(&value)
    }
}

impl From<ProfileName> for String {
    fn from(value: ProfileName) -> Self {
        value.0
    }
}

impl fmt::Display for ProfileName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Criteria {
    pub fn parse(value: &str) -> Self {
        if value == "*" {
            Self::Any
        } else {
            Self::Named(value.to_owned())
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Named(name) => name,
            Self::Any => "*",
        }
    }

    pub fn matches(&self, head: &OutputIdentity) -> bool {
        match self {
            Self::Any => true,
            Self::Named(text) => {
                text == head.connector.as_str()
                    || (text.contains(' ')
                        && head.descriptions().iter().any(|d| glob_match(text, d)))
            }
        }
    }
}

impl ProfileOutput {
    pub fn named(&self) -> Option<&str> {
        match (&self.criteria, self.rest_glob) {
            (Criteria::Named(name), false) => Some(name),
            _ => None,
        }
    }

    pub fn from_output(output: &Output, patch: Option<&ResolvedPatch>) -> Option<Self> {
        let own = patch.filter(|p| p.connector == output.connector);
        let mode = own.and_then(|p| p.mode).or_else(|| output.persisted_mode());
        let enable = own.and_then(|p| p.enable).unwrap_or(output.enabled);
        if mode.is_none() && enable {
            return None;
        }
        let position = patch
            .and_then(|p| p.position_of(&output.connector))
            .unwrap_or(output.position);
        Some(Self {
            criteria: Criteria::Named(output.connector.to_string()),
            enable: Some(enable),
            mode,
            position: Some(position),
            scale: Some(own.and_then(|p| p.scale).unwrap_or(output.scale)),
            transform: Some(own.and_then(|p| p.transform).unwrap_or(output.transform)),
            rest_glob: false,
            extra: Vec::new(),
        })
    }
}

/// Result of matching connected heads against one profile: for each head, the
/// index of the profile output that configures it.
type HeadSlots = Vec<usize>;

impl Profile {
    pub fn new(name: ProfileName) -> Self {
        Self {
            name,
            outputs: Vec::new(),
            extra_lines: Vec::new(),
        }
    }

    pub fn named_connectors(&self) -> Vec<&str> {
        self.outputs
            .iter()
            .filter_map(ProfileOutput::named)
            .collect()
    }

    pub fn has_rest_glob(&self) -> bool {
        self.outputs.iter().any(|o| o.rest_glob)
    }

    /// Kanshi's greedy assignment: every non-rest output, named ones first and
    /// `*` last, takes the first still-free head it matches. Returns head -> slot
    /// (unmatched heads are `None`) and whether every output found a head.
    fn greedy(&self, heads: &[OutputIdentity]) -> (Vec<Option<usize>>, bool) {
        let mut head_slot: Vec<Option<usize>> = vec![None; heads.len()];
        let mut order: Vec<usize> = (0..self.outputs.len())
            .filter(|&i| !self.outputs[i].rest_glob)
            .collect();
        order.sort_by_key(|&i| matches!(self.outputs[i].criteria, Criteria::Any));
        let mut all_found = true;
        for slot in order {
            let free = (0..heads.len()).find(|&h| {
                head_slot[h].is_none() && self.outputs[slot].criteria.matches(&heads[h])
            });
            match free {
                Some(h) => head_slot[h] = Some(slot),
                None => all_found = false,
            }
        }
        (head_slot, all_found)
    }

    /// The heads this profile would configure, or `None` when kanshi would not
    /// select it: every output must find a distinct head and no head may be left
    /// over, unless a `...output` absorbs the leftovers (at least one of them).
    pub fn assign(&self, heads: &[OutputIdentity]) -> Option<HeadSlots> {
        if heads.is_empty() {
            return None;
        }
        let (head_slot, all_found) = self.greedy(heads);
        if !all_found {
            return None;
        }
        let rest = self.outputs.iter().position(|o| o.rest_glob);
        let mut slots = Vec::with_capacity(heads.len());
        let mut leftover = 0;
        for (h, slot) in head_slot.iter().enumerate() {
            match (slot, rest) {
                (Some(s), _) => slots.push(*s),
                (None, Some(r)) if self.outputs[r].criteria.matches(&heads[h]) => {
                    leftover += 1;
                    slots.push(r);
                }
                (None, _) => return None,
            }
        }
        if rest.is_some() && leftover == 0 {
            return None;
        }
        Some(slots)
    }
}

/// Drops `key value` pairs that a rewrite is about to set again.
fn carried_extra(old: &[String]) -> Vec<String> {
    let mut kept = Vec::new();
    let mut i = 0;
    while i < old.len() {
        if matches!(old[i].as_str(), "mode" | "transform" | "position" | "scale")
            && i + 1 < old.len()
        {
            i += if old[i + 1].starts_with("--") { 3 } else { 2 };
        } else {
            kept.push(old[i].clone());
            i += 1;
        }
    }
    kept
}

/// A disabled output has no place in the layout; keep the position it had.
fn keep_saved_position(next: &mut ProfileOutput, slot: &ProfileOutput) {
    if next.enable == Some(false)
        && let Some(saved) = slot.position
    {
        next.position = Some(saved);
    }
}

impl ProfileSet {
    pub fn new(profiles: Vec<Profile>) -> Self {
        Self {
            profiles,
            preamble: Vec::new(),
        }
    }

    pub fn get(&self, name: &ProfileName) -> Option<&Profile> {
        self.profiles.iter().find(|p| &p.name == name)
    }

    /// Index of the named profile, inserting it before `default` when missing.
    fn ensure(&mut self, name: &ProfileName) -> usize {
        if let Some(i) = self.profiles.iter().position(|p| &p.name == name) {
            return i;
        }
        let at = self
            .profiles
            .iter()
            .position(|p| p.name.is_default())
            .unwrap_or(self.profiles.len());
        self.profiles.insert(at, Profile::new(name.clone()));
        at
    }

    /// The profile kanshi selects: the first one in file order that matches the
    /// connected heads. `default` is the bootstrap fallback and is never reported.
    pub fn match_active(&self, heads: &[OutputIdentity]) -> Option<ProfileName> {
        self.profiles
            .iter()
            .filter(|p| !p.name.is_default())
            .find(|p| p.assign(heads).is_some())
            .map(|p| p.name.clone())
    }

    /// Output slots of a profile with the connected output each configures.
    pub fn slots_for<'a>(
        &self,
        name: &ProfileName,
        topology: &'a Topology,
    ) -> Vec<(&ProfileOutput, Option<&'a Output>)> {
        let Some(profile) = self.get(name) else {
            return Vec::new();
        };
        let heads = topology.identities();
        let (head_slot, _) = profile.greedy(&heads);
        profile
            .outputs
            .iter()
            .enumerate()
            .map(|(i, slot)| {
                let head = head_slot.iter().position(|s| *s == Some(i));
                (slot, head.map(|h| &topology.outputs()[h]))
            })
            .collect()
    }

    fn unused_name(&self, base: &ProfileName, start: usize) -> ProfileName {
        (start..)
            .map(|n| format!("{base}-{n}"))
            .find(|candidate| !self.profiles.iter().any(|p| p.name.as_str() == candidate))
            .and_then(|candidate| ProfileName::from_stored(&candidate).ok())
            .unwrap_or_else(|| base.clone())
    }

    /// Update the profile that kanshi will apply so it matches the live layout
    /// plus `patch`. Every output the profile lists by name gets its resolved
    /// position. Outputs absorbed by a catch-all (`...output "*"`) cannot be
    /// stored there, so an edit that touches one, or moves anything, goes into a
    /// new specific profile placed ahead of the catch-all.
    pub fn upsert_layout(
        &mut self,
        active: &ProfileName,
        topology: &Topology,
        patch: Option<&ResolvedPatch>,
    ) -> UpsertOutcome {
        let heads = topology.identities();
        let outputs = topology.outputs();
        let index = self.ensure(active);
        let (head_slot, _) = self.profiles[index].greedy(&heads);
        let has_rest = self.profiles[index].has_rest_glob();
        let absorbed: Vec<usize> = if has_rest {
            (0..heads.len())
                .filter(|&h| head_slot[h].is_none())
                .collect()
        } else {
            Vec::new()
        };

        if let Some(patch) = patch
            && !absorbed.is_empty()
        {
            let touches = absorbed
                .iter()
                .any(|&h| outputs[h].connector == patch.connector);
            let moves = outputs.iter().filter(|o| o.enabled).any(|o| {
                patch
                    .position_of(&o.connector)
                    .is_some_and(|p| p != o.position)
            });
            if touches || moves {
                return self.promote(index, topology, patch);
            }
        }

        let mut patch_ok = true;
        let profile = &mut self.profiles[index];
        for (slot_index, slot) in profile.outputs.iter_mut().enumerate() {
            if slot.rest_glob {
                continue;
            }
            let Some(h) = head_slot.iter().position(|s| *s == Some(slot_index)) else {
                continue; // stale entry for a display that is not connected
            };
            let live = &outputs[h];
            let is_patch = patch.is_some_and(|p| p.connector == live.connector);
            match ProfileOutput::from_output(live, patch) {
                Some(mut next) => {
                    next.criteria = slot.criteria.clone();
                    next.extra = carried_extra(&slot.extra);
                    keep_saved_position(&mut next, slot);
                    *slot = next;
                }
                None => {
                    // Hyprland often drops mode geometry while disabled; keep the prior mode.
                    let re_enable = patch.is_some_and(|p| p.enable == Some(true));
                    if is_patch && re_enable && slot.mode.is_some() {
                        slot.enable = Some(true);
                        slot.position = patch
                            .and_then(|p| p.position_of(&live.connector))
                            .or(slot.position);
                    } else if is_patch {
                        patch_ok = false;
                    }
                }
            }
        }

        if let Some(patch) = patch
            && !has_rest
            && patch.enable == Some(true)
            && let Some(h) = outputs.iter().position(|o| o.connector == patch.connector)
            && head_slot[h].is_none()
            && let Some(mut next) = ProfileOutput::from_output(&outputs[h], Some(patch))
        {
            next.criteria = criteria_for(&outputs[h], &heads);
            profile.outputs.push(next);
        }

        UpsertOutcome {
            stored: patch_ok,
            profile: profile.name.clone(),
        }
    }

    /// Writes the whole live topology into a new profile ahead of the catch-all at
    /// `index`, which stays untouched.
    fn promote(
        &mut self,
        index: usize,
        topology: &Topology,
        patch: &ResolvedPatch,
    ) -> UpsertOutcome {
        let heads = topology.identities();
        let base = self.profiles[index].clone();
        let (head_slot, _) = base.greedy(&heads);
        let rest = base.outputs.iter().find(|o| o.rest_glob);

        let mut profile = Profile::new(self.unused_name(&base.name, topology.outputs().len()));
        profile.extra_lines = base.extra_lines.clone();
        let mut stored = true;

        for (h, live) in topology.outputs().iter().enumerate() {
            let slot = head_slot[h].map(|s| &base.outputs[s]).or(rest);
            let is_patch = live.connector == patch.connector;
            let mut next = match ProfileOutput::from_output(live, Some(patch)) {
                Some(next) => next,
                None if is_patch
                    && patch.enable == Some(true)
                    && slot.is_some_and(|s| s.mode.is_some()) =>
                {
                    // Re-enabled output with no live mode: fall back to the slot's.
                    let mut next = ProfileOutput::from_output(
                        &Output {
                            mode: slot.and_then(|s| s.mode),
                            ..live.clone()
                        },
                        Some(patch),
                    )
                    .expect("mode supplied");
                    next.mode = slot.and_then(|s| s.mode);
                    next
                }
                None => {
                    stored = false;
                    continue;
                }
            };
            next.criteria = match head_slot[h] {
                Some(s) => base.outputs[s].criteria.clone(),
                None => criteria_for(live, &heads),
            };
            next.extra = slot.map(|s| carried_extra(&s.extra)).unwrap_or_default();
            if let Some(slot) = slot {
                keep_saved_position(&mut next, slot);
            }
            profile.outputs.push(next);
        }

        let name = profile.name.clone();
        self.profiles.insert(index, profile);
        UpsertOutcome {
            stored,
            profile: name,
        }
    }

    /// Replace `home` with the full live topology (used when no profile matched).
    pub fn upsert_home_full_snapshot(
        &mut self,
        topology: &Topology,
        patch: Option<&ResolvedPatch>,
    ) -> bool {
        let name = ProfileName::from_stored(HOME_PROFILE).expect("static name is valid");
        let index = self.ensure(&name);
        let heads = topology.identities();
        self.profiles[index].outputs = topology
            .outputs()
            .iter()
            .filter_map(|o| {
                let mut out = ProfileOutput::from_output(o, patch)?;
                out.criteria = criteria_for(o, &heads);
                Some(out)
            })
            .collect();
        true
    }

    /// Overwrite a named profile with the live topology. Catch-all profiles only
    /// refresh the outputs they list by name, so `...output "*"` survives.
    pub fn snapshot_to_profile(&mut self, name: &ProfileName, topology: &Topology) -> bool {
        let heads = topology.identities();
        let index = self.ensure(name);
        let profile = &mut self.profiles[index];

        if profile.has_rest_glob() {
            let (head_slot, _) = profile.greedy(&heads);
            let mut any = false;
            for (slot_index, slot) in profile.outputs.iter_mut().enumerate() {
                let Some(live) = head_slot
                    .iter()
                    .position(|s| *s == Some(slot_index))
                    .map(|h| &topology.outputs()[h])
                else {
                    continue;
                };
                if let Some(mut next) = ProfileOutput::from_output(live, None) {
                    next.criteria = slot.criteria.clone();
                    next.extra = carried_extra(&slot.extra);
                    *slot = next;
                    any = true;
                }
            }
            return any;
        }

        let outputs: Vec<_> = topology
            .outputs()
            .iter()
            .filter_map(|o| {
                let mut out = ProfileOutput::from_output(o, None)?;
                out.criteria = criteria_for(o, &heads);
                Some(out)
            })
            .collect();
        if outputs.is_empty() {
            return false;
        }
        profile.outputs = outputs;
        true
    }
}
