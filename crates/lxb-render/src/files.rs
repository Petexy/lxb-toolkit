use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, TryRecvError};

use lxb_toolkit::input::{Action, Key};
use lxb_toolkit::picker::{Purpose, Selection};
use lxb_toolkit::sound::Sound;

use crate::components::FilePicker;
use crate::pointing::Spot;
use crate::renderer::Ui;

#[derive(Default)]
pub struct Files {
    picker: FilePicker,
    answer: Vec<PathBuf>,
    asking: Option<Receiver<lxb_portal::Answer>>,
    asked: Option<(Purpose, Selection, PathBuf, String)>,
    own_questions: bool,
}

impl Files {
    pub fn own_questions(&mut self) {
        self.own_questions = true;
    }

    pub fn one_file(&mut self, selection: Selection, at: impl AsRef<Path>) -> bool {
        self.ask(Purpose::of(selection), selection, at, "")
    }

    pub fn many_files(&mut self, selection: Selection, at: impl AsRef<Path>) -> bool {
        self.ask(Purpose::ManyFiles, selection, at, "")
    }

    pub fn a_folder(&mut self, at: impl AsRef<Path>) -> bool {
        self.ask(Purpose::AFolder, Selection::Folder, at, "")
    }

    pub fn somewhere_to_save(&mut self, name: &str, at: impl AsRef<Path>) -> bool {
        self.ask(Purpose::ANewFile, Selection::File, at, name)
    }

    pub fn ask(
        &mut self,
        purpose: Purpose,
        selection: Selection,
        at: impl AsRef<Path>,
        name: &str,
    ) -> bool {
        if self.busy() {
            return false;
        }
        let at = at.as_ref().to_path_buf();
        if self.ask_the_desktop(purpose, selection, &at, name) {
            return true;
        }
        self.picker.open_for(purpose, selection, at, name)
    }

    pub fn answered(&mut self) -> Option<Vec<PathBuf>> {
        (!self.answer.is_empty()).then(|| std::mem::take(&mut self.answer))
    }

    pub fn busy(&self) -> bool {
        self.picker.is_open() || self.asking.is_some()
    }

    pub fn is_open(&self) -> bool {
        self.picker.is_open()
    }

    pub fn asking_the_desktop(&self) -> bool {
        self.asking.is_some()
    }

    pub fn showing(&self) -> bool {
        self.picker.showing()
    }

    pub fn typing(&self) -> bool {
        self.picker.search_keyboard_open()
    }

    pub fn hand(&mut self, pad: bool) {
        self.picker.hand(pad);
    }

    pub fn advance(&mut self, dt: f32) -> bool {
        self.hear_the_desktop();
        self.picker.advance(dt)
    }

    pub fn draw(&mut self, ui: &mut Ui) {
        ui.file_picker(&mut self.picker);
    }

    pub fn act(&mut self, action: Action) -> Option<Sound> {
        if !self.picker.is_open() {
            return None;
        }
        if self.picker.search_keyboard_open() {
            return match action {
                Action::Up => self
                    .picker
                    .move_search_keyboard(0, -1)
                    .then_some(Sound::Move),
                Action::Down => self
                    .picker
                    .move_search_keyboard(0, 1)
                    .then_some(Sound::Move),
                Action::Left => self
                    .picker
                    .move_search_keyboard(-1, 0)
                    .then_some(Sound::Move),
                Action::Right => self
                    .picker
                    .move_search_keyboard(1, 0)
                    .then_some(Sound::Move),
                Action::Accept => self.picker.press_search_keyboard().then_some(Sound::Press),
                Action::Submit => self.picker.submit_search_keyboard().then_some(Sound::Press),
                Action::Back => self.picker.close_search_keyboard().then_some(Sound::Back),
                _ => None,
            };
        }
        if self.picker.menu_is_open() {
            return match action {
                Action::Up => self.picker.menu_step(-1).then_some(Sound::Move),
                Action::Down => self.picker.menu_step(1).then_some(Sound::Move),
                Action::Accept | Action::Submit => self.picker.menu_press().then_some(Sound::Press),
                Action::Back | Action::Menu => self.picker.close_menu().then_some(Sound::Back),
                _ => None,
            };
        }
        match action {
            Action::Up => self.picker.move_selection(-1).then_some(Sound::Move),
            Action::Down => self.picker.move_selection(1).then_some(Sound::Move),
            Action::Left => self.picker.leave().then_some(Sound::Back),
            Action::Right => self.picker.enter().then_some(Sound::Press),
            Action::Accept | Action::Submit => {
                self.picker.press();
                let chose = self.picker.chose();
                if chose.is_empty() {
                    return self.picker.activate().then_some(Sound::Press);
                }
                self.answer = chose;
                self.picker.close();
                Some(Sound::Press)
            }
            Action::Menu => self.picker.open_menu().then_some(Sound::Press),
            Action::Back => {
                self.picker.close();
                Some(Sound::Back)
            }
            _ => None,
        }
    }

    pub fn key(&mut self, key: Option<Key>, text: Option<&str>) -> bool {
        if !self.picker.is_open() {
            return false;
        }
        if self.picker.search_keyboard_open() {
            match key {
                Some(Key::Enter) => {
                    self.picker.submit_search_keyboard();
                }
                Some(Key::Backspace) => {
                    self.picker.erase_search();
                    self.picker.close_search_keyboard();
                }
                Some(Key::Escape) => {
                    self.picker.close_search_keyboard();
                }
                _ => {
                    if let Some(text) = text {
                        self.picker.type_text(text);
                    } else if let Some(Key::Letter(letter)) = key {
                        self.picker.type_text(&letter.to_string());
                    } else if matches!(key, Some(Key::Space)) {
                        self.picker.type_text(" ");
                    }
                    self.picker.close_search_keyboard();
                }
            }
            return true;
        }
        if matches!(key, Some(Key::Backspace)) && self.picker.erase_search() {
            return true;
        }
        if let Some(text) = text {
            if self.picker.type_text(text) {
                return true;
            }
        }
        if let Some(Key::Letter(letter)) = key {
            if self.picker.type_text(&letter.to_string()) {
                return true;
            }
        } else if matches!(key, Some(Key::Space)) && self.picker.type_text(" ") {
            return true;
        }
        false
    }

    pub fn point_at(&mut self, spot: Spot) -> bool {
        if !self.picker.is_open() {
            return false;
        }
        if self.picker.menu_is_open() {
            return self.picker.menu_point_at(spot);
        }
        if self.picker.search_keyboard_open() {
            return self.picker.point_at(spot);
        }
        matches!(spot, Spot::PickerRow(_)) && self.picker.point_at(spot)
    }

    pub fn press_at(&mut self, spot: Spot, right: bool) -> Option<Sound> {
        if !self.picker.is_open() {
            return None;
        }
        if self.picker.menu_is_open() {
            return match spot {
                Spot::MenuRow { .. } => {
                    if self.picker.menu_point_at(spot) {
                        Some(Sound::Move)
                    } else {
                        self.picker.menu_press().then_some(Sound::Press)
                    }
                }
                _ => self.picker.close_menu().then_some(Sound::Back),
            };
        }
        if matches!(spot, Spot::OutsidePicker) {
            self.picker.close();
            return Some(Sound::Back);
        }
        if right {
            return self.act(Action::Menu);
        }
        if self.picker.search_keyboard_open() {
            return match spot {
                Spot::PickerKey { row, column } => self
                    .picker
                    .press_search_key(row, column)
                    .then_some(Sound::Press),
                _ => None,
            };
        }
        match spot {
            Spot::PickerRow(_) => {
                if self.picker.point_at(spot) {
                    Some(Sound::Move)
                } else {
                    self.act(Action::Accept)
                }
            }
            Spot::PickerTrail(steps) => self.picker.leave_to(steps).then_some(Sound::Back),
            Spot::PickerLeave => self.act(Action::Left),
            Spot::PickerKey { row, column } => self
                .picker
                .press_search_key(row, column)
                .then_some(Sound::Press),
            _ => None,
        }
    }

    pub fn open_menu(&mut self) -> bool {
        self.picker.open_menu()
    }

    pub fn close(&mut self) -> bool {
        if !self.picker.is_open() {
            return false;
        }
        self.picker.close();
        true
    }

    fn ask_the_desktop(
        &mut self,
        purpose: Purpose,
        selection: Selection,
        at: &Path,
        name: &str,
    ) -> bool {
        if self.own_questions || !the_desktop_may_be_asked() {
            return false;
        }
        let Some(portal) = lxb_portal::Portal::open_for(purpose) else {
            return false;
        };
        let mut question = lxb_portal::Question::new(purpose)
            .at(at)
            .called(name)
            .of(selection.kind());
        question.accept = purpose.accept().unwrap_or_default().to_string();
        let (say, hear) = channel();
        let Ok(_) = std::thread::Builder::new()
            .name("lxb-file-question".to_string())
            .spawn(move || {
                let _ = say.send(portal.ask(&question, lxb_portal::PATIENCE));
            })
        else {
            return false;
        };
        self.asking = Some(hear);
        self.asked = Some((purpose, selection, at.to_path_buf(), name.to_string()));
        true
    }

    fn hear_the_desktop(&mut self) {
        let Some(hearing) = self.asking.as_ref() else {
            return;
        };
        let answer = match hearing.try_recv() {
            Ok(answer) => answer,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                lxb_portal::Answer::Broken("nobody answered the question".to_string())
            }
        };
        self.asking = None;
        let asked = self.asked.take();
        match answer {
            lxb_portal::Answer::Chose(files) => self.answer = files,
            lxb_portal::Answer::Cancelled => {}
            lxb_portal::Answer::Broken(_) => {
                if let Some((purpose, selection, at, name)) = asked {
                    self.picker.open_for(purpose, selection, at, &name);
                }
            }
        }
    }
}

fn the_desktop_may_be_asked() -> bool {
    match std::env::var("LXB_FILE_PORTAL") {
        Ok(said) => !matches!(
            said.trim().to_ascii_lowercase().as_str(),
            "0" | "no" | "off" | "never" | "false"
        ),
        Err(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "lxb-files-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).expect("a directory to walk");
        path
    }

    #[test]
    fn a_question_is_asked_once_and_answered_once() {
        let at = scratch("once");
        std::fs::write(at.join("only.txt"), b"x").expect("a file");

        let mut files = Files::default();
        files.own_questions();
        assert!(!files.busy());
        assert!(files.one_file(Selection::File, &at));
        assert!(files.busy());
        assert!(!files.one_file(Selection::File, &at), "one at a time");

        assert_eq!(files.answered(), None, "nothing has been chosen yet");
        assert_eq!(files.act(Action::Accept), Some(Sound::Press));
        assert_eq!(files.answered(), Some(vec![at.join("only.txt")]));
        assert_eq!(files.answered(), None, "the answer is one-shot");
        assert!(!files.busy(), "and the panel went with it");

        let _ = std::fs::remove_dir_all(&at);
    }

    #[test]
    fn every_way_out_is_a_way_out() {
        let at = scratch("out");
        std::fs::write(at.join("only.txt"), b"x").expect("a file");

        for (what, leave) in [
            ("Back", Files::back as fn(&mut Files)),
            ("a click past the panel", Files::outside),
        ] {
            let mut files = Files::default();
            files.own_questions();
            assert!(files.one_file(Selection::File, &at));
            leave(&mut files);
            assert!(!files.is_open(), "{what} did not close it");
            assert_eq!(files.answered(), None, "{what} answered with something");
        }
        let _ = std::fs::remove_dir_all(&at);
    }

    impl Files {
        fn back(&mut self) {
            self.act(Action::Back);
        }
        fn outside(&mut self) {
            self.press_at(Spot::OutsidePicker, false);
        }
    }

    #[test]
    fn the_menu_is_reached_by_the_button_the_legend_names() {
        let at = scratch("menu");
        std::fs::write(at.join("only.txt"), b"x").expect("a file");

        let mut files = Files::default();
        files.own_questions();
        assert!(files.one_file(Selection::File, &at));

        assert_eq!(files.press_at(Spot::PickerRow(0), true), Some(Sound::Press));
        assert!(files.picker.menu_is_open(), "the right button raises it");
        assert_eq!(files.act(Action::Back), Some(Sound::Back));
        assert!(!files.picker.menu_is_open());

        assert_eq!(files.act(Action::Menu), Some(Sound::Press));
        assert!(files.picker.menu_is_open(), "and so does the Menu action");
        assert!(files.is_open(), "closing the menu is not closing the panel");

        let _ = std::fs::remove_dir_all(&at);
    }

    #[test]
    fn nothing_is_answered_while_no_question_is_open() {
        let mut files = Files::default();
        files.own_questions();
        assert_eq!(files.act(Action::Accept), None);
        assert!(!files.key(Some(Key::Letter('a')), None));
        assert!(!files.point_at(Spot::PickerRow(0)));
        assert_eq!(files.press_at(Spot::PickerRow(0), false), None);
        assert!(!files.close());
        assert_eq!(files.answered(), None);
    }
}
