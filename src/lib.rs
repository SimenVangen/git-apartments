//! Draws a pixel-art apartment building from a GitHub account's
//! contributions.
//!
//! Every day of the past year is an apartment window. Weeks run left to
//! right and the days of the week are the floors, Sunday at the top, so
//! the building is the contribution graph. A day you contributed is an
//! occupied apartment with its light on, brighter the more you did. The
//! newest residents, from the last week's contributions, walk in carrying
//! boxes, and their windows light up as they reach their floor.
//!
//! | Activity | Building |
//! | --- | --- |
//! | Each day of the year | One window |
//! | Contributions that day | The light: dark, then dim amber to bright |
//! | Up to 3 recent days with contributions | New residents moving in |
//! | Contributions this year | The neon sign on the roof |
//! | Days with contributions | The residents count by the door |
//! | Top languages | The new residents' coats and the awning |
//!
//! # Example
//!
//! ```
//! use git_apartments::{Activity, Building};
//!
//! # fn main() -> Result<(), git_apartments::Error> {
//! // The response to `git_apartments::QUERY`, from `gh api graphql` or any client.
//! let json = r##"{"data":{"user":{
//!     "login": "octocat",
//!     "contributionsCollection": {"contributionCalendar": {
//!         "totalContributions": 12,
//!         "weeks": [{"contributionDays": [
//!             {"date": "2026-11-01", "contributionCount": 12},
//!             {"date": "2026-11-02", "contributionCount": 0}
//!         ]}]
//!     }},
//!     "repositories": {"nodes": [{"languages": {"edges": [
//!         {"size": 9000, "node": {"name": "Python", "color": "#3572A5"}}
//!     ]}}]}
//! }}}"##;
//!
//! let activity = Activity::from_graphql_json(json)?;
//! let building = Building::from_activity(&activity);
//! assert_eq!(building.residents(), 1);
//! assert_eq!(building.arrivals, [0]);
//!
//! let mut gif = Vec::new();
//! git_apartments::render_gif(&building, 3, &mut gif)?;
//! assert!(gif.starts_with(b"GIF89a"));
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod activity;
mod building;
mod canvas;
mod color;
mod error;
mod font;
pub mod render;

pub use activity::{Activity, Day, Language, QUERY};
pub use building::{seed_for, Apartment, Building, MAX_ARRIVALS, RECENT_DAYS};
pub use canvas::{encode_gif, Frame};
pub use color::Rgb;
pub use error::Error;
pub use render::{draw_frame, render_activity_gif, render_frames, render_gif};
