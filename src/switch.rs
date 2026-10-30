// Modes Télétravail et Personnel, et détection des rôles pour la configuration.
// Les règles de sécurité travaillent sur des `Display` : elles se testent sans toucher l'affichage.

use crate::config::Settings;
use crate::displays::{self, Display};
use crate::win::{self, Action};

// Résultat de la préparation du mode Télétravail.
#[derive(Debug, PartialEq)]
enum DisablePlan {
    AlreadyDisabled,
    // Index des chemins Windows à garder actifs.
    KeepPaths(Vec<usize>),
}

// Retrouver un écran par son nom exact, en refusant l'absence ou l'ambiguïté.
fn find_display<'a>(displays: &'a [Display], name: &str) -> Result<&'a Display, String> {
    let mut matches = displays.iter().filter(|display| display.name == name);

    match (matches.next(), matches.next()) {
        (None, _) => Err(format!("Moniteur introuvable : {name}")),
        (Some(_), Some(_)) => Err(format!("Plusieurs moniteurs portent le nom : {name}")),
        (Some(display), None) => Ok(display),
    }
}

// Retrouver les deux écrans configurés et vérifier qu'ils sont distincts.
fn find_both<'a>(
    displays: &'a [Display],
    settings: &Settings,
) -> Result<(&'a Display, &'a Display), String> {
    let primary = find_display(displays, &settings.primary_monitor)?;
    let secondary = find_display(displays, &settings.secondary_monitor)?;

    if primary.path_index == secondary.path_index {
        return Err("Les deux rôles désignent le même écran.".to_string());
    }
    Ok((primary, secondary))
}

// L'écran à conserver doit être actif et déjà principal dans Windows.
fn check_primary(primary: &Display) -> Result<(), String> {
    if !primary.active {
        return Err("Le moniteur principal configuré est inactif.".to_string());
    }
    if !primary.primary {
        return Err(
            "Le moniteur principal configuré n'est pas principal dans Windows. \
             Rétablissez-le dans les paramètres Windows ou utilisez « Configurer Télétravail… »."
                .to_string(),
        );
    }
    Ok(())
}

fn is_clone(primary: &Display, secondary: &Display) -> bool {
    primary.source_mode_index.is_some() && primary.source_mode_index == secondary.source_mode_index
}

// Préparer la désactivation : chemins actifs moins le secondaire, principal toujours inclus.
fn plan_disable(displays: &[Display], settings: &Settings) -> Result<DisablePlan, String> {
    let (primary, secondary) = find_both(displays, settings)?;
    check_primary(primary)?;

    if !secondary.active {
        return Ok(DisablePlan::AlreadyDisabled);
    }
    if is_clone(primary, secondary) {
        return Err(
            "Les écrans sont en duplication : passez en bureau étendu avant cette action."
                .to_string(),
        );
    }

    let keep: Vec<usize> = displays
        .iter()
        .filter(|display| display.active && display.path_index != secondary.path_index)
        .map(|display| display.path_index)
        .collect();

    // Garantit aussi qu'il reste au moins un écran actif.
    if !keep.contains(&primary.path_index) {
        return Err("Le principal doit rester actif.".to_string());
    }
    Ok(DisablePlan::KeepPaths(keep))
}

// État attendu après Télétravail : principal conservé, secondaire inactif.
fn verify_disabled(displays: &[Display], settings: &Settings) -> Result<(), String> {
    let (primary, secondary) = find_both(displays, settings)?;
    check_primary(primary)?;

    if secondary.active {
        return Err("Après application, l'écran secondaire est encore actif.".to_string());
    }
    Ok(())
}

// État attendu après Personnel : principal conservé, secondaire actif, pas de duplication.
fn verify_enabled(displays: &[Display], settings: &Settings) -> Result<(), String> {
    let (primary, secondary) = find_both(displays, settings)?;
    check_primary(primary)?;

    if !secondary.active {
        return Err("L'écran secondaire est resté inactif.".to_string());
    }
    if is_clone(primary, secondary) {
        return Err("Windows a activé une duplication au lieu d'un bureau étendu.".to_string());
    }
    Ok(())
}

// Mode Télétravail : valider auprès de Windows, puis appliquer sauf si `dry_run`, puis relire.
pub fn disable_secondary(settings: &Settings, dry_run: bool) -> Result<String, String> {
    let (displays, snapshot) = displays::read_displays()?;

    let keep = match plan_disable(&displays, settings)? {
        DisablePlan::AlreadyDisabled => {
            return Ok("L'écran secondaire est déjà désactivé.".to_string());
        }
        DisablePlan::KeepPaths(keep) => keep,
    };
    let paths: Vec<_> = keep.iter().map(|&index| snapshot.paths[index]).collect();

    win::set_supplied_config(&paths, &snapshot.modes, Action::Validate)?;
    if dry_run {
        return Ok("Désactivation validée par Windows — aucun changement appliqué.".to_string());
    }

    win::set_supplied_config(&paths, &snapshot.modes, Action::Apply)?;

    let (updated, _) = displays::read_displays()?;
    verify_disabled(&updated, settings)?;
    Ok("Écran secondaire désactivé — principal actif et conservé.".to_string())
}

// Mode Personnel : demander à Windows sa dernière topologie étendue mémorisée,
// vérifier le résultat, et revenir à l'état précédent en cas d'échec.
pub fn enable_secondary(settings: &Settings) -> Result<String, String> {
    let (displays, snapshot) = displays::read_displays()?;

    if displays.len() != 2 {
        return Err("La réactivation nécessite exactement deux écrans disponibles.".to_string());
    }
    let (primary, secondary) = find_both(&displays, settings)?;
    check_primary(primary)?;

    if secondary.active {
        if is_clone(primary, secondary) {
            return Err("Les écrans sont en duplication, pas en extension.".to_string());
        }
        return Ok("L'écran secondaire est déjà actif en bureau étendu.".to_string());
    }

    // Copie en mémoire de l'état actuel, pour le retour arrière.
    let previous_paths: Vec<_> = displays
        .iter()
        .filter(|display| display.active)
        .map(|display| snapshot.paths[display.path_index])
        .collect();

    win::set_extend_topology(Action::Validate)?;

    let error = match apply_extend_and_verify(settings) {
        Ok(()) => {
            return Ok(
                "Écran secondaire activé — bureau étendu et principal conservé.".to_string(),
            );
        }
        Err(error) => error,
    };

    if let Err(restore_error) =
        win::set_supplied_config(&previous_paths, &snapshot.modes, Action::Apply)
    {
        return Err(format!(
            "Réactivation échouée : {error}. Retour arrière échoué : {restore_error}. \
             Rétablissez l'affichage dans les paramètres Windows."
        ));
    }

    // Relire pour dire précisément si l'état précédent est revenu.
    let restored =
        displays::read_displays().and_then(|(updated, _)| verify_disabled(&updated, settings));
    match restored {
        Ok(()) => Err(format!(
            "Réactivation non confirmée : {error}. L'état précédent a été rétabli."
        )),
        Err(check_error) => Err(format!(
            "Réactivation non confirmée : {error}. Windows a accepté le retour arrière, \
             mais l'état relu est inattendu : {check_error}"
        )),
    }
}

fn apply_extend_and_verify(settings: &Settings) -> Result<(), String> {
    win::set_extend_topology(Action::Apply)?;
    let (updated, _) = displays::read_displays()?;
    verify_enabled(&updated, settings)
}

// Proposer les préférences à partir de la configuration Windows actuelle, sans rien modifier.
pub fn detect_telework_config() -> Result<Settings, String> {
    let (displays, _) = displays::read_displays()?;
    settings_from_displays(&displays)
}

// Exiger deux écrans actifs en bureau étendu, un seul principal et des noms distincts.
fn settings_from_displays(displays: &[Display]) -> Result<Settings, String> {
    let [first, second] = displays else {
        return Err("Deux écrans doivent être disponibles.".to_string());
    };

    for display in displays {
        if !display.active {
            return Err("Activez les deux écrans avant de configurer.".to_string());
        }
        if display.name.trim().is_empty() {
            return Err("Un écran ne possède pas de nom utilisable.".to_string());
        }
    }
    if first.name == second.name {
        return Err("Les deux écrans portent le même nom.".to_string());
    }

    let (primary, secondary) = match (first.primary, second.primary) {
        (true, false) => (first, second),
        (false, true) => (second, first),
        _ => {
            return Err(
                "Un bureau étendu avec un seul écran principal est nécessaire.".to_string(),
            );
        }
    };

    Ok(Settings {
        primary_monitor: primary.name.clone(),
        secondary_monitor: secondary.name.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const BIG: &str = "MSI MAG342CQR";
    const SMALL: &str = "MAG321CURV";

    // Écran fictif : `source` est l'index du mode source, None si l'écran est inactif.
    fn display(name: &str, path_index: usize, primary: bool, source: Option<u32>) -> Display {
        Display {
            name: name.to_string(),
            device_path: format!("\\\\?\\DISPLAY#{path_index}"),
            active: source.is_some(),
            primary,
            resolution: source.map(|_| (3840, 2160)),
            source_mode_index: source,
            path_index,
        }
    }

    fn settings(primary: &str, secondary: &str) -> Settings {
        Settings {
            primary_monitor: primary.to_string(),
            secondary_monitor: secondary.to_string(),
        }
    }

    fn extended() -> Vec<Display> {
        vec![
            display(BIG, 0, true, Some(0)),
            display(SMALL, 3, false, Some(1)),
        ]
    }

    #[test]
    fn disable_keeps_only_the_primary_path() {
        let plan = plan_disable(&extended(), &settings(BIG, SMALL)).unwrap();
        assert_eq!(plan, DisablePlan::KeepPaths(vec![0]));
    }

    #[test]
    fn disable_reports_already_disabled_secondary() {
        let displays = vec![
            display(BIG, 0, true, Some(0)),
            display(SMALL, 3, false, None),
        ];
        let plan = plan_disable(&displays, &settings(BIG, SMALL)).unwrap();
        assert_eq!(plan, DisablePlan::AlreadyDisabled);
    }

    #[test]
    fn disable_refuses_when_windows_primary_changed() {
        let displays = vec![
            display(BIG, 0, false, Some(0)),
            display(SMALL, 3, true, Some(1)),
        ];
        let error = plan_disable(&displays, &settings(BIG, SMALL)).unwrap_err();
        assert!(error.contains("n'est pas principal"));
    }

    #[test]
    fn disable_refuses_inactive_primary() {
        let displays = vec![
            display(BIG, 0, false, None),
            display(SMALL, 3, true, Some(1)),
        ];
        assert!(plan_disable(&displays, &settings(BIG, SMALL)).is_err());
    }

    #[test]
    fn disable_refuses_clone_mode() {
        let displays = vec![
            display(BIG, 0, true, Some(0)),
            display(SMALL, 3, true, Some(0)),
        ];
        let error = plan_disable(&displays, &settings(BIG, SMALL)).unwrap_err();
        assert!(error.contains("duplication"));
    }

    #[test]
    fn disable_refuses_missing_or_ambiguous_monitor() {
        let missing = plan_disable(&extended(), &settings(BIG, "Autre")).unwrap_err();
        assert!(missing.contains("introuvable"));

        let mut twins = extended();
        twins.push(display(SMALL, 5, false, Some(2)));
        let ambiguous = plan_disable(&twins, &settings(BIG, SMALL)).unwrap_err();
        assert!(ambiguous.contains("Plusieurs"));
    }

    #[test]
    fn disable_keeps_other_active_screens() {
        let mut three = extended();
        three.push(display("Troisième", 7, false, Some(2)));
        let plan = plan_disable(&three, &settings(BIG, SMALL)).unwrap();
        assert_eq!(plan, DisablePlan::KeepPaths(vec![0, 7]));
    }

    #[test]
    fn verify_disabled_needs_inactive_secondary() {
        assert!(verify_disabled(&extended(), &settings(BIG, SMALL)).is_err());

        let disabled = vec![
            display(BIG, 0, true, Some(0)),
            display(SMALL, 3, false, None),
        ];
        assert!(verify_disabled(&disabled, &settings(BIG, SMALL)).is_ok());
    }

    #[test]
    fn verify_enabled_rejects_clone_and_inactive_secondary() {
        assert!(verify_enabled(&extended(), &settings(BIG, SMALL)).is_ok());

        let clone = vec![
            display(BIG, 0, true, Some(0)),
            display(SMALL, 3, true, Some(0)),
        ];
        assert!(verify_enabled(&clone, &settings(BIG, SMALL)).is_err());

        let inactive = vec![
            display(BIG, 0, true, Some(0)),
            display(SMALL, 3, false, None),
        ];
        assert!(verify_enabled(&inactive, &settings(BIG, SMALL)).is_err());
    }

    #[test]
    fn detects_roles_from_windows_primary() {
        assert_eq!(
            settings_from_displays(&extended()).unwrap(),
            settings(BIG, SMALL)
        );

        let swapped = vec![
            display(BIG, 0, false, Some(0)),
            display(SMALL, 3, true, Some(1)),
        ];
        assert_eq!(
            settings_from_displays(&swapped).unwrap(),
            settings(SMALL, BIG)
        );
    }

    #[test]
    fn detection_refuses_unusable_configurations() {
        let one = vec![display(BIG, 0, true, Some(0))];
        assert!(settings_from_displays(&one).is_err());

        let inactive = vec![
            display(BIG, 0, true, Some(0)),
            display(SMALL, 3, false, None),
        ];
        assert!(settings_from_displays(&inactive).is_err());

        let same_name = vec![
            display(BIG, 0, true, Some(0)),
            display(BIG, 3, false, Some(1)),
        ];
        assert!(settings_from_displays(&same_name).is_err());

        let clone = vec![
            display(BIG, 0, true, Some(0)),
            display(SMALL, 3, true, Some(0)),
        ];
        assert!(settings_from_displays(&clone).is_err());

        let unnamed = vec![
            display(BIG, 0, true, Some(0)),
            display("", 3, false, Some(1)),
        ];
        assert!(settings_from_displays(&unnamed).is_err());
    }
}
