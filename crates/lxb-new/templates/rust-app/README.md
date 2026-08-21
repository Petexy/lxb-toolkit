# {{TITLE}}

A small standard Wayland application built on `lxb-app`, which is the whole of
the LineXinBar design language with a window around it.

{{BODY}}

## Build and run

```sh
cargo run
```

The generated manifest asks for the toolkit version matching `lxb-new`. If you
generated this from an unpublished toolkit checkout, point it at that checkout
instead:

```sh
cargo run -p lxb-new -- --toolkit-path crates/lxb-toolkit {{SLUG}}
```

`lxb_app` re-exports the four crates underneath it: `lxb_app::lxb_toolkit` is
what the language answers, `lxb_app::lxb_render` draws it, `lxb_app::lxb_input`
reads the controls and `lxb_app::lxb_sound` answers them. Reach for them
directly when you outgrow the flow; you never have to add them to the manifest.

## One frame, drawn to a file

```sh
cargo run -- --shot page.png
```

`App::shot` draws a settled frame with no display at all, through the same page
function and the same renderer as the window. It is how an interface in this
language is checked without a screen — see the toolkit's own greetings.
