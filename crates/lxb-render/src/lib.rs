mod components;
pub mod faces;
mod files;
mod pointing;
mod renderer;
mod wallpaper_clock;

pub use components::{ContextMenu, Dialog, Entry, FilePicker, Press, Pressing, Selection};
pub use files::Files;
pub use pointing::Spot;
pub use renderer::{instance, Ui, Written};
pub use wallpaper_clock::{monotonic_now_ns, WallpaperClock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Centre,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Fit {
    #[default]
    Contain,
    Cover,
}
