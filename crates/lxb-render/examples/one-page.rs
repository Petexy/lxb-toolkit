use lxb_render::{Align, Press, Ui};
use lxb_toolkit::{
    accent::Accent, material::Overlay, metrics::Metric, palette::Role, settings::ShellTheme,
    typography::Text,
};

fn main() -> Result<(), String> {
    let (width, height) = (960u32, 540u32);

    let theme = ShellTheme::load();
    let accent = Accent::new(theme.accent.name).unwrap_or_else(Accent::default_accent);

    let mut ui = Ui::headless(width, height)?;
    ui.begin(
        width as f32,
        height as f32,
        8.0,
        &accent,
        theme.wallpaper,
        theme.icons,
    );

    let inset = ui.m(Metric::PanelInset);
    let pad = ui.m(Metric::PanelPadding);
    let gap = ui.m(Metric::Gap);
    let icon = ui.m(Metric::ItemIcon);
    let pane = [
        inset,
        inset,
        width as f32 - 2.0 * inset,
        height as f32 - 2.0 * inset,
    ];
    ui.pane(pane, Overlay::Dialog);

    let head = ui.line(Text::Display).max(icon);
    ui.icon(
        [
            pane[0] + pad,
            pane[1] + pad + (head - icon) / 2.0,
            icon,
            icon,
        ],
        "launch",
        theme.icons,
    );
    ui.label(
        [pane[0] + pad + icon + gap, pane[1] + pad, pane[2], head],
        Text::Display,
        "Hello, world",
        Role::Text,
        Align::Left,
    );

    let capsule = ui.m(Metric::RowHeight) * 0.6;
    ui.button(
        [
            pane[0] + pad,
            pane[1] + pad + head + gap,
            ui.m(Metric::MenuWidth) * 0.5,
            capsule,
        ],
        "Press",
        Press::Focused,
    );

    let pixels = ui.end_to_image()?;
    write_png("one-page.png", width, height, &pixels)?;
    println!("one-page.png — {width}x{height}");
    Ok(())
}

fn write_png(path: &str, width: u32, height: u32, pixels: &[u8]) -> Result<(), String> {
    use std::io::Write;

    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(body);
        let mut crc = 0xffff_ffffu32;
        for byte in kind.iter().chain(body) {
            crc ^= *byte as u32;
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xedb8_8320
                } else {
                    crc >> 1
                };
            }
        }
        out.extend_from_slice(&(!crc).to_be_bytes());
    }

    let mut raw = Vec::with_capacity((width * height * 4 + height) as usize);
    for y in 0..height as usize {
        raw.push(0);
        let row = y * width as usize * 4;
        raw.extend_from_slice(&pixels[row..row + width as usize * 4]);
    }

    let mut zlib = vec![0x78, 0x01];
    for (index, block) in raw.chunks(65_535).enumerate() {
        let last = (index + 1) * 65_535 >= raw.len();
        zlib.push(if last { 1 } else { 0 });
        zlib.extend_from_slice(&(block.len() as u16).to_le_bytes());
        zlib.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for byte in &raw {
        a = (a + *byte as u32) % 65_521;
        b = (b + a) % 65_521;
    }
    zlib.extend_from_slice(&((b << 16) | a).to_be_bytes());

    let mut png = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut png, b"IHDR", &header);
    chunk(&mut png, b"IDAT", &zlib);
    chunk(&mut png, b"IEND", &[]);

    std::fs::File::create(path)
        .and_then(|mut file| file.write_all(&png))
        .map_err(|err| format!("{path}: {err}"))
}
