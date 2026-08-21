pub const REFERENCE_HEIGHT: f32 = 1080.0;

pub fn scale_for(height: f32) -> f32 {
    (height / REFERENCE_HEIGHT).clamp(0.6, 2.5)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Metric {
    CardRadius,

    PanelRadius,

    PanelInset,

    RowHeight,

    RowPadding,

    PanelPadding,

    Tile,
    Gap,

    TileGlyph,

    TileRadius,

    ItemSpacing,
    ColumnSpacing,

    ItemIcon,
    ItemIconFocused,
    ColumnIcon,
    ColumnIconFocused,

    MenuWidth,

    DialogWidth,

    DialogDim,

    PowerWidth,
    PowerDim,
}

impl Metric {
    pub const ALL: [Metric; 21] = [
        Metric::CardRadius,
        Metric::PanelRadius,
        Metric::PanelInset,
        Metric::RowHeight,
        Metric::RowPadding,
        Metric::PanelPadding,
        Metric::Tile,
        Metric::Gap,
        Metric::TileGlyph,
        Metric::TileRadius,
        Metric::ItemSpacing,
        Metric::ColumnSpacing,
        Metric::ItemIcon,
        Metric::ItemIconFocused,
        Metric::ColumnIcon,
        Metric::ColumnIconFocused,
        Metric::MenuWidth,
        Metric::DialogWidth,
        Metric::DialogDim,
        Metric::PowerWidth,
        Metric::PowerDim,
    ];

    pub const fn value(self) -> f32 {
        match self {
            Metric::CardRadius => 18.0,
            Metric::PanelRadius => 30.0,
            Metric::PanelInset => 14.0,
            Metric::RowHeight => 76.0,
            Metric::RowPadding => 24.0,
            Metric::PanelPadding => 44.0,
            Metric::Tile => 68.0,
            Metric::Gap => 14.0,
            Metric::TileGlyph => 0.62,
            Metric::TileRadius => 0.30,
            Metric::ItemSpacing => 124.0,
            Metric::ColumnSpacing => 200.0,
            Metric::ItemIcon => 64.0,
            Metric::ItemIconFocused => 105.0,
            Metric::ColumnIcon => 84.0,
            Metric::ColumnIconFocused => 148.0,
            Metric::MenuWidth => 440.0,
            Metric::DialogWidth => 680.0,
            Metric::DialogDim => 0.30,
            Metric::PowerWidth => 460.0,
            Metric::PowerDim => 0.28,
        }
    }

    pub const fn is_share(self) -> bool {
        matches!(
            self,
            Metric::TileGlyph | Metric::TileRadius | Metric::DialogDim | Metric::PowerDim
        )
    }

    pub const fn name(self) -> &'static str {
        match self {
            Metric::CardRadius => "card-radius",
            Metric::PanelRadius => "panel-radius",
            Metric::PanelInset => "panel-inset",
            Metric::RowHeight => "row-height",
            Metric::RowPadding => "row-padding",
            Metric::PanelPadding => "panel-padding",
            Metric::Tile => "tile",
            Metric::Gap => "gap",
            Metric::TileGlyph => "tile-glyph",
            Metric::TileRadius => "tile-radius",
            Metric::ItemSpacing => "item-spacing",
            Metric::ColumnSpacing => "column-spacing",
            Metric::ItemIcon => "item-icon",
            Metric::ItemIconFocused => "item-icon-focused",
            Metric::ColumnIcon => "column-icon",
            Metric::ColumnIconFocused => "column-icon-focused",
            Metric::MenuWidth => "menu-width",
            Metric::DialogWidth => "dialog-width",
            Metric::DialogDim => "dialog-dim",
            Metric::PowerWidth => "power-width",
            Metric::PowerDim => "power-dim",
        }
    }

    pub fn on(self, height: f32) -> f32 {
        if self.is_share() {
            self.value()
        } else {
            self.value() * scale_for(height)
        }
    }
}

pub fn capsule_radius(height: f32) -> f32 {
    height / 2.0
}

pub const CORNER_CIRCLE: f32 = 2.0;
pub const CORNER_SQUIRCLE: f32 = 4.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reference_screen_scales_by_one() {
        assert_eq!(scale_for(REFERENCE_HEIGHT), 1.0);
        assert_eq!(Metric::CardRadius.on(REFERENCE_HEIGHT), 18.0);
        assert_eq!(Metric::CardRadius.on(2160.0), 36.0, "4K is twice this");
        assert_eq!(Metric::CardRadius.on(720.0), 12.0);
        assert_eq!(scale_for(100.0), 0.6);
        assert_eq!(scale_for(10_000.0), 2.5);
    }

    #[test]
    fn a_share_is_not_a_length() {
        for metric in Metric::ALL.iter().filter(|m| m.is_share()) {
            assert_eq!(metric.on(2160.0), metric.value(), "{}", metric.name());
            assert!(metric.value() > 0.0 && metric.value() <= 1.0);
        }
    }

    #[test]
    fn a_control_is_a_capsule() {
        assert_eq!(capsule_radius(76.0), 38.0);
        assert_eq!(capsule_radius(Metric::RowHeight.on(2160.0)), 76.0);
    }

    #[test]
    fn the_cursor_makes_a_mark_grow() {
        assert!(Metric::ItemIconFocused.value() > Metric::ItemIcon.value());
        assert!(Metric::ColumnIconFocused.value() > Metric::ColumnIcon.value());

        assert!(Metric::ColumnIcon.value() > Metric::ItemIcon.value());
    }

    #[test]
    fn metric_names_are_unique() {
        let mut names: Vec<&str> = Metric::ALL.iter().map(|m| m.name()).collect();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count);
    }

    #[test]
    fn questions_keep_their_distinct_cuts() {
        assert_eq!(Metric::MenuWidth.value(), 440.0);
        assert_eq!(Metric::DialogWidth.value(), 680.0);
        assert_eq!(Metric::DialogDim.value(), 0.30);
        assert_eq!(Metric::PowerWidth.value(), 460.0);
        assert_eq!(Metric::PowerDim.value(), 0.28);
    }
}
