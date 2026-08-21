use winit::event::KeyEvent;
use winit::keyboard::{Key as WinitKey, NamedKey};

use lxb_toolkit::input::Key;

pub fn key_of(event: &KeyEvent, shift: bool) -> Option<Key> {
    match event.logical_key {
        WinitKey::Named(NamedKey::ArrowLeft) => Some(Key::Left),
        WinitKey::Named(NamedKey::ArrowRight) => Some(Key::Right),
        WinitKey::Named(NamedKey::ArrowUp) => Some(Key::Up),
        WinitKey::Named(NamedKey::ArrowDown) => Some(Key::Down),
        WinitKey::Named(NamedKey::Enter) => Some(Key::Enter),
        WinitKey::Named(NamedKey::Space) => Some(Key::Space),
        WinitKey::Named(NamedKey::Escape) => Some(Key::Escape),
        WinitKey::Named(NamedKey::Backspace) => Some(Key::Backspace),
        WinitKey::Named(NamedKey::Tab) => Some(if shift { Key::BackTab } else { Key::Tab }),
        WinitKey::Named(NamedKey::ContextMenu) => Some(Key::Menu),
        WinitKey::Named(NamedKey::F10) => Some(Key::F10),

        WinitKey::Character(ref text) => match text.chars().next() {
            Some(' ') => Some(Key::Space),
            Some(letter) if text.chars().count() == 1 => Some(Key::Letter(letter)),
            _ => None,
        },
        _ => None,
    }
}
