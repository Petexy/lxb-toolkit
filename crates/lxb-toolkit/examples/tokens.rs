use lxb_toolkit::accent::Accent;
use lxb_toolkit::material::Surface;
use lxb_toolkit::metrics::{capsule_radius, Metric};
use lxb_toolkit::motion::{self, duration};
use lxb_toolkit::palette::{Role, PALETTES};
use lxb_toolkit::sound::Sound;
use lxb_toolkit::typography::Text;

fn main() {
    let height: f32 = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(lxb_toolkit::metrics::REFERENCE_HEIGHT);

    println!(
        "lxb-toolkit {} — the LineXinBar design language, at {height:.0}p\n",
        lxb_toolkit::VERSION
    );

    println!("PALETTES");
    print!("  {:<16}", "");
    for palette in &PALETTES {
        print!("{:<10}", palette.name);
    }
    println!();
    for role in Role::ALL {
        print!("  {:<16}", role.name());
        for palette in &PALETTES {
            print!("{:<10}", palette.color(role).hex());
        }
        println!();
    }

    println!("\nSIZES");
    for metric in Metric::ALL {
        let value = metric.on(height);
        if metric.is_share() {
            println!("  {:<20} {value:>8.2}   (a share)", metric.name());
        } else {
            println!("  {:<20} {value:>8.1}px", metric.name());
        }
    }
    println!(
        "  {:<20} {:>8.1}px  (a control's radius is half its own height)",
        "capsule",
        capsule_radius(Metric::RowHeight.on(height))
    );

    println!("\nTYPE");
    for text in Text::ALL {
        println!(
            "  {:<20} {:>8.1}px  {}",
            text.name(),
            text.on(height),
            if text.face() == lxb_toolkit::typography::Face::Bold {
                "bold"
            } else {
                "regular"
            }
        );
    }

    println!("\nGLASS");
    for surface in Surface::ALL {
        let glass = surface.glass();
        println!(
            "  {:<10} depth {:>5.1}  frost {:>4.2}  gloss {:>4.2}  curve {:>4.2}",
            surface.name(),
            glass.depth * lxb_toolkit::metrics::scale_for(height),
            glass.frost,
            glass.gloss,
            glass.curve
        );
    }

    println!("\nMOTION");
    for (name, seconds) in lxb_toolkit::motion_durations() {
        println!("  {name:<20} {:>5.0}ms", seconds * 1000.0);
    }
    print!("  {:<20} ", "ease");
    for step in 0..=20 {
        let t = step as f32 / 20.0;

        let bar = (motion::ease(t) * 8.0).round() as usize;
        print!("{}", ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█", "█"][bar]);
    }
    println!();

    println!("\nSOUND");
    for sound in Sound::ALL {
        println!(
            "  {:<22} {:<12} {}",
            format!("{:?}", sound),
            format!("{:?}", sound.voice()),
            sound.name()
        );
    }

    println!("\nASSETS");
    println!(
        "  {} marks, {} recordings, 2 faces",
        lxb_toolkit::assets::GLYPHS.len(),
        lxb_toolkit::assets::SOUNDS.len()
    );

    println!("\nA PALETTE CHANGING (Purple to Blue, a frame at a time)");
    let mut accent = Accent::new("Purple").expect("Purple exists");
    accent.preview("Blue");
    let mut frame = 0;
    loop {
        if frame % 4 == 0 {
            println!(
                "  frame {frame:>2}   accent {}   glass {}   sky {}",
                accent.color(Role::Accent).srgb().hex(),
                accent.color(Role::Glass).srgb().hex(),
                accent.color(Role::SkyTop).srgb().hex(),
            );
        }
        if !accent.advance(1.0 / 60.0) {
            break;
        }
        frame += 1;
    }
    println!(
        "  settled after {frame} frames ({:.0}ms), applied is still {}",
        duration::ACCENT_CHANGE * 1000.0,
        accent.applied().name
    );
}
