use std::time::{Duration, Instant};

use crate::{AlmostEq, ErrorTolerance};

#[cfg(not(target_os = "macos"))]
pub fn system_native_tick_interval_in_nanos() -> (u64, bool) {
    (1, false)
}

#[cfg(target_os = "macos")]
pub fn system_native_tick_interval_in_nanos() -> (u64, bool) {
    macos::tick_interval()
}

#[cfg(not(target_os = "macos"))]
impl ErrorTolerance<Duration> for Instant {
    type Tolerance = u64;

    fn default_error_tolerance() -> Self::Tolerance {
        0
    }
}

#[cfg(target_os = "macos")]
impl ErrorTolerance<Duration> for Instant {
    type Tolerance = u64;

    fn default_error_tolerance() -> Self::Tolerance {
        macos::tolerance()
    }
}

impl AlmostEq<Instant> for Duration {
    fn almost_eq(
        &self,
        other: &Self,
        tolerance: <Instant as ErrorTolerance<Self>>::Tolerance,
    ) -> bool {
        use std::cmp::Ordering;

        match self.cmp(other) {
            Ordering::Equal => true,
            Ordering::Less => (other.as_nanos() - self.as_nanos()) as u64 <= tolerance,
            Ordering::Greater => (self.as_nanos() - other.as_nanos()) as u64 <= tolerance,
        }
    }
}

#[cfg(target_os = "macos")]
mod macos {
    #[repr(C)]
    #[derive(Copy, Clone, Debug)]
    pub(crate) struct mach_timebase_info {
        pub(crate) numer: u32,
        pub(crate) denom: u32,
    }

    #[allow(non_camel_case_types)]
    type mach_timebase_info_t = *mut mach_timebase_info;

    #[allow(non_camel_case_types)]
    type kern_return_t = std::os::raw::c_int;

    extern "C" {
        fn mach_timebase_info(info: mach_timebase_info_t) -> kern_return_t;
    }

    fn timebase_info() -> mach_timebase_info {
        let mut info = mach_timebase_info { numer: 0, denom: 0 };
        unsafe {
            mach_timebase_info(&mut info);
        }
        info
    }

    pub(crate) fn tick_interval() -> (u64, bool) {
        let mach_timebase_info { numer, denom } = timebase_info();
        let q = 1 / denom;
        let r = 1 % denom;
        let interval_f = (q * numer + r * numer) as f64 / denom as f64;
        let interval_i = interval_f.ceil() as u64;
        (interval_i, interval_i > interval_f.trunc() as u64)
    }

    pub(crate) fn tolerance() -> u64 {
        let (interval, is_truncated) = tick_interval();
        if is_truncated {
            interval - 1
        } else {
            interval
        }
    }
}

#[cfg(test)]
mod test {

    use super::system_native_tick_interval_in_nanos;
    use crate::{assert_almost_eq, AlmostEq, ErrorTolerance};
    use std::time::{Duration, Instant};

    #[test]
    fn test_tick_intervals() {
        let (interval, is_truncated) = system_native_tick_interval_in_nanos();
        assert!(interval > 0);

        let now = Instant::now();

        assert_eq!(
            (now + Duration::from_nanos(interval - 1)).duration_since(now),
            Duration::from_nanos(0)
        );

        let expected = if is_truncated {
            Duration::from_nanos(interval - 1)
        } else {
            Duration::from_nanos(interval)
        };
        assert_eq!(
            (now + Duration::from_nanos(interval)).duration_since(now),
            expected
        );
    }

    #[test]
    fn test_almost_eq_macro() {
        let now = Instant::now();
        let expected = Duration::from_nanos(29);
        let instant = now + expected;
        let actual = instant.duration_since(now);

        assert_almost_eq!(actual, expected, Instant::default_error_tolerance());

        let expected = Duration::from_nanos(109);
        let instant = now + expected;
        let actual = instant.duration_since(now);

        assert_almost_eq!(actual, expected, Instant::default_error_tolerance());
    }
}
