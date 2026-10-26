// Colour for the terminal face.
//
// Most of a face is `Ink::Plain`, which emits no escape at all — it inherits
// the terminal's foreground, so the face is themed by stylix along with
// everything else on this machine and follows `theme set` for free. Only the
// deliberate accents (a red pupil on Trouble, a green check) name a colour.
//
// Those come from the environment rather than being compiled in, so
// home/parker can hand in the active preset's own accent — the same trick
// home/tuis/music.nix uses to colour cava's gradient from the theme.

use super::pose::Ink;
use super::Frame;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

pub struct Palette {
    accent: Rgb,
    good: Rgb,
    bad: Rgb,
}

impl Default for Palette {
    fn default() -> Palette {
        Palette {
            // Only used if the environment says nothing. Deliberately muted:
            // a face that shouts in colours the desktop never uses looks
            // pasted on.
            accent: Rgb(0xFF, 0x46, 0x53),
            good: Rgb(0x8F, 0xB0, 0x88),
            bad: Rgb(0xE0, 0x4B, 0x4B),
        }
    }
}

impl Palette {
    pub fn from_env() -> Palette {
        let d = Palette::default();
        Palette {
            accent: env_rgb("PARKER_ACCENT").unwrap_or(d.accent),
            good: env_rgb("PARKER_GOOD").unwrap_or(d.good),
            bad: env_rgb("PARKER_BAD").unwrap_or(d.bad),
        }
    }

    fn of(&self, ink: u8) -> Option<Rgb> {
        match ink {
            x if x == Ink::Accent.slot() => Some(self.accent),
            x if x == Ink::Good.slot() => Some(self.good),
            x if x == Ink::Bad.slot() => Some(self.bad),
            // Plain, and anything unrecognised, inherits the terminal.
            _ => None,
        }
    }
}

fn env_rgb(key: &str) -> Option<Rgb> {
    let raw = std::env::var(key).ok()?;
    let hex = raw.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    Some(Rgb(
        u8::from_str_radix(&hex[0..2], 16).ok()?,
        u8::from_str_radix(&hex[2..4], 16).ok()?,
        u8::from_str_radix(&hex[4..6], 16).ok()?,
    ))
}

/// Render a frame with colour, emitting an escape only when the ink actually
/// changes. A run-length approach rather than per-cell: a face is mostly one
/// colour, and per-cell escapes would multiply the bytes written 25 times a
/// second by an order of magnitude for nothing.
pub fn ansi(frame: &Frame, palette: &Palette) -> Vec<String> {
    frame
        .rows
        .iter()
        .enumerate()
        .map(|(r, row)| {
            let mut out = String::with_capacity(row.len() + 16);
            let mut current: Option<u8> = None;
            for (c, ch) in row.chars().enumerate() {
                let ink = frame
                    .inks
                    .get(r * super::COLS as usize + c)
                    .copied()
                    .unwrap_or(0);
                if current != Some(ink) {
                    match palette.of(ink) {
                        Some(Rgb(r8, g8, b8)) => {
                            out.push_str(&format!("\x1b[38;2;{r8};{g8};{b8}m"))
                        }
                        None => out.push_str("\x1b[39m"),
                    }
                    current = Some(ink);
                }
                out.push(ch);
            }
            // Never leave a colour set past the end of a row.
            out.push_str("\x1b[39m");
            out
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // These two set an environment variable, which is a process-global write
    // in a threaded test runner: sharing one name between them means each can
    // observe the other's value mid-assertion. It did — the `not-a-colour`
    // test read `8fb088` and failed, roughly one run in three. A name per
    // test is the whole fix; the comment is so nobody merges them back.

    #[test]
    fn hex_parses_with_or_without_a_hash() {
        // Safety: no other test reads or writes this name.
        unsafe { std::env::set_var("PARKER_TEST_COLOR_HEX", "#FF4653") };
        assert_eq!(
            env_rgb("PARKER_TEST_COLOR_HEX"),
            Some(Rgb(0xFF, 0x46, 0x53))
        );
        unsafe { std::env::set_var("PARKER_TEST_COLOR_HEX", "8fb088") };
        assert_eq!(
            env_rgb("PARKER_TEST_COLOR_HEX"),
            Some(Rgb(0x8F, 0xB0, 0x88))
        );
        unsafe { std::env::remove_var("PARKER_TEST_COLOR_HEX") };
    }

    #[test]
    fn nonsense_falls_back_rather_than_failing() {
        // Safety: no other test reads or writes this name.
        unsafe { std::env::set_var("PARKER_TEST_COLOR_NONSENSE", "not-a-colour") };
        assert_eq!(env_rgb("PARKER_TEST_COLOR_NONSENSE"), None);
        unsafe { std::env::remove_var("PARKER_TEST_COLOR_NONSENSE") };
        assert_eq!(env_rgb("PARKER_TEST_ABSENT"), None);
    }

    #[test]
    fn plain_ink_emits_no_colour_so_the_theme_shows_through() {
        let p = Palette::default();
        assert_eq!(p.of(Ink::Plain.slot()), None);
        assert!(p.of(Ink::Good.slot()).is_some());
    }

    #[test]
    fn a_single_colour_face_sets_colour_once_per_row() {
        let f = crate::face::draw(
            crate::face::Expression::Idle.pose(),
            crate::face::Expression::Idle.motion(),
            0.5,
            0.0,
        );
        let rows = ansi(&f, &Palette::default());
        for row in &rows {
            assert!(
                row.matches("\x1b[").count() <= 2,
                "one set plus one reset expected, got {}",
                row.matches("\x1b[").count()
            );
        }
    }

    #[test]
    fn an_accented_face_actually_emits_a_colour() {
        let f = crate::face::draw(
            crate::face::Expression::Yes.pose(),
            crate::face::Expression::Yes.motion(),
            0.5,
            0.0,
        );
        let joined = ansi(&f, &Palette::default()).join("");
        assert!(joined.contains("\x1b[38;2;"), "the check should be inked");
    }

    #[test]
    fn every_row_ends_uncoloured() {
        let f = crate::face::draw(
            crate::face::Expression::No.pose(),
            crate::face::Expression::No.motion(),
            0.5,
            0.0,
        );
        for row in ansi(&f, &Palette::default()) {
            assert!(row.ends_with("\x1b[39m"), "colour leaked past a row");
        }
    }
}
