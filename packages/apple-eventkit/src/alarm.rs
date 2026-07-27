use objc2_event_kit::{EKAlarm, EKCalendarItem};
use objc2_foundation::NSArray;

use crate::{
    datetime::unix_to_ns_date,
    error::{EventKitError, EventKitResult},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlarmKind {
    Absolute,
    Relative,
}

#[derive(Debug, Clone)]
pub struct AlarmInput {
    pub kind: AlarmKind,
    pub at: Option<i64>,
    pub offset_seconds: Option<i64>,
}

pub fn apply_alarms_to_item(item: &EKCalendarItem, alarms: &[AlarmInput]) -> EventKitResult<()> {
    let ek_alarms: Vec<_> = alarms
        .iter()
        .map(build_alarm)
        .collect::<EventKitResult<_>>()?;
    let array = NSArray::from_retained_slice(&ek_alarms);
    unsafe { item.setAlarms(Some(&array)) };
    Ok(())
}

fn build_alarm(input: &AlarmInput) -> EventKitResult<objc2::rc::Retained<EKAlarm>> {
    match input.kind {
        AlarmKind::Absolute => {
            let at = input.at.ok_or_else(|| {
                EventKitError::ValidationFailed("absolute alarm requires at".into())
            })?;
            let date = unix_to_ns_date(at)?;
            Ok(unsafe { EKAlarm::alarmWithAbsoluteDate(&date) })
        }
        AlarmKind::Relative => {
            let offset = input.offset_seconds.ok_or_else(|| {
                EventKitError::ValidationFailed("relative alarm requires offset_seconds".into())
            })?;
            Ok(unsafe { EKAlarm::alarmWithRelativeOffset(offset as f64) })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_alarm_requires_offset() {
        let input = AlarmInput {
            kind: AlarmKind::Relative,
            at: None,
            offset_seconds: None,
        };
        assert!(build_alarm(&input).is_err());
    }
}
