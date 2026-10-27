//! A kept capture's JPEG frames back into [`Image`]s, through WinRT's own
//! bitmap decoder (`Windows.Graphics.Imaging`), for
//! `quest-tracker-replay panels`. No image crate: the decoder Windows ships
//! reads the research capture's JPEGs, and the `windows` features this crate
//! already enables cover it.
//!
//! The pixels come back as blue, green, red and an opaque alpha, unrotated
//! (the capture writes no orientation) and without colour management, so a
//! frame's values are the ones the capture encoded. Errors name the call
//! and the system's error, never a pixel.

use std::cell::Cell;
use std::path::Path;

use qt_screen::{BackendError, Image};
use windows::Graphics::Imaging::{
    BitmapAlphaMode, BitmapDecoder, BitmapPixelFormat, BitmapTransform, ColorManagementMode,
    ExifOrientationMode,
};
use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};
use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

fn failed(call: &str, e: windows::core::Error) -> BackendError {
    BackendError::new(format!("{call} failed: {e}"))
}

thread_local! {
    static WINRT_READY: Cell<bool> = const { Cell::new(false) };
}

/// WinRT on the calling thread, once: multithreaded, unless the thread
/// already chose a single-threaded apartment, which serves the decoder too.
fn ensure_winrt() -> Result<(), BackendError> {
    if WINRT_READY.with(Cell::get) {
        return Ok(());
    }
    match unsafe { RoInitialize(RO_INIT_MULTITHREADED) } {
        Ok(()) => {}
        Err(e) if e.code() == RPC_E_CHANGED_MODE => {}
        Err(e) => return Err(failed("RoInitialize (multithreaded)", e)),
    }
    WINRT_READY.with(|r| r.set(true));
    Ok(())
}

/// The JPEG file at `path` as an image. A file that cannot be read names
/// the path and the system's error.
pub fn decode_jpeg_file(path: &Path) -> Result<Image, BackendError> {
    let bytes = std::fs::read(path)
        .map_err(|e| BackendError::new(format!("reading {}: {e}", path.display())))?;
    decode_jpeg(&bytes)
}

/// The encoded image `bytes` (a JPEG, or any format Windows' decoder reads)
/// as an image: blue, green, red and alpha 255 per pixel, rows top down.
pub fn decode_jpeg(bytes: &[u8]) -> Result<Image, BackendError> {
    ensure_winrt()?;
    let writer = DataWriter::new().map_err(|e| failed("DataWriter::new", e))?;
    writer
        .WriteBytes(bytes)
        .map_err(|e| failed("DataWriter::WriteBytes", e))?;
    let buffer = writer
        .DetachBuffer()
        .map_err(|e| failed("DataWriter::DetachBuffer", e))?;
    let stream = InMemoryRandomAccessStream::new()
        .map_err(|e| failed("InMemoryRandomAccessStream::new", e))?;
    stream
        .WriteAsync(&buffer)
        .and_then(|op| op.join())
        .map_err(|e| failed("InMemoryRandomAccessStream::WriteAsync", e))?;
    stream
        .Seek(0)
        .map_err(|e| failed("InMemoryRandomAccessStream::Seek", e))?;
    let decoder = BitmapDecoder::CreateAsync(&stream)
        .and_then(|op| op.join())
        .map_err(|e| failed("BitmapDecoder::CreateAsync", e))?;
    let width = decoder
        .PixelWidth()
        .map_err(|e| failed("BitmapDecoder::PixelWidth", e))?;
    let height = decoder
        .PixelHeight()
        .map_err(|e| failed("BitmapDecoder::PixelHeight", e))?;
    let transform = BitmapTransform::new().map_err(|e| failed("BitmapTransform::new", e))?;
    let provider = decoder
        .GetPixelDataTransformedAsync(
            BitmapPixelFormat::Bgra8,
            BitmapAlphaMode::Ignore,
            &transform,
            ExifOrientationMode::IgnoreExifOrientation,
            ColorManagementMode::DoNotColorManage,
        )
        .and_then(|op| op.join())
        .map_err(|e| failed("BitmapDecoder::GetPixelDataTransformedAsync", e))?;
    let pixels = provider
        .DetachPixelData()
        .map_err(|e| failed("PixelDataProvider::DetachPixelData", e))?;
    let mut bgra = pixels.to_vec();
    if bgra.len() != width as usize * height as usize * 4 {
        return Err(BackendError::new(format!(
            "the decoded pixels do not fill {width}x{height}"
        )));
    }
    for px in bgra.chunks_exact_mut(4) {
        px[3] = 255;
    }
    Ok(Image {
        width,
        height,
        bgra,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use qt_screen::{ProbeStats, Rect};
    use windows::Graphics::Imaging::BitmapEncoder;
    use windows::Storage::Streams::{Buffer, DataReader, InputStreamOptions};

    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 180;

    /// Four flat quadrants of distinct colours (blue, green, red), with a
    /// thin grey border between them that the patches below stay clear of.
    const QUADRANTS: [[u8; 3]; 4] = [[40, 90, 160], [200, 180, 30], [15, 15, 15], [120, 60, 210]];

    fn synthetic() -> Image {
        let mut bgra = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let q = usize::from(x >= WIDTH / 2) + 2 * usize::from(y >= HEIGHT / 2);
                let border = x.abs_diff(WIDTH / 2) < 2 || y.abs_diff(HEIGHT / 2) < 2;
                let c = if border {
                    [128, 128, 128]
                } else {
                    QUADRANTS[q]
                };
                bgra.extend_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
        Image {
            width: WIDTH,
            height: HEIGHT,
            bgra,
        }
    }

    /// `image` encoded as a JPEG in memory by Windows' own encoder.
    fn encode_jpeg(image: &Image) -> Vec<u8> {
        let stream = InMemoryRandomAccessStream::new().expect("stream");
        let encoder = BitmapEncoder::CreateAsync(BitmapEncoder::JpegEncoderId().unwrap(), &stream)
            .and_then(|op| op.join())
            .expect("JPEG encoder");
        encoder
            .SetPixelData(
                BitmapPixelFormat::Bgra8,
                BitmapAlphaMode::Ignore,
                image.width,
                image.height,
                96.0,
                96.0,
                &image.bgra,
            )
            .expect("SetPixelData");
        encoder
            .FlushAsync()
            .and_then(|op| op.join())
            .expect("flush");
        let size = stream.Size().expect("size") as u32;
        stream.Seek(0).expect("seek");
        let buffer = Buffer::Create(size).expect("buffer");
        let read = stream
            .ReadAsync(&buffer, size, InputStreamOptions::None)
            .and_then(|op| op.join())
            .expect("read back");
        let reader = DataReader::FromBuffer(&read).expect("reader");
        let mut bytes = vec![0u8; read.Length().expect("length") as usize];
        reader.ReadBytes(&mut bytes).expect("bytes");
        bytes
    }

    #[test]
    fn decode_round_trips_a_synthetic_jpeg() {
        ensure_winrt().expect("WinRT");
        let image = synthetic();
        let jpeg = encode_jpeg(&image);
        assert_eq!(&jpeg[..2], &[0xFF, 0xD8], "a JPEG's start-of-image marker");

        let decoded = decode_jpeg(&jpeg).expect("decode");
        assert_eq!((decoded.width, decoded.height), (WIDTH, HEIGHT));
        assert_eq!(decoded.bgra.len(), (WIDTH * HEIGHT * 4) as usize);
        assert!(decoded.bgra.chunks_exact(4).all(|px| px[3] == 255));

        // A patch well inside each quadrant reads its colour back, within
        // the JPEG's loss.
        let patches = [
            Rect::new(0.10, 0.10, 0.40, 0.40),
            Rect::new(0.60, 0.10, 0.90, 0.40),
            Rect::new(0.10, 0.60, 0.40, 0.90),
            Rect::new(0.60, 0.60, 0.90, 0.90),
        ];
        for (rect, want) in patches.iter().zip(QUADRANTS) {
            let got = ProbeStats::of_frame(&decoded, *rect);
            for c in 0..3 {
                let d = (got.mean_bgr[c] - want[c] as f64).abs();
                assert!(d <= 6.0, "{rect:?} channel {c}: off by {d:.1}");
            }
            assert!(got.luma_std <= 4.0, "{rect:?}: spread {:.1}", got.luma_std);
        }

        // Negative control: the patches do not all read the same colour, so
        // a decode that returned one flat colour would fail above.
        let first = ProbeStats::of_frame(&decoded, patches[0]);
        let second = ProbeStats::of_frame(&decoded, patches[1]);
        assert!((first.mean_bgr[0] - second.mean_bgr[0]).abs() > 100.0);

        // Bytes that are no image are an error naming the call.
        let err = decode_jpeg(b"not an image").unwrap_err();
        assert!(err.message.contains("BitmapDecoder"), "{err}");
    }

    #[test]
    fn decode_names_a_missing_file() {
        let path = Path::new("no-such-dir-qt-capture").join("frame.jpg");
        let err = decode_jpeg_file(&path).unwrap_err();
        assert!(err.message.contains("frame.jpg"), "{err}");
    }
}
