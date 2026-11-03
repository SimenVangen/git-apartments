//! The building: what an account's year of activity turns into.

use std::collections::HashMap;

use crate::{Activity, Rgb};

/// The most new residents shown moving in on one loop.
pub const MAX_ARRIVALS: usize = 3;
/// How many recent days can have someone moving in.
pub const RECENT_DAYS: usize = 7;

/// One apartment: one day of the year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Apartment {
    /// The day, `YYYY-MM-DD`.
    pub date: String,
    /// Contributions that day.
    pub contributions: u32,
    /// How brightly its window is lit, from 0 (empty) to 4 (the busiest
    /// days), relative to the year's busiest day so far.
    pub light: u8,
    /// The column of windows: the week of the year, January first.
    pub week: usize,
    /// The floor, from 0 (Sunday, the top floor) to 6 (Saturday, just
    /// above the lobby), as the rows run on GitHub's calendar.
    pub floor: usize,
    /// Whether the day is still to come. Its apartment waits for a tenant.
    pub upcoming: bool,
}

impl Apartment {
    /// Whether anyone lives here: whether there were contributions that day.
    pub fn occupied(&self) -> bool {
        self.contributions > 0
    }
}

/// An apartment building, ready to render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Building {
    /// The account's login, shown by the door.
    pub login: String,
    /// The year the building shows.
    pub year: i32,
    /// Contributions so far this year, shown on the roof sign.
    pub total_contributions: u32,
    /// Columns of windows: weeks of the year.
    pub weeks: usize,
    /// One apartment per day of the year, January 1 first.
    pub apartments: Vec<Apartment>,
    /// Today's apartment, as an index into `apartments`, when today falls
    /// in [`year`](Self::year). Its window blinks until someone moves in.
    pub today: Option<usize>,
    /// The newest residents, as indexes into `apartments`, oldest first:
    /// the up to [`MAX_ARRIVALS`] most recent days with contributions in
    /// the last [`RECENT_DAYS`]. They walk in carrying boxes, and their
    /// windows light up.
    pub arrivals: Vec<usize>,
    /// What the new residents wear: the account's top language colors.
    pub coats: Vec<Rgb>,
    /// Places the snow, the stars and who lives where. The same seed draws
    /// the same scene.
    pub seed: u64,
}

impl Building {
    /// The building for one calendar year, as of `today` (`YYYY-MM-DD`).
    ///
    /// Every day of the year gets a window, from January 1 to December 31.
    /// Days from the activity that fall in the year fill their windows;
    /// days after `today` are empty apartments still to come. The activity
    /// can cover any span: ask GitHub for exactly this year with
    /// [`QUERY`](crate::QUERY)'s `from` and `to`, or pass the default
    /// past year and only this year's part is used.
    pub fn for_year(activity: &Activity, year: i32, today: &str) -> Self {
        let counts: HashMap<&str, u32> = activity
            .days
            .iter()
            .map(|d| (d.date.as_str(), d.count))
            .collect();
        let first = days_from_civil(&format!("{year:04}-01-01")).unwrap_or(0);
        let last = days_from_civil(&format!("{year:04}-12-31")).unwrap_or(first);
        let today_n = days_from_civil(today);
        let offset = (first + 4).rem_euclid(7) as usize;

        let mut apartments: Vec<Apartment> = (first..=last)
            .enumerate()
            .map(|(i, n)| {
                let date = civil_from_days(n);
                let upcoming = today_n.is_some_and(|t| n > t);
                let contributions = if upcoming {
                    0
                } else {
                    counts.get(date.as_str()).copied().unwrap_or(0)
                };
                Apartment {
                    date,
                    contributions,
                    light: 0,
                    week: (i + offset) / 7,
                    floor: (i + offset) % 7,
                    upcoming,
                }
            })
            .collect();

        let busiest = apartments
            .iter()
            .map(|a| a.contributions)
            .max()
            .unwrap_or(0)
            .max(1) as u64;
        for a in &mut apartments {
            if a.contributions > 0 {
                a.light = 1 + ((a.contributions as u64 * 4 - 1) / busiest).min(3) as u8;
            }
        }

        let today = today_n
            .filter(|t| (first..=last).contains(t))
            .map(|t| (t - first) as usize);
        // Days up to and including today, or the whole year once it's past.
        let elapsed = match (today, today_n) {
            (Some(i), _) => i + 1,
            (None, Some(t)) if t > last => apartments.len(),
            _ => 0,
        };
        let mut arrivals: Vec<usize> = (elapsed.saturating_sub(RECENT_DAYS)..elapsed)
            .filter(|&i| apartments[i].occupied())
            .collect();
        arrivals.drain(..arrivals.len().saturating_sub(MAX_ARRIVALS));

        let mut coats: Vec<Rgb> = activity
            .languages
            .iter()
            .filter_map(|l| l.color)
            .take(3)
            .collect();
        for fallback in [Rgb::hex(0xc8323c), Rgb::hex(0x2e7d4f), Rgb::hex(0x3572a5)] {
            if coats.len() < 3 {
                coats.push(fallback);
            }
        }

        Building {
            login: activity.login.clone(),
            year,
            total_contributions: apartments.iter().map(|a| a.contributions).sum(),
            weeks: apartments.last().map(|a| a.week + 1).unwrap_or(0),
            apartments,
            today,
            arrivals,
            coats,
            seed: seed_for(&activity.login),
        }
    }

    /// The building for the year of the activity's last day, taking that
    /// day as today. This suits GitHub's calendar, which ends today; when
    /// asking for a whole year that may run into the future, use
    /// [`for_year`](Self::for_year) with the real date instead.
    pub fn from_activity(activity: &Activity) -> Self {
        let today = activity
            .days
            .last()
            .map(|d| d.date.clone())
            .unwrap_or_else(|| "1970-01-01".to_string());
        let year = today.get(..4).and_then(|y| y.parse().ok()).unwrap_or(1970);
        Building::for_year(activity, year, &today)
    }

    /// How many apartments are occupied: days with contributions.
    pub fn residents(&self) -> usize {
        self.apartments.iter().filter(|a| a.occupied()).count()
    }
}

/// The `from` and `to` variables for [`QUERY`](crate::QUERY) that cover
/// one calendar year.
pub fn year_range(year: i32) -> (String, String) {
    (
        format!("{year:04}-01-01T00:00:00Z"),
        format!("{year:04}-12-31T23:59:59Z"),
    )
}

/// Today's date in UTC, `YYYY-MM-DD`, from the system clock.
pub fn today_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    civil_from_days(secs.div_euclid(86_400))
}

/// FNV-1a over the lowercase login, so an account always gets the same
/// scene.
pub fn seed_for(login: &str) -> u64 {
    login
        .to_lowercase()
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325, |h, b| {
            (h ^ b as u64).wrapping_mul(0x0100_0000_01b3)
        })
}

/// Days since 1970-01-01 for a `YYYY-MM-DD` date.
fn days_from_civil(date: &str) -> Option<i64> {
    let mut it = date.split('-').map(|p| p.parse::<i64>());
    let (y, m, d) = (it.next()?.ok()?, it.next()?.ok()?, it.next()?.ok()?);
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

/// The `YYYY-MM-DD` date that many days after 1970-01-01.
fn civil_from_days(n: i64) -> String {
    let z = n + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_round_trip() {
        for date in [
            "1970-01-01",
            "2000-02-29",
            "2026-01-01",
            "2026-12-31",
            "2028-02-29",
        ] {
            assert_eq!(civil_from_days(days_from_civil(date).unwrap()), date);
        }
        // 1970-01-01 was a Thursday; 2026-01-01 is too.
        assert_eq!(
            (days_from_civil("2026-01-01").unwrap() + 4).rem_euclid(7),
            4
        );
    }

    #[test]
    fn a_whole_year_of_windows() {
        let empty = Activity {
            login: "x".into(),
            total_contributions: 0,
            days: vec![],
            languages: vec![],
        };
        let b = Building::for_year(&empty, 2026, "2026-11-03");
        assert_eq!(b.apartments.len(), 365);
        assert_eq!(b.apartments[0].date, "2026-01-01");
        assert_eq!((b.apartments[0].week, b.apartments[0].floor), (0, 4));
        assert_eq!(b.apartments[364].date, "2026-12-31");
        assert_eq!(b.weeks, 53);
        assert_eq!(b.today, Some(306));
        assert_eq!(b.apartments.iter().filter(|a| a.upcoming).count(), 58);

        let leap = Building::for_year(&empty, 2028, "2028-06-01");
        assert_eq!(leap.apartments.len(), 366);
    }
}
