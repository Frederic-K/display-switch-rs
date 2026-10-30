// Display Switch : désactive ou réactive l'écran secondaire configuré.
// En version release, l'exécutable est une application fenêtrée sans console.
// En développement (cargo run), la console reste disponible pour les commandes.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cli;
mod config;
mod displays;
mod switch;
mod theme;
mod ui;
mod win;

use std::process::ExitCode;

// Sans argument : la fenêtre. Avec un argument : une commande de développement.
fn main() -> ExitCode {
    match std::env::args().nth(1) {
        None => ui::run(),
        Some(command) => cli::run(&command),
    }
}
