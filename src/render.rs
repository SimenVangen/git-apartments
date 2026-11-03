//! Drawing the building, frame by frame.
//!
//! Every day of the year is a window: weeks run left to right and the days
//! of the week are the floors, Sunday at the top, like GitHub's calendar.
//! A day with contributions is an occupied apartment with its light on,
//! brighter the more you contributed. Recent days' residents walk in
//! carrying boxes, and their windows light up as they reach their floor.

use std::io::Write;

use crate::building::Building;
use crate::canvas::{encode_gif, hash, Frame, Rng};
use crate::{font, Activity, Error, Rgb};

/// Scene width in pixels, before scaling.
pub const WIDTH: usize = 300;
/// Scene height in pixels, before scaling.
pub const HEIGHT: usize = 104;
/// Frames in one loop.
pub const FRAMES: usize = 84;
/// Each frame's delay, in hundredths of a second.
pub const FRAME_DELAY: u16 = 7;

const PITCH_X: i32 = 5;
const PITCH_Y: i32 = 7;
const ROOF_Y: i32 = 23;
const FLOORS_Y: i32 = 26;
const LOBBY_Y: i32 = 75;
const SIDEWALK_Y: i32 = 88;
const CURB_Y: i32 = 92;
const FEET_Y: i32 = 90;
const WALK_SPEED: i32 = 4;

const NIGHT: [Rgb; 4] = [
    Rgb::hex(0x0b1026),
    Rgb::hex(0x111a3a),
    Rgb::hex(0x18244e),
    Rgb::hex(0x223063),
];
const STAR: Rgb = Rgb::hex(0xdfe6ff);
const STAR_DIM: Rgb = Rgb::hex(0x6d7aa8);
const MOON: Rgb = Rgb::hex(0xf3efd2);
const SNOW: Rgb = Rgb::hex(0xf4f8ff);
const SNOW_SHADE: Rgb = Rgb::hex(0xc9d6ec);

const WALL: Rgb = Rgb::hex(0x343c62);
const WALL_LIGHT: Rgb = Rgb::hex(0x3f4870);
const WALL_DARK: Rgb = Rgb::hex(0x21263f);
const PARAPET: Rgb = Rgb::hex(0x3d4570);
const SILL: Rgb = Rgb::hex(0x4a5380);
const GLASS: Rgb = Rgb::hex(0x121630);
const GLASS_HI: Rgb = Rgb::hex(0x252e55);
/// Window light, from barely on to the busiest days.
const LIT: [Rgb; 5] = [
    GLASS,
    Rgb::hex(0xa8692a),
    Rgb::hex(0xd99a3c),
    Rgb::hex(0xf2c160),
    Rgb::hex(0xffe7a0),
];
const RESIDENT: Rgb = Rgb::hex(0x3a2412);
const TV: [Rgb; 2] = [Rgb::hex(0x7fb2ff), Rgb::hex(0xc9e0ff)];

const LOBBY: Rgb = Rgb::hex(0x3a3550);
const CORNICE: Rgb = Rgb::hex(0x4d557c);
const DOOR: Rgb = Rgb::hex(0x7a4a28);
const DOOR_DARK: Rgb = Rgb::hex(0x52301a);
const PLAQUE: Rgb = Rgb::hex(0x1a1d30);
const GOLD: Rgb = Rgb::hex(0xffd75e);
const LOBBY_GLOW: Rgb = Rgb::hex(0x6e4c2c);
const PLANT: Rgb = Rgb::hex(0x2f5a33);

const SIGN: Rgb = Rgb::hex(0x1a1222);
const NEON: Rgb = Rgb::hex(0xff6fb0);
const NEON_OFF: Rgb = Rgb::hex(0x4a2338);
const BULB: Rgb = Rgb::hex(0xffd75e);
const BULB_DIM: Rgb = Rgb::hex(0x6b5a2a);
const POST: Rgb = Rgb::hex(0x2a2f45);
const WOOD: Rgb = Rgb::hex(0x6b4426);
const WOOD_DARK: Rgb = Rgb::hex(0x4a2e19);
const BEACON: Rgb = Rgb::hex(0xff4d4d);
const BEACON_OFF: Rgb = Rgb::hex(0x5a1f1f);

const SIDEWALK: Rgb = Rgb::hex(0x535a78);
const SIDEWALK_LIT: Rgb = Rgb::hex(0x6e7594);
const CURB: Rgb = Rgb::hex(0x2a2f45);
const ROAD: Rgb = Rgb::hex(0x1a1d2e);
const LANE: Rgb = Rgb::hex(0x8a8448);
const LAMP: Rgb = Rgb::hex(0xffe9a8);

const PANTS: Rgb = Rgb::hex(0x2a2f45);
const BOX: Rgb = Rgb::hex(0xc49a5c);
const BOX_DARK: Rgb = Rgb::hex(0x8a6638);
const HAIR: [Rgb; 4] = [
    Rgb::hex(0x2b1b10),
    Rgb::hex(0x6b4426),
    Rgb::hex(0xe0c068),
    Rgb::hex(0x1c1c24),
];
const SKIN: [Rgb; 3] = [Rgb::hex(0xf1c27d), Rgb::hex(0xc68642), Rgb::hex(0x8d5524)];

/// Where the building sits in the frame.
struct Layout {
    x0: i32,
    width: i32,
    door: i32,
}

impl Layout {
    fn new(b: &Building) -> Self {
        let weeks = b.weeks.max(1) as i32;
        let width = weeks * PITCH_X + 4;
        let x0 = (WIDTH as i32 - width) / 2;
        Layout {
            x0,
            width,
            door: x0 + width / 2,
        }
    }

    fn right(&self) -> i32 {
        self.x0 + self.width - 1
    }

    fn window(&self, week: usize, floor: usize) -> (i32, i32) {
        (
            self.x0 + 3 + week as i32 * PITCH_X,
            FLOORS_Y + 1 + floor as i32 * PITCH_Y,
        )
    }
}

/// Someone on the street.
struct Walker {
    from_left: bool,
    start: i32,
    coat: Rgb,
    hair: Rgb,
    skin: Rgb,
    /// The apartment they're moving into; a passer-by has none.
    home: Option<usize>,
}

impl Walker {
    fn start_x(&self) -> i32 {
        if self.from_left {
            -4
        } else {
            WIDTH as i32 + 3
        }
    }

    /// The frame they step through the door.
    fn enters(&self, door: i32) -> i32 {
        let distance = (door - self.start_x()).abs();
        self.start + (distance + WALK_SPEED - 1) / WALK_SPEED
    }
}

fn walkers(b: &Building) -> Vec<Walker> {
    let look = |k: u64| {
        let h = hash(b.seed, 0x77 + k);
        (
            HAIR[(h % HAIR.len() as u64) as usize],
            SKIN[((h >> 8) % SKIN.len() as u64) as usize],
        )
    };
    if b.arrivals.is_empty() {
        // Nobody new: someone just walks past.
        let (hair, skin) = look(0);
        return vec![Walker {
            from_left: true,
            start: 0,
            coat: b.coats[0],
            hair,
            skin,
            home: None,
        }];
    }
    b.arrivals
        .iter()
        .enumerate()
        .map(|(k, &home)| {
            let (hair, skin) = look(k as u64 + 1);
            Walker {
                from_left: k % 2 == 0,
                start: k as i32 * 8,
                coat: b.coats[k % b.coats.len()],
                hair,
                skin,
                home: Some(home),
            }
        })
        .collect()
}

/// The frame an arriving resident's light comes on: after they're through
/// the door and up to their floor.
fn lights_on(w: &Walker, layout: &Layout, floor: usize) -> i32 {
    w.enters(layout.door) + 3 + (6 - floor as i32)
}

/// Draws one frame of a building's loop. `frame` runs from 0 to
/// [`FRAMES`]` - 1`.
pub fn draw_frame(b: &Building, frame: usize) -> Frame {
    let t = (frame % FRAMES) as i32;
    let layout = Layout::new(b);
    let people = walkers(b);
    let mut f = Frame::new(WIDTH, HEIGHT, NIGHT[0]);

    draw_sky(&mut f, b.seed, t);
    draw_roof(&mut f, b, &layout, t);
    draw_facade(&mut f, &layout);
    draw_windows(&mut f, b, &layout, &people, t);
    draw_lobby(&mut f, b, &layout, &people, t);
    draw_street(&mut f, &layout, b.seed);
    for w in &people {
        draw_walker(&mut f, w, &layout, t);
    }
    draw_snow(&mut f, b.seed, t);
    f
}

/// Draws every frame of a building's loop.
pub fn render_frames(b: &Building) -> Vec<Frame> {
    (0..FRAMES).map(|t| draw_frame(b, t)).collect()
}

/// Renders a building's loop as a GIF that repeats forever, each pixel
/// scaled up to a `scale` × `scale` block.
pub fn render_gif<W: Write>(b: &Building, scale: u16, out: W) -> Result<(), Error> {
    encode_gif(&render_frames(b), scale, FRAME_DELAY, out)
}

/// Builds an account's building and renders it as a GIF in one step.
pub fn render_activity_gif<W: Write>(activity: &Activity, scale: u16, out: W) -> Result<(), Error> {
    render_gif(&Building::from_activity(activity), scale, out)
}

fn draw_sky(f: &mut Frame, seed: u64, t: i32) {
    let band = SIDEWALK_Y / 4;
    for y in 0..SIDEWALK_Y {
        let b = ((y / band) as usize).min(3);
        for x in 0..WIDTH as i32 {
            let edge = b < 3 && y % band == band - 1 && (x + y) % 2 == 0;
            f.set(x, y, if edge { NIGHT[b + 1] } else { NIGHT[b] });
        }
    }
    let mut rng = Rng(seed ^ 0x57a2);
    for i in 0..26 {
        let (x, y) = (rng.below(WIDTH as u32), rng.below(60));
        let dim = (i * 5 + t) % 21 < 3;
        f.set(x, y, if dim { STAR_DIM } else { STAR });
    }
    // A crescent moon in the corner.
    let (mx, my) = (WIDTH as i32 - 9, 7);
    for dy in -3..=3 {
        for dx in -3..=3 {
            let inside = |a: i32, b: i32| a * a + b * b <= 12;
            if inside(dx, dy) && !inside(dx - 2, dy + 1) {
                f.set(mx + dx, my + dy, MOON);
            }
        }
    }
}

fn draw_roof(f: &mut Frame, b: &Building, l: &Layout, t: i32) {
    // Water tower on the left.
    let wx = l.x0 + 9;
    f.rect(wx, 13, wx + 8, 19, WOOD);
    for y in [14, 17] {
        f.rect(wx, y, wx + 8, y, WOOD_DARK);
    }
    f.rect(wx + 1, 11, wx + 7, 12, WOOD_DARK);
    f.rect(wx + 3, 10, wx + 5, 10, WOOD_DARK);
    f.rect(wx + 2, 11, wx + 6, 11, SNOW_SHADE);
    f.rect(wx + 3, 10, wx + 5, 10, SNOW);
    for x in [wx + 1, wx + 7] {
        f.rect(x, 20, x, ROOF_Y - 1, POST);
    }
    f.rect(wx + 1, 21, wx + 7, 21, POST);

    // An antenna on the right with a blinking beacon.
    let ax = l.right() - 8;
    f.rect(ax, 9, ax, ROOF_Y - 1, POST);
    f.rect(ax - 2, 14, ax + 2, 14, POST);
    f.rect(ax - 1, 11, ax + 1, 11, POST);
    f.set(ax, 8, if t % 20 < 10 { BEACON } else { BEACON_OFF });

    // The neon sign.
    let text = format!("{} CONTRIBUTIONS", b.total_contributions);
    let tw = font::width(&text, 2);
    let cx = l.x0 + l.width / 2;
    let (bx0, bx1, by0, by1) = (cx - tw / 2 - 4, cx + tw / 2 + 4, 4, 19);
    f.rect(bx0, by0, bx1, by1, SIGN);
    for x in [bx0 + 6, bx1 - 6] {
        f.rect(x, by1 + 1, x, ROOF_Y - 1, POST);
    }
    // Marquee bulbs chase around the edge.
    let mut edge = Vec::new();
    edge.extend((bx0..=bx1).map(|x| (x, by0)));
    edge.extend((by0 + 1..=by1).map(|y| (bx1, y)));
    edge.extend((bx0..bx1).rev().map(|x| (x, by1)));
    edge.extend((by0 + 1..by1).rev().map(|y| (bx0, y)));
    for (i, &(x, y)) in edge.iter().enumerate() {
        if i % 2 == 0 {
            let lit = (i / 2 + t as usize / 2) % 3 == 0;
            f.set(x, y, if lit { BULB } else { BULB_DIM });
        }
    }
    // One letter has a loose wire.
    let letters: Vec<usize> = text
        .char_indices()
        .filter(|(_, c)| *c != ' ')
        .map(|(i, _)| i)
        .collect();
    let loose = letters[(b.seed % letters.len() as u64) as usize];
    let flicker = matches!(t % 28, 3 | 4 | 6);
    font::draw(f, &text, cx - tw / 2, 7, 2, |i| {
        Some(if i == loose && flicker {
            NEON_OFF
        } else {
            NEON
        })
    });

    // The parapet, with snow on top.
    f.rect(l.x0 - 1, ROOF_Y, l.right() + 1, FLOORS_Y - 1, PARAPET);
    for x in l.x0 - 1..=l.right() + 1 {
        f.set(x, ROOF_Y, if x % 5 == 0 { SNOW_SHADE } else { SNOW });
        if hash(b.seed, x as u64) % 3 == 0 {
            f.set(x, ROOF_Y - 1, SNOW);
        }
    }
}

fn draw_facade(f: &mut Frame, l: &Layout) {
    f.rect(l.x0, FLOORS_Y, l.right(), LOBBY_Y - 1, WALL);
    // A lighter band where each floor's slab meets the wall, between the
    // windows only, so the windows stay the pattern.
    for floor in 0..7 {
        let y = FLOORS_Y + floor * PITCH_Y + 6;
        for x in (l.x0 + 2..l.right()).step_by(PITCH_X as usize) {
            f.set(x, y, WALL_LIGHT);
        }
    }
    f.rect(l.x0, FLOORS_Y, l.x0, SIDEWALK_Y - 1, WALL_DARK);
    f.rect(l.right(), FLOORS_Y, l.right(), SIDEWALK_Y - 1, WALL_DARK);
}

fn draw_windows(f: &mut Frame, b: &Building, l: &Layout, people: &[Walker], t: i32) {
    for (i, apt) in b.apartments.iter().enumerate() {
        let (x, y) = l.window(apt.week, apt.floor);
        let h = hash(b.seed, i as u64);

        // A new resident's light comes on once they get upstairs.
        let mut lit = apt.occupied();
        let mut flash = false;
        let mut new_here = false;
        if let Some(w) = people.iter().find(|w| w.home == Some(i)) {
            let on = lights_on(w, l, apt.floor);
            lit = t >= on;
            flash = (on..on + 2).contains(&t);
            new_here = true;
        }

        f.rect(x, y + 4, x + 2, y + 4, SILL);
        if h % 4 == 0 {
            f.set(x + (h >> 5) as i32 % 3, y + 4, SNOW_SHADE);
        }

        if !lit {
            f.rect(x, y, x + 2, y + 3, GLASS);
            if h % 3 == 0 {
                f.set(x, y, GLASS_HI);
            }
            continue;
        }
        if flash {
            f.rect(x, y, x + 2, y + 3, SNOW);
            continue;
        }

        let base = LIT[apt.light as usize];
        f.rect(x, y, x + 2, y + 3, base);
        if h % 5 == 2 {
            // Curtains.
            f.rect(x, y, x, y + 3, base.scale(0.72));
        }
        if new_here {
            // The new resident waves from the window.
            let px = x + 1;
            f.set(px, y + 2, RESIDENT);
            f.set(px, y + 3, RESIDENT);
            f.set(px + if t % 8 < 4 { -1 } else { 1 }, y + 1, RESIDENT);
        } else if h % 4 == 0 {
            let moves = h % 3 == 0;
            let step = if moves { t as u64 / 14 } else { 0 };
            let px = x + (((h >> 8) + step) % 3) as i32;
            f.set(px, y + 2, RESIDENT);
            f.set(px, y + 3, RESIDENT);
        }
        if h % 6 == 1 {
            let k = ((t as u64 / 3 + h) % 2) as usize;
            f.set(x + 2, y + 3, TV[k]);
        }
    }
}

fn draw_lobby(f: &mut Frame, b: &Building, l: &Layout, people: &[Walker], t: i32) {
    f.rect(l.x0, LOBBY_Y, l.right(), SIDEWALK_Y - 1, LOBBY);
    f.rect(l.x0 - 1, LOBBY_Y - 1, l.right() + 1, LOBBY_Y - 1, CORNICE);

    // The door opens for anyone walking in.
    let d = l.door;
    let open = people.iter().any(|w| {
        w.home.is_some() && {
            let e = w.enters(d);
            (e - 2..e + 3).contains(&t)
        }
    });
    f.rect(d - 4, 81, d + 4, SIDEWALK_Y - 1, WALL_DARK);
    if open {
        f.rect(d - 3, 82, d + 3, SIDEWALK_Y - 1, LIT[3]);
        f.rect(d - 3, 82, d - 3, SIDEWALK_Y - 1, DOOR_DARK);
    } else {
        f.rect(d - 3, 82, d + 3, SIDEWALK_Y - 1, DOOR);
        f.rect(d, 82, d, SIDEWALK_Y - 1, DOOR_DARK);
        f.rect(d - 2, 83, d - 1, 84, LIT[2]);
        f.rect(d + 1, 83, d + 2, 84, LIT[2]);
    }
    // An awning in the top language's color.
    let awning = b.coats[0];
    f.rect(d - 6, 79, d + 6, 80, awning);
    for x in (d - 6..=d + 6).step_by(2) {
        f.set(x, 81, awning.scale(0.72));
    }

    // Plaques: the login on the left, the residents on the right.
    let name = b.login.to_uppercase();
    let nw = font::width(&name, 1);
    let nx = d - 10 - nw;
    f.rect(nx - 2, 79, d - 9, 85, PLAQUE);
    font::draw(f, &name, nx, 80, 1, |_| Some(GOLD));

    let residents = b.residents();
    let label = if residents == 1 {
        "1 RESIDENT".to_string()
    } else {
        format!("{residents} RESIDENTS")
    };
    let rw = font::width(&label, 1);
    let rx = d + 10;
    f.rect(d + 9, 79, rx + rw + 1, 85, PLAQUE);
    font::draw(f, &label, rx, 80, 1, |_| Some(GOLD));

    // Lobby windows with plants, where there's room.
    for (x0, x1) in [(l.x0 + 4, nx - 7), (rx + rw + 6, l.right() - 4)] {
        if x1 - x0 < 8 {
            continue;
        }
        f.rect(x0, 79, x1, 85, LOBBY_GLOW);
        for x in (x0 + 3..x1 - 1).step_by(9) {
            f.rect(x, 83, x + 1, 85, PLANT);
            f.set(x - 1, 82, PLANT);
            f.set(x + 2, 82, PLANT);
        }
    }
}

fn draw_street(f: &mut Frame, l: &Layout, seed: u64) {
    f.rect(0, SIDEWALK_Y, WIDTH as i32 - 1, CURB_Y - 1, SIDEWALK);
    f.rect(0, CURB_Y, WIDTH as i32 - 1, CURB_Y, CURB);
    f.rect(0, CURB_Y + 1, WIDTH as i32 - 1, HEIGHT as i32 - 1, ROAD);
    for x in (0..WIDTH as i32).step_by(10) {
        f.rect(x + 2, 98, x + 6, 98, LANE);
    }
    // Snow along the buildings and the curb.
    let mut rng = Rng(seed ^ 0x5110);
    for x in 0..WIDTH as i32 {
        if rng.below(3) > 0 {
            f.set(x, SIDEWALK_Y, SNOW_SHADE);
        }
        if rng.below(4) == 0 {
            f.set(x, CURB_Y - 1, SNOW);
        }
    }
    // Street lamps either side of the building.
    for x in [l.x0 - 7, l.right() + 7] {
        for dx in -6..=6i32 {
            for y in SIDEWALK_Y..CURB_Y {
                if dx.abs() <= 6 - (CURB_Y - 1 - y) && (dx + y) % 2 == 0 {
                    f.set(x + dx, y, SIDEWALK_LIT);
                }
            }
        }
        f.rect(x, 72, x, CURB_Y - 1, POST);
        f.rect(x - 1, 70, x + 1, 71, LAMP);
        f.rect(x - 2, 69, x + 2, 69, POST);
    }
}

fn draw_walker(f: &mut Frame, w: &Walker, l: &Layout, t: i32) {
    let dir = if w.from_left { 1 } else { -1 };
    let (x, y_lift) = match w.home {
        Some(_) => {
            let enters = w.enters(l.door);
            if t < w.start || t >= enters {
                return;
            }
            let x = w.start_x() + dir * WALK_SPEED * (t - w.start);
            let x = if w.from_left {
                x.min(l.door)
            } else {
                x.max(l.door)
            };
            // A step up into the doorway.
            let lift = if enters - t <= 2 { 2 } else { 0 };
            (x, lift)
        }
        None => {
            let span = WIDTH as i32 + 8;
            (-4 + t * span / FRAMES as i32, 0)
        }
    };

    let top = FEET_Y - 6 - y_lift;
    f.set(x, top, w.hair);
    f.set(x - dir, top, w.hair);
    f.set(x, top + 1, w.skin);
    f.set(x - dir, top + 1, w.hair);
    f.rect(x - 1, top + 2, x + 1, top + 3, w.coat);
    f.set(x, top + 4, PANTS);
    if (t / 2) % 2 == 0 {
        f.set(x - 1, top + 5, PANTS);
        f.set(x + 1, top + 5, PANTS);
        f.set(x - 1, top + 6, PANTS);
        f.set(x + 1, top + 6, PANTS);
    } else {
        f.set(x, top + 5, PANTS);
        f.set(x, top + 6, PANTS);
    }
    if w.home.is_some() {
        // Carrying their things in a box.
        let bx = x + dir * 2;
        f.rect(bx - 1, top + 2, bx + 1, top + 4, BOX);
        f.rect(bx - 1, top + 2, bx + 1, top + 2, BOX_DARK);
    }
}

fn draw_snow(f: &mut Frame, seed: u64, t: i32) {
    let (w, h) = (WIDTH as i32, HEIGHT as i32);
    let mut rng = Rng(seed ^ 0xf1a4e);
    for _ in 0..34 {
        let (x0, y0) = (rng.below(w as u32), rng.below(h as u32));
        let near = rng.below(4) == 0;
        // Every flake falls a whole number of screens per loop.
        let fall = if near { 2 * h } else { h } * t / FRAMES as i32;
        let sway = if (t / 7 + x0) % 4 < 2 { 0 } else { 1 };
        let (x, y) = ((x0 + sway).rem_euclid(w), (y0 + fall).rem_euclid(h));
        if y < CURB_Y {
            f.set(x, y, if near { SNOW } else { SNOW_SHADE });
        }
    }
}
