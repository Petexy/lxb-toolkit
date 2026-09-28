# What this is, and what was changed

`gilrs-core` 0.6.8 from crates.io (<https://gitlab.com/gilrs-project/gilrs>,
commit `07e286e24b046cf39e5c367daa2770b805a64692`), Apache-2.0 or MIT, carried
in this tree rather than depended on from the registry. `LICENSE-MIT` and
`LICENSE-APACHE` are upstream's, from that commit; the published crate does
not include them.

It is the same fork as LineXinBar's `third_party/lxb-gilrs-core` (shell commit
`c08e2cf`), and byte for byte the same everywhere but this note. Carried here
too because every application built on `lxb-input` reads controllers through
it, and those applications run under the same shell.

**The directory and the package are named `lxb-gilrs-core` so that nothing — a
lock file, `cargo tree`, a vendored source archive, a packager reading the spec
— can take this for the published crate. It is not.** The library it builds is
still `gilrs_core`.

What this copy does that 0.6.8 does not:

- **A hot-plug event is never left unread behind another.** On Linux, a thread
  watches udev and passes each joystick that arrives or goes over a channel,
  writing an eventfd each time to say so. The eventfd is registered
  edge-triggered, and `handle_hotplug` returns as soon as one message has made
  an event, leaving the rest of the channel alone. Two devices changing inside
  one poll were one edge, so the second waited for some later, unrelated
  hot-plug to be read — and if none came, for ever. `next_event_impl` now asks
  the channel first on every call, in `src/platform/linux/gamepad.rs`, marked
  `LineXinBar:`.

  Why it matters to an application: LineXinBar's pad guard takes each pad away
  and gives back a copy without its guide button, so turning a pad off removes
  two devices within a millisecond. The copy's removal was left unread; when
  the pad came back, GilRs opened the grabbed original, which is silent, and
  never the new copy — an application answered nothing on a controller that
  every other program could read. `lxb-input`'s
  `a_pad_and_its_copy_going_together_are_both_seen_to_go` is that failure,
  and passes only with this fix.

Upstream is not patched anywhere else. Anything under `src/` that is not named
above is 0.6.8 as published, and upstream's master has the same code, checked
at the commit above.
