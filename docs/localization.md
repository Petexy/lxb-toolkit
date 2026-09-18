# Languages

The toolkit speaks the ten languages the shell and its login screen speak —
**German**, **English (UK)**, **English (US)**, **Spanish**, **French**,
**Hindi**, **Polish**, **Brazilian Portuguese**, **Russian** and **Simplified
Chinese** — and an application built on it speaks whatever languages it ships
catalogs for. Everything is compiled into the binary, the two faces for the
scripts Roboto has not got included: there is nothing to install at run time,
no translation package, no message directory and no font to find.

## Which language a program speaks

The session's. `lxb_toolkit::i18n::language()` reads the first of `LC_ALL`,
`LC_MESSAGES` and `LANG` that says anything, takes the part of it that names a
language — `pl` out of `pl_PL.UTF-8@euro`, `fr` out of `fr_CA` — and answers
`en-GB` for a language nothing here translates, for `C` and `POSIX`, and for an environment that says
nothing at all. It is decided once, on first use, and does not change again
while the program runs.

The **territory** is looked at for one case, because there is one case where it
decides a catalog: `en_US` is `en-US` and every other English — `en_GB`,
`en_AU`, a bare `en` — is `en-GB`. See "The two Englishes" below. Two catalogs
carry a territory in their tag without one deciding them: `pt-BR` and `zh-CN`
are the Portuguese and the Chinese that were written, and `pt_PT` and `zh_TW`
read them too, because the other variant of a reader's own language is a better
answer than English.

## The two Englishes

`en-GB` is the English the toolkit is written in and the catalog every other
language falls back to (`i18n::FALLBACK`). `en-US` is an **overlay** beside it:
a file holding only what America writes differently, which in this toolkit is
one message — the order of a date, `1 March 1970` against `March 1, 1970`.

`Catalog::validate` holds an overlay to the opposite rule from a translation. A
translation must carry every message the fallback has; an overlay must carry
*only* messages the fallback has, each taking the same variables and each
actually saying something else. A line copied across unchanged is a line that
has to be edited twice from the day it is copied, and one day will not be.

An application does the same: `en-GB.ftl` in full, and an `en-US.ftl` only if
that application really writes something differently. Most have nothing to put
in one.

## The clock

`lxb_toolkit::settings::twelve_hour_clock()` answers whether a time of day is
written with AM or PM after it, and `lxb_toolkit::i18n::time_of_day(hour,
minute)` writes one. Both read Settings > System > Clock out of the shell's own
`shell.toml`, the way the accent and the button hints are read, so an
application writes a time the same way the shell does.

Unlike the other keys there it always answers rather than handing back `None`,
because there is a right answer for a machine that has never been asked: the
**language** decides, and American English and Hindi are the two that write
`8:10 PM` — the clock America and India both read.

A time written into a *file name* or a *file format* must not go through it: a
sortable stamp and an RFC 3339 field are not somebody's setting to change.

Nothing in the toolkit ever *writes* to the environment. On LineXinBar the
shell exports the session's locale to everything it opens, so an application
started from the bar comes up in the language Settings > Language names without
being told; on any other desktop it comes up in that desktop's language. An
application already open keeps the language it started in — which is what every
other program on a Linux desktop does — so a language changed in Settings
reaches an application the next time it is opened.

To see a page in the other language without changing anything:

```sh
LC_ALL=pl_PL.UTF-8 videonsole
LC_ALL=en_US.UTF-8 videonsole
```

Those locales do not have to be generated on the machine: the catalogs are
chosen by the name, not by libc.

## Where the words live

| | |
|---|---|
| `crates/lxb-toolkit/locales/*.ftl` | what the controls **the toolkit draws** say: the file question, its menus, the notes under its rows, the months in a date |
| `crates/lxb-toolkit/src/i18n.rs` | the language registry, the resolver, `Catalog`, the English fallback and the checks |
| each application's `locales/*.ftl` | what **that application** says, embedded the same way |

The format is [Fluent](https://projectfluent.org/) — the same catalogs, the
same identifiers and the same plural rules the shell itself uses, so a
translator who has worked on one has worked on all of them.

An application builds a `Catalog` of its own from its own resources:

```rust
static CATALOG: LazyLock<Catalog> = LazyLock::new(|| Catalog::new(RESOURCES));
```

and asks it for `text("id")` — a label with nothing in it — or
`message!("id", "count" => files)` for a sentence with values in it. Anything a
person, a device, a repository or a file gave its name to is passed **as a
value**, never as an identifier and never as catalog source.

## Counts, and why a number stays a number

A count is passed as a number:

```rust
crate::message!("count-files", "count" => files)     // yes
crate::message!("count-files", "count" => files.to_string())  // no
```

Fluent picks the form of the noun by looking at the number. Handed a string it
falls silently to the `other` form, which in Polish is the one nothing else
uses — the whole catalog comes out reading like a machine. English has two
forms, Polish four (`one`, `few`, `many`, `other`), and the two that catch a
mistake are 22 (`few` — *22 pliki*) and 112 (`many` — *112 plików*).

Where a sentence names a *kind* of thing as well as a count, the whole sentence
is one message with the kind as a value the catalog selects on, so the noun can
be declined and the verb agreed. A sentence is never built by joining
fragments.

## Dates and numbers

A date goes to the catalog whole — day, month and year as three values — because
the order of the three is the language's, and so is the form of the month:
Polish writes *1 stycznia 1970*, the genitive, which is not the name of the
month on its own. That is why the months are catalog entries beside the date
that uses them (`lxb_toolkit::i18n::month`) rather than a table in Rust.

`lxb_toolkit::i18n::decimal` puts the decimal mark the language uses into a
number this toolkit has already written out — a comma for German, Spanish,
French, Polish, Portuguese and Russian, a full stop for the two Englishes,
Hindi and Chinese. It follows the **language** and not the country, which is
why it asks the catalog rather than the locale. It is for quantities only:
never a file's name, a version or anything off a protocol.

Counts are passed as numbers for the same kind of reason. Polish and Russian
need `one`, `few` and `many`; German, Spanish, French, Hindi and Portuguese
need two forms and disagree about nought, which CLDR puts in `one` for French,
Hindi and Brazilian Portuguese (*0 fichier*, *0 फ़ाइल*, *0 arquivo*) and in
`other` for German and Spanish; Chinese has one form and writes no selector.
None of those rules is written here — a count passed as a *string* matches
nothing and every language silently takes `other`, which for French is wrong
at exactly one value and therefore never noticed.

## Faces

Roboto carries Latin, Greek and Cyrillic, which is eight of the ten. Hindi is
written in Devanagari and Chinese in Han, so two Noto faces travel beside it
under `crates/lxb-toolkit/assets/fonts/`, regular and bold: Noto Sans
Devanagari UI as the whole block, and Noto Sans CJK SC cut to the 6,763
characters of GB 2312, because the whole face is twenty megabytes a weight. The
four files are the shell's own, byte for byte, held by `scripts/check-sync.sh`;
the shell cuts the Han subset (`scripts/subset-han-face.py` there) and this
side copies it. The renderer reads them after Roboto and before the machine's
fonts, so a Devanagari or a Han word is shaped in the face the layout was
measured against. `lxb_render`'s `every_catalog_word_is_drawn_by_a_shipped_face`
shapes every line of every toolkit catalog through the six faces alone and
fails on the first character none of them has; a sentence that fails it is
reworded, not a face regrown. An application's own catalogs are held to the
same faces by the same kind of test, one line in its `src/i18n.rs`. A program
with a painter of its own takes all three faces from `lxb_font_for`
(`typography::Face::bytes_for` in Rust).

## Adding a language

1. Add its tag and its own name for itself to `LANGUAGES` in
   `crates/lxb-toolkit/src/i18n.rs`.
2. Copy `crates/lxb-toolkit/locales/en-GB.ftl` to `<tag>.ftl`, translate the
   values, and add it to `RESOURCES` in the same module.
3. Do the same in each application: copy its `locales/en-GB.ftl`, translate it,
   and add it to `RESOURCES` in its `src/i18n.rs`. An application that has no
   catalog for a language falls back to `en-GB` on its own, so this can be done
   one program at a time.
4. Translate the `.desktop` entry (`Name[xx]`, `GenericName[xx]`, `Comment[xx]`,
   `Keywords[xx]`) and the AppStream metadata (`<name xml:lang="xx">`,
   `<summary>`, the description paragraphs, and an entry under `<languages>`).
5. Run the checks below, and look at the result on a screen: an accented letter
   the font has to reach for, a label that is half again as long as the English,
   a menu row that no longer fits.

A *regional* variant of English — `en-AU` beside `en-GB` — is smaller work: a
file holding only the differences, one line in `LANGUAGES`, and nothing else.
`is_an_overlay` decides which of the two rules a file is checked against. A
language in a script none of the six faces carries also needs a face bundled,
in the shell first and transcribed here.

Keep the identifiers and the variable names exactly as English has them. Put
the variables in whatever order the language wants, and give the language as
many plural forms as it has.

## The checks

```sh
cargo fmt --all --check
cargo test --workspace --all-targets --locked
```

Three of those tests are about the catalogs, and every application has the same
three:

* **the catalogs carry the same messages** — same identifiers, same variables,
  and every message in every language formats without an error for counts of
  0, 1, 2, 5, 12, 22 and 112;
* **every message the sources ask for is one the catalogs have** —
  `Catalog::check_references` reads the crate's own `src` and fails on an
  identifier nothing translates, which would otherwise be drawn on screen as
  the identifier itself, in every language including English;
* **a sentence a translator is likely to get wrong**, written out in both
  languages, so that a change to the wording has to be a deliberate one.

A test that compares a label against English text is a test that fails on a
Polish machine. Where a test needs the words, it asks the catalog for them the
way the code does, and a separate test names the two languages explicitly.

## What is not translated

Anything that is not this program's own writing: a file's name, a folder's, a
person's, a game's, a repository's or a remote's; a path; a permission key; an
application id; the value of a setting as it is written to a file; and what
another program said when it failed. A `.name()` that a setting is stored under
is an identifier, not a label, and must never be translated — the display text
beside it is.
