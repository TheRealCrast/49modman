use serde::{Deserialize, Serialize};
use time::{macros::format_description, Date};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum BaseZone {
    Orange,
    Green,
    Yellow,
    Red,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReferenceState {
    Verified,
    Broken,
    Neutral,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum EffectiveStatus {
    Broken,
    Verified,
    Green,
    Yellow,
    Orange,
    Red,
}

pub fn classify_base_zone(published_at: &str) -> BaseZone {
    let date = Date::parse(published_at, format_description!("[year]-[month]-[day]")).ok();

    match date {
        Some(date) if date < Date::from_calendar_date(2023, time::Month::December, 9).unwrap() => {
            BaseZone::Orange
        }
        Some(date) if date < Date::from_calendar_date(2024, time::Month::March, 31).unwrap() => {
            BaseZone::Green
        }
        Some(date) if date < Date::from_calendar_date(2024, time::Month::April, 13).unwrap() => {
            BaseZone::Yellow
        }
        _ => BaseZone::Red,
    }
}

pub fn resolve_effective_status(
    base_zone: BaseZone,
    bundled_reference_state: Option<ReferenceState>,
    override_reference_state: Option<ReferenceState>,
) -> EffectiveStatus {
    match override_reference_state.or(bundled_reference_state) {
        Some(ReferenceState::Broken) => EffectiveStatus::Broken,
        Some(ReferenceState::Verified) => EffectiveStatus::Verified,
        Some(ReferenceState::Neutral) | None => match base_zone {
            BaseZone::Orange => EffectiveStatus::Orange,
            BaseZone::Green => EffectiveStatus::Green,
            BaseZone::Yellow => EffectiveStatus::Yellow,
            BaseZone::Red => EffectiveStatus::Red,
        },
    }
}

pub fn parse_reference_state(value: &str) -> Option<ReferenceState> {
    match value {
        "verified" => Some(ReferenceState::Verified),
        "broken" => Some(ReferenceState::Broken),
        "neutral" => Some(ReferenceState::Neutral),
        _ => None,
    }
}
