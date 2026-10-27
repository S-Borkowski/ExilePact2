//! UTC timestamps without a date crate: the store needs two fixed formats,
//! one for quarantined file names and one for diagnostic log lines.

use std::time::{SystemTime, UNIX_EPOCH};

/// A UTC calendar time, to the second.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Utc {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl Utc {
    pub(crate) fn now() -> Self {
        Self::from_system_time(SystemTime::now())
    }

    pub(crate) fn from_system_time(t: SystemTime) -> Self {
        let secs = match t.duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_secs() as i64,
            Err(e) => -(e.duration().as_secs() as i64) - i64::from(e.duration().subsec_nanos() > 0),
        };
        Self::from_unix(secs)
    }

    /// Seconds since 1970-01-01T00:00:00Z to a proleptic Gregorian date
    /// (Howard Hinnant's `civil_from_days`).
    pub(crate) fn from_unix(secs: i64) -> Self {
        let days = secs.div_euclid(86_400);
        let sod = secs.rem_euclid(86_400);
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let year = yoe + era * 400 + i64::from(month <= 2);
        Utc {
            year,
            month,
            day,
            hour: (sod / 3_600) as u32,
            minute: (sod % 3_600 / 60) as u32,
            second: (sod % 60) as u32,
        }
    }

    /// `YYYYmmdd-HHMMSS`, the quarantine suffix.
    pub(crate) fn compact(&self) -> String {
        format!(
            "{:04}{:02}{:02}-{:02}{:02}{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }

    /// `YYYY-mm-dd HH:MM:SSZ`, the diagnostic log's line prefix.
    pub(crate) fn readable(&self) -> String {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}Z",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Utc;

    #[test]
    fn known_instants() {
        assert_eq!(Utc::from_unix(0).compact(), "19700101-000000");
        assert_eq!(Utc::from_unix(-1).compact(), "19691231-235959");
        assert_eq!(Utc::from_unix(1_709_251_199).compact(), "20240229-235959");
        assert_eq!(
            Utc::from_unix(1_792_931_696).readable(),
            "2026-10-25 12:34:56Z"
        );
    }
}
