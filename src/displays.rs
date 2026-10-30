// Lecture des écrans connectés, sous une forme simple indépendante des structures Windows.

use crate::win::{self, Snapshot};
use windows::Win32::Graphics::Gdi::DISPLAYCONFIG_PATH_ACTIVE;

// Un écran disponible pour Windows, actif ou non.
#[derive(Clone, Debug, PartialEq)]
pub struct Display {
    pub name: String,
    pub device_path: String,
    pub active: bool,
    // Principal = surface du bureau placée en (0, 0) ; toujours faux si l'écran est inactif.
    pub primary: bool,
    pub resolution: Option<(u32, u32)>,
    // Deux écrans actifs avec le même mode source sont en duplication.
    pub source_mode_index: Option<u32>,
    // Position du chemin de cet écran dans `Snapshot::paths`.
    pub path_index: usize,
}

// Lire les écrans et garder la lecture Windows brute, nécessaire pour les modifier ensuite.
pub fn read_displays() -> Result<(Vec<Display>, Snapshot), String> {
    let snapshot = win::query_all_paths()?;
    let mut displays = Vec::new();

    for path_index in select_paths(&snapshot) {
        let path = &snapshot.paths[path_index];
        let (name, device_path) = win::monitor_identity(&path.targetInfo)?;
        let active = path.flags & DISPLAYCONFIG_PATH_ACTIVE != 0;
        let mut display = Display {
            name,
            device_path,
            active,
            primary: false,
            resolution: None,
            source_mode_index: None,
            path_index,
        };

        if active {
            let index = win::source_mode_index(path)
                .ok_or_else(|| format!("Mode d'affichage absent pour {}.", display.name))?;
            let mode = win::source_mode(&snapshot.modes, index)
                .ok_or_else(|| format!("Mode d'affichage invalide pour {}.", display.name))?;

            display.primary = mode.position.x == 0 && mode.position.y == 0;
            display.resolution = Some((mode.width, mode.height));
            display.source_mode_index = Some(index);
        }

        displays.push(display);
    }

    Ok((displays, snapshot))
}

// Windows renvoie plusieurs chemins possibles pour un même écran.
// Garder un seul chemin par cible disponible, en privilégiant le chemin actif.
fn select_paths(snapshot: &Snapshot) -> Vec<usize> {
    let mut selected: Vec<usize> = Vec::new();

    for (index, path) in snapshot.paths.iter().enumerate() {
        let target = &path.targetInfo;

        if !target.targetAvailable.as_bool() {
            continue;
        }

        let same_target = selected.iter().position(|&kept| {
            let kept_target = &snapshot.paths[kept].targetInfo;
            kept_target.adapterId == target.adapterId && kept_target.id == target.id
        });
        let is_active = path.flags & DISPLAYCONFIG_PATH_ACTIVE != 0;

        match same_target {
            None => selected.push(index),
            Some(position) if is_active => selected[position] = index,
            Some(_) => {}
        }
    }

    selected
}

// Nom à afficher, même si Windows n'en fournit pas.
pub fn display_name(display: &Display) -> &str {
    if display.name.is_empty() {
        "Moniteur sans nom"
    } else {
        &display.name
    }
}

// État d'un écran en une ligne, commun au terminal et à la fenêtre.
pub fn describe(display: &Display) -> String {
    match display.resolution {
        Some((width, height)) if display.active => {
            let role = if display.primary {
                "Principal"
            } else {
                "Secondaire"
            };
            format!("{width} × {height} — {role} — Actif")
        }
        _ => "Inactif — Résolution courante indisponible".to_string(),
    }
}
