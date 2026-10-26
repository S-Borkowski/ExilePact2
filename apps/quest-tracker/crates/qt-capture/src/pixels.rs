//! The backend's pure arithmetic: where a region lands in screen pixels, the
//! clamp into the client rect, the 16:9 check on a client rect, the OCR's
//! scale and resample, and the mapping of OCR boxes back to the copy's own
//! pixels. No Windows call, so it tests anywhere.

use qt_screen::{aspect_is_16_9, Image, PixelRect, Region};

/// The upscale the OCR applies when the engine's maximum dimension allows
/// it: the research's `ocr.ps1` read every kept frame at scale 2.
pub const OCR_SCALE: f64 = 2.0;

/// The smallest scale [`ocr_scale`] returns, so a huge image still maps.
const MIN_SCALE: f64 = 0.01;

/// Whether the client rect is 16:9, the only aspect the regions were
/// measured at.
pub fn client_is_16_9(client: PixelRect) -> bool {
    aspect_is_16_9(client.width(), client.height())
}

/// `area` cut to the client rect `client`. An area wholly outside comes back
/// empty, on the client rect's nearest edge.
pub fn clamp_to(area: PixelRect, client: PixelRect) -> PixelRect {
    let left = area.left.clamp(client.left, client.right);
    let top = area.top.clamp(client.top, client.bottom);
    PixelRect::new(
        left,
        top,
        area.right.clamp(left, client.right),
        area.bottom.clamp(top, client.bottom),
    )
}

/// Where `region` sits in screen pixels for the client rect `client`, inside
/// it.
pub fn region_area(region: Region, client: PixelRect) -> PixelRect {
    clamp_to(region.pixels(client), client)
}

/// The scale the OCR reads a `width` x `height` image at: [`OCR_SCALE`],
/// lowered (to a hundredth, as `ocr.ps1` did) when that would push the
/// longer side past `max_dimension`.
pub fn ocr_scale(width: u32, height: u32, max_dimension: u32) -> f64 {
    let longest = width.max(height) as f64;
    if longest == 0.0 || max_dimension == 0 || longest * OCR_SCALE <= max_dimension as f64 {
        return OCR_SCALE;
    }
    ((100.0 * max_dimension as f64 / longest).floor() / 100.0).clamp(MIN_SCALE, OCR_SCALE)
}

/// The size of a `width` x `height` image at `scale`, at least 1x1.
pub fn scaled_size(width: u32, height: u32, scale: f64) -> (u32, u32) {
    let side = |n: u32| ((n as f64 * scale).round() as u32).max(1);
    (side(width), side(height))
}

/// The smallest box holding every word's rect (`x`, `y`, `width`,
/// `height`, in the scaled image's pixels), or a zero box when there is no
/// word, as `ocr.ps1` built a line's box.
pub fn union_of_words(words: &[(f64, f64, f64, f64)]) -> (f64, f64, f64, f64) {
    if words.is_empty() {
        return (0.0, 0.0, 0.0, 0.0);
    }
    words.iter().fold(
        (f64::MAX, f64::MAX, 0.0_f64, 0.0_f64),
        |(x0, y0, x1, y1), &(x, y, w, h)| (x0.min(x), y0.min(y), x1.max(x + w), y1.max(y + h)),
    )
}

/// A box read at `scale` back in the unscaled image's own pixels, rounded
/// outwards and kept inside its `width` x `height`.
pub fn unscale_box(scaled: (f64, f64, f64, f64), scale: f64, width: u32, height: u32) -> PixelRect {
    let (x0, y0, x1, y1) = scaled;
    let (w, h) = (width as i32, height as i32);
    let left = ((x0 / scale).floor() as i32).clamp(0, w);
    let top = ((y0 / scale).floor() as i32).clamp(0, h);
    PixelRect::new(
        left,
        top,
        ((x1 / scale).ceil() as i32).clamp(left, w),
        ((y1 / scale).ceil() as i32).clamp(top, h),
    )
}

/// The Catmull-Rom cubic (a = -0.5).
fn cubic(t: f64) -> f64 {
    const A: f64 = -0.5;
    let t = t.abs();
    if t <= 1.0 {
        ((A + 2.0) * t - (A + 3.0)) * t * t + 1.0
    } else if t < 2.0 {
        ((A * t - 5.0 * A) * t + 8.0 * A) * t - 4.0 * A
    } else {
        0.0
    }
}

/// For each of `dst` output positions, its four source positions (clamped to
/// the edge) and their normalized weights.
fn taps(src: u32, dst: u32) -> Vec<([usize; 4], [f64; 4])> {
    let ratio = src as f64 / dst as f64;
    let last = src.saturating_sub(1) as i64;
    (0..dst)
        .map(|d| {
            let center = (d as f64 + 0.5) * ratio - 0.5;
            let first = center.floor() as i64 - 1;
            let mut index = [0usize; 4];
            let mut weight = [0.0f64; 4];
            for k in 0..4 {
                let i = first + k as i64;
                index[k] = i.clamp(0, last) as usize;
                weight[k] = cubic(center - i as f64);
            }
            let sum: f64 = weight.iter().sum();
            if sum != 0.0 {
                weight.iter_mut().for_each(|w| *w /= sum);
            }
            (index, weight)
        })
        .collect()
}

/// `image` resampled (bicubic) to `width` x `height`, opaque. An image whose
/// pixels do not fill its size comes back black.
pub fn resize(image: &Image, width: u32, height: u32) -> Image {
    let (sw, sh) = (image.width as usize, image.height as usize);
    let (dw, dh) = (width as usize, height as usize);
    let mut out = vec![0u8; dw * dh * 4];
    out.chunks_exact_mut(4).for_each(|px| px[3] = 255);
    if sw == 0 || sh == 0 || image.bgra.len() < sw * sh * 4 {
        return Image {
            width,
            height,
            bgra: out,
        };
    }
    // Rows first: every source row to the new width, three channels.
    let across = taps(image.width, width);
    let mut rows = vec![0.0f32; sh * dw * 3];
    for y in 0..sh {
        let src = &image.bgra[y * sw * 4..(y + 1) * sw * 4];
        for (x, (index, weight)) in across.iter().enumerate() {
            for c in 0..3 {
                let v: f64 = (0..4)
                    .map(|k| src[index[k] * 4 + c] as f64 * weight[k])
                    .sum();
                rows[(y * dw + x) * 3 + c] = v as f32;
            }
        }
    }
    // Then columns, to the new height.
    let down = taps(image.height, height);
    for (y, (index, weight)) in down.iter().enumerate() {
        for x in 0..dw {
            for c in 0..3 {
                let v: f64 = (0..4)
                    .map(|k| rows[(index[k] * dw + x) * 3 + c] as f64 * weight[k])
                    .sum();
                out[(y * dw + x) * 4 + c] = v.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    Image {
        width,
        height,
        bgra: out,
    }
}

/// `image`'s pixels, opaque, for a bitmap that reads alpha.
pub fn opaque(image: &Image) -> Vec<u8> {
    let mut bgra = image.bgra.clone();
    bgra.chunks_exact_mut(4).for_each(|px| px[3] = 255);
    bgra
}

#[cfg(test)]
mod tests {
    use super::*;
    use qt_screen::BOSS_BAR_RECT;

    #[test]
    fn regions_land_in_pixels_of_an_offset_client_rect() {
        // A 2560x1440 client area whose top-left sits at 100,50 on screen.
        let client = PixelRect::new(100, 50, 2660, 1490);
        let bar = region_area(Region::BossBar, client);
        assert_eq!(bar, BOSS_BAR_RECT.to_pixels(client));
        assert_eq!(bar.left, 100 + (BOSS_BAR_RECT.left * 2560.0).round() as i32);
        assert_eq!(bar.top, 50 + (BOSS_BAR_RECT.top * 1440.0).round() as i32);
        assert_eq!(
            bar.right,
            100 + (BOSS_BAR_RECT.right * 2560.0).round() as i32
        );
        assert_eq!(
            bar.bottom,
            50 + (BOSS_BAR_RECT.bottom * 1440.0).round() as i32
        );
        for region in Region::ALL {
            let area = region_area(region, client);
            assert!(!area.is_empty(), "{region:?}");
            assert_eq!(clamp_to(area, client), area, "{region:?}");
        }
    }

    #[test]
    fn areas_are_clamped_into_the_client_rect() {
        let client = PixelRect::new(0, 0, 1920, 1080);
        assert_eq!(
            clamp_to(PixelRect::new(-50, -10, 300, 200), client),
            PixelRect::new(0, 0, 300, 200)
        );
        assert_eq!(
            clamp_to(PixelRect::new(1800, 1000, 2100, 1300), client),
            PixelRect::new(1800, 1000, 1920, 1080)
        );
        let outside = clamp_to(PixelRect::new(2000, 10, 2100, 50), client);
        assert!(outside.is_empty(), "{outside:?}");
    }

    #[test]
    fn the_aspect_check_takes_the_client_rect() {
        assert!(client_is_16_9(PixelRect::new(0, 0, 2560, 1440)));
        assert!(client_is_16_9(PixelRect::new(-1920, 0, 0, 1080)));
        assert!(!client_is_16_9(PixelRect::new(0, 0, 1920, 1200)));
        assert!(!client_is_16_9(PixelRect::new(0, 0, 3440, 1440)));
        assert!(!client_is_16_9(PixelRect::new(10, 10, 10, 10)));
    }

    #[test]
    fn the_ocr_scale_stays_within_the_maximum_dimension() {
        assert_eq!(ocr_scale(1280, 202, 10_000), OCR_SCALE);
        assert_eq!(ocr_scale(5000, 100, 10_000), OCR_SCALE);
        let s = ocr_scale(6000, 100, 10_000);
        assert_eq!(s, 1.66);
        let (w, h) = scaled_size(6000, 100, s);
        assert!(w <= 10_000 && h == 166, "{w}x{h}");
        // Larger than the maximum even unscaled: scaled down to fit.
        let s = ocr_scale(12_000, 10, 10_000);
        assert!(s < 1.0 && scaled_size(12_000, 10, s).0 <= 10_000, "{s}");
        assert_eq!(scaled_size(0, 0, OCR_SCALE), (1, 1));
    }

    #[test]
    fn ocr_boxes_map_back_outwards_and_stay_inside() {
        let words = [(10.0, 20.0, 30.0, 8.0), (44.5, 18.0, 20.0, 12.0)];
        let line = union_of_words(&words);
        assert_eq!(line, (10.0, 18.0, 64.5, 30.0));
        assert_eq!(
            unscale_box(line, 2.0, 100, 100),
            PixelRect::new(5, 9, 33, 15)
        );
        assert_eq!(
            unscale_box((150.0, 4.0, 400.0, 30.0), 2.0, 100, 10),
            PixelRect::new(75, 2, 100, 10)
        );
        assert_eq!(union_of_words(&[]), (0.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn resize_keeps_flat_color_and_makes_it_opaque() {
        let image = Image {
            width: 3,
            height: 2,
            bgra: [10u8, 20, 30, 0].repeat(6),
        };
        let big = resize(&image, 6, 4);
        assert_eq!((big.width, big.height, big.bgra.len()), (6, 4, 96));
        assert!(big.bgra.chunks_exact(4).all(|px| px == [10, 20, 30, 255]));
        assert!(opaque(&image).chunks_exact(4).all(|px| px[3] == 255));
    }

    #[test]
    fn resize_keeps_an_edge_between_its_sides() {
        // Left half black, right half white: the upscale keeps each side's
        // value away from the edge, and the edge between them.
        let mut bgra = Vec::new();
        for _ in 0..4 {
            for x in 0..8 {
                let v = if x < 4 { 0 } else { 255 };
                bgra.extend_from_slice(&[v, v, v, 255]);
            }
        }
        let image = Image {
            width: 8,
            height: 4,
            bgra,
        };
        let big = resize(&image, 16, 8);
        let at = |x: usize, y: usize| big.bgra[(y * 16 + x) * 4];
        assert_eq!(at(0, 3), 0);
        assert_eq!(at(15, 3), 255);
        assert!(
            at(5, 3) < 20 && at(10, 3) > 235,
            "{} {}",
            at(5, 3),
            at(10, 3)
        );
    }
}
