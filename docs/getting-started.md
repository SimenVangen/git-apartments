# Getting started

git-apartments can be used three ways: as a GitHub Action that keeps a building on your profile README, as a command, or as a Rust library.

## On your profile README

Your profile README lives in the repository named after your username, such as `octocat/octocat`.

1. In that repository, create `.github/workflows/apartments.yml` with the contents of [`examples/profile-workflow.yml`](../examples/profile-workflow.yml).
2. Add the building to `README.md`:

   ```markdown
   ![My apartment building](assets/apartments.gif)
   ```

3. Open the **Actions** tab, pick **Apartments**, and choose **Run workflow**. After that it runs on its own every night just after midnight UTC, so each day's window is filled in after the day ends.

The workflow needs `contents: write` to commit the GIF, which the example already asks for. The built-in token is enough to read public activity, so there is nothing to set up.

Until there's a release, use `SimenVangen/git-apartments@main` in the workflow instead of `@v0`. It builds from source, which takes about a minute.

### Action inputs

| Input | Default | What it does |
| --- | --- | --- |
| `login` | the repository's owner | The account to draw. |
| `output` | `apartments.gif` | Where to write the GIF. |
| `scale` | `3` | Screen pixels per art pixel, 1–8. `3` gives 900×312. |
| `token` | the workflow token | Token for the GitHub API. |

It sets two outputs, `path` and `summary` (such as `34 residents in 365 apartments, 89 contributions in 2026; moving in: 2026-11-02`), and writes the summary to the job summary.

## From the command line

Install a binary from the releases page, or build one:

```sh
cargo install git-apartments
```

The command fetches activity with the [GitHub CLI](https://cli.github.com/), so sign in first with `gh auth login`.

```sh
git-apartments                       # draws the account gh is signed in as
git-apartments octocat -o octo.gif   # draws someone else
git-apartments --year 2025           # draws a past year
git-apartments --save-json me.json   # keeps the API response
git-apartments --json me.json        # draws from a saved response, offline
git-apartments --today 2026-12-24    # pretends it's another day, for previews
git-apartments --print-query         # prints the GraphQL query it sends
```

It prints how many residents the building has and who's moving in.

## As a library

```toml
[dependencies]
git-apartments = { version = "0.1", default-features = false }
```

Turning off default features drops the command and its `clap` dependency.

The library doesn't make network requests. Fetch the response to [`QUERY`](https://docs.rs/git-apartments/latest/git_apartments/constant.QUERY.html) however you like, then:

```rust
use git_apartments::{Activity, Building};

// The response to QUERY with `from` and `to` from git_apartments::year_range(2026).
let activity = Activity::from_graphql_json(&json)?;
let building = Building::for_year(&activity, 2026, &git_apartments::today_utc());
git_apartments::render_gif(&building, 3, std::fs::File::create("apartments.gif")?)?;
```

Every field of `Building` is public, so you can change who's moving in, or the coats they wear, before rendering. `render_frames` and `draw_frame` give you the raw frames, and `encode_gif` turns any frames into a looping GIF.

## How the windows map to days

The windows are this year's contribution graph turned into a building:

- Each column of windows is a week, January on the left and December on the right.
- Each floor is a day of the week, Sunday at the top and Saturday just above the lobby, so the first and last columns may be partly blank wall.
- Days in a row run down a column and on into the next, exactly like the squares on GitHub.
- A window's light is relative to your busiest day of the year so far: days close to it are bright, light days are dim amber.
- Days after today are darker, empty apartments. Today's window blinks until you contribute.
- Each night the tool asks GitHub for this year's calendar and draws the whole building again, so the windows always match your graph. On January 1 it moves on to the new year.

The newest residents are the up to three most recent days with contributions in the last seven. They walk in from alternate ends of the street, and their window flashes white when they reach their floor. With none, a passer-by walks along the street instead.
