// Fenêtre egui : état des écrans, Télétravail, Personnel, Actualiser et Configurer.
// L'apparence (couleurs, tailles, espacements) vient de theme.rs.

use crate::config::{self, Settings};
use crate::displays::{self, Display};
use crate::theme::{self, *};
use crate::{switch, win};
use eframe::egui::{
    self, Align, Button, CornerRadius, Frame, Layout, Margin, RichText, Sense, Separator, Stroke,
    Ui, Vec2,
};
use std::process::ExitCode;

const WINDOW_TITLE: &str = "Display Switch";
// Taille intérieure : 420 points de large, hauteur prévue pour deux écrans et un message.
const WINDOW_SIZE: [f32; 2] = [420.0, 300.0];
const DIALOG_WIDTH: f32 = 340.0;
const CONFIGURE_DIALOG_ID: &str = "configure-telework";

// Ouvrir la fenêtre ; en cas d'échec, l'erreur est montrée dans une boîte Windows
// car l'exécutable distribué n'a pas de console.
pub fn run() -> ExitCode {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(WINDOW_SIZE)
            .with_min_inner_size(WINDOW_SIZE),
        ..Default::default()
    };

    let result = eframe::run_native(
        WINDOW_TITLE,
        options,
        Box::new(|creation_context| {
            theme::apply(&creation_context.egui_ctx);
            Ok(Box::new(MainWindow::new()))
        }),
    );

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            win::show_error(
                WINDOW_TITLE,
                &format!("Impossible d'ouvrir la fenêtre : {error}"),
            );
            ExitCode::FAILURE
        }
    }
}

// Ce que la fenêtre doit se rappeler d'une image à l'autre.
// egui redessine tout à chaque image à partir de ces champs.
struct MainWindow {
    displays: Result<Vec<Display>, String>,
    // Résultat de la dernière action : Ok = information, Err = erreur.
    message: Option<Result<String, String>>,
    // Proposition affichée dans la boîte « Configurer Télétravail » tant qu'elle est ouverte.
    proposal: Option<Settings>,
}

impl MainWindow {
    fn new() -> Self {
        let mut window = MainWindow {
            displays: Ok(Vec::new()),
            message: None,
            proposal: None,
        };
        window.refresh_displays();
        window
    }

    fn refresh_displays(&mut self) {
        self.displays = displays::read_displays().map(|(displays, _)| displays);
    }

    // Lire la configuration, lancer le mode demandé, puis relire les écrans.
    fn run_mode(&mut self, mode: fn(&Settings) -> Result<String, String>) {
        self.message = Some(config::load().and_then(|settings| mode(&settings)));
        self.refresh_displays();
    }

    // Ouvrir la boîte de confirmation avec les rôles détectés dans Windows.
    fn start_configuration(&mut self) {
        match switch::detect_telework_config() {
            Ok(settings) => self.proposal = Some(settings),
            Err(error) => self.message = Some(Err(error)),
        }
        self.refresh_displays();
    }

    // Revérifier Windows au moment d'enregistrer : les rôles ont pu changer entre-temps.
    fn save_configuration(&mut self, proposal: Settings) {
        let result = switch::detect_telework_config().and_then(|current| {
            if current != proposal {
                return Err("La configuration Windows a changé. \
                     Rouvrez « Configurer Télétravail… »."
                    .to_string());
            }
            config::save(&proposal)?;
            Ok("Configuration Télétravail enregistrée.".to_string())
        });
        self.message = Some(result);
        self.refresh_displays();
    }

    // Bloc des écrans : une ligne par écran, séparées par un trait.
    fn show_displays(&self, ui: &mut Ui) {
        Frame::new()
            .fill(SURFACE)
            .stroke(Stroke::new(1.0, BORDER))
            .corner_radius(CornerRadius::same(RADIUS_LG))
            .inner_margin(Margin::symmetric(SPACE_3 as i8, SPACE_2 as i8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                match &self.displays {
                    Ok(list) if list.is_empty() => {
                        ui.label("Aucun écran disponible.");
                    }
                    Ok(list) => {
                        for (index, display) in list.iter().enumerate() {
                            if index > 0 {
                                ui.add(Separator::default().spacing(SPACE_2));
                            }
                            monitor_row(ui, display);
                        }
                    }
                    Err(error) => {
                        ui.label(format!("Lecture des écrans impossible : {error}"));
                    }
                }
            });
    }

    // Ligne d'état centrée ; vide, elle garde sa hauteur pour que rien ne bouge.
    fn show_message(&self, ui: &mut Ui) {
        let text = match &self.message {
            Some(Ok(text)) => text.clone(),
            Some(Err(error)) => format!("Erreur : {error}"),
            None => " ".to_string(),
        };
        ui.vertical_centered(|ui| ui.label(text));
    }

    // Boîte modale : renvoie Some(true) pour Enregistrer, Some(false) pour Annuler.
    fn show_configuration_dialog(ui: &Ui, proposal: &Settings) -> Option<bool> {
        let response = egui::Modal::new(egui::Id::new(CONFIGURE_DIALOG_ID)).show(ui.ctx(), |ui| {
            ui.set_width(DIALOG_WIDTH);
            ui.label(RichText::new("Configurer Télétravail").heading().color(INK));
            ui.add_space(SPACE_2);
            ui.label(format!("Écran à conserver : {}", proposal.primary_monitor));
            ui.label(format!(
                "Écran à désactiver : {}",
                proposal.secondary_monitor
            ));
            ui.add_space(SPACE_2);
            ui.label("Ces réglages seront utilisés par le bouton Télétravail.");
            ui.label("Aucun changement d'affichage ne sera appliqué.");
            ui.add_space(SPACE_3);

            let [save, cancel] = button_row(
                ui,
                CONTROL_SM,
                RADIUS_SM,
                BUTTON_SIZE,
                ["Enregistrer", "Annuler"],
            );
            if save {
                Some(true)
            } else if cancel {
                Some(false)
            } else {
                None
            }
        });

        // Échap ou un clic hors de la boîte valent Annuler.
        if response.should_close() {
            return Some(false);
        }
        response.inner
    }
}

impl eframe::App for MainWindow {
    // Les blocs s'empilent de haut en bas, séparés de SPACE_3.
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        // `item_spacing` ajoute déjà SPACE_1 entre deux éléments.
        let gap = SPACE_3 - SPACE_1;
        let panel_frame = Frame::new()
            .fill(BG)
            .inner_margin(Margin::same(SPACE_4 as i8));

        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ui, |ui| {
                ui.label(RichText::new(WINDOW_TITLE).heading().color(INK));
                ui.add_space(gap);
                self.show_displays(ui);
                ui.add_space(gap);

                let [telework, personal] = button_row(
                    ui,
                    CONTROL_LG,
                    RADIUS_MD,
                    BUTTON_LG_SIZE,
                    ["Télétravail", "Personnel"],
                );
                if telework {
                    self.run_mode(|settings| switch::disable_secondary(settings, false));
                }
                if personal {
                    self.run_mode(switch::enable_secondary);
                }
                ui.add_space(gap);
                self.show_message(ui);
                ui.add_space(gap);

                let [refresh, configure] = button_row(
                    ui,
                    CONTROL_SM,
                    RADIUS_SM,
                    BUTTON_SIZE,
                    ["Actualiser", "Configurer Télétravail…"],
                );
                if refresh {
                    self.refresh_displays();
                }
                if configure {
                    self.start_configuration();
                }
            });

        // `take` sort la proposition du champ : on décide ensuite de la remettre ou non.
        if let Some(proposal) = self.proposal.take() {
            match Self::show_configuration_dialog(ui, &proposal) {
                Some(true) => self.save_configuration(proposal),
                Some(false) => {}
                None => self.proposal = Some(proposal),
            }
        }
    }
}

// Deux boutons de même largeur sur toute la ligne ; renvoie lesquels viennent d'être cliqués.
fn button_row(
    ui: &mut Ui,
    height: f32,
    radius: u8,
    text_size: f32,
    labels: [&str; 2],
) -> [bool; 2] {
    let width = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
    let mut clicked = [false; 2];

    ui.horizontal(|ui| {
        for (label, was_clicked) in labels.into_iter().zip(&mut clicked) {
            let button = Button::new(RichText::new(label).size(text_size))
                .corner_radius(CornerRadius::same(radius))
                .truncate();
            *was_clicked = ui.add_sized([width, height], button).clicked();
        }
    });
    clicked
}

// Une ligne d'écran : nom et détail à gauche, état à droite.
fn monitor_row(ui: &mut Ui, display: &Display) {
    let detail = match display.resolution {
        Some((width, height)) if display.active => {
            let role = if display.primary {
                "Principal"
            } else {
                "Secondaire"
            };
            format!("{width} × {height} — {role}")
        }
        _ => "Résolution indisponible".to_string(),
    };

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            let name = RichText::new(displays::display_name(display))
                .size(MONITOR_NAME_SIZE)
                .color(INK);
            ui.label(name);
            ui.label(detail);
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            state_badge(ui, display.active);
        });
    });
}

// Point d'état toujours accompagné de son mot : disque vert « Actif », anneau gris « Inactif ».
// La disposition va de droite à gauche : le libellé d'abord, le point ensuite.
fn state_badge(ui: &mut Ui, active: bool) {
    ui.spacing_mut().item_spacing.x = SPACE_1;
    ui.label(RichText::new(if active { "Actif" } else { "Inactif" }).small());

    let (rect, _) = ui.allocate_exact_size(Vec2::splat(DOT_SIZE), Sense::hover());
    let radius = DOT_SIZE / 2.0;
    if active {
        ui.painter().circle_filled(rect.center(), radius, SUCCESS);
    } else {
        ui.painter().circle_stroke(
            rect.center(),
            radius - 0.5,
            Stroke::new(1.0, BORDER_CONTROL),
        );
    }
}
