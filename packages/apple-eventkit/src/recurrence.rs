use objc2::AnyThread;
use objc2_event_kit::{EKCalendarItem, EKRecurrenceEnd, EKRecurrenceFrequency, EKRecurrenceRule};
use objc2_foundation::NSArray;

use crate::{
    datetime::unix_to_ns_date,
    error::{EventKitError, EventKitResult},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecurrenceFrequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

#[derive(Debug, Clone)]
pub struct RecurrenceInput {
    pub frequency: RecurrenceFrequency,
    pub interval: i64,
    pub count: Option<i64>,
    pub end_date: Option<i64>,
}

pub fn apply_recurrence_to_item(
    item: &EKCalendarItem,
    rules: &[RecurrenceInput],
) -> EventKitResult<()> {
    let ek_rules: Vec<_> = rules
        .iter()
        .map(build_rule)
        .collect::<EventKitResult<_>>()?;
    let array = NSArray::from_retained_slice(&ek_rules);
    unsafe { item.setRecurrenceRules(Some(&array)) };
    Ok(())
}

fn build_rule(input: &RecurrenceInput) -> EventKitResult<objc2::rc::Retained<EKRecurrenceRule>> {
    if input.interval <= 0 {
        return Err(EventKitError::ValidationFailed(
            "recurrence interval must be positive".into(),
        ));
    }
    let frequency = match input.frequency {
        RecurrenceFrequency::Daily => EKRecurrenceFrequency::Daily,
        RecurrenceFrequency::Weekly => EKRecurrenceFrequency::Weekly,
        RecurrenceFrequency::Monthly => EKRecurrenceFrequency::Monthly,
        RecurrenceFrequency::Yearly => EKRecurrenceFrequency::Yearly,
    };
    let end = match (input.count, input.end_date) {
        (Some(count), _) => {
            let count = usize::try_from(count).map_err(|_| {
                EventKitError::ValidationFailed("recurrence count out of range".into())
            })?;
            Some(unsafe { EKRecurrenceEnd::recurrenceEndWithOccurrenceCount(count) })
        }
        (_, Some(end_date)) => {
            let date = unix_to_ns_date(end_date)?;
            Some(unsafe { EKRecurrenceEnd::recurrenceEndWithEndDate(&date) })
        }
        _ => None,
    };
    let interval = isize::try_from(input.interval)
        .map_err(|_| EventKitError::ValidationFailed("recurrence interval out of range".into()))?;
    Ok(unsafe {
        EKRecurrenceRule::initRecurrenceWithFrequency_interval_end(
            EKRecurrenceRule::alloc(),
            frequency,
            interval,
            end.as_deref(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_interval_is_rejected() {
        let input = RecurrenceInput {
            frequency: RecurrenceFrequency::Weekly,
            interval: 0,
            count: None,
            end_date: None,
        };
        assert!(build_rule(&input).is_err());
    }
}
