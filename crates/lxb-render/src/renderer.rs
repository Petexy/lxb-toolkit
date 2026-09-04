use bytemuck::{Pod, Zeroable};
use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache,
    TextArea, TextAtlas, TextBounds, TextRenderer, Viewport, Weight,
};
use lxb_toolkit::{assets, glyph_material};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{mpsc, Arc, Mutex},
    time::UNIX_EPOCH,
};

const BLUR_LEVELS: u32 = lxb_toolkit::material::optics::BLUR_LEVELS as u32;
const MIP_LEVELS: u32 = BLUR_LEVELS + 1;
const THUMBNAIL_SIZE: u32 = 512;
const THUMBNAIL_COLUMNS: u32 = 8;
const THUMBNAIL_ROWS: u32 = 4;
const THUMBNAIL_COUNT: usize = (THUMBNAIL_COLUMNS * THUMBNAIL_ROWS) as usize;
pub(crate) const NO_CUT: [f32; 4] = [-1_000_000.0, -1_000_000.0, 1_000_000.0, 1_000_000.0];

pub(crate) const TARGET: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

pub(crate) const KIND_WALLPAPER: f32 = 0.0;
pub(crate) const KIND_BLIT: f32 = 1.0;
pub(crate) const KIND_SOLID: f32 = 2.0;
pub(crate) const KIND_LIGHT: f32 = 3.0;
pub(crate) const KIND_GLASS: f32 = 4.0;
pub(crate) const KIND_GLYPH: f32 = 5.0;
pub(crate) const KIND_IMAGE: f32 = 6.0;
pub(crate) const KIND_SOFT_EDGE: f32 = 7.0;
pub(crate) const KIND_FROST: f32 = 8.0;

/// How far into the blur pyramid the deepest part of a soft edge reaches.
const SOFT_EDGE_LOD: f32 = 3.6;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub(crate) struct Quad {
    pub rect: [f32; 4],

    pub shape: [f32; 4],

    pub tint: [f32; 4],

    pub material: [f32; 4],

    pub cell: [f32; 4],

    pub cut: [f32; 4],
}

impl Quad {
    pub fn solid(rect: [f32; 4], radius: f32, tint: [f32; 4]) -> Self {
        Self {
            rect,
            shape: [radius, KIND_SOLID, 0.0, 1.0],
            tint,
            material: [0.0; 4],
            cell: [0.0; 4],
            cut: NO_CUT,
        }
    }

    pub fn outline(rect: [f32; 4], radius: f32, tint: [f32; 4], width: f32) -> Self {
        Self {
            rect,
            shape: [radius, KIND_SOLID, 0.0, 1.0],
            tint,
            material: [width.max(1.0), 0.0, 0.0, 0.0],
            cell: [0.0; 4],
            cut: NO_CUT,
        }
    }

    pub fn light(rect: [f32; 4], tint: [f32; 4]) -> Self {
        Self {
            rect,
            shape: [0.0, KIND_LIGHT, 0.0, 1.0],
            tint,
            material: [0.0; 4],
            cell: [0.0; 4],
            cut: NO_CUT,
        }
    }

    pub fn glass(
        rect: [f32; 4],
        radius: f32,
        tint: [f32; 4],
        glass: lxb_toolkit::material::Glass,
        scale: f32,
    ) -> Self {
        Self {
            rect,
            shape: [radius, KIND_GLASS, 0.0, 1.0],
            tint,

            material: [glass.depth * scale, glass.frost, glass.gloss, glass.curve],
            cell: [0.0; 4],
            cut: NO_CUT,
        }
    }

    pub fn image(rect: [f32; 4], radius: f32, cell: [f32; 4], opacity: f32) -> Self {
        Self {
            rect,
            shape: [radius, KIND_IMAGE, 0.0, opacity.clamp(0.0, 1.0)],
            tint: [1.0; 4],
            material: [0.0; 4],
            cell,
            cut: NO_CUT,
        }
    }

    /// The page taken out of the blur pyramid and laid back over itself.
    ///
    /// `lod` is how deep into the pyramid it reaches, and the tint's own alpha
    /// is how much of the tint is mixed into what comes back — nought leaves
    /// the picture its own colour.
    pub fn frost(rect: [f32; 4], radius: f32, lod: f32, tint: [f32; 4]) -> Self {
        Self {
            rect,
            shape: [radius, KIND_FROST, 0.0, 1.0],
            tint,
            material: [lod.max(0.0), 0.0, 0.0, 0.0],
            cell: [0.0; 4],
            cut: NO_CUT,
        }
    }

    pub fn soft_vertical_edges(rect: [f32; 4], band: f32, top: f32, bottom: f32) -> Self {
        Self {
            rect,
            shape: [0.0, KIND_SOFT_EDGE, 0.0, 1.0],
            tint: [1.0; 4],
            // The feather in points, and how deep into the blur pyramid its
            // far end reaches.
            material: [band, SOFT_EDGE_LOD, 0.0, 0.0],
            cell: [top.clamp(0.0, 1.0), bottom.clamp(0.0, 1.0), 0.0, 0.0],
            cut: NO_CUT,
        }
    }

    pub fn clipped(mut self, [x, y, width, height]: [f32; 4]) -> Self {
        let requested = [x, y, x + width, y + height];
        self.cut = [
            self.cut[0].max(requested[0]),
            self.cut[1].max(requested[1]),
            self.cut[2].min(requested[2]),
            self.cut[3].min(requested[3]),
        ];
        self
    }

    pub fn faded(mut self, fade: f32) -> Self {
        self.shape[3] = fade.clamp(0.0, 1.0);
        self
    }

    pub fn full(width: f32, height: f32, kind: f32) -> Self {
        Self {
            rect: [0.0, 0.0, width, height],
            shape: [0.0, kind, 0.0, 1.0],
            tint: [1.0; 4],
            material: [0.0; 4],
            cell: [0.0; 4],
            cut: NO_CUT,
        }
    }
}

fn intersection([ax, ay, aw, ah]: [f32; 4], [bx, by, bw, bh]: [f32; 4]) -> [f32; 4] {
    let x = ax.max(bx);
    let y = ay.max(by);
    [x, y, (ax + aw).min(bx + bw) - x, (ay + ah).min(by + bh) - y]
}

fn behind(run: &Run, [x, y, w, h]: [f32; 4]) -> bool {
    let line = run.size * lxb_toolkit::typography::Text::LINE * run.lines as f32;
    run.width > 0.0
        && !(run.left >= x + w
            || run.left + run.width <= x
            || run.top >= y + h
            || run.top + line <= y)
}

#[derive(Debug, Clone)]
pub(crate) struct Run {
    pub text: String,
    pub left: f32,
    pub top: f32,

    pub width: f32,

    pub lines: u8,
    pub size: f32,
    pub bold: bool,

    pub tint: [f32; 4],

    pub clip: Option<[f32; 4]>,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct Layer {
    pub quads: Vec<Quad>,
    pub runs: Vec<Run>,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct Scene {
    pub width: f32,
    pub height: f32,
    pub time: f32,
    pub wallpaper_style: f32,
    pub sky: [[f32; 4]; 4],
    pub accent: [[f32; 4]; 3],
    pub glow: [f32; 4],
    pub layers: [Layer; 5],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Frame {
    resolution: [f32; 4],
    atlas_size: [f32; 4],
    sky: [[f32; 4]; 4],
    accent: [[f32; 4]; 3],
    glow: [f32; 4],
}

struct Target {
    view: wgpu::TextureView,

    levels: Vec<wgpu::TextureView>,
}

impl Target {
    fn new(device: &wgpu::Device, width: u32, height: u32, label: &str) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: MIP_LEVELS,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TARGET,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let levels = (0..MIP_LEVELS)
            .map(|level| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    base_mip_level: level,
                    mip_level_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        Self { view, levels }
    }
}

const MIP_WGSL: &str = r#"
@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;

struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs(@builtin(vertex_index) index: u32) -> Out {
    let points = array(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    var out: Out;
    out.position = vec4<f32>(points[index], 0.0, 1.0);
    out.uv = points[index] * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5);
    return out;
}

@fragment
fn fs(in: Out) -> @location(0) vec4<f32> {
    return textureSample(source, source_sampler, in.uv);
}
"#;

pub struct Ui {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    width: u32,
    height: u32,

    quads_offscreen: wgpu::RenderPipeline,
    quads_output: wgpu::RenderPipeline,
    mip: wgpu::RenderPipeline,
    mip_layout: wgpu::BindGroupLayout,

    frame_buffer: wgpu::Buffer,
    instances: wgpu::Buffer,
    instance_room: u64,

    bind_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    atlas_texture: wgpu::TextureView,
    atlas_size: [f32; 2],
    thumbnail_texture: wgpu::Texture,
    thumbnail_view: wgpu::TextureView,
    thumbnail_requests: mpsc::Sender<ThumbnailRequest>,
    thumbnail_results: mpsc::Receiver<ThumbnailResult>,
    thumbnail_pending: HashMap<PathBuf, ThumbnailFingerprint>,
    thumbnail_failures: HashMap<PathBuf, ThumbnailFingerprint>,
    thumbnails: HashMap<PathBuf, ResidentThumbnail>,
    thumbnail_slots: [Option<PathBuf>; THUMBNAIL_COUNT],
    thumbnail_frame: u64,
    headless: bool,

    cells: Vec<[f32; 4]>,

    // The final OVER layer writes straight to the output. Every earlier layer
    // needs a texture for the one after it to sample, including SOFTEN.
    chain: [Target; 4],

    font_system: FontSystem,
    swash: SwashCache,
    text_atlas_offscreen: TextAtlas,
    text_atlas_output: TextAtlas,
    text_offscreen: TextRenderer,
    text_output: TextRenderer,
    viewport: Viewport,
    buffers: Vec<Buffer>,
    measured: std::collections::HashMap<(String, u32, bool), f32>,

    pub(crate) scene: Scene,
    pub(crate) scale: f32,
    pub(crate) colours: [[f32; 4]; 14],

    pub(crate) in_overlay: bool,

    pub(crate) dt: f32,
    last_time: Option<f32>,

    pub(crate) light: Option<[f32; 4]>,

    pub(crate) icons: lxb_toolkit::settings::IconStyle,

    pub(crate) spots: Vec<(crate::Spot, [f32; 4])>,
}

impl Ui {
    pub async fn new(
        instance: &wgpu::Instance,
        surface: Option<&wgpu::Surface<'static>>,
        output_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        let headless = surface.is_none();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: surface,
                ..Default::default()
            })
            .await
            .map_err(|err| format!("no graphics adapter: {err}"))?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("hello-lxb"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults()
                    .using_resolution(adapter.limits()),
                ..Default::default()
            })
            .await
            .map_err(|err| format!("no graphics device: {err}"))?;

        let Atlas {
            plane: atlas_pixels,
            width: atlas_width,
            height: atlas_height,
            cells,
        } = build_atlas()?;
        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("marks"),
            size: wgpu::Extent3d {
                width: atlas_width,
                height: atlas_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas_pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(atlas_width),
                rows_per_image: Some(atlas_height),
            },
            wgpu::Extent3d {
                width: atlas_width,
                height: atlas_height,
                depth_or_array_layers: 1,
            },
        );
        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let thumbnail_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("file thumbnails"),
            size: wgpu::Extent3d {
                width: THUMBNAIL_COLUMNS * THUMBNAIL_SIZE,
                height: THUMBNAIL_ROWS * THUMBNAIL_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let thumbnail_view = thumbnail_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let (thumbnail_requests, thumbnail_results) = thumbnail_workers();

        let text =
            |wgsl: &'static [u8]| std::str::from_utf8(wgsl).expect("the shipped shaders are UTF-8");
        let source = format!(
            "{}\n{}\n{}\n{}",
            text(assets::WALLPAPER_WGSL),
            text(assets::GLASS_WGSL),
            text(assets::GLYPH_WGSL),
            include_str!("ui.wgsl"),
        );
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("tour"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });

        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frame"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                texture_entry(1),
                sampler_entry(2),
                texture_entry(3),
                sampler_entry(4),
                texture_entry(5),
                texture_entry(6),
            ],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("tour"),
            bind_group_layouts: &[Some(&bind_layout)],
            ..Default::default()
        });

        let attributes = wgpu::vertex_attr_array![
            0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4,
            5 => Float32x4];
        let buffers = [Some(wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Quad>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &attributes,
        })];

        let quads_offscreen = quad_pipeline(&device, &layout, &shader, &buffers, TARGET);
        let quads_output = quad_pipeline(&device, &layout, &shader, &buffers, output_format);

        let mip_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blur chain"),
            source: wgpu::ShaderSource::Wgsl(MIP_WGSL.into()),
        });
        let mip_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("blur chain"),
            entries: &[texture_entry(0), sampler_entry(1)],
        });
        let mip_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("blur chain"),
            bind_group_layouts: &[Some(&mip_layout)],
            ..Default::default()
        });
        let mip = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blur chain"),
            layout: Some(&mip_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &mip_shader,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &mip_shader,
                entry_point: Some("fs"),
                targets: &[Some(TARGET.into())],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("linear"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        let frame_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame"),
            size: std::mem::size_of::<Frame>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let instance_room = 4096;
        let instances = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quads"),
            size: instance_room * std::mem::size_of::<Quad>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut font_system = FontSystem::new();
        font_system
            .db_mut()
            .load_font_data(assets::FONT_REGULAR.to_vec());
        font_system
            .db_mut()
            .load_font_data(assets::FONT_BOLD.to_vec());

        let cache = Cache::new(&device);
        let mut text_atlas_offscreen = TextAtlas::new(&device, &queue, &cache, TARGET);
        let mut text_atlas_output = TextAtlas::new(&device, &queue, &cache, output_format);
        let text_offscreen = TextRenderer::new(
            &mut text_atlas_offscreen,
            &device,
            wgpu::MultisampleState::default(),
            None,
        );
        let text_output = TextRenderer::new(
            &mut text_atlas_output,
            &device,
            wgpu::MultisampleState::default(),
            None,
        );
        let viewport = Viewport::new(&device, &cache);

        Ok(Self {
            chain: [
                Target::new(&device, width, height, "ground"),
                Target::new(&device, width, height, "panes"),
                Target::new(&device, width, height, "page"),
                Target::new(&device, width, height, "soft edges"),
            ],
            device,
            queue,
            width: width.max(1),
            height: height.max(1),
            quads_offscreen,
            quads_output,
            mip,
            mip_layout,
            frame_buffer,
            instances,
            instance_room,
            bind_layout,
            sampler,
            atlas_texture: atlas_view,
            atlas_size: [atlas_width as f32, atlas_height as f32],
            thumbnail_texture,
            thumbnail_view,
            thumbnail_requests,
            thumbnail_results,
            thumbnail_pending: HashMap::new(),
            thumbnail_failures: HashMap::new(),
            thumbnails: HashMap::new(),
            thumbnail_slots: std::array::from_fn(|_| None),
            thumbnail_frame: 0,
            headless,
            cells,
            font_system,
            swash: SwashCache::new(),
            text_atlas_offscreen,
            text_atlas_output,
            text_offscreen,
            text_output,
            viewport,
            buffers: Vec::new(),
            measured: std::collections::HashMap::new(),
            scene: Scene::default(),
            scale: 1.0,
            colours: [[0.0; 4]; 14],
            in_overlay: false,
            light: None,
            icons: lxb_toolkit::settings::IconStyle::default(),
            spots: Vec::new(),
            dt: 0.0,
            last_time: None,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width.max(1) == self.width && height.max(1) == self.height {
            return;
        }
        self.width = width.max(1);
        self.height = height.max(1);
        self.chain = [
            Target::new(&self.device, self.width, self.height, "ground"),
            Target::new(&self.device, self.width, self.height, "panes"),
            Target::new(&self.device, self.width, self.height, "page"),
            Target::new(&self.device, self.width, self.height, "soft edges"),
        ];
    }
}

fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn sampler_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn quad_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    buffers: &[Option<wgpu::VertexBufferLayout>],
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("quads"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs"),
            buffers,
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs"),
            targets: &[Some(wgpu::ColorTargetState {
                format,

                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

struct Atlas {
    plane: Vec<u8>,
    width: u32,
    height: u32,
    cells: Vec<[f32; 4]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ThumbnailFingerprint {
    bytes: u64,
    modified_seconds: u64,
    modified_nanos: u32,
}

impl ThumbnailFingerprint {
    fn of(path: &Path) -> Option<Self> {
        let facts = std::fs::metadata(path).ok()?;
        let modified = facts.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
        Some(Self {
            bytes: facts.len(),
            modified_seconds: modified.as_secs(),
            modified_nanos: modified.subsec_nanos(),
        })
    }
}

#[derive(Debug)]
struct ThumbnailRequest {
    path: PathBuf,
    fingerprint: ThumbnailFingerprint,
}

#[derive(Debug)]
struct ThumbnailResult {
    path: PathBuf,
    fingerprint: ThumbnailFingerprint,
    picture: Option<ThumbnailPicture>,
}

#[derive(Debug)]
struct ThumbnailPicture {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Thumbnail {
    pub cell: [f32; 4],
    pub aspect: f32,
}

#[derive(Debug, Clone, Copy)]
struct ResidentThumbnail {
    slot: usize,
    fingerprint: ThumbnailFingerprint,
    shown: Thumbnail,
    used: u64,
}

fn thumbnail_workers() -> (
    mpsc::Sender<ThumbnailRequest>,
    mpsc::Receiver<ThumbnailResult>,
) {
    let (request_tx, request_rx) = mpsc::channel::<ThumbnailRequest>();
    let (result_tx, result_rx) = mpsc::channel::<ThumbnailResult>();
    let requests = Arc::new(Mutex::new(request_rx));
    for number in 0..2 {
        let requests = Arc::clone(&requests);
        let results = result_tx.clone();
        let _ = std::thread::Builder::new()
            .name(format!("lxb-thumbnail-{number}"))
            .spawn(move || loop {
                let request = {
                    let Ok(requests) = requests.lock() else {
                        break;
                    };
                    requests.recv()
                };
                let Ok(request) = request else {
                    break;
                };
                let picture = decode_thumbnail(&request.path);
                if results
                    .send(ThumbnailResult {
                        path: request.path,
                        fingerprint: request.fingerprint,
                        picture,
                    })
                    .is_err()
                {
                    break;
                }
            });
    }
    (request_tx, result_rx)
}

fn decode_thumbnail(path: &Path) -> Option<ThumbnailPicture> {
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"))
    {
        return decode_svg_thumbnail(path);
    }

    let data = std::fs::read(path).ok()?;
    let mut reader = image::ImageReader::new(std::io::Cursor::new(data))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode().ok()?;
    let scaled = if image.width() > THUMBNAIL_SIZE || image.height() > THUMBNAIL_SIZE {
        image.resize(
            THUMBNAIL_SIZE,
            THUMBNAIL_SIZE,
            image::imageops::FilterType::Lanczos3,
        )
    } else {
        image
    };
    let rgba = scaled.to_rgba8();
    Some(ThumbnailPicture {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    })
}

fn decode_svg_thumbnail(path: &Path) -> Option<ThumbnailPicture> {
    let data = std::fs::read(path).ok()?;
    let options = resvg::usvg::Options {
        resources_dir: path.parent().map(Path::to_path_buf),
        ..Default::default()
    };
    let tree = resvg::usvg::Tree::from_data(&data, &options).ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(THUMBNAIL_SIZE, THUMBNAIL_SIZE)?;
    let size = tree.size();
    let scale = (THUMBNAIL_SIZE as f32 / size.width()).min(THUMBNAIL_SIZE as f32 / size.height());
    let x = (THUMBNAIL_SIZE as f32 - size.width() * scale) * 0.5;
    let y = (THUMBNAIL_SIZE as f32 - size.height() * scale) * 0.5;
    let transform = resvg::tiny_skia::Transform::from_translate(x, y).pre_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    let mut rgba = pixmap.take();
    unpremultiply_rgba(&mut rgba);
    Some(ThumbnailPicture {
        width: THUMBNAIL_SIZE,
        height: THUMBNAIL_SIZE,
        rgba,
    })
}

fn unpremultiply_rgba(rgba: &mut [u8]) {
    for pixel in rgba.chunks_exact_mut(4) {
        let alpha = u16::from(pixel[3]);
        if alpha == 0 || alpha == 255 {
            continue;
        }
        for channel in &mut pixel[..3] {
            *channel = ((u16::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
        }
    }
}

fn build_atlas() -> Result<Atlas, String> {
    let cell = glyph_material::CELL;
    let fine = cell * glyph_material::SDF_SUPERSAMPLE;
    let names: Vec<&'static str> = assets::glyph_names().collect();
    let columns = (names.len() as f32).sqrt().ceil() as u32;
    let rows = (names.len() as u32).div_ceil(columns);
    let (width, height) = (columns * cell, rows * cell);

    let mut plane = vec![0u8; (width * height) as usize];
    let mut cells = Vec::with_capacity(names.len());
    let options = resvg::usvg::Options::default();

    for (index, name) in names.iter().enumerate() {
        let svg = assets::glyph(name)
            .ok_or_else(|| format!("the library lists {name} and does not carry it"))?;
        let tree = resvg::usvg::Tree::from_data(svg, &options)
            .map_err(|err| format!("{name} does not parse: {err}"))?;
        let mut pixmap = resvg::tiny_skia::Pixmap::new(fine, fine)
            .ok_or_else(|| format!("no room to rasterise {name}"))?;
        let size = tree.size();
        let scale = fine as f32 / size.width().max(size.height()).max(1.0);
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );

        let coverage: Vec<u8> = pixmap.pixels().iter().map(|pixel| pixel.alpha()).collect();
        let field = glyph_material::distance_field_alpha_from_coverage(&coverage, cell)
            .ok_or_else(|| format!("{name} could not be measured"))?;

        let column = index as u32 % columns;
        let row = index as u32 / columns;
        for y in 0..cell {
            let from = (y * cell) as usize;
            let into = ((row * cell + y) * width + column * cell) as usize;
            plane[into..into + cell as usize].copy_from_slice(&field[from..from + cell as usize]);
        }
        cells.push([
            (column * cell) as f32 / width as f32,
            (row * cell) as f32 / height as f32,
            ((column + 1) * cell) as f32 / width as f32,
            ((row + 1) * cell) as f32 / height as f32,
        ]);
    }

    Ok(Atlas {
        plane,
        width,
        height,
        cells,
    })
}

impl Ui {
    fn sync_thumbnail_results(&mut self) {
        while let Ok(result) = self.thumbnail_results.try_recv() {
            if self.thumbnail_pending.get(&result.path).copied() != Some(result.fingerprint) {
                continue;
            }
            self.thumbnail_pending.remove(&result.path);
            match result.picture {
                Some(picture) => {
                    self.thumbnail_failures.remove(&result.path);
                    let _ = self.install_thumbnail(result.path, result.fingerprint, picture);
                }
                None => {
                    self.thumbnail_failures
                        .insert(result.path, result.fingerprint);
                }
            }
        }
    }

    fn install_thumbnail(
        &mut self,
        path: PathBuf,
        fingerprint: ThumbnailFingerprint,
        picture: ThumbnailPicture,
    ) -> Option<Thumbnail> {
        if picture.width == 0
            || picture.height == 0
            || picture.width > THUMBNAIL_SIZE
            || picture.height > THUMBNAIL_SIZE
            || picture.rgba.len() != (picture.width * picture.height * 4) as usize
        {
            return None;
        }

        if let Some(old) = self.thumbnails.remove(&path) {
            self.thumbnail_slots[old.slot] = None;
        }
        let slot = self
            .thumbnail_slots
            .iter()
            .position(Option::is_none)
            .or_else(|| {
                let oldest = self
                    .thumbnails
                    .iter()
                    .min_by_key(|(_, resident)| resident.used)
                    .map(|(path, resident)| (path.clone(), resident.slot))?;
                self.thumbnails.remove(&oldest.0);
                self.thumbnail_slots[oldest.1] = None;
                Some(oldest.1)
            })?;

        let mut square = vec![0u8; (THUMBNAIL_SIZE * THUMBNAIL_SIZE * 4) as usize];
        let source_row = (picture.width * 4) as usize;
        let target_row = (THUMBNAIL_SIZE * 4) as usize;
        for y in 0..picture.height as usize {
            let source = y * source_row;
            let target = y * target_row;
            square[target..target + source_row]
                .copy_from_slice(&picture.rgba[source..source + source_row]);
        }

        let column = slot as u32 % THUMBNAIL_COLUMNS;
        let row = slot as u32 / THUMBNAIL_COLUMNS;
        let x = column * THUMBNAIL_SIZE;
        let y = row * THUMBNAIL_SIZE;
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.thumbnail_texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            &square,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(THUMBNAIL_SIZE * 4),
                rows_per_image: Some(THUMBNAIL_SIZE),
            },
            wgpu::Extent3d {
                width: THUMBNAIL_SIZE,
                height: THUMBNAIL_SIZE,
                depth_or_array_layers: 1,
            },
        );

        let atlas_width = (THUMBNAIL_COLUMNS * THUMBNAIL_SIZE) as f32;
        let atlas_height = (THUMBNAIL_ROWS * THUMBNAIL_SIZE) as f32;
        let shown = Thumbnail {
            cell: [
                x as f32 / atlas_width,
                y as f32 / atlas_height,
                (x + picture.width) as f32 / atlas_width,
                (y + picture.height) as f32 / atlas_height,
            ],
            aspect: picture.width as f32 / picture.height as f32,
        };
        self.thumbnail_slots[slot] = Some(path.clone());
        self.thumbnails.insert(
            path,
            ResidentThumbnail {
                slot,
                fingerprint,
                shown,
                used: self.thumbnail_frame,
            },
        );
        Some(shown)
    }

    pub(crate) fn thumbnail(&mut self, path: &Path) -> Option<Thumbnail> {
        let fingerprint = ThumbnailFingerprint::of(path)?;
        if let Some(resident) = self.thumbnails.get(path).copied() {
            if resident.fingerprint == fingerprint {
                if let Some(resident) = self.thumbnails.get_mut(path) {
                    resident.used = self.thumbnail_frame;
                }
                return Some(resident.shown);
            }
            self.thumbnails.remove(path);
            self.thumbnail_slots[resident.slot] = None;
        }
        if self.thumbnail_failures.get(path).copied() == Some(fingerprint) {
            return None;
        }

        if self.headless {
            return match decode_thumbnail(path) {
                Some(picture) => {
                    self.thumbnail_failures.remove(path);
                    self.install_thumbnail(path.to_path_buf(), fingerprint, picture)
                }
                None => {
                    self.thumbnail_failures
                        .insert(path.to_path_buf(), fingerprint);
                    None
                }
            };
        }

        if self.thumbnail_pending.get(path).copied() != Some(fingerprint) {
            let request = ThumbnailRequest {
                path: path.to_path_buf(),
                fingerprint,
            };
            if self.thumbnail_requests.send(request).is_ok() {
                self.thumbnail_pending
                    .insert(path.to_path_buf(), fingerprint);
            }
        }
        None
    }
}

impl Ui {
    fn bind(&self, source: &wgpu::TextureView, beneath: &wgpu::TextureView) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frame"),
            layout: &self.bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.frame_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(source),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&self.atlas_texture),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&self.thumbnail_view),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(beneath),
                },
            ],
        })
    }

    fn build_chain(&self, encoder: &mut wgpu::CommandEncoder, target: &Target) {
        for level in 1..MIP_LEVELS as usize {
            let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("blur rung"),
                layout: &self.mip_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&target.levels[level - 1]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("blur rung"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.levels[level],
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.mip);
            pass.set_bind_group(0, &bind, &[]);
            pass.draw(0..3, 0..1);
        }
    }

    fn shape(&mut self, runs: &[Run], first: usize) {
        while self.buffers.len() < first + runs.len() {
            let buffer = Buffer::new(
                &mut self.font_system,
                Metrics::new(16.0, 16.0 * lxb_toolkit::typography::Text::LINE),
            );
            self.buffers.push(buffer);
        }
        for (index, run) in runs.iter().enumerate() {
            let buffer = &mut self.buffers[first + index];
            buffer.set_metrics(Metrics::new(
                run.size,
                run.size * lxb_toolkit::typography::Text::LINE,
            ));
            buffer.set_size(None, None);
            let attrs = Attrs::new()
                .family(Family::Name("Roboto"))
                .weight(if run.bold {
                    Weight::BOLD
                } else {
                    Weight::NORMAL
                });
            buffer.set_text(&run.text, &attrs, Shaping::Advanced, None);
            if run.width > 0.0 && run.lines > 1 {
                buffer.set_size(
                    Some(run.width),
                    Some(run.lines as f32 * run.size * lxb_toolkit::typography::Text::LINE),
                );
            }
            buffer.shape_until_scroll(&mut self.font_system, false);
        }
    }

    fn draw(&mut self, scene: &Scene, output: &wgpu::TextureView) -> Result<(), String> {
        self.queue.write_buffer(
            &self.frame_buffer,
            0,
            bytemuck::bytes_of(&Frame {
                resolution: [scene.width, scene.height, scene.time, scene.wallpaper_style],
                atlas_size: [
                    1.0 / self.atlas_size[0],
                    1.0 / self.atlas_size[1],
                    BLUR_LEVELS as f32,
                    0.0,
                ],
                sky: scene.sky,
                accent: scene.accent,
                glow: scene.glow,
            }),
        );

        let last = scene.layers.len() - 1;
        let passes: Vec<usize> = (0..scene.layers.len())
            .filter(|&index| {
                index == GROUND
                    || index == last
                    || !scene.layers[index].quads.is_empty()
                    || !scene.layers[index].runs.is_empty()
            })
            .collect();

        let panes = passes.contains(&PANE);

        let mut sky = Quad::full(scene.width, scene.height, KIND_WALLPAPER);

        sky.material[0] = lxb_toolkit::wallpaper::SOFTEN;
        let mut quads = vec![sky];
        let mut spans: Vec<std::ops::Range<u32>> = Vec::with_capacity(passes.len());
        for (order, &index) in passes.iter().enumerate() {
            if order > 0 {
                quads.push(Quad::full(scene.width, scene.height, KIND_BLIT));
            }
            let first = if order == 0 {
                0
            } else {
                quads.len() as u32 - 1
            };
            quads.extend_from_slice(&scene.layers[index].quads);
            spans.push(first..quads.len() as u32);
        }

        if quads.len() as u64 > self.instance_room {
            self.instance_room = (quads.len() as u64).next_power_of_two();
            self.instances = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("quads"),
                size: self.instance_room * std::mem::size_of::<Quad>() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        self.queue
            .write_buffer(&self.instances, 0, bytemuck::cast_slice(&quads));

        self.viewport.update(
            &self.queue,
            Resolution {
                width: self.width,
                height: self.height,
            },
        );
        let page_runs = scene.layers[CONTROL].runs.len();
        self.shape(&scene.layers[CONTROL].runs, 0);
        let over_runs = scene.layers[OVER].runs.clone();
        self.shape(&over_runs, page_runs);

        let whole = TextBounds {
            left: 0,
            top: 0,
            right: self.width as i32,
            bottom: self.height as i32,
        };
        for (layer, first, count) in [
            (CONTROL, 0usize, page_runs),
            (OVER, page_runs, over_runs.len()),
        ] {
            let runs = &scene.layers[layer].runs;
            let areas = runs.iter().enumerate().map(|(index, run)| TextArea {
                buffer: &self.buffers[first + index],
                left: run.left,
                top: run.top,
                scale: 1.0,
                // **Rounded inwards, every side.** A clip on a quad is tested
                // against the pixel's own centre, in floats; a clip on a word
                // is a whole number of pixels handed to the text renderer.
                // Rounded outwards, the two disagree by up to a pixel — and
                // where a list is cut, that pixel is a row of letter-tips
                // surviving below everything else, sharp, under a boundary
                // that has already dissolved. Rounded inwards they cannot:
                // the worst it costs is the outermost pixel of a glyph that
                // was being cut in half anyway.
                bounds: match run.clip {
                    Some([x, y, w, h]) => TextBounds {
                        left: x.ceil() as i32,
                        top: y.ceil() as i32,
                        right: (x + w).floor() as i32,
                        bottom: (y + h).floor() as i32,
                    },
                    None => whole,
                },
                default_color: linear_color(run.tint),
                custom_glyphs: &[],
            });
            let (renderer, atlas) = if layer == CONTROL {
                (&mut self.text_offscreen, &mut self.text_atlas_offscreen)
            } else {
                (&mut self.text_output, &mut self.text_atlas_output)
            };
            let _ = count;
            renderer
                .prepare(
                    &self.device,
                    &self.queue,
                    &mut self.font_system,
                    atlas,
                    &self.viewport,
                    areas,
                    &mut self.swash,
                )
                .map_err(|err| format!("the words could not be laid out: {err}"))?;
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("tour"),
            });

        for (order, span) in spans.iter().enumerate() {
            let index = passes[order];

            let source = if order == 0 {
                &self.chain[CONTROL].view
            } else {
                &self.chain[passes[order - 1]].view
            };
            // What is behind a page's own content: its panes over the ground,
            // or the bare ground where it drew no panes. It is what a soft
            // edge dissolves into, and the one thing a pass cannot sample is
            // the texture it is drawing into — so a layer at or below the one
            // it would name is given the source it already has, which no
            // effect on those layers reads.
            let beneath = if index <= PANE {
                source
            } else if panes {
                &self.chain[PANE].view
            } else {
                &self.chain[GROUND].view
            };
            let bind = self.bind(source, beneath);
            let output_pass = index == last;
            let into = if output_pass {
                output
            } else {
                &self.chain[index].levels[0]
            };
            {
                let mut pass = colour_pass(&mut encoder, into, "layer");
                pass.set_pipeline(if output_pass {
                    &self.quads_output
                } else {
                    &self.quads_offscreen
                });
                pass.set_bind_group(0, &bind, &[]);
                pass.set_vertex_buffer(0, self.instances.slice(..));
                pass.draw(0..6, span.clone());

                if index == CONTROL {
                    self.text_offscreen
                        .render(&self.text_atlas_offscreen, &self.viewport, &mut pass)
                        .map_err(|err| format!("the page's words could not be drawn: {err}"))?;
                } else if output_pass {
                    self.text_output
                        .render(&self.text_atlas_output, &self.viewport, &mut pass)
                        .map_err(|err| format!("the overlay's words could not be drawn: {err}"))?;
                }
            }
            if !output_pass {
                self.build_chain(&mut encoder, &self.chain[index]);
            }
        }

        self.queue.submit(Some(encoder.finish()));
        self.text_atlas_offscreen.trim();
        self.text_atlas_output.trim();
        Ok(())
    }
}

fn colour_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    view: &'a wgpu::TextureView,
    label: &'static str,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    })
}

fn linear_color(tint: [f32; 4]) -> Color {
    let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color::rgba(byte(tint[0]), byte(tint[1]), byte(tint[2]), byte(tint[3]))
}

pub fn instance() -> wgpu::Instance {
    wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle())
}

impl Ui {
    pub(crate) fn shaped_width(&mut self, text: &str, size: f32, bold: bool) -> f32 {
        let key = (text.to_string(), size.to_bits(), bold);
        if let Some(width) = self.measured.get(&key) {
            return *width;
        }
        let mut buffer = Buffer::new(
            &mut self.font_system,
            Metrics::new(size, size * lxb_toolkit::typography::Text::LINE),
        );
        buffer.set_size(None, None);
        let attrs = Attrs::new().family(Family::Name("Roboto")).weight(if bold {
            Weight::BOLD
        } else {
            Weight::NORMAL
        });
        buffer.set_text(text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut self.font_system, false);
        let width = buffer
            .layout_runs()
            .map(|run| run.line_w)
            .fold(0.0f32, f32::max);
        self.measured.insert(key, width);
        width
    }

    pub(crate) fn cell(&self, mark: usize) -> [f32; 4] {
        self.cells.get(mark).copied().unwrap_or([0.0; 4])
    }
}

fn read_back(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, String> {
    let row = (width * 8).div_ceil(256) * 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (row * height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("readback"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));

    let slice = buffer.slice(..);
    let (send, receive) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = send.send(result);
    });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .map_err(|err| err.to_string())?;
    receive
        .recv()
        .map_err(|err| err.to_string())?
        .map_err(|err| err.to_string())?;

    let mapped = slice.get_mapped_range().map_err(|err| err.to_string())?;
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        let line = &mapped[(y * row) as usize..(y * row + width * 8) as usize];
        for pixel in line.chunks_exact(8) {
            let channel = |at: usize| half_to_f32(u16::from_le_bytes([pixel[at], pixel[at + 1]]));
            for at in [0, 2, 4] {
                pixels.push((lxb_toolkit::color::to_srgb(channel(at)) * 255.0).round() as u8);
            }
            pixels.push((channel(6).clamp(0.0, 1.0) * 255.0).round() as u8);
        }
    }
    drop(mapped);
    buffer.unmap();
    Ok(pixels)
}

fn half_to_f32(bits: u16) -> f32 {
    let sign = ((bits >> 15) & 1) as u32;
    let exponent = ((bits >> 10) & 0x1f) as u32;
    let mantissa = (bits & 0x3ff) as u32;
    let value = match exponent {
        0 => {
            if mantissa == 0 {
                0
            } else {
                let mut exponent = -1i32;
                let mut mantissa = mantissa;
                while mantissa & 0x400 == 0 {
                    mantissa <<= 1;
                    exponent -= 1;
                }
                let exponent = (exponent + 127 - 15) as u32;
                (sign << 31) | (exponent << 23) | ((mantissa & 0x3ff) << 13)
            }
        }
        0x1f => (sign << 31) | (0xff << 23) | (mantissa << 13),
        _ => (sign << 31) | ((exponent + 127 - 15) << 23) | (mantissa << 13),
    };
    f32::from_bits(value)
}

#[allow(dead_code)]
pub(crate) const GROUND: usize = 0;

pub(crate) const PANE: usize = 1;

pub(crate) const CONTROL: usize = 2;

/// Post-composite effects that need to sample the complete page — including
/// its words — but must remain behind menus and dialogs.
pub(crate) const SOFTEN: usize = 3;

pub(crate) const OVER: usize = 4;

/// How much of the scene had been written when this was taken.
///
/// Opaque on purpose: it means nothing except to the [`Ui`] that gave it out,
/// and only until that frame ends. See [`Ui::written`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Written {
    layer: usize,
    quads: usize,
    runs: usize,
}

impl Ui {
    pub fn begin(
        &mut self,
        width: f32,
        height: f32,
        seconds: f32,
        accent: &lxb_toolkit::accent::Accent,
        wallpaper: lxb_toolkit::settings::WallpaperStyle,
        icons: lxb_toolkit::settings::IconStyle,
    ) {
        self.thumbnail_frame = self.thumbnail_frame.wrapping_add(1);
        self.sync_thumbnail_results();
        self.icons = icons;

        self.spots.clear();
        self.resize(width.max(1.0) as u32, height.max(1.0) as u32);
        self.scale = lxb_toolkit::metrics::scale_for(height);

        self.light = None;

        self.dt = self
            .last_time
            .map_or(0.0, |last| (seconds - last).clamp(0.0, 0.1));
        self.last_time = Some(seconds);

        let shown = accent.shown();
        for (index, role) in lxb_toolkit::palette::Role::ALL.iter().enumerate() {
            self.colours[index] = shown.a(*role, 1.0);
        }
        let role = |role: lxb_toolkit::palette::Role| shown.a(role, 1.0);

        self.scene = Scene {
            width,
            height,
            time: seconds,
            wallpaper_style: lxb_toolkit::wallpaper::shader_style(wallpaper),
            sky: [
                role(lxb_toolkit::palette::Role::SkyTop),
                role(lxb_toolkit::palette::Role::SkyBottom),
                role(lxb_toolkit::palette::Role::SkyTopAlt),
                role(lxb_toolkit::palette::Role::SkyBottomAlt),
            ],
            accent: [
                role(lxb_toolkit::palette::Role::Accent),
                role(lxb_toolkit::palette::Role::AccentSoft),
                role(lxb_toolkit::palette::Role::AccentDeep),
            ],
            glow: role(lxb_toolkit::palette::Role::Glow),
            layers: Default::default(),
        };
    }

    pub fn end(&mut self, view: &wgpu::TextureView) -> Result<(), String> {
        let scene = std::mem::take(&mut self.scene);
        self.draw(&scene, view)
    }

    pub(crate) fn mark(&self, layer: usize) -> (usize, usize) {
        (
            self.scene.layers[layer].quads.len(),
            self.scene.layers[layer].runs.len(),
        )
    }

    /// Where the scene had got to, so that what is drawn after it can be
    /// treated as one thing.
    ///
    /// See [`Ui::cut_between`]. Quads and words are counted apart because they
    /// are drawn apart: every quad of a layer goes down before any of that
    /// layer's words, so a list's cards and a list's names are two ranges of
    /// the scene and not one.
    pub fn written(&self) -> Written {
        let layer = self.layer();
        let (quads, runs) = self.mark(layer);
        Written { layer, quads, runs }
    }

    /// Cut everything drawn between two marks down to a rectangle.
    ///
    /// **This is what lets a list end at an edge instead of at a whole row.**
    /// Without it a row shown in part is a row drawn in full somewhere, and
    /// the somewhere is whatever the list was supposed to stop short of: a
    /// heading, a legend, the far side of a panel. A quad outside the
    /// rectangle is discarded by the pixel; a word is bounded to it.
    ///
    /// Both marks have to have been taken on the same layer — one inside an
    /// overlay and one outside it describe two different scenes — and `to`
    /// cannot be before `from`. Either mistake cuts nothing rather than
    /// cutting the wrong thing.
    pub fn cut_between(&mut self, from: Written, to: Written, rect: [f32; 4]) {
        if from.layer != to.layer || to.quads < from.quads || to.runs < from.runs {
            return;
        }
        self.clipped(
            from.layer,
            (from.quads, from.runs),
            Some((to.quads, to.runs)),
            rect,
        );
    }

    /// Fade everything drawn between two marks, all the way down to nothing.
    ///
    /// **The companion to [`Ui::cut_between`], and the only way a page can
    /// fade what it has drawn.** A page that hands a strength to every call it
    /// makes can fade its words and its pictures, but not its glass: a glass
    /// quad's material is not scaled by its own tint, so `Ui::card` at a tint
    /// alpha of a thousandth still refracts, glosses and rims exactly as hard
    /// as at one, and then goes out in a single frame. This multiplies the one
    /// channel the shader carries through every kind — `shape[3]` — so glass
    /// fades with everything beside it.
    ///
    /// It is also the shorter road for a page crossing out of the card that
    /// opened it: draw the page, fade the whole of it, rather than carry one
    /// number through every call that draws a part of it.
    ///
    /// The same two rules as the cut: both marks have to have been taken on
    /// the same layer, and `to` cannot be before `from`. Either mistake fades
    /// nothing rather than fading the wrong thing.
    pub fn fade_between(&mut self, from: Written, to: Written, fade: f32) {
        if from.layer != to.layer || to.quads < from.quads || to.runs < from.runs {
            return;
        }
        self.faded(
            from.layer,
            (from.quads, from.runs),
            Some((to.quads, to.runs)),
            fade.clamp(0.0, 1.0),
        );
    }

    pub(crate) fn flew(
        &mut self,
        layer: usize,
        mark: (usize, usize),
        factor: f32,
        offset: [f32; 2],
    ) {
        for quad in &mut self.scene.layers[layer].quads[mark.0..] {
            quad.rect[0] = quad.rect[0] * factor + offset[0];
            quad.rect[1] = quad.rect[1] * factor + offset[1];
            quad.rect[2] *= factor;
            quad.rect[3] *= factor;
            quad.shape[0] *= factor;
            quad.material[0] *= factor;
            if quad.cut != NO_CUT {
                quad.cut[0] = quad.cut[0] * factor + offset[0];
                quad.cut[1] = quad.cut[1] * factor + offset[1];
                quad.cut[2] = quad.cut[2] * factor + offset[0];
                quad.cut[3] = quad.cut[3] * factor + offset[1];
            }
        }
        for run in &mut self.scene.layers[layer].runs[mark.1..] {
            run.left = run.left * factor + offset[0];
            run.top = run.top * factor + offset[1];
            run.width *= factor;
            run.size *= factor;

            run.clip = run.clip.map(|[x, y, w, h]| {
                [
                    x * factor + offset[0],
                    y * factor + offset[1],
                    w * factor,
                    h * factor,
                ]
            });
        }
    }

    pub(crate) fn faded(
        &mut self,
        layer: usize,
        from: (usize, usize),
        to: Option<(usize, usize)>,
        alpha: f32,
    ) {
        let layer = &mut self.scene.layers[layer];
        let quads_to = to.map_or(layer.quads.len(), |mark| mark.0);
        let runs_to = to.map_or(layer.runs.len(), |mark| mark.1);
        for quad in &mut layer.quads[from.0..quads_to] {
            quad.shape[3] *= alpha;
        }
        for run in &mut layer.runs[from.1..runs_to] {
            run.tint[3] *= alpha;
        }
    }

    pub(crate) fn clipped(
        &mut self,
        layer: usize,
        from: (usize, usize),
        to: Option<(usize, usize)>,
        clip: [f32; 4],
    ) {
        let layer = &mut self.scene.layers[layer];
        let quads_to = to.map_or(layer.quads.len(), |mark| mark.0);
        let runs_to = to.map_or(layer.runs.len(), |mark| mark.1);
        for quad in &mut layer.quads[from.0..quads_to] {
            *quad = quad.clipped(clip);
        }
        for run in &mut layer.runs[from.1..runs_to] {
            let cut = run
                .clip
                .map_or(clip, |existing| intersection(existing, clip));
            run.clip = Some([cut[0], cut[1], cut[2].max(0.0), cut[3].max(0.0)]);
        }
    }

    pub fn recede(&mut self, depth: f32, about: [f32; 4]) {
        let depth = depth.clamp(0.0, 1.0);
        if depth <= 0.0 {
            return;
        }
        let factor = 1.0 + (1.0 - lxb_toolkit::menu::DEPTH - 1.0) * depth;

        let offset = [
            (about[0] + about[2] * 0.5) * (1.0 - factor),
            (about[1] + about[3] * 0.5) * (1.0 - factor),
        ];
        for layer in [PANE, CONTROL, SOFTEN] {
            self.flew(layer, (0, 0), factor, offset);
        }
    }

    pub fn recede_behind(&mut self, panel: [f32; 4], dim: f32, amount: f32) {
        for layer in [PANE, CONTROL, SOFTEN] {
            self.faded(layer, (0, 0), None, dim);
        }
        self.cut_text_behind(panel, amount.clamp(0.0, 1.0));
    }

    fn cut_text_behind(&mut self, rect: [f32; 4], amount: f32) {
        self.cut_text_under(&[PANE, CONTROL], rect, amount);
    }

    pub fn cut_text_under(&mut self, layers: &[usize], rect: [f32; 4], amount: f32) {
        let [x, _, w, _] = rect;
        if w <= 0.0 || amount <= 0.0 {
            return;
        }
        let panel = [x, w];
        for &layer in layers {
            let runs = std::mem::take(&mut self.scene.layers[layer].runs);
            let mut kept = Vec::with_capacity(runs.len());
            for run in runs {
                if !behind(&run, rect) {
                    kept.push(run);
                    continue;
                }
                let piece = |from: f32, width: f32| {
                    let line = lxb_toolkit::typography::Text::LINE;
                    let box_of_it = [from, run.top, width, run.size * line * run.lines as f32];
                    let cut = match run.clip {
                        Some(clip) => intersection(clip, box_of_it),
                        None => box_of_it,
                    };
                    (cut[2] > 0.0 && cut[3] > 0.0).then_some(cut)
                };
                for clip in [
                    piece(run.left, panel[0] - run.left),
                    piece(
                        panel[0] + panel[1],
                        run.left + run.width - (panel[0] + panel[1]),
                    ),
                ]
                .into_iter()
                .flatten()
                {
                    kept.push(Run {
                        clip: Some(clip),
                        ..run.clone()
                    });
                }
                if amount < 1.0 {
                    if let Some(clip) = piece(panel[0], panel[1]) {
                        let mut under = run;
                        under.tint[3] *= 1.0 - amount;
                        under.clip = Some(clip);
                        kept.push(under);
                    }
                }
            }
            self.scene.layers[layer].runs = kept;
        }
    }

    pub fn seconds(&self) -> f32 {
        self.last_time.unwrap_or(0.0)
    }

    pub fn dt(&self) -> f32 {
        self.dt
    }

    pub(crate) fn layer(&self) -> usize {
        if self.in_overlay {
            OVER
        } else {
            CONTROL
        }
    }

    pub fn s(&self, reference: f32) -> f32 {
        reference * self.scale
    }

    pub fn m(&self, metric: lxb_toolkit::metrics::Metric) -> f32 {
        metric.on(self.scene.height)
    }

    pub fn size(&self, text: lxb_toolkit::typography::Text) -> f32 {
        text.on(self.scene.height)
    }

    pub fn line(&self, text: lxb_toolkit::typography::Text) -> f32 {
        text.on(self.scene.height) * lxb_toolkit::typography::Text::LINE
    }

    pub fn role(&self, role: lxb_toolkit::palette::Role) -> [f32; 4] {
        self.colours[role as usize]
    }

    pub fn tinted(&self, role: lxb_toolkit::palette::Role, alpha: f32) -> [f32; 4] {
        let [r, g, b, _] = self.role(role);
        [r, g, b, alpha]
    }

    pub fn width(&self) -> f32 {
        self.scene.width
    }

    pub fn height(&self) -> f32 {
        self.scene.height
    }

    pub fn measure(&mut self, text: lxb_toolkit::typography::Text, string: &str) -> f32 {
        let size = self.size(text);
        let bold = text.face() == lxb_toolkit::typography::Face::Bold;
        self.shaped_width(string, size, bold)
    }

    pub fn wrap(
        &mut self,
        text: lxb_toolkit::typography::Text,
        string: &str,
        room: f32,
    ) -> Vec<String> {
        let mut lines = Vec::new();
        let mut line = String::new();
        for word in string.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && self.measure(text, &candidate) > room {
                lines.push(std::mem::take(&mut line));
                line = word.to_string();
            } else {
                line = candidate;
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
        lines
    }

    pub(crate) fn quad(&mut self, layer: usize, quad: Quad) {
        if quad.tint[3] > 0.0 {
            self.scene.layers[layer].quads.push(quad);
        }
    }

    pub(crate) fn run(&mut self, layer: usize, run: Run) {
        if !run.text.is_empty() && run.tint[3] > 0.0 {
            self.scene.layers[layer].runs.push(run);
        }
    }

    pub(crate) fn mark_index(&self, name: &str) -> Option<usize> {
        lxb_toolkit::assets::glyph_names().position(|other| other == name)
    }
}

impl Ui {
    pub fn headless(width: u32, height: u32) -> Result<Self, String> {
        let instance = instance();
        pollster_block_on(Ui::new(&instance, None, TARGET, width, height))
    }

    pub fn end_to_image(&mut self) -> Result<Vec<u8>, String> {
        let width = self.width_px();
        let height = self.height_px();
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("image"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TARGET,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.end(&view)?;
        read_back(&self.device, &self.queue, &texture, width, height)
    }

    pub fn width_px(&self) -> u32 {
        self.width
    }

    pub fn height_px(&self) -> u32 {
        self.height
    }
}

fn pollster_block_on<F: std::future::Future>(mut future: F) -> F::Output {
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

    static VTABLE: RawWakerVTable =
        RawWakerVTable::new(|data| RawWaker::new(data, &VTABLE), |_| {}, |_| {}, |_| {});
    let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) };
    let mut context = Context::from_waker(&waker);
    let mut future = unsafe { std::pin::Pin::new_unchecked(&mut future) };
    loop {
        if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
            return value;
        }
        std::thread::yield_now();
    }
}
