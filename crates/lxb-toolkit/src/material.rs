#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glass {
    pub depth: f32,

    pub frost: f32,

    pub gloss: f32,

    pub curve: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Surface {
    Panel,

    Control,

    Sidebar,
}

impl Surface {
    pub const ALL: [Surface; 3] = [Surface::Panel, Surface::Control, Surface::Sidebar];

    pub const fn name(self) -> &'static str {
        match self {
            Surface::Panel => "panel",
            Surface::Control => "control",
            Surface::Sidebar => "sidebar",
        }
    }

    pub const fn glass(self) -> Glass {
        match self {
            Surface::Panel => Glass {
                depth: 22.0,
                frost: 0.95,
                gloss: 1.0,
                curve: 0.0,
            },
            Surface::Control => Glass {
                depth: 9.0,
                frost: 0.2,
                gloss: 1.0,
                curve: 0.0,
            },
            Surface::Sidebar => Glass {
                depth: 15.0,
                frost: 0.46,
                gloss: 0.66,
                curve: 1.0,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Overlay {
    ContextMenu,
    Dialog,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverlayMaterial {
    pub radius: f32,
    pub glass: Glass,

    pub stain_role: crate::palette::Role,
    pub stain: f32,

    pub light_inset: f32,

    pub header_x: f32,
    pub header_width: f32,

    pub header_height: f32,
    pub header_height_share: f32,

    pub header_role: crate::palette::Role,
    pub header_light: f32,

    pub foot_x: f32,
    pub foot_width: f32,

    pub foot_height: f32,
    pub foot_height_share: f32,

    pub foot_role: crate::palette::Role,
    pub foot_light: f32,

    pub rim_role: crate::palette::Role,
    pub rim: f32,

    pub rim_width: f32,
}

pub const MENU_DIALOG_MATERIAL: OverlayMaterial = OverlayMaterial {
    radius: crate::metrics::Metric::PanelRadius.value(),
    glass: Surface::Sidebar.glass(),
    stain_role: crate::palette::Role::Glass,
    stain: light::SIDEBAR_STAIN,
    light_inset: 2.0,
    header_x: 0.06,
    header_width: 0.88,
    header_height: 300.0,
    header_height_share: 0.36,
    header_role: crate::palette::Role::AccentSoft,
    header_light: light::SIDEBAR_HEADER,
    foot_x: 0.10,
    foot_width: 0.80,
    foot_height: 360.0,
    foot_height_share: 0.38,
    foot_role: crate::palette::Role::Accent,
    foot_light: light::SIDEBAR_FOOT,
    rim_role: crate::palette::Role::AccentSoft,
    rim: light::SIDEBAR_RIM,
    rim_width: 1.0,
};

impl Overlay {
    pub const ALL: [Overlay; 2] = [Overlay::ContextMenu, Overlay::Dialog];

    pub const fn material(self) -> OverlayMaterial {
        match self {
            Self::ContextMenu | Self::Dialog => MENU_DIALOG_MATERIAL,
        }
    }
}

pub const GLOSS_QUIET: f32 = 0.45;

pub mod optics {

    pub const KEY_LIGHT: [f32; 3] = [-0.42, -0.66, 0.62];

    pub const IOR: f32 = 1.47;

    pub const DISPERSION: f32 = 0.055;

    pub const FLOAT: f32 = 1.0;

    pub const FROST_SCATTER: [f32; 3] = [0.030, 0.029, 0.046];

    pub const MAX_SLAB_SHARE: f32 = 0.44;

    pub const BLUR_LEVELS: f32 = 4.0;
}

pub mod light {

    pub const SIDEBAR_STAIN: f32 = 0.38;

    pub const SIDEBAR_HEADER: f32 = 0.075;
    pub const SIDEBAR_FOOT: f32 = 0.04;
    pub const SIDEBAR_RIM: f32 = 0.10;

    pub const HALO_RINGS: usize = 5;
    pub const HALO_STEP: f32 = 3.0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_cuts_are_the_three_the_prose_describes() {
        let panel = Surface::Panel.glass();
        let control = Surface::Control.glass();
        let sidebar = Surface::Sidebar.glass();

        assert!(panel.depth > control.depth, "a control is a thinner sheet");
        assert!(
            panel.frost > control.frost,
            "a panel carries text over anything"
        );
        assert!(
            sidebar.frost < panel.frost && sidebar.depth < panel.depth,
            "a large sheet is cut shallower and clearer"
        );
        assert_eq!(control.curve, 0.0, "a compact pane stays level");
        assert!(sidebar.curve > 0.0, "a broad sheet bows");
        assert!(GLOSS_QUIET < control.gloss);
    }

    #[test]
    fn the_dialog_is_cut_from_the_context_menus_material() {
        let menu = Overlay::ContextMenu.material();
        let dialog = Overlay::Dialog.material();
        assert_eq!(dialog, menu);
        assert_eq!(dialog.glass, Surface::Sidebar.glass());
        assert_eq!(dialog.radius, 30.0);
        assert_eq!(dialog.stain_role, crate::palette::Role::Glass);
        assert_eq!(dialog.stain, light::SIDEBAR_STAIN);
        assert_eq!(dialog.light_inset, 2.0);
        assert_eq!((dialog.header_x, dialog.header_width), (0.06, 0.88));
        assert_eq!(
            (dialog.header_height, dialog.header_height_share),
            (300.0, 0.36)
        );
        assert_eq!(dialog.header_role, crate::palette::Role::AccentSoft);
        assert_eq!(dialog.header_light, light::SIDEBAR_HEADER);
        assert_eq!((dialog.foot_x, dialog.foot_width), (0.10, 0.80));
        assert_eq!(
            (dialog.foot_height, dialog.foot_height_share),
            (360.0, 0.38)
        );
        assert_eq!(dialog.foot_role, crate::palette::Role::Accent);
        assert_eq!(dialog.foot_light, light::SIDEBAR_FOOT);
        assert_eq!(dialog.rim_role, crate::palette::Role::AccentSoft);
        assert_eq!(dialog.rim, light::SIDEBAR_RIM);
        assert_eq!(dialog.rim_width, 1.0);
    }

    #[test]
    fn every_pane_is_a_pane_that_could_exist() {
        for surface in Surface::ALL {
            let glass = surface.glass();
            assert!(glass.depth > 0.0, "{}", surface.name());
            assert!((0.0..=1.0).contains(&glass.frost), "{}", surface.name());
            assert!((0.0..=1.0).contains(&glass.gloss), "{}", surface.name());
            assert!((0.0..=1.0).contains(&glass.curve), "{}", surface.name());
        }
    }

    #[test]
    fn there_is_one_lamp_and_it_is_above() {
        let [x, y, z] = optics::KEY_LIGHT;
        assert!(x < 0.0, "the lamp is to the left");
        assert!(y < 0.0, "and above, in a y-down interface");
        assert!(z > 0.0, "and in front");
        let length = (x * x + y * y + z * z).sqrt();
        assert!(
            (length - 1.0).abs() < 0.01,
            "not a unit direction: {length}"
        );
    }

    const _: () = {
        assert!(optics::IOR > 1.4 && optics::IOR < 1.6);
        assert!(optics::DISPERSION > 0.0);
        assert!(optics::MAX_SLAB_SHARE < 0.5, "a pane is not all bevel");
    };
}
