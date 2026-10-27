//! The panel table the app ships: which game panels the overlay hides under,
//! where each draws its frame, and the colours its probes expect.
//!
//! The entries are measured, never guessed: footprints and probes come from
//! `quest-tracker-replay panels measure` over a no-kill live session's kept
//! capture, and the tests here check them against the numbers sampled from
//! it. Until that session has run, the table is empty, and an empty table
//! hides the overlay under nothing.

use crate::panels::PanelTable;

/// The table the panel watcher uses unless its config hands it another.
pub fn shipped_panels() -> PanelTable {
    PanelTable::new(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_panels_is_empty_until_measured() {
        assert!(shipped_panels().is_empty());
    }
}
