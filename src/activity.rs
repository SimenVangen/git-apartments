//! A GitHub account's public activity, as read from the GraphQL API.

use std::collections::HashMap;

use serde::Deserialize;

use crate::{Error, Rgb};

/// The GraphQL query whose response [`Activity::from_graphql_json`] reads.
///
/// It takes one variable, `$login`. The contribution calendar covers the
/// past year and ends today; the languages come from the account's own
/// public, non-fork repositories, the 100 most recently pushed.
///
/// With the GitHub CLI:
///
/// ```text
/// gh api graphql -f query="$QUERY" -f login=octocat
/// ```
pub const QUERY: &str = r#"query($login: String!) {
  user(login: $login) {
    login
    contributionsCollection {
      contributionCalendar {
        totalContributions
        weeks { contributionDays { date contributionCount } }
      }
    }
    repositories(first: 100, ownerAffiliations: OWNER, isFork: false, privacy: PUBLIC, orderBy: {field: PUSHED_AT, direction: DESC}) {
      nodes {
        languages(first: 10, orderBy: {field: SIZE, direction: DESC}) {
          edges { size node { name color } }
        }
      }
    }
  }
}"#;

/// One day on the contribution calendar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Day {
    /// The date, `YYYY-MM-DD`.
    pub date: String,
    /// Contributions made that day.
    pub count: u32,
}

/// A language, summed across the account's repositories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Language {
    /// The language's name, such as `Rust`.
    pub name: String,
    /// GitHub's color for it, when it has one.
    pub color: Option<Rgb>,
    /// Bytes of code written in it.
    pub bytes: u64,
}

/// A GitHub account's public activity over the past year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Activity {
    /// The account's login.
    pub login: String,
    /// Contributions over the past year.
    pub total_contributions: u32,
    /// The contribution calendar, oldest day first. The last day is today.
    pub days: Vec<Day>,
    /// Languages, most-written first.
    pub languages: Vec<Language>,
}

impl Activity {
    /// Reads the response to [`QUERY`].
    ///
    /// Accepts the whole response (`{"data": {...}}`) or only its `data`
    /// object. A response carrying `errors` becomes [`Error::Api`], and one
    /// whose `user` is `null` becomes [`Error::NoSuchUser`].
    pub fn from_graphql_json(json: &str) -> Result<Self, Error> {
        let value: serde_json::Value = serde_json::from_str(json)?;

        if let Some(errors) = value.get("errors").and_then(|e| e.as_array()) {
            let messages: Vec<&str> = errors
                .iter()
                .filter_map(|e| e.get("message").and_then(|m| m.as_str()))
                .collect();
            let user_missing = errors
                .iter()
                .any(|e| e.get("type").and_then(|t| t.as_str()) == Some("NOT_FOUND"));
            if user_missing {
                return Err(Error::NoSuchUser(guess_login(&messages)));
            }
            if !messages.is_empty() {
                return Err(Error::Api(messages.join("; ")));
            }
        }

        // REST-style failures, such as bad credentials, carry a message.
        if let Some(message) = value.get("message").and_then(|m| m.as_str()) {
            return Err(Error::Api(message.to_string()));
        }
        let data = value.get("data").unwrap_or(&value);
        if data.get("user").is_none() {
            return Err(Error::Parse("the response has no `user`".into()));
        }
        let raw: RawData = serde_json::from_value(data.clone())?;
        let user = raw
            .user
            .ok_or_else(|| Error::NoSuchUser(String::from("(unknown)")))?;

        let cal = user.contributions_collection.contribution_calendar;
        let mut days: Vec<Day> = cal
            .weeks
            .into_iter()
            .flat_map(|w| w.contribution_days)
            .map(|d| Day {
                date: d.date,
                count: d.contribution_count,
            })
            .collect();
        days.sort_by(|a, b| a.date.cmp(&b.date));

        let mut sums: HashMap<String, (Option<Rgb>, u64)> = HashMap::new();
        for repo in user.repositories.nodes.into_iter().flatten() {
            let Some(langs) = repo.languages else {
                continue;
            };
            for edge in langs.edges {
                let color = edge.node.color.as_deref().and_then(|c| c.parse().ok());
                let entry = sums.entry(edge.node.name).or_insert((color, 0));
                entry.1 += edge.size;
            }
        }
        let mut languages: Vec<Language> = sums
            .into_iter()
            .map(|(name, (color, bytes))| Language { name, color, bytes })
            .collect();
        languages.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));

        Ok(Activity {
            login: user.login,
            total_contributions: cal.total_contributions,
            days,
            languages,
        })
    }

    /// Contributions on the day that just ended: the day before the last
    /// day on the calendar.
    ///
    /// The calendar always ends today, and today is still in progress, so
    /// a workflow that runs just after midnight reads yesterday as the
    /// finished day.
    pub fn day_that_ended(&self) -> u32 {
        match self.days.len() {
            0 | 1 => 0,
            n => self.days[n - 2].count,
        }
    }

    /// Days in a row with at least one contribution, up to today.
    ///
    /// An empty today does not break the streak, since the day is not over.
    pub fn current_streak(&self) -> u32 {
        let mut days = self.days.iter().rev().peekable();
        if days.peek().is_some_and(|d| d.count == 0) {
            days.next();
        }
        days.take_while(|d| d.count > 0).count() as u32
    }

    /// The most days in a row with at least one contribution, this year.
    pub fn longest_streak(&self) -> u32 {
        let mut best = 0;
        let mut run = 0;
        for d in &self.days {
            run = if d.count > 0 { run + 1 } else { 0 };
            best = best.max(run);
        }
        best
    }
}

/// GitHub's NOT_FOUND message reads "Could not resolve to a User with the
/// login of 'name'."; pull the name out when it is there.
fn guess_login(messages: &[&str]) -> String {
    messages
        .iter()
        .find_map(|m| {
            let start = m.find('\'')? + 1;
            let end = start + m[start..].find('\'')?;
            Some(m[start..end].to_string())
        })
        .unwrap_or_else(|| String::from("(unknown)"))
}

#[derive(Deserialize)]
struct RawData {
    user: Option<RawUser>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawUser {
    login: String,
    contributions_collection: RawCollection,
    repositories: RawRepos,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCollection {
    contribution_calendar: RawCalendar,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCalendar {
    total_contributions: u32,
    weeks: Vec<RawWeek>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawWeek {
    contribution_days: Vec<RawDay>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDay {
    date: String,
    contribution_count: u32,
}

#[derive(Deserialize)]
struct RawRepos {
    nodes: Vec<Option<RawRepo>>,
}

#[derive(Deserialize)]
struct RawRepo {
    languages: Option<RawLanguages>,
}

#[derive(Deserialize)]
struct RawLanguages {
    edges: Vec<RawLanguageEdge>,
}

#[derive(Deserialize)]
struct RawLanguageEdge {
    size: u64,
    node: RawLanguage,
}

#[derive(Deserialize)]
struct RawLanguage {
    name: String,
    color: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_counts(counts: &[u32]) -> Activity {
        Activity {
            login: "t".into(),
            total_contributions: counts.iter().sum(),
            days: counts
                .iter()
                .enumerate()
                .map(|(i, &count)| Day {
                    date: format!("2026-01-{:02}", i + 1),
                    count,
                })
                .collect(),
            languages: vec![],
        }
    }

    #[test]
    fn streaks() {
        let a = with_counts(&[1, 1, 1, 0, 2, 3, 0]);
        assert_eq!(a.current_streak(), 2, "an empty today keeps the streak");
        assert_eq!(a.longest_streak(), 3);
        assert_eq!(a.day_that_ended(), 3);

        let b = with_counts(&[1, 0, 0, 4]);
        assert_eq!(b.current_streak(), 1);
        assert_eq!(b.day_that_ended(), 0);

        let empty = with_counts(&[]);
        assert_eq!(empty.current_streak(), 0);
        assert_eq!(empty.day_that_ended(), 0);
    }

    #[test]
    fn login_from_message() {
        let m = ["Could not resolve to a User with the login of 'nobody-here'."];
        assert_eq!(guess_login(&m), "nobody-here");
    }
}
