// Préférences Télétravail : un petit fichier JSON placé à côté de l'exécutable.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const CONFIG_FILE_NAME: &str = "config.json";
const TEMPORARY_FILE_NAME: &str = "config.json.tmp";

// Noms exacts des deux écrans, tels que Windows les renvoie.
// Les clés JSON sont les mêmes que dans la version Python.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub primary_monitor: String,
    pub secondary_monitor: String,
}

// Vérifier que les deux noms sont renseignés et différents.
pub fn validate(settings: &Settings) -> Result<(), String> {
    if settings.primary_monitor.trim().is_empty() {
        return Err("Nom de moniteur manquant ou invalide : primary_monitor".to_string());
    }
    if settings.secondary_monitor.trim().is_empty() {
        return Err("Nom de moniteur manquant ou invalide : secondary_monitor".to_string());
    }
    if settings.primary_monitor == settings.secondary_monitor {
        return Err("Les deux moniteurs doivent avoir des noms différents.".to_string());
    }
    Ok(())
}

// Chemin du JSON : dans le dossier de l'exécutable, quel que soit le dossier courant.
pub fn config_path() -> Result<PathBuf, String> {
    let exe = std::env::current_exe()
        .map_err(|error| format!("Emplacement de l'exécutable introuvable : {error}"))?;
    Ok(exe.with_file_name(CONFIG_FILE_NAME))
}

pub fn load() -> Result<Settings, String> {
    load_from(&config_path()?)
}

pub fn save(settings: &Settings) -> Result<(), String> {
    save_to(&config_path()?, settings)
}

// Lire et valider le fichier, avec un message différent selon le problème rencontré.
fn load_from(path: &Path) -> Result<Settings, String> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(format!(
                "Configuration absente : {}. Utilisez « Configurer Télétravail… ».",
                path.display()
            ));
        }
        Err(error) => {
            return Err(format!(
                "Lecture impossible de {} : {error}",
                path.display()
            ));
        }
    };

    let settings: Settings = serde_json::from_str(&content)
        .map_err(|error| format!("Configuration invalide ({}) : {error}", path.display()))?;
    validate(&settings)?;
    Ok(settings)
}

// Écrire d'abord un fichier temporaire complet, puis le renommer.
// Le renommage remplace l'ancien fichier d'un coup : jamais de JSON à moitié écrit.
fn save_to(path: &Path, settings: &Settings) -> Result<(), String> {
    validate(settings)?;

    let content = serde_json::to_string_pretty(settings)
        .map_err(|error| format!("Conversion en JSON impossible : {error}"))?;
    let temporary_path = path.with_file_name(TEMPORARY_FILE_NAME);

    let result = write_file(&temporary_path, &(content + "\n"))
        .and_then(|()| fs::rename(&temporary_path, path));

    if let Err(error) = result {
        let _ = fs::remove_file(&temporary_path);
        return Err(format!(
            "Enregistrement impossible de {} : {error}",
            path.display()
        ));
    }
    Ok(())
}

// Écrire le contenu et attendre qu'il soit sur le disque avant le renommage.
fn write_file(path: &Path, content: &str) -> std::io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(primary: &str, secondary: &str) -> Settings {
        Settings {
            primary_monitor: primary.to_string(),
            secondary_monitor: secondary.to_string(),
        }
    }

    // Dossier temporaire propre à chaque test, vidé au début.
    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("display-switch-test-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn rejects_empty_or_identical_names() {
        assert!(validate(&settings("", "B")).is_err());
        assert!(validate(&settings("A", "  ")).is_err());
        assert!(validate(&settings("A", "A")).is_err());
        assert!(validate(&settings("A", "B")).is_ok());
    }

    #[test]
    fn save_then_load_returns_same_settings() {
        let path = test_dir("roundtrip").join(CONFIG_FILE_NAME);
        let original = settings("MSI MAG342CQR", "MAG321CURV");

        save_to(&path, &original).unwrap();

        assert_eq!(load_from(&path).unwrap(), original);
        assert!(!path.with_file_name(TEMPORARY_FILE_NAME).exists());
    }

    #[test]
    fn save_replaces_existing_file() {
        let path = test_dir("replace").join(CONFIG_FILE_NAME);
        save_to(&path, &settings("A", "B")).unwrap();
        save_to(&path, &settings("B", "A")).unwrap();

        assert_eq!(load_from(&path).unwrap(), settings("B", "A"));
    }

    #[test]
    fn invalid_settings_are_not_written() {
        let path = test_dir("invalid-save").join(CONFIG_FILE_NAME);

        assert!(save_to(&path, &settings("A", "A")).is_err());
        assert!(!path.exists());
    }

    #[test]
    fn reads_python_config_file() {
        let path = test_dir("python").join(CONFIG_FILE_NAME);
        let python_content = "{\n  \"secondary_monitor\": \"MAG321CURV\",\n  \"primary_monitor\": \"MSI MAG342CQR\"\n}";
        fs::write(&path, python_content).unwrap();

        assert_eq!(
            load_from(&path).unwrap(),
            settings("MSI MAG342CQR", "MAG321CURV")
        );
    }

    #[test]
    fn explains_missing_and_invalid_files() {
        let dir = test_dir("errors");
        let missing = load_from(&dir.join(CONFIG_FILE_NAME)).unwrap_err();
        assert!(missing.contains("Configuration absente"));

        let broken = dir.join("broken.json");
        fs::write(&broken, "{ pas du json").unwrap();
        assert!(
            load_from(&broken)
                .unwrap_err()
                .contains("Configuration invalide")
        );

        let incomplete = dir.join("incomplete.json");
        fs::write(&incomplete, "{\"primary_monitor\": \"A\"}").unwrap();
        assert!(
            load_from(&incomplete)
                .unwrap_err()
                .contains("Configuration invalide")
        );
    }
}
