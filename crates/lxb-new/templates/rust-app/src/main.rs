fn main() -> Result<(), String> {
    let mut activated = 0;

    lxb_app::App::new("{{APP_ID}}", "{{TITLE}}").run(move |page| {
        page.head("launch", "{{TITLE}}");
        page.text(
            "A standard Wayland window, drawn with the LineXinBar design \
             language. Move with the arrow keys, a stick or a D-pad.",
        );
        page.gap();

        if page.button("Activate") {
            activated += 1;
            println!("Activated {activated} times");
        }
        if page.button("Leave") {
            page.quit();
        }
    })
}
