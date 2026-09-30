//! A column and the page beside it, on a window that may be too narrow for both.
//!
//! Everything the toolkit draws is sized by the window's height, so a window
//! standing on its side has far less width for a page than it was laid out
//! for — and a column squeezed down to fit beside its page is a column whose
//! words are cut off and whose marks shrink to nothing. LineXinBar's Home menu
//! answers that the way this module does: the column keeps the room its
//! contents need, the page is laid out past the right-hand edge at a width it
//! can use, and the whole view slides over to the page when the page has the
//! focus and back when the column does. The same size as the rest of the
//! desktop, a different arrangement.
//!
//! [`beside`] says where the page stands and how far the view slides; a
//! [`Slide`] moves it there on the spring the shell's own cards ride. A window
//! wide enough for both is laid out exactly as it would be without any of this:
//! the page takes what is left beside the column, and nothing slides.

use crate::motion;

/// How much of the window the other half is always seen in, at least: the page
/// peeking in at the right while the column has the focus, and the strip of the
/// column at the left while the page has it. Each says that a direction leads
/// somewhere. LineXinBar's `overview::PEEK`, which `check-sync` holds this to.
pub const PEEK: f32 = 0.12;

/// Where a view that is sliding is close enough to where it is going to be
/// put there, in pixels — and how slowly it has to be moving as well.
const AT_REST: f32 = 0.5;

/// How a column and the page beside it share a window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Beside {
    /// Where the page starts, before the view has slid anywhere: just beyond
    /// the column, as it always was.
    pub page_x: f32,
    /// How wide the page is laid out. Everything left beside the column where
    /// the two fit side by side; the window less its margins and a peek of the
    /// column where they do not.
    pub page_w: f32,
    /// How far the view slides to the left while the page has the focus, so
    /// that the page stands whole with a strip of the column beside it. Nought
    /// where the two stand side by side, which is every window wide enough.
    pub reach: f32,
}

impl Beside {
    /// Whether the view slides at all on this window.
    pub fn slides(&self) -> bool {
        self.reach > 0.0
    }

    /// How far the view stands slid for whichever half has the focus.
    pub fn target(&self, page_has_the_focus: bool) -> f32 {
        if page_has_the_focus {
            self.reach
        } else {
            0.0
        }
    }
}

/// Where the page beside a column stands in a window `window` wide.
///
/// The column is `column` wide and stands `margin` in from the left-hand edge;
/// the page starts `gap` beyond it and keeps `margin` from the right-hand
/// edge. `least` is the narrowest the page can be and still be the page —
/// every width here in the same pixels.
///
/// Where the page has at least that much beside the column, it has all of it
/// and nothing slides. Where it has not, the column keeps its width and the
/// page is laid out as wide as the window less its margins and a peek of the
/// column — never narrower than it would have been beside it — and
/// [`Beside::reach`] is how far the view slides to show it.
pub fn beside(window: f32, margin: f32, column: f32, gap: f32, least: f32) -> Beside {
    let page_x = margin + column + gap;
    let room = (window - page_x - margin).max(0.0);
    if room >= least {
        return Beside {
            page_x,
            page_w: room,
            reach: 0.0,
        };
    }
    let page_w = (window - margin * 2.0 - window * PEEK).max(room);
    Beside {
        page_x,
        page_w,
        reach: (page_x + page_w + margin - window).max(0.0),
    }
}

/// The view's slide between a column and its page, on the spring LineXinBar's
/// cards ride — critically damped, so it accelerates away from rest and
/// settles without crossing, and a change of mind half way is picked up where
/// the view is rather than started again.
///
/// Laid out for C as it is here: a program on the C ABI or in Python keeps one
/// of these and hands it back every frame.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Slide {
    /// Where the view stands, in pixels slid to the left.
    pub at: f32,
    /// How fast it is going there, in pixels a second.
    pub speed: f32,
    /// Where it was last asked to go.
    pub target: f32,
    /// Whether it has stood anywhere yet. The first frame does not slide: a
    /// window that opens with the page in view opens on the page.
    pub placed: bool,
}

impl Slide {
    /// Move `dt` seconds of the way towards `target`, and say where the view
    /// stands now.
    pub fn follow(&mut self, target: f32, dt: f32) -> f32 {
        self.target = target;
        if !self.placed {
            self.placed = true;
            self.at = target;
            self.speed = 0.0;
            return self.at;
        }
        let (at, speed) = motion::spring(
            self.at as f64,
            self.speed as f64,
            target as f64,
            motion::CARD_SPRING,
            dt as f64,
        );
        self.at = at as f32;
        self.speed = speed as f32;
        if (self.at - target).abs() < AT_REST && self.speed.abs() < AT_REST {
            self.at = target;
            self.speed = 0.0;
        }
        self.at
    }

    /// Whether the view is still on its way, and so wants another frame.
    pub fn moving(&self) -> bool {
        self.placed && (self.at != self.target || self.speed != 0.0)
    }

    /// Start again from nothing: the next [`Self::follow`] stands wherever it
    /// is asked to, without sliding there. For a window whose column and page
    /// have just changed width, where a slide from the old place would be a
    /// slide out of a layout that is no longer on the screen.
    pub fn place_again(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A window wide enough for both lays the page out beside the column,
    /// exactly as it would be laid out without any of this, and never slides.
    #[test]
    fn a_wide_window_keeps_the_page_beside_the_column() {
        let wide = beside(1920.0, 24.0, 300.0, 18.0, 600.0);
        assert_eq!(wide.page_x, 24.0 + 300.0 + 18.0);
        assert_eq!(wide.page_w, 1920.0 - 342.0 - 24.0);
        assert_eq!(wide.reach, 0.0);
        assert!(!wide.slides());
        assert_eq!(wide.target(true), 0.0);
    }

    /// A narrow one keeps the column's width, lays the page out as wide as the
    /// window less its margins and a peek, and slides exactly far enough for
    /// the page to stand whole against the right-hand margin — with the rest
    /// of the peek showing the column at the left.
    #[test]
    fn a_narrow_window_slides_to_a_page_it_can_read() {
        let (window, margin, column, gap) = (620.0, 16.0, 380.0, 12.0);
        let narrow = beside(window, margin, column, gap, 400.0);
        assert!(narrow.slides());
        assert_eq!(narrow.page_x, margin + column + gap);
        assert!((narrow.page_w - (window - margin * 2.0 - window * PEEK)).abs() < 1e-3);
        let slid_to = narrow.page_x - narrow.reach;
        assert!((slid_to + narrow.page_w - (window - margin)).abs() < 1e-3);
        // What is left of the column in view is the peek, less the gap.
        let column_ends = margin + column - narrow.reach;
        assert!(column_ends > 0.0, "none of the column left in view");
        // And the page, before the slide, peeks in at the right.
        assert!(narrow.page_x < window);
        // Wider than it would have been beside the column, by a long way.
        assert!(narrow.page_w > (window - narrow.page_x - margin) * 2.0);
    }

    /// The page is never laid out narrower for sliding than it was beside the
    /// column: a column so thin that the peek would cost more than it gives
    /// keeps the page where it is.
    #[test]
    fn sliding_never_narrows_the_page() {
        for window in [200.0f32, 400.0, 620.0, 800.0, 1080.0] {
            for column in [40.0f32, 120.0, 380.0] {
                let laid = beside(window, 16.0, column, 12.0, 2000.0);
                let beside_it = (window - 16.0 - column - 12.0 - 16.0).max(0.0);
                assert!(laid.page_w >= beside_it, "{window} {column}");
                assert!(laid.reach >= 0.0);
            }
        }
    }

    /// The first frame stands where it is asked to; every one after moves on
    /// the cards' spring and arrives, and then asks for no more frames.
    #[test]
    fn the_slide_snaps_first_and_then_rides_the_spring() {
        let mut slide = Slide::default();
        assert!(!slide.moving());
        assert_eq!(
            slide.follow(300.0, 1.0 / 60.0),
            300.0,
            "no slide on the first frame"
        );
        assert!(!slide.moving());

        let at = slide.follow(0.0, 1.0 / 60.0);
        let (spring, _) = motion::spring(300.0, 0.0, 0.0, motion::CARD_SPRING, 1.0 / 60.0);
        assert!((at - spring as f32).abs() < 1e-3, "the cards' own spring");
        assert!(slide.moving());
        for _ in 0..120 {
            slide.follow(0.0, 1.0 / 60.0);
        }
        assert_eq!(slide.at, 0.0, "arrived");
        assert!(!slide.moving(), "and asks for no more frames");

        slide.place_again();
        assert_eq!(
            slide.follow(120.0, 1.0 / 60.0),
            120.0,
            "placed again, no slide"
        );
    }
}
