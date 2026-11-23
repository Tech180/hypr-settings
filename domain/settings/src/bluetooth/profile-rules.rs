use super::profile::{ProfileKind, ProfileState};
use super::snapshot::ProfileSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileConflict {
    MediaBlocksPhone,
    PhoneBlocksMedia,
}

/// Applies a profile toggle and returns the updated snapshot or a conflict.
pub fn apply_profile_toggle(
    profiles: &ProfileSnapshot,
    kind: ProfileKind,
    enable: bool,
) -> Result<ProfileSnapshot, ProfileConflict> {
    let mut next = profiles.clone();
    match kind {
        ProfileKind::MediaAudio => {
            if enable && next.phone_calls.enabled {
                return Err(ProfileConflict::MediaBlocksPhone);
            }
            next.media_audio = ProfileState {
                available: next.media_audio.available,
                enabled: enable,
            };
        }
        ProfileKind::PhoneCalls => {
            if enable && next.media_audio.enabled {
                return Err(ProfileConflict::PhoneBlocksMedia);
            }
            next.phone_calls = ProfileState {
                available: next.phone_calls.available,
                enabled: enable,
            };
        }
        ProfileKind::ContactSharing => {
            next.contact_sharing = ProfileState {
                available: next.contact_sharing.available,
                enabled: enable,
            };
        }
        ProfileKind::AbsoluteVolume => {
            next.absolute_volume = ProfileState {
                available: next.absolute_volume.available,
                enabled: enable,
            };
        }
    }
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profiles(media: bool, phone: bool) -> ProfileSnapshot {
        ProfileSnapshot {
            media_audio: ProfileState {
                available: true,
                enabled: media,
            },
            phone_calls: ProfileState {
                available: true,
                enabled: phone,
            },
            contact_sharing: ProfileState::default(),
            absolute_volume: ProfileState::default(),
        }
    }

    #[test]
    fn media_and_phone_exclude_each_other() {
        let base = profiles(false, true);
        assert_eq!(
            apply_profile_toggle(&base, ProfileKind::MediaAudio, true),
            Err(ProfileConflict::MediaBlocksPhone)
        );
        let base = profiles(true, false);
        assert_eq!(
            apply_profile_toggle(&base, ProfileKind::PhoneCalls, true),
            Err(ProfileConflict::PhoneBlocksMedia)
        );
    }
}
