//! The overlay over the game: where it sits ([`overlay_origin`], pure, from
//! the game's client area, the setting and the overlay's size) and what it
//! shows ([`overlay_guide`], from the guiding view).
//!
//! It shows while a craft is guiding, in any guide status, and the screen
//! backend reports the game's window in front; `App` decides that, places
//! it through the shell when the client area, the setting or the overlay's
//! size changed (never on every tick) and logs `overlay: shown`, `overlay:
//! hidden (not guiding|game not in front)` and `overlay: placed
//! <position>`. The shell only moves, shows and hides the window.

use crate::screen::ClientRect;
use crate::view::{GuideStatusView, GuidingView, OverlayGuide, OverlayPosition};

/// The overlay's distance from the game's client edges, in physical pixels
/// (`docs/crafting-helper/ui-design.md` § "Overlay").
pub const OVERLAY_INSET: u32 = 12;

/// The overlay's top-left corner, in physical screen pixels, for an overlay
/// of `size` (width, height, physical pixels) at `position` inside
/// `client`: [`OVERLAY_INSET`] from the edges it sits on, centred across
/// for the `*-center` positions. A client smaller than the overlay and its
/// insets puts it at the client's top-left corner.
pub fn overlay_origin(
    position: OverlayPosition,
    client: ClientRect,
    size: (u32, u32),
) -> (i32, i32) {
    let (w, h) = (i64::from(size.0), i64::from(size.1));
    let inset = i64::from(OVERLAY_INSET);
    let (cw, ch) = (i64::from(client.width()), i64::from(client.height()));
    let (left, top) = (i64::from(client.left), i64::from(client.top));
    if cw < w + 2 * inset || ch < h + 2 * inset {
        return (client.left, client.top);
    }
    let x = match position {
        OverlayPosition::TopLeft | OverlayPosition::BottomLeft => left + inset,
        OverlayPosition::TopCenter | OverlayPosition::BottomCenter => left + (cw - w) / 2,
        OverlayPosition::TopRight | OverlayPosition::BottomRight => left + cw - w - inset,
    };
    let y = match position {
        OverlayPosition::TopLeft | OverlayPosition::TopCenter | OverlayPosition::TopRight => {
            top + inset
        }
        _ => top + ch - h - inset,
    };
    (clamp_i32(x), clamp_i32(y))
}

fn clamp_i32(v: i64) -> i32 {
    v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

/// What replan asks of the player, in the overlay's instruction row.
pub const REPLAN_LINE: &str = "End the craft in the app to plan again from this item.";
/// What the overlay's instruction row says once the craft is done.
pub const DONE_LINE: &str = "The craft is done.";

/// The overlay's content for a guide: the current step (its place, try,
/// instruction, uses, odds and both branch lines) while active; the step,
/// its branches and the first reason while stuck; what to do instead and
/// the first reason with replan; that it is done. The last observed change
/// in every status, and the screen watch's line while active or stuck (the
/// guiding view's own, so the two never disagree).
pub fn overlay_guide(g: &GuidingView) -> OverlayGuide {
    let active = g.status == GuideStatusView::Active;
    let step = match g.status {
        GuideStatusView::Active | GuideStatusView::Stuck => g.step.as_ref(),
        GuideStatusView::Replan | GuideStatusView::Done => None,
    };
    let instruction = match g.status {
        GuideStatusView::Replan => REPLAN_LINE.to_string(),
        GuideStatusView::Done => DONE_LINE.to_string(),
        _ => step.map(|s| s.step.instruction.clone()).unwrap_or_default(),
    };
    let why = match g.status {
        GuideStatusView::Stuck | GuideStatusView::Replan => g.reasons.first().cloned(),
        _ => None,
    };
    OverlayGuide {
        status: g.status,
        route_name: g.route_name.clone(),
        step_number: step.map(|s| s.step.number),
        try_number: step.map(|s| s.try_number),
        instruction,
        uses: step
            .filter(|_| active)
            .map(|s| s.step.uses.clone())
            .unwrap_or_default(),
        odds: step.filter(|_| active).and_then(|s| s.step.odds.clone()),
        why,
        on_hit: step.map(|s| s.on_hit.clone()),
        on_miss: step.map(|s| s.on_miss.clone()),
        last: g.history.last().cloned(),
        watch: g.watch.clone(),
    }
}
