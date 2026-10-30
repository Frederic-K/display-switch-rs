// Commandes de développement : list, check, disable, enable.

use crate::{config, displays, switch};
use std::process::ExitCode;

const USAGE: &str = "Usage : display-switch [list | check | disable | enable]
  list     affiche les écrans disponibles
  check    valide la désactivation auprès de Windows, sans rien appliquer
  disable  mode Télétravail : désactive l'écran secondaire configuré
  enable   mode Personnel : réactive le bureau étendu
Sans argument, la fenêtre s'ouvre.";

// Exécuter une commande ; code de sortie 0 en cas de succès, 1 en cas d'erreur.
pub fn run(command: &str) -> ExitCode {
    let result = match command {
        "list" => list_displays(),
        "check" => config::load().and_then(|settings| switch::disable_secondary(&settings, true)),
        "disable" => {
            config::load().and_then(|settings| switch::disable_secondary(&settings, false))
        }
        "enable" => config::load().and_then(|settings| switch::enable_secondary(&settings)),
        "-h" | "--help" | "help" => Ok(USAGE.to_string()),
        _ => Err(format!("Commande inconnue : {command}\n{USAGE}")),
    };

    match result {
        Ok(text) => {
            println!("{text}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Erreur : {error}");
            ExitCode::FAILURE
        }
    }
}

// Texte de `list` : nom, état et chemin Windows de chaque écran.
fn list_displays() -> Result<String, String> {
    let (displays, _) = displays::read_displays()?;

    if displays.is_empty() {
        return Ok("Aucun écran disponible.".to_string());
    }

    let blocks: Vec<String> = displays
        .iter()
        .map(|display| {
            format!(
                "{}\n{}\nIdentifiant : {}",
                displays::display_name(display),
                displays::describe(display),
                display.device_path
            )
        })
        .collect();
    Ok(blocks.join("\n\n"))
}
