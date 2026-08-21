mod components;
mod pointing;
mod renderer;

pub use components::{ContextMenu, Dialog, Entry, Press, Pressing, Selection};
pub use pointing::Spot;
pub use renderer::{instance, Ui};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Centre,
    Right,
}
