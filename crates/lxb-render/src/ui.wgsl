const KIND_WALLPAPER: i32 = 0;
const KIND_BLIT: i32 = 1;
const KIND_SOLID: i32 = 2;
const KIND_LIGHT: i32 = 3;
const KIND_GLASS: i32 = 4;
const KIND_GLYPH: i32 = 5;

const CORNER: f32 = 2.0;

struct Frame {

    resolution: vec4<f32>,

    atlas_size: vec4<f32>,
    sky: array<vec4<f32>, 4>,
    accent: array<vec4<f32>, 3>,
    glow: vec4<f32>,
}

@group(0) @binding(0) var<uniform> frame: Frame;

@group(0) @binding(1) var source: texture_2d<f32>;
@group(0) @binding(2) var source_sampler: sampler;
@group(0) @binding(3) var atlas: texture_2d<f32>;
@group(0) @binding(4) var atlas_sampler: sampler;

struct Instance {
    @location(0) rect: vec4<f32>,

    @location(1) shape: vec4<f32>,
    @location(2) tint: vec4<f32>,

    @location(3) material: vec4<f32>,

    @location(4) cell: vec4<f32>,
}

struct VertexOut {
    @builtin(position) position: vec4<f32>,

    @location(0) local: vec2<f32>,
    @location(1) half_size: vec2<f32>,

    @location(2) unit: vec2<f32>,
    @location(3) tint: vec4<f32>,
    @location(4) material: vec4<f32>,
    @location(5) cell: vec4<f32>,

    @location(6) shape: vec4<f32>,
    @location(7) pixel: vec2<f32>,
}

@vertex
fn vs(@builtin(vertex_index) index: u32, quad: Instance) -> VertexOut {
    let corners = array(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0),
    );
    let unit = corners[index];
    let pixel = quad.rect.xy + unit * quad.rect.zw;
    let ndc = pixel / max(frame.resolution.xy, vec2<f32>(1.0)) * 2.0 - 1.0;

    var out: VertexOut;
    out.position = vec4<f32>(ndc.x, -ndc.y, 0.0, 1.0);
    out.half_size = quad.rect.zw * 0.5;
    out.local = (unit - 0.5) * quad.rect.zw;
    out.unit = unit;
    out.tint = quad.tint;
    out.material = quad.material;
    out.cell = quad.cell;
    out.shape = quad.shape;
    out.pixel = pixel;
    return out;
}

fn wallpaper_at(pixel: vec2<f32>, soften: f32) -> vec3<f32> {
    let resolution = max(frame.resolution.xy, vec2<f32>(1.0));
    let uv = clamp(pixel / resolution, vec2<f32>(0.0), vec2<f32>(1.0));
    return lxb_wallpaper(
        uv,
        resolution.x / resolution.y,
        frame.resolution.z,
        clamp(soften, 0.0, 1.0),
        frame.resolution.w,
        frame.sky,
        frame.accent,
        frame.glow,
        1.0 / resolution,
    );
}

fn behind(pixel: vec2<f32>, lod: f32) -> vec3<f32> {
    let resolution = max(frame.resolution.xy, vec2<f32>(1.0));
    let uv = clamp(pixel / resolution, vec2<f32>(0.0), vec2<f32>(1.0));
    return textureSampleLevel(source, source_sampler, uv, lod).rgb;
}

fn field(uv: vec2<f32>) -> f32 {
    return lxb_glyph_decode(textureSampleLevel(atlas, atlas_sampler, uv, 0.0).r);
}

@fragment
fn fs(in: VertexOut) -> @location(0) vec4<f32> {
    let kind = i32(in.shape.y + 0.5);

    if (kind == KIND_WALLPAPER) {

        return vec4<f32>(wallpaper_at(in.pixel, in.material.x), 1.0);
    }

    if (kind == KIND_BLIT) {
        return vec4<f32>(behind(in.pixel, 0.0), 1.0);
    }

    if (kind == KIND_SOLID) {
        let d = lxb_rounded_box(in.local, in.half_size, in.shape.x, CORNER);
        var coverage = 1.0 - smoothstep(-0.75, 0.75, d);

        let border = in.material.x;
        if (border > 0.0) {
            coverage = coverage * smoothstep(-0.75, 0.75, d + border);
        }
        return vec4<f32>(in.tint.rgb, in.tint.a * coverage * in.shape.w);
    }

    if (kind == KIND_LIGHT) {

        let q = in.local / max(in.half_size, vec2<f32>(1.0));
        let radius = length(q);
        let gaussian = exp(-5.5 * radius * radius);
        let sealed = clamp((1.0 - radius) / 0.12, 0.0, 1.0);
        return vec4<f32>(in.tint.rgb, in.tint.a * gaussian * sealed * in.shape.w);
    }

    if (kind == KIND_GLASS) {
        let pane = lxb_glass_begin_curved(
            in.local,
            in.half_size,
            in.shape.x,
            CORNER,
            in.material.x,
            in.material.y,
            in.material.w,
        );
        if (pane.coverage <= 0.0) {
            discard;
        }

        let lod = clamp(pane.lod, 0.0, frame.atlas_size.z);
        let red = behind(in.pixel + pane.outward * pane.shift_red, lod);
        let green = behind(in.pixel + pane.outward * pane.shift_green, lod);
        let blue = behind(in.pixel + pane.outward * pane.shift_blue, lod);
        let shade = lxb_glass_shade(
            pane, in.tint, in.material.z, in.material.y, red, green, blue);

        return vec4<f32>(shade.rgb, shade.a * pane.coverage * in.shape.w);
    }

    let uv = mix(in.cell.xy, in.cell.zw, in.unit);
    let texel = frame.atlas_size.xy;
    let size = in.half_size * 2.0;

    var material: LxbGlyphMaterial;
    material.size = size;
    material.local_y = in.unit.y * size.y;
    material.color = in.tint.rgb;
    material.accent_soft = frame.accent[1].rgb;
    material.depth = lxb_glyph_depth(size.x);
    material.gloss = in.material.z;
    material.opacity = in.tint.a * in.shape.w;
    material.simple = in.shape.z;

    var samples: LxbGlyphSamples;
    samples.center = field(uv);
    if (material.simple > 0.5) {

        samples.east = samples.center;
        samples.west = samples.center;
        samples.south = samples.center;
        samples.north = samples.center;
        samples.shadow = samples.center;
    } else {
        let arm = lxb_glyph_gradient_offset(texel);
        let per_pixel = lxb_glyph_per_pixel(size, texel);
        samples.east = field(uv + vec2<f32>(arm.x, 0.0));
        samples.west = field(uv - vec2<f32>(arm.x, 0.0));
        samples.south = field(uv + vec2<f32>(0.0, arm.y));
        samples.north = field(uv - vec2<f32>(0.0, arm.y));
        samples.shadow = field(
            uv + lxb_glyph_shadow_offset(material.depth, per_pixel));
    }

    return lxb_glyph_material(samples, material);
}
