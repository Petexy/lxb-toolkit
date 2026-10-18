use winit::event::KeyEvent;
use winit::keyboard::{Key as WinitKey, NamedKey};

use lxb_toolkit::input::Key;

pub fn key_of(event: &KeyEvent, shift: bool) -> Option<Key> {
    key_of_logical(&event.logical_key, shift)
}

/// The same, given the logical key alone.
///
/// `shift` is whether Shift is held, as the window's last
/// `WindowEvent::ModifiersChanged` said. It turns Tab into BackTab and
/// changes no other key: a letter or a symbol already carries its case in the
/// logical key, and what is typed is the event's own text.
///
/// Public, and split out, because a `KeyEvent` cannot be built outside winit
/// — one of its fields is crate-private — so this is the largest part of the
/// mapping a test can reach, here or in a window that keeps the keyboard's
/// modifiers itself. See `tests::the_menu_key_is_the_context_menus`.
pub fn key_of_logical(logical: &WinitKey, shift: bool) -> Option<Key> {
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
            key_of_logical(&WinitKey::Named(NamedKey::ContextMenu), false),
            Some(Key::Menu)
        );
        // The other chord a desktop spells it with. `Shift+F10` arrives as
        // plain F10, so the shift flag must not change the answer.
        for shift in [false, true] {
            assert_eq!(
                key_of_logical(&WinitKey::Named(NamedKey::F10), shift),
                Some(Key::F10)
            );
        }
    }

    /// Shift+Tab is BackTab, which walks back (`Action::Previous`), and Shift
    /// changes no other key: every key the mapping names, and every letter,
    /// figure and symbol, is the same key with it held as without. A letter
    /// reads as the keyboard reports it, capital or not, whatever the flag.
    #[test]
    fn shift_turns_tab_back_and_no_other_key() {
        let tab = WinitKey::Named(NamedKey::Tab);
        assert_eq!(key_of_logical(&tab, false), Some(Key::Tab));
        assert_eq!(key_of_logical(&tab, true), Some(Key::BackTab));

        let named = [
            NamedKey::ArrowLeft,
            NamedKey::ArrowRight,
            NamedKey::ArrowUp,
            NamedKey::ArrowDown,
            NamedKey::Enter,
            NamedKey::Space,
            NamedKey::Escape,
            NamedKey::Backspace,
            NamedKey::ContextMenu,
            NamedKey::F10,
            NamedKey::Shift,
            NamedKey::Control,
            NamedKey::Home,
            NamedKey::Delete,
            NamedKey::F1,
        ];
        let characters = ["a", "A", "y", "Y", "1", "!", "?", " ", "ż", "Ż", "ab"];
        let others = named.into_iter().map(WinitKey::Named).chain(
            characters
                .into_iter()
                .map(|text| WinitKey::Character(text.into())),
        );
        for key in others {
            assert_eq!(
                key_of_logical(&key, true),
                key_of_logical(&key, false),
                "{key:?}"
            );
        }
        assert_eq!(
            key_of_logical(&WinitKey::Character("A".into()), true),
            Some(Key::Letter('A'))
        );
        assert_eq!(
            key_of_logical(&WinitKey::Character("!".into()), true),
            Some(Key::Letter('!'))
        );
    }
}
