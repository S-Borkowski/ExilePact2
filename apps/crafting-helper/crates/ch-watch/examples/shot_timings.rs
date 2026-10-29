//! A developer's measure of one screenshot: how long the locator takes on
//! the whole client, how long Windows' OCR takes on each tooltip's crop, and
//! what the read says of the item in the paired copy (the `.txt` beside the
//! image, when there is one). It prints times, boxes and read kinds, never a
//! line's text.
//!
//! ```text
//! cargo run --release -p ch-watch --example shot_timings -- <image.png> [left top right bottom]
//! ```
//!
//! The optional rect is the game's client inside the image (a windowed
//! capture with its title bar); by default the whole image. The image is
//! decoded from a file, so the copy of the screen the app makes (a GDI copy
//! of the client) is not part of the times: Live 1's minute lines measure
//! that.

#[cfg(windows)]
fn main() {
    use std::path::Path;
    use std::time::Instant;

    use ch_watch::{classify, locate, ReadCalls, ReadKind, Reader, TooltipRead};
    use qt_capture::WindowsBackend;
    use qt_screen::{
        BackendError, Image, OverlayState, PixelLine, PixelRect, ScreenBackend, WindowHandle,
    };

    struct Calls {
        image: Image,
        ocr: WindowsBackend,
        ocr_ms: Vec<u128>,
    }

    impl ReadCalls for Calls {
        fn capture(&mut self, _: WindowHandle, _: PixelRect) -> Result<Image, BackendError> {
            Ok(self.image.clone())
        }
        fn overlay(&mut self) -> Result<Option<OverlayState>, BackendError> {
            Ok(None)
        }
        fn ocr(&mut self, image: &Image) -> Result<Vec<PixelLine>, BackendError> {
            let start = Instant::now();
            let lines = ScreenBackend::ocr(&mut self.ocr, image);
            self.ocr_ms.push(start.elapsed().as_millis());
            lines
        }
    }

    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(path) = args.first() else {
        eprintln!("usage: shot_timings <image.png> [left top right bottom]");
        std::process::exit(2);
    };
    let image = match qt_capture::decode_jpeg_file(Path::new(path)) {
        Ok(image) => image,
        Err(e) => {
            eprintln!("{path}: {}", e.message);
            std::process::exit(1);
        }
    };
    let client = match args[1..]
        .iter()
        .map(|a| a.parse::<i32>())
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(n) if n.len() == 4 => PixelRect::new(n[0], n[1], n[2], n[3]),
        Ok(n) if n.is_empty() => PixelRect::new(0, 0, image.width as i32, image.height as i32),
        _ => {
            eprintln!("the client rect is four whole numbers: left top right bottom");
            std::process::exit(2);
        }
    };
    let (w, h) = (client.width() as u32, client.height() as u32);
    let mut bgra = Vec::with_capacity((w * h * 4) as usize);
    for y in client.top..client.bottom {
        let start = (y as usize * image.width as usize + client.left as usize) * 4;
        bgra.extend_from_slice(&image.bgra[start..start + w as usize * 4]);
    }
    let image = Image {
        width: w,
        height: h,
        bgra,
    };
    println!("client {w}x{h}");

    let mut times: Vec<u128> = (0..5)
        .map(|_| {
            let start = Instant::now();
            let _ = locate(&image);
            start.elapsed().as_millis()
        })
        .collect();
    times.sort_unstable();
    let found = locate(&image);
    println!(
        "locate: {} ms (median of 5), {} tooltip(s)",
        times[2],
        found.len()
    );

    let mut calls = Calls {
        image,
        ocr: WindowsBackend::new(None),
        ocr_ms: Vec::new(),
    };
    let start = Instant::now();
    let read = match Reader::new().read(
        &mut calls,
        WindowHandle(0),
        PixelRect::new(0, 0, w as i32, h as i32),
    ) {
        Ok(read) => read,
        Err(e) => {
            eprintln!("read failed: {}", e.message);
            std::process::exit(1);
        }
    };
    println!("read: {} ms in all", start.elapsed().as_millis());
    if let TooltipRead::Boxes(boxes) = &read {
        for (b, ms) in boxes.iter().zip(&calls.ocr_ms) {
            println!(
                "  box {}x{} at {},{}: OCR {ms} ms, {} name line(s), {} line(s), rarity {:?}",
                b.rect.width(),
                b.rect.height(),
                b.rect.left,
                b.rect.top,
                b.name_lines.len(),
                b.lines.len(),
                b.rarity
            );
        }
    }

    let copy = Path::new(path).with_extension("txt");
    match std::fs::read_to_string(&copy)
        .ok()
        .and_then(|text| ch_engine::ch_item::parse(&text).ok())
    {
        Some(item) => {
            let kind = classify(&read, &item);
            let why = match &kind {
                ReadKind::Unreadable(why) => format!(" ({})", why.as_str()),
                _ => String::new(),
            };
            println!("read kind against the copy: {}{why}", kind.state());
        }
        None => println!(
            "read: {}",
            match read {
                TooltipRead::NoTooltip => "no tooltip",
                TooltipRead::Covered => "covered",
                TooltipRead::Black => "black",
                TooltipRead::Boxes(_) => "tooltips (no copy beside the image)",
            }
        ),
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("shot_timings reads with Windows' own OCR: run it on Windows");
}
