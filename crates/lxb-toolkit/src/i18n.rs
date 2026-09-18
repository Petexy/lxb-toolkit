//! What the language *says*, beside what it looks like.
//!
//! The toolkit draws controls of its own — the file question above all — and
//! those controls carry words. This is where the words come from, in the same
//! form the shell keeps them in: Fluent catalogs, compiled into the binary, one
//! per language, read by message identifier. An application embeds its own
//! catalogs the same way and asks them through a [`Catalog`] of its own; the
//! toolkit's own strings are the ones behind [`text`] and [`format`].
//!
//! The language is the session's, resolved once from the environment, and
//! nothing here ever writes to it. On LineXinBar the shell exports the locale
//! to everything it opens, so an application started from the bar speaks what
//! Settings > Language says without being told; on any other desktop it speaks
//! what that desktop set. A running program keeps the language it started in,
//! which is what every other program on a Linux desktop does.
//!
//! Identifiers are never user text: a path, a file's name, a remote's id or
//! anything a person typed is passed as a *variable*, never as a message id or
//! as catalog source.
use std::collections::BTreeMap;
use std::sync::LazyLock;

pub use fluent_bundle::FluentArgs;
use fluent_bundle::{concurrent::FluentBundle, FluentResource, FluentValue};

/// One language the toolkit knows the name of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Language {
    /// The base tag a locale is matched against: `pl` matches `pl_PL.UTF-8`.
    pub tag: &'static str,
    /// What the language calls itself, which is the same in every language.
    pub name: &'static str,
}

/// Every language there is a catalog for. Adding one is a line here, a `.ftl`
/// beside `en-GB.ftl` in this crate, and one in each application — see
/// `docs/localization.md`.
///
/// [`FALLBACK`] is first, because it is the one every other entry falls back
/// to and the one an application must ship. The rest stand in the order their
/// own names sort in, alphabet by alphabet, which is the order the shell's
/// Settings > Language reads down.
///
/// Two catalogs carry a region in their tag without a region deciding them:
/// `pt-BR` and `zh-CN` are the Portuguese and the Chinese that were written,
/// and `pt_PT` and `zh_TW` read them too — see [`language_from`].
pub const LANGUAGES: &[Language] = &[
    Language {
        tag: "en-GB",
        name: "English (UK)",
    },
    Language {
        tag: "de",
        name: "Deutsch",
    },
    Language {
        tag: "en-US",
        name: "English (US)",
    },
    Language {
        tag: "es",
        name: "Español",
    },
    Language {
        tag: "fr",
        name: "Français",
    },
    Language {
        tag: "pl",
        name: "Polski",
    },
    Language {
        tag: "pt-BR",
        name: "Português (Brasil)",
    },
    Language {
        tag: "ru",
        name: "Русский",
    },
    Language {
        tag: "hi",
        name: "हिन्दी",
    },
    Language {
        tag: "zh-CN",
        name: "简体中文",
    },
];

/// The catalog behind all of them: the English this toolkit is written in, and
/// the answer to every message another language has not got.
///
/// It is British, which matters in exactly two places — the order of a date
/// and a handful of spellings — and `en-US.ftl` is an **overlay** carrying
/// those and nothing else. See [`Catalog::validate`], which holds it to that.
pub const FALLBACK: &str = "en-GB";

/// The tag a locale name resolves to, or [`FALLBACK`] for one nothing
/// translates.
///
/// A locale carries more than a language — `pl_PL.UTF-8@euro` — and usually
/// only the first part of it names one. The territory is looked at too, for
/// the one case where it decides a catalog: `en_US` is written differently
/// from every other English, so it is the one region named here. `en_GB`,
/// `en_AU` and a bare `en` are all the British catalog, and so are `C`,
/// `POSIX` and a language nothing translates.
///
/// `pt-BR` and `zh-CN` carry a region in their tags and are read by the whole
/// language all the same: `pt_PT` lands on the Brazilian catalog and `zh_TW`
/// on the Simplified one, because a reader of the other variant of their own
/// language is answered better by it than by English, which is the
/// alternative. A regional catalog is found by its region first and by its
/// language second, and [`LANGUAGES`] puts [`FALLBACK`] first so that a bare
/// `en` finds the British catalog and not the American overlay.
pub fn language_from(locale: &str) -> &'static str {
    let mut parts = locale.trim().split(['_', '-', '.', '@']);
    let base = parts.next().unwrap_or_default();
    let region = parts.next().unwrap_or_default();
    let language_of = |tag: &'static str| tag.split('-').next().unwrap_or(tag);
    // A regional catalog first, so `en_US` never matches on `en` alone.
    LANGUAGES
        .iter()
        .find(|language| {
            language
                .tag
                .split_once('-')
                .is_some_and(|(theirs, country)| {
                    base.eq_ignore_ascii_case(theirs) && region.eq_ignore_ascii_case(country)
                })
        })
        .or_else(|| {
            LANGUAGES
                .iter()
                .find(|language| base.eq_ignore_ascii_case(language_of(language.tag)))
        })
        .map_or(FALLBACK, |language| language.tag)
}

/// POSIX precedence: the first of `LC_ALL`, `LC_MESSAGES`, `LANG` that says
/// anything. Split out from [`language`] so it can be tested without an
/// environment.
pub fn resolve(mut get: impl FnMut(&str) -> Option<String>) -> &'static str {
    for name in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Some(value) = get(name).filter(|value| !value.trim().is_empty()) {
            return language_from(&value);
        }
    }
    FALLBACK
}

static LANGUAGE: LazyLock<&'static str> =
    LazyLock::new(|| resolve(|name| std::env::var(name).ok()));

/// The language this process speaks, decided on first use and then fixed.
pub fn language() -> &'static str {
    *LANGUAGE
}

struct Bundle {
    fluent: FluentBundle<FluentResource>,
    /// Every message that takes no variables, formatted once. A label is
    /// borrowed from here rather than built on every frame.
    plain: BTreeMap<String, String>,
}

/// The catalogs of one crate: its languages, and English behind all of them.
///
/// Built once and then only read, which is why a label can be handed out as a
/// `&'static str` from a `static` catalog. A message a language has not got is
/// answered in English; one that is in no catalog at all is answered with its
/// own identifier, which is a missing translation somebody can see and search
/// for rather than an empty label.
pub struct Catalog {
    bundles: BTreeMap<&'static str, Bundle>,
}

impl Catalog {
    /// `resources` is `(tag, catalog source)`, English included.
    pub fn new(resources: &[(&'static str, &'static str)]) -> Self {
        let mut bundles = BTreeMap::new();
        for &(language, source) in resources {
            let resource =
                FluentResource::try_new(source.to_owned()).expect("valid embedded Fluent catalog");
            let mut fluent = FluentBundle::new_concurrent(vec![language
                .parse()
                .expect("valid catalog language")]);
            // Every shipped language reads left to right, and the isolating
            // marks would otherwise be drawn as boxes by a font that has no
            // glyph for them.
            fluent.set_use_isolating(false);
            // `PAD2($n)`: a day, a month or a minute written with two digits.
            // Whether a date reads `16/09` or `09/16` and whether a minute is
            // padded are the language's decisions, so they are made in the
            // catalog — and fluent-bundle parses NUMBER's minimumIntegerDigits
            // and then does not apply it, which is why this is a function of
            // its own. The shell carries the same one; see its `i18n.rs`.
            fluent
                .add_function("PAD2", |positional, _named| match positional.first() {
                    Some(FluentValue::Number(number)) => {
                        FluentValue::String(format!("{:02}", number.value as i64).into())
                    }
                    Some(FluentValue::String(text)) => FluentValue::String(
                        text.parse::<i64>()
                            .map_or_else(|_| text.clone(), |n| format!("{n:02}").into()),
                    ),
                    _ => FluentValue::Error,
                })
                .expect("PAD2 is registered once per bundle");
            fluent.add_resource(resource).expect("unique message ids");
            let plain = identifiers(source)
                .filter_map(|id| {
                    let pattern = fluent.get_message(&id)?.value()?;
                    let mut errors = Vec::new();
                    let value = fluent
                        .format_pattern(pattern, None, &mut errors)
                        .into_owned();
                    errors.is_empty().then_some((id, value))
                })
                .collect();
            assert!(
                bundles.insert(language, Bundle { fluent, plain }).is_none(),
                "one catalog per language"
            );
        }
        assert!(bundles.contains_key(FALLBACK), "the fallback catalog");
        Self { bundles }
    }

    /// A label with nothing in it, in the session's language.
    pub fn text<'a>(&'a self, id: &'a str) -> &'a str {
        self.text_for(language(), id)
    }

    /// The same, in a language named rather than the session's.
    pub fn text_for<'a>(&'a self, locale: &str, id: &'a str) -> &'a str {
        self.bundles
            .get(language_from(locale))
            .and_then(|bundle| bundle.plain.get(id))
            .or_else(|| self.bundles[FALLBACK].plain.get(id))
            .map_or(id, String::as_str)
    }

    /// A sentence with values in it, in the session's language.
    pub fn format(&self, id: &str, args: &FluentArgs<'_>) -> String {
        self.format_for(language(), id, args)
    }

    /// The same, in a language named rather than the session's.
    pub fn format_for(&self, locale: &str, id: &str, args: &FluentArgs<'_>) -> String {
        let render = |bundle: &Bundle| {
            let pattern = bundle.fluent.get_message(id)?.value()?;
            let mut errors = Vec::new();
            let value = bundle
                .fluent
                .format_pattern(pattern, Some(args), &mut errors)
                .into_owned();
            errors.is_empty().then_some(value)
        };
        self.bundles
            .get(language_from(locale))
            .and_then(render)
            .or_else(|| render(&self.bundles[FALLBACK]))
            .unwrap_or_else(|| id.to_owned())
    }

    /// Every catalog carries the same messages, with the same variables in
    /// them, and every one of them formats.
    ///
    /// A translation that dropped a variable, or spelled one differently, is a
    /// sentence with a hole in it; a plural form the language has no rule for
    /// is a sentence that comes out in the wrong case. Both are caught here,
    /// which is why each application's catalog test is one line.
    ///
    /// **A regional overlay is held to the opposite rule.** `en-US` is the
    /// same language as [`FALLBACK`] with a few things written differently, so
    /// its file carries *only* those: every message in it must be one the
    /// fallback has, must take the same variables, and must actually say
    /// something else. A message copied across unchanged is a message that has
    /// to be edited twice from the day it is copied, and one day will not be —
    /// which is the whole reason an overlay is worth having.
    pub fn validate(resources: &[(&'static str, &'static str)]) {
        let catalog = Self::new(resources);
        let english = messages(
            resources
                .iter()
                .find(|(tag, _)| *tag == FALLBACK)
                .expect("the fallback catalog")
                .1,
        );
        for &(locale, source) in resources {
            let translated = messages(source);
            if is_an_overlay(locale) {
                assert!(
                    !translated.is_empty(),
                    "the {locale} overlay carries no difference at all"
                );
                for (id, value) in &translated {
                    let Some(theirs) = english.get(id) else {
                        panic!("the {locale} overlay writes {id}, which no catalog has");
                    };
                    assert_ne!(
                        theirs, value,
                        "the {locale} overlay copies {id} out of {FALLBACK} unchanged"
                    );
                    assert_eq!(
                        variables(theirs),
                        variables(value),
                        "the {locale} overlay's {id} takes other variables"
                    );
                }
                continue;
            }
            assert_eq!(
                english.keys().collect::<Vec<_>>(),
                translated.keys().collect::<Vec<_>>(),
                "the {locale} catalog carries other messages than the English one"
            );
            for (id, value) in &english {
                let wanted = variables(value);
                assert_eq!(
                    wanted,
                    variables(&translated[id]),
                    "the {locale} translation of {id} takes other variables"
                );
                // The counts that pick out every plural form Polish and
                // Russian have, and the teens, which are the ones a rule
                // written by hand gets wrong.
                for count in [0, 1, 2, 5, 12, 22, 112] {
                    let mut args = FluentArgs::new();
                    for variable in &wanted {
                        args.set(*variable, count);
                    }
                    let bundle = &catalog.bundles[locale].fluent;
                    let pattern = bundle
                        .get_message(id)
                        .and_then(|message| message.value())
                        .expect("every message has a value");
                    let mut errors = Vec::new();
                    let _ = bundle.format_pattern(pattern, Some(&args), &mut errors);
                    assert!(errors.is_empty(), "{locale}:{id} with {count}: {errors:?}");
                }
            }
        }
    }

    /// Every message a crate's own sources ask for is one the catalogs have.
    ///
    /// The one failure [`Catalog::validate`] cannot see: a label reached by an
    /// identifier nothing translates is drawn as the identifier itself, in
    /// every language including English. `directory` is the crate root — a
    /// test passes `env!("CARGO_MANIFEST_DIR")` — and every `.rs` file under
    /// its `src` is read.
    pub fn check_references(directory: &str, resources: &[(&'static str, &'static str)]) {
        let english: std::collections::BTreeSet<String> = identifiers(
            resources
                .iter()
                .find(|(tag, _)| *tag == FALLBACK)
                .expect("the fallback catalog")
                .1,
        )
        .collect();
        let mut asked = 0usize;
        for file in rust_files(std::path::Path::new(directory).join("src")) {
            let source = std::fs::read_to_string(&file).expect("a source file that can be read");
            for marker in ["i18n::text(\"", "i18n::format(\"", "message!(\""] {
                for (at, _) in source.match_indices(marker) {
                    let rest = &source[at + marker.len()..];
                    let Some(end) = rest.find('"') else { continue };
                    let id = &rest[..end];
                    asked += 1;
                    assert!(
                        english.contains(id),
                        "{} asks for {id}, which no catalog has",
                        file.display()
                    );
                }
            }
        }
        assert!(asked > 0, "no messages are asked for under {directory}/src");
    }
}

/// Whether a tag names a *regional* catalog of a language another entry in
/// [`LANGUAGES`] already carries in full — `en-US` beside `en-GB`. Such a file
/// is an overlay of the differences, which is why it is validated the other
/// way round from a translation.
fn is_an_overlay(tag: &str) -> bool {
    tag != FALLBACK
        && tag
            .split_once('-')
            .is_some_and(|(base, _)| FALLBACK.split('-').next() == Some(base))
}

/// The message identifiers a catalog declares, in the order it declares them.
/// A continuation line is indented and a comment begins with `#`; everything
/// else is `id = value`.
fn identifiers(source: &str) -> impl Iterator<Item = String> + '_ {
    source
        .lines()
        .filter(|line| !line.starts_with(char::is_whitespace) && !line.starts_with('#'))
        .filter_map(|line| line.split_once(" =").map(|(id, _)| id.to_owned()))
}

/// A catalog as `id` to its whole value, continuation lines joined on.
fn messages(source: &str) -> BTreeMap<String, String> {
    let mut messages: BTreeMap<String, String> = BTreeMap::new();
    let mut current = None;
    for line in source.lines() {
        if line.starts_with('#') {
            continue;
        }
        if !line.starts_with(char::is_whitespace) {
            if let Some((id, value)) = line.split_once(" =") {
                assert!(
                    messages.insert(id.to_owned(), value.to_owned()).is_none(),
                    "{id} is declared twice"
                );
                current = Some(id.to_owned());
                continue;
            }
        }
        if let Some(id) = &current {
            messages
                .get_mut(id)
                .expect("the message the lines belong to")
                .push_str(line);
        }
    }
    messages
}

/// The variables a value reads, by name.
fn variables(value: &str) -> std::collections::BTreeSet<&str> {
    value
        .split('$')
        .skip(1)
        .map(|tail| {
            tail.split(|letter: char| !letter.is_ascii_alphanumeric() && letter != '-')
                .next()
                .unwrap_or_default()
        })
        .collect()
}

/// Every `.rs` file under a directory, itself and downwards.
fn rust_files(directory: std::path::PathBuf) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let mut looking = vec![directory];
    while let Some(here) = looking.pop() {
        let Ok(reading) = std::fs::read_dir(&here) else {
            continue;
        };
        for entry in reading.flatten() {
            let path = entry.path();
            if path.is_dir() {
                looking.push(path);
            } else if path.extension().is_some_and(|kind| kind == "rs") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// The toolkit's own catalogs: what the controls it draws say.
pub const RESOURCES: &[(&str, &str)] = &[
    ("en-GB", include_str!("../locales/en-GB.ftl")),
    ("de", include_str!("../locales/de.ftl")),
    ("en-US", include_str!("../locales/en-US.ftl")),
    ("es", include_str!("../locales/es.ftl")),
    ("fr", include_str!("../locales/fr.ftl")),
    ("hi", include_str!("../locales/hi.ftl")),
    ("pl", include_str!("../locales/pl.ftl")),
    ("pt-BR", include_str!("../locales/pt-BR.ftl")),
    ("ru", include_str!("../locales/ru.ftl")),
    ("zh-CN", include_str!("../locales/zh-CN.ftl")),
];

static SHARED: LazyLock<Catalog> = LazyLock::new(|| Catalog::new(RESOURCES));

/// A toolkit label with nothing in it.
pub fn text(id: &'static str) -> &'static str {
    SHARED.text(id)
}

/// A toolkit sentence with values in it. Reached through [`crate::message`].
pub fn format(id: &str, args: &FluentArgs<'_>) -> String {
    SHARED.format(id, args)
}

/// The month in a date the toolkit writes, by number from one.
///
/// A month is not a word on its own in every language: Polish writes the
/// genitive in a date — *1 stycznia*, never *1 styczeń* — so the names live in
/// the catalog beside the date that uses them rather than in a table here.
pub fn month(month: usize) -> &'static str {
    const IDS: [&str; 12] = [
        "month-january",
        "month-february",
        "month-march",
        "month-april",
        "month-may",
        "month-june",
        "month-july",
        "month-august",
        "month-september",
        "month-october",
        "month-november",
        "month-december",
    ];
    IDS.get(month.wrapping_sub(1)).map_or("", |id| text(id))
}

/// The time of day, on whichever clock the session is set to: `20:10`, or
/// `8:10 PM`. `hour` is 0 to 23.
///
/// Settings > System > Clock is the answer and it comes out of the shell's own
/// settings file, so an application built on this toolkit writes a time the
/// same way the shell does — see [`crate::settings::twelve_hour_clock`], which
/// is also what decides it for a desktop that is not this shell's.
///
/// A time written into a *file name* or a *file format* must not come through
/// here: a sortable stamp and an RFC 3339 field are not somebody's setting to
/// change.
pub fn time_of_day(hour: u32, minute: u32) -> String {
    let (hour, minute) = (hour.min(23), minute.min(59));
    if !crate::settings::twelve_hour_clock() {
        return crate::message!("clock-24-hour",
            "hour" => hour.to_string(),
            "minute" => minute.to_string(),
        );
    }
    // Midnight is twelve, not zero, and so is noon: the hour rolls to twelve
    // at each end rather than counting from it.
    let half = text(if hour < 12 { "clock-am" } else { "clock-pm" });
    let shown = match hour % 12 {
        0 => 12,
        other => other,
    };
    crate::message!("clock-12-hour",
        "hour" => shown.to_string(),
        "minute" => minute.to_string(),
        "half" => half,
    )
}

/// A quantity this toolkit has already written out, with the decimal mark the
/// language uses. Never a file's name, a version or anything off a protocol:
/// this replaces a full stop, and a name is not a number.
pub fn decimal(number: String) -> String {
    // German, Spanish, French, Polish, Portuguese and Russian write the
    // fraction after a comma; the two Englishes, Hindi and Chinese after a
    // full stop. It follows the *language* and not the country, which is why
    // this asks the catalog rather than the locale.
    if matches!(language(), "en-GB" | "en-US" | "hi" | "zh-CN") {
        number
    } else {
        number.replace('.', ",")
    }
}

/// A sentence with values in it, from the toolkit's own catalogs.
///
/// A count is passed as a number — `"count" => files` — never as text, because
/// a language picks the form of the noun by looking at the number.
#[macro_export]
macro_rules! message {
    ($id:literal $(, $name:literal => $value:expr)* $(,)?) => {{
        let mut args = $crate::i18n::FluentArgs::new();
        $(args.set($name, $value);)*
        $crate::i18n::format($id, &args)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalogs_carry_the_same_messages_and_every_one_of_them_formats() {
        Catalog::validate(RESOURCES);
    }

    #[test]
    fn every_message_the_toolkit_asks_for_is_one_the_catalogs_have() {
        Catalog::check_references(env!("CARGO_MANIFEST_DIR"), RESOURCES);
    }

    #[test]
    fn polish_counts_take_the_form_the_number_calls_for() {
        let catalog = Catalog::new(RESOURCES);
        for (count, expected) in [
            (0, "0 plików"),
            (1, "1 plik"),
            (2, "2 pliki"),
            (5, "5 plików"),
            (12, "12 plików"),
            (22, "22 pliki"),
            (112, "112 plików"),
        ] {
            let mut args = FluentArgs::new();
            args.set("count", count);
            assert_eq!(
                catalog.format_for("pl_PL.UTF-8", "count-files", &args),
                expected
            );
        }
    }

    #[test]
    fn a_name_is_a_value_and_never_catalog_source() {
        let catalog = Catalog::new(RESOURCES);
        let mut args = FluentArgs::new();
        args.set("name", "Żółć { $name }.png");
        assert_eq!(
            catalog.format_for("pl", "save-as-name", &args),
            "Zapisz jako Żółć { $name }.png"
        );
    }

    #[test]
    fn the_word_for_a_press_is_the_word_for_what_it_does() {
        // Two English words the picker tells apart — one file is selected,
        // several are chosen — and Polish tells them apart the other way
        // round. A table of words would have crossed them over.
        assert_eq!(text_in("pl", "open"), "Otwórz");
        assert_eq!(text_in("pl", "select"), "Wybierz");
        assert_eq!(text_in("pl", "choose"), "Zaznacz");
    }

    /// A locale names a language by its first part, and a *country* only
    /// where the catalogs tell that country apart — which is `en_US` and
    /// nothing else.
    #[test]
    fn a_locale_names_a_language_by_its_first_part_and_the_one_country_that_counts() {
        assert_eq!(language_from("pl-PL"), "pl");
        assert_eq!(language_from("pl_PL.UTF-8@euro"), "pl");
        // A language is matched on the language and not the country, so every
        // French is French and every Spanish is Spanish.
        assert_eq!(language_from("fr_FR.UTF-8"), "fr");
        assert_eq!(language_from("fr_CA.UTF-8"), "fr");
        assert_eq!(language_from("es_ES.UTF-8"), "es");
        assert_eq!(language_from("es_MX"), "es");
        assert_eq!(language_from("de_AT.UTF-8"), "de");
        assert_eq!(language_from("hi_IN.UTF-8"), "hi");
        assert_eq!(language_from("ru_RU.UTF-8"), "ru");
        // The two catalogs named for a region are read by the whole language:
        // the other variant is a better answer than English.
        assert_eq!(language_from("pt_BR.UTF-8"), "pt-BR");
        assert_eq!(language_from("pt_PT.UTF-8"), "pt-BR");
        assert_eq!(language_from("pt"), "pt-BR");
        assert_eq!(language_from("zh_CN.UTF-8"), "zh-CN");
        assert_eq!(language_from("zh_TW.UTF-8"), "zh-CN");
        assert_eq!(language_from("zh"), "zh-CN");
        // And a language with no catalog at all falls to the one this toolkit
        // is written in.
        assert_eq!(language_from("ja_JP.UTF-8"), "en-GB");
        assert_eq!(language_from("C.UTF-8"), "en-GB");
        assert_eq!(language_from(""), "en-GB");

        assert_eq!(language_from("en_US.UTF-8"), "en-US");
        assert_eq!(language_from("en-US"), "en-US");
        assert_eq!(language_from("en_us"), "en-US", "a locale is not case");
        // Every other English, including the bare one, is the catalog this
        // toolkit is written in.
        assert_eq!(language_from("en_GB.UTF-8"), "en-GB");
        assert_eq!(language_from("en_AU.UTF-8"), "en-GB");
        assert_eq!(language_from("en"), "en-GB");
        // And a language whose *country* happens to spell an English one is
        // still read by its language.
        assert_eq!(language_from("pl_US.UTF-8"), "pl");
    }

    #[test]
    fn the_first_variable_that_says_anything_decides_the_language() {
        let set = |name: &str| match name {
            "LC_ALL" => Some("C.UTF-8".to_string()),
            "LANG" => Some("pl_PL.UTF-8".to_string()),
            _ => None,
        };
        assert_eq!(resolve(set), FALLBACK);
        assert_eq!(
            resolve(|name| (name == "LC_MESSAGES").then(|| "pl_PL.UTF-8".to_string())),
            "pl"
        );
        assert_eq!(
            resolve(|name| (name == "LC_ALL").then(|| "   ".to_string())),
            FALLBACK
        );
        assert_eq!(resolve(|_| None), FALLBACK);
        assert_eq!(
            resolve(|name| (name == "LANG").then(|| "en_US.UTF-8".to_string())),
            "en-US"
        );
    }

    #[test]
    fn a_language_that_is_missing_a_message_falls_back_to_english() {
        let catalog = Catalog::new(&[
            (FALLBACK, "hello = Hello\nnamed = Hello { $name }"),
            ("pl", "other = Inne"),
        ]);
        assert_eq!(catalog.text_for("pl", "hello"), "Hello");
        let mut args = FluentArgs::new();
        args.set("name", "Settings");
        assert_eq!(catalog.format_for("pl", "named", &args), "Hello Settings");
        // And one nothing has at all is drawn as itself, which is a missing
        // translation somebody can see and grep for.
        assert_eq!(catalog.text_for("pl", "missing"), "missing");
    }

    /// An overlay answers what it writes and the fallback answers the rest,
    /// which is what lets `en-US.ftl` be a page of differences.
    #[test]
    fn an_overlay_answers_only_what_it_writes() {
        let catalog = Catalog::new(&[
            (FALLBACK, "date = { $day } { $month }\nopen = Open"),
            ("en-US", "date = { $month } { $day }"),
        ]);
        let mut args = FluentArgs::new();
        args.set("day", 16);
        args.set("month", "September");
        assert_eq!(catalog.format_for("en-US", "date", &args), "September 16");
        assert_eq!(catalog.format_for("en-GB", "date", &args), "16 September");
        assert_eq!(catalog.text_for("en-US", "open"), "Open");

        assert!(is_an_overlay("en-US"));
        assert!(
            !is_an_overlay("en-GB"),
            "the fallback is not its own overlay"
        );
        assert!(!is_an_overlay("pl"));
        assert!(!is_an_overlay("pt-BR"), "another language, not this one");
    }

    /// The toolkit's own catalogs write a time on whichever clock, and both
    /// forms come out of the set the shell's corner can draw.
    #[test]
    fn a_time_is_written_on_whichever_clock() {
        let catalog = Catalog::new(RESOURCES);
        let written = |locale: &str, id: &str, hour: i32, half: &str| {
            let mut args = FluentArgs::new();
            args.set("hour", hour);
            args.set("minute", 5);
            args.set("half", half.to_string());
            catalog.format_for(locale, id, &args)
        };
        for locale in LANGUAGES.iter().map(|language| language.tag) {
            assert_eq!(written(locale, "clock-24-hour", 20, ""), "20:05");
            assert_eq!(written(locale, "clock-12-hour", 8, "PM"), "8:05 PM");
            assert_eq!(
                catalog.text_for(locale, "clock-am"),
                "AM",
                "the corner draws these three letters and no others"
            );
        }
    }

    /// Every language names itself, and the names stand in the order they
    /// sort in, alphabet by alphabet — which is the shell's order too.
    #[test]
    fn the_languages_name_themselves_in_the_order_the_shell_lists_them() {
        let names: Vec<&str> = LANGUAGES.iter().map(|language| language.name).collect();
        assert_eq!(
            names,
            [
                "English (UK)",
                "Deutsch",
                "English (US)",
                "Español",
                "Français",
                "Polski",
                "Português (Brasil)",
                "Русский",
                "हिन्दी",
                "简体中文",
            ]
        );
        assert_eq!(LANGUAGES[0].tag, FALLBACK, "the fallback comes first");
        for language in LANGUAGES {
            assert_eq!(language_from(language.tag), language.tag);
            assert!(
                RESOURCES.iter().any(|(tag, _)| *tag == language.tag),
                "{} has no catalog",
                language.tag
            );
        }
    }

    /// The five languages that followed French and Spanish, each on CLDR's
    /// own rule: Russian has Polish's three forms, Hindi and Brazilian
    /// Portuguese count nought as singular, German does not, and Chinese has
    /// one form and no selector at all.
    #[test]
    fn the_later_languages_take_the_form_the_number_calls_for() {
        let catalog = Catalog::new(RESOURCES);
        let files = |locale: &str, count: i32| {
            let mut args = FluentArgs::new();
            args.set("count", count);
            catalog.format_for(locale, "count-files", &args)
        };
        for (count, expected) in [
            (0, "0 файлов"),
            (1, "1 файл"),
            (2, "2 файла"),
            (5, "5 файлов"),
            (12, "12 файлов"),
            (22, "22 файла"),
            (112, "112 файлов"),
        ] {
            assert_eq!(files("ru_RU.UTF-8", count), expected);
        }
        assert_eq!(files("de", 0), "0 Dateien");
        assert_eq!(files("de", 1), "1 Datei");
        assert_eq!(files("pt_BR", 0), "0 arquivo");
        assert_eq!(files("pt_BR", 2), "2 arquivos");
        assert_eq!(files("hi", 0), "0 फ़ाइल");
        assert_eq!(files("hi", 2), "2 फ़ाइलें");
        assert_eq!(files("zh_CN", 1), "1 个文件");
        assert_eq!(files("zh_CN", 22), "22 个文件");
    }

    fn text_in(locale: &str, id: &'static str) -> String {
        Catalog::new(RESOURCES).text_for(locale, id).to_string()
    }
}
