const LXB_WALLPAPER_KEY_LIGHT: vec3<f32> = vec3<f32>(-0.42, -0.66, 0.62);
const LXB_WALLPAPER_PI: f32 = 3.14159265;
const LXB_WALLPAPER_BAND_FOLD: f32 = 0.020;
const LXB_WALLPAPER_BAND_BOW: f32 = 0.50;
const LXB_WALLPAPER_BAND_SHARE: f32 = 0.88;
const LXB_WALLPAPER_BAND_EDGE_SAMPLES: f32 = 0.8;
const LXB_WALLPAPER_DISPERSION: f32 = 0.055;

fn lxb_wallpaper_bevel_rise(inset: f32) -> f32 {
    return sqrt(max(1.0 - (1.0 - inset) * (1.0 - inset), 0.0));
}

fn lxb_wallpaper_bevel_slope(inset: f32) -> f32 {
    return (1.0 - inset) / max(lxb_wallpaper_bevel_rise(inset), 0.16);
}

fn lxb_wallpaper_ambient_field(
    point: vec2<f32>,
    center: vec2<f32>,
    radius: vec2<f32>,
) -> f32 {
    let q = (point - center) / radius;
    return exp(-dot(q, q) * 1.65);
}

struct LxbWallpaperSpine {
    gather: f32,
    height: f32,
    slope: f32,
};

const LXB_WALLPAPER_SPINE_REST: f32 = 0.62;

fn lxb_wallpaper_spine_at(u: f32, t: f32, aspect: f32) -> LxbWallpaperSpine {
    let gather = 0.28 + 0.72 * sin(LXB_WALLPAPER_PI * u);
    let gather_slope = 0.72 * LXB_WALLPAPER_PI * cos(LXB_WALLPAPER_PI * u);

    let spine_a = u * 6.8 + t * 0.56;
    let spine_b = u * 3.4 - t * 0.39 + 0.8;
    let swing = sin(spine_a) * 0.055 + sin(spine_b) * 0.085;
    let swing_slope = cos(spine_a) * 0.055 * 6.8 + cos(spine_b) * 0.085 * 3.4;

    var spine: LxbWallpaperSpine;
    spine.gather = gather;
    spine.height = LXB_WALLPAPER_SPINE_REST + swing * gather;
    spine.slope = (swing_slope * gather + swing * gather_slope) / aspect;
    return spine;
}

fn lxb_wallpaper_water(
    into: vec3<f32>,
    uv: vec2<f32>,
    aspect: f32,
    time: f32,
    soften: f32,
    footprint: vec2<f32>,
    spine: LxbWallpaperSpine,
    accent: array<vec4<f32>, 3>,
) -> vec3<f32> {
    var color = into;
    let lip = 0.45;
    let ribbon_gloss = mix(1.0, 0.10, soften);
    let ribbon_wave = mix(1.0, 0.25, soften);
    let key = normalize(LXB_WALLPAPER_KEY_LIGHT);
    let half_vector = normalize(key + vec3<f32>(0.0, 0.0, 1.0));

    let gather = spine.gather;
    let slope = spine.slope;
    let across = normalize(vec2<f32>(slope, -1.0));
    let along = vec2<f32>(-across.y, across.x);
    let band = (uv.y - spine.height) / sqrt(1.0 + slope * slope);
    let sample = abs(across.x * footprint.x * aspect) + abs(across.y * footprint.y);

    var tilt = array<f32, 3>(0.0, 0.0, 0.0);
    var width = array<f32, 3>(0.0, 0.0, 0.0);
    for (var i = 0; i < 3; i = i + 1) {
        let fi = f32(i);
        let x = uv.x * (2.0 + fi * 0.6);
        let turning = sin(x * 2.10 - time * (0.23 + fi * 0.05) + fi * 1.9) * 0.95
            + sin(x * 1.25 + time * 0.15 + fi * 2.7) * 0.55
            + sin(x * 4.30 - time * 0.35 + fi * 0.7) * 0.22;
        tilt[i] = (turning * abs(turning) * 0.62
            + sin(x * 7.0 + time * 0.9 + fi) * 0.08) * ribbon_wave;
        let broad = sqrt(cos(tilt[i]) * cos(tilt[i]) + LXB_WALLPAPER_BAND_FOLD);
        width[i] = (0.0640 - 0.0110 * fi) * broad * mix(1.0, 1.5, soften);
    }

    let lift = LXB_WALLPAPER_BAND_SHARE
        * (0.32 + 0.68 * sin(uv.x * 4.3 - time * 0.37));
    let drop = LXB_WALLPAPER_BAND_SHARE
        * (0.32 + 0.68 * sin(uv.x * 3.1 + time * 0.29 + 2.2));
    let offset = array<f32, 3>(
        -(width[0] + width[1]) * lift * gather,
        0.0,
        (width[1] + width[2]) * drop * gather,
    );

    for (var i = 0; i < 3; i = i + 1) {
        let fi = f32(i);
        let x = uv.x * (2.0 + fi * 0.6);
        let d = band - offset[i];
        let sheet_width = width[i];
        let along_wave = (cos(x * 9.0 - time * 1.1 + fi * 2.0) * 0.34
            + cos(x * 23.0 + time * 1.9 + fi * 1.3) * 0.14) * ribbon_wave;

        let reach = abs(d / sheet_width);
        let s_across = clamp(d / sheet_width, -1.0, 1.0);
        let spread = LXB_WALLPAPER_BAND_EDGE_SAMPLES * sample / sheet_width;
        let cover = 1.0
            - smoothstep(mix(0.94, 0.40, soften) - spread, 1.0 + spread, reach);
        let inset = clamp((1.0 - reach) / lip, 0.0, 1.0);
        let rise = lxb_wallpaper_bevel_rise(inset);

        let wall = normalize(vec2<f32>(
            -sign(d) * lxb_wallpaper_bevel_slope(inset)
                - s_across * LXB_WALLPAPER_BAND_BOW,
            1.0,
        ));
        let turn = sin(tilt[i]);
        let level = cos(tilt[i]);
        let face = vec2<f32>(
            wall.x * level + wall.y * turn,
            wall.y * level - wall.x * turn,
        );
        let broad = sqrt(level * level + LXB_WALLPAPER_BAND_FOLD);
        let fold = min(1.0 / broad, 2.2);
        let surface = normalize(vec3<f32>(
            across * face.x + along * along_wave * face.y,
            face.y,
        ));

        let facing = clamp(dot(surface, key), 0.0, 1.0);
        let fresnel = 0.04 + 0.96 * pow(1.0 - clamp(surface.z, 0.0, 1.0), 5.0);
        let glint = pow(max(dot(surface, half_vector), 0.0), 42.0);
        let mirrored = reflect(vec3<f32>(0.0, 0.0, -1.0), surface);
        let room = 0.42 + 0.73 * (0.5 - 0.5 * dot(mirrored, key));
        let travelling = 0.60 + 0.40
            * sin(x * 3.1 - time * (0.9 + fi * 0.25) + fi);
        let focusing = 0.5 + 0.5 * sin(x * 2.3 - time * 0.5 + fi);
        let gathered = pow(max(sin(x * 23.0 + time * 1.9 + fi * 1.3
            + s_across * 3.4), 0.0), 5.0)
            * (0.15 + 0.85 * focusing * focusing);

        let depth = 1.0 - fi * 0.26;
        let skirt = exp(-d * d * mix(90.0, 45.0, soften));
        let shadow_d = d - sheet_width - 0.010;
        let shadow = exp(-shadow_d * shadow_d * mix(1400.0, 500.0, soften))
            * (1.0 - cover);
        let caustic_d = d - sheet_width - 0.0040;
        let caustic = exp(-caustic_d * caustic_d * mix(30000.0, 2000.0, soften))
            * broad;

        let haze_tint = mix(accent[0].rgb, accent[2].rgb, 0.46);
        let body_tint = mix(
            accent[0].rgb,
            accent[2].rgb,
            0.24 + fi * 0.05 + 0.36 * rise,
        );
        let split = LXB_WALLPAPER_DISPERSION * (1.0 - inset) * (1.0 - inset)
            * cover * depth * ribbon_gloss * -sign(d);

        let haze_strength = mix(1.0, 0.40, soften) * depth;
        let body_strength = mix(1.0, 0.55, soften) * depth;
        let water = cover * fold * body_strength;
        color *= 1.0 - cover * (0.05 + 0.11 * rise) * body_strength;
        color *= 1.0 - shadow * 0.20 * body_strength;
        color += haze_tint * skirt * 0.007 * haze_strength;
        color += body_tint * water * (0.005 + 0.012 * rise + 0.028 * facing);
        color += accent[1].rgb * fresnel * room * water * 0.100
            * mix(1.0, 0.45, soften);
        color += accent[1].rgb * glint * water * 0.120 * travelling
            * ribbon_gloss;
        color += accent[1].rgb * gathered * cover * rise * 0.011
            * depth * ribbon_gloss;
        color += accent[1].rgb * caustic * 0.011 * depth * ribbon_gloss;
        color *= vec3<f32>(1.0 + split, 1.0, 1.0 - split);
    }
    return color;
}

fn lxb_wallpaper_silk(
    into: vec3<f32>,
    uv: vec2<f32>,
    aspect: f32,
    time: f32,
    soften: f32,
    accent: array<vec4<f32>, 3>,
) -> vec3<f32> {
    var color = into;
    for (var i = 0; i < 3; i = i + 1) {
        let fi = f32(i);
        let speed = 0.42 + fi * 0.14;
        let lane = 0.62 + (fi - 1.0) * 0.050;
        let x_scale = 2.0 + fi * 0.6;
        let x = uv.x * x_scale;

        let phase_a = x * 2.6 + time * speed + fi * 2.1;
        let phase_b = x * 1.3 - time * speed * 0.7 + fi * 0.8;
        let center = lane + sin(phase_a) * 0.055 + sin(phase_b) * 0.085;
        let uv_slope = (cos(phase_a) * 0.055 * 2.6
            + cos(phase_b) * 0.085 * 1.3) * x_scale;
        let slope = uv_slope / aspect;
        let d = (uv.y - center) / sqrt(1.0 + slope * slope);

        let skirt = exp(-d * d * mix(320.0, 110.0, soften));
        let body = exp(-d * d * mix(5200.0, 680.0, soften));
        let bevel_d = d + mix(0.0045, 0.012, soften);
        let bevel = exp(-bevel_d * bevel_d * mix(20000.0, 1000.0, soften));
        let crest_d = d + mix(0.0065, 0.014, soften);
        let crest = exp(-crest_d * crest_d * mix(70000.0, 1400.0, soften));
        let fold_d = d - mix(0.008, 0.016, soften);
        let lower_fold = exp(-fold_d * fold_d * mix(9500.0, 900.0, soften));

        let upper_normal = normalize(vec2<f32>(slope, -1.0));
        let lamp_facing = max(
            dot(upper_normal, normalize(LXB_WALLPAPER_KEY_LIGHT.xy)),
            0.0,
        );
        let key_glint = 0.52 + 0.48 * pow(lamp_facing, 4.0);
        let travelling = 0.64 + 0.36
            * sin(x * 3.1 - time * (0.9 + fi * 0.25) + fi);

        let depth = 1.0 - fi * 0.18;
        let haze_strength = mix(1.0, 0.40, soften) * depth;
        let gloss_strength = mix(1.0, 0.10, soften) * depth;
        let haze_tint = mix(accent[0].rgb, accent[2].rgb, 0.46);
        let body_tint = mix(accent[0].rgb, accent[2].rgb, 0.28 + fi * 0.04);

        color += haze_tint * skirt * 0.020 * haze_strength;
        color += body_tint * body * 0.040 * haze_strength;
        color += accent[2].rgb * lower_fold * 0.010 * haze_strength;
        color += accent[1].rgb
            * (bevel * 0.022 * (0.82 + 0.18 * travelling)
                + crest * 0.010 * travelling * key_glint)
            * gloss_strength;
    }
    return color;
}

fn lxb_wallpaper_silk_spine(u: f32, t: f32, aspect: f32) -> LxbWallpaperSpine {
    let speed = 0.56;
    let x_scale = 2.6;
    let x = u * x_scale;
    let phase_a = x * 2.6 + t * speed + 2.1;
    let phase_b = x * 1.3 - t * speed * 0.7 + 0.8;
    var spine: LxbWallpaperSpine;
    spine.gather = 1.0;
    spine.height = LXB_WALLPAPER_SPINE_REST + sin(phase_a) * 0.055 + sin(phase_b) * 0.085;
    spine.slope = (cos(phase_a) * 0.055 * 2.6 + cos(phase_b) * 0.085 * 1.3) * x_scale / aspect;
    return spine;
}

const LXB_WALLPAPER_SPARKLE_LANE: f32 = 0.28;

const LXB_WALLPAPER_SPARKLE_BIRTH: f32 = 0.03;

const LXB_WALLPAPER_SPARKLE_SQUEEZE: f32 = 0.85;

const LXB_WALLPAPER_SPARKLE_SINK: f32 = 0.6;

const LXB_WALLPAPER_SPARKLE_STEEPEST: f32 = 0.95;

struct LxbWallpaperSparkleLayer {
    seed: u32,
    cell: vec2<f32>,
    drift: f32,
    rise: f32,
    density: f32,
    core: f32,
    reach: f32,
    travel: vec2<f32>,
    smallest: f32,
    brightness: vec2<f32>,
    push: vec2<f32>,
    hold: vec2<f32>,
};

const LXB_WALLPAPER_SPARKLE_DUST: LxbWallpaperSparkleLayer = LxbWallpaperSparkleLayer(
    0u,
    vec2<f32>(0.020, 0.023),
    0.018,
    0.016,
    0.90,
    0.0020,
    0.0070,
    vec2<f32>(0.05, 0.14),
    0.60,
    vec2<f32>(1.00, 0.15),
    vec2<f32>(0.03, 0.03),
    vec2<f32>(0.45, 0.15),
);

const LXB_WALLPAPER_SPARKLE_GLINTS: LxbWallpaperSparkleLayer = LxbWallpaperSparkleLayer(
    1013904223u,
    vec2<f32>(0.075, 0.085),
    0.022,
    0.018,
    0.60,
    0.0048,
    0.026,
    vec2<f32>(0.07, 0.18),
    0.35,
    vec2<f32>(0.85, 0.22),
    vec2<f32>(0.06, 0.02),
    vec2<f32>(0.35, 0.15),
);

fn lxb_wallpaper_sparkle_hash(value: u32) -> u32 {
    let state = value * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn lxb_wallpaper_sparkle_unit(value: u32) -> f32 {
    return f32(value >> 8u) / 16777216.0;
}

fn lxb_wallpaper_sparkle_bits(value: u32, shift: u32, mask: u32) -> f32 {
    return f32((value >> shift) & mask) / f32(mask + 1u);
}

fn lxb_wallpaper_sparkle_pushed(drift: f32, push: vec2<f32>) -> f32 {
    return drift + push.x * drift / (push.y + abs(drift));
}

fn lxb_wallpaper_sparkle_unpushed(out: f32, push: vec2<f32>) -> f32 {
    let d = abs(out);
    let b = push.y + push.x - d;
    let root = sqrt(b * b + 4.0 * d * push.y);
    let drift = select(0.5 * (root - b), 2.0 * d * push.y / (b + root), b > 0.0);
    return sign(out) * drift;
}

fn lxb_wallpaper_sparkle_hold(out: f32, hold: vec2<f32>) -> f32 {
    let a = max(out, 0.0);
    return (hold.y + hold.x * a) / (hold.y + a);
}

fn lxb_wallpaper_sparkle_unheld(offset: f32, held: f32, hold: vec2<f32>) -> f32 {
    if (offset <= held) {
        return offset - held;
    }
    let b = hold.y + held * hold.x - offset;
    let c = 4.0 * hold.y * (offset - held);
    let root = sqrt(b * b + c);
    return select(0.5 * (root - b), 2.0 * hold.y * (offset - held) / (b + root), b > 0.0);
}

fn lxb_wallpaper_sparkle_bump(x: f32) -> f32 {
    let q = max(1.0 - x * x, 0.0);
    return q * q * q;
}

fn lxb_wallpaper_sparkle_half(
    layer: LxbWallpaperSparkleLayer,
    side: f32,
    offset: f32,
    swing: f32,
    along: f32,
    slope: f32,
    lane: f32,
    t: f32,
    sample: f32,
    soften: f32,
) -> vec2<f32> {
    var light = vec2<f32>(0.0);
    let reach_across = layer.reach * sqrt(1.0 + slope * slope);
    let rise = layer.rise * select(LXB_WALLPAPER_SPARKLE_SINK, 1.0, side < 0.0);
    let half_seed = layer.seed ^ select(0u, 0x9e3779b9u, side < 0.0);
    let out_here = lxb_wallpaper_sparkle_unheld(side * offset, side * swing, layer.hold);
    let row_at = (lxb_wallpaper_sparkle_unpushed(out_here, layer.push) - rise * t) / layer.cell.y;
    let row_here = floor(row_at);
    let row_in = row_at - row_here;
    let row_side = select(-1.0, 1.0, row_in >= 0.5);
    let row_gap = select(row_in, 1.0 - row_in, row_in >= 0.5) * layer.cell.y;
    let rows = select(1, 2, row_gap * LXB_WALLPAPER_SPARKLE_SQUEEZE < reach_across);
    for (var r = 0; r < rows; r = r + 1) {
        let row = row_here + f32(r) * row_side;
        let row_seed = lxb_wallpaper_sparkle_hash(bitcast<u32>(i32(row)) + half_seed);
        let drifted = along - layer.drift * (0.6 + 0.8 * lxb_wallpaper_sparkle_unit(row_seed)) * t;
        let column_at = drifted / layer.cell.x;
        let column_here = floor(column_at);
        let column_in = column_at - column_here;
        let column_side = select(-1.0, 1.0, column_in >= 0.5);
        let column_gap = select(column_in, 1.0 - column_in, column_in >= 0.5) * layer.cell.x;
        let columns = select(1, 2, column_gap < layer.reach);
        for (var c = 0; c < columns; c = c + 1) {
            let column = column_here + f32(c) * column_side;
            let cell_seed = lxb_wallpaper_sparkle_hash(row_seed + bitcast<u32>(i32(column)));
            if (lxb_wallpaper_sparkle_unit(cell_seed) >= layer.density) {
                continue;
            }

            let shape = lxb_wallpaper_sparkle_hash(cell_seed);
            let look = lxb_wallpaper_sparkle_hash(shape);
            let phase = 2.0 * LXB_WALLPAPER_PI * lxb_wallpaper_sparkle_bits(look, 24u, 0xffu);
            let sway = sin(t * (0.12 + 0.20 * lxb_wallpaper_sparkle_bits(cell_seed, 0u, 0xffu)) + 2.0 * phase + 1.0)
                * 0.18;
            let d_along = (column + 0.5 + 0.60 * (lxb_wallpaper_sparkle_bits(shape, 0u, 0xffffu) - 0.5) + sway)
                * layer.cell.x - drifted;
            if (abs(d_along) >= layer.reach) {
                continue;
            }
            let strength = lxb_wallpaper_sparkle_bits(look, 0u, 0xffu);
            let grain = lxb_wallpaper_sparkle_bits(look, 8u, 0xffu);
            let pace = lxb_wallpaper_sparkle_bits(look, 16u, 0xffu);
            let wander = sin(t * (0.15 + 0.25 * pace) + phase) * 0.22;
            let out = lxb_wallpaper_sparkle_pushed((row + 0.5 + 0.56 * (lxb_wallpaper_sparkle_bits(shape, 16u, 0xffffu) - 0.5)
                + wander) * layer.cell.y + rise * t, layer.push);
            let hold = lxb_wallpaper_sparkle_hold(out, layer.hold);
            let d_across = hold * swing + side * out - offset + hold * slope * d_along;
            let apart = sqrt(d_along * d_along + d_across * d_across);
            let size = mix(layer.smallest, 1.0, grain * grain);
            let reach = layer.reach * size;
            if (apart >= reach) {
                continue;
            }

            let fate = lxb_wallpaper_sparkle_hash(look);
            let birth = LXB_WALLPAPER_SPARKLE_BIRTH * lxb_wallpaper_sparkle_bits(fate, 0u, 0xffffu);
            let travel = mix(layer.travel.x, layer.travel.y, lxb_wallpaper_sparkle_bits(fate, 16u, 0xffffu));
            let journey = out - birth;
            let left = clamp(1.0 - journey / travel, 0.0, 1.0);
            let life = smoothstep(0.0, 0.015, journey) * left * left * (3.0 - 2.0 * left);
            let twinkle = 0.75 + 0.25 * sin(t * (1.5 + 2.5 * grain) + phase);
            let fade = lxb_wallpaper_sparkle_bump(out / lane);
            let amount = (0.35 + 0.65 * strength * strength) * life * twinkle * fade * size;

            let radius = layer.core * size;
            let spread = min(sqrt(radius * radius + 2.25 * sample * sample
                + 0.25 * reach * reach * soften * soften), reach);
            let kept = radius / spread;
            let fall = 1.0 - apart / reach;
            light += amount * vec2<f32>(
                lxb_wallpaper_sparkle_bump(apart / spread) * kept * kept,
                fall * fall * fall,
            );
        }
    }
    return light;
}

fn lxb_wallpaper_sparkle_layer(
    layer: LxbWallpaperSparkleLayer,
    offset: f32,
    swing: f32,
    along: f32,
    slope: f32,
    lane: f32,
    t: f32,
    sample: f32,
    soften: f32,
) -> vec2<f32> {
    let side = select(-1.0, 1.0, offset >= swing);
    let reach_across = layer.reach * sqrt(1.0 + slope * slope);
    let lag = select(0.0, (1.0 - layer.hold.x) * abs(swing), side * swing < 0.0);
    if (abs(offset - swing) >= lane + lag + reach_across) {
        return vec2<f32>(0.0);
    }
    var light = lxb_wallpaper_sparkle_half(layer, side, offset, swing, along, slope, lane, t, sample, soften);
    if (abs(offset - swing) < reach_across) {
        light += lxb_wallpaper_sparkle_half(layer, -side, offset, swing, along, slope, lane, t, sample, soften);
    }
    return light;
}

fn lxb_wallpaper_sparkles(
    into: vec3<f32>,
    uv: vec2<f32>,
    aspect: f32,
    t: f32,
    soften: f32,
    footprint: vec2<f32>,
    spine: LxbWallpaperSpine,
    accent: array<vec4<f32>, 3>,
) -> vec3<f32> {
    let offset = uv.y - LXB_WALLPAPER_SPINE_REST;
    let swing = spine.height - LXB_WALLPAPER_SPINE_REST;
    let slope = clamp(spine.slope, -LXB_WALLPAPER_SPARKLE_STEEPEST, LXB_WALLPAPER_SPARKLE_STEEPEST);
    let lane = LXB_WALLPAPER_SPARKLE_LANE * (0.45 + 0.55 * spine.gather);
    let behind = (offset - swing) * swing < 0.0;
    let lag = select(0.0, (1.0 - min(LXB_WALLPAPER_SPARKLE_DUST.hold.x, LXB_WALLPAPER_SPARKLE_GLINTS.hold.x)) * abs(swing),
                     behind);
    if (abs(offset - swing) >= lane + lag + LXB_WALLPAPER_SPARKLE_GLINTS.reach * sqrt(1.0 + slope * slope)) {
        return into;
    }
    let along = uv.x * aspect;
    let sample = max(footprint.x * aspect, footprint.y);
    let dust = lxb_wallpaper_sparkle_layer(LXB_WALLPAPER_SPARKLE_DUST, offset, swing, along, slope, lane, t, sample, soften);
    let glints = lxb_wallpaper_sparkle_layer(LXB_WALLPAPER_SPARKLE_GLINTS, offset, swing, along, slope, lane, t, sample, soften);
    let core = dust.x * LXB_WALLPAPER_SPARKLE_DUST.brightness.x + glints.x * LXB_WALLPAPER_SPARKLE_GLINTS.brightness.x;
    let glow = dust.y * LXB_WALLPAPER_SPARKLE_DUST.brightness.y + glints.y * LXB_WALLPAPER_SPARKLE_GLINTS.brightness.y;
    let hot = mix(accent[1].rgb, vec3<f32>(1.0), 0.45);
    let haze = mix(accent[0].rgb, accent[1].rgb, 0.5);
    return into + (hot * core + haze * glow) * mix(1.0, 0.5, soften);
}

fn lxb_wallpaper(
    uv: vec2<f32>,
    aspect: f32,
    time: f32,
    soften: f32,
    style: f32,
    particles: f32,
    sky: array<vec4<f32>, 4>,
    accent: array<vec4<f32>, 3>,
    glow: vec4<f32>,
    footprint: vec2<f32>,
) -> vec3<f32> {
    let mood = 0.5 + 0.5 * sin(time * 0.03);
    let top = mix(sky[0].rgb, sky[2].rgb, mood);
    let bottom = mix(sky[1].rgb, sky[3].rgb, mood);
    let gradient_y = uv.y
        + sin(uv.x * 2.7 + time * 0.075) * 0.045
        + sin(uv.x * 5.3 - time * 0.052) * 0.018;
    var color = mix(top, bottom, smoothstep(0.0, 1.0, gradient_y)) * 0.42;

    let glow_center = vec2<f32>(0.24, 0.34);
    let glow_strength = 1.0 - smoothstep(
        0.0,
        0.8,
        distance(
            uv * vec2<f32>(aspect, 1.0),
            glow_center * vec2<f32>(aspect, 1.0),
        ),
    );
    color += glow.rgb * glow_strength * 0.25;

    let point = vec2<f32>((uv.x - 0.5) * aspect, uv.y - 0.5);
    let warp = vec2<f32>(
        sin(point.y * 3.8 + time * 0.11)
            + sin((point.x + point.y) * 2.1 - time * 0.071),
        sin(point.x * 2.6 - time * 0.093)
            + sin((point.x - point.y) * 2.4 + time * 0.063),
    ) * 0.035;
    let flowed = point + warp;
    let spread = mix(1.0, 1.45, soften);

    let deep = lxb_wallpaper_ambient_field(
        flowed,
        vec2<f32>(
            -aspect * 0.34 + sin(time * 0.083) * aspect * 0.28,
            -0.23 + cos(time * 0.067) * 0.18,
        ),
        vec2<f32>(aspect * 0.36, 0.34) * spread,
    );
    let main = lxb_wallpaper_ambient_field(
        flowed,
        vec2<f32>(
            aspect * 0.31 + cos(time * 0.061) * aspect * 0.30,
            0.20 + sin(time * 0.089) * 0.20,
        ),
        vec2<f32>(aspect * 0.34, 0.38) * spread,
    );
    let soft = lxb_wallpaper_ambient_field(
        flowed,
        vec2<f32>(
            sin(time * 0.047 + 2.0) * aspect * 0.42,
            sin(time * 0.073 + 1.1) * 0.30,
        ),
        vec2<f32>(aspect * 0.40, 0.29) * spread,
    );

    let field_strength = mix(1.0, 0.48, soften);
    color += accent[2].rgb * deep * 0.16 * field_strength;
    color += accent[0].rgb * main * 0.085 * field_strength;
    color += accent[1].rgb * soft * 0.035 * field_strength;

    let current = sin(flowed.x * 2.15 + flowed.y * 1.25 + time * 0.13)
        + sin(flowed.x * 0.78 - flowed.y * 2.35 - time * 0.087);
    let current_light = smoothstep(0.32, 1.62, current);
    color += accent[0].rgb * current_light * 0.035
        * mix(1.0, 0.40, soften);

    var spine = lxb_wallpaper_spine_at(uv.x, time, aspect);
    if (style > 0.5 && style < 1.5) {
        color = lxb_wallpaper_silk(color, uv, aspect, time, soften, accent);
        spine = lxb_wallpaper_silk_spine(uv.x, time, aspect);
    } else {
        color = lxb_wallpaper_water(
            color,
            uv,
            aspect,
            time,
            soften,
            footprint,
            spine,
            accent,
        );
    }
    if (particles > 0.5) {
        color = lxb_wallpaper_sparkles(
            color,
            uv,
            aspect,
            time,
            soften,
            footprint,
            spine,
            accent,
        );
    }

    let upper_center = 0.28
        + sin(uv.x * 2.6 + time * 0.16) * 0.10
        + sin(uv.x * 5.4 - time * 0.11) * 0.040;
    let upper_d = uv.y - upper_center;
    let upper_veil = exp(-upper_d * upper_d * mix(32.0, 13.0, soften));
    let upper_crest = exp(-upper_d * upper_d * mix(230.0, 55.0, soften));
    let upper_sheen = 0.72 + 0.28 * sin(uv.x * 4.2 - time * 0.22);

    let lower_center = 0.72
        + sin(uv.x * 2.1 - time * 0.13 + 2.4) * 0.12
        + sin(uv.x * 4.7 + time * 0.083) * 0.035;
    let lower_d = uv.y - lower_center;
    let lower_veil = exp(-lower_d * lower_d * mix(26.0, 11.0, soften));
    let lower_crest = exp(-lower_d * lower_d * mix(180.0, 45.0, soften));
    let lower_sheen = 0.74 + 0.26 * sin(uv.x * 3.7 + time * 0.18 + 1.7);

    let veil_strength = mix(1.0, 0.38, soften);
    color += accent[0].rgb
        * (upper_veil * 0.052 + upper_crest * 0.025)
        * upper_sheen * veil_strength;
    color += (accent[2].rgb * lower_veil * 0.16
        + accent[0].rgb * lower_crest * 0.028)
        * lower_sheen * veil_strength;

    let edge = distance(uv, vec2<f32>(0.5, 0.5));
    color *= 1.0 - smoothstep(0.55, 1.05, edge) * 0.55;
    return color * mix(1.0, 0.55, soften);
}
