//! Renders the examples in `docs/gallery` from the saved test responses,
//! as of November 3, 2026: a quiet year, a year like most, and a busy one.
//!
//! ```text
//! cargo run --example gallery --no-default-features
//! ```

use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;

use git_apartments::{render_gif, Activity, Building};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = Path::new("docs/gallery");
    fs::create_dir_all(out)?;
    for name in ["quiet", "this_year", "busy"] {
        let json = fs::read_to_string(format!("tests/fixtures/{name}.json"))?;
        let activity = Activity::from_graphql_json(&json)?;
        let building = Building::for_year(&activity, 2026, "2026-11-03");
        let path = out.join(format!("{name}.gif"));
        render_gif(&building, 3, BufWriter::new(File::create(&path)?))?;
        println!("wrote {}", path.display());
    }
    Ok(())
}
