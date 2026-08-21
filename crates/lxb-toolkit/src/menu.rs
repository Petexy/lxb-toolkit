use crate::metrics::Metric;

pub const WIDTH: f32 = Metric::MenuWidth.value();

pub const EXTRA_WIDTH: f32 = 100.0;

pub const ROW: f32 = 64.0;

pub const STACKED_ROW: f32 = 96.0;

pub const TITLE: f32 = 62.0;

pub const TITLE_SIZE: f32 = 24.0;
pub const LABEL_SIZE: f32 = 23.0;
pub const DETAIL_SIZE: f32 = 20.0;

pub const STAMP_SIZE: f32 = 18.0;

pub const MAX_LINES: u8 = 3;

pub const GROUP_GAP: f32 = 20.0;

pub const GAP: f32 = 16.0;

pub const GLOW_REACH: f32 = 1.35;

pub const SCROLL_STRIP: f32 = 22.0;
pub const SCROLL_ARROW: f32 = 14.0;

pub const DIM: f32 = 0.42;

pub const SCRIM: f32 = 0.55;

pub const DEPTH: f32 = 0.10;

pub const CONTENT_IN: f32 = 0.45;

pub const PANEL_IN: f32 = 0.25;

pub const ICON: f32 = 0.70;

pub const GLYPH: f32 = 0.46;

pub const ASIDE: f32 = 1.0;
pub const ASIDE_GAP: f32 = 12.0;
pub const ASIDE_GLYPH: f32 = 0.42;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Menu {
    pub width: f32,
    pub extra_width: f32,
    pub row: f32,
    pub stacked_row: f32,
    pub title: f32,
    pub title_size: f32,
    pub label_size: f32,
    pub detail_size: f32,
    pub stamp_size: f32,
    pub max_lines: u8,
    pub group_gap: f32,
    pub gap: f32,
    pub glow_reach: f32,
    pub scroll_strip: f32,
    pub scroll_arrow: f32,
    pub dim: f32,
    pub scrim: f32,
    pub depth: f32,
    pub content_in: f32,
    pub panel_in: f32,
    pub icon: f32,
    pub glyph: f32,
    pub aside: f32,
    pub aside_gap: f32,
    pub aside_glyph: f32,
}

pub const CONTEXT: Menu = Menu {
    width: WIDTH,
    extra_width: EXTRA_WIDTH,
    row: ROW,
    stacked_row: STACKED_ROW,
    title: TITLE,
    title_size: TITLE_SIZE,
    label_size: LABEL_SIZE,
    detail_size: DETAIL_SIZE,
    stamp_size: STAMP_SIZE,
    max_lines: MAX_LINES,
    group_gap: GROUP_GAP,
    gap: GAP,
    glow_reach: GLOW_REACH,
    scroll_strip: SCROLL_STRIP,
    scroll_arrow: SCROLL_ARROW,
    dim: DIM,
    scrim: SCRIM,
    depth: DEPTH,
    content_in: CONTENT_IN,
    panel_in: PANEL_IN,
    icon: ICON,
    glyph: GLYPH,
    aside: ASIDE,
    aside_gap: ASIDE_GAP,
    aside_glyph: ASIDE_GLYPH,
};

impl Menu {
    pub fn shown(&self, travelled: f32, opening: bool) -> f32 {
        let travelled = travelled.clamp(0.0, 1.0);
        if opening {
            crate::motion::ease(travelled / self.panel_in)
        } else {
            travelled
        }
    }

    pub fn content_shown(&self, travelled: f32, opening: bool) -> f32 {
        let travelled = travelled.clamp(0.0, 1.0);
        if opening {
            crate::motion::ease((travelled - self.content_in) / (1.0 - self.content_in))
        } else {
            travelled
        }
    }
}

pub const MARGIN: f32 = 24.0;

pub const LABEL_PADDING: f32 = Metric::RowPadding.value();

pub const ROW_PADDING: f32 = crate::control::PADDING;

pub const TITLE_LINE: f32 = TITLE_SIZE * crate::typography::Text::LINE;
pub const LABEL_LINE: f32 = LABEL_SIZE * crate::typography::Text::LINE;
pub const DETAIL_LINE: f32 = DETAIL_SIZE * crate::typography::Text::LINE;

pub const STAMP_ROOM: f32 = STAMP_SIZE * crate::typography::Text::LINE;

pub const ASIDE_RADIUS: f32 = 0.32;

pub const BADGE: f32 = 0.46;
pub const BADGE_AT: f32 = 0.34;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Row {
    pub stacked: bool,

    pub stamp: bool,

    pub aside: bool,

    pub reading: bool,

    pub group: u8,

    pub lines: u8,
    pub detail_lines: u8,
}

impl Row {
    pub const fn command(group: u8) -> Self {
        Self {
            stacked: false,
            stamp: false,
            aside: false,
            reading: false,
            group,
            lines: 1,
            detail_lines: 0,
        }
    }
}

pub fn panel_width(extra: f32) -> f32 {
    WIDTH + extra
}

pub fn row_height(row: &Row) -> f32 {
    let stacked = if row.stacked { STACKED_ROW } else { ROW };
    stacked + if row.stamp { STAMP_ROOM } else { 0.0 }
}

pub fn opening(row: &Row) -> (f32, f32) {
    let cap = if row.reading { u8::MAX } else { MAX_LINES };
    let extra = |lines: u8| lines.min(cap).saturating_sub(1) as f32;
    (
        extra(row.lines) * LABEL_LINE,
        extra(row.detail_lines) * DETAIL_LINE,
    )
}

pub fn row_settled(row: &Row) -> f32 {
    let base = row_height(row);
    if !row.reading {
        return base;
    }
    let (label, detail) = opening(row);
    base + label + detail
}

pub fn chip_height(row: &Row) -> f32 {
    row_settled(row) - ROW_PADDING * 2.0
}

pub fn aside_width(row: &Row) -> f32 {
    if row.aside {
        chip_height(row) * ASIDE
    } else {
        0.0
    }
}

pub fn aside_radius(row: &Row, height: f32) -> f32 {
    chip_height(row) * crate::metrics::scale_for(height) * ASIDE_RADIUS
}

pub fn chip_radius(row: &Row, drawn: f32, height: f32) -> f32 {
    if row_settled(row) > ROW {
        ((chip_height(row) * crate::metrics::scale_for(height)) * ASIDE_RADIUS).min(drawn * 0.5)
    } else {
        drawn * 0.5
    }
}

pub fn lines_in(room: f32, line: f32) -> u8 {
    1 + (room / line + 1e-3).floor().max(0.0) as u8
}

pub fn title_growth(lines: u8) -> f32 {
    (lines.max(1) - 1) as f32 * TITLE_LINE
}

pub fn title_height(lines: Option<u8>) -> f32 {
    match lines {
        Some(lines) => TITLE + title_growth(lines),
        None => 0.0,
    }
}

pub fn column_room(height: f32, reserve: f32) -> f32 {
    let scale = crate::metrics::scale_for(height);
    let furniture = (MARGIN * 2.0 + TITLE + SCROLL_STRIP * 2.0 + GROUP_GAP + reserve) * scale;
    height - Metric::PanelInset.value() * scale * 2.0 - furniture
}

pub fn rows_that_fit(height: f32, row: f32) -> usize {
    let scale = crate::metrics::scale_for(height);
    ((column_room(height, 0.0) / ((row + GROUP_GAP) * scale)) as usize).max(1)
}

pub fn rows_of_that_fit(rows: &[Row], title_lines: Option<u8>, height: f32) -> usize {
    let opening_most = rows
        .iter()
        .filter(|row| !row.reading)
        .map(|row| {
            let (label, detail) = opening(row);
            label + detail
        })
        .fold(0.0, f32::max);

    let scale = crate::metrics::scale_for(height);
    let reserve = opening_most + title_lines.map_or(0.0, title_growth);
    let mut room = column_room(height, reserve);
    let mut fit = 0;
    for row in rows {
        let span = (row_settled(row) + GROUP_GAP) * scale;
        if fit > 0 && span > room {
            break;
        }
        room -= span;
        fit += 1;
    }
    fit.max(1)
}

#[derive(Debug, Clone, Copy)]
pub struct Layout<'a> {
    rows: &'a [Row],

    pub rect: [f32; 4],

    pub first: usize,
    pub visible: usize,

    pub selected: usize,
    pub unfolded: f32,

    pub title_lines: Option<u8>,
    scale: f32,
}

impl<'a> Layout<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        rows: &'a [Row],
        title_lines: Option<u8>,
        anchor: [f32; 4],
        display: [f32; 2],
        first: usize,
        visible: usize,
        selected: usize,
        unfolded: f32,
        extra: f32,
    ) -> Self {
        let scale = crate::metrics::scale_for(display[1]);
        let visible = visible.min(rows.len()).max(1);
        let first = first.min(rows.len().saturating_sub(visible));
        let scrolling = visible < rows.len();
        let body: f32 = (first..(first + visible).min(rows.len()))
            .map(|index| span(rows, index, first, selected, unfolded))
            .sum();
        let tall = (top(title_lines, scrolling)
            + body
            + if scrolling { SCROLL_STRIP } else { 0.0 }
            + MARGIN)
            * scale;
        let rect = place(anchor, [panel_width(extra) * scale, tall], display);
        Self {
            rows,
            rect,
            first,
            visible,
            selected,
            unfolded,
            title_lines,
            scale,
        }
    }

    pub fn scrolls(&self) -> bool {
        self.visible < self.rows.len()
    }

    pub fn rows_top(&self) -> f32 {
        top(self.title_lines, self.scrolls()) * self.scale
    }

    pub fn row(&self, index: usize) -> Option<[f32; 4]> {
        let margin = MARGIN * self.scale;
        let padding = ROW_PADDING * self.scale;
        if index < self.first || index >= self.first + self.visible || index >= self.rows.len() {
            return None;
        }
        let mut y = self.rect[1] + self.rows_top();
        for row in self.first..=index {
            let line = height_at(self.rows, row, self.selected, self.unfolded);
            y += (span(self.rows, row, self.first, self.selected, self.unfolded) - line)
                * self.scale;
            if row == index {
                return Some([
                    self.rect[0] + margin,
                    y + padding,
                    self.rect[2] - margin * 2.0,
                    line * self.scale - padding * 2.0,
                ]);
            }
            y += line * self.scale;
        }
        None
    }

    pub fn chip(&self, index: usize) -> Option<[f32; 4]> {
        let row = self.rows.get(index)?;
        let line = self.row(index)?;
        if !row.aside {
            return Some(line);
        }
        let taken = (aside_width(row) + ASIDE_GAP) * self.scale;
        Some([line[0], line[1], (line[2] - taken).max(0.0), line[3]])
    }

    pub fn aside(&self, index: usize) -> Option<[f32; 4]> {
        let row = self.rows.get(index)?;
        if !row.aside {
            return None;
        }
        let line = self.row(index)?;
        let button = aside_width(row) * self.scale;
        Some([line[0] + line[2] - button, line[1], button, line[3]])
    }

    pub fn highlight(&self, on_aside: bool) -> Option<[f32; 4]> {
        if on_aside {
            if let Some(rect) = self.aside(self.selected) {
                return Some(rect);
            }
        }
        self.chip(self.selected)
    }

    pub fn separators(&self) -> Vec<[f32; 4]> {
        let side = MARGIN * self.scale;
        let rule = (self.scale).max(1.0);
        ((self.first + 1)..(self.first + self.visible).min(self.rows.len()))
            .filter(|index| self.rows[*index].group != self.rows[index - 1].group)
            .filter_map(|index| {
                let [_, above_y, _, above_h] = self.row(index - 1)?;
                let [_, below_y, _, _] = self.row(index)?;

                let y = (above_y + above_h + below_y - rule) * 0.5;
                Some([self.rect[0] + side, y, self.rect[2] - side * 2.0, rule])
            })
            .collect()
    }

    pub fn opened(&self, index: usize) -> (f32, f32) {
        let (label, detail) = growth(self.rows, index, self.selected, self.unfolded);
        (label * self.scale, detail * self.scale)
    }
}

fn top(title_lines: Option<u8>, scrolling: bool) -> f32 {
    MARGIN + title_height(title_lines) + if scrolling { SCROLL_STRIP } else { 0.0 }
}

fn growth(rows: &[Row], index: usize, selected: usize, unfolded: f32) -> (f32, f32) {
    let Some(row) = rows.get(index) else {
        return (0.0, 0.0);
    };
    let out = if row.reading {
        1.0
    } else if index == selected {
        unfolded
    } else {
        return (0.0, 0.0);
    };
    let (label, detail) = opening(row);
    (label * out, detail * out)
}

fn height_at(rows: &[Row], index: usize, selected: usize, unfolded: f32) -> f32 {
    let Some(row) = rows.get(index) else {
        return ROW;
    };
    let (label, detail) = growth(rows, index, selected, unfolded);
    row_height(row) + label + detail
}

fn span(rows: &[Row], index: usize, first: usize, selected: usize, unfolded: f32) -> f32 {
    let opened = index > first && rows[index].group != rows[index - 1].group;
    height_at(rows, index, selected, unfolded) + if opened { GROUP_GAP } else { 0.0 }
}

pub fn place(anchor: [f32; 4], size: [f32; 2], display: [f32; 2]) -> [f32; 4] {
    let scale = crate::metrics::scale_for(display[1]);
    let inset = Metric::PanelInset.value() * scale;
    let gap = GAP * scale;
    let panel_w = size[0].min((display[0] - inset * 2.0).max(0.0));
    let panel_h = size[1].min((display[1] - inset * 2.0).max(0.0));

    let [ax, ay, aw, ah] = anchor;
    let right = ax + aw + gap;
    let x = if right + panel_w <= display[0] - inset {
        right
    } else {
        (ax - gap - panel_w).max(inset)
    };

    let y = (ay + ah * 0.5 - panel_h * 0.5).clamp(inset, (display[1] - inset - panel_h).max(inset));
    [
        x.min((display[0] - inset - panel_w).max(inset)),
        y,
        panel_w,
        panel_h,
    ]
}

pub fn growing(anchor: [f32; 4], panel: [f32; 4], travelled: f32) -> [f32; 4] {
    let t = travelled.clamp(0.0, 1.0);
    let [ax, ay, aw, ah] = anchor;
    let [px, py, pw, ph] = panel;
    if pw <= 0.0 {
        return panel;
    }
    let lerp = |from: f32, to: f32| from + (to - from) * t;
    let factor = lerp((aw / pw).min(1.0), 1.0);
    let cx = lerp(ax + aw * 0.5, px + pw * 0.5);
    let cy = lerp(ay + ah * 0.5, py + ph * 0.5);
    [
        cx - pw * factor * 0.5,
        cy - ph * factor * 0.5,
        pw * factor,
        ph * factor,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panel_arrives_faster_than_it_leaves() {
        let shape = CONTEXT;
        assert_eq!(shape.shown(0.0, true), 0.0);
        assert_eq!(shape.shown(1.0, true), 1.0);
        assert_eq!(shape.shown(0.0, false), 0.0);
        assert_eq!(shape.shown(1.0, false), 1.0);

        assert_eq!(shape.shown(PANEL_IN, true), 1.0);
        assert_eq!(shape.shown(PANEL_IN * 0.5, true), 0.5);
        assert!(shape.shown(0.5, false) < shape.shown(PANEL_IN, true));

        assert!(shape.shown(0.75, false) > 0.7);
        assert_eq!(shape.shown(0.5, false), 0.5);

        let crossing = 1.0 / 16.0;
        assert!((shape.shown(crossing, true) - crossing).abs() < 1e-4);
        for step in 0..=32 {
            let travelled = crossing + (1.0 - crossing) * step as f32 / 32.0;
            assert!(
                shape.shown(travelled, false) <= shape.shown(travelled, true) + 1e-6,
                "at {travelled} the fold is fuller than the arrival"
            );
        }

        assert_eq!(shape.shown(2.0, false), 1.0);
        assert_eq!(shape.content_shown(-1.0, true), 0.0);
    }

    #[test]
    fn the_rows_wait_for_the_panel_and_leave_with_it() {
        let shape = CONTEXT;
        assert_eq!(shape.content_shown(CONTENT_IN, true), 0.0);
        assert!(shape.content_shown(CONTENT_IN - 0.01, true) <= 0.0);
        assert_eq!(shape.content_shown(1.0, true), 1.0);

        for step in 0..=20 {
            let travelled = step as f32 / 20.0;
            assert_eq!(
                shape.content_shown(travelled, false),
                shape.shown(travelled, false)
            );
        }
    }

    #[test]
    fn the_panel_stands_beside_what_it_is_about() {
        let rows = [Row::command(0), Row::command(0), Row::command(1)];
        let display = [1280.0, 800.0];
        let fit = rows_of_that_fit(&rows, Some(1), display[1]);
        assert_eq!(fit, 3);

        let anchor = [150.0, 200.0, 120.0, 120.0];
        let it = Layout::new(&rows, Some(1), anchor, display, 0, fit, 0, 0.0, 0.0);
        let [x, _, width, _] = it.rect;
        assert!(x > anchor[0] + anchor[2], "the panel is over its anchor");

        let right = [1100.0, 200.0, 120.0, 120.0];
        let flipped = Layout::new(&rows, Some(1), right, display, 0, fit, 0, 0.0, 0.0);
        assert!(flipped.rect[0] + flipped.rect[2] < right[0]);
        assert_eq!(flipped.rect[2], width, "it changed size on the way over");

        let inset = Metric::PanelInset.on(display[1]);
        assert!(it.rect[1] >= inset - 1e-3);
        assert!(it.rect[1] + it.rect[3] <= display[1] - inset + 1e-3);
    }

    #[test]
    fn a_change_of_band_opens_a_gap_and_rules_it() {
        let rows = [Row::command(0), Row::command(0), Row::command(1)];
        let it = Layout::new(
            &rows,
            Some(1),
            [150.0, 200.0, 120.0, 120.0],
            [1280.0, 800.0],
            0,
            3,
            0,
            0.0,
            0.0,
        );
        let first = it.row(0).expect("a drawn row");
        let second = it.row(1).expect("a drawn row");
        let third = it.row(2).expect("a drawn row");
        assert!((first[3] - second[3]).abs() < 1e-3);
        assert!(third[1] - second[1] > second[1] - first[1]);

        let rules = it.separators();
        assert_eq!(rules.len(), 1, "one rule, between the two bands");
        assert!(second[1] + second[3] < rules[0][1] && rules[0][1] < third[1]);

        assert!(rules[0][0] > it.rect[0] && rules[0][2] < it.rect[2]);

        assert_eq!(it.highlight(false), it.chip(0));
    }

    #[test]
    fn a_button_stands_beside_its_row_rather_than_on_it() {
        let rows = [Row {
            aside: true,
            ..Row::command(0)
        }];
        let it = Layout::new(
            &rows,
            None,
            [10.0, 400.0, 40.0, 40.0],
            [1280.0, 800.0],
            0,
            1,
            0,
            0.0,
            0.0,
        );
        let line = it.row(0).expect("a drawn row");
        let chip = it.chip(0).expect("a face");
        let button = it.aside(0).expect("a button");
        assert!(chip[2] < line[2], "the row kept its whole width");
        assert!(chip[0] + chip[2] < button[0], "they overlap");
        assert_eq!(button[0] + button[2], line[0] + line[2]);
        assert_eq!(button[3], line[3], "the pair are different heights");

        assert_eq!(it.highlight(true), Some(button));
        assert_eq!(it.highlight(false), Some(chip));
    }

    #[test]
    fn one_row_opens_out_at_a_time() {
        let rows = [
            Row {
                lines: 3,
                ..Row::command(0)
            },
            Row {
                lines: 3,
                ..Row::command(0)
            },
        ];
        let display = [1280.0, 800.0];
        let shut = Layout::new(
            &rows,
            None,
            [10.0, 400.0, 40.0, 40.0],
            display,
            0,
            2,
            0,
            0.0,
            0.0,
        );
        let open = Layout::new(
            &rows,
            None,
            [10.0, 400.0, 40.0, 40.0],
            display,
            0,
            2,
            0,
            1.0,
            0.0,
        );
        assert_eq!(shut.opened(0), (0.0, 0.0));
        assert!(open.opened(0).0 > 0.0);
        assert_eq!(open.opened(1), (0.0, 0.0), "a second row opened as well");
        assert!(open.row(0).unwrap()[3] > shut.row(0).unwrap()[3]);
        assert_eq!(open.row(1).unwrap()[3], shut.row(1).unwrap()[3]);

        assert!(
            (open.rect[3] - shut.rect[3] - (open.row(0).unwrap()[3] - shut.row(0).unwrap()[3]))
                .abs()
                < 1e-2
        );
    }

    #[test]
    fn a_row_to_be_read_is_open_before_anything_is_selected() {
        let reading = Row {
            reading: true,
            lines: 8,
            ..Row::command(0)
        };
        let scanning = Row {
            lines: 8,
            ..Row::command(0)
        };
        assert!(row_settled(&reading) > row_height(&reading));
        assert_eq!(row_settled(&scanning), row_height(&scanning));
        assert!(opening(&reading).0 > opening(&scanning).0);
        assert_eq!(
            opening(&scanning).0,
            (MAX_LINES - 1) as f32 * LABEL_LINE,
            "a row in a list opened past the cap"
        );
    }

    #[test]
    fn a_panel_draws_only_the_rows_it_has_room_for() {
        let rows: Vec<Row> = (0..40).map(|_| Row::command(0)).collect();
        let display = [1280.0, 800.0];
        let fit = rows_of_that_fit(&rows, Some(1), display[1]);
        assert!(fit > 1 && fit < rows.len());
        let it = Layout::new(
            &rows,
            Some(1),
            [10.0, 400.0, 40.0, 40.0],
            display,
            0,
            fit,
            0,
            0.0,
            0.0,
        );
        assert!(it.scrolls());
        assert!(it.row(fit - 1).is_some());
        assert!(it.row(fit).is_none(), "it drew a row it has no room for");
        assert!(it.rect[3] <= display[1] - Metric::PanelInset.on(display[1]) * 2.0);
    }

    #[test]
    fn the_width_is_the_metric_and_not_a_second_copy_of_it() {
        assert_eq!(CONTEXT.width, Metric::MenuWidth.value());
    }

    #[test]
    fn a_stacked_row_is_half_again_a_command_row() {
        assert_eq!(STACKED_ROW, ROW * 1.5);
    }

    #[test]
    fn the_panel_starts_over_its_anchor_and_ends_at_itself() {
        let anchor = [100.0, 200.0, 64.0, 64.0];
        let panel = [300.0, 400.0, 440.0, 320.0];
        let centre = |[x, y, w, h]: [f32; 4]| [x + w * 0.5, y + h * 0.5];
        let near =
            |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).abs() < 1e-3 && (a[1] - b[1]).abs() < 1e-3;

        let shut = growing(anchor, panel, 0.0);
        assert!(near(centre(shut), centre(anchor)), "{shut:?}");
        assert_eq!(growing(anchor, panel, 1.0), panel);

        assert_eq!(growing(anchor, panel, 1.4), panel);
        assert!(near(centre(growing(anchor, panel, -0.2)), centre(anchor)));
    }

    #[test]
    fn the_panel_never_takes_its_anchors_shape() {
        let anchor = [40.0, 600.0, 300.0, 48.0];
        let panel = [500.0, 200.0, 440.0, 700.0];
        let want = panel[2] / panel[3];
        for step in 0..=20 {
            let [_, _, w, h] = growing(anchor, panel, step as f32 / 20.0);
            assert!(w > 0.0 && h > 0.0);
            assert!(
                ((w / h) - want).abs() < 1e-3,
                "at {step}/20 it is {w}x{h}, which is not the panel's shape"
            );
        }

        let [_, _, w, _] = growing(anchor, panel, 0.0);
        assert!((w - anchor[2]).abs() < 1e-3, "it started {w} wide");

        let wide = [0.0, 0.0, 900.0, 48.0];
        assert_eq!(growing(wide, panel, 0.0)[2], panel[2]);
    }

    #[test]
    fn a_very_short_display_still_gets_one_row() {
        assert_eq!(rows_that_fit(1080.0, ROW), 10);
        assert!(rows_that_fit(60.0, ROW) >= 1);

        assert_eq!(rows_that_fit(2160.0, ROW), rows_that_fit(1080.0, ROW));
        assert_eq!(rows_that_fit(800.0, ROW), rows_that_fit(1080.0, ROW));
    }
}
