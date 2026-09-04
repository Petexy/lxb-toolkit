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
    "$toolkit_palette" "$toolkit_motion" "$toolkit_sound_source"; do
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
    "highlight-spring $project_guide HIGHLIGHT_EASE_RATE $toolkit_motion HIGHLIGHT_SPRING" \
    "pulse-period $project_ui PULSE_PERIOD $toolkit_motion PULSE"; do
    read -r label project_file project_name toolkit_file toolkit_name <<< "$mapping"
    compare_number "control.$label" \
        "$(rust_scalar "control.$label.project" "$project_file" "$project_name")" \
        "$(rust_scalar "control.$label.toolkit" "$toolkit_file" "$toolkit_name")"
done

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
    'water 68f6fd6b4b3f202d479e21b3586d5ea276c04d9cd23e14644a921b236e0bf35b' \
    'silk 3ab33633ad66658c4f0a8b3e75051373c04038f7dc73eaf37e440240486831ad' \
    'wallpaper 636cc1610a4cf73c95debc952da61cb1e8fa1aa16b93ead75077e3aea2cadb5a' \
    'over_the_wallpaper b1e529ca0a75e9aa04d71613b25127e867dd3f1de0dd40c19a6bf53ecd05df40' \
    'ambient_field 4ddb8b9a40e94f936978b46f768ae416d1014e31048df4759affc0c9a8f86cc5' \
    'bevel_rise 69807eaf7ef5ba36426bcaf86c7f8b219747afd33d0781392ae42f3ff08b76ff' \
    'bevel_slope 12d5605a16134707b75a01faf0b852849f77df99d946de22daccfeae566a018d'; do
    read -r function expected <<< "$mapping"
    compare_text "wallpaper.arithmetic.$function" \
        "$(function_hash "wallpaper.arithmetic.$function" "$project_shader" "$function")" \
        "$expected"
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

# The one place the toolkit deliberately differs, pinned so that it cannot be
# quietly "corrected" into the shell's answer: the shell has to keep a way out
# of a game that the game cannot take, and an application has nothing to keep a
# way out of.
file_contains input.divergence.tab "$project_main" \
    "Keysym::Tab => Some(Action::NextScreen),"
file_contains input.divergence.tab.toolkit "$toolkit_input" \
    "Key::Tab => Some(Action::Next),"

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

printf 'lxb-toolkit matches project-linexinbar %s: %s glyphs, fonts and recordings, 168 palette colours, 34 durations, the context menu shape and material, the file chooser shape, the wallpaper handoff, the control, the three shaders on the processor, what the controls mean and the token contracts\n' \
    "$head" "$toolkit_count"
