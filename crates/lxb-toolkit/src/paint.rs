use crate::{
    color, glyph_material as mark,
    material::{optics, Glass},
    settings::WallpaperStyle,
    wallpaper::{
        ShaderPalette, BAND_BOW, BAND_EDGE_SAMPLES, BAND_FOLD, BAND_SHARE,
        KEY_LIGHT as WALLPAPER_KEY_LIGHT,
    },
};

pub struct Canvas<'a> {
    pixels: &'a mut [u8],
    width: u32,
    height: u32,
    stride: usize,
}

impl<'a> Canvas<'a> {
    pub fn new(pixels: &'a mut [u8], width: u32, height: u32, stride: usize) -> Option<Self> {
        if width == 0 || height == 0 || stride < width as usize * 4 {
            return None;
        }
        let last = (height as usize).checked_sub(1)?.checked_mul(stride)?;
        if pixels.len() < last + width as usize * 4 {
            return None;
        }
        Some(Self {
            pixels,
            width,
            height,
            stride,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}

fn read(row: &[u8], x: usize) -> [f32; 3] {
    let word = u32::from_ne_bytes([row[x * 4], row[x * 4 + 1], row[x * 4 + 2], row[x * 4 + 3]]);
    [
        from_byte((word >> 16) & 0xff),
        from_byte((word >> 8) & 0xff),
        from_byte(word & 0xff),
    ]
}

struct Transfer {
    decode: [f32; 256],
    edges: [f32; 255],
}

fn transfer() -> &'static Transfer {
    static TRANSFER: std::sync::OnceLock<Transfer> = std::sync::OnceLock::new();
    TRANSFER.get_or_init(|| {
        let mut decode = [0.0; 256];
        for (byte, value) in decode.iter_mut().enumerate() {
            *value = color::to_linear(byte as f32 / 255.0);
        }
        let mut edges = [0.0; 255];
        for (byte, edge) in edges.iter_mut().enumerate() {
            *edge = color::to_linear((byte as f32 + 0.5) / 255.0);
        }
        Transfer { decode, edges }
    })
}

fn from_byte(byte: u32) -> f32 {
    transfer().decode[(byte & 0xff) as usize]
}

fn encode(value: f32) -> u32 {
    let value = value.clamp(0.0, 1.0);
    transfer().edges.partition_point(|&edge| edge <= value) as u32
}

fn over(row: &mut [u8], x: usize, color: [f32; 3], alpha: f32) {
    let alpha = alpha.clamp(0.0, 1.0);
    if alpha <= 0.0 {
        return;
    }
    let under = read(row, x);
    let word = u32::from_ne_bytes([row[x * 4], row[x * 4 + 1], row[x * 4 + 2], row[x * 4 + 3]]);
    let below = ((word >> 24) & 0xff) as f32 / 255.0;
    let keep = 1.0 - alpha;
    let out = [
        color[0] * alpha + under[0] * keep,
        color[1] * alpha + under[1] * keep,
        color[2] * alpha + under[2] * keep,
    ];
    let total = alpha + below * keep;
    let word = ((total * 255.0 + 0.5) as u32) << 24
        | encode(out[0]) << 16
        | encode(out[1]) << 8
        | encode(out[2]);
    row[x * 4..x * 4 + 4].copy_from_slice(&word.to_ne_bytes());
}

fn in_bands<F>(pixels: &mut [u8], stride: usize, height: u32, draw: F)
where
    F: Fn(&mut [u8], u32) + Sync,
{
    let cores = std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(1);

    let bands = cores.max(1).min((height as usize / 16).max(1));
    if bands == 1 {
        draw(pixels, 0);
        return;
    }
    let rows = height as usize / bands + usize::from(height as usize % bands != 0);

    std::thread::scope(|scope| {
        for (band, slice) in pixels.chunks_mut(rows * stride).enumerate() {
            let first = (band * rows) as u32;
            let draw = &draw;
            scope.spawn(move || draw(slice, first));
        }
    });
}

fn mix(from: f32, to: f32, amount: f32) -> f32 {
    from + (to - from) * amount
}

fn mix3(from: [f32; 3], to: [f32; 3], amount: f32) -> [f32; 3] {
    [
        mix(from[0], to[0], amount),
        mix(from[1], to[1], amount),
        mix(from[2], to[2], amount),
    ]
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalize2(v: [f32; 2]) -> [f32; 2] {
    let len = (v[0] * v[0] + v[1] * v[1]).sqrt();
    if len < 1e-20 {
        return [0.0, 0.0];
    }
    [v[0] / len, v[1] / len]
}

fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let len = dot3(v, v).sqrt();
    if len < 1e-20 {
        return [0.0, 0.0, 0.0];
    }
    [v[0] / len, v[1] / len, v[2] / len]
}

fn reflect(incident: [f32; 3], normal: [f32; 3]) -> [f32; 3] {
    let twice = 2.0 * dot3(normal, incident);
    [
        incident[0] - twice * normal[0],
        incident[1] - twice * normal[1],
        incident[2] - twice * normal[2],
    ]
}

fn refract(incident: [f32; 3], normal: [f32; 3], eta: f32) -> [f32; 3] {
    let cosine = dot3(normal, incident);
    let k = 1.0 - eta * eta * (1.0 - cosine * cosine);
    if k < 0.0 {
        return [0.0, 0.0, 0.0];
    }
    let scale = eta * cosine + k.sqrt();
    [
        eta * incident[0] - scale * normal[0],
        eta * incident[1] - scale * normal[1],
        eta * incident[2] - scale * normal[2],
    ]
}

fn accumulate(color: &mut [f32; 3], tint: [f32; 3], amount: f32) {
    color[0] += tint[0] * amount;
    color[1] += tint[1] * amount;
    color[2] += tint[2] * amount;
}

fn attenuate(color: &mut [f32; 3], amount: f32) {
    color[0] *= amount;
    color[1] *= amount;
    color[2] *= amount;
}

#[derive(Debug, Clone, Copy)]
pub struct Scene {
    pub time: f32,

    pub soften: f32,

    pub style: WallpaperStyle,

    pub palette: ShaderPalette,
}

impl Scene {
    pub fn new(palette: &crate::palette::Palette, time: f32) -> Self {
        Self {
            time,
            soften: 0.0,
            style: WallpaperStyle::Default,
            palette: ShaderPalette::from_palette(palette),
        }
    }
}

pub fn wallpaper(canvas: &mut Canvas, scene: &Scene) {
    let width = canvas.width;
    let height = canvas.height;
    let stride = canvas.stride;
    let aspect = width as f32 / height as f32;
    let footprint = [1.0 / width as f32, 1.0 / height as f32];
    in_bands(canvas.pixels, stride, height, |band, first| {
        for (row, pixels) in band.chunks_mut(stride).enumerate() {
            let y = first + row as u32;
            if y >= height {
                break;
            }
            let v = (y as f32 + 0.5) / height as f32;
            for x in 0..width as usize {
                let u = (x as f32 + 0.5) / width as f32;
                let color = sample(scene, [u, v], aspect, footprint);
                let word =
                    0xff00_0000 | encode(color[0]) << 16 | encode(color[1]) << 8 | encode(color[2]);
                pixels[x * 4..x * 4 + 4].copy_from_slice(&word.to_ne_bytes());
            }
        }
    });
}

pub fn sample(scene: &Scene, uv: [f32; 2], aspect: f32, footprint: [f32; 2]) -> [f32; 3] {
    let (u, v) = (uv[0], uv[1]);
    let time = scene.time;
    let soften = scene.soften;
    let sky = |index: usize| {
        let c = scene.palette.sky[index];
        [c[0], c[1], c[2]]
    };
    let accent = |index: usize| {
        let c = scene.palette.accent[index];
        [c[0], c[1], c[2]]
    };
    let glow = [
        scene.palette.glow[0],
        scene.palette.glow[1],
        scene.palette.glow[2],
    ];

    let mood = 0.5 + 0.5 * (time * 0.03).sin();
    let top = mix3(sky(0), sky(2), mood);
    let bottom = mix3(sky(1), sky(3), mood);
    let gradient_y =
        v + (u * 2.7 + time * 0.075).sin() * 0.045 + (u * 5.3 - time * 0.052).sin() * 0.018;
    let mut color = mix3(top, bottom, smoothstep(0.0, 1.0, gradient_y));
    attenuate(&mut color, 0.42);

    let glow_center = [0.24, 0.34];
    let reach = [(u - glow_center[0]) * aspect, v - glow_center[1]];
    let glow_strength =
        1.0 - smoothstep(0.0, 0.8, (reach[0] * reach[0] + reach[1] * reach[1]).sqrt());
    accumulate(&mut color, glow, glow_strength * 0.25);

    let point = [(u - 0.5) * aspect, v - 0.5];
    let warp = [
        ((point[1] * 3.8 + time * 0.11).sin() + ((point[0] + point[1]) * 2.1 - time * 0.071).sin())
            * 0.035,
        ((point[0] * 2.6 - time * 0.093).sin()
            + ((point[0] - point[1]) * 2.4 + time * 0.063).sin())
            * 0.035,
    ];
    let flowed = [point[0] + warp[0], point[1] + warp[1]];
    let spread = mix(1.0, 1.45, soften);

    let deep = ambient_field(
        flowed,
        [
            -aspect * 0.34 + (time * 0.083).sin() * aspect * 0.28,
            -0.23 + (time * 0.067).cos() * 0.18,
        ],
        [aspect * 0.36 * spread, 0.34 * spread],
    );
    let main = ambient_field(
        flowed,
        [
            aspect * 0.31 + (time * 0.061).cos() * aspect * 0.30,
            0.20 + (time * 0.089).sin() * 0.20,
        ],
        [aspect * 0.34 * spread, 0.38 * spread],
    );
    let soft = ambient_field(
        flowed,
        [
            (time * 0.047 + 2.0).sin() * aspect * 0.42,
            (time * 0.073 + 1.1).sin() * 0.30,
        ],
        [aspect * 0.40 * spread, 0.29 * spread],
    );

    let field_strength = mix(1.0, 0.48, soften);
    accumulate(&mut color, accent(2), deep * 0.16 * field_strength);
    accumulate(&mut color, accent(0), main * 0.085 * field_strength);
    accumulate(&mut color, accent(1), soft * 0.035 * field_strength);

    let current = (flowed[0] * 2.15 + flowed[1] * 1.25 + time * 0.13).sin()
        + (flowed[0] * 0.78 - flowed[1] * 2.35 - time * 0.087).sin();
    let current_light = smoothstep(0.32, 1.62, current);
    accumulate(
        &mut color,
        accent(0),
        current_light * 0.035 * mix(1.0, 0.40, soften),
    );

    color = match scene.style {
        WallpaperStyle::Simple => silk(color, scene, uv, aspect),

        WallpaperStyle::Default | WallpaperStyle::Custom => {
            water(color, scene, uv, aspect, footprint)
        }
    };

    let upper_center =
        0.28 + (u * 2.6 + time * 0.16).sin() * 0.10 + (u * 5.4 - time * 0.11).sin() * 0.040;
    let upper_d = v - upper_center;
    let upper_veil = (-upper_d * upper_d * mix(32.0, 13.0, soften)).exp();
    let upper_crest = (-upper_d * upper_d * mix(230.0, 55.0, soften)).exp();
    let upper_sheen = 0.72 + 0.28 * (u * 4.2 - time * 0.22).sin();

    let lower_center =
        0.72 + (u * 2.1 - time * 0.13 + 2.4).sin() * 0.12 + (u * 4.7 + time * 0.083).sin() * 0.035;
    let lower_d = v - lower_center;
    let lower_veil = (-lower_d * lower_d * mix(26.0, 11.0, soften)).exp();
    let lower_crest = (-lower_d * lower_d * mix(180.0, 45.0, soften)).exp();
    let lower_sheen = 0.74 + 0.26 * (u * 3.7 + time * 0.18 + 1.7).sin();

    let veil_strength = mix(1.0, 0.38, soften);
    accumulate(
        &mut color,
        accent(0),
        (upper_veil * 0.052 + upper_crest * 0.025) * upper_sheen * veil_strength,
    );
    let lower = [
        accent(2)[0] * lower_veil * 0.16 + accent(0)[0] * lower_crest * 0.028,
        accent(2)[1] * lower_veil * 0.16 + accent(0)[1] * lower_crest * 0.028,
        accent(2)[2] * lower_veil * 0.16 + accent(0)[2] * lower_crest * 0.028,
    ];
    accumulate(&mut color, lower, lower_sheen * veil_strength);

    let edge = ((u - 0.5) * (u - 0.5) + (v - 0.5) * (v - 0.5)).sqrt();
    attenuate(&mut color, 1.0 - smoothstep(0.55, 1.05, edge) * 0.55);
    attenuate(&mut color, mix(1.0, 0.55, soften));
    color
}

fn ambient_field(point: [f32; 2], center: [f32; 2], radius: [f32; 2]) -> f32 {
    let q = [
        (point[0] - center[0]) / radius[0],
        (point[1] - center[1]) / radius[1],
    ];
    (-(q[0] * q[0] + q[1] * q[1]) * 1.65).exp()
}

fn bevel_rise(inset: f32) -> f32 {
    (1.0 - (1.0 - inset) * (1.0 - inset)).max(0.0).sqrt()
}

fn bevel_slope(inset: f32) -> f32 {
    (1.0 - inset) / bevel_rise(inset).max(0.16)
}

fn water(
    into: [f32; 3],
    scene: &Scene,
    uv: [f32; 2],
    aspect: f32,
    footprint: [f32; 2],
) -> [f32; 3] {
    let (u, v) = (uv[0], uv[1]);
    let time = scene.time;
    let soften = scene.soften;
    let accent = |index: usize| {
        let c = scene.palette.accent[index];
        [c[0], c[1], c[2]]
    };
    let mut color = into;

    let lip = 0.45;
    let ribbon_gloss = mix(1.0, 0.10, soften);
    let ribbon_wave = mix(1.0, 0.25, soften);
    let key = normalize3(WALLPAPER_KEY_LIGHT);
    let half_vector = normalize3([key[0], key[1], key[2] + 1.0]);

    let gather = 0.28 + 0.72 * (std::f32::consts::PI * u).sin();
    let gather_slope = 0.72 * std::f32::consts::PI * (std::f32::consts::PI * u).cos();

    let spine_a = u * 6.8 + time * 0.56;
    let spine_b = u * 3.4 - time * 0.39 + 0.8;
    let swing = spine_a.sin() * 0.055 + spine_b.sin() * 0.085;
    let swing_slope = spine_a.cos() * 0.055 * 6.8 + spine_b.cos() * 0.085 * 3.4;
    let spine = 0.62 + swing * gather;
    let slope = (swing_slope * gather + swing * gather_slope) / aspect;
    let across = normalize2([slope, -1.0]);
    let along = [-across[1], across[0]];
    let band = (v - spine) / (1.0 + slope * slope).sqrt();
    let sample_size = (across[0] * footprint[0] * aspect).abs() + (across[1] * footprint[1]).abs();

    let mut tilt = [0.0_f32; 3];
    let mut width = [0.0_f32; 3];
    for i in 0..3 {
        let fi = i as f32;
        let x = u * (2.0 + fi * 0.6);
        let turning = (x * 2.10 - time * (0.23 + fi * 0.05) + fi * 1.9).sin() * 0.95
            + (x * 1.25 + time * 0.15 + fi * 2.7).sin() * 0.55
            + (x * 4.30 - time * 0.35 + fi * 0.7).sin() * 0.22;
        tilt[i] = (turning * turning.abs() * 0.62 + (x * 7.0 + time * 0.9 + fi).sin() * 0.08)
            * ribbon_wave;
        let broad = (tilt[i].cos() * tilt[i].cos() + BAND_FOLD).sqrt();
        width[i] = (0.0640 - 0.0110 * fi) * broad * mix(1.0, 1.5, soften);
    }

    let lift = BAND_SHARE * (0.32 + 0.68 * (u * 4.3 - time * 0.37).sin());
    let drop = BAND_SHARE * (0.32 + 0.68 * (u * 3.1 + time * 0.29 + 2.2).sin());
    let offset = [
        -(width[0] + width[1]) * lift * gather,
        0.0,
        (width[1] + width[2]) * drop * gather,
    ];

    for i in 0..3 {
        let fi = i as f32;
        let x = u * (2.0 + fi * 0.6);
        let d = band - offset[i];
        let sheet_width = width[i];
        let along_wave = ((x * 9.0 - time * 1.1 + fi * 2.0).cos() * 0.34
            + (x * 23.0 + time * 1.9 + fi * 1.3).cos() * 0.14)
            * ribbon_wave;

        let reach = (d / sheet_width).abs();
        let s_across = (d / sheet_width).clamp(-1.0, 1.0);
        let spread = BAND_EDGE_SAMPLES * sample_size / sheet_width;
        let cover = 1.0 - smoothstep(mix(0.94, 0.40, soften) - spread, 1.0 + spread, reach);
        let inset = ((1.0 - reach) / lip).clamp(0.0, 1.0);
        let rise = bevel_rise(inset);

        let wall = normalize2([-d.signum() * bevel_slope(inset) - s_across * BAND_BOW, 1.0]);
        let turn = tilt[i].sin();
        let level = tilt[i].cos();
        let face = [
            wall[0] * level + wall[1] * turn,
            wall[1] * level - wall[0] * turn,
        ];
        let broad = (level * level + BAND_FOLD).sqrt();
        let fold = (1.0 / broad).min(2.2);
        let surface = normalize3([
            across[0] * face[0] + along[0] * along_wave * face[1],
            across[1] * face[0] + along[1] * along_wave * face[1],
            face[1],
        ]);

        let facing = dot3(surface, key).clamp(0.0, 1.0);
        let fresnel = 0.04 + 0.96 * (1.0 - surface[2].clamp(0.0, 1.0)).powi(5);
        let glint = dot3(surface, half_vector).max(0.0).powi(42);
        let mirrored = reflect([0.0, 0.0, -1.0], surface);
        let room = 0.42 + 0.73 * (0.5 - 0.5 * dot3(mirrored, key));
        let travelling = 0.60 + 0.40 * (x * 3.1 - time * (0.9 + fi * 0.25) + fi).sin();
        let focusing = 0.5 + 0.5 * (x * 2.3 - time * 0.5 + fi).sin();
        let gathered = (x * 23.0 + time * 1.9 + fi * 1.3 + s_across * 3.4)
            .sin()
            .max(0.0)
            .powi(5)
            * (0.15 + 0.85 * focusing * focusing);

        let depth = 1.0 - fi * 0.26;
        let skirt = (-d * d * mix(90.0, 45.0, soften)).exp();
        let shadow_d = d - sheet_width - 0.010;
        let shadow = (-shadow_d * shadow_d * mix(1400.0, 500.0, soften)).exp() * (1.0 - cover);
        let caustic_d = d - sheet_width - 0.0040;
        let caustic = (-caustic_d * caustic_d * mix(30000.0, 2000.0, soften)).exp() * broad;

        let haze_tint = mix3(accent(0), accent(2), 0.46);
        let body_tint = mix3(accent(0), accent(2), 0.24 + fi * 0.05 + 0.36 * rise);
        let split = optics::DISPERSION
            * (1.0 - inset)
            * (1.0 - inset)
            * cover
            * depth
            * ribbon_gloss
            * -d.signum();

        let haze_strength = mix(1.0, 0.40, soften) * depth;
        let body_strength = mix(1.0, 0.55, soften) * depth;
        let sheet = cover * fold * body_strength;
        attenuate(
            &mut color,
            1.0 - cover * (0.05 + 0.11 * rise) * body_strength,
        );
        attenuate(&mut color, 1.0 - shadow * 0.20 * body_strength);
        accumulate(&mut color, haze_tint, skirt * 0.007 * haze_strength);
        accumulate(
            &mut color,
            body_tint,
            sheet * (0.005 + 0.012 * rise + 0.028 * facing),
        );
        accumulate(
            &mut color,
            accent(1),
            fresnel * room * sheet * 0.100 * mix(1.0, 0.45, soften),
        );
        accumulate(
            &mut color,
            accent(1),
            glint * sheet * 0.120 * travelling * ribbon_gloss,
        );
        accumulate(
            &mut color,
            accent(1),
            gathered * cover * rise * 0.011 * depth * ribbon_gloss,
        );
        accumulate(
            &mut color,
            accent(1),
            caustic * 0.011 * depth * ribbon_gloss,
        );
        color[0] *= 1.0 + split;
        color[2] *= 1.0 - split;
    }
    color
}

fn silk(into: [f32; 3], scene: &Scene, uv: [f32; 2], aspect: f32) -> [f32; 3] {
    let (u, v) = (uv[0], uv[1]);
    let time = scene.time;
    let soften = scene.soften;
    let accent = |index: usize| {
        let c = scene.palette.accent[index];
        [c[0], c[1], c[2]]
    };
    let mut color = into;

    for i in 0..3 {
        let fi = i as f32;
        let speed = 0.42 + fi * 0.14;
        let lane = 0.62 + (fi - 1.0) * 0.050;
        let x_scale = 2.0 + fi * 0.6;
        let x = u * x_scale;

        let phase_a = x * 2.6 + time * speed + fi * 2.1;
        let phase_b = x * 1.3 - time * speed * 0.7 + fi * 0.8;
        let center = lane + phase_a.sin() * 0.055 + phase_b.sin() * 0.085;
        let uv_slope = (phase_a.cos() * 0.055 * 2.6 + phase_b.cos() * 0.085 * 1.3) * x_scale;
        let slope = uv_slope / aspect;
        let d = (v - center) / (1.0 + slope * slope).sqrt();

        let skirt = (-d * d * mix(320.0, 110.0, soften)).exp();
        let body = (-d * d * mix(5200.0, 680.0, soften)).exp();
        let bevel_d = d + mix(0.0045, 0.012, soften);
        let bevel = (-bevel_d * bevel_d * mix(20000.0, 1000.0, soften)).exp();
        let crest_d = d + mix(0.0065, 0.014, soften);
        let crest = (-crest_d * crest_d * mix(70000.0, 1400.0, soften)).exp();
        let fold_d = d - mix(0.008, 0.016, soften);
        let lower_fold = (-fold_d * fold_d * mix(9500.0, 900.0, soften)).exp();

        let upper_normal = normalize2([slope, -1.0]);
        let lamp = normalize2([WALLPAPER_KEY_LIGHT[0], WALLPAPER_KEY_LIGHT[1]]);
        let lamp_facing = (upper_normal[0] * lamp[0] + upper_normal[1] * lamp[1]).max(0.0);
        let key_glint = 0.52 + 0.48 * lamp_facing.powi(4);
        let travelling = 0.64 + 0.36 * (x * 3.1 - time * (0.9 + fi * 0.25) + fi).sin();

        let depth = 1.0 - fi * 0.18;
        let haze_strength = mix(1.0, 0.40, soften) * depth;
        let gloss_strength = mix(1.0, 0.10, soften) * depth;
        let haze_tint = mix3(accent(0), accent(2), 0.46);
        let body_tint = mix3(accent(0), accent(2), 0.28 + fi * 0.04);

        accumulate(&mut color, haze_tint, skirt * 0.020 * haze_strength);
        accumulate(&mut color, body_tint, body * 0.040 * haze_strength);
        accumulate(&mut color, accent(2), lower_fold * 0.010 * haze_strength);
        accumulate(
            &mut color,
            accent(1),
            (bevel * 0.022 * (0.82 + 0.18 * travelling) + crest * 0.010 * travelling * key_glint)
                * gloss_strength,
        );
    }
    color
}

pub fn light(canvas: &mut Canvas, rect: [f32; 4], tint: [f32; 4]) {
    let [x, y, width, height] = rect;
    if width <= 0.0 || height <= 0.0 || tint[3] <= 0.0 {
        return;
    }
    let half = [(width * 0.5).max(1.0), (height * 0.5).max(1.0)];
    let centre = [x + width * 0.5, y + height * 0.5];

    let first = y.floor().max(0.0) as u32;
    let last = ((y + height).ceil() as i64).clamp(0, canvas.height as i64) as u32;
    let left = x.floor().max(0.0) as usize;
    let right = ((x + width).ceil() as i64).clamp(0, canvas.width as i64) as usize;
    if first >= last || left >= right {
        return;
    }

    let stride = canvas.stride;
    let start = first as usize * stride;
    let end = (last as usize * stride).min(canvas.pixels.len());
    let rows = &mut canvas.pixels[start..end];
    in_bands(rows, stride, last - first, |band, band_first| {
        for (row, line) in band.chunks_mut(stride).enumerate() {
            let py = (first + band_first + row as u32) as f32 + 0.5;
            for column in left..right {
                let px = column as f32 + 0.5;
                let q = [(px - centre[0]) / half[0], (py - centre[1]) / half[1]];
                let radius = (q[0] * q[0] + q[1] * q[1]).sqrt();
                let gaussian = (-5.5 * radius * radius).exp();
                let sealed = ((1.0 - radius) / 0.12).clamp(0.0, 1.0);
                over(
                    line,
                    column,
                    [tint[0], tint[1], tint[2]],
                    tint[3] * gaussian * sealed,
                );
            }
        }
    });
}

#[derive(Debug, Clone, Copy)]
pub struct Pane {
    pub rect: [f32; 4],

    pub radius: f32,

    pub power: f32,

    pub glass: Glass,

    pub tint: [f32; 4],

    pub opacity: f32,
}

impl Pane {
    pub fn new(
        rect: [f32; 4],
        radius: f32,
        surface: crate::material::Surface,
        height: f32,
        tint: [f32; 4],
    ) -> Self {
        let scale = crate::metrics::scale_for(height);
        let mut glass = surface.glass();
        glass.depth *= scale;
        Self {
            rect,
            radius,
            power: 2.0,
            glass,
            tint,
            opacity: 1.0,
        }
    }
}

struct PanePixel {
    coverage: f32,
    inset: f32,
    rise: f32,
    outward: [f32; 2],
    surface: [f32; 3],
    shift_red: f32,
    shift_green: f32,
    shift_blue: f32,
    lod: f32,
    caustic: f32,
    slab: f32,
    curve: f32,
}

fn corner_norm(v: [f32; 2], power: f32) -> f32 {
    if power <= 2.0 {
        return (v[0] * v[0] + v[1] * v[1]).sqrt();
    }
    (v[0].powf(power) + v[1].powf(power)).powf(1.0 / power)
}

fn rounded_box(point: [f32; 2], half: [f32; 2], radius: f32, power: f32) -> f32 {
    let r = radius.min(half[0].min(half[1]));
    let q = [point[0].abs() - half[0] + r, point[1].abs() - half[1] + r];
    corner_norm([q[0].max(0.0), q[1].max(0.0)], power) + q[0].max(q[1]).min(0.0) - r
}

fn edge_normal(point: [f32; 2], half: [f32; 2], radius: f32, power: f32) -> [f32; 2] {
    let gradient = [
        rounded_box([point[0] + 1.0, point[1]], half, radius, power)
            - rounded_box([point[0] - 1.0, point[1]], half, radius, power),
        rounded_box([point[0], point[1] + 1.0], half, radius, power)
            - rounded_box([point[0], point[1] - 1.0], half, radius, power),
    ];
    let len = (gradient[0] * gradient[0] + gradient[1] * gradient[1]).sqrt();
    if len < 0.0001 {
        return [0.0, -1.0];
    }
    [gradient[0] / len, gradient[1] / len]
}

fn bevel_shift(inset: f32, slab: f32, eta: f32) -> f32 {
    let rise = bevel_rise(inset);
    let normal = normalize3([bevel_slope(inset), 0.0, 1.0]);
    let inside = refract([0.0, 0.0, -1.0], normal, eta);

    let mut shift = inside[0] * (slab * rise / (-inside[2]).max(0.05));

    let sin_in = inside[0].abs();
    if sin_in > 1e-5 {
        let sin_out = (sin_in / eta).min(0.90);
        let cos_out = (1.0 - sin_out * sin_out).max(1e-4).sqrt();
        shift += inside[0].signum() * (sin_out / cos_out) * slab * optics::FLOAT;
    }
    shift
}

fn lamp() -> [f32; 3] {
    static LAMP: std::sync::OnceLock<[f32; 3]> = std::sync::OnceLock::new();
    *LAMP.get_or_init(|| normalize3(optics::KEY_LIGHT))
}

fn environment(mirrored: [f32; 3], key_strength: f32) -> [f32; 3] {
    let sky = (-mirrored[1]).clamp(0.0, 1.0);
    let ambient = mix3([0.03, 0.03, 0.05], [1.20, 1.16, 1.45], sky * sky);
    let key = dot3(mirrored, lamp()).max(0.0).powi(36);
    [
        ambient[0] + 1.0 * key * 26.0 * key_strength,
        ambient[1] + 0.98 * key * 26.0 * key_strength,
        ambient[2] + 0.93 * key * 26.0 * key_strength,
    ]
}

#[derive(Clone, Copy)]
struct Bevel {
    shift_red: f32,
    shift_green: f32,
    shift_blue: f32,
    caustic: f32,
}

fn bevel_at(inset: f32, slab: f32) -> Bevel {
    let eta = 1.0 / optics::IOR;
    let spread = optics::DISPERSION * eta;
    let shift_green = bevel_shift(inset, slab, eta);

    let step = 0.02;
    let moves = (bevel_shift(inset + step, slab, eta) - shift_green) / (step * slab.max(1e-4));

    Bevel {
        shift_red: bevel_shift(inset, slab, eta + spread),
        shift_green,
        shift_blue: bevel_shift(inset, slab, eta - spread),
        caustic: (1.0 / (1.0 - moves).abs().max(0.3)).clamp(0.45, 2.6),
    }
}

fn pane_pixel(point: [f32; 2], half: [f32; 2], pane: &Pane, face: Bevel) -> PanePixel {
    let radius = pane.radius;
    let power = pane.power;
    let d = rounded_box(point, half, radius, power);
    let coverage = 1.0 - smoothstep(-0.75, 0.75, d);

    let slab = pane
        .glass
        .depth
        .min(half[0].min(half[1]) * optics::MAX_SLAB_SHARE);
    let inset = if slab > 0.0 {
        (-d / slab).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let rise = bevel_rise(inset);
    let outward = edge_normal(point, half, radius, power);

    let curve = pane.glass.curve;
    let face_position = [point[0] / half[0].max(1.0), point[1] / half[1].max(1.0)];
    let face_slope = [
        face_position[0] * 0.28 * curve * rise,
        face_position[1] * 0.45 * curve * rise,
    ];
    let slope = bevel_slope(inset);
    let surface = normalize3([
        outward[0] * slope + face_slope[0],
        outward[1] * slope + face_slope[1],
        1.0,
    ]);

    let lod = pane.glass.frost * optics::BLUR_LEVELS * (0.3 + 0.7 * inset);

    let bevel = if inset >= 1.0 {
        face
    } else {
        bevel_at(inset, slab)
    };

    PanePixel {
        coverage,
        inset,
        rise,
        outward,
        surface,
        shift_red: bevel.shift_red,
        shift_green: bevel.shift_green,
        shift_blue: bevel.shift_blue,
        lod,
        caustic: bevel.caustic,
        slab,
        curve,
    }
}

fn pane_shade(
    pixel: &PanePixel,
    tint: [f32; 4],
    gloss: f32,
    frost: f32,
    red: [f32; 3],
    green: [f32; 3],
    blue: [f32; 3],
) -> [f32; 4] {
    let mut glass = [tint[0], tint[1], tint[2]];
    let mut alpha = tint[3];

    if pixel.slab > 0.0 {
        let stain = tint[3] * mix(0.4, 1.0, pixel.rise);
        let bent = [
            red[0] * pixel.caustic,
            green[1] * pixel.caustic,
            blue[2] * pixel.caustic,
        ];
        glass = mix3(bent, [tint[0], tint[1], tint[2]], stain);
        accumulate(&mut glass, optics::FROST_SCATTER, frost);

        alpha = 1.0;
    }

    if gloss > 0.0 {
        let fresnel = 0.04 + 0.96 * (1.0 - pixel.surface[2].clamp(0.0, 1.0)).powi(5);
        let lit = fresnel * gloss;

        let broad_face = smoothstep(0.68, 1.0, pixel.inset) * pixel.curve.clamp(0.0, 1.0);
        let key_strength = mix(1.0, 0.10, broad_face);
        glass = mix3(
            glass,
            environment(reflect([0.0, 0.0, -1.0], pixel.surface), key_strength),
            lit,
        );

        alpha = alpha.max(lit);
    }

    [
        glass[0].max(0.0),
        glass[1].max(0.0),
        glass[2].max(0.0),
        alpha,
    ]
}

struct Behind {
    levels: Vec<(Vec<[f32; 3]>, usize, usize)>,
    origin: [i32; 2],
}

impl Behind {
    fn take(canvas: &Canvas, region: [i32; 4], deepest: f32) -> Self {
        let [x0, y0, w, h] = region;
        let (w, h) = (w as usize, h as usize);
        let mut level = vec![[0.0_f32; 3]; w * h];
        for row in 0..h {
            let y = (y0 + row as i32).clamp(0, canvas.height as i32 - 1) as usize;
            let line =
                &canvas.pixels[y * canvas.stride..y * canvas.stride + canvas.width as usize * 4];
            for column in 0..w {
                let x = (x0 + column as i32).clamp(0, canvas.width as i32 - 1) as usize;
                level[row * w + column] = read(line, x);
            }
        }
        let rungs = deepest.clamp(0.0, optics::BLUR_LEVELS).ceil() as usize;
        let mut levels = vec![(level, w, h)];
        for _ in 0..rungs {
            let (above, aw, ah) = levels.last().expect("the chain starts with its top rung");
            let (nw, nh) = ((aw / 2).max(1), (ah / 2).max(1));
            let mut down = vec![[0.0_f32; 3]; nw * nh];
            for row in 0..nh {
                for column in 0..nw {
                    let mut total = [0.0_f32; 3];
                    for dy in 0..2 {
                        for dx in 0..2 {
                            let sy = (row * 2 + dy).min(ah - 1);
                            let sx = (column * 2 + dx).min(aw - 1);
                            let it = above[sy * aw + sx];
                            total[0] += it[0];
                            total[1] += it[1];
                            total[2] += it[2];
                        }
                    }
                    down[row * nw + column] = [total[0] / 4.0, total[1] / 4.0, total[2] / 4.0];
                }
            }
            levels.push((down, nw, nh));
        }
        Self {
            levels,
            origin: [x0, y0],
        }
    }

    fn at(&self, point: [f32; 2], lod: f32) -> [f32; 3] {
        let lod = lod.clamp(0.0, (self.levels.len() - 1) as f32);
        let low = lod.floor() as usize;
        let between = lod - low as f32;
        let near = self.rung(low, point);
        let high = (low + 1).min(self.levels.len() - 1);
        if high == low || between <= 0.0 {
            return near;
        }
        mix3(near, self.rung(high, point), between)
    }

    fn rung(&self, level: usize, point: [f32; 2]) -> [f32; 3] {
        let (pixels, width, height) = &self.levels[level];
        let step = (1 << level) as f32;
        let x = (point[0] - self.origin[0] as f32) / step - 0.5;
        let y = (point[1] - self.origin[1] as f32) / step - 0.5;
        let x0 = x.floor();
        let y0 = y.floor();
        let fx = x - x0;
        let fy = y - y0;
        let at = |cx: f32, cy: f32| {
            let cx = (cx as i32).clamp(0, *width as i32 - 1) as usize;
            let cy = (cy as i32).clamp(0, *height as i32 - 1) as usize;
            pixels[cy * width + cx]
        };
        let top = mix3(at(x0, y0), at(x0 + 1.0, y0), fx);
        let bottom = mix3(at(x0, y0 + 1.0), at(x0 + 1.0, y0 + 1.0), fx);
        mix3(top, bottom, fy)
    }
}

pub fn glass(canvas: &mut Canvas, pane: &Pane) {
    let [x, y, width, height] = pane.rect;
    if width <= 0.0 || height <= 0.0 || pane.opacity <= 0.0 {
        return;
    }
    let half = [width * 0.5, height * 0.5];
    let centre = [x + half[0], y + half[1]];

    let margin = (pane.glass.depth * 3.0).max(1.0) + (1 << optics::BLUR_LEVELS as usize) as f32;
    let region = [
        (x - margin).floor() as i32,
        (y - margin).floor() as i32,
        (width + margin * 2.0).ceil() as i32 + 1,
        (height + margin * 2.0).ceil() as i32 + 1,
    ];

    let behind = Behind::take(canvas, region, pane.glass.frost * optics::BLUR_LEVELS);
    let slab = pane
        .glass
        .depth
        .min(half[0].min(half[1]) * optics::MAX_SLAB_SHARE);
    let face = bevel_at(1.0, slab);

    let first = (y - 1.0).floor().max(0.0) as u32;
    let last = ((y + height + 1.0).ceil() as i64).clamp(0, canvas.height as i64) as u32;
    let left = (x - 1.0).floor().max(0.0) as usize;
    let right = ((x + width + 1.0).ceil() as i64).clamp(0, canvas.width as i64) as usize;
    if first >= last || left >= right {
        return;
    }

    let stride = canvas.stride;
    let start = first as usize * stride;
    let end = (last as usize * stride).min(canvas.pixels.len());
    let rows = &mut canvas.pixels[start..end];
    let tint = pane.tint;
    let gloss = pane.glass.gloss;
    let frost = pane.glass.frost;
    let opacity = pane.opacity;
    in_bands(rows, stride, last - first, |band, band_first| {
        for (row, line) in band.chunks_mut(stride).enumerate() {
            let py = (first + band_first + row as u32) as f32 + 0.5;
            for column in left..right {
                let px = column as f32 + 0.5;
                let point = [px - centre[0], py - centre[1]];
                let pixel = pane_pixel(point, half, pane, face);
                if pixel.coverage <= 0.0 {
                    continue;
                }
                let sample = |shift: f32| {
                    behind.at(
                        [px + pixel.outward[0] * shift, py + pixel.outward[1] * shift],
                        pixel.lod,
                    )
                };
                let green = sample(pixel.shift_green);

                let (red, blue) = if pixel.shift_red == pixel.shift_green
                    && pixel.shift_blue == pixel.shift_green
                {
                    (green, green)
                } else {
                    (sample(pixel.shift_red), sample(pixel.shift_blue))
                };
                let shaded = pane_shade(&pixel, tint, gloss, frost, red, green, blue);
                over(
                    line,
                    column,
                    [shaded[0], shaded[1], shaded[2]],
                    shaded[3] * pixel.coverage * opacity,
                );
            }
        }
    });
}

pub fn scrim(canvas: &mut Canvas, rect: [f32; 4], blur: f32, tint: [f32; 4]) {
    let [x, y, width, height] = rect;
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    let margin = (1 << optics::BLUR_LEVELS as usize) as f32;
    let region = [
        (x - margin).floor() as i32,
        (y - margin).floor() as i32,
        (width + margin * 2.0).ceil() as i32 + 1,
        (height + margin * 2.0).ceil() as i32 + 1,
    ];
    let behind = Behind::take(canvas, region, blur);

    let first = y.floor().max(0.0) as u32;
    let last = ((y + height).ceil() as i64).clamp(0, canvas.height as i64) as u32;
    let left = x.floor().max(0.0) as usize;
    let right = ((x + width).ceil() as i64).clamp(0, canvas.width as i64) as usize;
    if first >= last || left >= right {
        return;
    }

    let stride = canvas.stride;
    let start = first as usize * stride;
    let end = (last as usize * stride).min(canvas.pixels.len());
    let rows = &mut canvas.pixels[start..end];
    in_bands(rows, stride, last - first, |band, band_first| {
        for (row, line) in band.chunks_mut(stride).enumerate() {
            let py = (first + band_first + row as u32) as f32 + 0.5;
            for column in left..right {
                let px = column as f32 + 0.5;
                let pushed = mix3(
                    behind.at([px, py], blur),
                    [tint[0], tint[1], tint[2]],
                    tint[3],
                );
                over(line, column, pushed, 1.0);
            }
        }
    });
}

#[derive(Debug, Clone, Copy)]
pub struct Mark {
    pub rect: [f32; 4],

    pub color: [f32; 3],

    pub accent_soft: [f32; 3],

    pub gloss: f32,
    pub opacity: f32,

    pub simple: bool,
}

impl Mark {
    pub fn new(rect: [f32; 4], color: [f32; 3], accent_soft: [f32; 3]) -> Self {
        Self {
            rect,
            color,
            accent_soft,
            gloss: crate::material::Surface::Control.glass().gloss,
            opacity: 1.0,
            simple: false,
        }
    }
}

pub fn glyph(canvas: &mut Canvas, glyph: &Mark, field: &[u8]) -> bool {
    let cell = mark::CELL as usize;
    if field.len() != cell * cell {
        return false;
    }
    let [x, y, width, height] = glyph.rect;
    if width <= 0.0 || height <= 0.0 || glyph.opacity <= 0.0 {
        return false;
    }

    let size = [width, height];
    let depth = mark::depth_for(size[0]);

    let texel = 1.0 / mark::CELL as f32;
    let arm = texel * mark::GRADIENT_ARM;
    let per_pixel = mark::CELL as f32 * texel / size[0].max(1.0);
    let lamp = normalize2([mark::LAMP[0], mark::LAMP[1]]);
    let shadow_offset = [
        lamp[0] * depth.max(mark::MIN_DEPTH) * mark::SHADOW_OFFSET * per_pixel,
        lamp[1] * depth.max(mark::MIN_DEPTH) * mark::SHADOW_OFFSET * per_pixel,
    ];

    let first = y.floor().max(0.0) as u32;
    let last = ((y + height).ceil() as i64).clamp(0, canvas.height as i64) as u32;
    let left = x.floor().max(0.0) as usize;
    let right = ((x + width).ceil() as i64).clamp(0, canvas.width as i64) as usize;
    if first >= last || left >= right {
        return true;
    }

    let stride = canvas.stride;
    let start = first as usize * stride;
    let end = (last as usize * stride).min(canvas.pixels.len());
    let rows = &mut canvas.pixels[start..end];
    in_bands(rows, stride, last - first, |band, band_first| {
        for (row, line) in band.chunks_mut(stride).enumerate() {
            let py = (first + band_first + row as u32) as f32 + 0.5;
            let unit_y = (py - y) / height;
            for column in left..right {
                let px = column as f32 + 0.5;
                let uv = [(px - x) / width, unit_y];
                let centre = decode(field, uv);
                let shaded = if glyph.simple {
                    glyph_simple(centre, size, glyph)
                } else {
                    glyph_default(
                        centre,
                        decode(field, [uv[0] + arm, uv[1]]),
                        decode(field, [uv[0] - arm, uv[1]]),
                        decode(field, [uv[0], uv[1] + arm]),
                        decode(field, [uv[0], uv[1] - arm]),
                        decode(field, [uv[0] + shadow_offset[0], uv[1] + shadow_offset[1]]),
                        size,
                        unit_y * size[1],
                        depth,
                        glyph,
                    )
                };
                over(line, column, [shaded[0], shaded[1], shaded[2]], shaded[3]);
            }
        }
    });
    true
}

fn decode(field: &[u8], uv: [f32; 2]) -> f32 {
    let cell = mark::CELL as i32;
    let x = uv[0] * cell as f32 - 0.5;
    let y = uv[1] * cell as f32 - 0.5;
    let x0 = x.floor();
    let y0 = y.floor();
    let at = |cx: f32, cy: f32| {
        let cx = (cx as i32).clamp(0, cell - 1);
        let cy = (cy as i32).clamp(0, cell - 1);
        field[(cy * cell + cx) as usize] as f32 / 255.0
    };
    let top = mix(at(x0, y0), at(x0 + 1.0, y0), x - x0);
    let bottom = mix(at(x0, y0 + 1.0), at(x0 + 1.0, y0 + 1.0), x - x0);
    mark::decode_sdf(mix(top, bottom, y - y0))
}

fn coverage_of(distance_px: f32) -> f32 {
    1.0 - smoothstep(-mark::COVERAGE_FEATHER, mark::COVERAGE_FEATHER, distance_px)
}

fn glyph_simple(distance: f32, size: [f32; 2], glyph: &Mark) -> [f32; 4] {
    let coverage = coverage_of(distance * size[0]);
    let stain = mix3([1.0, 1.0, 1.0], glyph.color, mark::SIMPLE_STAIN);
    let flat = mix3(stain, glyph.accent_soft, mark::SIMPLE_TINT);
    [
        flat[0],
        flat[1],
        flat[2],
        coverage * mark::SIMPLE_ALPHA * glyph.opacity,
    ]
}

#[allow(clippy::too_many_arguments)]
fn glyph_default(
    center: f32,
    east: f32,
    west: f32,
    south: f32,
    north: f32,
    shadow_sample: f32,
    size: [f32; 2],
    local_y: f32,
    depth: f32,
    glyph: &Mark,
) -> [f32; 4] {
    let distance_px = center * size[0];
    let coverage = coverage_of(distance_px);

    let gradient = [east - west, south - north];
    let outward = normalize2([gradient[0] + 1e-6, gradient[1]]);
    let slope = ((gradient[0] * gradient[0] + gradient[1] * gradient[1]).sqrt()
        / (2.0 * mark::GRADIENT_ARM / mark::CELL as f32))
        .clamp(0.0, 1.0);
    let ridge = smoothstep(mark::RIDGE_BEGIN, mark::RIDGE_END, slope);

    let slab = depth.max(mark::MIN_DEPTH);
    let inset = (-distance_px / slab).clamp(0.0, 1.0);
    let surface = normalize3([
        outward[0] * bevel_slope(inset) * ridge,
        outward[1] * bevel_slope(inset) * ridge,
        1.0,
    ]);

    let foot = (local_y / size[1].max(1.0)).clamp(0.0, 1.0);
    let mut glass = [
        glyph.color[0] * mix(0.50, 1.0, foot * foot),
        glyph.color[1] * mix(0.50, 1.0, foot * foot),
        glyph.color[2] * mix(0.50, 1.0, foot * foot),
    ];
    let mut alpha = mix(0.52, 0.94, foot * foot);

    let facing = dot3(surface, mark::LAMP).clamp(0.0, 1.0);
    attenuate(&mut glass, mix(0.70, 1.26, facing));

    let edge = (1.0 - inset) * (1.0 - inset) * ridge;
    let split = optics::DISPERSION * edge * outward[0];
    glass[0] *= 1.0 + split;
    glass[2] *= 1.0 - split;

    if glyph.gloss > 0.0 {
        let fresnel = 0.04 + 0.96 * (1.0 - surface[2].clamp(0.0, 1.0)).powi(5);
        let lit = fresnel * glyph.gloss;
        let mirrored = reflect([0.0, 0.0, -1.0], surface);
        let value = mix(
            0.42,
            1.15,
            0.5 + 0.5 * dot3(mirrored, [-mark::LAMP[0], -mark::LAMP[1], -mark::LAMP[2]]),
        );
        glass = mix3(
            glass,
            [
                glyph.color[0] * value,
                glyph.color[1] * value,
                glyph.color[2] * value,
            ],
            lit,
        );
        let half_way = normalize3([mark::LAMP[0], mark::LAMP[1], mark::LAMP[2] + 1.0]);
        let spec = dot3(surface, half_way).clamp(0.0, 1.0).powi(42);
        accumulate(&mut glass, glyph.color, spec * glyph.gloss * 0.9);
        alpha = alpha.max(lit.max(spec));
    }

    let occluder = shadow_sample * size[0];
    let blocked = 1.0 - smoothstep(-slab * 0.1, slab * 0.4, occluder);
    let shade = blocked * mark::SHADOW * (1.0 - coverage);

    let mark_alpha = alpha * coverage;
    let total = mark_alpha + shade * (1.0 - mark_alpha);
    let scale = mark_alpha / total.max(1e-4);
    [
        glass[0].max(0.0) * scale,
        glass[1].max(0.0) * scale,
        glass[2].max(0.0) * scale,
        total * glyph.opacity,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::PURPLE;

    struct Sheet {
        pixels: Vec<u8>,
        width: u32,
        height: u32,
    }

    impl Sheet {
        fn new(width: u32, height: u32) -> Self {
            Self {
                pixels: vec![0; width as usize * height as usize * 4],
                width,
                height,
            }
        }

        fn canvas(&mut self) -> Canvas<'_> {
            let stride = self.width as usize * 4;
            Canvas::new(&mut self.pixels, self.width, self.height, stride)
                .expect("a sheet is a canvas")
        }

        fn fill(&mut self, color: [f32; 3]) {
            let stride = self.width as usize * 4;
            for y in 0..self.height as usize {
                for x in 0..self.width as usize {
                    over(
                        &mut self.pixels[y * stride..(y + 1) * stride],
                        x,
                        color,
                        1.0,
                    );
                }
            }
        }

        fn at(&self, x: u32, y: u32) -> [f32; 3] {
            let stride = self.width as usize * 4;
            let row = y as usize * stride;
            read(&self.pixels[row..row + stride], x as usize)
        }

        fn mean(&self) -> f32 {
            let mut total = 0.0;
            for y in 0..self.height {
                for x in 0..self.width {
                    let it = self.at(x, y);
                    total += it[0] + it[1] + it[2];
                }
            }
            total / (self.width * self.height * 3) as f32
        }
    }

    #[test]
    fn the_transfer_table_is_the_transfer_function() {
        for step in 0..=4000 {
            let value = step as f32 / 4000.0;
            let arithmetic = (color::to_srgb(value) * 255.0 + 0.5) as u32;
            assert_eq!(
                encode(value),
                arithmetic,
                "{value} encodes differently through the table"
            );
        }
        for byte in 0..=255_u32 {
            assert_eq!(from_byte(byte), color::to_linear(byte as f32 / 255.0));
        }
    }

    #[test]
    fn a_canvas_it_cannot_hold_is_refused() {
        let mut pixels = vec![0_u8; 16 * 16 * 4];
        assert!(Canvas::new(&mut pixels, 16, 16, 16 * 4).is_some());
        assert!(Canvas::new(&mut pixels, 16, 16, 16 * 4 - 1).is_none());
        assert!(Canvas::new(&mut pixels, 16, 17, 16 * 4).is_none());
        assert!(Canvas::new(&mut pixels, 0, 16, 16 * 4).is_none());

        let mut tight = vec![0_u8; 15 * 68 + 16 * 4];
        assert!(Canvas::new(&mut tight, 16, 16, 68).is_some());
    }

    #[test]
    fn the_wallpaper_is_a_moving_picture() {
        let mut sheet = Sheet::new(64, 40);
        wallpaper(&mut sheet.canvas(), &Scene::new(&PURPLE, 10.0));
        let first = sheet.pixels.clone();
        let corner = sheet.at(1, 1);
        let middle = sheet.at(32, 24);
        assert!(
            (corner[0] - middle[0]).abs() + (corner[2] - middle[2]).abs() > 0.001,
            "a wallpaper with the same colour everywhere is a fill"
        );

        wallpaper(&mut sheet.canvas(), &Scene::new(&PURPLE, 10.0));
        assert_eq!(sheet.pixels, first, "the same clock is the same frame");

        wallpaper(&mut sheet.canvas(), &Scene::new(&PURPLE, 14.0));
        assert_ne!(sheet.pixels, first, "four seconds later is a later frame");

        assert!(sheet.pixels.chunks(4).all(|pixel| pixel[3] == 0xff));
    }

    #[test]
    fn softening_the_scene_quietens_it() {
        let sharp = {
            let mut sheet = Sheet::new(64, 40);
            wallpaper(&mut sheet.canvas(), &Scene::new(&PURPLE, 10.0));
            sheet.mean()
        };
        let quiet = |soften: f32| {
            let mut sheet = Sheet::new(64, 40);
            let mut scene = Scene::new(&PURPLE, 10.0);
            scene.soften = soften;
            wallpaper(&mut sheet.canvas(), &scene);
            sheet.mean()
        };
        assert!(quiet(1.0) < sharp, "a fully softened scene is not dimmer");
        assert!(quiet(crate::wallpaper::SOFTEN) < sharp);
        assert!(quiet(crate::wallpaper::SOFTEN) > quiet(1.0));
    }

    #[test]
    fn a_custom_wallpaper_draws_the_default_water() {
        let drawn = |style| {
            let mut sheet = Sheet::new(48, 30);
            let mut scene = Scene::new(&PURPLE, 3.0);
            scene.style = style;
            wallpaper(&mut sheet.canvas(), &scene);
            sheet.pixels
        };
        assert_ne!(
            drawn(WallpaperStyle::Default),
            drawn(WallpaperStyle::Simple)
        );
        assert_eq!(
            drawn(WallpaperStyle::Custom),
            drawn(WallpaperStyle::Default)
        );
    }

    #[test]
    fn a_pane_brightens_what_is_behind_it() {
        let mut sheet = Sheet::new(120, 120);
        sheet.fill([0.05, 0.05, 0.08]);
        let behind = sheet.at(60, 60);
        glass(
            &mut sheet.canvas(),
            &Pane {
                rect: [20.0, 20.0, 80.0, 80.0],
                radius: 16.0,
                power: 2.0,
                glass: Glass {
                    depth: 9.0,
                    frost: 0.2,
                    gloss: 1.0,
                    curve: 0.0,
                },
                tint: [0.1, 0.08, 0.2, 0.1],
                opacity: 1.0,
            },
        );
        let face = sheet.at(60, 60);
        assert!(
            face[0] > behind[0] && face[2] > behind[2],
            "a clear pane over {behind:?} came out {face:?}"
        );

        assert_eq!(sheet.at(4, 4), behind);
    }

    #[test]
    fn a_rim_splits_what_it_bends() {
        let mut sheet = Sheet::new(120, 120);

        for y in 0..120 {
            let stride = 120 * 4;
            for x in 0..120 {
                let value = if x < 60 { 0.02 } else { 0.45 };
                over(
                    &mut sheet.pixels[y * stride..(y + 1) * stride],
                    x,
                    [value; 3],
                    1.0,
                );
            }
        }
        glass(
            &mut sheet.canvas(),
            &Pane {
                rect: [10.0, 10.0, 100.0, 100.0],
                radius: 20.0,
                power: 2.0,
                glass: Glass {
                    depth: 14.0,
                    frost: 0.0,
                    gloss: 0.66,
                    curve: 0.0,
                },
                tint: [0.0, 0.0, 0.0, 0.0],
                opacity: 1.0,
            },
        );
        let split = (10..110)
            .map(|y| {
                let it = sheet.at(12, y);
                (it[0] - it[2]).abs()
            })
            .fold(0.0_f32, f32::max);
        assert!(split > 0.002, "the rim split nothing at all ({split})");
    }

    #[test]
    fn the_light_is_sealed_at_its_own_edge() {
        let mut sheet = Sheet::new(80, 80);
        light(
            &mut sheet.canvas(),
            [10.0, 10.0, 60.0, 60.0],
            [1.0, 1.0, 1.0, 0.9],
        );
        let middle = sheet.at(40, 40)[0];
        let halfway = sheet.at(55, 40)[0];
        assert!(middle > halfway, "it is not brightest in the middle");
        assert!(halfway > 0.0, "it does not reach halfway out");
        assert_eq!(sheet.at(11, 11), [0.0; 3], "it drew into its own corner");
        assert_eq!(sheet.at(75, 40), [0.0; 3], "it drew outside its rectangle");
    }

    #[test]
    fn a_scrim_blurs_as_well_as_dims() {
        let mut sheet = Sheet::new(64, 64);
        let stride = 64 * 4;
        for y in 0..64 {
            for x in 0..64 {
                let value = if (x / 4 + y / 4) % 2 == 0 { 0.8 } else { 0.02 };
                over(
                    &mut sheet.pixels[y * stride..(y + 1) * stride],
                    x,
                    [value; 3],
                    1.0,
                );
            }
        }
        let before = (sheet.at(15, 30)[0] - sheet.at(16, 30)[0]).abs();
        scrim(
            &mut sheet.canvas(),
            [0.0, 0.0, 64.0, 64.0],
            2.0,
            [0.0, 0.0, 0.0, 0.5],
        );
        let after = (sheet.at(15, 30)[0] - sheet.at(16, 30)[0]).abs();
        assert!(before > 0.5, "the check was not a hard edge to begin with");
        assert!(
            after < before * 0.25,
            "the scrim only dimmed: {before} became {after}"
        );
        assert!(sheet.mean() < 0.41, "the scrim did not dim at all");
    }

    #[test]
    fn a_mark_needs_one_whole_cell() {
        let cell = mark::CELL as usize;
        let mut sheet = Sheet::new(64, 64);
        let glyph_mark = Mark::new([8.0, 8.0, 48.0, 48.0], [1.0, 1.0, 1.0], [0.5, 0.5, 0.9]);
        assert!(!glyph(&mut sheet.canvas(), &glyph_mark, &[128; 16]));
        assert!(!glyph(
            &mut sheet.canvas(),
            &glyph_mark,
            &vec![128; cell * cell + 1]
        ));
        assert_eq!(sheet.at(32, 32), [0.0; 3], "a refused mark drew anyway");
        assert!(glyph(
            &mut sheet.canvas(),
            &glyph_mark,
            &vec![0; cell * cell]
        ));
    }

    #[test]
    fn a_mark_is_shaded_from_its_own_field() {
        let cell = mark::CELL;
        let fine = (cell * mark::SDF_SUPERSAMPLE) as usize;
        let mut coverage = vec![0_u8; fine * fine];
        let centre = fine as f32 / 2.0;
        let radius = fine as f32 * 0.35;
        for y in 0..fine {
            for x in 0..fine {
                let dx = x as f32 + 0.5 - centre;
                let dy = y as f32 + 0.5 - centre;
                if (dx * dx + dy * dy).sqrt() <= radius {
                    coverage[y * fine + x] = 255;
                }
            }
        }
        let field =
            mark::distance_field_alpha_from_coverage(&coverage, cell).expect("a disc is a shape");

        let mut sheet = Sheet::new(128, 128);
        assert!(glyph(
            &mut sheet.canvas(),
            &Mark::new([14.0, 14.0, 100.0, 100.0], [1.0, 1.0, 1.0], [0.5, 0.5, 0.9]),
            &field,
        ));

        let face = sheet.at(64, 64)[0];
        let wall = sheet.at(32, 64)[0];
        assert!(face > 0.1, "the middle of the mark is not drawn");
        assert!(
            wall > face,
            "the wall the lamp is on is not the bright part: {wall} against {face}"
        );
        assert_eq!(sheet.at(4, 4), [0.0; 3], "the mark drew outside its box");
    }
}
