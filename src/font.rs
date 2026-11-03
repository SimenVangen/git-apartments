//! A 3×5 pixel font for the signs.

use crate::canvas::Frame;
use crate::Rgb;

/// Glyph rows, top first; bit 2 is the left column.
fn glyph(c: char) -> [u8; 5] {
    match c.to_ascii_uppercase() {
        '0' => [7, 5, 5, 5, 7],
        '1' => [2, 6, 2, 2, 7],
        '2' => [7, 1, 7, 4, 7],
        '3' => [7, 1, 3, 1, 7],
        '4' => [5, 5, 7, 1, 1],
        '5' => [7, 4, 7, 1, 7],
        '6' => [7, 4, 7, 5, 7],
        '7' => [7, 1, 2, 2, 2],
        '8' => [7, 5, 7, 5, 7],
        '9' => [7, 5, 7, 1, 7],
        'A' => [2, 5, 7, 5, 5],
        'B' => [6, 5, 6, 5, 6],
        'C' => [3, 4, 4, 4, 3],
        'D' => [6, 5, 5, 5, 6],
        'E' => [7, 4, 6, 4, 7],
        'F' => [7, 4, 6, 4, 4],
        'G' => [3, 4, 5, 5, 3],
        'H' => [5, 5, 7, 5, 5],
        'I' => [7, 2, 2, 2, 7],
        'J' => [1, 1, 1, 5, 2],
        'K' => [5, 5, 6, 5, 5],
        'L' => [4, 4, 4, 4, 7],
        'M' => [5, 7, 7, 5, 5],
        'N' => [6, 5, 5, 5, 5],
        'O' => [2, 5, 5, 5, 2],
        'P' => [6, 5, 6, 4, 4],
        'Q' => [2, 5, 5, 6, 3],
        'R' => [6, 5, 6, 5, 5],
        'S' => [3, 4, 2, 1, 6],
        'T' => [7, 2, 2, 2, 2],
        'U' => [5, 5, 5, 5, 7],
        'V' => [5, 5, 5, 5, 2],
        'W' => [5, 5, 7, 7, 5],
        'X' => [5, 5, 2, 5, 5],
        'Y' => [5, 5, 2, 2, 2],
        'Z' => [7, 1, 2, 4, 7],
        '-' => [0, 0, 7, 0, 0],
        '/' => [1, 1, 2, 4, 4],
        '.' => [0, 0, 0, 0, 2],
        '!' => [2, 2, 2, 0, 2],
        _ => [0; 5],
    }
}

/// The width in pixels of `text` drawn at `size`.
pub(crate) fn width(text: &str, size: i32) -> i32 {
    let n = text.chars().count() as i32;
    if n == 0 {
        0
    } else {
        (n * 4 - 1) * size
    }
}

/// Draws `text` with its top left at `(x, y)`. `color` picks each
/// character's color by its position, which lets a sign flicker one
/// letter at a time.
pub(crate) fn draw(
    f: &mut Frame,
    text: &str,
    x: i32,
    y: i32,
    size: i32,
    color: impl Fn(usize) -> Option<Rgb>,
) {
    for (i, ch) in text.chars().enumerate() {
        let Some(c) = color(i) else { continue };
        let gx = x + i as i32 * 4 * size;
        for (row, bits) in glyph(ch).iter().enumerate() {
            for col in 0..3 {
                if bits & (4 >> col) != 0 {
                    let px = gx + col * size;
                    let py = y + row as i32 * size;
                    f.rect(px, py, px + size - 1, py + size - 1, c);
                }
            }
        }
    }
}
