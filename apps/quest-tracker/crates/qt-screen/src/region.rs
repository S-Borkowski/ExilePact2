//! The three regions the reader may copy, where they sit in the client rect,
//! and how often each is read while demanded.

use crate::geometry::{PixelRect, Rect};
use crate::Millis;

/// The boss bar's band. The name sits near its middle (x 0.456 to 0.544 for
/// the Miller) but moves with some panels open (x 0.28 with the inventory).
pub const BOSS_BAR_RECT: Rect = Rect::new(0.25, 0.00, 0.75, 0.14);

/// The lower-left list of voiced lines, `<speaker>: <text>` from x 0.03, the
/// newest at about y 0.72.
pub const VOICED_LINES_RECT: Rect = Rect::new(0.00, 0.35, 0.45, 0.80);

/// The dialogue window: its heading, its text block (shifted or not), the
/// banner and the topic list, left of the overlay's default place.
pub const DIALOGUE_WINDOW_RECT: Rect = Rect::new(0.22, 0.20, 0.70, 0.85);

/// How often the bar's band is read while demanded (2 Hz).
pub const BOSS_BAR_PERIOD_MS: Millis = 500;

/// How often the list of voiced lines is read while demanded (2 Hz).
pub const VOICED_LINES_PERIOD_MS: Millis = 500;

/// How often the dialogue window is read while demanded (1 Hz).
pub const DIALOGUE_WINDOW_PERIOD_MS: Millis = 1000;

/// A region of the game's client rect the reader may copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Region {
    BossBar,
    VoicedLines,
    DialogueWindow,
}

impl Region {
    /// Every region, in the order status and diagnostic lines name them.
    pub const ALL: [Region; 3] = [Region::BossBar, Region::VoicedLines, Region::DialogueWindow];

    /// The name diagnostic and status lines use.
    pub fn name(self) -> &'static str {
        match self {
            Region::BossBar => "boss bar",
            Region::VoicedLines => "voiced lines",
            Region::DialogueWindow => "dialogue window",
        }
    }

    /// Where it sits, as fractions of the client rect.
    pub fn rect(self) -> Rect {
        match self {
            Region::BossBar => BOSS_BAR_RECT,
            Region::VoicedLines => VOICED_LINES_RECT,
            Region::DialogueWindow => DIALOGUE_WINDOW_RECT,
        }
    }

    /// How long the reader waits between two reads of it.
    pub fn period_ms(self) -> Millis {
        match self {
            Region::BossBar => BOSS_BAR_PERIOD_MS,
            Region::VoicedLines => VOICED_LINES_PERIOD_MS,
            Region::DialogueWindow => DIALOGUE_WINDOW_PERIOD_MS,
        }
    }

    /// Where it sits in screen pixels, for the client rect `client`.
    pub fn pixels(self, client: PixelRect) -> PixelRect {
        self.rect().to_pixels(client)
    }

    /// Whether the overlay's window, at `overlay` in screen pixels, covers
    /// any of this region. A region the overlay covers is not visible.
    pub fn covered_by(self, client: PixelRect, overlay: PixelRect) -> bool {
        self.pixels(client).intersects(&overlay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_have_their_names_places_and_rates() {
        assert_eq!(Region::BossBar.name(), "boss bar");
        assert_eq!(Region::VoicedLines.name(), "voiced lines");
        assert_eq!(Region::DialogueWindow.name(), "dialogue window");
        assert_eq!(Region::BossBar.rect(), Rect::new(0.25, 0.0, 0.75, 0.14));
        assert_eq!(Region::VoicedLines.rect(), Rect::new(0.0, 0.35, 0.45, 0.80));
        assert_eq!(
            Region::DialogueWindow.rect(),
            Rect::new(0.22, 0.20, 0.70, 0.85)
        );
        assert_eq!(Region::BossBar.period_ms(), 500);
        assert_eq!(Region::VoicedLines.period_ms(), 500);
        assert_eq!(Region::DialogueWindow.period_ms(), 1000);
    }

    #[test]
    fn the_overlay_covers_a_region_it_overlaps() {
        let client = PixelRect::new(0, 0, 2560, 1440);
        // The overlay at its default place, right of the dialogue window.
        let right = PixelRect::new(1800, 300, 2200, 900);
        assert!(!Region::DialogueWindow.covered_by(client, right));
        assert!(!Region::BossBar.covered_by(client, right));
        // Dragged over the bar's band.
        let top = PixelRect::new(1000, 0, 1400, 300);
        assert!(Region::BossBar.covered_by(client, top));
        assert!(Region::DialogueWindow.covered_by(client, top));
        assert!(!Region::VoicedLines.covered_by(client, top));
    }
}
