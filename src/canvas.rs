//! A pixel canvas, a small random number generator, and the GIF encoder.

use std::borrow::Cow;
use std::collections::BTreeSet;
use std::io::Write;

use crate::{Error, Rgb};

/// One frame: a grid of pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
    /// Pixels, row by row from the top left.
    pub pixels: Vec<Rgb>,
}

impl Frame {
    /// A frame filled with one color.
    pub fn new(width: usize, height: usize, fill: Rgb) -> Self {
        Frame {
            width,
            height,
            pixels: vec![fill; width * height],
        }
    }

    /// The pixel at `(x, y)`, if it is inside the frame.
    pub fn get(&self, x: i32, y: i32) -> Option<Rgb> {
        self.index(x, y).map(|i| self.pixels[i])
    }

    /// Sets the pixel at `(x, y)`. Points outside the frame are ignored.
    pub fn set(&mut self, x: i32, y: i32, c: Rgb) {
        if let Some(i) = self.index(x, y) {
            self.pixels[i] = c;
        }
    }

    /// Fills the rectangle from `(x0, y0)` to `(x1, y1)`, inclusive.
    pub fn rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: Rgb) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.set(x, y, c);
            }
        }
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height)
            .then(|| y as usize * self.width + x as usize)
    }
}

/// SplitMix64: small, fast and the same everywhere.
pub(crate) struct Rng(pub(crate) u64);

impl Rng {
    pub(crate) fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    pub(crate) fn below(&mut self, n: u32) -> i32 {
        (self.next() % n.max(1) as u64) as i32
    }
}

/// Mixes two numbers into a well-spread one.
pub(crate) fn hash(a: u64, b: u64) -> u64 {
    Rng(a ^ b.wrapping_mul(0x9e37_79b9_7f4a_7c15)).next()
}

/// Encodes frames as a GIF that loops forever, each pixel scaled up to a
/// `scale` × `scale` block and each frame shown for `delay` hundredths of
/// a second.
///
/// All frames must be the same size and use 255 colors or fewer between
/// them. After the first frame, only the pixels that changed are stored,
/// which keeps a mostly still scene small.
pub fn encode_gif<W: Write>(frames: &[Frame], scale: u16, delay: u16, out: W) -> Result<(), Error> {
    let first = frames
        .first()
        .ok_or_else(|| Error::Encode("there are no frames".into()))?;
    let (w, h) = (first.width, first.height);
    if frames.iter().any(|f| f.width != w || f.height != h) {
        return Err(Error::Encode("frames differ in size".into()));
    }
    let scale = scale.max(1) as usize;
    let (sw, sh) = (w * scale, h * scale);
    let gw = u16::try_from(sw).map_err(|_| Error::Encode("the GIF would be too wide".into()))?;
    let gh = u16::try_from(sh).map_err(|_| Error::Encode("the GIF would be too tall".into()))?;

    let colors: BTreeSet<Rgb> = frames
        .iter()
        .flat_map(|f| f.pixels.iter().copied())
        .collect();
    if colors.len() > 255 {
        return Err(Error::Encode(format!(
            "the frames use {} colors; this GIF holds 255",
            colors.len()
        )));
    }
    let colors: Vec<Rgb> = colors.into_iter().collect();
    // The last palette entry marks pixels that didn't change.
    let unchanged = colors.len() as u8;
    let mut palette: Vec<u8> = colors.iter().flat_map(|c| [c.0, c.1, c.2]).collect();
    palette.extend([0, 0, 0]);

    let mut enc = gif::Encoder::new(out, gw, gh, &palette).map_err(enc_err)?;
    enc.set_repeat(gif::Repeat::Infinite).map_err(enc_err)?;

    let mut buf = vec![0u8; sw * sh];
    let mut previous: Option<&Frame> = None;
    for frame in frames {
        for y in 0..sh {
            for x in 0..sw {
                let i = (y / scale) * w + x / scale;
                let c = frame.pixels[i];
                buf[y * sw + x] = match previous {
                    Some(p) if p.pixels[i] == c => unchanged,
                    _ => colors.binary_search(&c).expect("color is in palette") as u8,
                };
            }
        }
        let gf = gif::Frame {
            width: gw,
            height: gh,
            delay,
            dispose: gif::DisposalMethod::Keep,
            transparent: previous.map(|_| unchanged),
            buffer: Cow::Borrowed(&buf),
            ..gif::Frame::default()
        };
        enc.write_frame(&gf).map_err(enc_err)?;
        previous = Some(frame);
    }
    Ok(())
}

fn enc_err(e: impl std::fmt::Display) -> Error {
    Error::Encode(e.to_string())
}
