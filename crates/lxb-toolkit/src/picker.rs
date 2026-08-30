use std::path::{Path, PathBuf};

pub const MOST: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Selection {
    File = 0,
    Image = 1,
    Scenery = 2,
    Folder = 3,
}

impl Selection {
    pub const ALL: [Self; 4] = [Self::File, Self::Image, Self::Scenery, Self::Folder];

    pub const fn name(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Image => "image",
            Self::Scenery => "scenery",
            Self::Folder => "folder",
        }
    }

    pub fn named(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.name().eq_ignore_ascii_case(name))
    }

    pub const fn chooses_folder(self) -> bool {
        matches!(self, Self::Folder)
    }

    pub const fn supports_search(self) -> bool {
        !self.chooses_folder()
    }

    fn accepts_file(self, path: &Path) -> bool {
        match self {
            Self::File => true,
            Self::Image => image(path),
            Self::Scenery => image(path) || video(path),
            Self::Folder => false,
        }
    }

    pub fn kind(self) -> Option<Kind> {
        let (name, names): (&str, Vec<&str>) = match self {
            Self::File | Self::Folder => return None,
            Self::Image => ("Images", IMAGES.to_vec()),
            Self::Scenery => ("Images and films", [IMAGES, VIDEOS].concat()),
        };
        Some(Kind {
            name: name.to_string(),
            patterns: names
                .into_iter()
                .map(|extension| Pattern::Glob(whatever_the_case(extension)))
                .collect(),
        })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Purpose {
    #[default]
    OneFile = 0,
    ManyFiles = 1,
    AFolder = 2,
    ANewFile = 3,
}

impl Purpose {
    pub const ALL: [Self; 4] = [
        Self::OneFile,
        Self::ManyFiles,
        Self::AFolder,
        Self::ANewFile,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::OneFile => "one-file",
            Self::ManyFiles => "many-files",
            Self::AFolder => "a-folder",
            Self::ANewFile => "a-new-file",
        }
    }

    pub fn named(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.name().eq_ignore_ascii_case(name))
    }

    pub const fn lists_files(self) -> bool {
        !matches!(self, Self::AFolder)
    }

    pub const fn makes_folders(self) -> bool {
        matches!(self, Self::AFolder | Self::ANewFile)
    }

    pub const fn takes_several(self) -> bool {
        matches!(self, Self::ManyFiles)
    }

    pub const fn answers_with_a_head_row(self) -> bool {
        !matches!(self, Self::OneFile)
    }

    pub const fn accept(self) -> Option<&'static str> {
        match self {
            Self::OneFile => None,
            Self::ManyFiles => Some("Open"),
            Self::AFolder => Some("Use this folder"),
            Self::ANewFile => Some("Save here"),
        }
    }

    pub const fn asking(self) -> &'static str {
        match self {
            Self::OneFile => "Choose a file",
            Self::ManyFiles => "Choose some files",
            Self::AFolder => "Choose a folder",
            Self::ANewFile => "Choose somewhere to save",
        }
    }

    pub const fn of(selection: Selection) -> Self {
        if selection.chooses_folder() {
            Self::AFolder
        } else {
            Self::OneFile
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    Glob(String),
    Mime(String),
}

impl Pattern {
    pub fn text(&self) -> &str {
        match self {
            Pattern::Glob(pattern) | Pattern::Mime(pattern) => pattern,
        }
    }

    pub const fn is_mime(&self) -> bool {
        matches!(self, Pattern::Mime(_))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Sort {
    #[default]
    NameAscending = 0,
    NameDescending = 1,
    LargestFirst = 2,
    SmallestFirst = 3,
    Type = 4,
    NewestFirst = 5,
    OldestFirst = 6,
    LastChangedFirst = 7,
    LongestUntouchedFirst = 8,
}

pub const SORTS: [Sort; 9] = [
    Sort::NameAscending,
    Sort::NameDescending,
    Sort::LargestFirst,
    Sort::SmallestFirst,
    Sort::Type,
    Sort::NewestFirst,
    Sort::OldestFirst,
    Sort::LastChangedFirst,
    Sort::LongestUntouchedFirst,
];

impl Sort {
    pub const ALL: [Self; 9] = SORTS;

    pub const fn label(self) -> &'static str {
        match self {
            Self::NameAscending => "Name (A to Z)",
            Self::NameDescending => "Name (Z to A)",
            Self::LargestFirst => "Size (largest first)",
            Self::SmallestFirst => "Size (smallest first)",
            Self::Type => "Type",
            Self::NewestFirst => "Created (newest first)",
            Self::OldestFirst => "Created (oldest first)",
            Self::LastChangedFirst => "Modified (newest first)",
            Self::LongestUntouchedFirst => "Modified (oldest first)",
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::NameAscending => "name",
            Self::NameDescending => "name-reversed",
            Self::LargestFirst => "size-largest-first",
            Self::SmallestFirst => "size-smallest-first",
            Self::Type => "type",
            Self::NewestFirst => "created-newest-first",
            Self::OldestFirst => "created-oldest-first",
            Self::LastChangedFirst => "modified-newest-first",
            Self::LongestUntouchedFirst => "modified-oldest-first",
        }
    }

    pub fn named(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.name().eq_ignore_ascii_case(name))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kind {
    pub name: String,
    pub patterns: Vec<Pattern>,
}

impl Kind {
    pub fn gather(kinds: &mut Vec<Kind>, name: String, pattern: Pattern) {
        if let Some(kind) = kinds.iter_mut().find(|kind| kind.name == name) {
            kind.patterns.push(pattern);
            return;
        }
        kinds.push(Kind {
            name,
            patterns: vec![pattern],
        });
    }
}

fn whatever_the_case(extension: &str) -> String {
    let mut pattern = String::from("*.");
    for character in extension.chars() {
        if character.is_ascii_alphabetic() {
            pattern.push('[');
            pattern.push(character.to_ascii_lowercase());
            pattern.push(character.to_ascii_uppercase());
            pattern.push(']');
        } else {
            pattern.push(character);
        }
    }
    pattern
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum EntryKind {
    Folder = 0,
    File = 1,
}

impl EntryKind {
    pub const ALL: [Self; 2] = [Self::Folder, Self::File];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Folder => "folder",
            Self::File => "file",
        }
    }

    pub fn named(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.name().eq_ignore_ascii_case(name))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub kind: EntryKind,
    pub size: u64,
    pub created: Option<std::time::SystemTime>,
    pub modified: Option<std::time::SystemTime>,
}

impl Entry {
    pub fn is_folder(&self) -> bool {
        self.kind == EntryKind::Folder
    }

    pub fn is_file(&self) -> bool {
        self.kind == EntryKind::File
    }
}

#[derive(Debug, Clone)]
pub struct Picker {
    selection: Selection,
    sort: Sort,
    hidden: bool,
    filtered: bool,
    location: PathBuf,
    entries: Vec<Entry>,
    selected: Option<usize>,
    query: String,
    note: String,
    accessible: bool,
    searchable: bool,
}

impl Picker {
    pub fn new(selection: Selection, directory: impl AsRef<Path>) -> Self {
        let mut picker = Self {
            selection,
            sort: Sort::default(),
            hidden: false,
            filtered: true,
            location: absolute(directory.as_ref()),
            entries: Vec::new(),
            selected: None,
            query: String::new(),
            note: String::new(),
            accessible: false,
            searchable: false,
        };
        picker.refresh();
        picker
    }

    pub const fn selection(&self) -> Selection {
        self.selection
    }

    pub fn location(&self) -> &Path {
        &self.location
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn note(&self) -> &str {
        &self.note
    }

    pub fn can_search(&self) -> bool {
        self.searchable
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn selected_entry(&self) -> Option<&Entry> {
        self.selected.and_then(|index| self.entries.get(index))
    }

    pub const fn sort(&self) -> Sort {
        self.sort
    }

    pub fn sort_by(&mut self, sort: Sort) -> bool {
        if self.sort == sort {
            return false;
        }
        self.sort = sort;
        self.refresh();
        true
    }

    pub const fn hidden_shown(&self) -> bool {
        self.hidden
    }

    pub fn show_hidden(&mut self, shown: bool) -> bool {
        if self.hidden == shown {
            return false;
        }
        self.hidden = shown;
        self.refresh();
        true
    }

    pub const fn kind(&self) -> Option<usize> {
        if self.filtered {
            Some(0)
        } else {
            None
        }
    }

    pub fn kinds(&self) -> Vec<Kind> {
        self.selection.kind().into_iter().collect()
    }

    pub fn showing(&self) -> String {
        match self.kind().and_then(|_| self.selection.kind()) {
            Some(kind) => kind.name,
            None => "Everything".to_string(),
        }
    }

    pub fn show_kind(&mut self, kind: Option<usize>) -> bool {
        let filtered = kind.is_some();
        if self.filtered == filtered {
            return false;
        }
        self.filtered = filtered;
        self.refresh();
        true
    }

    pub const fn accessible(&self) -> bool {
        self.accessible
    }

    pub fn can_choose(&self) -> bool {
        if self.selection.chooses_folder() {
            self.accessible
        } else {
            self.selected_entry().is_some_and(Entry::is_file)
        }
    }

    pub fn select(&mut self, index: usize) -> bool {
        if index >= self.entries.len() || self.selected == Some(index) {
            return false;
        }
        self.selected = Some(index);
        true
    }

    pub fn move_selection(&mut self, delta: isize) -> bool {
        let Some(selected) = self.selected else {
            return false;
        };
        let wanted = selected.saturating_add_signed(delta);
        if wanted >= self.entries.len() {
            return false;
        }
        self.select(wanted)
    }

    pub fn enter(&mut self) -> bool {
        let Some(entry) = self.selected_entry() else {
            return false;
        };
        if !entry.is_folder() {
            return false;
        }
        self.location = entry.path.clone();
        self.query.clear();
        self.refresh();
        true
    }

    pub fn leave(&mut self) -> bool {
        let Some(parent) = self.location.parent() else {
            return false;
        };
        let parent = parent.to_path_buf();
        if parent == self.location {
            return false;
        }
        self.location = parent;
        self.query.clear();
        self.refresh();
        true
    }

    pub fn refresh(&mut self) {
        let showing = if self.filtered {
            self.selection
        } else {
            match self.selection {
                Selection::Folder => Selection::Folder,
                _ => Selection::File,
            }
        };
        let (entries, note, accessible, found) =
            listing(&self.location, showing, &self.query, self.sort, self.hidden);
        self.entries = entries;
        self.note = note;
        self.accessible = accessible;
        self.searchable = self.selection.supports_search() && found != 0;
        self.selected = (!self.entries.is_empty()).then_some(0);
    }

    pub fn search(&mut self, query: impl Into<String>) {
        if !self.can_search() {
            return;
        }
        self.query = query.into();
        self.refresh();
    }

    pub fn make_folder(&mut self, name: &str) -> Result<PathBuf, String> {
        let made = make_folder(&self.location, name)?;
        self.refresh();
        Ok(made)
    }

    pub fn choose(&self) -> Option<PathBuf> {
        if !self.can_choose() {
            return None;
        }
        if self.selection.chooses_folder() {
            return Some(self.location.clone());
        }
        self.selected_entry().map(|entry| entry.path.clone())
    }
}

pub fn writable(directory: &Path) -> bool {
    std::fs::metadata(directory)
        .is_ok_and(|facts| facts.is_dir() && !facts.permissions().readonly())
}

pub fn make_folder(inside: &Path, name: &str) -> Result<PathBuf, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("A folder needs a name".to_string());
    }
    if name == "." || name == ".." || name.contains('/') || name.contains('\0') {
        return Err("That is not a name a folder can have".to_string());
    }
    let made = inside.join(name);
    if made.exists() {
        return Err("Something here is called that already".to_string());
    }
    std::fs::create_dir(&made).map_err(|err| format!("The folder could not be made: {err}"))?;
    Ok(made)
}

fn absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    std::env::current_dir()
        .map(|current| current.join(path))
        .unwrap_or_else(|_| path.to_path_buf())
}

fn listing(
    directory: &Path,
    selection: Selection,
    query: &str,
    sort: Sort,
    hidden: bool,
) -> (Vec<Entry>, String, bool, usize) {
    let Ok(reading) = std::fs::read_dir(directory) else {
        return (Vec::new(), "This cannot be opened".to_string(), false, 0);
    };

    let mut folders = 0usize;
    let mut files = 0usize;
    let mut left_out = 0usize;
    let mut entries = Vec::new();

    for found in reading.flatten() {
        let name = found.file_name();
        let Some(name) = name.to_str() else {
            left_out += 1;
            continue;
        };
        if name.starts_with('.') && !hidden {
            continue;
        }

        if folders + files >= MOST {
            left_out += 1;
            continue;
        }

        let path = found.path();
        let folder = found
            .file_type()
            .map(|kind| {
                if kind.is_symlink() {
                    std::fs::metadata(&path)
                        .map(|facts| facts.is_dir())
                        .unwrap_or(false)
                } else {
                    kind.is_dir()
                }
            })
            .unwrap_or(false);

        if !folder && !selection.accepts_file(&path) {
            continue;
        }
        if folder {
            folders += 1;
        } else {
            files += 1;
        }
        if !matched(name, query) {
            continue;
        }

        let facts = std::fs::metadata(&path).ok();
        entries.push(Entry {
            name: name.to_string(),
            kind: if folder {
                EntryKind::Folder
            } else {
                EntryKind::File
            },
            size: facts.as_ref().map(std::fs::Metadata::len).unwrap_or(0),
            created: facts.as_ref().and_then(|facts| facts.created().ok()),
            modified: facts.as_ref().and_then(|facts| facts.modified().ok()),
            path,
        });
    }

    order(&mut entries, sort);
    (
        entries,
        note(folders, files, left_out),
        true,
        folders + files + left_out,
    )
}

fn order(entries: &mut [Entry], sort: Sort) {
    let by_name = |left: &Entry, right: &Entry| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.name.cmp(&right.name))
    };
    entries.sort_by(|left, right| {
        let first = left.kind.cmp(&right.kind);
        if first != std::cmp::Ordering::Equal {
            return first;
        }
        match sort {
            Sort::NameAscending => by_name(left, right),
            Sort::NameDescending => by_name(right, left),
            Sort::LargestFirst => right
                .size
                .cmp(&left.size)
                .then_with(|| by_name(left, right)),
            Sort::SmallestFirst => left
                .size
                .cmp(&right.size)
                .then_with(|| by_name(left, right)),
            Sort::Type => extension(&left.path)
                .unwrap_or_default()
                .cmp(&extension(&right.path).unwrap_or_default())
                .then_with(|| by_name(left, right)),
            Sort::NewestFirst => right
                .created
                .cmp(&left.created)
                .then_with(|| by_name(left, right)),
            Sort::OldestFirst => left
                .created
                .cmp(&right.created)
                .then_with(|| by_name(left, right)),
            Sort::LastChangedFirst => right
                .modified
                .cmp(&left.modified)
                .then_with(|| by_name(left, right)),
            Sort::LongestUntouchedFirst => left
                .modified
                .cmp(&right.modified)
                .then_with(|| by_name(left, right)),
        }
    });
}

fn matched(name: &str, query: &str) -> bool {
    query.is_empty() || name.to_lowercase().contains(&query.to_lowercase())
}

fn note(folders: usize, files: usize, left_out: usize) -> String {
    let plural = |count: usize, one: &str, many: &str| {
        if count == 1 {
            format!("{count} {one}")
        } else {
            format!("{count} {many}")
        }
    };
    let counted = match (folders, files) {
        (0, 0) => "Empty".to_string(),
        (0, files) => plural(files, "file", "files"),
        (folders, 0) => plural(folders, "folder", "folders"),
        (folders, files) => format!(
            "{}, {}",
            plural(folders, "folder", "folders"),
            plural(files, "file", "files")
        ),
    };
    match left_out {
        0 => counted,
        left_out => format!("{counted}, {left_out} more not shown"),
    }
}

const IMAGES: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "webp", "avif", "jxl", "heic", "heif", "bmp", "tif", "tiff",
    "svg", "psd", "dng", "cr2", "cr3", "nef", "arw", "orf", "raf", "rw2",
];

const VIDEOS: &[&str] = &[
    "mp4", "m4v", "mkv", "webm", "avi", "divx", "mov", "wmv", "flv", "mpg", "mpeg", "vob", "ogv",
    "3gp", "m2ts", "rmvb",
];

fn image(path: &Path) -> bool {
    extension(path).is_some_and(|found| IMAGES.contains(&found.as_str()))
}

fn video(path: &Path) -> bool {
    extension(path).is_some_and(|found| VIDEOS.contains(&found.as_str()))
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let number = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("lxb-picker-{name}-{}-{number}", std::process::id()));
            std::fs::create_dir_all(&path).expect("a picker test directory");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn names(picker: &Picker) -> Vec<(&str, EntryKind)> {
        picker
            .entries()
            .iter()
            .map(|entry| (entry.name.as_str(), entry.kind))
            .collect()
    }

    #[test]
    fn the_listing_hides_dotfiles_and_puts_folders_before_files() {
        let directory = Scratch::new("order");
        for name in ["zebra", "Ada", ".hidden"] {
            std::fs::create_dir(directory.path().join(name)).unwrap();
        }
        for name in ["b.txt", "A.txt", ".secret"] {
            std::fs::write(directory.path().join(name), b"x").unwrap();
        }

        let picker = Picker::new(Selection::File, directory.path());
        assert_eq!(
            names(&picker),
            [
                ("Ada", EntryKind::Folder),
                ("zebra", EntryKind::Folder),
                ("A.txt", EntryKind::File),
                ("b.txt", EntryKind::File),
            ]
        );
        assert_eq!(picker.note(), "2 folders, 2 files");
        assert_eq!(picker.selected(), Some(0));
        assert!(picker.can_search());
    }

    #[test]
    fn a_folder_picker_only_lists_folders_and_answers_with_the_current_one() {
        let directory = Scratch::new("folder");
        std::fs::create_dir(directory.path().join("inside")).unwrap();
        std::fs::write(directory.path().join("note.txt"), b"x").unwrap();

        let mut picker = Picker::new(Selection::Folder, directory.path());
        assert_eq!(names(&picker), [("inside", EntryKind::Folder)]);
        assert_eq!(picker.choose().as_deref(), Some(directory.path()));
        assert!(picker.can_choose());
        assert!(!picker.selection().supports_search());
        picker.search("inside");
        assert!(picker.query().is_empty());
        assert_eq!(names(&picker), [("inside", EntryKind::Folder)]);
        assert!(picker.enter());
        assert_eq!(
            picker.choose().as_deref(),
            Some(directory.path().join("inside").as_path())
        );
        assert!(!picker.enter(), "there is nothing below an empty folder");
    }

    #[test]
    fn a_file_picker_opens_folders_but_only_chooses_files() {
        let directory = Scratch::new("file");
        std::fs::create_dir(directory.path().join("inside")).unwrap();
        std::fs::write(directory.path().join("note.txt"), b"x").unwrap();
        std::fs::write(directory.path().join("inside").join("picked.txt"), b"x").unwrap();

        let mut picker = Picker::new(Selection::File, directory.path());
        assert_eq!(picker.choose(), None, "a folder is a way in, not an answer");
        assert!(picker.enter());
        assert_eq!(
            picker.choose().as_deref(),
            Some(directory.path().join("inside/picked.txt").as_path())
        );
        assert!(picker.leave());
        assert_eq!(picker.location(), directory.path());
    }

    #[test]
    fn image_and_scenery_walks_keep_their_own_file_filters() {
        let directory = Scratch::new("filters");
        std::fs::create_dir(directory.path().join("Albums")).unwrap();
        for name in ["cover.png", "holiday.mp4", "notes.txt", "song.flac"] {
            std::fs::write(directory.path().join(name), b"x").unwrap();
        }

        let image = Picker::new(Selection::Image, directory.path());
        assert_eq!(
            names(&image),
            [
                ("Albums", EntryKind::Folder),
                ("cover.png", EntryKind::File)
            ]
        );
        let scenery = Picker::new(Selection::Scenery, directory.path());
        assert_eq!(
            names(&scenery),
            [
                ("Albums", EntryKind::Folder),
                ("cover.png", EntryKind::File),
                ("holiday.mp4", EntryKind::File),
            ]
        );
    }

    #[test]
    fn searching_rereads_only_this_folder_and_navigation_clears_it() {
        let directory = Scratch::new("search");
        std::fs::create_dir(directory.path().join("inside")).unwrap();
        for name in ["alpha.txt", "beta.txt"] {
            std::fs::write(directory.path().join(name), b"x").unwrap();
        }

        let mut picker = Picker::new(Selection::File, directory.path());
        picker.search("ALP");
        assert_eq!(picker.query(), "ALP");
        assert_eq!(names(&picker), [("alpha.txt", EntryKind::File)]);
        assert!(picker.can_search());
        picker.search("not a file");
        assert!(picker.entries().is_empty());
        assert!(
            picker.can_search(),
            "a zero-match query still needs clearing"
        );
        picker.search("inside");
        assert!(picker.enter());
        assert!(picker.query().is_empty());
        assert!(picker.leave());
        assert!(picker.query().is_empty());
        assert_eq!(picker.entries().len(), 3);
    }

    #[test]
    fn an_unreadable_directory_is_not_reported_as_empty() {
        let picker = Picker::new(Selection::File, "/proc/1/fdinfo/nothing-here");
        assert!(picker.entries().is_empty());
        assert_eq!(picker.note(), "This cannot be opened");
        assert!(!picker.can_choose());
        assert!(!picker.can_search());

        let folders = Picker::new(Selection::Folder, "/proc/1/fdinfo/nothing-here");
        assert_eq!(folders.choose(), None);
    }

    #[cfg(unix)]
    #[test]
    fn a_link_to_a_directory_is_walked_as_a_folder() {
        use std::os::unix::fs::symlink;

        let directory = Scratch::new("symlink");
        std::fs::create_dir(directory.path().join("real")).unwrap();
        symlink(
            directory.path().join("real"),
            directory.path().join("linked"),
        )
        .unwrap();

        let mut picker = Picker::new(Selection::File, directory.path());
        assert_eq!(
            names(&picker),
            [("linked", EntryKind::Folder), ("real", EntryKind::Folder)]
        );
        assert!(picker.enter());
        assert_eq!(picker.location(), directory.path().join("linked"));
    }

    #[test]
    fn a_folder_is_made_only_where_the_name_is_one() {
        let directory = Scratch::new("make");
        std::fs::write(directory.path().join("taken"), b"x").unwrap();

        let mut picker = Picker::new(Selection::File, directory.path());
        assert_eq!(
            picker.make_folder("Holiday").as_deref(),
            Ok(directory.path().join("Holiday").as_path())
        );
        assert!(directory.path().join("Holiday").is_dir());
        assert!(names(&picker).contains(&("Holiday", EntryKind::Folder)));

        for refused in ["", "   ", ".", "..", "a/b", "taken", "Holiday"] {
            assert!(
                picker.make_folder(refused).is_err(),
                "a folder was made called {refused:?}"
            );
        }
        assert!(writable(directory.path()));
        assert!(!writable(&directory.path().join("nothing-here")));
    }

    #[test]
    fn a_purpose_says_what_the_column_offers() {
        assert_eq!(Purpose::of(Selection::Folder), Purpose::AFolder);
        assert_eq!(Purpose::of(Selection::Image), Purpose::OneFile);
        assert_eq!(Purpose::named("MANY-FILES"), Some(Purpose::ManyFiles));
        assert!(!Purpose::AFolder.lists_files());
        assert!(Purpose::ANewFile.lists_files());
        assert!(Purpose::ANewFile.makes_folders());
        assert!(!Purpose::ManyFiles.makes_folders());
        assert!(Purpose::ManyFiles.takes_several());
        assert_eq!(Purpose::OneFile.accept(), None);
        assert_eq!(Purpose::ANewFile.accept(), Some("Save here"));
        assert!(!Purpose::OneFile.answers_with_a_head_row());
        assert!(Purpose::AFolder.answers_with_a_head_row());
    }

    #[test]
    fn a_selection_becomes_the_kind_of_file_a_portal_understands() {
        assert_eq!(Selection::File.kind(), None);
        assert_eq!(Selection::Folder.kind(), None);
        let images = Selection::Image.kind().expect("images are a kind");
        assert_eq!(images.name, "Images");
        assert_eq!(images.patterns.len(), IMAGES.len());
        assert!(images
            .patterns
            .contains(&Pattern::Glob("*.[pP][nN][gG]".to_string())));
        assert!(images
            .patterns
            .contains(&Pattern::Glob("*.[cC][rR]2".to_string())));
        let scenery = Selection::Scenery.kind().expect("scenery is a kind");
        assert_eq!(scenery.name, "Images and films");
        assert_eq!(scenery.patterns.len(), IMAGES.len() + VIDEOS.len());
        assert!(!scenery.patterns[0].is_mime());
        assert_eq!(scenery.patterns[0].text(), "*.[jJ][pP][gG]");
    }

    #[test]
    fn kinds_gather_every_pattern_under_the_name_it_was_given() {
        let mut kinds = Vec::new();
        Kind::gather(
            &mut kinds,
            "Images".to_string(),
            Pattern::Glob("*.png".into()),
        );
        Kind::gather(
            &mut kinds,
            "Images".to_string(),
            Pattern::Mime("image/jpeg".into()),
        );
        Kind::gather(
            &mut kinds,
            "Films".to_string(),
            Pattern::Glob("*.mkv".into()),
        );
        assert_eq!(kinds.len(), 2);
        assert_eq!(kinds[0].name, "Images");
        assert_eq!(kinds[0].patterns.len(), 2);
        assert_eq!(kinds[1].name, "Films");
    }

    #[test]
    fn the_hidden_names_are_shown_only_when_they_are_asked_for() {
        let directory = Scratch::new("hidden");
        std::fs::write(directory.path().join("plain.txt"), b"x").unwrap();
        std::fs::write(directory.path().join(".secret"), b"x").unwrap();

        let mut picker = Picker::new(Selection::File, directory.path());
        assert_eq!(names(&picker), [("plain.txt", EntryKind::File)]);
        assert!(!picker.hidden_shown());
        assert!(picker.show_hidden(true));
        assert!(picker.hidden_shown());
        assert_eq!(
            names(&picker),
            [(".secret", EntryKind::File), ("plain.txt", EntryKind::File)]
        );
        assert!(!picker.show_hidden(true), "asking twice changes nothing");
        assert!(picker.show_hidden(false));
        assert_eq!(names(&picker), [("plain.txt", EntryKind::File)]);
    }

    #[test]
    fn the_kind_in_force_can_be_got_past() {
        let directory = Scratch::new("kinds");
        std::fs::write(directory.path().join("cover.png"), b"x").unwrap();
        std::fs::write(directory.path().join("notes.txt"), b"x").unwrap();

        let mut picker = Picker::new(Selection::Image, directory.path());
        assert_eq!(picker.kind(), Some(0));
        assert_eq!(picker.showing(), "Images");
        assert_eq!(names(&picker), [("cover.png", EntryKind::File)]);

        assert!(picker.show_kind(None), "everything on the disk");
        assert_eq!(picker.kind(), None);
        assert_eq!(picker.showing(), "Everything");
        assert_eq!(
            names(&picker),
            [
                ("cover.png", EntryKind::File),
                ("notes.txt", EntryKind::File)
            ]
        );
        assert!(!picker.show_kind(None));
        assert!(picker.show_kind(Some(0)));
        assert_eq!(names(&picker), [("cover.png", EntryKind::File)]);

        let mut folders = Picker::new(Selection::Folder, directory.path());
        assert!(folders.kinds().is_empty());
        assert!(folders.show_kind(None));
        assert!(names(&folders).is_empty(), "still only folders");
    }

    #[test]
    fn every_order_puts_the_folders_first_and_then_answers_its_own_question() {
        let directory = Scratch::new("sort");
        std::fs::create_dir(directory.path().join("Zulu")).unwrap();
        std::fs::write(directory.path().join("big.bin"), vec![0u8; 4096]).unwrap();
        std::fs::write(directory.path().join("small.aaa"), b"x").unwrap();

        let mut picker = Picker::new(Selection::File, directory.path());
        assert_eq!(picker.sort(), Sort::NameAscending);
        assert_eq!(
            names(&picker),
            [
                ("Zulu", EntryKind::Folder),
                ("big.bin", EntryKind::File),
                ("small.aaa", EntryKind::File)
            ]
        );

        assert!(picker.sort_by(Sort::NameDescending));
        assert_eq!(
            names(&picker),
            [
                ("Zulu", EntryKind::Folder),
                ("small.aaa", EntryKind::File),
                ("big.bin", EntryKind::File)
            ]
        );

        assert!(picker.sort_by(Sort::LargestFirst));
        assert_eq!(names(&picker)[0], ("Zulu", EntryKind::Folder));
        assert_eq!(names(&picker)[1], ("big.bin", EntryKind::File));

        assert!(picker.sort_by(Sort::SmallestFirst));
        assert_eq!(names(&picker)[1], ("small.aaa", EntryKind::File));

        assert!(picker.sort_by(Sort::Type));
        assert_eq!(
            names(&picker)[1],
            ("small.aaa", EntryKind::File),
            "aaa sorts before bin"
        );

        assert!(!picker.sort_by(Sort::Type), "asking twice changes nothing");
        for sort in Sort::ALL {
            picker.sort_by(sort);
            assert_eq!(
                names(&picker)[0],
                ("Zulu", EntryKind::Folder),
                "{} let a file above a folder",
                sort.label()
            );
        }
        assert_eq!(Sort::named("SIZE-LARGEST-FIRST"), Some(Sort::LargestFirst));
        assert_eq!(Sort::named("nothing"), None);
    }

    #[test]
    fn names_and_count_notes_cover_the_public_values() {
        assert_eq!(Selection::named("SCENERY"), Some(Selection::Scenery));
        assert_eq!(EntryKind::named("folder"), Some(EntryKind::Folder));
        assert_eq!(note(0, 0, 0), "Empty");
        assert_eq!(note(1, 0, 0), "1 folder");
        assert_eq!(note(0, 1, 0), "1 file");
        assert_eq!(note(2, 3, 4), "2 folders, 3 files, 4 more not shown");
    }
}
