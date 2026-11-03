//! Tests through the public API only, against saved GraphQL responses.

use git_apartments::render::{FRAMES, HEIGHT, WIDTH};
use git_apartments::{render_frames, render_gif, Activity, Building, Error, Rgb};

fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(path).expect("fixture exists")
}

fn building(name: &str) -> Building {
    Building::from_activity(&Activity::from_graphql_json(&fixture(name)).unwrap())
}

#[test]
fn one_apartment_per_day_laid_out_like_the_calendar() {
    let b = building("sparse.json");
    assert_eq!(b.apartments.len(), 365);
    assert_eq!(b.total_contributions, 89);
    assert_eq!(b.residents(), 34);

    // 2025-11-04 was a Tuesday, so the first window is on the third floor
    // from the top, and each later day steps down a floor or across a week.
    assert_eq!(b.apartments[0].date, "2025-11-04");
    assert_eq!((b.apartments[0].week, b.apartments[0].floor), (0, 2));
    for pair in b.apartments.windows(2) {
        let (a, n) = (&pair[0], &pair[1]);
        if a.floor == 6 {
            assert_eq!((n.week, n.floor), (a.week + 1, 0));
        } else {
            assert_eq!((n.week, n.floor), (a.week, a.floor + 1));
        }
    }
    assert_eq!(b.weeks, 53);
}

#[test]
fn lights_follow_contributions() {
    let b = building("sparse.json");
    for a in &b.apartments {
        assert_eq!(a.occupied(), a.light > 0, "{}", a.date);
        assert!(a.light <= 4);
    }
    let busiest = b.apartments.iter().max_by_key(|a| a.contributions).unwrap();
    assert_eq!(busiest.light, 4);
}

#[test]
fn recent_days_move_in() {
    let b = building("sparse.json");
    let dates: Vec<&str> = b
        .arrivals
        .iter()
        .map(|&i| b.apartments[i].date.as_str())
        .collect();
    assert_eq!(dates, ["2026-10-29", "2026-10-31", "2026-11-02"]);

    // Nothing in the last week: nobody moves in.
    assert!(building("quiet.json").arrivals.is_empty());
}

#[test]
fn new_residents_light_up_during_the_loop() {
    let b = building("sparse.json");
    let frames = render_frames(&b);
    assert_eq!(frames.len(), FRAMES);
    assert_eq!((frames[0].width, frames[0].height), (WIDTH, HEIGHT));
    // The first and last frames differ: the new arrivals' windows are dark
    // when the loop starts and lit when it ends.
    assert_ne!(frames[0], frames[FRAMES - 1]);
}

#[test]
fn coats_come_from_languages() {
    let b = building("sparse.json");
    assert_eq!(b.coats[0], Rgb::hex(0x3572a5));
    assert_eq!(building("quiet.json").coats.len(), 3);
}

#[test]
fn gif_has_the_promised_shape() {
    let b = building("busy.json");
    let mut gif = Vec::new();
    render_gif(&b, 2, &mut gif).unwrap();

    let mut opts = gif::DecodeOptions::new();
    opts.set_color_output(gif::ColorOutput::RGBA);
    let mut dec = opts.read_info(gif.as_slice()).unwrap();
    assert_eq!(dec.width() as usize, WIDTH * 2);
    assert_eq!(dec.height() as usize, HEIGHT * 2);
    let mut frames = 0;
    while dec.read_next_frame().unwrap().is_some() {
        frames += 1;
    }
    assert_eq!(frames, FRAMES);
}

#[test]
fn the_same_account_draws_the_same_gif() {
    let b = building("sparse.json");
    let (mut one, mut two) = (Vec::new(), Vec::new());
    render_gif(&b, 1, &mut one).unwrap();
    render_gif(&b, 1, &mut two).unwrap();
    assert_eq!(one, two);
}

#[test]
fn an_empty_account_still_draws() {
    let a = Activity {
        login: "nobody".into(),
        total_contributions: 0,
        days: vec![],
        languages: vec![],
    };
    let b = Building::from_activity(&a);
    assert_eq!(b.residents(), 0);
    assert_eq!(render_frames(&b).len(), FRAMES);
}

#[test]
fn errors_are_explained() {
    match Activity::from_graphql_json(&fixture("not_found.json")) {
        Err(Error::NoSuchUser(login)) => assert_eq!(login, "no-such-snowman"),
        other => panic!("expected NoSuchUser, got {other:?}"),
    }
    match Activity::from_graphql_json(&fixture("bad_credentials.json")) {
        Err(Error::Api(m)) => assert_eq!(m, "Bad credentials"),
        other => panic!("expected Api, got {other:?}"),
    }
    assert!(matches!(
        Activity::from_graphql_json("not json"),
        Err(Error::Parse(_))
    ));
}
