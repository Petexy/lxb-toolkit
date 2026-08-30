use winit::event::KeyEvent;
use winit::keyboard::{Key as WinitKey, NamedKey};

use lxb_toolkit::input::Key;

pub fn key_of(event: &KeyEvent, shift: bool) -> Option<Key> {
    named(&event.logical_key, shift)
}

/// The same, given the logical key alone.
///
/// Split out because a `KeyEvent` cannot be built outside winit — one of its
/// fields is crate-private — so this is the largest part of the mapping a test
/// can reach. See [`tests::the_menu_key_is_the_context_menus`].
fn named(logical: &WinitKey, shift: bool) -> Option<Key> {
    match *logical {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The Menu key raises the context menu, which is what the Menu key does on
    /// every other system on the machine — and is what the right mouse button
    /// does in `lxb-app`, two branches apart in the same event handler.
    ///
    /// Pinned here because this crate is the only place the physical key is
    /// named. winit reports it as `NamedKey::ContextMenu` (X11 keycode 135,
    /// keysym `0xff67`, `KEY_COMPOSE` to the kernel), and everything after this
    /// point is talking about `Key::Menu`, which `Action::of_key` already sends
    /// to `Action::Menu`.
    #[test]
    fn the_menu_key_is_the_context_menus() {
        assert_eq!(
            named(&WinitKey::Named(NamedKey::ContextMenu), false),
            Some(Key::Menu)
        );
        // The other chord a desktop spells it with. `Shift+F10` arrives as
        // plain F10, so the shift flag must not change the answer.
        for shift in [false, true] {
            assert_eq!(
                named(&WinitKey::Named(NamedKey::F10), shift),
                Some(Key::F10)
            );
        }
    }
}
