//! The provisional locator and the reader, over synthetic images: a noisy
//! scene with a tooltip-like panel drawn on it, and fake read calls that
//! count what the reader asks for. No image file is read or written.

mod common;

use ch_engine::ch_item::Rarity;
use ch_watch::{locate, ReadCounts, Reader, TooltipRead};
use common::{draw_tooltip, fill, header_height, noisy_scene, plain_image, FakeCalls, PANEL};
use qt_screen::{OverlayState, PixelLine, PixelRect, WindowHandle};

const GAME: WindowHandle = WindowHandle(0x1234);

fn near(a: i32, b: i32, slack: i32) -> bool {
    (a - b).abs() <= slack
}

fn assert_box_near(found: PixelRect, drawn: PixelRect) {
    assert!(
        near(found.left, drawn.left, 3)
            && near(found.top, drawn.top, 3)
            && near(found.right, drawn.right, 3)
            && near(found.bottom, drawn.bottom, 3),
        "found {found:?}, drawn {drawn:?}"
    );
}

#[test]
fn locator_finds_a_synthetic_tooltip() {
    let mut image = noisy_scene(1280, 720, 7);
    let rare = PixelRect::new(420, 140, 780, 470);
    draw_tooltip(&mut image, rare, &Rarity::Rare, 8, 0);
    let found = locate(&image);
    assert_eq!(found.len(), 1, "{found:?}");
    let tip = &found[0];
    assert_box_near(tip.rect, rare);
    assert_eq!(tip.rarity, Some(Rarity::Rare));
    assert!(!tip.touches_edge);
    assert!(
        near(
            tip.header_bottom,
            rare.top + 2 + header_height(&Rarity::Rare),
            4
        ),
        "{tip:?}"
    );

    // A magic tooltip and a normal one side by side: two boxes, each with
    // its own colour.
    let mut image = noisy_scene(1280, 720, 11);
    let magic = PixelRect::new(100, 200, 400, 420);
    let normal = PixelRect::new(700, 260, 980, 460);
    draw_tooltip(&mut image, magic, &Rarity::Magic, 6, 0);
    draw_tooltip(&mut image, normal, &Rarity::Normal, 5, 0);
    let mut found = locate(&image);
    found.sort_by_key(|f| f.rect.left);
    assert_eq!(found.len(), 2, "{found:?}");
    assert_box_near(found[0].rect, magic);
    assert_eq!(found[0].rarity, Some(Rarity::Magic));
    assert_box_near(found[1].rect, normal);
    assert_eq!(found[1].rarity, Some(Rarity::Normal));

    // A tooltip running off the copy's edge is found and marked cut.
    let mut image = noisy_scene(1280, 720, 13);
    let cut = PixelRect::new(1000, 300, 1290, 600);
    draw_tooltip(&mut image, cut, &Rarity::Rare, 8, 0);
    let found = locate(&image);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].touches_edge);
}

#[test]
fn locator_finds_nothing_on_a_plain_scene() {
    assert!(locate(&noisy_scene(1280, 720, 3)).is_empty());
    assert!(locate(&plain_image(640, 360, [90, 90, 90])).is_empty());
    assert!(locate(&plain_image(640, 360, [0, 0, 0])).is_empty());
    // A dark area of the scene with no header band is not a tooltip.
    let mut image = noisy_scene(1280, 720, 5);
    fill(&mut image, PixelRect::new(300, 200, 800, 520), PANEL);
    assert!(locate(&image).is_empty());
    // Nor is a lone coloured strip.
    let mut image = noisy_scene(1280, 720, 9);
    fill(&mut image, PixelRect::new(300, 200, 800, 240), [20, 45, 60]);
    assert!(locate(&image).is_empty());
}

/// A client rect offset on the screen, and a scene the size of it with a
/// rare tooltip at `TIP` (client pixels).
const CLIENT: PixelRect = PixelRect::new(100, 50, 1380, 770);
const TIP: PixelRect = PixelRect::new(420, 140, 780, 470);

fn scene(wide: i32) -> qt_screen::Image {
    let mut image = noisy_scene(1280, 720, 21);
    draw_tooltip(&mut image, TIP, &Rarity::Rare, 8, wide);
    image
}

/// What the fake OCR reads of the crop: two name lines in the header band,
/// one modifier line below it (crop pixels).
fn ocr_lines() -> Vec<PixelLine> {
    vec![
        PixelLine::new("Doom Loop", PixelRect::new(120, 10, 240, 22)),
        PixelLine::new("Amethyst Ring", PixelRect::new(110, 28, 250, 40)),
        PixelLine::new("+95 to maximum Mana", PixelRect::new(80, 80, 280, 92)),
    ]
}

#[test]
fn reader_reports_a_box_under_the_overlay_as_covered() {
    let mut calls = FakeCalls::new(scene(0));
    calls.ocr_lines = ocr_lines();
    // The overlay, in screen pixels, over the tooltip's top-right corner.
    let over_tip = PixelRect::new(
        CLIENT.left + TIP.right - 40,
        CLIENT.top + TIP.top - 30,
        CLIENT.left + TIP.right + 200,
        CLIENT.top + TIP.top + 30,
    );
    calls.overlay = Some(OverlayState {
        rect: over_tip,
        visible: true,
    });
    let mut reader = Reader::new();
    let read = reader.read(&mut calls, GAME, CLIENT).unwrap();
    assert_eq!(read, TooltipRead::Covered);
    assert_eq!(calls.ocr_calls, 0, "a covered box is never read in part");
    assert_eq!(calls.areas, [CLIENT]);

    // Hidden, the same overlay covers nothing.
    calls.overlay = Some(OverlayState {
        rect: over_tip,
        visible: false,
    });
    let read = reader.read(&mut calls, GAME, CLIENT).unwrap();
    let TooltipRead::Boxes(boxes) = read else {
        panic!("{read:?}");
    };
    assert_eq!(boxes.len(), 1);
    let tip = &boxes[0];
    assert!(near(tip.rect.left, TIP.left, 3) && near(tip.rect.top, TIP.top, 3));
    assert_eq!(tip.rarity, Some(Rarity::Rare));
    let names: Vec<&str> = tip.name_lines.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(names, ["Doom Loop", "Amethyst Ring"]);
    assert_eq!(tip.lines.len(), 1);
    // Boxes are in client pixels.
    assert_eq!(tip.lines[0].bbox.left, tip.rect.left + 80);
    assert_eq!(tip.lines[0].bbox.top, tip.rect.top + 80);

    // Shown elsewhere, it covers nothing either.
    calls.overlay = Some(OverlayState {
        rect: PixelRect::new(CLIENT.left, CLIENT.top, CLIENT.left + 200, CLIENT.top + 60),
        visible: true,
    });
    assert!(matches!(
        reader.read(&mut calls, GAME, CLIENT).unwrap(),
        TooltipRead::Boxes(_)
    ));
}

#[test]
fn an_unchanged_crop_is_ocr_read_once() {
    let mut calls = FakeCalls::new(scene(0));
    calls.ocr_lines = ocr_lines();
    let mut reader = Reader::new();
    let first = reader.read(&mut calls, GAME, CLIENT).unwrap();
    let second = reader.read(&mut calls, GAME, CLIENT).unwrap();
    assert_eq!(calls.ocr_calls, 1);
    assert_eq!(first, second);
    assert!(matches!(first, TooltipRead::Boxes(_)));
    assert_eq!(
        reader.take_counts(),
        ReadCounts {
            copies: 2,
            tooltips: 2,
            ocr_reads: 1,
        }
    );
    assert_eq!(reader.take_counts(), ReadCounts::default());

    // The tooltip's text changes: read again.
    calls.image = scene(12);
    reader.read(&mut calls, GAME, CLIENT).unwrap();
    assert_eq!(calls.ocr_calls, 2);

    // No tooltip, then a black copy: nothing to OCR.
    calls.image = noisy_scene(1280, 720, 21);
    assert_eq!(
        reader.read(&mut calls, GAME, CLIENT).unwrap(),
        TooltipRead::NoTooltip
    );
    calls.image = plain_image(1280, 720, [0, 0, 0]);
    assert_eq!(
        reader.read(&mut calls, GAME, CLIENT).unwrap(),
        TooltipRead::Black
    );
    assert_eq!(calls.ocr_calls, 2);
    // After a read with no tooltip, the same crop is read afresh.
    calls.image = scene(12);
    reader.read(&mut calls, GAME, CLIENT).unwrap();
    assert_eq!(calls.ocr_calls, 3);
}
