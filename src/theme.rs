// Apparence de la fenêtre, d'après le design system « Display Switch » :
// un thème clair et neutre, une seule police (Ubuntu Light, intégrée à egui), pas d'icône.

use eframe::egui::{Color32, Context, FontId, Stroke, TextStyle, Theme, ThemePreference, Vec2};

// Couleurs.
pub const BG: Color32 = Color32::from_rgb(0xf8, 0xf8, 0xf8);
pub const SURFACE: Color32 = Color32::from_rgb(0xff, 0xff, 0xff);
pub const BORDER: Color32 = Color32::from_rgb(0xde, 0xde, 0xde);
pub const BORDER_CONTROL: Color32 = Color32::from_rgb(0x8c, 0x8c, 0x8c);
pub const INK: Color32 = Color32::from_rgb(0x1f, 0x1f, 0x1f);
pub const INK_MUTED: Color32 = Color32::from_rgb(0x5c, 0x5c, 0x5c);
const CONTROL: Color32 = Color32::from_rgb(0xff, 0xff, 0xff);
const CONTROL_HOVER: Color32 = Color32::from_rgb(0xf0, 0xf0, 0xf0);
const CONTROL_ACTIVE: Color32 = Color32::from_rgb(0xe4, 0xe4, 0xe4);
const ACCENT: Color32 = Color32::from_rgb(0x1a, 0x5f, 0xb4);
const ON_ACCENT: Color32 = Color32::from_rgb(0xff, 0xff, 0xff);
pub const SUCCESS: Color32 = Color32::from_rgb(0x1e, 0x7b, 0x34);

// Espacements, rayons et hauteurs, en points egui (ils suivent la mise à l'échelle de Windows).
pub const SPACE_1: f32 = 4.0;
pub const SPACE_2: f32 = 8.0;
pub const SPACE_3: f32 = 12.0;
pub const SPACE_4: f32 = 16.0;
pub const RADIUS_SM: u8 = 4;
pub const RADIUS_MD: u8 = 6;
pub const RADIUS_LG: u8 = 8;
pub const CONTROL_LG: f32 = 36.0;
pub const CONTROL_SM: f32 = 28.0;

// Tailles de texte qui n'ont pas de style egui dédié.
pub const MONITOR_NAME_SIZE: f32 = 14.0;
pub const BUTTON_LG_SIZE: f32 = 14.0;
pub const BUTTON_SIZE: f32 = 13.0;
pub const DOT_SIZE: f32 = 8.0;

// Appliquer le thème une fois, au démarrage : egui le réutilise ensuite à chaque image.
pub fn apply(ctx: &Context) {
    ctx.set_theme(ThemePreference::Light);
    ctx.style_mut_of(Theme::Light, |style| {
        let text_sizes = [
            (TextStyle::Heading, 18.0),
            (TextStyle::Body, 13.0),
            (TextStyle::Button, BUTTON_SIZE),
            (TextStyle::Small, 12.0),
        ];
        for (text_style, size) in text_sizes {
            style
                .text_styles
                .insert(text_style, FontId::proportional(size));
        }
        style.spacing.item_spacing = Vec2::new(SPACE_2, SPACE_1);
        style.spacing.button_padding = Vec2::new(SPACE_3, SPACE_1);

        let visuals = &mut style.visuals;
        visuals.panel_fill = BG;
        visuals.window_fill = SURFACE;
        visuals.window_stroke = Stroke::new(1.0, BORDER);
        visuals.selection.bg_fill = ACCENT;
        visuals.selection.stroke = Stroke::new(1.0, ON_ACCENT);

        // Texte courant en gris, séparateurs clairs.
        let widgets = &mut visuals.widgets;
        widgets.noninteractive.fg_stroke = Stroke::new(1.0, INK_MUTED);
        widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);

        // Boutons : repos, survol, puis pressé ou focus clavier (même état dans egui).
        let button_states = [
            (
                &mut widgets.inactive,
                CONTROL,
                Stroke::new(1.0, BORDER_CONTROL),
            ),
            (
                &mut widgets.hovered,
                CONTROL_HOVER,
                Stroke::new(1.0, BORDER_CONTROL),
            ),
            (
                &mut widgets.active,
                CONTROL_ACTIVE,
                Stroke::new(2.0, ACCENT),
            ),
        ];
        for (state, fill, stroke) in button_states {
            state.weak_bg_fill = fill;
            state.bg_fill = fill;
            state.bg_stroke = stroke;
            state.fg_stroke = Stroke::new(1.0, INK);
            state.expansion = 0.0;
        }
    });
}
