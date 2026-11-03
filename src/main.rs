//! The `git-apartments` command.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

use clap::Parser;
use git_apartments::{Activity, Building, QUERY};

/// Draws a pixel-art apartment building from a GitHub account's
/// contributions: one window per day, lit when you contributed.
///
/// Fetches the activity with the GitHub CLI (`gh`), which must be
/// installed and signed in, or reads a saved response with --json.
#[derive(Parser)]
#[command(version, about, long_about)]
struct Args {
    /// The GitHub login to draw. Defaults to the account `gh` is signed in as.
    login: Option<String>,

    /// Where to write the GIF.
    #[arg(short, long, default_value = "apartments.gif")]
    output: PathBuf,

    /// How many screen pixels each art pixel becomes. 3 gives a 900×312 GIF.
    #[arg(short, long, default_value_t = 3, value_parser = clap::value_parser!(u16).range(1..=8))]
    scale: u16,

    /// Read the GraphQL response from this file instead of fetching it.
    #[arg(long, value_name = "FILE", conflicts_with = "login")]
    json: Option<PathBuf>,

    /// Also save the fetched GraphQL response to this file.
    #[arg(long, value_name = "FILE", conflicts_with = "json")]
    save_json: Option<PathBuf>,

    /// Print the GraphQL query and exit.
    #[arg(long)]
    print_query: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();
    if args.print_query {
        println!("{QUERY}");
        return ExitCode::SUCCESS;
    }
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("git-apartments: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let json = match &args.json {
        Some(path) => fs::read_to_string(path)
            .map_err(|e| format!("could not read {}: {e}", path.display()))?,
        None => {
            let login = match &args.login {
                Some(l) => l.clone(),
                None => gh(&["api", "user", "--jq", ".login"])?.trim().to_string(),
            };
            let json = gh(&[
                "api",
                "graphql",
                "-f",
                &format!("query={QUERY}"),
                "-f",
                &format!("login={login}"),
            ])?;
            if let Some(path) = &args.save_json {
                fs::write(path, &json)?;
            }
            json
        }
    };

    let activity = Activity::from_graphql_json(&json).map_err(|e| match (e, &args.login) {
        (git_apartments::Error::NoSuchUser(_), Some(login)) => {
            git_apartments::Error::NoSuchUser(login.clone())
        }
        (e, _) => e,
    })?;
    let building = Building::from_activity(&activity);

    let mut gif = Vec::new();
    git_apartments::render_gif(&building, args.scale, &mut gif)?;
    if let Some(dir) = args.output.parent().filter(|d| !d.as_os_str().is_empty()) {
        fs::create_dir_all(dir)?;
    }
    fs::write(&args.output, &gif)?;

    let moving_in: Vec<&str> = building
        .arrivals
        .iter()
        .map(|&i| building.apartments[i].date.as_str())
        .collect();
    println!(
        "{}: {} residents in {} apartments, {} contributions this year; moving in: {}",
        building.login,
        building.residents(),
        building.apartments.len(),
        building.total_contributions,
        if moving_in.is_empty() {
            "nobody this week".to_string()
        } else {
            moving_in.join(", ")
        },
    );
    println!("wrote {}", args.output.display());
    Ok(())
}

/// Runs the GitHub CLI and returns what it printed.
fn gh(args: &[&str]) -> Result<String, String> {
    let out = Command::new("gh").args(args).output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "the GitHub CLI (gh) is not installed; get it from https://cli.github.com, or pass --json".to_string()
        } else {
            format!("could not run gh: {e}")
        }
    })?;
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    if out.status.success() {
        return Ok(stdout);
    }
    // GraphQL errors come back on stdout as JSON with a failing status;
    // hand them to the parser, which explains them better than gh does.
    if stdout.trim_start().starts_with('{') {
        return Ok(stdout);
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    Err(format!(
        "gh {} failed: {}",
        args[..2].join(" "),
        stderr.trim()
    ))
}
