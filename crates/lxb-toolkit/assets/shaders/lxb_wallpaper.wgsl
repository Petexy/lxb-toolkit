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

fn lxb_wallpaper_water(
    into: vec3<f32>,
    uv: vec2<f32>,
    aspect: f32,
    time: f32,
    soften: f32,
    footprint: vec2<f32>,
    accent: array<vec4<f32>, 3>,
) -> vec3<f32> {
    var color = into;
    let lip = 0.45;
    let ribbon_gloss = mix(1.0, 0.10, soften);
    let ribbon_wave = mix(1.0, 0.25, soften);
    let key = normalize(LXB_WALLPAPER_KEY_LIGHT);
    let half_vector = normalize(key + vec3<f32>(0.0, 0.0, 1.0));

    let gather = 0.28 + 0.72 * sin(LXB_WALLPAPER_PI * uv.x);
    let gather_slope = 0.72 * LXB_WALLPAPER_PI * cos(LXB_WALLPAPER_PI * uv.x);

    let spine_a = uv.x * 6.8 + time * 0.56;
    let spine_b = uv.x * 3.4 - time * 0.39 + 0.8;
    let swing = sin(spine_a) * 0.055 + sin(spine_b) * 0.085;
    let swing_slope = cos(spine_a) * 0.055 * 6.8 + cos(spine_b) * 0.085 * 3.4;
    let spine = 0.62 + swing * gather;
    let slope = (swing_slope * gather + swing * gather_slope) / aspect;
    let across = normalize(vec2<f32>(slope, -1.0));
    let along = vec2<f32>(-across.y, across.x);
    let band = (uv.y - spine) / sqrt(1.0 + slope * slope);
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

fn lxb_wallpaper(
    uv: vec2<f32>,
    aspect: f32,
    time: f32,
    soften: f32,
    style: f32,
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

    if (style > 0.5 && style < 1.5) {
        color = lxb_wallpaper_silk(color, uv, aspect, time, soften, accent);
    } else {
        color = lxb_wallpaper_water(
            color,
            uv,
            aspect,
            time,
            soften,
            footprint,
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
