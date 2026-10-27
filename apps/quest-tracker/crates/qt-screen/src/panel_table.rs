//! The panel table the app ships: which game panels the overlay hides under,
//! where each draws its frame, and the colours its probes expect.
//!
//! The entries are measured, never guessed. Each probe is one patch of
//! `quest-tracker-replay panels measure`'s 160 × 90 grid of the client rect
//! (16 px square at 2560 × 1440), chosen from the candidates it printed over
//! the no-kill live session of 2026-10-27 (the research note's Live 12), and
//! its limits follow the margin rules for raw copies: per channel the larger
//! of 3 × the patch's spread and 6 levels, never more than half its margin;
//! a largest luma spread of 2 × its luma spread + 8, at least 16; and no
//! patch so dark that its colour less its largest tolerance is below luma 16.
//! Each footprint is the panel's own extent as its frames show it.
//!
//! `apps/quest-tracker/fixtures/panels/shipped_panels.json` holds the numbers
//! every limit was set from and the probe statistics of sampled frames, open
//! and closed, from that session and from Live 8's capture (the hold-out the
//! table was not chosen on). The tests here paint synthetic copies from those
//! numbers; no frame or pixel is kept.
//!
//! The stash's and a merchant's windows share the inventory's entry: their
//! own left-hand windows have no entry, so an overlay moved to the left does
//! not hide under them. The character panel (C) is not one of the owner's
//! panels and has no entry.

use crate::geometry::Rect;
use crate::panels::{PanelEntry, PanelProbe, PanelTable};

/// The patch at `col`, `row` of `measure`'s 160 × 90 grid, as fractions.
fn cell(col: u32, row: u32) -> Rect {
    let (c, r) = (col as f64, row as f64);
    Rect::new(c / 160.0, r / 90.0, (c + 1.0) / 160.0, (r + 1.0) / 90.0)
}

fn probe(rect: Rect, bgr: [f64; 3], tolerance: [f64; 3], max_luma_std: f64) -> PanelProbe {
    PanelProbe {
        rect,
        bgr,
        tolerance,
        max_luma_std,
    }
}

/// The table the panel watcher uses unless its config hands it another.
pub fn shipped_panels() -> PanelTable {
    PanelTable::new(vec![
        // The inventory's header plate; the stash and a merchant open it on
        // the right beside their own window.
        PanelEntry {
            id: "inventory",
            opens_for: vec!["inventory", "stash", "merchant"],
            footprint: Rect::new(0.65, 0.0, 1.0, 0.845),
            probes: vec![
                probe(cell(120, 18), [118.0, 143.0, 147.0], [6.0, 6.0, 6.0], 20.0),
                probe(cell(124, 19), [92.0, 104.0, 108.0], [6.0, 6.0, 6.0], 22.0),
                probe(cell(123, 22), [78.0, 89.0, 91.0], [6.0, 6.0, 6.0], 20.0),
                probe(cell(126, 11), [82.0, 95.0, 99.0], [6.0, 6.0, 6.0], 30.0),
                probe(cell(120, 10), [97.0, 113.0, 117.0], [6.0, 6.0, 6.0], 36.0),
                probe(cell(141, 10), [137.0, 189.0, 202.0], [6.0, 6.0, 6.0], 80.0),
            ],
            min_match: 3,
        },
        // The passive tree's title plate, the same for every character, and
        // one patch of its frame on the right.
        PanelEntry {
            id: "passive-tree",
            opens_for: vec!["passive-tree"],
            footprint: Rect::new(0.0, 0.0, 1.0, 1.0),
            probes: vec![
                probe(cell(73, 2), [158.0, 228.0, 245.0], [6.0, 6.0, 6.0], 34.0),
                probe(cell(86, 2), [125.0, 191.0, 219.0], [6.0, 6.0, 6.0], 64.0),
                probe(cell(89, 2), [93.0, 151.0, 177.0], [6.0, 6.0, 6.0], 78.0),
                probe(cell(70, 2), [87.0, 145.0, 168.0], [6.0, 6.0, 6.0], 94.0),
                probe(cell(145, 12), [104.0, 117.0, 160.0], [6.0, 6.0, 6.0], 96.0),
            ],
            min_match: 3,
        },
        // M: the microtransactions shop, which covers the whole client.
        PanelEntry {
            id: "key-m",
            opens_for: vec!["key-m"],
            footprint: Rect::new(0.0, 0.0, 1.0, 1.0),
            probes: vec![
                probe(cell(9, 27), [38.0, 43.0, 44.0], [16.0, 16.0, 9.0], 18.0),
                probe(cell(8, 24), [35.0, 39.0, 40.0], [13.0, 13.0, 13.0], 16.0),
                probe(cell(12, 23), [31.0, 36.0, 37.0], [11.5, 11.5, 11.5], 16.0),
                probe(cell(9, 62), [37.0, 42.0, 38.0], [9.0, 9.0, 9.0], 16.0),
                probe(cell(14, 5), [22.0, 32.0, 36.0], [10.0, 10.0, 10.0], 16.0),
                probe(cell(140, 9), [17.0, 51.0, 73.0], [15.5, 15.5, 15.5], 24.0),
                probe(cell(150, 8), [17.0, 34.0, 59.0], [12.0, 6.0, 12.0], 58.0),
            ],
            min_match: 3,
        },
        // The top of the Escape menu's box: static art over the dimmed
        // client.
        PanelEntry {
            id: "escape-menu",
            opens_for: vec!["escape-menu"],
            footprint: Rect::new(0.0, 0.0, 1.0, 1.0),
            probes: vec![
                probe(cell(72, 32), [66.0, 92.0, 95.0], [6.0, 6.0, 6.0], 178.0),
                probe(cell(75, 32), [57.0, 79.0, 82.0], [6.0, 6.0, 6.0], 162.0),
                probe(cell(71, 25), [53.0, 73.0, 76.0], [6.0, 6.0, 6.0], 170.0),
                probe(cell(78, 32), [56.0, 78.0, 81.0], [6.0, 6.0, 6.0], 166.0),
                probe(cell(84, 32), [63.0, 87.0, 91.0], [6.0, 6.0, 6.0], 172.0),
            ],
            min_match: 3,
        },
        // O: the options, a panel on the left; its header and its foot.
        PanelEntry {
            id: "key-o",
            opens_for: vec!["key-o"],
            footprint: Rect::new(0.0, 0.0, 0.345, 0.8),
            probes: vec![
                probe(cell(22, 5), [160.0, 232.0, 245.0], [6.0, 6.0, 6.0], 36.0),
                probe(cell(32, 5), [127.0, 201.0, 228.0], [6.0, 6.0, 6.0], 46.0),
                probe(cell(35, 5), [124.0, 193.0, 215.0], [6.0, 6.0, 6.0], 64.0),
                probe(cell(14, 70), [28.0, 30.0, 30.0], [6.0, 6.0, 6.0], 16.0),
                probe(cell(27, 67), [68.0, 96.0, 99.0], [6.0, 6.0, 6.0], 174.0),
            ],
            min_match: 3,
        },
        // U: the world map, over the whole client; its title plate and a
        // patch of its frame.
        PanelEntry {
            id: "key-u",
            opens_for: vec!["key-u"],
            footprint: Rect::new(0.0, 0.0, 1.0, 1.0),
            probes: vec![
                probe(cell(74, 2), [138.0, 201.0, 219.0], [6.0, 6.0, 6.0], 26.0),
                probe(cell(84, 2), [112.0, 173.0, 204.0], [6.0, 6.0, 6.0], 30.0),
                probe(cell(71, 2), [114.0, 175.0, 196.0], [6.0, 6.0, 6.0], 46.0),
                probe(cell(87, 2), [104.0, 164.0, 191.0], [6.0, 6.0, 6.0], 52.0),
                probe(cell(22, 24), [102.0, 126.0, 135.0], [6.0, 6.0, 6.0], 36.0),
                probe(cell(26, 24), [86.0, 110.0, 121.0], [6.0, 6.0, 6.0], 36.0),
            ],
            min_match: 3,
        },
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observation::Image;
    use crate::panels::{PanelRead, ProbeStats, DEFAULT_OVERLAY_REACH, PANEL_IDS};
    use serde_json::Value;

    const FIXTURE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/panels/shipped_panels.json"
    );

    fn fixture() -> Value {
        let raw = std::fs::read_to_string(FIXTURE).expect("the panel fixture");
        serde_json::from_str(&raw).expect("JSON")
    }

    fn num(v: &Value) -> f64 {
        v.as_f64().unwrap_or_else(|| panic!("not a number: {v}"))
    }

    fn triple(v: &Value) -> [f64; 3] {
        let a = v.as_array().unwrap_or_else(|| panic!("not a list: {v}"));
        assert_eq!(a.len(), 3, "{v}");
        [num(&a[0]), num(&a[1]), num(&a[2])]
    }

    fn luma(bgr: [f64; 3]) -> f64 {
        0.114 * bgr[0] + 0.587 * bgr[1] + 0.299 * bgr[2]
    }

    /// A 16 × 16 copy whose mean is `mean` (blue, green, red) and whose
    /// luma spread is `luma_std`: two levels, the same offset in every
    /// channel, in the share of pixels that keeps both inside 0..255.
    fn paint(mean: [f64; 3], luma_std: f64) -> Image {
        const N: usize = 256;
        let lo = mean.iter().cloned().fold(f64::MAX, f64::min);
        let hi = mean.iter().cloned().fold(f64::MIN, f64::max);
        // `k` pixels at `+up`, the rest at `-down`: k * up = (N - k) * down
        // keeps the mean, and the variance is up * down.
        let (k, up, down) = (1..N)
            .map(|k| {
                let p = k as f64 / N as f64;
                (
                    k,
                    luma_std * ((1.0 - p) / p).sqrt(),
                    luma_std * (p / (1.0 - p)).sqrt(),
                )
            })
            .filter(|&(_, up, down)| hi + up <= 255.0 && lo - down >= 0.0)
            .min_by(|a, b| {
                let (pa, pb) = (a.0 as f64 - N as f64 / 2.0, b.0 as f64 - N as f64 / 2.0);
                pa.abs().total_cmp(&pb.abs())
            })
            .unwrap_or_else(|| panic!("cannot paint a spread of {luma_std}"));
        let level = |d: f64| -> [u8; 4] {
            let c = |x: f64| (x + d).round().clamp(0.0, 255.0) as u8;
            [c(mean[0]), c(mean[1]), c(mean[2]), 255]
        };
        let (a, b) = (level(up), level(-down));
        let bgra = (0..N).flat_map(|i| if i < k { a } else { b }).collect();
        Image {
            width: 16,
            height: 16,
            bgra,
        }
    }

    /// The copies of `entry`'s probes a sample records, painted.
    fn copies(sample: &Value, entry: &PanelEntry) -> Vec<Option<ProbeStats>> {
        let probes = sample["probes"][entry.id]
            .as_array()
            .unwrap_or_else(|| panic!("{} has no {} probes", sample["utc"], entry.id));
        assert_eq!(probes.len(), entry.probes.len(), "{}", sample["utc"]);
        probes
            .iter()
            .map(|p| {
                let (mean, std) = (triple(&p["mean_bgr"]), num(&p["luma_std"]));
                let s = ProbeStats::of_image(&paint(mean, std));
                // The painted copy holds the recorded numbers.
                for c in 0..3 {
                    assert!((s.mean_bgr[c] - mean[c]).abs() <= 0.6, "{p} {s:?}");
                }
                assert!((s.luma_std - std).abs() <= 0.6, "{p}");
                Some(s)
            })
            .collect()
    }

    fn entry<'a>(table: &'a PanelTable, id: &str) -> &'a PanelEntry {
        table
            .entries
            .iter()
            .find(|e| e.id == id)
            .unwrap_or_else(|| panic!("no entry {id}"))
    }

    fn samples(kind: &str) -> Vec<Value> {
        fixture()["samples"]
            .as_array()
            .expect("samples")
            .iter()
            .filter(|s| s["kind"] == kind)
            .cloned()
            .collect()
    }

    const NOWHERE: Rect = Rect::new(0.0, 0.0, 0.0, 0.0);

    #[test]
    fn shipped_panels_open_on_the_measured_open_samples_target() {
        let table = shipped_panels();
        let open = samples("open");
        assert!(open.len() >= 20, "{}", open.len());
        let mut seen: Vec<String> = Vec::new();
        for s in &open {
            let id = s["entry"].as_str().expect("an open sample's entry");
            let e = entry(&table, id);
            // The window it was sampled in is one this entry stands for.
            let panel = s["window"]["panel"].as_str().expect("its window");
            assert!(e.opens_for.contains(&panel), "{id} {panel}");
            assert_eq!(s["reads"][id], "open", "{}", s["utc"]);
            let read = e.classify(&NOWHERE, false, &copies(s, e));
            assert_eq!(read, PanelRead::Open, "{id} at {}", s["utc"]);
            seen.push(format!("{}/{panel}", s["session"].as_str().unwrap()));
        }
        // Every entry has open samples, and both sessions are sampled.
        for e in &table.entries {
            assert!(open.iter().any(|s| s["entry"] == e.id), "{}", e.id);
        }
        for want in [
            "live-1/inventory",
            "live-1/stash",
            "live-1/merchant",
            "live-8/inventory",
        ] {
            assert!(seen.iter().any(|s| s == want), "{want}: {seen:?}");
        }
        for want in ["live-8/passive-tree", "live-8/escape-menu"] {
            assert!(seen.iter().any(|s| s == want), "{want}: {seen:?}");
        }
    }

    #[test]
    fn shipped_panels_stay_closed_on_the_measured_closed_samples_baseline() {
        let table = shipped_panels();
        let closed = samples("closed");
        assert!(closed.len() >= 40, "{}", closed.len());
        for session in ["live-1", "live-8"] {
            assert!(closed.iter().filter(|s| s["session"] == session).count() >= 20);
        }
        for s in &closed {
            for e in &table.entries {
                assert_eq!(s["reads"][e.id], "closed", "{} at {}", e.id, s["utc"]);
                let read = e.classify(&NOWHERE, false, &copies(s, e));
                assert_eq!(read, PanelRead::Closed, "{} at {}", e.id, s["utc"]);
            }
        }
        // The character panel's windows are among them: C hides nothing.
        assert!(closed.iter().any(|s| s["window"]["panel"] == "character"));
    }

    #[test]
    fn shipped_panels_are_seen_beside_the_default_overlay() {
        for e in &shipped_panels().entries {
            let readable = e.readable_probes(&DEFAULT_OVERLAY_REACH, true);
            assert!(
                readable.len() >= e.min_match && e.probes.len() >= 3,
                "{}: {readable:?}",
                e.id
            );
            assert!(e.min_match >= 2, "{}", e.id);
        }
    }

    #[test]
    fn shipped_panels_cover_only_the_owners_list() {
        let table = shipped_panels();
        let covered: Vec<&str> = table
            .entries
            .iter()
            .flat_map(|e| e.opens_for.iter().copied())
            .collect();
        for id in &covered {
            assert!(PANEL_IDS.contains(id), "{id}");
        }
        for id in ["inventory", "passive-tree", "escape-menu", "stash"] {
            assert!(covered.contains(&id), "{id}");
        }
        assert!(!covered.contains(&"character"));
        let mut ids: Vec<&str> = table.entries.iter().map(|e| e.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), table.entries.len());
    }

    #[test]
    fn shipped_inventory_is_not_under_a_left_overlay() {
        let left = Rect::new(0.0, 0.30, 0.52, 0.80);
        let table = shipped_panels();
        let inventory: Vec<&PanelEntry> = table
            .entries
            .iter()
            .filter(|e| e.opens_for.contains(&"inventory"))
            .collect();
        assert!(!inventory.is_empty());
        for e in inventory {
            assert!(!e.is_under(&left), "{}", e.id);
            // Positive control: it is under the overlay's default place.
            assert!(e.is_under(&DEFAULT_OVERLAY_REACH), "{}", e.id);
        }
    }

    #[test]
    fn shipped_panels_leave_room_for_raw_copies() {
        let table = shipped_panels();
        let data = fixture();
        let entries = data["entries"].as_array().expect("entries");
        assert_eq!(entries.len(), table.entries.len());
        for (f, e) in entries.iter().zip(&table.entries) {
            assert_eq!(f["id"], e.id);
            assert_eq!(num(&f["min_match"]) as usize, e.min_match, "{}", e.id);
            let probes = f["probes"].as_array().expect("probes");
            assert_eq!(probes.len(), e.probes.len(), "{}", e.id);
            for (fp, p) in probes.iter().zip(&e.probes) {
                let m = &fp["measured"];
                let (centre, margin) = (triple(&m["bgr"]), num(&m["margin"]));
                let luma_std = num(&m["luma_std"]);
                // The fixture describes the shipped probe.
                let (col, row) = (num(&fp["cell"]["col"]), num(&fp["cell"]["row"]));
                assert_eq!(p.rect, cell(col as u32, row as u32), "{} {fp}", e.id);
                assert_eq!(p.bgr, centre, "{} {fp}", e.id);
                assert_eq!(p.tolerance, triple(&fp["tolerance"]), "{} {fp}", e.id);
                assert_eq!(p.max_luma_std, num(&fp["max_luma_std"]), "{} {fp}", e.id);
                // The margin rules.
                for t in p.tolerance {
                    assert!(t >= 6.0 && t <= margin / 2.0, "{} {fp}", e.id);
                }
                assert!(p.max_luma_std >= 2.0 * luma_std + 4.0, "{} {fp}", e.id);
                let widest = p.tolerance.iter().cloned().fold(0.0, f64::max);
                assert!(luma(centre) - widest >= 16.0, "{} {fp}", e.id);
            }
        }
    }

    #[test]
    fn shipped_panels_painted_copies_keep_their_numbers() {
        // The painter's own control: a flat copy, a spread one, and a dark
        // one keep their mean and spread; the dark flat one is black.
        for (mean, std) in [
            ([40.0, 110.0, 150.0], 0.0),
            ([66.0, 92.0, 95.0], 85.0),
            ([3.0, 4.0, 2.0], 0.5),
        ] {
            let s = ProbeStats::of_image(&paint(mean, std));
            for c in 0..3 {
                assert!((s.mean_bgr[c] - mean[c]).abs() <= 0.6, "{mean:?} {std}");
            }
            assert!(
                (s.luma_std - std).abs() <= 0.6,
                "{mean:?} {std} {}",
                s.luma_std
            );
        }
        assert!(paint([3.0, 4.0, 2.0], 0.5).is_black());
        assert!(!paint([40.0, 110.0, 150.0], 0.0).is_black());
    }
}
