pub fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let back = -2.0 * t + 2.0;
        1.0 - back * back * back / 2.0
    }
}

pub fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn spring(position: f64, velocity: f64, target: f64, rate: f64, dt: f64) -> (f64, f64) {
    let dt = dt.clamp(0.0, 0.1);
    let offset = position - target;

    let c = velocity + rate * offset;
    let decay = (-rate * dt).exp();
    (
        target + (offset + c * dt) * decay,
        (velocity - c * rate * dt) * decay,
    )
}

pub fn approach(value: f32, target: f32, step: f32) -> f32 {
    if value < target {
        (value + step).min(target)
    } else {
        (value - step).max(target)
    }
}

pub const CARD_SPRING: f64 = 19.0;

pub const HIGHLIGHT_SPRING: f64 = 21.0;

pub const PRESS_DIP: f32 = 0.14;
pub const PRESS_BOUNCE: f32 = 0.05;

pub const PRESS_DOWN: f32 = 0.3;

pub fn press_scale(t: f32) -> f32 {
    if !(0.0..1.0).contains(&t) {
        return 1.0;
    }
    if t < PRESS_DOWN {
        return 1.0 - PRESS_DIP * ease(t / PRESS_DOWN);
    }
    let back = ease((t - PRESS_DOWN) / (1.0 - PRESS_DOWN));

    1.0 - PRESS_DIP * (1.0 - back) + PRESS_BOUNCE * (back.powi(3) * std::f32::consts::PI).sin()
}

pub fn fill_arrival(t: f32) -> f32 {
    ease((t - PRESS_DOWN * 0.6) / (1.0 - PRESS_DOWN * 0.6))
}

pub fn scaled_about_centre([x, y, w, h]: [f32; 4], scale: f32) -> [f32; 4] {
    [
        x + w * (1.0 - scale) * 0.5,
        y + h * (1.0 - scale) * 0.5,
        w * scale,
        h * scale,
    ]
}

pub fn pressed(rect: [f32; 4], through: Option<f32>) -> [f32; 4] {
    scaled_about_centre(rect, through.map_or(1.0, press_scale))
}

pub fn pulse(seconds: f32) -> f32 {
    0.5 + 0.5 * (seconds * std::f32::consts::TAU / duration::PULSE).sin()
}

pub mod duration {

    pub const FLIGHT: f32 = 0.300;

    pub const PANEL: f32 = 0.200;

    pub const LAUNCH_OPEN: f32 = 0.340;

    pub const LAUNCH_SETTLE: f32 = 0.120;

    pub const LAUNCH_HANDOVER: f32 = 0.300;

    pub const ARRIVAL: f32 = 0.300;

    pub const BLACK_HANDOVER: f32 = 0.100;

    pub const SIDEBAR_SLIDE: f32 = 0.280;

    pub const ENTRY_LEAD: f32 = 0.120;
    pub const ENTRY_STAGGER: f32 = 0.045;
    pub const ENTRY_SLIDE: f32 = 0.260;

    pub const ACCENT_CHANGE: f32 = 0.360;

    pub const SCENERY_FADE: f32 = 0.450;

    pub const COLOUR_FADE: f32 = 0.350;

    pub const PULSE: f32 = 1.800;

    pub const SPIN: f32 = 1.400;

    pub const BLACK_IN: f32 = 0.260;
    pub const BLACK_HOLD: f32 = 0.340;
    pub const BLACK_OUT: f32 = 0.420;

    pub const MENU_FLIGHT: f32 = 0.200;
    pub const MENU_UNFOLD: f32 = 0.320;
    pub const MENU_PRESS: f32 = 0.160;

    pub const GUIDE_PRESS: f32 = 0.340;
    pub const GUIDE_POWER_FLIGHT: f32 = 0.220;
    pub const GUIDE_MEDIA_FLIGHT: f32 = 0.340;
    pub const GUIDE_TRANSPORT_FLIGHT: f32 = 0.120;

    pub const NOTIFICATION_HOLD: f32 = 4.000;
    pub const NOTIFICATION_ENTER: f32 = 0.340;
    pub const NOTIFICATION_LEAVE: f32 = 0.220;
    pub const NOTIFICATION_BADGE: f32 = 0.200;

    pub const VOLUME_FILL: f32 = 0.140;
    pub const VOLUME_HOLD: f32 = 1.000;
    pub const VOLUME_LEAVE: f32 = 0.280;

    pub const KEYBOARD_SLIDE: f32 = 0.240;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easing_starts_still_ends_still_and_arrives() {
        assert_eq!(ease(0.0), 0.0);
        assert_eq!(ease(1.0), 1.0);
        assert!((ease(0.5) - 0.5).abs() < 1e-6);

        assert_eq!(ease(-1.0), 0.0);
        assert_eq!(ease(9.0), 1.0);
    }

    #[test]
    fn easing_is_never_linear() {
        for t in [0.05f32, 0.1, 0.9, 0.95] {
            assert!(
                (ease(t) - t).abs() > 0.01,
                "at {t} the curve is indistinguishable from a straight line"
            );
        }

        let middle = ease(0.55) - ease(0.45);
        assert!(middle > ease(0.1) - ease(0.0));
        assert!(middle > ease(1.0) - ease(0.9));
        assert_eq!(smoothstep(0.0), 0.0);
        assert_eq!(smoothstep(1.0), 1.0);
    }

    #[test]
    fn a_spring_arrives_without_crossing() {
        let (mut at, mut speed) = (0.0f64, 0.0f64);
        let mut fastest = 0.0f64;
        for _ in 0..200 {
            (at, speed) = spring(at, speed, 1.0, CARD_SPRING, 1.0 / 60.0);
            fastest = fastest.max(speed.abs());
            assert!(at <= 1.0, "it overshot to {at}");
        }
        assert!(at > 0.999, "it never arrived: {at}");
        assert!(fastest > 0.0, "it never moved");
    }

    #[test]
    fn a_stalled_frame_cannot_explode_a_spring() {
        let (at, speed) = spring(0.0, 0.0, 1.0, CARD_SPRING, 30.0);
        assert!((0.0..=1.0).contains(&at), "{at}");
        assert!(speed.is_finite());
    }

    #[test]
    fn approaching_lands_exactly() {
        assert_eq!(approach(0.0, 1.0, 0.3), 0.3);
        assert_eq!(approach(0.9, 1.0, 0.3), 1.0);
        assert_eq!(approach(1.0, 0.0, 0.3), 0.7);
        assert_eq!(approach(0.1, 0.0, 0.3), 0.0);
    }

    #[test]
    fn a_press_dips_bounces_and_settles_on_its_own_size() {
        assert_eq!(press_scale(0.0), 1.0);

        assert!((press_scale(PRESS_DOWN) - (1.0 - PRESS_DIP)).abs() < 1e-6);

        let over = (30..100)
            .map(|n| press_scale(n as f32 / 100.0))
            .fold(0.0f32, f32::max);
        assert!(over > 1.0, "it never went past its own size: {over}");
        assert!(
            over <= 1.0 + PRESS_BOUNCE + 1e-6,
            "it bounced too far: {over}"
        );

        assert!((press_scale(0.999) - 1.0).abs() < 1e-3);
        assert_eq!(press_scale(1.0), 1.0);
        assert_eq!(press_scale(-1.0), 1.0);
        assert_eq!(press_scale(4.0), 1.0);
    }

    #[test]
    fn a_switch_is_thrown_on_the_way_up() {
        assert_eq!(fill_arrival(0.0), 0.0);
        assert!(fill_arrival(PRESS_DOWN * 0.5) < 0.05);
        assert!(fill_arrival(PRESS_DOWN) > 0.0);
        assert!((fill_arrival(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_press_is_about_the_controls_own_centre() {
        let rect = [100.0, 40.0, 200.0, 60.0];
        assert_eq!(pressed(rect, None), rect);
        let sunk = pressed(rect, Some(PRESS_DOWN));
        let centre = |[x, y, w, h]: [f32; 4]| [x + w / 2.0, y + h / 2.0];
        let (was, now) = (centre(rect), centre(sunk));
        assert!((was[0] - now[0]).abs() < 1e-4 && (was[1] - now[1]).abs() < 1e-4);
        assert!(sunk[2] < rect[2] && sunk[3] < rect[3]);
    }

    #[test]
    fn the_light_breathes_over_its_own_period() {
        assert!((pulse(0.0) - 0.5).abs() < 1e-6);
        assert!((pulse(duration::PULSE) - 0.5).abs() < 1e-4);
        let lowest = (0..180)
            .map(|n| pulse(n as f32 * duration::PULSE / 180.0))
            .fold(1.0f32, f32::min);
        let highest = (0..180)
            .map(|n| pulse(n as f32 * duration::PULSE / 180.0))
            .fold(0.0f32, f32::max);
        assert!(lowest < 0.01 && highest > 0.99, "{lowest} to {highest}");
    }

    #[test]
    fn the_durations_agree_with_each_other() {
        use duration::*;
        for (name, seconds) in crate::motion_durations() {
            assert!(
                seconds > 0.0 && seconds <= NOTIFICATION_HOLD,
                "{name} is {seconds}s"
            );
        }
        assert_eq!(ARRIVAL, FLIGHT);
        assert_eq!(LAUNCH_HANDOVER, FLIGHT);
    }

    const _: () = {
        use duration::*;
        assert!(BLACK_HANDOVER < ARRIVAL / 2.0, "the join outlasts the join");
        assert!(
            BLACK_OUT > BLACK_IN,
            "a dip comes up slower than it goes down"
        );
        assert!(
            ENTRY_STAGGER < ENTRY_SLIDE,
            "the column would arrive as a queue"
        );
        assert!(
            NOTIFICATION_LEAVE < NOTIFICATION_ENTER,
            "a notification would linger while leaving"
        );
        assert!(
            VOLUME_FILL < VOLUME_LEAVE,
            "a key-raised control would be late to appear"
        );
    };
}
