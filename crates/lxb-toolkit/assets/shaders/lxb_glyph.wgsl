const LXB_GLYPH_SDF_RANGE: f32 = 0.125;
const LXB_GLYPH_CELL: f32 = 128.0;
const LXB_GLYPH_DEPTH_SHARE: f32 = 0.075;
const LXB_GLYPH_LAMP: vec3<f32> = vec3<f32>(-0.4915, -0.7078, 0.5069);
const LXB_GLYPH_SHADOW: f32 = 0.30;
const LXB_GLYPH_SIMPLE_TINT: f32 = 0.22;
const LXB_GLYPH_SIMPLE_ALPHA: f32 = 0.88;
const LXB_GLYPH_SIMPLE_STAIN: f32 = 0.15;
const LXB_GLYPH_GRADIENT_ARM: f32 = 1.5;
const LXB_GLYPH_COVERAGE_FEATHER: f32 = 0.75;
const LXB_GLYPH_MIN_DEPTH: f32 = 0.5;
const LXB_GLYPH_RIDGE_BEGIN: f32 = 0.20;
const LXB_GLYPH_RIDGE_END: f32 = 0.80;
const LXB_GLYPH_SHADOW_OFFSET: f32 = 0.22;
const LXB_GLYPH_DISPERSION: f32 = 0.055;

struct LxbGlyphSamples {
    center: f32,
    east: f32,
    west: f32,
    south: f32,
    north: f32,
    shadow: f32,
}

struct LxbGlyphMaterial {
    size: vec2<f32>,
    local_y: f32,
    color: vec3<f32>,
    accent_soft: vec3<f32>,
    depth: f32,
    gloss: f32,
    opacity: f32,
    simple: f32,
}

fn lxb_glyph_decode(stored: f32) -> f32 {
    return (stored - 0.5) * 2.0 * LXB_GLYPH_SDF_RANGE;
}

fn lxb_glyph_depth(size: f32) -> f32 {
    return size * LXB_GLYPH_DEPTH_SHARE;
}

fn lxb_glyph_per_pixel(size: vec2<f32>, atlas_texel: vec2<f32>) -> vec2<f32> {
    return LXB_GLYPH_CELL * atlas_texel / max(size, vec2<f32>(1.0));
}

fn lxb_glyph_gradient_offset(atlas_texel: vec2<f32>) -> vec2<f32> {
    return atlas_texel * LXB_GLYPH_GRADIENT_ARM;
}

fn lxb_glyph_shadow_offset(depth: f32, per_pixel: vec2<f32>) -> vec2<f32> {
    return normalize(LXB_GLYPH_LAMP.xy)
        * max(depth, LXB_GLYPH_MIN_DEPTH)
        * LXB_GLYPH_SHADOW_OFFSET
        * per_pixel;
}

fn lxb_glyph_coverage(distance_px: f32) -> f32 {
    return 1.0 - smoothstep(
        -LXB_GLYPH_COVERAGE_FEATHER,
        LXB_GLYPH_COVERAGE_FEATHER,
        distance_px,
    );
}

fn lxb_glyph_bevel_rise(inset: f32) -> f32 {
    return sqrt(max(1.0 - (1.0 - inset) * (1.0 - inset), 0.0));
}

fn lxb_glyph_bevel_slope(inset: f32) -> f32 {
    return (1.0 - inset) / max(lxb_glyph_bevel_rise(inset), 0.16);
}

fn lxb_glyph_simple(
    distance: f32,
    size: vec2<f32>,
    color: vec3<f32>,
    accent_soft: vec3<f32>,
    opacity: f32,
) -> vec4<f32> {
    let distance_px = distance * size.x;
    let coverage = lxb_glyph_coverage(distance_px);
    let stain = mix(vec3<f32>(1.0), color, LXB_GLYPH_SIMPLE_STAIN);
    let flat = mix(stain, accent_soft, LXB_GLYPH_SIMPLE_TINT);
    return vec4<f32>(flat, coverage * LXB_GLYPH_SIMPLE_ALPHA * opacity);
}

fn lxb_glyph_default(
    samples: LxbGlyphSamples,
    material: LxbGlyphMaterial,
) -> vec4<f32> {
    let distance_px = samples.center * material.size.x;
    let coverage = lxb_glyph_coverage(distance_px);

    let gradient = vec2<f32>(
        samples.east - samples.west,
        samples.south - samples.north,
    );
    let outward = normalize(gradient + vec2<f32>(1e-6, 0.0));
    let slope = clamp(
        length(gradient)
            / (2.0 * LXB_GLYPH_GRADIENT_ARM / LXB_GLYPH_CELL),
        0.0,
        1.0,
    );
    let ridge = smoothstep(LXB_GLYPH_RIDGE_BEGIN, LXB_GLYPH_RIDGE_END, slope);

    let slab = max(material.depth, LXB_GLYPH_MIN_DEPTH);
    let inset = clamp(-distance_px / slab, 0.0, 1.0);
    let surface = normalize(vec3<f32>(
        outward * lxb_glyph_bevel_slope(inset) * ridge,
        1.0,
    ));

    let foot = clamp(material.local_y / max(material.size.y, 1.0), 0.0, 1.0);
    var glass = material.color * mix(0.50, 1.0, foot * foot);
    var alpha = mix(0.52, 0.94, foot * foot);

    let facing = clamp(dot(surface, LXB_GLYPH_LAMP), 0.0, 1.0);
    glass *= mix(0.70, 1.26, facing);

    let edge = (1.0 - inset) * (1.0 - inset) * ridge;
    let split = LXB_GLYPH_DISPERSION * edge * outward.x;
    glass *= vec3<f32>(1.0 + split, 1.0, 1.0 - split);

    if (material.gloss > 0.0) {
        let fresnel = 0.04
            + 0.96 * pow(1.0 - clamp(surface.z, 0.0, 1.0), 5.0);
        let lit = fresnel * material.gloss;
        let mirrored = reflect(vec3<f32>(0.0, 0.0, -1.0), surface);
        let value = mix(
            0.42,
            1.15,
            0.5 + 0.5 * dot(mirrored, -LXB_GLYPH_LAMP),
        );
        glass = mix(glass, material.color * value, lit);
        let half_way = normalize(LXB_GLYPH_LAMP + vec3<f32>(0.0, 0.0, 1.0));
        let spec = pow(clamp(dot(surface, half_way), 0.0, 1.0), 42.0);
        glass += material.color * spec * material.gloss * 0.9;
        alpha = max(alpha, max(lit, spec));
    }

    let occluder = samples.shadow * material.size.x;
    let blocked = 1.0 - smoothstep(-slab * 0.1, slab * 0.4, occluder);
    let shade = blocked * LXB_GLYPH_SHADOW * (1.0 - coverage);

    let mark = alpha * coverage;
    let total = mark + shade * (1.0 - mark);
    let rgb = max(glass, vec3<f32>(0.0)) * mark / max(total, 1e-4);
    return vec4<f32>(rgb, total * material.opacity);
}

fn lxb_glyph_material(
    samples: LxbGlyphSamples,
    material: LxbGlyphMaterial,
) -> vec4<f32> {
    if (material.simple > 0.5) {
        return lxb_glyph_simple(
            samples.center,
            material.size,
            material.color,
            material.accent_soft,
            material.opacity,
        );
    }
    return lxb_glyph_default(samples, material);
}
