const LXB_KEY_LIGHT: vec3<f32> = vec3<f32>(-0.42, -0.66, 0.62);

const LXB_GLASS_IOR: f32 = 1.47;

const LXB_GLASS_DISPERSION: f32 = 0.055;

const LXB_GLASS_FLOAT: f32 = 1.0;

const LXB_FROST_SCATTER: vec3<f32> = vec3<f32>(0.030, 0.029, 0.046);

const LXB_BLUR_LEVELS: f32 = 4.0;

const LXB_MAX_SLAB_SHARE: f32 = 0.44;

fn lxb_corner_norm(v: vec2<f32>, power: f32) -> f32 {
    if (power <= 2.0) {
        return length(v);
    }
    return pow(pow(v.x, power) + pow(v.y, power), 1.0 / power);
}

fn lxb_rounded_box(point: vec2<f32>, half: vec2<f32>, radius: f32, power: f32) -> f32 {
    let r = min(radius, min(half.x, half.y));
    let q = abs(point) - half + vec2<f32>(r);
    return lxb_corner_norm(max(q, vec2<f32>(0.0)), power) + min(max(q.x, q.y), 0.0) - r;
}

fn lxb_coverage(point: vec2<f32>, half: vec2<f32>, radius: f32, power: f32) -> f32 {
    return 1.0 - smoothstep(-0.75, 0.75, lxb_rounded_box(point, half, radius, power));
}

fn lxb_edge_normal(point: vec2<f32>, half: vec2<f32>, radius: f32, power: f32) -> vec2<f32> {
    let e = vec2<f32>(1.0, 0.0);
    let gradient = vec2<f32>(
        lxb_rounded_box(point + e.xy, half, radius, power)
            - lxb_rounded_box(point - e.xy, half, radius, power),
        lxb_rounded_box(point + e.yx, half, radius, power)
            - lxb_rounded_box(point - e.yx, half, radius, power),
    );
    let len = length(gradient);
    if (len < 0.0001) {
        return vec2<f32>(0.0, -1.0);
    }
    return gradient / len;
}

fn lxb_bevel_rise(inset: f32) -> f32 {
    return sqrt(max(1.0 - (1.0 - inset) * (1.0 - inset), 0.0));
}

fn lxb_bevel_slope(inset: f32) -> f32 {
    return (1.0 - inset) / max(lxb_bevel_rise(inset), 0.16);
}

fn lxb_bevel_shift(inset: f32, slab: f32, eta: f32) -> f32 {
    let rise = lxb_bevel_rise(inset);
    let normal = normalize(vec3<f32>(lxb_bevel_slope(inset), 0.0, 1.0));
    let inside = refract(vec3<f32>(0.0, 0.0, -1.0), normal, eta);

    var shift = inside.x * (slab * rise / max(-inside.z, 0.05));

    let sin_in = abs(inside.x);
    if (sin_in > 1e-5) {

        let sin_out = min(sin_in / eta, 0.90);
        let cos_out = sqrt(max(1.0 - sin_out * sin_out, 1e-4));
        shift += sign(inside.x) * (sin_out / cos_out) * slab * LXB_GLASS_FLOAT;
    }
    return shift;
}

fn lxb_environment(mirrored: vec3<f32>, key_strength: f32) -> vec3<f32> {

    let sky = clamp(-mirrored.y, 0.0, 1.0);
    let ambient = mix(
        vec3<f32>(0.03, 0.03, 0.05),
        vec3<f32>(1.20, 1.16, 1.45),
        sky * sky,
    );
    let key = pow(max(dot(mirrored, normalize(LXB_KEY_LIGHT)), 0.0), 36.0);
    return ambient + vec3<f32>(1.0, 0.98, 0.93) * key * 26.0 * key_strength;
}

struct LxbPane {

    coverage: f32,

    inset: f32,

    rise: f32,

    outward: vec2<f32>,

    surface: vec3<f32>,

    shift_red: f32,
    shift_green: f32,
    shift_blue: f32,

    lod: f32,

    caustic: f32,

    slab: f32,

    curve: f32,
}

fn lxb_glass_begin_curved(
    point: vec2<f32>,
    half: vec2<f32>,
    radius: f32,
    power: f32,
    depth: f32,
    frost: f32,
    curve: f32,
) -> LxbPane {
    var pane: LxbPane;

    let d = lxb_rounded_box(point, half, radius, power);
    pane.coverage = 1.0 - smoothstep(-0.75, 0.75, d);

    let slab = min(depth, min(half.x, half.y) * LXB_MAX_SLAB_SHARE);
    pane.slab = slab;
    pane.inset = select(1.0, clamp(-d / slab, 0.0, 1.0), slab > 0.0);
    pane.rise = lxb_bevel_rise(pane.inset);
    pane.outward = lxb_edge_normal(point, half, radius, power);

    pane.curve = curve;
    let face_position = point / max(half, vec2<f32>(1.0));
    let face_slope = face_position * vec2<f32>(0.28, 0.45) * curve * pane.rise;
    pane.surface = normalize(vec3<f32>(
        pane.outward * lxb_bevel_slope(pane.inset) + face_slope,
        1.0));

    pane.lod = frost * LXB_BLUR_LEVELS * (0.3 + 0.7 * pane.inset);

    let eta = 1.0 / LXB_GLASS_IOR;
    let spread = LXB_GLASS_DISPERSION * eta;
    pane.shift_green = lxb_bevel_shift(pane.inset, slab, eta);
    pane.shift_red = lxb_bevel_shift(pane.inset, slab, eta + spread);
    pane.shift_blue = lxb_bevel_shift(pane.inset, slab, eta - spread);

    let step = 0.02;
    let moves = (lxb_bevel_shift(pane.inset + step, slab, eta) - pane.shift_green)
        / (step * max(slab, 1e-4));
    pane.caustic = clamp(1.0 / max(abs(1.0 - moves), 0.3), 0.45, 2.6);

    return pane;
}

fn lxb_glass_begin(
    point: vec2<f32>,
    half: vec2<f32>,
    radius: f32,
    power: f32,
    depth: f32,
    frost: f32,
) -> LxbPane {
    return lxb_glass_begin_curved(point, half, radius, power, depth, frost, 0.0);
}

fn lxb_glass_shade(
    pane: LxbPane,
    tint: vec4<f32>,
    gloss: f32,
    frost: f32,
    red: vec3<f32>,
    green: vec3<f32>,
    blue: vec3<f32>,
) -> vec4<f32> {
    var glass = tint.rgb;
    var alpha = tint.a;

    if (pane.slab > 0.0) {

        let stain = tint.a * mix(0.4, 1.0, pane.rise);
        let bent = vec3<f32>(red.r, green.g, blue.b) * pane.caustic;
        glass = mix(bent, tint.rgb, stain);
        glass += LXB_FROST_SCATTER * frost;

        alpha = 1.0;
    }

    if (gloss > 0.0) {

        let fresnel = 0.04 + 0.96 * pow(1.0 - clamp(pane.surface.z, 0.0, 1.0), 5.0);
        let lit = fresnel * gloss;

        let broad_face = smoothstep(0.68, 1.0, pane.inset)
            * clamp(pane.curve, 0.0, 1.0);
        let key_strength = mix(1.0, 0.10, broad_face);
        glass = mix(
            glass,
            lxb_environment(reflect(vec3<f32>(0.0, 0.0, -1.0), pane.surface), key_strength),
            lit);

        alpha = max(alpha, lit);
    }

    return vec4<f32>(max(glass, vec3<f32>(0.0)), alpha);
}
