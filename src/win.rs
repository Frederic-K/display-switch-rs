// Appels aux API Windows d'affichage.
// C'est le seul fichier du programme qui contient du code `unsafe`.

use windows::Win32::Devices::Display::{
    DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME, DISPLAYCONFIG_MODE_INFO,
    DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE, DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_PATH_TARGET_INFO,
    DISPLAYCONFIG_SOURCE_MODE, DISPLAYCONFIG_TARGET_DEVICE_NAME, DisplayConfigGetDeviceInfo,
    GetDisplayConfigBufferSizes, QDC_ALL_PATHS, QueryDisplayConfig, SDC_APPLY, SDC_TOPOLOGY_EXTEND,
    SDC_USE_SUPPLIED_DISPLAY_CONFIG, SDC_VALIDATE, SET_DISPLAY_CONFIG_FLAGS, SetDisplayConfig,
};
use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS};
use windows::Win32::Graphics::Gdi::DISPLAYCONFIG_PATH_MODE_IDX_INVALID;
use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
use windows::core::{HRESULT, HSTRING};

// Nombre de lectures tentées si la configuration change entre deux appels.
const QUERY_ATTEMPTS: usize = 3;

// Chemins et modes lus ensemble : les index de mode des chemins
// ne sont valables que dans le tableau `modes` de la même lecture.
pub struct Snapshot {
    pub paths: Vec<DISPLAYCONFIG_PATH_INFO>,
    pub modes: Vec<DISPLAYCONFIG_MODE_INFO>,
}

// Valider seulement, ou appliquer réellement une configuration.
#[derive(Clone, Copy)]
pub enum Action {
    Validate,
    Apply,
}

impl Action {
    fn flag(self) -> SET_DISPLAY_CONFIG_FLAGS {
        match self {
            Action::Validate => SDC_VALIDATE,
            Action::Apply => SDC_APPLY,
        }
    }
}

// Transformer un code d'erreur Windows en message lisible (traduit par Windows).
fn check_code(code: u32, call: &str) -> Result<(), String> {
    if code == ERROR_SUCCESS.0 {
        return Ok(());
    }
    let message = HRESULT::from_win32(code).message();
    Err(format!("{call} a échoué : {message} (code {code})"))
}

// Lire tous les chemins d'affichage, actifs ou non, avec leurs modes.
// Recommencer si Windows signale que la capacité a changé entre les deux appels.
pub fn query_all_paths() -> Result<Snapshot, String> {
    for _ in 0..QUERY_ATTEMPTS {
        let mut path_count = 0u32;
        let mut mode_count = 0u32;

        // Sûr : les deux pointeurs désignent des variables locales vivantes pendant l'appel.
        let code =
            unsafe { GetDisplayConfigBufferSizes(QDC_ALL_PATHS, &mut path_count, &mut mode_count) };
        check_code(code.0, "GetDisplayConfigBufferSizes")?;

        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];

        // Sûr : chaque tableau contient exactement le nombre d'éléments annoncé à Windows,
        // qui n'écrit jamais au-delà ; les Vec restent possédés ici pendant tout l'appel.
        let code = unsafe {
            QueryDisplayConfig(
                QDC_ALL_PATHS,
                &mut path_count,
                paths.as_mut_ptr(),
                &mut mode_count,
                modes.as_mut_ptr(),
                None,
            )
        };

        if code == ERROR_INSUFFICIENT_BUFFER {
            continue;
        }
        check_code(code.0, "QueryDisplayConfig")?;

        // Windows indique combien d'éléments il a réellement remplis.
        paths.truncate(path_count as usize);
        modes.truncate(mode_count as usize);
        return Ok(Snapshot { paths, modes });
    }

    Err("La configuration des écrans change en permanence. Réessayez.".to_string())
}

// Index du mode source d'un chemin, ou None s'il n'y en a pas.
pub fn source_mode_index(path: &DISPLAYCONFIG_PATH_INFO) -> Option<u32> {
    // Sûr : sans l'option QDC_VIRTUAL_MODE_AWARE, Windows remplit la variante `modeInfoIdx`
    // de l'union ; les deux variantes sont de simples u32, toute valeur est donc lisible.
    let index = unsafe { path.sourceInfo.Anonymous.modeInfoIdx };
    (index != DISPLAYCONFIG_PATH_MODE_IDX_INVALID).then_some(index)
}

// Mode source (résolution et position sur le bureau) désigné par un index.
pub fn source_mode(
    modes: &[DISPLAYCONFIG_MODE_INFO],
    index: u32,
) -> Option<DISPLAYCONFIG_SOURCE_MODE> {
    let mode = modes.get(index as usize)?;

    if mode.infoType != DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE {
        return None;
    }

    // Sûr : `infoType` vient d'être vérifié, l'union contient bien un mode source.
    Some(unsafe { mode.Anonymous.sourceMode })
}

// Nom lisible et chemin de périphérique du moniteur relié à une cible.
pub fn monitor_identity(
    target: &DISPLAYCONFIG_PATH_TARGET_INFO,
) -> Result<(String, String), String> {
    let mut request = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
    request.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
    request.header.size = size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
    request.header.adapterId = target.adapterId;
    request.header.id = target.id;

    // Sûr : l'en-tête est le premier champ de la structure complète, dont la taille
    // est indiquée dans `header.size` ; Windows remplit donc uniquement `request`.
    let code = unsafe { DisplayConfigGetDeviceInfo(&mut request.header) };
    check_code(code.cast_unsigned(), "DisplayConfigGetDeviceInfo")?;

    Ok((
        utf16_to_string(&request.monitorFriendlyDeviceName),
        utf16_to_string(&request.monitorDevicePath),
    ))
}

// Convertir un texte Windows (UTF-16 terminé par un zéro) en String Rust.
fn utf16_to_string(text: &[u16]) -> String {
    let end = text.iter().position(|&c| c == 0).unwrap_or(text.len());
    String::from_utf16_lossy(&text[..end])
}

// Valider ou appliquer exactement les chemins et modes fournis.
pub fn set_supplied_config(
    paths: &[DISPLAYCONFIG_PATH_INFO],
    modes: &[DISPLAYCONFIG_MODE_INFO],
    action: Action,
) -> Result<(), String> {
    // Sûr : les tranches sont empruntées pendant tout l'appel et Windows ne fait que les lire.
    let code = unsafe {
        SetDisplayConfig(
            Some(paths),
            Some(modes),
            SDC_USE_SUPPLIED_DISPLAY_CONFIG | action.flag(),
        )
    };
    check_code(code.cast_unsigned(), "SetDisplayConfig")
}

// Valider ou appliquer la dernière topologie étendue mémorisée par Windows.
pub fn set_extend_topology(action: Action) -> Result<(), String> {
    // Sûr : aucun tableau n'est transmis, Windows utilise sa propre base de configuration.
    let code = unsafe { SetDisplayConfig(None, None, SDC_TOPOLOGY_EXTEND | action.flag()) };
    check_code(code.cast_unsigned(), "SetDisplayConfig")
}

// Boîte de message Windows, pour les erreurs survenant avant l'ouverture de la fenêtre.
pub fn show_error(title: &str, message: &str) {
    let title = HSTRING::from(title);
    let message = HSTRING::from(message);

    // Sûr : les deux textes UTF-16 terminés par zéro vivent jusqu'à la fin de l'appel.
    unsafe {
        MessageBoxW(None, &message, &title, MB_OK | MB_ICONERROR);
    }
}
