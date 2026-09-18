//! The faces the toolkit ships, and the check that every word a catalog
//! carries can be drawn by one of them.
//!
//! Roboto has no Devanagari and no Han in it, so before the two Noto faces
//! were bundled every Hindi and Chinese label rasterised to a row of empty
//! boxes — and nothing that reads strings rather than glyphs would have said
//! a word about it. [`check_drawable`] is the test that says it, and every
//! application runs it over its own catalogs in one line beside
//! `Catalog::validate`.
use glyphon::{
    cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, Weight},
    fontdb,
};
use lxb_toolkit::assets;

/// A font system of the toolkit's own six faces and nothing else, so what a
/// run shapes to is the same on any machine.
pub fn shipped_faces() -> FontSystem {
    let mut db = fontdb::Database::new();
    for face in assets::FONTS {
        db.load_font_data(face.to_vec());
    }
    FontSystem::new_with_locale_and_db("en-GB".to_string(), db)
}

/// Whether every character of `text` came out as a glyph of a shipped face.
///
/// Glyph zero is `.notdef`, the empty box a reader sees where no face on the
/// machine had the character. A run that shaped to nothing at all is not
/// drawn either.
pub fn drawn(fonts: &mut FontSystem, text: &str, bold: bool) -> bool {
    let mut buffer = Buffer::new(fonts, Metrics::new(18.0, 22.0));
    buffer.set_size(None, None);
    let attrs = Attrs::new().family(Family::Name("Roboto")).weight(if bold {
        Weight::BOLD
    } else {
        Weight::NORMAL
    });
    buffer.set_text(text, &attrs, Shaping::Advanced, None);
    buffer.shape_until_scroll(fonts, false);
    let mut any = false;
    let mut all = true;
    for run in buffer.layout_runs() {
        for glyph in run.glyphs.iter() {
            any = true;
            all &= glyph.glyph_id != 0;
        }
    }
    any && all
}

/// Every word of every catalog in `resources`, drawn by a face the toolkit
/// ships, at both weights. Panics on the first that is not.
///
/// Shaped from the catalog's *text* rather than through Fluent, with the
/// placeables and selector heads cut out: a `{ $count }` is somebody else's
/// characters. The Han face is a subset, so this also holds a `zh-CN.ftl` to
/// GB 2312: a character outside it is a sentence to reword, not a face to
/// regrow.
pub fn check_drawable(resources: &[(&str, &str)]) {
    let mut fonts = shipped_faces();
    for (tag, source) in resources {
        for line in source.lines().filter(|line| !line.starts_with('#')) {
            let value = line.split_once(" = ").map_or(line, |(_, value)| value);
            let mut words = String::new();
            let mut depth = 0usize;
            for c in value.chars() {
                match c {
                    '{' => depth += 1,
                    '}' => depth = depth.saturating_sub(1),
                    _ if depth == 0 && !c.is_whitespace() => words.push(c),
                    _ => {}
                }
            }
            let words = words.trim_start_matches(['*', '[']);
            if words.is_empty() {
                continue;
            }
            for bold in [false, true] {
                assert!(
                    drawn(&mut fonts, words, bold),
                    "{tag}: {line:?} has a character no shipped face can draw"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_catalog_word_is_drawn_by_a_shipped_face() {
        check_drawable(lxb_toolkit::i18n::RESOURCES);
        // And the names the languages call themselves, which an application
        // may list in any language.
        let mut fonts = shipped_faces();
        for language in lxb_toolkit::i18n::LANGUAGES {
            assert!(drawn(&mut fonts, language.name, false), "{}", language.name);
        }
        // The check has teeth: a script none of the six faces carries is
        // reported as undrawable, and so is a Han character outside GB 2312.
        assert!(!drawn(&mut fonts, "한글 ᚠᚢᚦ", false));
        assert!(!drawn(&mut fonts, "龘", false));
    }

    /// Each script's face is a face of its own, at both weights, and says so
    /// in its own name table — which is what a text engine picking a fallback
    /// by family name goes looking for.
    #[test]
    fn each_script_has_a_face_of_its_own_at_both_weights() {
        use lxb_toolkit::typography::{Face, Script};
        let mut db = fontdb::Database::new();
        for script in Script::ALL {
            for face in [Face::Regular, Face::Bold] {
                let bytes = face.bytes_for(script);
                assert!(!bytes.is_empty());
                let before = db.len();
                db.load_font_data(bytes.to_vec());
                assert_eq!(
                    db.len(),
                    before + 1,
                    "{script:?} {face:?} did not parse as a face"
                );
                let loaded = db.faces().last().unwrap();
                assert!(
                    loaded
                        .families
                        .iter()
                        .any(|(name, _)| name == script.family()),
                    "{script:?} {face:?} calls itself {:?}, not {:?}",
                    loaded.families,
                    script.family()
                );
                assert_eq!(loaded.weight.0, face.weight());
            }
        }
        assert_eq!(Script::named("han"), Some(Script::Han));
        assert_eq!(Script::named("Han"), None, "a name is exact");
    }
}
