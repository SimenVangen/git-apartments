//! The building: what an account's activity turns into.

use crate::{Activity, Rgb};

/// The most new residents shown moving in on one loop.
pub const MAX_ARRIVALS: usize = 3;
/// How many recent days can have someone moving in.
pub const RECENT_DAYS: usize = 7;

/// One apartment: one day on the contribution calendar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Apartment {
    /// The day, `YYYY-MM-DD`.
    pub date: String,
    /// Contributions that day.
    pub contributions: u32,
    /// How brightly its window is lit, from 0 (empty) to 4 (the busiest
    /// days), relative to the account's busiest day.
    pub light: u8,
    /// The column of windows: the week, oldest first.
    pub week: usize,
    /// The floor, from 0 (Sunday, the top floor) to 6 (Saturday, just
    /// above the lobby), as the rows run on GitHub's calendar.
    pub floor: usize,
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
    /// The account's login, shown above the door.
    pub login: String,
    /// Contributions over the past year, shown on the roof sign.
    pub total_contributions: u32,
    /// Columns of windows: weeks on the calendar.
    pub weeks: usize,
    /// One apartment per day, oldest first. The last is today.
    pub apartments: Vec<Apartment>,
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
    /// The building an account's activity draws.
    pub fn from_activity(activity: &Activity) -> Self {
        let offset = activity.days.first().map(|d| weekday(&d.date)).unwrap_or(0);
        let busiest = activity
            .days
            .iter()
            .map(|d| d.count)
            .max()
            .unwrap_or(0)
            .max(1) as u64;

        let apartments: Vec<Apartment> = activity
            .days
            .iter()
            .enumerate()
            .map(|(i, d)| Apartment {
                date: d.date.clone(),
                contributions: d.count,
                light: if d.count == 0 {
                    0
                } else {
                    1 + ((d.count as u64 * 4 - 1) / busiest).min(3) as u8
                },
                week: (i + offset) / 7,
                floor: (i + offset) % 7,
            })
            .collect();

        let weeks = apartments.last().map(|a| a.week + 1).unwrap_or(0);
        let recent_start = apartments.len().saturating_sub(RECENT_DAYS);
        let mut arrivals: Vec<usize> = (recent_start..apartments.len())
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
            total_contributions: activity.total_contributions,
            weeks,
            apartments,
            arrivals,
            coats,
            seed: seed_for(&activity.login),
        }
    }

    /// How many apartments are occupied: days with contributions.
    pub fn residents(&self) -> usize {
        self.apartments.iter().filter(|a| a.occupied()).count()
    }
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

/// 0 for Sunday through 6 for Saturday.
fn weekday(date: &str) -> usize {
    days_from_civil(date)
        .map(|d| (d + 4).rem_euclid(7) as usize)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weekdays() {
        assert_eq!(weekday("1970-01-01"), 4);
        assert_eq!(weekday("2026-11-03"), 2);
        assert_eq!(weekday("2024-02-29"), 4);
        assert_eq!(weekday("2000-03-01"), 3);
    }
}
