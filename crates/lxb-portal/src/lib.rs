mod dbus;

use std::ffi::OsString;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use lxb_toolkit::picker::{Kind, Purpose};

use crate::dbus::{Bus, Message, Value};

const DESKTOP: &str = "org.freedesktop.portal.Desktop";
const DESKTOP_PATH: &str = "/org/freedesktop/portal/desktop";
const FILE_CHOOSER: &str = "org.freedesktop.portal.FileChooser";
const REQUEST: &str = "org.freedesktop.portal.Request";
const PROPERTIES: &str = "org.freedesktop.DBus.Properties";
const BUS: &str = "org.freedesktop.DBus";
const BUS_PATH: &str = "/org/freedesktop/DBus";

const CHOSE: u32 = 0;
const CANCELLED: u32 = 1;

const FOLDERS_FROM: u32 = 3;

pub const PATIENCE: Duration = Duration::from_secs(30 * 60);

static NEXT: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Clone)]
pub struct Question {
    pub purpose: Purpose,
    pub title: String,
    pub accept: String,
    pub name: String,
    pub at: Option<PathBuf>,
    pub kinds: Vec<Kind>,
}

impl Question {
    pub fn new(purpose: Purpose) -> Self {
        Self {
            purpose,
            title: String::new(),
            accept: String::new(),
            name: String::new(),
            at: None,
            kinds: Vec::new(),
        }
    }

    pub fn at(mut self, directory: impl AsRef<Path>) -> Self {
        let directory = directory.as_ref();
        self.at = directory.is_dir().then(|| directory.to_path_buf());
        self
    }

    pub fn called(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn of(mut self, kind: Option<Kind>) -> Self {
        self.kinds = kind.into_iter().collect();
        self
    }

    fn heading(&self) -> String {
        if self.title.trim().is_empty() {
            self.purpose.asking().to_string()
        } else {
            self.title.clone()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    Chose(Vec<PathBuf>),
    Cancelled,
    Broken(String),
}

pub struct Portal {
    bus: Bus,
    version: u32,
}

impl Portal {
    pub fn open() -> Option<Self> {
        let mut bus = Bus::session().ok()?;
        let asked = Message::call(DESKTOP, DESKTOP_PATH, PROPERTIES, "Get")
            .with(vec![Value::text(FILE_CHOOSER), Value::text("version")]);
        let version = bus
            .call(asked, Duration::from_secs(5))
            .ok()?
            .body
            .first()
            .and_then(Value::as_u32)?;
        Some(Self { bus, version })
    }

    pub fn open_for(purpose: Purpose) -> Option<Self> {
        Self::open().filter(|portal| portal.can(purpose))
    }

    pub const fn version(&self) -> u32 {
        self.version
    }

    pub const fn can(&self, purpose: Purpose) -> bool {
        match purpose {
            Purpose::AFolder => self.version >= FOLDERS_FROM,
            _ => true,
        }
    }

    pub fn ask(mut self, question: &Question, patience: Duration) -> Answer {
        match self.put(question, patience) {
            Ok(answer) => answer,
            Err(said) => Answer::Broken(said),
        }
    }

    fn put(&mut self, question: &Question, patience: Duration) -> Result<Answer, String> {
        let token = format!(
            "lxb{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let ours = self
            .bus
            .unique_name()
            .trim_start_matches(':')
            .replace('.', "_");
        let expected = format!("{DESKTOP_PATH}/request/{ours}/{token}");
        self.watch(&expected)?;

        let member = if matches!(question.purpose, Purpose::ANewFile) {
            "SaveFile"
        } else {
            "OpenFile"
        };
        let asked = Message::call(DESKTOP, DESKTOP_PATH, FILE_CHOOSER, member).with(vec![
            Value::text(""),
            Value::text(question.heading()),
            options(question, &token),
        ]);
        let reply = self.bus.call(asked, Duration::from_secs(30))?;
        let given = reply
            .body
            .first()
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if !given.is_empty() && given != expected {
            self.watch(&given)?;
        }

        let deadline = Instant::now() + patience;
        let answered = self.bus.wait_for(deadline, |message| {
            message.is_signal(&expected, REQUEST, "Response")
                || (!given.is_empty() && message.is_signal(&given, REQUEST, "Response"))
        });
        match answered {
            Ok(answered) => Ok(read(&answered)),
            Err(said) => {
                self.withdraw(if given.is_empty() { &expected } else { &given });
                Err(said)
            }
        }
    }

    fn withdraw(&mut self, path: &str) {
        let _ = self.bus.call(
            Message::call(DESKTOP, path, REQUEST, "Close"),
            Duration::from_secs(5),
        );
    }

    fn watch(&mut self, path: &str) -> Result<(), String> {
        let rule = format!(
            "type='signal',sender='{DESKTOP}',interface='{REQUEST}',member='Response',path='{path}'"
        );
        self.bus.call(
            Message::call(BUS, BUS_PATH, BUS, "AddMatch").with(vec![Value::text(rule)]),
            Duration::from_secs(5),
        )?;
        Ok(())
    }
}

fn read(answered: &Message) -> Answer {
    match answered.body.first().and_then(Value::as_u32) {
        Some(CHOSE) => {}
        Some(CANCELLED) => return Answer::Cancelled,
        _ => return Answer::Broken("the desktop could not answer the question".to_string()),
    }
    let files: Vec<PathBuf> = answered
        .body
        .get(1)
        .and_then(|results| results.at("uris"))
        .and_then(Value::as_strings)
        .unwrap_or_default()
        .iter()
        .filter_map(|uri| path_of(uri))
        .collect();
    if files.is_empty() {
        return Answer::Cancelled;
    }
    Answer::Chose(files)
}

fn options(question: &Question, token: &str) -> Value {
    let mut entries = vec![
        ("handle_token", Value::text(token)),
        ("modal", Value::Bool(true)),
    ];
    if !question.accept.trim().is_empty() {
        entries.push(("accept_label", Value::text(question.accept.clone())));
    }
    match question.purpose {
        Purpose::ManyFiles => entries.push(("multiple", Value::Bool(true))),
        Purpose::AFolder => entries.push(("directory", Value::Bool(true))),
        Purpose::ANewFile if !question.name.trim().is_empty() => {
            entries.push(("current_name", Value::text(question.name.clone())));
        }
        _ => {}
    }
    if let Some(at) = question.at.as_ref().filter(|at| at.is_dir()) {
        entries.push(("current_folder", Value::bytes(&terminated(at))));
    }
    if question.purpose.lists_files() {
        if let Some(filters) = filters_of(&question.kinds) {
            entries.push(("filters", filters));
        }
    }
    Value::options(entries)
}

fn filters_of(kinds: &[Kind]) -> Option<Value> {
    let named: Vec<Value> = kinds
        .iter()
        .filter(|kind| !kind.patterns.is_empty())
        .map(|kind| {
            Value::Struct(vec![
                Value::text(kind.name.clone()),
                Value::Array(
                    "(us)".to_string(),
                    kind.patterns
                        .iter()
                        .map(|pattern| {
                            Value::Struct(vec![
                                Value::U32(u32::from(pattern.is_mime())),
                                Value::text(pattern.text()),
                            ])
                        })
                        .collect(),
                ),
            ])
        })
        .collect();
    (!named.is_empty()).then(|| Value::Array("(sa(us))".to_string(), named))
}

fn terminated(path: &Path) -> Vec<u8> {
    let mut raw = path.as_os_str().as_bytes().to_vec();
    raw.push(0);
    raw
}

fn path_of(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    if !rest.starts_with('/') {
        return None;
    }
    let raw = uri_decoded(rest);
    (!raw.is_empty()).then(|| PathBuf::from(OsString::from_vec(raw)))
}

fn uri_decoded(text: &str) -> Vec<u8> {
    let raw = text.as_bytes();
    let mut out = Vec::with_capacity(raw.len());
    let mut at = 0;
    while at < raw.len() {
        if raw[at] == b'%' && at + 2 < raw.len() {
            let pair = std::str::from_utf8(&raw[at + 1..at + 3]).unwrap_or("");
            if let Ok(byte) = u8::from_str_radix(pair, 16) {
                out.push(byte);
                at += 3;
                continue;
            }
        }
        out.push(raw[at]);
        at += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use lxb_toolkit::picker::{Pattern, Selection};

    #[test]
    fn a_uri_becomes_the_path_it_names() {
        assert_eq!(
            path_of("file:///home/me/a%20file.png"),
            Some(PathBuf::from("/home/me/a file.png"))
        );
        assert_eq!(
            path_of("file://localhost/tmp/plain.txt"),
            Some(PathBuf::from("/tmp/plain.txt"))
        );
        assert_eq!(path_of("file:///a/%23b"), Some(PathBuf::from("/a/#b")));
        assert_eq!(path_of("https://example.com/a.png"), None);
        assert_eq!(path_of("file://relative"), None);
        assert_eq!(path_of("file:///a%"), Some(PathBuf::from("/a%")));
    }

    #[test]
    fn what_an_application_asks_for_is_spelled_the_portals_way() {
        let question = Question::new(Purpose::ManyFiles)
            .at("/")
            .of(Selection::Image.kind());
        let options = options(&question, "lxb1_0");
        assert_eq!(options.signature(), "a{sv}");
        assert_eq!(
            options.at("handle_token").and_then(Value::as_str),
            Some("lxb1_0")
        );
        assert_eq!(
            options.at("multiple"),
            Some(&Value::Variant(Box::new(Value::Bool(true))))
        );
        assert_eq!(options.at("directory"), None);
        assert_eq!(options.at("current_name"), None);
        assert_eq!(
            options
                .at("current_folder")
                .and_then(Value::as_list)
                .map(<[Value]>::len),
            Some(2)
        );
        assert_eq!(
            options
                .at("filters")
                .map(|filters| filters.peeled().signature()),
            Some("a(sa(us))".to_string())
        );
    }

    #[test]
    fn a_folder_is_never_narrowed_to_a_kind_of_file() {
        let folder = Question::new(Purpose::AFolder).of(Selection::Image.kind());
        assert!(!folder.kinds.is_empty());
        assert_eq!(options(&folder, "t").at("filters"), None);
    }

    #[test]
    fn a_folder_and_a_save_are_asked_for_differently() {
        let folder = options(&Question::new(Purpose::AFolder), "t");
        assert_eq!(
            folder.at("directory"),
            Some(&Value::Variant(Box::new(Value::Bool(true))))
        );
        assert_eq!(folder.at("multiple"), None);

        let save = options(&Question::new(Purpose::ANewFile).called("notes.txt"), "t");
        assert_eq!(
            save.at("current_name").and_then(Value::as_str),
            Some("notes.txt")
        );
        assert_eq!(save.at("directory"), None);

        let unnamed = options(&Question::new(Purpose::ANewFile), "t");
        assert_eq!(unnamed.at("current_name"), None);
    }

    #[test]
    fn a_kind_carries_its_patterns_and_says_which_are_media_types() {
        let kinds = vec![Kind {
            name: "Images".to_string(),
            patterns: vec![
                Pattern::Glob("*.png".to_string()),
                Pattern::Mime("image/jpeg".to_string()),
            ],
        }];
        let Some(Value::Array(element, named)) = filters_of(&kinds) else {
            panic!("a list of kinds");
        };
        assert_eq!(element, "(sa(us))");
        let Some(Value::Struct(fields)) = named.first() else {
            panic!("one kind");
        };
        assert_eq!(fields[0].as_str(), Some("Images"));
        let patterns = fields[1].as_list().expect("its patterns");
        assert_eq!(
            patterns[0],
            Value::Struct(vec![Value::U32(0), Value::Str("*.png".to_string())])
        );
        assert_eq!(
            patterns[1],
            Value::Struct(vec![Value::U32(1), Value::Str("image/jpeg".to_string())])
        );
        assert_eq!(filters_of(&[]), None);
        assert_eq!(
            filters_of(&[Kind {
                name: "Nothing".to_string(),
                patterns: Vec::new()
            }]),
            None
        );
    }

    #[test]
    fn a_folder_that_is_not_there_is_not_asked_for() {
        let question = Question::new(Purpose::OneFile).at("/nowhere/at/all");
        assert!(question.at.is_none());
        assert_eq!(options(&question, "t").at("current_folder"), None);
    }

    #[test]
    fn the_heading_falls_back_to_what_is_being_asked() {
        assert_eq!(Question::new(Purpose::AFolder).heading(), "Choose a folder");
        let mut named = Question::new(Purpose::AFolder);
        named.title = "Where do the saves go".to_string();
        assert_eq!(named.heading(), "Where do the saves go");
    }

    #[test]
    fn what_came_back_is_read_as_files_a_cancel_or_a_failure() {
        let said = |code: u32, uris: Vec<&str>| {
            read(&Message::default().with(vec![
                Value::U32(code),
                Value::options(vec![(
                    "uris",
                    Value::Array("s".to_string(), uris.into_iter().map(Value::text).collect()),
                )]),
            ]))
        };
        assert_eq!(
            said(0, vec!["file:///tmp/one.png", "file:///tmp/two.png"]),
            Answer::Chose(vec![
                PathBuf::from("/tmp/one.png"),
                PathBuf::from("/tmp/two.png")
            ])
        );
        assert_eq!(said(1, vec!["file:///tmp/one.png"]), Answer::Cancelled);
        assert_eq!(said(0, Vec::new()), Answer::Cancelled);
        assert!(matches!(said(2, Vec::new()), Answer::Broken(_)));
        assert!(matches!(read(&Message::default()), Answer::Broken(_)));
    }
}
