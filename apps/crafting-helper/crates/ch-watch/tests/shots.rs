//! The owner's tooltip screenshots, read as the app reads the game: decoded
//! with `qt-capture`'s decoder, the tooltips found by the locator, and their
//! crops read by Windows' own OCR (en-US), the backend the app uses.
//!
//! The images and their item copies stay on the owner's machine, never in a
//! repository: these tests read them from the folder `CH_TOOLTIP_SHOTS`
//! names and are `#[ignore]`d, so the default run needs no image. Run them
//! with
//!
//! ```text
//! CH_TOOLTIP_SHOTS="$USERPROFILE/crafting-helper-shots" cargo test -p ch-watch --test shots -- --include-ignored
//! ```
//!
//! `tests/shots.txt` lists the shots, their kinds and the hand-measured
//! tooltip boxes. Nothing here prints or keeps a line's text: the messages
//! carry file names, counts and read kinds.

#![cfg(windows)]

mod common;

use std::path::PathBuf;

use ch_engine::ch_item::{Item, ModKind};
use ch_watch::{classify, locate, ReadCalls, ReadKind, Reader, TooltipRead};
use qt_capture::WindowsBackend;
use qt_screen::{
    fuzzy_score, normalize, BackendError, Image, OverlayState, PixelLine, PixelRect, ScreenBackend,
    WindowHandle,
};

const EXPECTED: &str = include_str!("shots.txt");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Plain,
    Alt,
    Empty,
    Currency,
}

struct Shot {
    file: String,
    kind: Kind,
    /// The client inside the image, pixels.
    client: PixelRect,
    /// Each tooltip's box, fractions of the client.
    boxes: Vec<[f64; 4]>,
}

impl Shot {
    /// The item copy's file: the image's name with `.txt`.
    fn copy_file(&self) -> String {
        format!("{}.txt", self.file.trim_end_matches(".png"))
    }

    /// An equipment shot: a hovered piece of gear, plain or Alt held.
    fn is_equipment(&self) -> bool {
        matches!(self.kind, Kind::Plain | Kind::Alt)
    }

    /// The hand-measured boxes in client pixels.
    fn measured(&self) -> Vec<PixelRect> {
        let (w, h) = (self.client.width() as f64, self.client.height() as f64);
        self.boxes
            .iter()
            .map(|b| {
                PixelRect::new(
                    (b[0] * w).round() as i32,
                    (b[1] * h).round() as i32,
                    (b[2] * w).round() as i32,
                    (b[3] * h).round() as i32,
                )
            })
            .collect()
    }
}

fn shots() -> Vec<Shot> {
    EXPECTED
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            let kind = match words[1] {
                "plain" => Kind::Plain,
                "alt" => Kind::Alt,
                "empty" => Kind::Empty,
                "currency" => Kind::Currency,
                other => panic!("tests/shots.txt: unknown kind {other}"),
            };
            let n: Vec<f64> = words[2..]
                .iter()
                .map(|w| w.parse().expect("tests/shots.txt: a number"))
                .collect();
            assert!(n.len() >= 4 && (n.len() - 4).is_multiple_of(4), "{line}");
            Shot {
                file: words[0].to_string(),
                kind,
                client: PixelRect::new(n[0] as i32, n[1] as i32, n[2] as i32, n[3] as i32),
                boxes: n[4..].chunks(4).map(|c| [c[0], c[1], c[2], c[3]]).collect(),
            }
        })
        .collect()
}

/// The folder `CH_TOOLTIP_SHOTS` names. These tests run only when asked
/// for, so an unset folder is a failure, never a skip.
fn folder() -> PathBuf {
    let dir = std::env::var_os("CH_TOOLTIP_SHOTS")
        .expect("set CH_TOOLTIP_SHOTS to the folder of the owner's tooltip screenshots");
    PathBuf::from(dir)
}

/// The shot's client, decoded.
fn client_image(shot: &Shot) -> Image {
    let path = folder().join(&shot.file);
    let image = qt_capture::decode_jpeg_file(&path)
        .unwrap_or_else(|e| panic!("{}: {}", shot.file, e.message));
    crop(&image, shot.client)
}

fn crop(image: &Image, rect: PixelRect) -> Image {
    assert!(
        rect.right as u32 <= image.width && rect.bottom as u32 <= image.height,
        "the client {rect:?} lies outside the {}x{} image",
        image.width,
        image.height
    );
    let (w, h) = (rect.width() as u32, rect.height() as u32);
    let mut bgra = Vec::with_capacity((w * h * 4) as usize);
    for y in rect.top..rect.bottom {
        let start = (y as usize * image.width as usize + rect.left as usize) * 4;
        bgra.extend_from_slice(&image.bgra[start..start + w as usize * 4]);
    }
    Image {
        width: w,
        height: h,
        bgra,
    }
}

/// The item the shot's copy describes.
fn copy_item(shot: &Shot) -> Item {
    let path = folder().join(shot.copy_file());
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", shot.copy_file()));
    ch_engine::ch_item::parse(&text)
        .unwrap_or_else(|e| panic!("{}: the copy does not parse: {e:?}", shot.copy_file()))
}

/// The read calls over one shot: its client as the copy, no overlay, and
/// Windows' OCR.
struct ShotCalls {
    image: Image,
    ocr: WindowsBackend,
}

impl ReadCalls for ShotCalls {
    fn capture(&mut self, _window: WindowHandle, _area: PixelRect) -> Result<Image, BackendError> {
        Ok(self.image.clone())
    }

    fn overlay(&mut self) -> Result<Option<OverlayState>, BackendError> {
        Ok(None)
    }

    fn ocr(&mut self, image: &Image) -> Result<Vec<PixelLine>, BackendError> {
        ScreenBackend::ocr(&mut self.ocr, image)
    }
}

/// One shot read as the watch reads the game.
fn read(shot: &Shot) -> TooltipRead {
    let image = client_image(shot);
    let client = PixelRect::new(0, 0, image.width as i32, image.height as i32);
    let mut calls = ShotCalls {
        image,
        ocr: WindowsBackend::new(None),
    };
    Reader::new()
        .read(&mut calls, WindowHandle(0), client)
        .unwrap_or_else(|e| panic!("{}: {}", shot.file, e.message))
}

/// A line's words with its numbers, signs and ranges taken out.
fn words(line: &str) -> String {
    let stripped = common::strip_ranges(line);
    let letters: String = stripped
        .chars()
        .map(|c| if c.is_ascii_digit() { ' ' } else { c })
        .collect();
    normalize(&letters)
}

fn alike(a: &str, b: &str) -> bool {
    !a.is_empty() && fuzzy_score(a, b).min(fuzzy_score(b, a)) >= 0.85
}

#[test]
#[ignore = "reads the owner's screenshots: set CH_TOOLTIP_SHOTS"]
fn shots_folder_holds_the_set() {
    let dir = folder();
    let mut missing = Vec::new();
    for shot in shots() {
        if !dir.join(&shot.file).is_file() {
            missing.push(shot.file.clone());
        }
        if shot.kind != Kind::Empty && !dir.join(shot.copy_file()).is_file() {
            missing.push(shot.copy_file());
        }
    }
    assert!(
        missing.is_empty(),
        "missing from {}: {missing:?}",
        dir.display()
    );
    assert_eq!(shots().len(), 15);
}

/// The positive control: OCR of each whole equipment shot, no locator,
/// reads at least half of its copy's explicit modifier lines. If it does
/// not, nothing the locator or the matcher does with OCR can be trusted.
#[test]
#[ignore = "reads the owner's screenshots: set CH_TOOLTIP_SHOTS"]
fn whole_image_ocr_reads_the_copied_mod_lines() {
    let mut backend = WindowsBackend::new(None);
    let mut short = Vec::new();
    for shot in shots().iter().filter(|s| s.is_equipment()) {
        let item = copy_item(shot);
        let want: Vec<String> = item
            .modifiers
            .iter()
            .filter(|m| matches!(m.kind, ModKind::Prefix | ModKind::Suffix) && !m.unrevealed)
            .flat_map(|m| m.lines.iter().map(|l| words(&l.text)))
            .collect();
        let lines = ScreenBackend::ocr(&mut backend, &client_image(shot))
            .unwrap_or_else(|e| panic!("{}: {}", shot.file, e.message));
        let read: Vec<String> = lines.iter().map(|l| words(&l.text)).collect();
        let found = want
            .iter()
            .filter(|w| read.iter().any(|r| alike(w, r)))
            .count();
        println!("{}: {found} of {} modifier lines", shot.file, want.len());
        if found * 2 < want.len() {
            short.push(format!("{} ({found} of {})", shot.file, want.len()));
        }
    }
    assert!(short.is_empty(), "OCR read under half of: {short:?}");
}

#[test]
#[ignore = "reads the owner's screenshots: set CH_TOOLTIP_SHOTS"]
fn shot_tooltip_boxes_match_the_measured_ones() {
    let mut wrong = Vec::new();
    for shot in shots() {
        let image = client_image(&shot);
        let found = locate(&image);
        let measured = shot.measured();
        // An edge within a hundredth of the client; the bottom may run on
        // below a tooltip over a dark scene, never stop short of it.
        let (sx, sy) = (shot.client.width() / 100, shot.client.height() / 100);
        let near = |a: i32, b: i32, slack: i32| (a - b).abs() <= slack;
        let matches = |f: &PixelRect, m: &PixelRect| {
            near(f.left, m.left, sx)
                && near(f.top, m.top, sy)
                && near(f.right, m.right, sx)
                && f.bottom >= m.bottom - sy
        };
        let all_found = measured
            .iter()
            .all(|m| found.iter().any(|f| matches(&f.rect, m)));
        if found.len() != measured.len() || !all_found {
            let got: Vec<PixelRect> = found.iter().map(|f| f.rect).collect();
            wrong.push(format!(
                "{}: found {got:?}, measured {measured:?}",
                shot.file
            ));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The target: every shot's read equals its copy (rarity, base, and the
/// explicit ids and tiers `ch_data::join` gives the copy) or is unreadable,
/// never a different item; and at least three quarters of the plain-form
/// equipment shots equal it.
#[test]
#[ignore = "reads the owner's screenshots: set CH_TOOLTIP_SHOTS"]
fn shots_read_the_copied_mods_or_say_unreadable() {
    let mut wrong = Vec::new();
    let (mut plain, mut plain_equal) = (0, 0);
    for shot in shots().iter().filter(|s| s.is_equipment()) {
        let kind = classify(&read(shot), &copy_item(shot));
        println!("{}: {}", shot.file, describe(&kind));
        match kind {
            ReadKind::Unchanged => {
                if shot.kind == Kind::Plain {
                    plain_equal += 1;
                }
            }
            ReadKind::Unreadable(_) => {}
            other => wrong.push(format!("{}: {}", shot.file, describe(&other))),
        }
        if shot.kind == Kind::Plain {
            plain += 1;
        }
    }
    println!("plain-form equipment shots equal to their copy: {plain_equal} of {plain}");
    assert!(wrong.is_empty(), "read as a different item: {wrong:?}");
    assert!(
        plain_equal * 4 >= plain * 3,
        "only {plain_equal} of {plain} plain-form equipment shots read as their copy"
    );
}

/// The rare belt Live 1 hovered (2026-10-29), which the watch called
/// unreadable (a line not accounted for): its plain tooltip, with the app's
/// overlay drawn at the top centre of the shot, reads as its own copy.
#[test]
#[ignore = "reads the owner's screenshots: set CH_TOOLTIP_SHOTS"]
fn the_belt_shot_reads_unchanged() {
    let all = shots();
    let shot = all
        .iter()
        .find(|s| s.file == "13-belt-plain.png")
        .expect("tests/shots.txt lists 13-belt-plain.png");
    let kind = classify(&read(shot), &copy_item(shot));
    // Compared without `assert_eq!`, whose message would print a changed
    // read's item text.
    assert!(
        kind == ReadKind::Unchanged,
        "{}: {}",
        shot.file,
        describe(&kind)
    );
}

/// The shot with no tooltip finds none; the currency shot is another item
/// to the equipment the guide would be watching.
#[test]
#[ignore = "reads the owner's screenshots: set CH_TOOLTIP_SHOTS"]
fn shots_without_a_tooltip_find_none() {
    let all = shots();
    let amulet = all
        .iter()
        .find(|s| s.kind == Kind::Plain)
        .map(copy_item)
        .expect("a plain shot");
    for shot in &all {
        match shot.kind {
            Kind::Empty => {
                assert!(locate(&client_image(shot)).is_empty(), "{}", shot.file);
                assert_eq!(read(shot), TooltipRead::NoTooltip, "{}", shot.file);
            }
            Kind::Currency => {
                let kind = classify(&read(shot), &amulet);
                assert_eq!(kind, ReadKind::AnotherItem, "{}", shot.file);
            }
            Kind::Plain | Kind::Alt => {}
        }
    }
}

/// A read kind in words, with an unreadable read's reason: never a line's
/// text.
fn describe(kind: &ReadKind) -> String {
    match kind {
        ReadKind::Unreadable(why) => format!("unreadable ({})", why.as_str()),
        other => other.state().to_string(),
    }
}
