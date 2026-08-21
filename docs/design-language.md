# The design language

This is the language itself rather than the API: what the material is, how
things move, what a colour means, and why each of those is what it is. The core
library hands a renderer the exact vocabulary; the starter applies it to a
normal desktop application. Read this once and most of either API stops needing
to be looked up.

It was built for a particular kind of interface — one driven from a controller,
read from across a room, running on a machine that might have no desktop
installed on it at all — and nearly every decision below follows from one of
those three.

---

## Colour

### Roles, not colours

Nothing in this language is coloured by picking a colour. It is coloured by
naming what the thing is **for**, and letting the palette answer:

| role | what it is |
| --- | --- |
| `accent` | the colour of being chosen: selections, focused rims, the bloom behind whatever the cursor is on |
| `accent-soft` | a lighter cast of it, for the lit edge of a selected pane — glass catches light brighter than its own tint |
| `accent-deep` | deep enough to sit *under* content as a fill |
| `glass` | the neutral tint of a pane: what the background behind it is stained with. Nearly black, so labels stay legible over anything |
| `glass-raised` | the tint of a control sitting on a pane — lighter, because glass over glass reads as frostier, not darker |
| `rim` | the specular highlight along a lit edge |
| `text`, `text-soft` | what is written, and what is written quietly |
| `danger` | the one warning colour, for choices that cannot be taken back |
| `glow` | the soft lift behind whatever the interface is anchored on |
| `sky-*` | the background's own gradient, as two pairs the mood drifts between |

Five palettes answer all fourteen. There is no fifteenth role, and adding one
is a decision about the language rather than about the screen you are on: the
whole value of a role vocabulary is that an application drawn in it cannot find
itself without a colour under one accent that it had under another.

The background is in the palette rather than baked into a wallpaper because the
accent is drawn *in* the sky as well as on the glass, and a sky that no longer
belongs to the accent in front of it reads as two themes fighting rather than
as one.

### Two forms of a colour

sRGB hex is how a colour is **authored**: what a swatch shows, what goes in a
config file, what a stylesheet prints. Linear light is how a colour is
**drawn**. They are the same colour, and the conversion is not optional:

* A GPU surface declared sRGB interprets what a shader writes as light. Hand it
  the hex and everything comes out about twice as bright as it was picked.
* Blending must happen in the light. Mixing two eight-bit sRGB integers passes
  through colours that are in neither of them, which on a whole-interface
  transition reads as the palette going muddy in the middle.

Middle grey is the number to remember: `#808080` is 0.216 of the light, not
half.

### Changing palette is a movement

A whole interface changing colour at once is one of the few moments a design
system is visibly a system, and it only reads that way if every surface arrives
together. So the transition holds **one blend**, in linear light, and everything
on screen reads that one blend — the glass cannot arrive in the new accent while
the background is still in the old one.

It also draws a distinction the setting needs: the palette **shown** and the
palette **applied** are not the same thing. Walking a list of colours should
show each one on the whole interface as the cursor passes it — that is the only
honest preview of a colour that stains everything — but none of them is chosen
until a press chooses it, and leaving without pressing flows back rather than
snapping. Retargeting mid-flight carries on from the blend on screen, so
somebody walking the list quickly sees one colour flowing through several rather
than a series of jumps.

### Colour and material are separate choices

Accent answers what colour the interface is. Material answers how much work is
done to draw it. LineXinBar keeps the two independent, and splits material once
more so a user can choose the cost and look of the wallpaper separately from
the marks on top of it:

| part | choices |
| --- | --- |
| wallpaper | `Default`, `Simple`, `Custom wallpaper` |
| icons | `Default`, `Simple` |

`Default` is the full material: the moving water behind the shell and a bead of
water built from each mark's shape. `Simple` keeps the same composition and the
same drawings, but uses the glass-silk background and flat marks. Custom
wallpaper is a file, not a third material, and therefore cannot be an icon
choice.

There is no transition between materials. A colour halfway between two colours
is still a colour; a glyph halfway between a flat shape and a water bead is an
unreadable compromise. Preview lands immediately, and leaving restores the
applied choice immediately.

---

## The glass

A pane is a **real slab**: flat on top, its rim rounded over, its underside
level, floating a little way above whatever it is laid on. Every number
describes that object, and the picture falls out of tracing a ray through it.
That is the difference between glass and a rectangle with a bright edge painted
on, and it is not a subtle one — a painted edge cannot bend what is behind the
pane, cannot split it into colour at the corners, and cannot brighten where the
bevel squeezes a wide band of background into a narrow one.

Four properties describe any pane, and all four are properties of the material
rather than of the drawing:

* **depth** — how thick the slab is, which is also how wide its rim is rounded
  over. One number, as on a real edge.
* **frost** — how much of what it transmits it scatters.
* **gloss** — how strongly it takes the light.
* **curve** — how much its face bows.

Three cuts, and each is the answer to a different job:

| | depth | frost | gloss | curve | |
| --- | --- | --- | --- | --- | --- |
| **panel** | 22 | 0.95 | 1.00 | 0 | the dense compact slab used by the Power question and opaque wells. Current context menus and general dialogs use the clearer layered overlay below |
| **control** | 9 | 0.20 | 1.00 | 0 | a row, a button, a switch. A lozenge cut from a thinner sheet, sitting on something already legible, so it can afford to be nearly clear — and it takes the light fully, because it is a thing you can act on |
| **sidebar** | 15 | 0.46 | 0.66 | 1 | the broad cut beneath the guide sidebar, context menus, and general dialogs. The dense panel recipe at this size turns almost all of its face into one flat field, so this cut is shallower and clearer |

The current context menu and general dialog are deliberately one material:
`Overlay::ContextMenu.material()` and `Overlay::Dialog.material()` both return
`MENU_DIALOG_MATERIAL`. Two quiet lights sit under one Sidebar pane — an
accent-soft header at 0.075 and an accent foot at 0.04 — the pane is stained
with Glass at 0.38, and an accent-soft hairline at 0.10 restores its edge. A
dialog is wider, centred, and dims the scene further; those are layout and
staging differences, never permission to substitute the dense `Surface::Panel`
cut. That cut belongs to the compact Power question.

A surface that is only somewhere to put things, rather than something to act on,
takes gloss 0.45. Light belongs to what you can press.

### Coherent light

Panes and the background share one key light — up, to the left, and towards the
viewer — so every rim highlight across those large surfaces belongs to the same
room. Marks use a second fixed lamp vector tuned for their much smaller
signed-distance field. It points in the same up-left direction, but it is not
numerically the glass key light. Keep each light consistent inside its material
domain rather than quietly relighting individual objects.

### What the shader does

`assets/shaders/lxb_glass.wgsl` is the model, ready to paste into your own
shader. It declares no bindings, no entry points and no uniforms, so it composes
with any pipeline; what it cannot know is where your background comes from, so
the three refracted samples are arguments. You take the offsets it works out,
sample your own backdrop at them, and hand the colours back.

Inside it: a quarter-round bevel that stands vertical at the lip and lies flat
across the face (which is why the middle of a pane shows what is behind it
undistorted and only its edges do anything); a refraction through the slab and
out of its underside, asked three times at three slightly different wavelengths,
which is what splits white light into colour at a steep edge; a caustic measured
off how fast the sample runs away from the pixel, which is the thin bright line
along the edge of anything thick and clear; a Schlick term, so the surface turns
to mirror as the line of sight lies down along it — which around a rounded rim
happens by itself, all the way round, in exactly the proportion the corner
curves.

A stylesheet cannot have any of this. `css::stylesheet` emits a blur and a lit
edge under names that say so, which is the honest approximation.

### When there is no renderer

Three of the things in this language ship as WGSL — the glass above, the
analytic wallpaper, and the bead of water a mark is shaded into — and running
WGSL needs a renderer. A program drawing with cairo, or with Qt's painter, or
into a buffer of its own, has none, and what it falls back to is the rectangle
with a bright edge painted on that this whole section is written against.

So the same three are also in `paint`, as plain arithmetic over a buffer of
pixels: the same constants, the same terms, in the same order. `paint::glass`
lays one pane over what a surface already holds — which is both what it
refracts and where it lands, so a caller draws back to front —
`paint::wallpaper` fills a surface with the scene, `paint::glyph` shades a mark
out of the distance field `glyph_material` measured of it, and `paint::light`
is the soft gaussian lift that goes under a pane and behind whatever is being
aimed at. All four are in C as `lxb_paint_*` and in Python as `lxb_toolkit.paint`.

It is not free — the scene is thirty-odd transcendental functions per pixel —
so each of them draws on every core the machine has, and a program that wants
the scene cheaply should fill a smaller surface and scale that up with its own
painter. Every term in the scene is broad enough to survive that; the glass and
the type are drawn at the real size. `examples/c` and `examples/python` are the
same seven pages as `examples/rust`, drawn this way rather than on a GPU.

### Standing on the background

A background is a thing the eye is meant to be able to ignore, and the band of
water drawn sharp is not one: it carries a bright crest right across the
display, and a label crossing that crest is a label with a moving highlight
through it. The scene already has the number for this — `soften` widens every
sheet, flattens the gloss on it and takes the whole picture down — so the
answer is a value of it rather than a blur laid over afterwards.
`wallpaper::SOFTEN` is that value, and an interface standing on the scene draws
it with that.

The same question again, harder, when a panel opens — and the answer is not a
blur. Three things happen at once, and it takes all three:

1. **The page steps back.** It scales down by `menu::DEPTH` *towards the control
   the panel is about*, so the one thing the panel is a note on does not move
   while everything around it draws away from it. Shrinking towards the middle
   of the display instead slides that control out from under the very panel
   growing out of it, which reads as the page sliding rather than as the page
   receding. It is the mirror of the lean this language gives something opening
   *out* of a control.
2. **The page dims**, to `menu::DIM` — through each pane's own opacity, not its
   colour: on glass the alpha in a tint is how strongly the pane *stains* what
   is behind it, and much of what makes it visible is added afterwards.
3. **The page gives up the words the panel covers.** Not because the panel
   would fail to cover them — it is glass, and glass reproduces what is behind
   it, so words left underneath come back through the pane blurred and
   swimming. The run is *cut* rather than dropped: the pieces on either side of
   the panel keep their own place, so every glyph stays where it was and only
   the scissor differs. And it happens on **how solid the panel is** —
   `Menu::shown` — rather than on how far out it is. Nothing is hidden before
   the thing hiding it can be seen, and nothing stays hidden once it has gone.
   `Ui::recede_behind` does all three; the C and Python greetings do the same
   three by hand, because an immediate painter has to know before it draws.

Dimming alone leaves a dark, sharp page, which is a dark page rather than a
distant one.

A panel arrives faster than it leaves, and that is the one place its two
directions differ. Out of the control the glass is all there by `menu::PANEL_IN`
— a quarter of the flight — so that what grows out of it reads as a pane rather
than as a rectangle being inflated. Back into the control it gives up its colour
over the whole fold, and the page has its words again at exactly that rate.

Both halves of that were learnt from the same defect. A panel that held full
colour for three quarters of its fold and then went out over three frames did
not fade, it blinked; and the frame it blinked in was the frame the page
underneath got back every word the panel had taken — because no panel folds away
to nothing, and one narrower than its control does not shrink at all.

### The panel is laid out once

A context menu **settles beside the control it is about** — never over it. The
anchor is the whole reason the menu is where it is, so covering it with the
answer throws away the only context the panel has. To the right by preference,
because this language puts what can be selected on the left of a display, and
flipped to the other side when the panel would run off the edge.

Then it is measured: `menu::Layout` takes the rows, the header and the anchor
and answers where the panel stands, where each row sits, where its face ends
and its button begins, where the rules between its bands go, and how far the
selected row has opened out. One place answers all of it, because two places
computing one rectangle is the light landing on one row and the press on
another.

And it is **laid out once, at the size it settles at, and then flown out of its
anchor as one whole shape** — both the glass and everything standing on it on
the very same factor. A panel whose rows re-spaced themselves as it grew would
read as a list being typeset rather than as a note opening out of a control.
The glass arrives over `menu::PANEL_IN` of that flight and the commands over
the rest from `menu::CONTENT_IN`, because rows do not read at the size of an
icon.

---

## Motion

Two kinds of movement, and the difference between them is the whole vocabulary:

* Something running **on its own clock** — a fade, a panel growing out of the
  control that raised it, a screen arriving — is a duration and an easing.
* Something **chasing a target that can move under it** — a card following a
  scroll — is a critically damped spring, because a target that moves
  mid-flight has to be picked up rather than restarted.

Four rules hold over both. None is a stylistic preference; each was learned by
shipping the opposite and having it rejected on sight.

1. **Never linear.** A ramp straight off its clock is what makes an animation
   read as mechanical. A pure ease-out — the usual choice for something a button
   summoned — still leaves at full speed, and the departure is the half you
   watch.
2. **Nothing disappears before its transition has finished.** The outgoing thing
   stays behind the incoming one and only then fades. A list that deletes the
   old row on the frame the new one is chosen has lost the half of the movement
   that says which way it went.
3. **A selection frame never springs and never jumps.** It is already where it
   is going while the content scrolls underneath it. What moves outside it is a
   slow pulse of light, not the frame.
4. **Every screen animates, always.** A transition that plays on the screen in
   front of the user and lands instantly on the other one is a bug, not a
   saving.

And one rule about the shell-owned application launch, which is where most of
these show up at once: it must animate **immediately**, with loading happening
inside the animation and a visible loading state — never a delayed pop-in once
the application is ready. The shell owns that transition; an ordinary
application should map and draw promptly, not copy the private launch protocol.
The animation is the acknowledgement, not the decoration.

The durations are named rather than numbered, because the number is not the
point — what a duration means is which of them it *is*. Two things that happen
for the same reason take the same time, and that is most of what makes a set of
animations read as one interface rather than as a pile of tuned constants.

---

## Sizes, shapes and type

Everything is written against a **1080-pixel-tall screen** and scaled by height.
Height, never width: an interface laid out against width changes shape when a
display gets wider, while one laid out against height keeps its proportions and
simply has more room beside it — which is what a screen you sit back from has to
do between 16:9 and 21:9. The multiplier is clamped to `0.6..=2.5`, as it is in
the shell, so an unusually short or tall surface stays usable instead of
shrinking into dust or growing past its own display.

There are no points, no ems and no device-independent anything. A console
interface is read from across a room, and the only honest unit for that is a
fraction of the screen it is on.

**Controls are capsules.** A row, a button, a switch, a pill — its radius is
half its own height, so it is never a number anybody has to pick. What is left
to name is the card, which follows the shape of the window inside it, and the
panel, which is the largest shape drawn and carries the largest radius.

**A control is three things, not one.** A button, a list row, a switch and a
dialog's answer are the same object here, and it is not a rectangle filled with
a colour that changes when you are on it. It is drawn in this order, and the
order is the whole effect:

1. **A halo** under it, in the accent, breathing on the pulse. It has no edge of
   its own; it is what gives the control something to sit on. Its width is
   `1.24` of the *surface* the controls sit on rather than of the control, so a
   taller row is not read as more selected than a shorter one.
2. **The lit capsule** — a slab of the control's own glass taking the light
   fully, stained with the accent at `0.46` (`0.50` for a dialog's answer),
   plus `0.05` of the pulse. This is the selection, and it *glides* from
   wherever it was, so it is drawn once for a whole set of controls rather than
   by each of them.
3. **The chip** — the control's own face: the same glass, stained with
   `glass-raised` at `0.10` and taking the light quietly at gloss `0.45`. Drawn
   *over* the capsule and faded out by how much of the light has arrived, so
   the lit control is the one place the capsule shows through.

Filling the chosen control with the accent instead gets the same still frame
and none of the movement: the light cannot glide onto a control whose colour is
a property of the control, and the moment the selection changes there is a hole
in the column where the outgoing chip used to be. A control that cannot be
chosen is outlined in `text-soft` at `0.22` rather than given a dimmer chip —
every chip is glass over something dark, so a dimmer one is a chip in slightly
less light, which is also what an unlit one looks like.

Its label is one label in two weights: bold and full white under the light,
regular at `0.84` beside it. Quieter must still be legible over glass with a
wallpaper coming through it.

**A press is a journey, not a state.** The control sinks by `0.14` of itself
over the first `0.3` of the press, comes back past its own size by `0.05`, and
settles — over `guide-press`, whatever the key does in the meantime. Letting go
early does not cut it short, and holding the key down does not hold the control
at the bottom: that is what makes a quick tap read as a press at all rather
than as a flicker. Everything the control is made of goes down with it; a label
that stayed put while its chip sank reads as a hole opening behind it. Where a
press also changes a state, the colour arrives with the spring back rather than
with the dip, because a switch is thrown on the way up.

**Tiles are squircles, not circles.** The corner has its bend spread along the
edges instead of stopping dead. On a shape whose radius is its whole half-width,
that difference is the whole difference between a disc and a rounded square.

Type is two faces and five sizes. Bold is for the thing being named — a heading,
a title — and normal for everything else. Nothing is italic: at this size and
this distance, on a dark ground, italic loses more legibility than it gains
emphasis. One line height for the whole scale, because leading that changed per
size would make two paragraphs at two sizes read as two typographies.

Text over scenery carries a three-ring halo rather than a rectangular backing.
Its reach is `0.24 * font size`; the rings sit at `0.34`, `0.67`, and `1.0` of
that reach with per-copy opacities `0.20`, `0.13`, and `0.075`. Copies are no
more than 1.3 pixels apart and every ring has at least eight, so captions do not
turn into polygons. Layout separately names left/centre/right alignment and
wrap/ellipsis/clip overflow; clipping hides pixels without reshaping the run.

---

## Marks

Ninety-eight drawings, each authored as a **shape rather than a finished
picture**. The SVG is a neutral white silhouette carrying the `lxb:shape`
marker. At load time it becomes a signed-distance field; at draw time the glyph
shader makes either the Default water bead or the Simple flat mark from that
same field. In Default, an opening is just another edge, so its displaced rim
comes from the same calculation as the outside instead of from a second
hand-painted gradient.

The distance field spans one quarter of its 128-pixel cell (`0.125` either side
of the encoded midpoint) and is measured at four times the cell resolution
before reduction. The shader uses the field's gradient for the rounded wall, a
fixed up-left lamp for every mark, and a tight contact shadow. It does not
refract: an application may draw many marks over a pane produced moments ago,
and sampling an older unrelated backdrop into their edges would be worse than
reflecting a self-contained room.

Drawings use paths, never `<text>`: a renderer built without SVG text support is
a perfectly ordinary one, and a glyph that needed a font would come out empty.
Most use a 32-unit box; compact marks may use 24. Both materials use the same
signed-distance field and `lxb_glyph.wgsl`: Default selects the water-bead
branch, while Simple selects its flat tint/alpha/stain branch. Rasterising the
SVG directly is only a neutral source preview, not the shell's Simple material.

Two things this set does not do. It never copies anybody else's icon: the
material language is the thing to take, and the shape is drawn from scratch.
And it does not bundle application icons — each application owns and installs
its icon under its desktop ID for standard icon-theme lookup, because that icon
belongs to the application rather than to the interface.

---

## Sound

An interface driven from a controller answers a press twice: something moves,
and it clicks. The click is not decoration — it is the half of the
acknowledgement that survives the user looking somewhere else on the screen, and
on a list that eases rather than snaps it arrives on the press while the
highlight is still travelling.

* **A screen with a voice of its own does not borrow another's.** Each
  full-screen surface answers a move and a press in its own pair, so somebody
  who has looked away can hear which screen they are driving.
* **A move sounds by where it landed, not by what moved it.** A click that puts
  the selection on a row is the same move as the direction that would have
  walked there. Hovering is not a move: a swept mouse must not be a stream of
  clicks.
* **A press that starts nothing stays silent**, and so does a move that landed
  nowhere.
* **A component keeps its own voice wherever it is raised.** A menu, a dialog, a
  mixer sound the same whichever screen opened them.
* **No clip is laid on top of a copy of itself.** Identical samples add in
  phase, and one wheel event can be worth several rows — a spun wheel takes the
  output to full scale. Sixty milliseconds is the shortest gap; nearer than
  that, drop the request.

The toolkit carries fourteen recordings because bundled compatibility assets
must remain available to existing applications. The current shell uses eleven
short effects plus its background music. `trash.ogg` and `error.ogg` are still
bundled, but are not current shell actions; do not infer behaviour merely from
their presence. Three of the effects answer something that *happened* rather
than a control that was pressed — a notification, a completed screenshot, and
an authorisation panel going up. The music is the one recording that loops and
the only one that can be turned off outright: the effects answer an action or
event, and an action answered by nothing feels dead.

---

## What the user pressed

An interface in this language is driven from a keyboard, a controller and a
pointer at once, and the whole of the point is that they are one interface
rather than three. What decides that is not the device. It is one table, in
`lxb_toolkit::input`, which nothing here reads a device to consult.

* **A direction is a direction.** An arrow key, a D-pad and a thumbstick send
  the same action, and the code that acts on it cannot tell which sent it.
* **The middle of a held direction is invented, not taken.** Wayland gives a
  client one press and one release; a pad has no repeat at all. So the pace is
  decided here — 350 ms before a held direction starts stepping, then one step
  every 90 ms — and it is the same pace for the keyboard and the pad, because a
  held arrow that walked a list faster than a held D-pad would be two
  interfaces.
* **A stick engages and releases at different distances**, 0.55 and 0.35. One
  threshold would make a stick resting a hair either side of it chatter, and a
  worn stick that never quite centres would do it untouched.
* **A press acts on the way down.** A key, a face button and a mouse button
  alike: a row activated on the release fires twice on a chord.
* **The thing under the pointer is the thing the buttons act on.** Pointing at
  something inside a panel selects it — silently, because a swept pointer must
  not be a stream of clicks — so that a menu raised with the right button is
  about what is being pointed at rather than about whatever was selected
  already. A list moves in two steps instead: the first click carries the
  selection there and the second acts, or a folder opens under a pointer that
  was only crossing the screen.
* **A part-turned wheel is carried rather than lost.** A touchpad reports
  fractions of a notch, and rounding each of them away is a list that never
  moves under a slow drag.
* **The guide button is the shell's, always.** It is how somebody gets back out
  of an application, so an application that could act on it could swallow the
  way out of itself. The left-hand face button is left free for the
  application; every other control means what it means everywhere.

Two of the shell's own bindings are deliberately not an application's, and both
differences are the same difference. The shell has to keep a way out of a game
that the game cannot take, so it gives the Menu key to the guide and Tab to the
next display. An application has nothing to keep a way out of: the key with a
menu printed on it opens the menu, and Tab moves within the application.

---

## The application boundary

An LXB application is an ordinary Wayland application with an ordinary desktop
entry. The compositor's `lxb_shell_v1` protocol is private coordination between
the compositor and its shell; it is not an application SDK and the toolkit does
not expose it.

The renderer-neutral core remains usable from Rust, C and Python. The Rust
starter adds the practical application path: a window, focus and activation,
the LXB layout/material vocabulary, and a desktop entry whose filename,
application id and startup class agree. That separation keeps the portable
tokens portable without making a new application start from a contact sheet.
