use std::ffi::OsString;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const USAGE: &str = "usage: lxb-new [--language rust|c|python] [--toolkit-path PATH] NAME [DIR]\n\nNAME is a lowercase slug, for example: hello-lxb\n--language chooses what the project is written in; the default is rust.\n--toolkit-path points at a local lxb-toolkit crate checkout.\nWithout it, an installed toolkit is used if one is found; otherwise the\ngenerated project asks the package registry for lxb-toolkit by version.\n\n      --version   print the generator's version\n  -h, --help      print this message";

const INSTALLED_CRATES: &str = "share/lxb-toolkit/crates";

const CRATES: [&str; 6] = [
    "lxb-toolkit",
    "lxb-render",
    "lxb-input",
    "lxb-sound",
    "lxb-app",
    "lxb-portal",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    Rust,
    C,
    Python,
}

impl Language {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::C => "c",
            Self::Python => "python",
        }
    }

    fn named(name: &str) -> Option<Self> {
        [Self::Rust, Self::C, Self::Python]
            .into_iter()
            .find(|language| language.name() == name)
    }

    fn next_step(self) -> &'static str {
        match self {
            Self::Rust => "cargo run",
            Self::C => "make && ./",
            Self::Python => "python3 main.py",
        }
    }
}

struct Template {
    output: &'static str,
    source: &'static str,
}

const SHARED: &[Template] = &[
    Template {
        output: "LICENSE",
        source: include_str!("../templates/shared/LICENSE"),
    },
    Template {
        output: "assets/app.svg",
        source: include_str!("../templates/shared/assets/app.svg"),
    },
];

const RUST_TEMPLATES: &[Template] = &[
    Template {
        output: ".gitignore",
        source: include_str!("../templates/rust-app/.gitignore"),
    },
    Template {
        output: "Cargo.toml",
        source: include_str!("../templates/rust-app/Cargo.toml"),
    },
    Template {
        output: "README.md",
        source: include_str!("../templates/rust-app/README.md"),
    },
    Template {
        output: "src/main.rs",
        source: include_str!("../templates/rust-app/src/main.rs"),
    },
];

const C_TEMPLATES: &[Template] = &[
    Template {
        output: ".gitignore",
        source: include_str!("../templates/c-app/.gitignore"),
    },
    Template {
        output: "Makefile",
        source: include_str!("../templates/c-app/Makefile"),
    },
    Template {
        output: "README.md",
        source: include_str!("../templates/c-app/README.md"),
    },
    Template {
        output: "src/main.c",
        source: include_str!("../templates/c-app/src/main.c"),
    },
];

const PYTHON_TEMPLATES: &[Template] = &[
    Template {
        output: ".gitignore",
        source: include_str!("../templates/python-app/.gitignore"),
    },
    Template {
        output: "README.md",
        source: include_str!("../templates/python-app/README.md"),
    },
    Template {
        output: "main.py",
        source: include_str!("../templates/python-app/main.py"),
    },
];

fn templates(language: Language) -> &'static [Template] {
    match language {
        Language::Rust => RUST_TEMPLATES,
        Language::C => C_TEMPLATES,
        Language::Python => PYTHON_TEMPLATES,
    }
}

const README_BODY: &str = include_str!("../templates/shared/README-body.md");

#[derive(Debug)]
pub struct Error(String);

impl Error {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

pub fn run<I>(args: I) -> Result<PathBuf, Error>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter();
    let mut positional = Vec::new();
    let mut toolkit_path = None;
    let mut language = Language::default();
    while let Some(argument) = args.next() {
        if argument == "--language" {
            let asked = args
                .next()
                .ok_or_else(|| Error::new("--language needs rust, c or python"))?;
            language = asked.to_str().and_then(Language::named).ok_or_else(|| {
                Error::new(format!(
                    "no such language: {};\n{USAGE}",
                    asked.to_string_lossy()
                ))
            })?;
        } else if argument == "--toolkit-path" {
            if toolkit_path.is_some() {
                return Err(Error::new("--toolkit-path may only be passed once"));
            }
            toolkit_path = Some(PathBuf::from(
                args.next()
                    .ok_or_else(|| Error::new("--toolkit-path needs a PATH"))?,
            ));
        } else if argument == "--version" {
            println!("lxb-new {}", env!("CARGO_PKG_VERSION"));
            return Ok(PathBuf::new());
        } else if argument == "-h" || argument == "--help" {
            println!("{USAGE}");
            return Ok(PathBuf::new());
        } else if argument.to_string_lossy().starts_with('-') {
            return Err(Error::new(format!(
                "unknown option {};\n{USAGE}",
                argument.to_string_lossy()
            )));
        } else {
            positional.push(argument);
        }
    }

    let name = positional
        .first()
        .ok_or_else(|| Error::new(USAGE))?
        .clone()
        .into_string()
        .map_err(|_| Error::new("NAME must be valid UTF-8"))?;
    let destination = match positional.get(1) {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(&name),
    };
    if positional.len() > 2 {
        return Err(Error::new(USAGE));
    }

    let chosen = match toolkit_path {
        Some(path) => Some(path),
        None => installed_toolkit_path(),
    };
    scaffold_with_toolkit(&name, &destination, chosen.as_deref(), language)?;
    println!(
        "Created {} in {}",
        display_title(&name),
        destination.display()
    );
    match chosen.as_deref() {
        Some(path) => println!("Against the toolkit at {}", path.display()),
        None => println!(
            "Against lxb-toolkit {} from the package registry",
            env!("CARGO_PKG_VERSION")
        ),
    }
    println!("Next steps:");
    println!("  cd {}", shell_quote(&destination));
    match language {
        Language::C => println!("  make && ./{name}"),
        other => println!("  {}", other.next_step()),
    }
    Ok(destination)
}

pub fn scaffold(slug: &str, destination: &Path) -> Result<(), Error> {
    scaffold_with_toolkit(slug, destination, None, Language::default())
}

pub fn scaffold_with_toolkit(
    slug: &str,
    destination: &Path,
    toolkit_path: Option<&Path>,
    language: Language,
) -> Result<(), Error> {
    validate_slug(slug)?;
    validate_app_id(slug)?;
    let dependencies = dependencies(toolkit_path)?;
    if destination.as_os_str().is_empty() {
        return Err(Error::new("DIR must not be empty"));
    }
    if fs::symlink_metadata(destination).is_ok() {
        return Err(Error::new(format!(
            "{} already exists; refusing to overwrite it",
            destination.display()
        )));
    }

    if let Some(parent) = destination.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|err| Error::new(format!("cannot create {}: {err}", parent.display())))?;
        }
    }
    fs::create_dir(destination).map_err(|err| {
        Error::new(format!(
            "cannot create {} without overwriting anything: {err}",
            destination.display()
        ))
    })?;
    let title = display_title(slug);

    for template in templates(language).iter().chain(SHARED) {
        let path = destination.join(template.output);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| Error::new(format!("cannot create {}: {err}", parent.display())))?;
        }
        write_new(&path, &render(template.source, slug, &title, &dependencies))?;
    }
    write_new(
        &destination.join(format!("{slug}.desktop")),
        &render(
            include_str!("../templates/shared/app.desktop"),
            slug,
            &title,
            &dependencies,
        ),
    )?;
    Ok(())
}

fn write_new(path: &Path, contents: &str) -> Result<(), Error> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|err| Error::new(format!("refusing to overwrite {}: {err}", path.display())))?;
    file.write_all(contents.as_bytes())
        .map_err(|err| Error::new(format!("cannot write {}: {err}", path.display())))
}

fn render(source: &str, slug: &str, title: &str, dependencies: &[String; CRATES.len()]) -> String {
    source
        .replace("{{BODY}}", README_BODY.trim_end())
        .replace("{{SLUG}}", slug)
        .replace("{{APP_ID}}", slug)
        .replace("{{TITLE}}", title)
        .replace("{{TOOLKIT_DEPENDENCY}}", &dependencies[0])
        .replace("{{RENDER_DEPENDENCY}}", &dependencies[1])
        .replace(
            "{{INPUT_DEPENDENCY}}",
            &with_feature(&dependencies[2], "winit"),
        )
        .replace("{{SOUND_DEPENDENCY}}", &dependencies[3])
        .replace("{{APP_DEPENDENCY}}", &dependencies[4])
}

fn installed_toolkit_path() -> Option<PathBuf> {
    installed_toolkit_path_from(
        std::env::var_os("LXB_TOOLKIT_PATH"),
        option_env!("LXB_TOOLKIT_CRATE_DIR").map(Path::new),
        std::env::current_exe().ok().as_deref(),
    )
}

fn installed_toolkit_path_from(
    from_environment: Option<OsString>,
    baked: Option<&Path>,
    executable: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(asked) = from_environment {
        if !asked.is_empty() {
            return Some(PathBuf::from(asked));
        }
    }

    if let Some(baked) = baked {
        if baked.join("Cargo.toml").is_file() {
            return Some(baked.to_path_buf());
        }
    }

    let prefix = executable?.parent()?.parent()?;
    let path = prefix.join(INSTALLED_CRATES).join(CRATES[0]);
    path.join("Cargo.toml").is_file().then_some(path)
}

fn dependencies(local: Option<&Path>) -> Result<[String; CRATES.len()], Error> {
    let Some(local) = local else {
        let version = env!("CARGO_PKG_VERSION");
        return Ok(CRATES.map(|_| format!("\"{version}\"")));
    };

    let toolkit = local.canonicalize().map_err(|err| {
        Error::new(format!(
            "cannot resolve toolkit path {}: {err}",
            local.display()
        ))
    })?;
    if !toolkit.join("Cargo.toml").is_file() {
        return Err(Error::new(format!(
            "toolkit path {} does not contain Cargo.toml",
            toolkit.display()
        )));
    }

    let mut out = CRATES.map(|_| String::new());
    for (index, crate_name) in CRATES.iter().enumerate() {
        let path = if index == 0 {
            toolkit.clone()
        } else {
            let beside = toolkit
                .parent()
                .map(|parent| parent.join(crate_name))
                .unwrap_or_default();
            if !beside.join("Cargo.toml").is_file() {
                return Err(Error::new(format!(
                    "{crate_name} is not beside the toolkit at {}: a project needs both",
                    toolkit.display()
                )));
            }
            beside
        };
        out[index] = path_dependency(&path)?;
    }
    Ok(out)
}

fn with_feature(dependency: &str, feature: &str) -> String {
    let inner = match dependency.strip_prefix('{') {
        Some(table) => table.trim().trim_end_matches('}').trim_end().to_string(),
        None => format!("version = {dependency}"),
    };
    format!("{{ {inner}, features = [\"{feature}\"] }}")
}

fn path_dependency(path: &Path) -> Result<String, Error> {
    let text = path
        .to_str()
        .ok_or_else(|| Error::new(format!("path {} is not valid UTF-8", path.display())))?;
    if text.chars().any(char::is_control) {
        return Err(Error::new(
            "path contains a control character Cargo.toml cannot safely represent",
        ));
    }
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
    Ok(format!(
        "{{ version = \"{}\", path = \"{escaped}\" }}",
        env!("CARGO_PKG_VERSION")
    ))
}

fn shell_quote(path: &Path) -> String {
    let path = path.to_string_lossy();
    format!("'{}'", path.replace('\'', "'\\''"))
}

pub fn validate_slug(slug: &str) -> Result<(), Error> {
    if slug.is_empty() || slug.len() > 64 {
        return Err(Error::new("NAME must contain between 1 and 64 bytes"));
    }
    let bytes = slug.as_bytes();
    if !bytes[0].is_ascii_lowercase() {
        return Err(Error::new("NAME must begin with a lowercase ASCII letter"));
    }
    if bytes.last() == Some(&b'-') || bytes.windows(2).any(|pair| pair == b"--") {
        return Err(Error::new(
            "NAME must not end with '-' or contain consecutive '-' characters",
        ));
    }
    if !bytes
        .iter()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
    {
        return Err(Error::new(
            "NAME may contain only lowercase ASCII letters, digits, and '-'",
        ));
    }
    Ok(())
}

fn validate_app_id(app_id: &str) -> Result<(), Error> {
    if app_id.contains('/') || app_id.contains('\\') || app_id.starts_with('.') {
        return Err(Error::new(
            "the generated app_id is not a safe desktop-file ID",
        ));
    }
    Ok(())
}

fn display_title(slug: &str) -> String {
    slug.split('-')
        .map(|word| {
            if word == "lxb" {
                return "LXB".to_string();
            }
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("lxb-new-test-{}-{id}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn slug_is_a_safe_package_and_desktop_id() {
        for valid in ["hello", "hello-lxb", "app2"] {
            validate_slug(valid).unwrap();
            validate_app_id(valid).unwrap();
        }
        for invalid in [
            "",
            "Hello",
            "2fast",
            "two words",
            "two--words",
            "trailing-",
            "../escape",
            "hello.world",
        ] {
            assert!(validate_slug(invalid).is_err(), "accepted {invalid:?}");
        }
    }

    #[test]
    fn generated_identity_matches_every_integration_point() {
        let scratch = Scratch::new();
        let destination = scratch.0.join("hello-lxb");
        scaffold("hello-lxb", &destination).unwrap();

        let manifest = fs::read_to_string(destination.join("Cargo.toml")).unwrap();
        let source = fs::read_to_string(destination.join("src/main.rs")).unwrap();
        let desktop = fs::read_to_string(destination.join("hello-lxb.desktop")).unwrap();
        assert!(manifest.contains("name = \"hello-lxb\""));

        assert!(manifest.contains(&format!("lxb-app = \"{}\"", env!("CARGO_PKG_VERSION"))));
        assert!(source.contains("App::new(\"hello-lxb\", \"Hello LXB\")"));
        assert!(desktop.contains("Name=Hello LXB\n"));
        assert!(desktop.contains("Exec=hello-lxb\n"));
        assert!(desktop.contains("Icon=hello-lxb\n"));
        assert!(desktop.contains("StartupWMClass=hello-lxb\n"));
        assert!(desktop.contains("OnlyShowIn=LineXinBar;\n"));
        assert!(!source.contains("lxb_shell_v1"));

        assert!(source.contains("page.head("));
        assert!(source.contains("page.text("));
        assert!(source.contains("page.button("));

        for plumbing in [
            "GLASS_WGSL",
            "WALLPAPER_WGSL",
            "GLYPH_WGSL",
            "create_render_pipeline",
            "distance_field",
            "BindGroupLayout",
            "EventLoop",
            "wgpu::",
            "winit::",
            "Controls::new",
            "Sounds::new",
        ] {
            assert!(
                !source.contains(plumbing),
                "a generated application still owns {plumbing}"
            );
        }
        assert!(!manifest.contains("{{"));
    }

    #[test]
    fn an_existing_destination_is_never_touched() {
        let scratch = Scratch::new();
        let destination = scratch.0.join("kept");
        fs::create_dir(&destination).unwrap();
        fs::write(destination.join("mine"), "untouched").unwrap();

        let err = scaffold("kept", &destination).unwrap_err().to_string();
        assert!(err.contains("refusing to overwrite"));
        assert_eq!(
            fs::read_to_string(destination.join("mine")).unwrap(),
            "untouched"
        );
        assert_eq!(fs::read_dir(&destination).unwrap().count(), 1);
    }

    #[test]
    fn all_embedded_templates_are_present_and_fully_rendered() {
        let scratch = Scratch::new();
        let destination = scratch.0.join("sample");
        scaffold("sample", &destination).unwrap();
        for relative in [
            ".gitignore",
            "Cargo.toml",
            "LICENSE",
            "README.md",
            "sample.desktop",
            "assets/app.svg",
            "src/main.rs",
        ] {
            let contents = fs::read_to_string(destination.join(relative)).unwrap();
            assert!(!contents.contains("{{"), "unrendered {relative}");
        }
    }

    #[test]
    fn an_explicit_local_toolkit_is_versioned_and_buildable() {
        let scratch = Scratch::new();
        let destination = scratch.0.join("local-app");
        let toolkit = Path::new(env!("CARGO_MANIFEST_DIR")).join("../lxb-toolkit");
        scaffold_with_toolkit("local-app", &destination, Some(&toolkit), Language::Rust).unwrap();

        let canonical = toolkit.canonicalize().unwrap();
        let beside = canonical.parent().unwrap();
        let manifest = fs::read_to_string(destination.join("Cargo.toml")).unwrap();
        assert!(
            manifest.contains(&format!(
                "lxb-app = {{ version = \"{}\", path = \"{}\" }}",
                env!("CARGO_PKG_VERSION"),
                beside.join("lxb-app").display()
            )),
            "lxb-app is missing from the generated manifest:\n{manifest}"
        );
        for named_separately in ["lxb-render", "lxb-input", "lxb-sound"] {
            assert!(
                !manifest.contains(&format!("{named_separately} =")),
                "{named_separately} is named separately; it comes with lxb-app"
            );
        }
        assert!(
            !manifest.contains(".."),
            "a spread is Rust syntax, not TOML"
        );
    }

    #[test]
    fn each_language_gets_a_project_its_own_tools_can_build() {
        let scratch = Scratch::new();
        for (language, wanted, unwanted) in [
            (
                Language::Rust,
                ["Cargo.toml", "src/main.rs"],
                ["Makefile", "main.py"],
            ),
            (
                Language::C,
                ["Makefile", "src/main.c"],
                ["Cargo.toml", "main.py"],
            ),
            (
                Language::Python,
                ["main.py", "README.md"],
                ["Cargo.toml", "Makefile"],
            ),
        ] {
            let destination = scratch.0.join(language.name());
            scaffold_with_toolkit("some-app", &destination, None, language).unwrap();
            for file in wanted {
                assert!(
                    destination.join(file).is_file(),
                    "{} has no {file}",
                    language.name()
                );
            }
            for file in unwanted {
                assert!(
                    !destination.join(file).exists(),
                    "{} was given {file}, which is another language's",
                    language.name()
                );
            }

            for shared in ["LICENSE", "assets/app.svg", "some-app.desktop"] {
                assert!(
                    destination.join(shared).is_file(),
                    "{} has no {shared}",
                    language.name()
                );
            }

            let entry = fs::read_to_string(destination.join("some-app.desktop")).unwrap();
            assert!(entry.contains("StartupWMClass=some-app"));
            let source = match language {
                Language::Rust => "src/main.rs",
                Language::C => "src/main.c",
                Language::Python => "main.py",
            };
            let written = fs::read_to_string(destination.join(source)).unwrap();
            assert!(
                written.contains("\"some-app\""),
                "{} does not open its window under its own id",
                language.name()
            );

            for file in wanted.iter().chain(&["README.md"]) {
                let contents = fs::read_to_string(destination.join(file)).unwrap();
                assert!(
                    !contents.contains("{{"),
                    "{}/{file} still carries an unrendered placeholder",
                    language.name()
                );
            }
        }
    }

    #[test]
    fn an_unknown_language_is_refused() {
        assert_eq!(Language::named("rust"), Some(Language::Rust));
        assert_eq!(Language::named("c"), Some(Language::C));
        assert_eq!(Language::named("python"), Some(Language::Python));
        assert_eq!(Language::named("Rust"), None);
        assert_eq!(Language::named("go"), None);
    }

    #[test]
    fn a_feature_can_be_turned_on_either_shape_of_dependency() {
        assert_eq!(
            with_feature("\"0.2.0\"", "winit"),
            "{ version = \"0.2.0\", features = [\"winit\"] }"
        );
        assert_eq!(
            with_feature("{ version = \"0.2.0\", path = \"/x\" }", "winit"),
            "{ version = \"0.2.0\", path = \"/x\", features = [\"winit\"] }"
        );
    }

    #[test]
    fn an_installed_toolkit_is_found_by_the_most_explicit_route_available() {
        let scratch = Scratch::new();
        let prefix = scratch.0.join("usr");
        let crate_dir = prefix.join(INSTALLED_CRATES).join(CRATES[0]);
        let executable = prefix.join("bin/lxb-new");
        fs::create_dir_all(&crate_dir).unwrap();
        fs::create_dir_all(executable.parent().unwrap()).unwrap();

        assert_eq!(
            installed_toolkit_path_from(None, None, Some(&executable)),
            None
        );

        fs::write(crate_dir.join("Cargo.toml"), "[package]\n").unwrap();
        assert_eq!(
            installed_toolkit_path_from(None, None, Some(&executable)),
            Some(crate_dir.clone())
        );

        let baked = scratch.0.join("baked");
        fs::create_dir_all(&baked).unwrap();
        fs::write(baked.join("Cargo.toml"), "[package]\n").unwrap();
        assert_eq!(
            installed_toolkit_path_from(None, Some(&baked), Some(&executable)),
            Some(baked.clone())
        );

        assert_eq!(
            installed_toolkit_path_from(None, Some(Path::new("/nonexistent")), Some(&executable)),
            Some(crate_dir.clone())
        );

        assert_eq!(
            installed_toolkit_path_from(
                Some(OsString::from("/asked/for")),
                Some(&baked),
                Some(&executable)
            ),
            Some(PathBuf::from("/asked/for"))
        );

        assert_eq!(
            installed_toolkit_path_from(Some(OsString::new()), None, Some(&executable)),
            Some(crate_dir)
        );
    }

    #[test]
    fn next_step_directory_is_shell_quoted() {
        assert_eq!(shell_quote(Path::new("two words")), "'two words'");
        assert_eq!(shell_quote(Path::new("it's-here")), "'it'\\''s-here'");
    }
}
