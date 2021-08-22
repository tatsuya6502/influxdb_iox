use chrono::{DateTime, Utc};
use once_cell::sync::OnceCell;
use std::time::Instant;

/// Stores an Instant and DateTime<Utc> captured as close as possible together
static INSTANCE: OnceCell<(DateTime<Utc>, Instant)> = OnceCell::new();

/// Provides a conversion from Instant to DateTime<Utc> for display purposes
///
/// It is an approximation as if the system clock changes, the returned DateTime will not be
/// the same as the DateTime that would have been recorded at the time the Instant was created.
///
/// The conversion does, however, preserve the monotonic property of Instant, i.e. a larger
/// Instant will have a larger returned DateTime.
///
/// This should ONLY be used for display purposes, the results should not be used to
/// drive logic, nor persisted
pub fn to_approximate_datetime(instant: Instant) -> DateTime<Utc> {
    let (ref_date, ref_instant) = *INSTANCE.get_or_init(|| (Utc::now(), Instant::now()));

    if ref_instant > instant {
        ref_date
            - chrono::Duration::from_std(ref_instant.duration_since(instant))
                .expect("date overflow")
    } else {
        ref_date
            + chrono::Duration::from_std(instant.duration_since(ref_instant))
                .expect("date overflow")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use test_helpers::{assert_almost_eq, AlmostEq, ErrorTolerance};

    // Create a wrapper struct for chrono::Duration and implement test_helpers::
    // AlmostEq and ErrorTolerance traits for it. We have to create the wrapper
    // struct because this crate (data_types) does not define neither the type
    // (chrono::Duration) nor the traits.
    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
    struct DT(DateTime<Utc>);

    impl ErrorTolerance<DT> for Instant {
        type Tolerance = u64;

        fn default_error_tolerance() -> Self::Tolerance {
            <Instant as ErrorTolerance<Duration>>::default_error_tolerance()
        }
    }

    impl AlmostEq<Instant> for DT {
        fn almost_eq(
            &self,
            other: &Self,
            tolerance: <Instant as ErrorTolerance<Self>>::Tolerance,
        ) -> bool {
            use std::cmp::Ordering;

            let is_within_tolerance = |dur: chrono::Duration| {
                let duration_nanos = dur
                    .to_std()
                    .expect("Cannot convert to std::time::Duration")
                    .as_nanos() as u64;
                duration_nanos <= tolerance
            };

            match self.cmp(other) {
                Ordering::Equal => true,
                Ordering::Less => is_within_tolerance(other.0 - self.0),
                Ordering::Greater => is_within_tolerance(self.0 - other.0),
            }
        }
    }
    #[test]
    fn test_to_datetime() {
        // Seed global state
        to_approximate_datetime(Instant::now());

        let (ref_date, ref_instant) = *INSTANCE.get().unwrap();
        let tolerance = <Instant as ErrorTolerance<DT>>::default_error_tolerance();

        assert_almost_eq!(
            DT(to_approximate_datetime(
                ref_instant + std::time::Duration::from_nanos(78)
            )),
            DT(ref_date + chrono::Duration::nanoseconds(78)),
            tolerance,
        );

        assert_almost_eq!(
            DT(to_approximate_datetime(
                ref_instant - std::time::Duration::from_nanos(23)
            )),
            DT(ref_date - chrono::Duration::nanoseconds(23)),
            tolerance,
        );
    }

    #[test]
    fn test_to_datetime_simple() {
        let d = std::time::Duration::from_nanos(78);
        let a = Instant::now();
        let b = a + d;
        // assert_almost_eq!(b.duration_since(a), d, 3);
        let tolerance = <Instant as ErrorTolerance<Duration>>::default_error_tolerance();
        assert_almost_eq!(b.duration_since(a), d, tolerance,);
    }
}
