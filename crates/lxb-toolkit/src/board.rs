//! Where LineXinBar's on-screen keyboard stands, for an application whose text
//! field it types into.
//!
//! The shell draws its board on a layer of its own, centred along the foot of
//! the display, wherever the field that summoned it is: it never moves the
//! board out of a field's way, and Wayland's text input carries no answer back
//! to say where the board went. A field low on a full-screen window is typed
//! into blind under the keys unless the application itself stands it clear of
//! them — which is what [`rect`] is for: the board's settled rectangle, by the
//! shell's own rule, for a window that fills the display.
//!
//! The shell's `keyboard_panel_rect` and the constants and rows it is measured
//! from (`ui.rs`, `keyboard.rs`), which `check-sync` holds this to.

use crate::metrics::scale_for;

/// How wide one key's column is, at the reference height.
pub const KEY_UNIT: f32 = 62.0;
/// How tall a full key is, at the reference height.
pub const KEY_HEIGHT: f32 = 56.0;
/// The air between neighbouring rows of keys, at the reference height.
pub const KEY_GAP: f32 = 8.0;
/// The glass round the keys, at the reference height.
pub const BOARD_PADDING: f32 = 20.0;
/// How far the board floats clear of the display's bottom edge, at the
/// reference height.
pub const BOARD_MARGIN: f32 = 34.0;
/// How many key columns wide the board is.
pub const COLUMNS: f32 = 15.0;
/// How many rows of keys it has: the function row, the four rows of
/// characters, and the row of modifiers, space and arrows.
pub const ROW_COUNT: usize = 6;
/// How tall the function row is, as a share of a full key: a strip a little
/// over half height.
pub const FUNCTION_ROW_SCALE: f32 = 0.56;

/// How tall a row is drawn, as a share of a full key.
fn row_scale(row: usize) -> f32 {
    if row == 0 {
        FUNCTION_ROW_SCALE
    } else {
        1.0
    }
}

/// How far down the keys a row starts, and how tall it is, at the reference
/// height.
fn row_band(row: usize) -> (f32, f32) {
    let top = (0..row)
        .map(|above| row_scale(above) * KEY_HEIGHT + KEY_GAP)
        .sum();
    (top, row_scale(row) * KEY_HEIGHT)
}

/// How tall all the keys together are, at the reference height.
fn keys_height() -> f32 {
    let (top, height) = row_band(ROW_COUNT - 1);
    top + height
}

/// The scale the board is drawn at on a display `width` × `height`: the one
/// everything is sized by, or smaller where the keys would not fit across the
/// display — a display standing on its side shrinks the whole board rather
/// than letting it run off both sides.
fn board_scale(width: f32, height: f32) -> f32 {
    let scale = scale_for(height);
    let natural = (KEY_UNIT * COLUMNS + (BOARD_PADDING + BOARD_MARGIN) * 2.0) * scale;
    if width > 0.0 && natural > width {
        scale * (width / natural)
    } else {
        scale
    }
}

/// Where LineXinBar's on-screen keyboard settles over a display `width` ×
/// `height`, `[x, y, w, h]` in the display's pixels: centred, floating its
/// margin clear of the foot.
///
/// For a window that fills the display, as every application under LineXinBar
/// does, these are the window's own pixels: a field being typed into with the
/// shell's board belongs above `y`. The board rises there from below the
/// display's edge over about a quarter of a second, and only while the shell
/// has a controller in hand — an application that asked for it with a pad's
/// press knows when that is.
pub fn rect(width: f32, height: f32) -> [f32; 4] {
    let scale = board_scale(width, height);
    let w = (KEY_UNIT * COLUMNS + BOARD_PADDING * 2.0) * scale;
    let h = (keys_height() + BOARD_PADDING * 2.0) * scale;
    [(width - w) * 0.5, height - BOARD_MARGIN * scale - h, w, h]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// At the reference height the board is its own size, centred along the
    /// foot: the function row's strip, five full rows, the gaps between them
    /// and the glass round them.
    #[test]
    fn at_the_reference_height_the_board_is_its_own_size() {
        let keys = FUNCTION_ROW_SCALE * KEY_HEIGHT + 5.0 * KEY_HEIGHT + 5.0 * KEY_GAP;
        assert!((keys_height() - keys).abs() < 1e-3);
        let [x, y, w, h] = rect(1920.0, 1080.0);
        assert_eq!(w, KEY_UNIT * COLUMNS + BOARD_PADDING * 2.0);
        assert!((h - (keys + BOARD_PADDING * 2.0)).abs() < 1e-3);
        assert_eq!(x, (1920.0 - w) * 0.5);
        assert!((y + h + BOARD_MARGIN - 1080.0).abs() < 1e-3);
    }

    /// Sized by the height, it covers the same share of every landscape
    /// display; on one standing on its side it is shrunk to fit across, and
    /// covers less of it.
    #[test]
    fn it_covers_the_same_share_of_every_landscape_display() {
        let share = |width: f32, height: f32| rect(width, height)[1] / height;
        let reference = share(1920.0, 1080.0);
        for (width, height) in [(1280.0, 800.0), (2560.0, 1440.0), (1280.0, 720.0)] {
            assert!(
                (share(width, height) - reference).abs() < 1e-4,
                "{width}×{height}"
            );
        }
        // As measured over a nested LineXinBar at 1280×800: from y ≈ 485.
        assert!((rect(1280.0, 800.0)[1] - 484.8).abs() < 0.5);
        let [x, y, w, _] = rect(1080.0, 1920.0);
        assert!(x >= 0.0 && x + w <= 1080.0, "it runs off the sides");
        assert!(
            y / 1920.0 > reference,
            "it covers no less of a portrait display"
        );
    }
}
