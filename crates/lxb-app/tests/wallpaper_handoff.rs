use lxb_app::{monotonic_now_ns, WallpaperClock};
use lxb_toolkit::handoff::{self, Handoff};

#[test]
fn the_environment_hands_the_phase_over_once_and_then_says_nothing() {
    let Ok(boot_id) = handoff::boot_id() else {
        eprintln!("no boot id: the wallpaper handoff was not checked");
        return;
    };
    let Ok(now_ns) = monotonic_now_ns() else {
        eprintln!("no monotonic clock: the wallpaper handoff was not checked");
        return;
    };
    let theme = lxb_toolkit::settings::ShellTheme::load();
    let record = Handoff::of(
        boot_id,
        now_ns,
        42_000_000_000,
        theme.accent.name,
        None,
        None,
    )
    .expect("a canonical accent and this machine's boot id")
    .encode();

    std::env::set_var(handoff::ENV, &record);
    let clock = WallpaperClock::from_environment(theme.accent.name)
        .expect("a record written a moment ago on this boot");
    let carried = clock.elapsed_secs();
    assert!(carried >= 42.0, "{carried}");
    assert!(carried < 43.0, "{carried}");

    assert_eq!(
        std::env::var_os(handoff::ENV),
        None,
        "a one-shot record must not survive to be inherited by a child"
    );
    assert!(WallpaperClock::from_environment(theme.accent.name).is_none());

    std::env::set_var(handoff::ENV, "v=1;visual=lxb-wallpaper-v1");
    assert!(WallpaperClock::from_environment(theme.accent.name).is_none());
    assert_eq!(
        std::env::var_os(handoff::ENV),
        None,
        "a record refused is still a record consumed"
    );
}
