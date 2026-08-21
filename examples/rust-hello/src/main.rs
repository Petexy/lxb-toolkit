//! Hello, world — the whole of an application in this language.
//!
//! A Wayland window carrying the shell's wallpaper and glass, driven by a
//! keyboard, a pointer and every controller plugged into the machine, and
//! answering with the shell's own sounds. Nothing below names a colour, a
//! radius, a duration or a device.
//!
//!     cargo run                       # the window
//!     cargo run -- --shot hello.png   # one frame of it, with no display

fn page(page: &mut lxb_app::Page, clicked: &mut usize) {
    page.head("launch", "Hello, world");
    page.text(match clicked {
        0 => "Press the button. Move with the arrow keys, a stick or a D-pad.",
        1 => "You clicked me!",
        _ => "You clicked me again.",
    });
    page.gap();

    if page.button("Hello World!") {
        *clicked += 1;
    }
    if page.button("Leave") {
        page.quit();
    }
}

fn main() -> Result<(), String> {
    let app = lxb_app::App::new("com.example.HelloWorld", "Hello World");
    let mut clicked = 0;
    match std::env::args().nth(2) {
        Some(path) => app.shot(path, 1280, 800, 10.0, |frame| page(frame, &mut clicked)),
        None => app.run(move |frame| page(frame, &mut clicked)),
    }
}
