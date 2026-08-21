use crate::Ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spot {
    Control(u32),

    MenuRow { row: usize, aside: bool },

    DialogButton(usize),

    OutsidePanel,

    Nothing,
}

impl Spot {
    pub fn pressable(self) -> bool {
        !matches!(self, Spot::Nothing)
    }
}

impl Ui {
    pub fn spot(&mut self, id: u32, rect: [f32; 4]) {
        self.mark_spot(Spot::Control(id), rect);
    }

    pub fn at(&self, x: f32, y: f32) -> Spot {
        self.spots
            .iter()
            .rev()
            .find(|(_, rect)| inside(*rect, x, y))
            .map(|(spot, _)| *spot)
            .unwrap_or(Spot::Nothing)
    }

    pub(crate) fn mark_spot(&mut self, spot: Spot, rect: [f32; 4]) {
        if rect[2] > 0.0 && rect[3] > 0.0 {
            self.spots.push((spot, rect));
        }
    }
}

fn inside([x, y, w, h]: [f32; 4], px: f32, py: f32) -> bool {
    px >= x && px < x + w && py >= y && py < y + h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rectangle_holds_its_left_and_top_edge_and_not_its_right() {
        assert!(inside([10.0, 20.0, 30.0, 40.0], 10.0, 20.0));
        assert!(inside([10.0, 20.0, 30.0, 40.0], 39.9, 59.9));
        assert!(!inside([10.0, 20.0, 30.0, 40.0], 40.0, 40.0));
        assert!(!inside([10.0, 20.0, 30.0, 40.0], 20.0, 60.0));
        assert!(!inside([10.0, 20.0, 30.0, 40.0], 9.9, 30.0));
    }

    #[test]
    fn everything_but_nothing_is_worth_pressing() {
        assert!(Spot::Control(0).pressable());
        assert!(Spot::MenuRow {
            row: 0,
            aside: false
        }
        .pressable());
        assert!(Spot::DialogButton(0).pressable());
        assert!(Spot::OutsidePanel.pressable());
        assert!(!Spot::Nothing.pressable());
    }
}
