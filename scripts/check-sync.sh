#!/usr/bin/env bash
set -euo pipefail

# Read-only synchronization audit for the two independent repositories.
#
# Usage: scripts/check-sync.sh [path-to-project-linexinbar]
#
# This never rewrites either tree. It checks the recorded source commit, every
# bundled glyph geometry, every sound and selected font byte-for-byte, then
# compares the design constants deliberately transcribed rather than shared.
# Keep each comparison named: the useful answer to drift is which contract
# moved, not merely that two large source files differ.

toolkit_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
project_arg=${1:-"$toolkit_root/../project-linexinbar"}
if ! project_root=$(cd -- "$project_arg" && pwd); then
    printf 'sync check setup error: project directory is unavailable: %s\n' \
        "$project_arg" >&2
    exit 2
fi

fail_setup() {
    printf 'sync check setup error [%s]: %s\n' "$1" "$2" >&2
    exit 2
}

fail_mismatch() {
    printf 'sync mismatch [%s]: project=%s toolkit=%s\n' "$1" "$2" "$3" >&2
    exit 1
}

require_dir() {
    [[ -d "$2" ]] || fail_setup "$1" "missing directory $2"
}

require_file() {
    [[ -f "$2" ]] || fail_setup "$1" "missing file $2"
}

# Toolkit SVGs intentionally omit the shell sources' design notes and carry
# their material marker as an XML attribute. Remove those two representation
# differences while retaining every element, attribute and authored number.
normalised_svg() {
    perl -0pe '
        s/<!--.*?-->//gs;
        s/ data-lxb-material="lxb:shape"//g;
        s/[ \t]+$//mg;
        s/^[ \t]*\n//mg;
    ' "$1"
}

# Print exactly one sed capture. Zero or multiple captures mean a source shape
# moved and should be reported as a checker setup problem, not compared as an
# empty value.
extract_one() {
    local label=$1
    local file=$2
    local expression=$3
    local found
    found=$(sed -nE "$expression" "$file")
    if [[ -z "$found" ]]; then
        fail_setup "$label" "value not found in $file"
    fi
    if [[ "$found" == *$'\n'* ]]; then
        fail_setup "$label" "value is ambiguous in $file"
    fi
    printf '%s' "$found"
}

rust_scalar() {
    extract_one "$1" "$2" \
        "s/^[[:space:]]*(pub[[:space:]]+)?const[[:space:]]+$3:[^=]+=[[:space:]]*([^;]+);[[:space:]]*$/\\2/p"
}

# A shell constant is sometimes written by naming another one —
# `const PANEL_RADIUS: f32 = lxb_protocol::pip::MENU_RADIUS as f32;` — which is
# a constant moving house rather than a value changing. Follow the name to
# wherever it is now written instead of reporting the expression as "not a
# decimal number", which is what a reader that only understood literals did the
# first time the shell did this.
rust_scalar_followed() {
    local label=$1 file=$2 name=$3
    local found compact named home depth=0
    found=$(rust_scalar "$label" "$file" "$name")
    while true; do
        compact=$(compact_number "$found")
        if [[ "$compact" =~ ^-?([0-9]+([.][0-9]*)?|[.][0-9]+)$ ]]; then
            printf '%s' "$found"
            return
        fi
        if (( depth >= 4 )); then
            fail_setup "$label" "followed $name through four names without a number"
        fi
        named=${compact%%as[a-z]*[0-9]*}
        named=${named##*::}
        if [[ ! "$named" =~ ^[A-Z][A-Z0-9_]*$ ]]; then
            printf '%s' "$found"
            return
        fi
        home=$(grep -rlE "^(pub )?const $named:" "$project_root/crates" --include='*.rs')
        if [[ -z "$home" || "$home" == *$'\n'* ]]; then
            fail_setup "$label" "$name names $named, which is not written in exactly one place"
        fi
        found=$(rust_scalar "$label" "$home" "$named")
        depth=$(( depth + 1 ))
    done
}

rust_vector() {
    extract_one "$1" "$2" \
        "s/^[[:space:]]*(pub[[:space:]]+)?const[[:space:]]+$3:[^=]+=[[:space:]]*\\[([^]]+)\\];[[:space:]]*$/\\2/p"
}

wgsl_scalar() {
    extract_one "$1" "$2" \
        "s/^[[:space:]]*const[[:space:]]+$3:[^=]+=[[:space:]]*([^;]+);[[:space:]]*$/\\1/p"
}

wgsl_vector() {
    extract_one "$1" "$2" \
        "s/^[[:space:]]*const[[:space:]]+$3:[^=]+=[[:space:]]*vec3<f32>\\(([^)]+)\\);[[:space:]]*$/\\1/p"
}

# One field of one palette literal, as six uppercase hex digits. The two
# repositories spell the wrapper differently — the shell's `Color`, the
# toolkit's `Srgb` — so the constructor is a parameter rather than an
# assumption.
palette_field() {
    local label=$1 file=$2 konst=$3 field=$4 ctor=$5
    extract_one "$label" "$file" \
        "/^pub const $konst: /,/^\};/{s/^[[:space:]]*$field: $ctor\(0x([0-9A-Fa-f]{6})\),?[[:space:]]*\$/\\1/p}"
}

# The four sky colours of one palette, comma separated. They are an array
# rather than four named fields, and the order is the contract: top, bottom,
# then the two the wallpaper crosses to.
palette_sky() {
    local label=$1 file=$2 konst=$3 ctor=$4
    local found
    found=$(sed -nE \
        "/^pub const $konst: /,/^\};/{/sky: \[/,/\],/{s/^[[:space:]]*$ctor\(0x([0-9A-Fa-f]{6})\),?[[:space:]]*\$/\1/p}}" \
        "$file" | paste -sd,)
    [[ -n "$found" ]] || fail_setup "$label" "no sky array in $konst"
    printf '%s' "$found"
}

# A duration named in the toolkit's own module.
toolkit_duration() {
    extract_one "$1" "$toolkit_motion" \
        "s/^[[:space:]]*pub const $2: f32 = ([0-9.]+);[[:space:]]*\$/\1/p"
}

# A `Duration::from_millis(N)` in the shell, as seconds — the one duration
# that is not written as an f32 of seconds on the shell's side.
millis_as_seconds() {
    local millis
    millis=$(extract_one "$1" "$2" \
        "s|^[[:space:]]*pub const $3: std::time::Duration = std::time::Duration::from_millis\(([0-9]+)\);[[:space:]]*\$|\1|p")
    awk -v ms="$millis" 'BEGIN { printf "%g", ms / 1000 }'
}

# A `Duration::from_millis(N)` or `from_secs(N)` constant, as seconds. The two
# repositories qualify the type differently and the shell writes some of these
# in whole seconds, so both the path and the unit are read rather than assumed.
duration_seconds() {
    local label=$1 file=$2 name=$3
    local found unit value
    found=$(extract_one "$label" "$file" \
        "s#^[[:space:]]*(pub[[:space:]]+)?const $name: (std::time::)?Duration = (std::time::)?Duration::from_(millis|secs)\\(([0-9]+)\\);[[:space:]]*\$#\\4 \\5#p")
    unit=${found%% *}
    value=${found##* }
    awk -v unit="$unit" -v value="$value" \
        'BEGIN { printf "%g", (unit == "millis" ? value / 1000 : value) }'
}

metric_value() {
    extract_one "$1" "$2" \
        "s/^[[:space:]]*Metric::$3[[:space:]]*=>[[:space:]]*(-?([0-9]+([.][0-9]*)?|[.][0-9]+)),[[:space:]]*$/\\1/p"
}

compact_number() {
    printf '%s' "$1" | tr -d '[:space:]'
}

compare_number() {
    local label=$1
    local project
    local toolkit
    project=$(compact_number "$2")
    toolkit=$(compact_number "$3")
    if [[ ! "$project" =~ ^-?([0-9]+([.][0-9]*)?|[.][0-9]+)$ ]]; then
        fail_setup "$label.project" "not a decimal number: $project"
    fi
    if [[ ! "$toolkit" =~ ^-?([0-9]+([.][0-9]*)?|[.][0-9]+)$ ]]; then
        fail_setup "$label.toolkit" "not a decimal number: $toolkit"
    fi
    if ! awk -v project="$project" -v toolkit="$toolkit" \
        'BEGIN { exit !((project + 0) == (toolkit + 0)) }'; then
        fail_mismatch "$label" "$project" "$toolkit"
    fi
}

compare_vector() {
    local label=$1
    local project_text=${2//[[:space:]]/}
    local toolkit_text=${3//[[:space:]]/}
    local -a project_parts
    local -a toolkit_parts
    local index
    IFS=',' read -r -a project_parts <<< "$project_text"
    IFS=',' read -r -a toolkit_parts <<< "$toolkit_text"
    if [[ ${#project_parts[@]} -ne ${#toolkit_parts[@]} ]]; then
        fail_mismatch "$label" "$project_text" "$toolkit_text"
    fi
    for index in "${!project_parts[@]}"; do
        compare_number "$label[$index]" \
            "${project_parts[$index]}" "${toolkit_parts[$index]}"
    done
}

compare_text() {
    [[ "$2" == "$3" ]] || fail_mismatch "$1" "$2" "$3"
}

normalize_domain() {
    printf '%s' "$1" \
        | sed -E 's/Self:://g; s/"//g; s/CUSTOM/Custom/g; s/[][[:space:]]//g'
}

function_clamp() {
    local label=$1
    local file=$2
    local start=$3
    local body
    local found
    body=$(sed -n "/$start/,/^}/p" "$file")
    found=$(printf '%s\n' "$body" \
        | sed -nE 's/.*[.]clamp\(([^,]+),[[:space:]]*([^)]+)\).*/\1,\2/p')
    if [[ -z "$found" || "$found" == *$'\n'* ]]; then
        fail_setup "$label" "could not isolate one clamp in $file"
    fi
    printf '%s' "$found"
}

# Fingerprint one complete top-level shader function. Internal closing braces
# are indented, so the first column-zero brace after the declaration is the end
# of the function. These fingerprints make bumping TRANSCRIBED_FROM without
# reviewing the standalone wallpaper fail loudly even when all named constants
# happened to stay put.
function_hash() {
    local label=$1
    local file=$2
    local name=$3
    local body
    body=$(sed -n "/^fn $name(/,/^}/p" "$file")
    [[ -n "$body" ]] || fail_setup "$label" "function $name not found in $file"
    printf '%s\n' "$body" | sha256sum | awk '{print $1}'
}

# One top-level Rust function, comments and whitespace taken out, for a
# function the toolkit carries whole rather than by its numbers.
rust_body_hash() {
    local label=$1
    local file=$2
    local name=$3
    local body
    body=$(sed -n "/^\(pub \)\?fn $name(/,/^}/p" "$file")
    [[ -n "$body" ]] || fail_setup "$label" "function $name not found in $file"
    printf '%s\n' "$body" \
        | sed -e 's#//.*##' -e 's/[[:space:]]//g' \
        | grep -v '^$' | sha256sum | awk '{print $1}'
}

# The body of one WGSL function, with comments, indentation and the library's
# own name prefixes taken out — so the shell's `bevel_shift` and the shipped
# `lxb_bevel_shift` hash the same when they are the same arithmetic.
shader_body_hash() {
    local label=$1
    local file=$2
    local name=$3
    local body
    body=$(sed -n "/^fn $name(/,/^}/p" "$file")
    [[ -n "$body" ]] || fail_setup "$label" "function $name not found in $file"
    printf '%s\n' "$body" \
        | sed -e 's#//.*##' -e 's/LXB_//g' -e 's/lxb_//g' -e 's/[[:space:]]//g' \
        | grep -v '^$' | sha256sum | awk '{print $1}'
}

# The same, for the wallpaper module: its names carry the module's own prefix
# as well as the library's, so the shell's `sparkle_hash` and the shipped
# `lxb_wallpaper_sparkle_hash` — and the shell's `Spine` and the shipped
# `LxbWallpaperSpine` — hash the same when they are the same arithmetic. The
# one thing the shipped module is handed rather than reading from a uniform is
# the accent, so the shell's `globals.accent` is read as that argument and the
# argument's own declaration goes.
wallpaper_body() {
    printf '%s\n' "$1" \
        | sed -e 's#//.*##' -e 's/LXB_WALLPAPER_//g' -e 's/lxb_wallpaper_//g' \
              -e 's/LxbWallpaper//g' -e 's/globals\.accent/accent/g' \
              -e 's/[[:space:]]//g' -e '/^accent:array<vec4<f32>,3>,$/d' \
        | grep -v '^$' | sha256sum | awk '{print $1}'
}

wallpaper_function_hash() {
    local label=$1
    local file=$2
    local name=$3
    local body
    body=$(sed -n "/^fn $name(/,/^}/p" "$file")
    [[ -n "$body" ]] || fail_setup "$label" "function $name not found in $file"
    wallpaper_body "$body"
}

# One top-level declaration: a constant up to its semicolon, or a structure up
# to its closing brace.
declaration() {
    local label=$1
    local file=$2
    local name=$3
    local body
    body=$(awk -v name="$name" '
        !found && index($0, "const " name ":") == 1 { found = 1 }
        found { print; if ($0 ~ /;[[:space:]]*$/) exit }' "$file")
    [[ -n "$body" ]] || body=$(awk -v name="$name" '
        !found && index($0, "struct " name " {") == 1 { found = 1 }
        found { print; if ($0 ~ /^}/) exit }' "$file")
    [[ -n "$body" ]] || fail_setup "$label" "declaration $name not found in $file"
    printf '%s\n' "$body"
}

# Every number in a declaration, in order — whole numbers too, because a seed
# is one — for a constant written as a WGSL constructor on one side and a Rust
# struct literal on the other.
declaration_numbers() {
    printf '%s\n' "$1" \
        | sed -e 's#//.*##' -e 's/vec[234]<f32>//g' -e 's/[fiu]32//g' \
        | grep -oE '[0-9][0-9_]*(\.[0-9]+)?' | tr -d '_' | tr '\n' ' '
}

# A line the file has to contain, verbatim once whitespace is normalised.
file_contains() {
    local label=$1
    local file=$2
    local wanted=$3
    local found
    found=$(sed -e 's#//.*##' -e 's/[[:space:]]\+/ /g' -e 's/^ //' -e 's/ $//' "$file" \
        | grep -F -c -- "$wanted" || true)
    [[ "$found" -ge 1 ]] \
        || fail_mismatch "$label" "$wanted" "no such line in $file"
}

# The same, inside one function — a top-level one, or a method one level in —
# for a line that is only the rule where it is written: the colour a menu's
# title is written in, say, which the rest of a file uses for other things.
function_contains() {
    local label=$1
    local file=$2
    local function=$3
    local wanted=$4
    local body
    body=$(sed -n "/^\(pub \)\?fn $function(/,/^}/p" "$file")
    [[ -n "$body" ]] \
        || body=$(sed -n "/^    \(pub \)\?fn $function(/,/^    }/p" "$file")
    [[ -n "$body" ]] || fail_setup "$label" "function $function not found in $file"
    # Normalised into a variable first: `grep -q` stops reading at the first
    # match, and under pipefail the writer it leaves behind fails the pipe.
    body=$(printf '%s\n' "$body" \
        | sed -e 's#//.*##' -e 's/[[:space:]]\+/ /g' -e 's/^ //' -e 's/ $//')
    grep -qF -- "$wanted" <<< "$body" \
        || fail_mismatch "$label" "$wanted" "not in $function in $file"
}

# The same, inside one arm of a match in one function: from the line that opens
# the arm up to the line that opens the next. For a line written in more than
# one arm, as the shell's `self.held_bar = None;` is in a leave's and a
# release's, where only one of them is the rule being pinned. The arm is read to
# the end rather than left early, for the reason given above.
arm_contains() {
    local label=$1
    local file=$2
    local function=$3
    local arm=$4
    local next=$5
    local wanted=$6
    local body
    body=$(sed -n "/^\(pub \)\?fn $function(/,/^}/p" "$file")
    [[ -n "$body" ]] \
        || body=$(sed -n "/^    \(pub \)\?fn $function(/,/^    }/p" "$file")
    [[ -n "$body" ]] || fail_setup "$label" "function $function not found in $file"
    body=$(printf '%s\n' "$body" \
        | sed -e 's#//.*##' -e 's/[[:space:]]\+/ /g' -e 's/^ //' -e 's/ $//' \
        | awk -v arm="$arm" -v next_arm="$next" '
            open && index($0, next_arm) == 1 { open = 0; done = 1 }
            !done && index($0, arm) == 1 { open = 1 }
            open { print }')
    [[ -n "$body" ]] || fail_setup "$label" "no arm $arm in $function in $file"
    grep -qF -- "$wanted" <<< "$body" \
        || fail_mismatch "$label" "$wanted" "not in $arm in $function in $file"
}

# The variants of one top-level enum, in order, each by its name and payload.
enum_variants() {
    local label=$1
    local file=$2
    local name=$3
    local body
    body=$(sed -n "/^\(pub \)\?enum $name {/,/^}/p" "$file")
    [[ -n "$body" ]] || fail_setup "$label" "enum $name not found in $file"
    printf '%s\n' "$body" \
        | sed -e '1d' -e '$d' -e 's#//.*##' -e '/^[[:space:]]*#\[/d' \
              -e 's/[[:space:]]//g' -e 's/,$//' -e '/^$/d' \
        | tr '\n' ' ' | sed 's/ $//'
}

# Every line of a file above its tests that sets one field, in order and
# normalised, joined by " | ".
assignments() {
    local label=$1
    local file=$2
    local field=$3
    require_file "$label" "$file"
    sed -n '1,/^mod tests {/p' "$file" \
        | sed -e 's#//.*##' -e 's/[[:space:]]\+/ /g' -e 's/^ //' -e 's/ $//' \
        | { grep -F -- ".$field = " || true; } \
        | sed ':a;N;$!ba;s/\n/ | /g'
}

# A line the file must not contain: for something the toolkit adds that the
# shell has no counterpart to, so the day the shell grows one the two are
# compared rather than left to be two answers. The file has to be there: grep
# answers a missing one as "not found", and a shader renamed away would then
# pass this for ever without anything in it being looked at.
file_lacks() {
    local label=$1
    local file=$2
    local unwanted=$3
    require_file "$label" "$file"
    if grep -qF -- "$unwanted" "$file"; then
        fail_mismatch "$label" "$unwanted is now in $file" \
            'an addition with nothing of the shell to compare it to'
    fi
}

function_calls_once() {
    local label=$1
    local file=$2
    local function=$3
    local call=$4
    local body
    local after
    body=$(sed -n "/^pub fn $function(/,/^}/p" "$file")
    [[ -n "$body" ]] || fail_setup "$label" "function $function not found in $file"
    [[ "$body" == *"$call"* ]] \
        || fail_mismatch "$label" "$function calls $call" "call is absent"
    after=${body#*"$call"}
    [[ "$after" != *"$call"* ]] \
        || fail_mismatch "$label" "$function calls $call once" "call is repeated"
}

# Every number one function body uses, as a set.
#
# The CPU transcription in `paint.rs` cannot be hashed against the WGSL it was
# taken from — one is Rust and the other is a shading language — but the
# numbers in it can be, and those are what a transcription loses. A term
# dropped, a coefficient nudged, a threshold moved or a `mix` reversed all show
# up here. Only swapping one value for another the same function already uses
# does not, which is why the shaders are still hashed against the shell as
# well.
#
# Type names go first so `f32` is not a number, and a `powi(5)` is written the
# way the shader writes it before anything is counted. The one line the two
# genuinely cannot share goes too: the shader is handed the wallpaper style as
# a float and tests a neighbourhood of it, where the transcription is handed
# the enumeration and matches on that. Nothing else in either file reads a
# style as a number. Rust's range operator is opened out as well, or `0..3`
# reads as the number nought.
numbers_in() {
    local body=$1
    printf '%s\n' "$body" \
        | sed -e '/style > 0.5 && style < 1.5/d' \
              -e 's#//.*##' \
              -e 's/\.\./ /g' \
              -e 's/f32//g' -e 's/i32//g' -e 's/u32//g' -e 's/f64//g' \
              -e 's/\.powi(\([0-9]\+\))/.pow(\1.0)/g' \
        | grep -oE '[0-9]+\.[0-9]*(e-?[0-9]+)?|[0-9]+e-[0-9]+' \
        | sort -u | tr '\n' ' '
}

wgsl_numbers() {
    local label=$1
    local file=$2
    local name=$3
    local body
    body=$(sed -n "/^fn $name(/,/^}/p" "$file")
    [[ -n "$body" ]] || fail_setup "$label" "function $name not found in $file"
    numbers_in "$body"
}

# One or more Rust functions, taken together: the shipped shader packs into one
# function what the transcription splits over two.
rust_numbers() {
    local label=$1
    local file=$2
    shift 2
    local body=""
    local name
    for name in "$@"; do
        local one
        one=$(sed -n "/^\(pub \)\?fn $name(/,/^}/p" "$file")
        # A method reached through `impl` is indented, so both depths are
        # looked for before giving up.
        [[ -n "$one" ]] \
            || one=$(sed -n "/^    \(pub \)\?fn $name(/,/^    }/p" "$file")
        [[ -n "$one" ]] || fail_setup "$label" "function $name not found in $file"
        body+="$one"$'\n'
    done
    numbers_in "$body"
}

project_glyphs="$project_root/crates/lxb-desktop/src/glyphs"
toolkit_glyphs="$toolkit_root/crates/lxb-toolkit/assets/glyphs"
project_sounds="$project_root/crates/lxb-desktop/src/sounds"
toolkit_sounds="$toolkit_root/crates/lxb-toolkit/assets/sounds"
project_icons="$project_root/crates/lxb-desktop/src/icons.rs"
project_gpu="$project_root/crates/lxb-desktop/src/gpu.rs"
project_shader="$project_root/crates/lxb-desktop/src/shaders.wgsl"
project_ui="$project_root/crates/lxb-desktop/src/ui.rs"
project_menu="$project_root/crates/lxb-desktop/src/menu.rs"
project_theme="$project_root/crates/lxb-protocol/src/wallpaper.rs"
toolkit_lib="$toolkit_root/crates/lxb-toolkit/src/lib.rs"
toolkit_glyph_material="$toolkit_root/crates/lxb-toolkit/src/glyph_material.rs"
toolkit_material="$toolkit_root/crates/lxb-toolkit/src/material.rs"
toolkit_metrics="$toolkit_root/crates/lxb-toolkit/src/metrics.rs"
toolkit_menu="$toolkit_root/crates/lxb-toolkit/src/menu.rs"
toolkit_components="$toolkit_root/crates/lxb-render/src/components.rs"
project_ui="$project_root/crates/lxb-desktop/src/ui.rs"
toolkit_control="$toolkit_root/crates/lxb-toolkit/src/control.rs"
toolkit_settings="$toolkit_root/crates/lxb-toolkit/src/settings.rs"
toolkit_wallpaper="$toolkit_root/crates/lxb-toolkit/src/wallpaper.rs"
toolkit_wallpaper_shader="$toolkit_root/crates/lxb-toolkit/assets/shaders/lxb_wallpaper.wgsl"
toolkit_glass_shader="$toolkit_root/crates/lxb-toolkit/assets/shaders/lxb_glass.wgsl"
toolkit_glyph_shader="$toolkit_root/crates/lxb-toolkit/assets/shaders/lxb_glyph.wgsl"
toolkit_paint="$toolkit_root/crates/lxb-toolkit/src/paint.rs"
toolkit_render_ui="$toolkit_root/crates/lxb-render/src/ui.wgsl"
toolkit_renderer="$toolkit_root/crates/lxb-render/src/renderer.rs"
project_palettes="$project_root/crates/lxb-desktop/src/theme.rs"
project_launch="$project_root/crates/lxb-desktop/src/launch.rs"
project_menu="$project_root/crates/lxb-desktop/src/menu.rs"
project_guide="$project_root/crates/lxb-desktop/src/guide.rs"
project_notify="$project_root/crates/lxb-desktop/src/notify.rs"
project_volume="$project_root/crates/lxb-desktop/src/volume.rs"
project_keyboard="$project_root/crates/lxb-desktop/src/keyboard.rs"
project_main="$project_root/crates/lxb-desktop/src/main.rs"
project_overview="$project_root/crates/lxb-protocol/src/overview.rs"
project_sound_source="$project_root/crates/lxb-desktop/src/sound.rs"
toolkit_palette="$toolkit_root/crates/lxb-toolkit/src/palette.rs"
toolkit_motion="$toolkit_root/crates/lxb-toolkit/src/motion.rs"
toolkit_layout="$toolkit_root/crates/lxb-toolkit/src/layout.rs"
toolkit_board="$toolkit_root/crates/lxb-toolkit/src/board.rs"
toolkit_sound_source="$toolkit_root/crates/lxb-toolkit/src/sound.rs"

require_dir assets.glyphs.project "$project_glyphs"
require_dir assets.glyphs.toolkit "$toolkit_glyphs"
require_dir assets.sounds.project "$project_sounds"
require_dir assets.sounds.toolkit "$toolkit_sounds"
for required in \
    "$project_icons" "$project_gpu" "$project_shader" "$project_ui" \
    "$project_theme" "$toolkit_lib" "$toolkit_glyph_material" \
    "$toolkit_material" "$toolkit_metrics" "$toolkit_settings" \
    "$toolkit_control" "$toolkit_glass_shader" \
    "$toolkit_wallpaper" "$toolkit_wallpaper_shader" \
    "$project_palettes" "$project_launch" "$project_menu" "$project_guide" \
    "$project_notify" "$project_volume" "$project_keyboard" "$project_main" \
    "$project_overview" "$project_sound_source" \
    "$toolkit_palette" "$toolkit_motion" "$toolkit_layout" "$toolkit_sound_source" \
    "$toolkit_board"; do
    require_file source "$required"
done

head=$(git -C "$project_root" rev-parse --short=7 HEAD)
recorded=$(extract_one source.commit "$toolkit_lib" \
    's/^pub const TRANSCRIBED_FROM: &str = "([^"]*)";/\1/p')
compare_text source.commit "$head" "$recorded"

# Glyph SDF encoding and the two materials drawn from it.
compare_number glyph.sdf-range \
    "$(rust_scalar glyph.sdf-range.project "$project_icons" SDF_RANGE)" \
    "$(rust_scalar glyph.sdf-range.toolkit "$toolkit_glyph_material" SDF_RANGE)"
compare_number glyph.sdf-supersample \
    "$(rust_scalar glyph.sdf-supersample.project "$project_icons" SDF_SUPERSAMPLE)" \
    "$(rust_scalar glyph.sdf-supersample.toolkit "$toolkit_glyph_material" SDF_SUPERSAMPLE)"
compare_number glyph.cell \
    "$(rust_scalar glyph.cell.project "$project_gpu" CELL)" \
    "$(rust_scalar glyph.cell.toolkit "$toolkit_glyph_material" CELL)"
compare_number glyph.depth-share \
    "$(rust_scalar glyph.depth-share.project "$project_ui" GLYPH_DEPTH)" \
    "$(rust_scalar glyph.depth-share.toolkit "$toolkit_glyph_material" DEPTH_SHARE)"
compare_vector glyph.lamp \
    "$(wgsl_vector glyph.lamp.project "$project_shader" GLYPH_LAMP)" \
    "$(rust_vector glyph.lamp.toolkit "$toolkit_glyph_material" LAMP)"
compare_number glyph.shadow \
    "$(wgsl_scalar glyph.shadow.project "$project_shader" GLYPH_SHADOW)" \
    "$(rust_scalar glyph.shadow.toolkit "$toolkit_glyph_material" SHADOW)"
compare_number glyph.simple-tint \
    "$(wgsl_scalar glyph.simple-tint.project "$project_shader" GLYPH_FLAT_TINT)" \
    "$(rust_scalar glyph.simple-tint.toolkit "$toolkit_glyph_material" SIMPLE_TINT)"
compare_number glyph.simple-alpha \
    "$(wgsl_scalar glyph.simple-alpha.project "$project_shader" GLYPH_FLAT_ALPHA)" \
    "$(rust_scalar glyph.simple-alpha.toolkit "$toolkit_glyph_material" SIMPLE_ALPHA)"
compare_number glyph.simple-stain \
    "$(wgsl_scalar glyph.simple-stain.project "$project_shader" GLYPH_FLAT_STAIN)" \
    "$(rust_scalar glyph.simple-stain.toolkit "$toolkit_glyph_material" SIMPLE_STAIN)"

project_gradient_arm=$(extract_one glyph.gradient-arm.project "$project_shader" \
    's/.*let arm = texel \* ([0-9.]+);/\1/p')
compare_number glyph.gradient-arm "$project_gradient_arm" \
    "$(rust_scalar glyph.gradient-arm.toolkit "$toolkit_glyph_material" GRADIENT_ARM)"
project_coverage=$(extract_one glyph.coverage-feather.project "$project_shader" \
    's/^[[:space:]]*let coverage = 1[.]0 - smoothstep\(-([0-9.]+),[[:space:]]*([0-9.]+), d\);/\1,\2/p')
toolkit_coverage=$(rust_scalar glyph.coverage-feather.toolkit \
    "$toolkit_glyph_material" COVERAGE_FEATHER)
compare_vector glyph.coverage-feather "$project_coverage" \
    "$toolkit_coverage,$toolkit_coverage"
project_min_depth=$(extract_one glyph.min-depth.project "$project_shader" \
    's/.*let slab = max\(in[.]material[.]x,[[:space:]]*([0-9.]+)\);/\1/p')
compare_number glyph.min-depth "$project_min_depth" \
    "$(rust_scalar glyph.min-depth.toolkit "$toolkit_glyph_material" MIN_DEPTH)"
project_ridge=$(extract_one glyph.ridge.project "$project_shader" \
    's/.*let ridge = smoothstep\(([0-9.]+),[[:space:]]*([0-9.]+), slope\);/\1,\2/p')
compare_vector glyph.ridge "$project_ridge" \
    "$(rust_scalar glyph.ridge-begin.toolkit "$toolkit_glyph_material" RIDGE_BEGIN),$(rust_scalar glyph.ridge-end.toolkit "$toolkit_glyph_material" RIDGE_END)"
project_shadow_offset=$(extract_one glyph.shadow-offset.project "$project_shader" \
    's/.*normalize\(GLYPH_LAMP[.]xy\) \* slab \* ([0-9.]+);/\1/p')
compare_number glyph.shadow-offset "$project_shadow_offset" \
    "$(rust_scalar glyph.shadow-offset.toolkit "$toolkit_glyph_material" SHADOW_OFFSET)"

# Glass optics exported to renderers by the toolkit.
compare_vector glass.key-light \
    "$(wgsl_vector glass.key-light.project "$project_shader" KEY_LIGHT)" \
    "$(rust_vector glass.key-light.toolkit "$toolkit_material" KEY_LIGHT)"
for mapping in \
    'ior GLASS_IOR IOR' \
    'dispersion GLASS_DISPERSION DISPERSION' \
    'float GLASS_FLOAT FLOAT' \
    'blur-levels BACKDROP_LEVELS BLUR_LEVELS'; do
    read -r label project_name toolkit_name <<< "$mapping"
    compare_number "glass.$label" \
        "$(wgsl_scalar "glass.$label.project" "$project_shader" "$project_name")" \
        "$(rust_scalar "glass.$label.toolkit" "$toolkit_material" "$toolkit_name")"
done
compare_vector glass.frost-scatter \
    "$(wgsl_vector glass.frost-scatter.project "$project_shader" FROST_SCATTER)" \
    "$(rust_vector glass.frost-scatter.toolkit "$toolkit_material" FROST_SCATTER)"
project_max_slab=$(extract_one glass.max-slab-share.project "$project_shader" \
    's/.*let slab = min\(thickness, min\(in[.]half_size[.]x, in[.]half_size[.]y\) \* ([0-9.]+)\);/\1/p')
compare_number glass.max-slab-share "$project_max_slab" \
    "$(rust_scalar glass.max-slab-share.toolkit "$toolkit_material" MAX_SLAB_SHARE)"

# The optics themselves, function by function. The shipped shader is the
# shell's model transcribed, so the arithmetic has to be the same arithmetic —
# not merely the same constants fed to something that has quietly drifted.
for mapping in \
    'rounded-box rounded_box lxb_rounded_box' \
    'edge-normal edge_normal lxb_edge_normal' \
    'bevel-rise bevel_rise lxb_bevel_rise' \
    'bevel-slope bevel_slope lxb_bevel_slope' \
    'bevel-shift bevel_shift lxb_bevel_shift' \
    'environment environment lxb_environment'; do
    read -r label project_name toolkit_name <<< "$mapping"
    compare_text "glass.arithmetic.$label" \
        "$(shader_body_hash "glass.arithmetic.$label.project" "$project_shader" "$project_name")" \
        "$(shader_body_hash "glass.arithmetic.$label.toolkit" "$toolkit_glass_shader" "$toolkit_name")"
done

# The pane's own body is inline in the shell's fragment shader and a pair of
# functions in the shipped one, so it cannot be hashed as a unit. These are the
# two lines of it that have already been got wrong once: the broad reflection
# is a property of the *pane* — the curve its caller asked for — and not of the
# pixel. Read back out of the surface normal instead, a flat pane and the
# middle of a bowed one are indistinguishable, and every broad sheet comes out
# washed with the key light at nearly full strength.
file_contains glass.broad-face.project "$project_shader" \
    'let broad_face = smoothstep(0.68, 1.0, inset)'
file_contains glass.broad-face.project-curve "$project_shader" \
    '* clamp(in.face_curve, 0.0, 1.0);'
file_contains glass.broad-face.project-key "$project_shader" \
    'let key_strength = mix(1.0, 0.10, broad_face);'
file_contains glass.broad-face.toolkit "$toolkit_glass_shader" \
    'let broad_face = smoothstep(0.68, 1.0, pane.inset)'
file_contains glass.broad-face.toolkit-curve "$toolkit_glass_shader" \
    '* clamp(pane.curve, 0.0, 1.0);'
file_contains glass.broad-face.toolkit-key "$toolkit_glass_shader" \
    'let key_strength = mix(1.0, 0.10, broad_face);'

# --- the panel's shape ------------------------------------------------------
#
# The context menu is the one component this library draws rather than merely
# describes, so its geometry has to be the shell's and not merely its
# neighbourhood: where the panel settles beside its anchor, how tall a row is,
# what a change of band opens, and how far a row opens out under the highlight.
for mapping in \
    'menu.margin GUIDE_MARGIN MARGIN' \
    'menu.badge CHOSEN_BADGE BADGE' \
    'menu.badge-at CHOSEN_BADGE_AT BADGE_AT' \
    'menu.panel-in DIALOG_PANEL_IN PANEL_IN'; do
    read -r label project_name toolkit_name <<< "$mapping"
    compare_number "$label" \
        "$(rust_scalar "$label.project" "$project_ui" "$project_name")" \
        "$(rust_scalar "$label.toolkit" "$toolkit_menu" "$toolkit_name")"
done

# The two the toolkit does not write down twice — they are the metric and the
# control's own padding, read through — so they are compared to the shell's
# by value rather than by where they are written.
compare_number menu.label-padding \
    "$(rust_scalar menu.label-padding.project "$project_ui" GUIDE_LABEL_PADDING)" \
    "$(extract_one menu.label-padding.toolkit "$toolkit_metrics" \
        's/^[[:space:]]*Metric::RowPadding => ([0-9.]+),[[:space:]]*$/\1/p')"
compare_number menu.row-padding \
    "$(rust_scalar menu.row-padding.project "$project_ui" GUIDE_ROW_PADDING)" \
    "$(rust_scalar menu.row-padding.toolkit "$toolkit_control" PADDING)"

# And the arithmetic, function by function, by the numbers each one uses. The
# shell's are threaded through its own `Menu`, so they cannot be hashed against
# the toolkit's — which is handed the rows — but a coefficient nudged, a term
# dropped or a threshold moved shows up here just the same.
for mapping in \
    "settle context_menu_rect place" \
    "rows context_menu_row_rect row" \
    "chip context_menu_chip_rect chip" \
    "aside context_menu_aside_rect aside" \
    "separators context_separator_rects separators" \
    "opening entry_opening opening" \
    "row-height context_row_height row_height" \
    "settled context_row_settled row_settled" \
    "lines lines_in lines_in" \
    "room column_room column_room" \
    "grown context_menu_bounds growing" \
    "rows-top context_rows_top top" \
    "span context_row_span span" \
    "chip-height context_chip_height chip_height" \
    "aside-width context_aside_width aside_width"; do
    read -r label project_name toolkit_name <<< "$mapping"
    compare_text "menu.arithmetic.$label" \
        "$(rust_numbers "menu.arithmetic.$label.project" "$project_ui" "$project_name")" \
        "$(rust_numbers "menu.arithmetic.$label.toolkit" "$toolkit_menu" "$toolkit_name")"
done

# The one rule a numbers comparison cannot see, because it is about which side
# of a subtraction the anchor is on: the panel stands *beside* the control it
# is about and never over it. The anchor is the only context the panel has.
file_contains menu.beside.project "$project_ui" \
    'let right = ax + aw + gap;'
file_contains menu.beside.project-flip "$project_ui" \
    '(ax - gap - panel_w).max(inset)'
file_contains menu.beside.toolkit "$toolkit_menu" \
    'let right = ax + aw + gap;'
file_contains menu.beside.toolkit-flip "$toolkit_menu" \
    '(ax - gap - panel_w).max(inset)'

# How round a row's chip and its button are. Written inline in the shell and
# named here, so the number is compared rather than the line.
project_aside_radius=$(sed -nE \
    's/^[[:space:]]*context_chip_height\(entry\) \* scale \* ([0-9.]+)$/\1/p' \
    "$project_ui" | head -1)
compare_number menu.aside-radius "$project_aside_radius" \
    "$(rust_scalar menu.aside-radius.toolkit "$toolkit_menu" ASIDE_RADIUS)"

# --- the shaders, on the processor ------------------------------------------
#
# `paint.rs` draws the three shipped WGSL modules over a buffer of pixels, for
# a consumer with a painter and no renderer. It is a transcription, and a
# transcription that drifts is worse than none: two programs written against
# this library would then disagree about what the material is depending on
# which one of them had a GPU. The constants are read out of the same Rust
# modules the shaders' own are checked against, so what is left to check is the
# arithmetic — function by function, by the numbers each one uses.
for mapping in \
    "corner-norm $toolkit_glass_shader lxb_corner_norm corner_norm" \
    "rounded-box $toolkit_glass_shader lxb_rounded_box rounded_box" \
    "edge-normal $toolkit_glass_shader lxb_edge_normal edge_normal" \
    "bevel-rise $toolkit_glass_shader lxb_bevel_rise bevel_rise" \
    "bevel-slope $toolkit_glass_shader lxb_bevel_slope bevel_slope" \
    "bevel-shift $toolkit_glass_shader lxb_bevel_shift bevel_shift" \
    "environment $toolkit_glass_shader lxb_environment environment" \
    "pane $toolkit_glass_shader lxb_glass_begin_curved pane_pixel bevel_at" \
    "shade $toolkit_glass_shader lxb_glass_shade pane_shade" \
    "scene $toolkit_wallpaper_shader lxb_wallpaper sample" \
    "water $toolkit_wallpaper_shader lxb_wallpaper_water water" \
    "silk $toolkit_wallpaper_shader lxb_wallpaper_silk silk" \
    "field $toolkit_wallpaper_shader lxb_wallpaper_ambient_field ambient_field" \
    "spine $toolkit_wallpaper_shader lxb_wallpaper_spine_at spine_at" \
    "silk-spine $toolkit_wallpaper_shader lxb_wallpaper_silk_spine silk_spine" \
    "sparkle-unit $toolkit_wallpaper_shader lxb_wallpaper_sparkle_unit sparkle_unit" \
    "sparkle-pushed $toolkit_wallpaper_shader lxb_wallpaper_sparkle_pushed sparkle_pushed" \
    "sparkle-unpushed $toolkit_wallpaper_shader lxb_wallpaper_sparkle_unpushed sparkle_unpushed" \
    "sparkle-hold $toolkit_wallpaper_shader lxb_wallpaper_sparkle_hold sparkle_hold" \
    "sparkle-unheld $toolkit_wallpaper_shader lxb_wallpaper_sparkle_unheld sparkle_unheld" \
    "sparkle-bump $toolkit_wallpaper_shader lxb_wallpaper_sparkle_bump sparkle_bump" \
    "sparkle-half $toolkit_wallpaper_shader lxb_wallpaper_sparkle_half sparkle_half" \
    "sparkle-layer $toolkit_wallpaper_shader lxb_wallpaper_sparkle_layer sparkle_layer" \
    "sparkles $toolkit_wallpaper_shader lxb_wallpaper_sparkles sparkles" \
    "glyph-simple $toolkit_glyph_shader lxb_glyph_simple glyph_simple" \
    "glyph-default $toolkit_glyph_shader lxb_glyph_default glyph_default" \
    "glyph-coverage $toolkit_glyph_shader lxb_glyph_coverage coverage_of"; do
    read -r label shader shader_name rust_names <<< "$mapping"
    # shellcheck disable=SC2086 — the trailing field is one or two names.
    compare_text "paint.$label" \
        "$(wgsl_numbers "paint.$label.shader" "$shader" "$shader_name")" \
        "$(rust_numbers "paint.$label.cpu" "$toolkit_paint" $rust_names)"
done

# The soft lift under a pane and behind a selection is lxb-render's own quad
# rather than one of the three modules, so it is pinned by its two numbers: a
# gaussian, and the seal that stops it drawing a visible disc.
file_contains paint.light.gaussian "$toolkit_paint" \
    'let gaussian = (-5.5 * radius * radius).exp();'
file_contains paint.light.sealed "$toolkit_paint" \
    'let sealed = ((1.0 - radius) / 0.12).clamp(0.0, 1.0);'
file_contains paint.light.renderer "$toolkit_render_ui" \
    'let gaussian = exp(-5.5 * radius * radius);'
file_contains paint.light.renderer-seal "$toolkit_render_ui" \
    'let sealed = clamp((1.0 - radius) / 0.12, 0.0, 1.0);'

# The current context menu and general dialog are not cut from the dense
# Surface::Panel recipe. Both call the sidebar's four-layer surface directly:
# two quiet lights, one curved live pane, and one hairline. Compare the named
# material values and the source relationship so neither overlay can silently
# fall back to the old compact-panel approximation.
for mapping in \
    'stain SIDEBAR_STAIN SIDEBAR_STAIN' \
    'header-light SIDEBAR_HEADER_LIGHT SIDEBAR_HEADER' \
    'foot-light SIDEBAR_FOOT_LIGHT SIDEBAR_FOOT' \
    'rim SIDEBAR_RIM SIDEBAR_RIM'; do
    read -r label project_name toolkit_name <<< "$mapping"
    compare_number "overlay.$label" \
        "$(rust_scalar "overlay.$label.project" "$project_ui" "$project_name")" \
        "$(rust_scalar "overlay.$label.toolkit" "$toolkit_material" "$toolkit_name")"
done
function_calls_once overlay.context-menu.material "$project_ui" \
    build_context_menu 'panel.quads.extend(sidebar_surface('
function_calls_once overlay.dialog.material "$project_ui" \
    build_dialog 'panel.quads.extend(sidebar_surface('
compare_text overlay.sidebar-surface.arithmetic \
    "$(function_hash overlay.sidebar-surface.arithmetic "$project_ui" sidebar_surface)" \
    'b67bc47dbc697afacee775b148ebb7c725b7a52c7ef5d74dc7869072ed917a52'

# Screen scaling and the three different panel cuts applications can reuse.
compare_number scale.reference-height \
    "$(rust_scalar scale.reference-height.project "$project_ui" REFERENCE_HEIGHT)" \
    "$(rust_scalar scale.reference-height.toolkit "$toolkit_metrics" REFERENCE_HEIGHT)"
compare_vector scale.clamp \
    "$(function_clamp scale.clamp.project "$project_ui" '^fn guide_scale')" \
    "$(function_clamp scale.clamp.toolkit "$toolkit_metrics" '^pub fn scale_for')"
for mapping in \
    'context-width CONTEXT_WIDTH MenuWidth' \
    'card-radius CARD_RADIUS CardRadius' \
    'panel-radius PANEL_RADIUS PanelRadius' \
    'panel-inset PANEL_INSET PanelInset' \
    'row-height GUIDE_ROW_HEIGHT RowHeight' \
    'row-padding GUIDE_LABEL_PADDING RowPadding' \
    'panel-padding GUIDE_PADDING PanelPadding' \
    'tile GUIDE_TILE Tile' \
    'gap GUIDE_TILE_GAP Gap' \
    'tile-glyph GUIDE_TILE_GLYPH TileGlyph' \
    'tile-radius GUIDE_TILE_RADIUS TileRadius' \
    'item-spacing ITEM_SPACING ItemSpacing' \
    'column-spacing CATEGORY_SPACING ColumnSpacing' \
    'item-icon ITEM_ICON ItemIcon' \
    'item-icon-focused ITEM_ICON_FOCUSED ItemIconFocused' \
    'column-icon CATEGORY_ICON ColumnIcon' \
    'column-icon-focused CATEGORY_ICON_FOCUSED ColumnIconFocused' \
    'dialog-width DIALOG_WIDTH DialogWidth' \
    'dialog-dim DIALOG_DIM DialogDim' \
    'power-width POWER_WIDTH PowerWidth' \
    'power-dim POWER_DIM PowerDim'; do
    read -r label project_name toolkit_name <<< "$mapping"
    compare_number "metric.$label" \
        "$(rust_scalar_followed "metric.$label.project" "$project_ui" "$project_name")" \
        "$(metric_value "metric.$label.toolkit" "$toolkit_metrics" "$toolkit_name")"
done

# The file chooser's own shape. It is drawn by lxb-render rather than by the
# values crate, so the numbers live beside the drawing — which is exactly the
# kind of place a transcribed number drifts unwatched. What is compared is the
# panel's proportions and every band inside it, because a chooser with the
# shell's material and its own margins is not the shell's chooser.
project_picker="$project_root/crates/lxb-desktop/src/picker.rs"
require_file picker.project.source "$project_picker"
compare_number picker.share \
    "$(rust_scalar picker.share.project "$project_picker" SHARE)" \
    "$(rust_scalar picker.share.toolkit "$toolkit_components" PICKER_WINDOW_WIDTH)"
compare_number picker.share.square \
    "$(rust_scalar picker.share.square.project "$project_picker" SHARE)" \
    "$(rust_scalar picker.share.square.toolkit "$toolkit_components" PICKER_WINDOW_HEIGHT)"
for mapping in \
    'margin GUIDE_MARGIN PICKER_MARGIN' \
    'head PICKER_HEAD PICKER_HEAD' \
    'foot PICKER_FOOT PICKER_FOOT' \
    'where PICKER_WHERE PICKER_WHERE' \
    'cross-x PICKER_CROSS_X PICKER_CROSS_X' \
    'cross-y PICKER_CROSS_Y PICKER_CROSS_Y' \
    'hint-glyph PICKER_HINT_GLYPH PICKER_HINT_GLYPH' \
    'hint-label PICKER_HINT_LABEL PICKER_HINT_LABEL' \
    'hint-gap PICKER_HINT_GAP PICKER_HINT_GAP' \
    'hint-step PICKER_HINT_STEP PICKER_HINT_STEP'; do
    read -r label project_name toolkit_name <<< "$mapping"
    compare_number "picker.$label" \
        "$(rust_scalar_followed "picker.$label.project" "$project_ui" "$project_name")" \
        "$(rust_scalar "picker.$label.toolkit" "$toolkit_components" "$toolkit_name")"
done

# The context menu is a component rather than a value: the material says what
# the panel is made of and these say what shape it is. Transcribed like
# everything else here, so checked like everything else here — a panel drawn in
# the right material at the wrong row height is not this menu.
# The panel's width is not in this list: the toolkit defines it once, as
# Metric::MenuWidth, and menu::WIDTH reads it back rather than repeating it —
# so it is checked above as metric.context-width, and the toolkit's own unit
# test is what keeps the two from becoming two numbers.
for mapping in \
    'extra-width NOTIFICATION_EXTRA_WIDTH EXTRA_WIDTH' \
    'row CONTEXT_ROW ROW' \
    'stacked-row MIXER_ROW STACKED_ROW' \
    'title CONTEXT_TITLE TITLE' \
    'title-size CONTEXT_TITLE_SIZE TITLE_SIZE' \
    'label-size CONTEXT_LABEL_SIZE LABEL_SIZE' \
    'detail-size CONTEXT_DETAIL_SIZE DETAIL_SIZE' \
    'stamp-size CONTEXT_STAMP_SIZE STAMP_SIZE' \
    'max-lines CONTEXT_MAX_LINES MAX_LINES' \
    'group-gap CONTEXT_GROUP_GAP GROUP_GAP' \
    'gap CONTEXT_GAP GAP' \
    'glow-reach CONTEXT_GLOW_REACH GLOW_REACH' \
    'scroll-strip CONTEXT_SCROLL_STRIP SCROLL_STRIP' \
    'scroll-arrow CONTEXT_SCROLL_ARROW SCROLL_ARROW' \
    'dim CONTEXT_DIM DIM' \
    'scrim CONTEXT_SCRIM SCRIM' \
    'depth CONTEXT_DEPTH DEPTH' \
    'content-in CONTEXT_CONTENT_IN CONTENT_IN' \
    'icon CONTEXT_ICON ICON' \
    'glyph CONTEXT_GLYPH GLYPH' \
    'aside CONTEXT_ASIDE ASIDE' \
    'aside-gap CONTEXT_ASIDE_GAP ASIDE_GAP' \
    'aside-glyph CONTEXT_ASIDE_GLYPH ASIDE_GLYPH'; do
    read -r label project_name toolkit_name <<< "$mapping"
    compare_number "menu.$label" \
        "$(rust_scalar "menu.$label.project" "$project_ui" "$project_name")" \
        "$(rust_scalar "menu.$label.toolkit" "$toolkit_menu" "$toolkit_name")"
done

# How the panel arrives. `context_menu_bounds` is a `pub fn`, so the body hash
# above cannot reach it, and the arithmetic is the whole component: the panel
# keeps its *own* shape and only its scale and its centre move. Interpolating
# the two rectangles instead — which is what this was, and the obvious way to
# write it — carries the panel through the anchor's proportions, so a wide flat
# button visibly reshapes into a tall menu on its way out of itself.
for mapping in \
    "factor $project_ui 'let factor = lerp((aw / pw).min(1.0), 1.0, progress);'" \
    "centre-x $project_ui 'let cx = lerp(ax + aw * 0.5, px + pw * 0.5, progress);'" \
    "centre-y $project_ui 'let cy = lerp(ay + ah * 0.5, py + ph * 0.5, progress);'" \
    "factor.toolkit $toolkit_menu 'let factor = lerp((aw / pw).min(1.0), 1.0);'" \
    "centre-x.toolkit $toolkit_menu 'let cx = lerp(ax + aw * 0.5, px + pw * 0.5);'" \
    "centre-y.toolkit $toolkit_menu 'let cy = lerp(ay + ah * 0.5, py + ph * 0.5);'"; do
    eval "set -- $mapping"
    file_contains "menu.growing.$1" "$2" "$3"
done

# The control everything you can act on is cut from. Only the shell's *named*
# numbers can be compared here — the tints and alphas are written inline at
# each call site in ui.rs — but the ones that are named are the ones a change
# would move: how deep the press goes, how long it spends going down, how much
# air a chip leaves in its row, how quietly it takes the light, and how stiff
# the spring under a gliding highlight is.
for mapping in \
    "padding $project_ui GUIDE_ROW_PADDING $toolkit_control PADDING" \
    "gloss-quiet $project_ui GLOSS_QUIET $toolkit_material GLOSS_QUIET" \
    "press-dip $project_ui PRESS_DIP $toolkit_motion PRESS_DIP" \
    "press-bounce $project_ui PRESS_BOUNCE $toolkit_motion PRESS_BOUNCE" \
    "press-down $project_ui PRESS_DOWN $toolkit_motion PRESS_DOWN" \
    "highlight-spring $project_menu HIGHLIGHT_EASE_RATE $toolkit_motion HIGHLIGHT_SPRING" \
    "pulse-period $project_ui PULSE_PERIOD $toolkit_motion PULSE"; do
    read -r label project_file project_name toolkit_file toolkit_name <<< "$mapping"
    compare_number "control.$label" \
        "$(rust_scalar "control.$label.project" "$project_file" "$project_name")" \
        "$(rust_scalar "control.$label.toolkit" "$toolkit_file" "$toolkit_name")"
done

# The slide between a column and its page on a window standing on its side is
# the shell's Home menu's own: how much of the other half stays in view, and
# the spring the view rides. See `layout.rs`, and `overview.rs` in the shell.
compare_number layout.peek \
    "$(rust_scalar layout.peek.project "$project_overview" PEEK)" \
    "$(rust_scalar layout.peek.toolkit "$toolkit_layout" PEEK)"
compare_number layout.spring \
    "$(rust_scalar layout.spring.project "$project_overview" CARD_SPRING)" \
    "$(rust_scalar layout.spring.toolkit "$toolkit_motion" CARD_SPRING)"

# Where the shell's on-screen keyboard settles, which an application under it
# keeps the field being typed into clear of (`board.rs`): the shell's
# `keyboard_panel_rect`, by its numbers, its rows and every line of the rule.
for mapping in \
    "key-unit $project_ui KEY_UNIT KEY_UNIT" \
    "key-height $project_ui KEY_HEIGHT KEY_HEIGHT" \
    "key-gap $project_ui KEY_GAP KEY_GAP" \
    "padding $project_ui BOARD_PADDING BOARD_PADDING" \
    "margin $project_ui BOARD_MARGIN BOARD_MARGIN" \
    "columns $project_keyboard COLUMNS COLUMNS" \
    "rows $project_keyboard ROW_COUNT ROW_COUNT"; do
    read -r label project_file project_name toolkit_name <<< "$mapping"
    compare_number "board.$label" \
        "$(rust_scalar "board.$label.project" "$project_file" "$project_name")" \
        "$(rust_scalar "board.$label.toolkit" "$toolkit_board" "$toolkit_name")"
done
compare_number board.function-row \
    "$(rust_numbers board.function-row.project "$project_keyboard" row_scale | awk '{print $1}')" \
    "$(rust_scalar board.function-row.toolkit "$toolkit_board" FUNCTION_ROW_SCALE)"
function_contains board.function-row.first "$project_keyboard" row_scale \
    'if row == FUNCTION_ROW {'
compare_number board.function-row.index \
    "$(rust_scalar board.function-row.index.project "$project_keyboard" FUNCTION_ROW)" 0
function_contains board.function-row.first.toolkit "$toolkit_board" row_scale 'if row == 0 {'
arm_contains board.function-row.short.toolkit "$toolkit_board" row_scale \
    'if row == 0 {' '} else {' 'FUNCTION_ROW_SCALE'
function_contains board.rows.band "$project_ui" row_band \
    '.map(|above| keyboard::row_scale(above) * KEY_HEIGHT + KEY_GAP)'
function_contains board.rows.band.toolkit "$toolkit_board" row_band \
    '.map(|above| row_scale(above) * KEY_HEIGHT + KEY_GAP)'
function_contains board.rows.keys "$project_ui" keys_height \
    'let (top, height) = row_band(keyboard::ROW_COUNT - 1);'
function_contains board.rows.keys.toolkit "$toolkit_board" keys_height \
    'let (top, height) = row_band(ROW_COUNT - 1);'
function_contains board.scale "$project_ui" board_scale 'let scale = guide_scale(height);'
function_contains board.scale.toolkit "$toolkit_board" board_scale 'let scale = scale_for(height);'
function_contains board.scale.natural "$project_ui" board_scale \
    'let natural = (KEY_UNIT * keyboard::COLUMNS + (BOARD_PADDING + BOARD_MARGIN) * 2.0) * scale;'
function_contains board.scale.natural.toolkit "$toolkit_board" board_scale \
    'let natural = (KEY_UNIT * COLUMNS + (BOARD_PADDING + BOARD_MARGIN) * 2.0) * scale;'
for side in project toolkit; do
    if [[ $side == project ]]; then file=$project_ui; else file=$toolkit_board; fi
    function_contains "board.scale.narrow.$side" "$file" board_scale \
        'if width > 0.0 && natural > width {'
    function_contains "board.scale.shrunk.$side" "$file" board_scale \
        'scale * (width / natural)'
done
function_contains board.rect.width "$project_ui" keyboard_panel_rect \
    'let w = (KEY_UNIT * keyboard::COLUMNS + BOARD_PADDING * 2.0) * scale;'
function_contains board.rect.width.toolkit "$toolkit_board" rect \
    'let w = (KEY_UNIT * COLUMNS + BOARD_PADDING * 2.0) * scale;'
for side in project toolkit; do
    if [[ $side == project ]]; then
        file=$project_ui
        name=keyboard_panel_rect
    else
        file=$toolkit_board
        name=rect
    fi
    function_contains "board.rect.height.$side" "$file" "$name" \
        'let h = (keys_height() + BOARD_PADDING * 2.0) * scale;'
    function_contains "board.rect.where.$side" "$file" "$name" \
        '[(width - w) * 0.5, height - BOARD_MARGIN * scale - h, w, h]'
    function_contains "board.rect.scale.$side" "$file" "$name" \
        'let scale = board_scale(width, height);'
done

# The spring itself, which every light, slide and card in both rides: the
# longest step it is taken for, and the whole of its arithmetic. It is solved
# in closed form so that two processes stepping it on their own frames arrive
# together, and that only holds while it is the same function on both sides —
# a constant compared beside a formula that has drifted would say nothing.
compare_number motion.longest-step \
    "$(rust_scalar motion.longest-step.project "$project_overview" LONGEST_STEP)" \
    "$(rust_scalar motion.longest-step.toolkit "$toolkit_motion" LONGEST_STEP)"
compare_text motion.spring.arithmetic \
    "$(rust_body_hash motion.spring.project "$project_overview" spring)" \
    "$(rust_body_hash motion.spring.toolkit "$toolkit_motion" spring)"

# The panel's own flight, which the shell keeps with the menu rather than with
# the other durations.
compare_number menu.flight \
    "$(rust_scalar menu.flight.project "$project_menu" FLIGHT)" \
    "$(toolkit_duration menu.flight.toolkit MENU_FLIGHT)"

# Theme is two independent domains: icons have the two materials, while the
# wallpaper additionally admits the user's own picture or film.
project_icon_domain=$(extract_one theme.icons.domain.project "$project_theme" \
    's/^pub const STYLES: [^=]+=[[:space:]]*(\[[^;]+\]);/\1/p')
toolkit_icon_domain=$(extract_one theme.icons.domain.toolkit "$toolkit_settings" \
    's/^[[:space:]]*pub const ALL: \[Self; 2\] = (\[[^;]+\]);/\1/p')
compare_text theme.icons.domain \
    "$(normalize_domain "$project_icon_domain")" \
    "$(normalize_domain "$toolkit_icon_domain")"
project_wallpaper_domain=$(extract_one theme.wallpaper.domain.project "$project_theme" \
    's/^pub const WALLPAPER_STYLES: [^=]+=[[:space:]]*(\[[^;]+\]);/\1/p')
toolkit_wallpaper_domain=$(extract_one theme.wallpaper.domain.toolkit "$toolkit_settings" \
    's/^[[:space:]]*pub const ALL: \[Self; 3\] = (\[[^;]+\]);/\1/p')
compare_text theme.wallpaper.domain \
    "$(normalize_domain "$project_wallpaper_domain")" \
    "$(normalize_domain "$toolkit_wallpaper_domain")"
project_custom=$(extract_one theme.wallpaper.custom-name.project "$project_theme" \
    's/^pub const CUSTOM: &str = "([^"]+)";/\1/p')
toolkit_custom=$(extract_one theme.wallpaper.custom-name.toolkit "$toolkit_settings" \
    's/^[[:space:]]*"([^"]+)" => Some\(Self::Custom\),/\1/p')
compare_text theme.wallpaper.custom-name "$project_custom" "$toolkit_custom"

# The analytic wallpaper applications can compose directly. Constants are
# compared by value and the three authoritative shell functions by fingerprint:
# changing their arithmetic must be an explicit toolkit synchronization, not a
# quiet version bump. The standalone module intentionally removes scenery and
# custom-file texture sampling, so it cannot be byte-identical as one block.
project_visual=$(extract_one wallpaper.visual.project "$project_theme" \
    's/^pub const VISUAL: &str = "([^"]+)";/\1/p')
toolkit_visual=$(extract_one wallpaper.visual.toolkit "$toolkit_wallpaper" \
    's/^pub const VISUAL: &str = "([^"]+)";/\1/p')
compare_text wallpaper.visual "$project_visual" "$toolkit_visual"

# --- the wallpaper's clock, handed from one process to the next --------------

# The scene is a function of a palette and a number of seconds, so a window
# opening over a wallpaper already on screen only has to be told which second
# it is. That is a record one process writes and another reads, which makes
# every byte of it a contract: the shell reads one from its display manager,
# and a toolkit window reads one from whatever started it — the same variable,
# the same fields, the same refusals, in the same words.
#
# One deliberate difference, and it is an addition rather than a divergence:
# the toolkit writes records as well as reading them, so it canonicalizes the
# material a record names instead of accepting it and dropping it on the floor
# as the shell does. Both still accept every record the other writes.
project_wallpaper_clock="$project_root/crates/lxb-desktop/src/wallpaper_clock.rs"
toolkit_handoff="$toolkit_root/crates/lxb-toolkit/src/handoff.rs"
require_file handoff.project "$project_wallpaper_clock"
require_file handoff.toolkit "$toolkit_handoff"

compare_text handoff.variable \
    "$(rust_scalar handoff.variable.project "$project_wallpaper_clock" HANDOFF_ENV)" \
    "$(rust_scalar handoff.variable.toolkit "$toolkit_handoff" ENV)"
compare_text handoff.clock \
    "$(rust_scalar handoff.clock.project "$project_wallpaper_clock" CLOCK_ID)" \
    "$(rust_scalar handoff.clock.toolkit "$toolkit_handoff" CLOCK)"
compare_text handoff.boot-id \
    "$(rust_scalar handoff.boot-id.project "$project_wallpaper_clock" BOOT_ID_PATH)" \
    "$(rust_scalar handoff.boot-id.toolkit "$toolkit_handoff" BOOT_ID_PATH)"
compare_text handoff.max-record-bytes \
    "$(rust_scalar handoff.max-record-bytes.project "$project_wallpaper_clock" MAX_RECORD_BYTES)" \
    "$(rust_scalar handoff.max-record-bytes.toolkit "$toolkit_handoff" MAX_RECORD_BYTES)"
compare_text handoff.expires-after \
    "$(rust_scalar handoff.expires-after.project "$project_wallpaper_clock" MAX_HANDOFF_AGE_NS)" \
    "$(rust_scalar handoff.expires-after.toolkit "$toolkit_handoff" MAX_AGE_NS)"
compare_text handoff.nanos-per-second \
    "$(rust_scalar handoff.nanos.project "$project_wallpaper_clock" NANOS_PER_SECOND)" \
    "$(rust_scalar handoff.nanos.toolkit "$toolkit_handoff" NANOS_PER_SECOND)"

# The scene a record names is the one named beside the wallpaper itself, in
# both, so neither can offer a phase for a picture its reader does not draw.
file_contains handoff.visual.project "$project_wallpaper_clock" \
    "const VISUAL_ID: &str = lxb_protocol::wallpaper::VISUAL;"
file_contains handoff.visual.toolkit "$toolkit_handoff" \
    "use crate::{palette::PALETTES, settings::WallpaperStyle, wallpaper::VISUAL};"
file_contains handoff.version.project "$project_wallpaper_clock" 'if version != "1" {'
compare_text handoff.version '"1"' \
    "$(rust_scalar handoff.version.toolkit "$toolkit_handoff" VERSION)"

# Every field on the wire, in the order each reader writes them down.
record_fields() {
    local found
    found=$(sed -nE 's/^[[:space:]]*"([a-z-]+)" => &mut [a-z_]+,$/\1/p' "$2" | paste -sd,)
    [[ -n "$found" ]] || fail_setup "$1" "no record fields in $2"
    printf '%s' "$found"
}
compare_text handoff.fields \
    "$(record_fields handoff.fields.project "$project_wallpaper_clock")" \
    "$(record_fields handoff.fields.toolkit "$toolkit_handoff")"

# And every reason a record is refused. A reader that quietly accepted one the
# other refuses would draw a different wallpaper at the same second, which is
# the whole of what this exists to prevent.
refusals() {
    local found
    found=$(sed -nE 's/^[[:space:]]*Self::[A-Za-z0-9]+ => "([^"]+)",$/\1/p' "$2" \
        | sort | paste -sd'|')
    [[ -n "$found" ]] || fail_setup "$1" "no refusals in $2"
    printf '%s' "$found"
}
compare_text handoff.refusals \
    "$(refusals handoff.refusals.project "$project_wallpaper_clock")" \
    "$(refusals handoff.refusals.toolkit "$toolkit_handoff")"

# One whole record, byte for byte: the fixture each side's own tests assert
# against, which is also the fixture the display manager's encoder is held to.
canonical_record() {
    extract_one "$1" "$2" 's/^[[:space:]]*"(v=1;visual=lxb-[^"]+)",?[[:space:]]*$/\1/p'
}
compare_text handoff.record \
    "$(canonical_record handoff.record.project "$project_wallpaper_clock")" \
    "$(canonical_record handoff.record.toolkit "$toolkit_handoff")"
compare_vector wallpaper.key-light \
    "$(wgsl_vector wallpaper.key-light.project "$project_shader" KEY_LIGHT)" \
    "$(rust_vector wallpaper.key-light.toolkit "$toolkit_wallpaper" KEY_LIGHT)"
for mapping in \
    'band-fold BAND_FOLD BAND_FOLD LXB_WALLPAPER_BAND_FOLD' \
    'band-bow BAND_BOW BAND_BOW LXB_WALLPAPER_BAND_BOW' \
    'band-share BAND_SHARE BAND_SHARE LXB_WALLPAPER_BAND_SHARE' \
    'band-edge-samples BAND_EDGE_SAMPLES BAND_EDGE_SAMPLES LXB_WALLPAPER_BAND_EDGE_SAMPLES'; do
    read -r label project_name toolkit_name shader_name <<< "$mapping"
    project_value=$(wgsl_scalar "wallpaper.$label.project" "$project_shader" "$project_name")
    compare_number "wallpaper.$label.rust" "$project_value" \
        "$(rust_scalar "wallpaper.$label.toolkit" "$toolkit_wallpaper" "$toolkit_name")"
    compare_number "wallpaper.$label.shader" "$project_value" \
        "$(wgsl_scalar "wallpaper.$label.asset" "$toolkit_wallpaper_shader" "$shader_name")"
done
for mapping in \
    'water c8ab8ac33019654e9836b86bcd6c4799b88e6d666e7dc2ab0e61b64a37c28544' \
    'silk 3ab33633ad66658c4f0a8b3e75051373c04038f7dc73eaf37e440240486831ad' \
    'wallpaper 6fa870d7761f37fba79043465b020c3a4faa4e318a3dcdefc6e52d3c11e545e5' \
    'over_the_wallpaper b1e529ca0a75e9aa04d71613b25127e867dd3f1de0dd40c19a6bf53ecd05df40' \
    'ambient_field 4ddb8b9a40e94f936978b46f768ae416d1014e31048df4759affc0c9a8f86cc5' \
    'bevel_rise 69807eaf7ef5ba36426bcaf86c7f8b219747afd33d0781392ae42f3ff08b76ff' \
    'bevel_slope 12d5605a16134707b75a01faf0b852849f77df99d946de22daccfeae566a018d'; do
    read -r function expected <<< "$mapping"
    compare_text "wallpaper.arithmetic.$function" \
        "$(function_hash "wallpaper.arithmetic.$function" "$project_shader" "$function")" \
        "$expected"
done

# The sparkles the current carries, and the spine they are shed from. These
# the shipped module takes from the shell whole, so they are compared as the
# shell's own text rather than pinned: a change there fails here until it has
# been carried over, and the shipped copy cannot drift on its own.
for name in spine_at silk_spine sparkle_hash sparkle_unit sparkle_bits \
    sparkle_pushed sparkle_unpushed sparkle_hold sparkle_unheld sparkle_bump \
    sparkle_half sparkle_layer sparkles; do
    compare_text "wallpaper.sparkles.$name" \
        "$(wallpaper_function_hash "wallpaper.sparkles.$name.project" "$project_shader" "$name")" \
        "$(wallpaper_function_hash "wallpaper.sparkles.$name.toolkit" \
            "$toolkit_wallpaper_shader" "lxb_wallpaper_$name")"
done
for name in Spine SPINE_REST SPARKLE_LANE SPARKLE_BIRTH SPARKLE_SQUEEZE \
    SPARKLE_SINK SPARKLE_STEEPEST SparkleLayer SPARKLE_DUST SPARKLE_GLINTS; do
    case $name in
        [A-Z][a-z]*) toolkit_name="LxbWallpaper$name" ;;
        *) toolkit_name="LXB_WALLPAPER_$name" ;;
    esac
    compare_text "wallpaper.sparkles.$name" \
        "$(wallpaper_body "$(declaration "wallpaper.sparkles.$name.project" \
            "$project_shader" "$name")")" \
        "$(wallpaper_body "$(declaration "wallpaper.sparkles.$name.toolkit" \
            "$toolkit_wallpaper_shader" "$toolkit_name")")"
done
# And the constants the processor's copy draws with, number for number in the
# order they are written: a depth's seed, its cells, its pace and its light.
for name in SPINE_REST SPARKLE_LANE SPARKLE_BIRTH SPARKLE_SQUEEZE SPARKLE_SINK \
    SPARKLE_STEEPEST SPARKLE_DUST SPARKLE_GLINTS; do
    compare_text "paint.constant.$name" \
        "$(declaration_numbers "$(declaration "paint.constant.$name.shader" \
            "$toolkit_wallpaper_shader" "LXB_WALLPAPER_$name")")" \
        "$(declaration_numbers "$(declaration "paint.constant.$name.cpu" \
            "$toolkit_paint" "$name")")"
done

# Colour, by role. Twelve palettes of fourteen roles each: 168 authored
# values, and the whole of what "coloured by role rather than by name" means.
# Nothing else in this checker would notice a palette that had drifted, and a
# drifted palette is the one kind of mismatch a user sees immediately.
for palette in PURPLE BLUE GREEN YELLOW RED TEAL INDIGO PINK ORANGE WHITE \
    SILVER BLACK; do
    for role in accent accent_soft accent_deep glass glass_raised rim text \
        text_soft danger glow; do
        compare_text "palette.$palette.$role" \
            "$(palette_field "palette.$palette.$role.project" \
                "$project_palettes" "$palette" "$role" Color)" \
            "$(palette_field "palette.$palette.$role.toolkit" \
                "$toolkit_palette" "$palette" "$role" Srgb)"
    done
    compare_text "palette.$palette.sky" \
        "$(palette_sky "palette.$palette.sky.project" \
            "$project_palettes" "$palette" Color)" \
        "$(palette_sky "palette.$palette.sky.toolkit" \
            "$toolkit_palette" "$palette" Srgb)"
done

# Motion. The shell keeps each duration beside the thing it moves, under the
# name that thing calls it; the toolkit gathers them into one module under
# names an application can read. That renaming is exactly where a value can be
# copied wrong and nothing complains, so the mapping is written out in full.
#
# Two toolkit names deliberately share one shell constant: PANEL and
# MENU_FLIGHT are both the menu's own flight, offered once generally and once
# by the surface it came from.
for mapping in \
    "panel $project_menu FLIGHT PANEL" \
    "menu-flight $project_menu FLIGHT MENU_FLIGHT" \
    "menu-unfold $project_menu UNFOLD MENU_UNFOLD" \
    "menu-press $project_menu PRESS_TIME MENU_PRESS" \
    "launch-open $project_launch OPEN LAUNCH_OPEN" \
    "launch-settle $project_launch SETTLE LAUNCH_SETTLE" \
    "launch-handover $project_launch HANDOVER LAUNCH_HANDOVER" \
    "black-in $project_launch BLACK_IN BLACK_IN" \
    "black-hold $project_launch BLACK_HOLD BLACK_HOLD" \
    "black-out $project_launch BLACK_OUT BLACK_OUT" \
    "black-handover $project_ui BLACK_HANDOVER BLACK_HANDOVER" \
    "arrival $project_ui ARRIVAL ARRIVAL" \
    "sidebar-slide $project_ui GUIDE_SLIDE SIDEBAR_SLIDE" \
    "entry-lead $project_ui ENTRY_LEAD ENTRY_LEAD" \
    "entry-stagger $project_ui ENTRY_STAGGER ENTRY_STAGGER" \
    "entry-slide $project_ui ENTRY_SLIDE ENTRY_SLIDE" \
    "pulse $project_ui PULSE_PERIOD PULSE" \
    "spin $project_ui LAUNCH_SPIN SPIN" \
    "accent-change $project_palettes ACCENT_CHANGE ACCENT_CHANGE" \
    "scenery-fade $project_main SCENERY_FADE SCENERY_FADE" \
    "colour-fade $project_main COLOUR_FADE COLOUR_FADE" \
    "guide-press $project_guide PRESS_TIME GUIDE_PRESS" \
    "guide-power-flight $project_guide POWER_FLIGHT GUIDE_POWER_FLIGHT" \
    "guide-media-flight $project_guide MEDIA_FLIGHT GUIDE_MEDIA_FLIGHT" \
    "guide-transport-flight $project_guide TRANSPORT_FLIGHT GUIDE_TRANSPORT_FLIGHT" \
    "notification-hold $project_notify DWELL NOTIFICATION_HOLD" \
    "notification-enter $project_notify FLY_IN NOTIFICATION_ENTER" \
    "notification-leave $project_notify FLY_OUT NOTIFICATION_LEAVE" \
    "notification-badge $project_notify BADGE_FLIGHT NOTIFICATION_BADGE" \
    "volume-fill $project_volume RISE VOLUME_FILL" \
    "volume-hold $project_volume DWELL VOLUME_HOLD" \
    "volume-leave $project_volume FALL VOLUME_LEAVE" \
    "keyboard-slide $project_keyboard SLIDE KEYBOARD_SLIDE"; do
    read -r label project_file project_name toolkit_name <<< "$mapping"
    compare_number "motion.$label" \
        "$(rust_scalar "motion.$label.project" "$project_file" "$project_name")" \
        "$(toolkit_duration "motion.$label.toolkit" "$toolkit_name")"
done

# The longest journey in the interface, and the only one the shell keeps as a
# Duration rather than as seconds — it crosses the compositor protocol, where
# a float of seconds would be the odd type out.
compare_number motion.flight \
    "$(millis_as_seconds motion.flight.project "$project_overview" FLIGHT)" \
    "$(toolkit_duration motion.flight.toolkit FLIGHT)"

# Sound. The recordings themselves are compared byte-for-byte further down;
# this is the other half — which recording answers which action, and which are
# carried without being played. A clip the shell has started or stopped using
# is a semantic change the file comparison cannot see.
project_effects=$(sed -nE \
    's/^[[:space:]]*Effect::[A-Za-z]+ => "([a-z-]+)\.ogg",[[:space:]]*$/\1/p' \
    "$project_sound_source" | sort | paste -sd,)
[[ -n "$project_effects" ]] \
    || fail_setup sound.effects.project "no Effect table in $project_sound_source"
# The shell's background loop is not an Effect; the toolkit lists it with them.
project_effects=$(printf '%s\nstart-bg-music\n' "${project_effects//,/$'\n'}" \
    | sed '/^$/d' | sort | paste -sd,)
toolkit_used=$(sed -nE \
    '/pub const SHELL_USED: \[Sound; [0-9]+\] = \[/,/\];/{s/^[[:space:]]*Sound::([A-Za-z]+),[[:space:]]*$/\1/p}' \
    "$toolkit_sound_source" | sort | paste -sd,)
[[ -n "$toolkit_used" ]] \
    || fail_setup sound.effects.toolkit "no SHELL_USED table in $toolkit_sound_source"
# Compare the recordings each side says the shell plays, not the enum spellings.
toolkit_used_files=$(
    for variant in ${toolkit_used//,/ }; do
        # extract_one prints without a trailing newline, and these are a list.
        printf '%s\n' "$(extract_one "sound.$variant" "$toolkit_sound_source" \
            "s/^[[:space:]]*Sound::$variant => \"([a-z-]+)\",[[:space:]]*\$/\\1/p")"
    done | sort | paste -sd,
)
compare_text sound.shell-used "$project_effects" "$toolkit_used_files"

# Payload checks come last so a stale asset does not hide a malformed token
# comparison in this checker itself.
project_glyph_names=$(find "$project_glyphs" -maxdepth 1 -type f -name '*.svg' \
    -printf '%f\n' | sort)
toolkit_glyph_names=$(find "$toolkit_glyphs" -maxdepth 1 -type f -name '*.svg' \
    -printf '%f\n' | sort)
# The toolkit ships a curated subset — 98 of the shell's marks — and has since
# its set was frozen: the shell keeps marks for the Users page, for
# Picture-in-Picture and for the guide that an application has nothing to draw
# with. So the assertion is one-directional. Every mark shipped here must be
# the shell's, and none may be invented; a library that drew a mark the shell
# does not have would be a second design language wearing the first one's name.
# The shell growing a mark is not drift and is not reported here — what would
# report it is somebody wanting that mark, which is a decision and not a check.
invented_glyphs=$(comm -23 \
    <(printf '%s\n' "$toolkit_glyph_names") \
    <(printf '%s\n' "$project_glyph_names"))
if [[ -n "$invented_glyphs" ]]; then
    fail_mismatch assets.glyph.names \
        'the shell has no such mark' "$(printf '%s' "$invented_glyphs" | tr '\n' ' ')"
fi
while IFS= read -r glyph_name; do
    if ! diff -q \
        <(normalised_svg "$project_glyphs/$glyph_name") \
        <(normalised_svg "$toolkit_glyphs/$glyph_name") >/dev/null; then
        fail_mismatch "assets.glyph.$glyph_name" 'geometry differs' 'geometry differs'
    fi
done <<< "$toolkit_glyph_names"
if ! diff -qr "$project_sounds" "$toolkit_sounds"; then
    fail_mismatch assets.sounds 'directory contents differ' 'directory contents differ'
fi

project_font_regular="$project_root/font/Roboto/static/Roboto-Regular.ttf"
project_font_bold="$project_root/font/Roboto/static/Roboto-Bold.ttf"
project_font_license="$project_root/font/Roboto/LICENSE.txt"
toolkit_font_regular="$toolkit_root/crates/lxb-toolkit/assets/fonts/Roboto-Regular.ttf"
toolkit_font_bold="$toolkit_root/crates/lxb-toolkit/assets/fonts/Roboto-Bold.ttf"
toolkit_font_license="$toolkit_root/crates/lxb-toolkit/assets/fonts/LICENSE-Roboto.txt"
for required in \
    "$project_font_regular" "$project_font_bold" "$project_font_license" \
    "$toolkit_font_regular" "$toolkit_font_bold" "$toolkit_font_license"; do
    require_file assets.fonts "$required"
done
if ! cmp -s "$project_font_regular" "$toolkit_font_regular"; then
    fail_mismatch assets.font.regular "$project_font_regular" "$toolkit_font_regular"
fi
if ! cmp -s "$project_font_bold" "$toolkit_font_bold"; then
    fail_mismatch assets.font.bold "$project_font_bold" "$toolkit_font_bold"
fi
if ! cmp -s "$project_font_license" "$toolkit_font_license"; then
    fail_mismatch assets.font.license "$project_font_license" "$toolkit_font_license"
fi

# The two faces for the scripts Roboto has not got, and their licences. The
# Han face is a subset the shell cuts from Noto Sans CJK SC, so the file is
# the shell's to regenerate (scripts/subset-han-face.py) and this side's to
# copy; a byte of difference is a subset cut twice.
for face in \
    "NotoSansDevanagariUI/NotoSansDevanagariUI-Regular.ttf:NotoSansDevanagariUI-Regular.ttf" \
    "NotoSansDevanagariUI/NotoSansDevanagariUI-Bold.ttf:NotoSansDevanagariUI-Bold.ttf" \
    "NotoSansDevanagariUI/OFL.txt:LICENSE-NotoSansDevanagariUI.txt" \
    "NotoSansCJKsc/NotoSansCJKsc-Regular.ttf:NotoSansCJKsc-Regular.ttf" \
    "NotoSansCJKsc/NotoSansCJKsc-Bold.ttf:NotoSansCJKsc-Bold.ttf" \
    "NotoSansCJKsc/OFL.txt:LICENSE-NotoSansCJKsc.txt"; do
    project_face="$project_root/font/${face%%:*}"
    toolkit_face="$toolkit_root/crates/lxb-toolkit/assets/fonts/${face##*:}"
    require_file assets.fonts "$project_face"
    require_file assets.fonts "$toolkit_face"
    if ! cmp -s "$project_face" "$toolkit_face"; then
        fail_mismatch "assets.font.${face##*:}" "$project_face" "$toolkit_face"
    fi
done

# --- what the user pressed, and what it means -------------------------------
#
# The cadence a held direction walks at, the two distances a stick engages and
# releases at, and how far a device with no notches travels. Every one of them
# is a number somebody using both programs would feel the difference in, and
# none of them is a number anybody can see.

project_controller="$project_root/crates/lxb-desktop/src/controller.rs"
toolkit_input="$toolkit_root/crates/lxb-toolkit/src/input.rs"
toolkit_pad="$toolkit_root/crates/lxb-input/src/lib.rs"
require_file input.project "$project_controller"
require_file input.toolkit "$toolkit_input"
require_file input.pad "$toolkit_pad"

for pair in \
    "poll-interval:POLL_INTERVAL:POLL_INTERVAL" \
    "initial-repeat:INITIAL_REPEAT_DELAY:INITIAL_REPEAT_DELAY" \
    "repeat-interval:REPEAT_INTERVAL:REPEAT_INTERVAL"; do
    IFS=':' read -r label project_name toolkit_name <<< "$pair"
    compare_number "input.$label" \
        "$(duration_seconds "input.$label.project" "$project_controller" "$project_name")" \
        "$(duration_seconds "input.$label.toolkit" "$toolkit_input" "$toolkit_name")"
done

compare_number input.stick-engage \
    "$(rust_scalar input.stick-engage.project "$project_controller" STICK_ENGAGE)" \
    "$(rust_scalar input.stick-engage.toolkit "$toolkit_input" STICK_ENGAGE)"
compare_number input.stick-release \
    "$(rust_scalar input.stick-release.project "$project_controller" STICK_RELEASE)" \
    "$(rust_scalar input.stick-release.toolkit "$toolkit_input" STICK_RELEASE)"
compare_number input.hat \
    "$(rust_scalar input.hat.project "$project_controller" HAT_THRESHOLD)" \
    "$(rust_scalar input.hat.toolkit "$toolkit_pad" HAT)"
compare_number input.scroll-step \
    "$(rust_scalar input.scroll-step.project "$project_main" SCROLL_STEP)" \
    "$(rust_scalar input.scroll-step.toolkit "$toolkit_input" SCROLL_STEP)"
compare_number input.tap-slop \
    "$(rust_scalar input.tap-slop.project "$project_main" TAP_SLOP)" \
    "$(rust_scalar input.tap-slop.toolkit "$toolkit_input" TAP_SLOP)"

# The middle of a held direction, which is the one piece of arithmetic here.
# Both schedule the next step from `now` rather than from the deadline they
# missed, so a stalled frame is never followed by a burst of stale repeats.
compare_text input.repeat.arithmetic \
    "$(rust_numbers input.repeat.project "$project_controller" update)" \
    "$(rust_numbers input.repeat.toolkit "$toolkit_input" update)"
file_contains input.repeat.scheduled-from-now "$project_controller" \
    "held.next_repeat = Some(now + REPEAT_INTERVAL);"
file_contains input.repeat.scheduled-from-now.toolkit "$toolkit_input" \
    "held.next = Some(now + REPEAT_INTERVAL);"

# One press two controllers both report is answered once, whoever the second
# one is: Steam Input's pad for a game, a copy made under ids neither side was
# shipped knowing, or one pad read through two drivers. The question is not
# which device a press came from but whether the same action already came from
# a *different* one inside the window; a repeat on the same controller is a
# second press, and a held direction is read from every pad merged, not from
# this. The shell's controllers are pads, its raw Steam Controller and its
# guard; an application's are the pads it reads, which every press passes.
compare_number input.coincidence \
    "$(duration_seconds input.coincidence.project "$project_controller" COINCIDENCE)" \
    "$(duration_seconds input.coincidence.toolkit "$toolkit_input" COINCIDENCE)"
file_contains input.coincidence.window "$project_controller" \
    'answered.retain(|(_, _, at)| now.saturating_sub(*at) < COINCIDENCE);'
file_contains input.coincidence.window.toolkit "$toolkit_input" \
    '.retain(|(_, _, at)| now.saturating_sub(*at) < COINCIDENCE);'
file_contains input.coincidence.another-controller "$project_controller" \
    '.find(|(had, by, _)| *had == action && *by != from)'
file_contains input.coincidence.another-controller.toolkit "$toolkit_input" \
    '.any(|(had, by, _)| *had == action && *by != from)'
file_contains input.coincidence.every-pad.toolkit "$toolkit_pad" \
    'if self.answered.answer(action, event.id, now) {'

# The mapping itself, pinned on both sides. A table cannot be hashed against
# one written for another program — the shell has actions an application must
# never be given — so what is pinned is every row the two do share.
file_contains input.map.accept "$project_main" \
    "Keysym::Return | Keysym::KP_Enter | Keysym::space => Some(Action::Launch),"
file_contains input.map.back "$project_main" \
    "Keysym::Escape | Keysym::BackSpace | Keysym::XF86_Back => Some(Action::Back),"
file_contains input.map.accept.toolkit "$toolkit_input" \
    "Key::Enter | Key::Space => Some(Action::Accept),"
file_contains input.map.back.toolkit "$toolkit_input" \
    "Key::Escape | Key::Backspace => Some(Action::Back),"
file_contains input.map.letters "$project_main" \
    "Keysym::Up | Keysym::KP_Up | Keysym::w | Keysym::W | Keysym::k | Keysym::K => {"
file_contains input.map.letters.toolkit "$toolkit_input" \
    "'w' | 'k' => Some(Action::Up),"
file_contains input.map.south "$project_controller" \
    "Button::South => return Some(Action::Launch),"
file_contains input.map.east "$project_controller" \
    "Button::East => return Some(Action::Back),"
file_contains input.map.start "$project_controller" \
    "Button::Start => return Some(Action::Submit),"
file_contains input.map.south.toolkit "$toolkit_input" \
    "Button::South => Some(Action::Accept),"
file_contains input.map.east.toolkit "$toolkit_input" \
    "Button::East => Some(Action::Back),"
file_contains input.map.start.toolkit "$toolkit_input" \
    "Button::Start => Some(Action::Submit),"
# The top face button raises the context menu in both, and on a pad nothing
# has a mapping for both middle face buttons are taken to be it.
file_contains input.map.top-face "$project_controller" \
    "Layout::Mapped => matches!(button, Button::North),"
file_contains input.map.top-face.toolkit "$toolkit_pad" \
    "Layout::Mapped => matches!(button, PadButton::North),"
# And the one control an application is never given.
file_contains input.guide-is-the-shells "$toolkit_input" \
    "Button::Guide | Button::West | Button::Select => None,"

# The Menu key raises the context menu on both sides, pinned so neither can
# drift off it. This used to be pinned as a *divergence* — the shell gave that
# key to the guide — and it stopped being one when the shell adopted this
# crate's answer, which is what a key printed with a menu on it should do.
file_contains input.map.menu-key "$project_main" \
    "Keysym::Menu | Keysym::F10 | Keysym::y | Keysym::Y => Some(Action::Menu),"
file_contains input.map.menu-key.toolkit "$toolkit_input" \
    "Key::Menu | Key::F10 => Some(Action::Menu),"
# And the shell's y beside them, which a window reads where no field takes it
# as text (lxb-app's letter_press); the toolkit's own y in of_letter is
# pinned to the shell's line above with the rest of the shorthands.
function_contains input.map.menu-letter.toolkit \
    "$toolkit_root/crates/lxb-app/src/lib.rs" letter_press \
    'Key::Letter(letter) => Action::of_letter(letter).filter(|action| *action == Action::Menu),'
file_contains input.map.menu-letter.of-letter.toolkit "$toolkit_input" \
    "'y' => Some(Action::Menu),"

# The one place the toolkit deliberately differs, pinned so that it cannot be
# quietly "corrected" into the shell's answer: the shell has to keep a way out
# of a game that the game cannot take, and an application has nothing to keep a
# way out of.
file_contains input.divergence.tab "$project_main" \
    "Keysym::Tab => Some(Action::NextScreen),"
file_contains input.divergence.tab.toolkit "$toolkit_input" \
    "Key::Tab => Some(Action::Next),"

# --- what the session says --------------------------------------------------
#
# The clock is one key in shell.toml, read by both by one rule: "12-hour" or
# "24-hour" is the answer, and anything else — no file at all, no key,
# "language", a word with no clock — leaves it to the language, where American
# English and Hindi read the twelve-hour clock. The toolkit reads the key each
# time it writes a time, so it follows a clock another program writes into the
# file, which a running shell has done since 6f2fdc5.

project_i18n="$project_root/crates/lxb-desktop/src/i18n.rs"
project_settings="$project_root/crates/lxb-desktop/src/settings.rs"
toolkit_i18n="$toolkit_root/crates/lxb-toolkit/src/i18n.rs"
require_file session.project "$project_i18n"
require_file session.project "$project_settings"
require_file session.toolkit "$toolkit_i18n"

file_contains session.clock.twelve "$project_i18n" \
    '"12-hour" => Some(Self::TwelveHour),'
file_contains session.clock.twenty-four "$project_i18n" \
    '"24-hour" => Some(Self::TwentyFourHour),'
file_contains session.clock.twelve.toolkit "$toolkit_settings" \
    'Some("12-hour") => true,'
file_contains session.clock.twenty-four.toolkit "$toolkit_settings" \
    'Some("24-hour") => false,'
file_contains session.clock.from-language "$project_i18n" \
    'Clock::FromLanguage => matches!(spoken(), Language::American | Language::Hindi),'
file_contains session.clock.from-language.toolkit "$toolkit_settings" \
    '_ => matches!(language, "en-US" | "hi"),'

# A shell nobody has set up — no shell.toml at all, as on a live medium or a
# machine just installed — follows the session's language and that language's
# clock (f480316). The toolkit has always stood there: its language is the
# session's and nothing else, and no file leaves the clock to it.
function_contains session.no-file.language "$project_settings" load_from \
    'crate::i18n::set(crate::i18n::Language::System);'
function_contains session.no-file.clock "$project_settings" load_from \
    'crate::i18n::set_clock(crate::i18n::Clock::FromLanguage);'
function_contains session.no-file.toolkit "$toolkit_settings" twelve_hour_clock \
    '.unwrap_or_default(),'

# Where they differ, pinned so neither is quietly "corrected". A running shell
# follows a language another program writes into shell.toml (6f2fdc5; the
# Linexin live medium's installer is that program) and gives every program it
# opens from then on that language's locale. A toolkit application keeps the
# language it started in, as every program on a Linux desktop does: it is one
# of the programs the shell opens, and is handed the new locale when it is
# opened. Its own words come out of one catalog, chosen once per process.
function_contains i18n.divergence.running-language "$project_settings" \
    follow_the_file 'crate::i18n::set(language);'
file_contains i18n.divergence.running-language.toolkit "$toolkit_i18n" \
    'LazyLock::new(|| resolve(|name| std::env::var(name).ok()));'

# --- where an application is answered differently, on purpose ---------------
#
# Each of these is a place the toolkit deliberately stands apart from the
# shell, pinned on both sides like input.divergence.tab so that neither can be
# quietly "corrected" into the other's answer, and so that the shell moving
# either way is named here rather than passing unseen. All but the dialog's ink
# came in for the Linexin Installer, an application written in every one of the
# shell's ten languages over pages it draws itself. The menu's fold is the
# older one of these and is held by its own numbers above.
toolkit_app="$toolkit_root/crates/lxb-app/src/lib.rs"
require_file divergence.toolkit "$toolkit_app"

# A menu grows to its widest row, within the window. The shell's panel is its
# own width plus whatever its caller asked for, and a row longer than that is
# cut, or opens onto more lines when lit; its rows are the shell's own words,
# measured for that width. An application's are not: "Przechowywanie ·
# Standardowe" did not fit in Polish, so the toolkit measures every one-line row
# (bold, as a lit row is written) and the title, and grows the panel to the
# widest, never below its own width and never past the window less its inset.
# What still does not fit is cut with "…" to the chip at rest. (37a0c68,
# bd478fe, 624bb1b)
function_contains menu.divergence.width "$project_ui" context_menu_rect \
    '(context_panel_width(menu.extra_width()) * scale).min((width - inset * 2.0).max(0.0));'
function_contains menu.divergence.width.toolkit "$toolkit_components" context_menu \
    'let extra = state.extra.max(self.widest_row(state) - menu::WIDTH);'

# The quiet lines of a menu and a question are white made quiet by strength.
# The shell writes a menu's title (with its stamp, detail and disabled rows)
# and a question's notes in text_soft, a pale cast of the accent, over its own
# dark surfaces, where it passes for a soft white. Over an application's page,
# showing through the pane, it is lavender on grey: the installer's "Opcje"
# measured (162,144,209). The design language's rule is text_soft only over
# the language's own ground. (2c382aa; the question's body before it)
function_contains menu.divergence.ink "$project_ui" build_context_menu \
    'color: theme.text_soft.a(0.85),'
function_contains menu.divergence.ink.toolkit "$toolkit_components" context_menu \
    'let tint = self.tinted(Role::Text, 0.85);'
function_contains dialog.divergence.ink "$project_ui" build_dialog \
    'color: theme.text_soft.a(0.82),'
function_contains dialog.divergence.ink.toolkit "$toolkit_components" dialog \
    'let quiet = self.tinted(Role::Text, control::INK_QUIET * out);'

# A question keeps the lines it is given, as the shell's does — each of its
# notes is a line — but a line too wide for the pane is broken again where a
# line may break, rather than cut at its tail. The shell writes its own
# sentences to fit; an application hands over a disk's model, a quoted name or
# a Chinese sentence it has broken itself, and an ellipsis there hides the one
# word somebody is being asked about. (bd0a742)
function_contains dialog.divergence.too-wide "$project_ui" build_dialog \
    'Line::Note(text) => {'
function_contains dialog.divergence.too-wide.cut "$project_ui" build_dialog \
    'cut: Cut::Tail,'
function_contains dialog.divergence.too-wide.toolkit "$toolkit_components" dialog \
    'let body = body_lines(&state.body, width - 2.0 * pad, |line| {'

# On low-end hardware a light stands on what is chosen at once. The shell's
# glides there too, its display drawn at its refresh for as long as the
# highlight says it is moving (dbb9a75, GLIDE_AT_REST). A toolkit window on
# low-end hardware draws every frame at the one held instant, so a light
# gliding by the frame's clock never left where it was first drawn — the
# installer's erase question showed "Go back" lit while Enter erased — and its
# frames are paced by a second after any press rather than by a list of what is
# moving. A light that stands at once needs no frames to arrive. (bbd79b1)
function_contains lowend.divergence.light "$project_menu" is_animating \
    '|| (self.is_on_screen() && self.highlight.is_moving())'
file_contains lowend.divergence.light.at-rest "$project_menu" \
    'pub const GLIDE_AT_REST: f32 = 0.5;'
file_contains lowend.divergence.light.toolkit "$toolkit_components" \
    'light.place(target)'

# And two that came in the same week and are the shell's own answer, pinned as
# the agreements they are. A menu of alternatives opens with the light on the
# one in force — the first row that can be chosen at or after it, wrapping
# round — as the shell's confirmations open on No. (0b5abe5, 9704822)
function_contains menu.open-selecting "$project_menu" open_selecting \
    '.chain(entries.iter().enumerate())'
function_contains menu.open-selecting.toolkit "$toolkit_components" open_selecting \
    '.chain(entries.iter().enumerate())'

# Hovering is selecting, for whatever stands still: the pointer moving onto a
# control lights it, silently. (62d8f54)
function_contains pointer.hover-selects "$project_main" hover \
    'self.point_at(spot);'
function_contains pointer.hover-selects.toolkit "$toolkit_app" pointer_to \
    'self.focused = index;'

# And what a held press stops, which is the same on both sides: only a bar is
# held. The shell sets held_bar only where press_at answers with a Held — a
# bar, a quick-settings level, a mixer row, the friends list's or a
# conversation's scroll bar. The toolkit holds only a control the page followed
# with Page::dragging on the frame the press landed on, which is what a bar is
# to an application. While one is held every motion is the drag and nothing
# hovers, the light is not lit from under the handle; a press held on any other
# control goes on hovering, and lighting, with the button down. The click is
# the press, answered on what it went down on; letting go answers nothing, and
# only the left button's lets go. Pinned as a divergence while the toolkit held
# a press on any control (d11dea8, 961ffc3) until the user's word that toolkit
# applications match the shell here.
function_contains pointer.held "$project_main" pointer_frame \
    'self.held_bar = self.press_at(spot).map(|held| (index, held));'
function_contains pointer.held.toolkit "$toolkit_app" take_hold \
    'Spot::Control(id) if self.bars.contains(&id) => Some(id),'
function_contains pointer.held.press.toolkit "$toolkit_app" window_event \
    'self.state.take_hold(spot);'
function_contains pointer.held.bars.toolkit "$toolkit_app" dragging \
    'self.state.asked_bars.push(id);'
function_contains pointer.held.bars.frame.toolkit "$toolkit_app" settle \
    'state.bars = std::mem::take(&mut state.asked_bars);'
function_contains pointer.held.drag "$project_main" pointer_frame \
    'if self.held_bar.is_some() {'
function_contains pointer.held.carries "$project_main" pointer_frame \
    'self.drag_bar(index, x, y);'
function_contains pointer.held.drag.toolkit "$toolkit_app" pointer_to \
    'if !moved || self.held.is_some() {'
function_contains pointer.held.carries.toolkit "$toolkit_app" window_event \
    'if self.state.held.is_some() {'
function_contains pointer.held.let-go "$project_main" pointer_frame \
    'if button == BTN_LEFT {'
function_contains pointer.held.let-go.toolkit "$toolkit_app" let_go \
    'if button == MouseButton::Left {'
arm_contains pointer.held.let-go.release "$project_main" pointer_frame \
    'PointerEventKind::Release {' 'PointerEventKind::' 'self.held_bar = None;'
function_contains pointer.held.let-go.release.toolkit "$toolkit_app" let_go \
    'self.held = None;'

# And what each side can take hold of at all, so that the agreement is not only
# the lines above. The shell holds what its Held names, every one a bar: a
# column's, a quick-settings level, a mixer row, the friends list's scroll bar
# and a conversation's. The toolkit sets its hold in take_hold and nowhere
# else; everything else that touches it lets go. The day the shell holds
# anything more, a button or a row, or the toolkit takes hold anywhere but in
# take_hold, this is where the two are read again.
compare_text pointer.held.what \
    "$(enum_variants pointer.held.what "$project_main" Held)" \
    'Bar(usize) Entry(Item) Mixer(usize) Friends FriendsChat'
compare_text pointer.held.what.toolkit \
    "$(assignments pointer.held.what.toolkit "$toolkit_app" held)" \
    'self.held = match spot { | self.held = None; | self.held = None; | self.state.held = None;'

# A leave lets go as well, on Wayland, which is the whole of the shell's world:
# a button held there keeps the pointer on the surface wherever it travels, so a
# leave is the hold taken away and the release goes to whatever has the pointer
# now. A toolkit window can be an X client too, and X tells a window it was left
# in the middle of a drag (a LeaveNotify, mode Normal, with the motion and the
# release still arriving from outside it), so there a leave does not let go.
arm_contains pointer.held.leave "$project_main" pointer_frame \
    'PointerEventKind::Leave {' 'PointerEventKind::' 'self.held_bar = None;'
function_contains pointer.held.leave.toolkit "$toolkit_app" pointer_left \
    'if on_wayland {'
function_contains pointer.held.leave.lets-go.toolkit "$toolkit_app" pointer_left \
    'self.held = None;'
function_contains pointer.held.leave.wayland.toolkit "$toolkit_app" window_event \
    'self.state.pointer_left(event_loop.is_wayland());'

# An addition rather than a divergence: a soft edge, the blur and fade at the
# ends of a scrolling list, now also at its sides for a page carried across a
# pane (3b2312d). The shell draws none, so there is nothing to compare; the
# day its shaders grow one, they have to be compared rather than left to be two.
# The rest of the week's additions are calls with no behaviour of the shell's
# behind them: a page told where the pointer is and whether the session is
# low-end, a page carried whole, a picture part of the way through, a press a
# page hands the window.
file_contains addition.soft-side-edges.toolkit "$toolkit_render_ui" \
    'let sideways = in.material.z > 0.5;'
file_lacks addition.soft-side-edges "$project_shader" 'SOFT_EDGE'
file_lacks addition.soft-side-edges.offscreen \
    "$project_root/crates/lxb-desktop/src/offscreen.wgsl" 'SOFT_EDGE'

# Additions for an application's controls (2026-10-13, for the Linexin
# Installer's third round). The shell has no switch at all — a row it turns on
# and off wears the chosen mark or the empty one — and its glyph material reads
# every mark out of the atlas. The toolkit draws a switch, and the ring round
# the chosen mark, as beads of the same material: a rounded box measured by
# arithmetic and shaded by the glyphs' own lxb_glyph_material, lamp, depth and
# shadow, nothing of their own. The day the shell's shader measures a box
# itself, the two are compared rather than left to be two.
file_contains addition.bead.toolkit "$toolkit_render_ui" \
    'return lxb_glyph_material(at, bead);'
file_lacks addition.bead "$project_shader" 'bead_field'
# And the shell has no switch to compare the toolkit's with: any it drew would
# have a knob, which nothing it lays out has.
file_lacks addition.switch "$project_ui" 'knob'
file_lacks addition.switch.shader "$project_shader" 'knob'
# The chosen mark is the shell's drawing and the shell's material; what the
# toolkit adds is the dark ring round its bead, for a mark standing on a lit
# control, where the bead is water and took the light's accent. The shell
# lays the same mark on an icon tile with nothing round it (its `build`), so
# a ring there would be a change to compare.
function_contains chosen.divergence.ring "$project_ui" build \
    'slots.glyph(icons::CHOSEN),'
# Nothing round it: the block that lays the badge lays one quad, the badge.
# A ring of the shell's own there would be a second.
chosen_block=$(awk '
    !open && index($0, "if entry.chosen() || marks.is_some_and(|marks| marks.holds(entry)) {") {
        open = 1
    }
    open {
        print
        depth += gsub(/\{/, "{") - gsub(/\}/, "}")
        if (depth <= 0) exit
    }' "$project_ui")
[[ -n "$chosen_block" ]] \
    || fail_setup chosen.divergence.ring.alone "the block laying the chosen badge is not in $project_ui"
[[ "$chosen_block" == *'slots.glyph(icons::CHOSEN),'* ]] \
    || fail_mismatch chosen.divergence.ring.alone "$chosen_block" 'the badge laid in its block'
chosen_quads=$(grep -o 'quads.push(' <<< "$chosen_block" | wc -l)
[[ "$chosen_quads" -eq 1 ]] \
    || fail_mismatch chosen.divergence.ring.alone "$chosen_quads quads" 'one quad, the badge'
function_contains chosen.divergence.ring.toolkit "$toolkit_components" chosen \
    'self.bead_ring(ring, outer * 0.5, thickness, dark, style);'
function_contains chosen.divergence.ring.mark.toolkit "$toolkit_components" chosen \
    'self.icon_tinted(rect, "chosen", style, Role::Text, alpha);'
# And a mark of the application's own: drawn by name like one of the set,
# measured the way the set is (the same rasterising and distance field as the
# atlas is built with), never under one of the set's names. The shell's own
# marks are all its own.
function_contains addition.own-marks.toolkit "$toolkit_renderer" add_mark \
    'return Err(format!("{name} is one of the toolkit'"'"'s own marks"));'
function_contains addition.own-marks.measured.toolkit "$toolkit_renderer" build_atlas \
    'let field = measure_drawing(svg).map_err(|err| format!("{name}: {err}"))?;'

# --- when the shell's on-screen keyboard stands over an application --------
#
# What a window of lxb-app knows of it (`ShellsBoard`, `Page::shells_board`),
# by the shell's own rules, since nothing in Wayland's text input says. The
# board rises once a field, as the field takes the cursor, where the keyboard
# is not at hand, and goes with the field.
function_contains osk.rises.with-the-field "$project_keyboard" set_focused \
    'if !self.offered && !self.keyboard_at_hand {'
function_contains osk.goes.with-the-field "$project_keyboard" set_focused 'self.close();'
function_contains osk.rises.with-the-field.toolkit "$toolkit_app" enabled \
    'self.up = self.watched && controller_in_hand;'
function_contains osk.goes.with-the-field.toolkit "$toolkit_app" window_event \
    'WindowEvent::Ime(Ime::Disabled) => self.state.board.disabled(),'
# The keyboard at hand is the controller-in-hand the shell writes into
# shell.toml as the hands move (and seeds itself from), true until it says
# otherwise; the window reads it as its field takes the cursor.
function_contains osk.at-hand "$project_keyboard" set_controller_in_hand \
    'self.keyboard_at_hand = !in_hand;'
function_contains osk.at-hand.written "$project_main" hands_moved_to \
    'self.osk.set_controller_in_hand(controller);'
file_contains osk.at-hand.seeded "$project_main" \
    '.set_controller_in_hand(settings::controller_in_hand());'
file_contains osk.at-hand.default "$project_settings" \
    'static CONTROLLER_IN_HAND: Mutex<bool> = Mutex::new(true);'
function_contains osk.at-hand.toolkit "$toolkit_app" window_event \
    '.enabled(lxb_toolkit::settings::controller_in_hand().unwrap_or(true)),'
# However it is put away, it lets go of the keyboard, and letting go hands the
# application a report of nothing held: the one thing a window is sent as the
# board goes (seen in the Linexin Installer's protocol log, 2026-10-14). The
# board's own keys carry no report of their own.
function_contains osk.goes.lets-go "$project_keyboard" close 'self.release_keyboard();'
function_contains osk.goes.reported "$project_keyboard" release_keyboard \
    'typist.send_modifiers(Modifiers::default());'
typist_stroke=$(awk '
    index($0, "    fn type_stroke(&self, stroke: Stroke, at: u32) -> bool {") { open = 1 }
    open && !done { print; if ($0 == "    }") done = 1 }' "$project_keyboard")
[[ -n "$typist_stroke" ]] \
    || fail_setup osk.keys.bare "the virtual keyboard's own keystroke is not in $project_keyboard"
[[ "$typist_stroke" == *'self.keyboard.key(at, *code, 1);'* && "$typist_stroke" != *modifiers* ]] \
    || fail_mismatch osk.keys.bare "$typist_stroke" 'a press and a release, and no report of the modifiers'
function_contains osk.goes.reported.toolkit "$toolkit_app" modifiers \
    'if self.up && !held && !was {'
file_contains osk.goes.settled.toolkit "$toolkit_app" \
    'if std::mem::take(&mut self.letting_go) {'
# While it is up the shell keeps every pad from applications, so a pad's
# press reaching a window means it is not.
function_contains osk.keeps-the-pads "$project_main" sync_surface_state \
    'keeps_the_pad(self.keep_keyboard_grabbed, *state, self.osk.is_open())'
function_contains osk.keeps-the-pads.toolkit "$toolkit_app" poll_pads \
    'self.state.board.pad();'

# --- the noise it answers with ----------------------------------------------

compare_number sound.rest \
    "$(duration_seconds sound.rest.project "$project_sound_source" RESTED)" \
    "$(rust_scalar sound.rest.toolkit "$toolkit_sound_source" REST)"
compare_number sound.music-fade-in \
    "$(duration_seconds sound.music-fade-in.project "$project_sound_source" MUSIC_FADE_IN)" \
    "$(rust_scalar sound.music-fade-in.toolkit "$toolkit_sound_source" MUSIC_FADE_IN)"
compare_number sound.music-fade-out \
    "$(duration_seconds sound.music-fade-out.project "$project_sound_source" MUSIC_FADE_OUT)" \
    "$(rust_scalar sound.music-fade-out.toolkit "$toolkit_sound_source" MUSIC_FADE_OUT)"
compare_number sound.retry-after \
    "$(duration_seconds sound.retry-after.project "$project_sound_source" RETRY_AFTER)" \
    "$(rust_scalar sound.retry-after.toolkit "$toolkit_sound_source" RETRY_AFTER)"

# The two curves, by the numbers each of them uses: a level read as loudness,
# and the smoothstep a fade is heard through.
compare_text sound.amplitude \
    "$(rust_numbers sound.amplitude.project "$project_sound_source" normalized_volume)" \
    "$(rust_numbers sound.amplitude.toolkit "$toolkit_sound_source" amplitude)"
compare_text sound.fade \
    "$(rust_numbers sound.fade.project "$project_sound_source" smooth_step)" \
    "$(rust_numbers sound.fade.toolkit "$toolkit_sound_source" fade)"

toolkit_count=$(find "$toolkit_glyphs" -maxdepth 1 -type f -name '*.svg' -print | wc -l)
shape_count=$({ rg -l 'lxb:shape' "$toolkit_glyphs" -g '*.svg' || true; } | wc -l)
# Every mark shipped here is computed material rather than a painted picture,
# so every one of them carries the marker. How many the shell has is reported
# for the person reading a failure and is not what is asserted; see
# assets.glyph.names above for why.
project_count=$(find "$project_glyphs" -maxdepth 1 -type f -name '*.svg' -print | wc -l)
if [[ "$shape_count" -ne "$toolkit_count" ]]; then
    printf 'sync mismatch [assets.glyph.contract]: project=%s toolkit=%s shapes=%s\n' \
        "$project_count" "$toolkit_count" "$shape_count" >&2
    exit 1
fi

printf 'lxb-toolkit matches project-linexinbar %s: %s glyphs, fonts and recordings, 168 palette colours, 34 durations, the context menu shape and material, the file chooser shape, the wallpaper handoff, the control, the three shaders on the processor, what the controls mean, the slide beside a column and its spring, where the on-screen keyboard stands and when, the clock, the token contracts, and the places it differs on purpose, each pinned by a line or a sentinel\n' \
    "$head" "$toolkit_count"
