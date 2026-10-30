# Display Switch (Rust)

Petit utilitaire Windows 11 qui désactive ou réactive un écran secondaire dans la configuration d'affichage Windows, en conservant l'écran principal configuré. C'est la réécriture en Rust de la version Python/PySide6 ([display-switch](https://github.com/Frederic-K/display-switch)), avec le même comportement et le même fichier `config.json`.

- **Télétravail** : désactive l'écran enregistré (par exemple le 32 pouces branché aussi au laptop professionnel).
- **Personnel** : réactive le bureau étendu.
- **Configurer Télétravail…** : enregistre les rôles actuellement choisis dans Windows.

L'application ne change pas l'entrée vidéo du moniteur et ne pilote pas le laptop.

## Prérequis

- Windows 11, session de bureau locale.
- Pour développer : Rust stable récent (testé avec 1.99.0, `rustup update stable`) et la chaîne MSVC (Visual Studio Build Tools, composant C++).
- Sur le PC cible : rien à installer. Voir « Taille et dépendances ».

## Construire

```powershell
cargo build --release
```

Le livrable est `target\release\DisplaySwitch.exe`. Le copier dans un dossier accessible en écriture : `config.json` sera lu et enregistré à côté de lui.

Contrôles utilisés pendant le développement :

```powershell
cargo fmt --check
cargo clippy --all-targets
cargo test
```

Les tests portent sur les règles de sécurité (avec de faux écrans) et sur le fichier de configuration. Ils ne modifient jamais l'affichage.

## Utilisation

Double-cliquer sur `DisplaySwitch.exe`. La fenêtre affiche chaque écran disponible avec son nom, sa résolution, son rôle (principal ou secondaire) et son état (point vert « Actif », anneau gris « Inactif »). Un écran désactivé reste listé. L'apparence suit le design system « Display Switch » : thème clair et neutre, police Ubuntu Light intégrée à egui, aucune icône.

| Bouton | Effet |
| --- | --- |
| Télétravail | Désactive l'écran secondaire configuré, garde le principal. |
| Personnel | Redemande à Windows le bureau étendu mémorisé. |
| Actualiser | Relit les écrans (après un changement fait dans les paramètres Windows). |
| Configurer Télétravail… | Propose les rôles actuels de Windows, à Enregistrer ou Annuler. |

Les appels Windows sont synchrones : la fenêtre peut rester figée un court instant pendant une bascule.

### Première configuration

1. Activer les deux écrans en bureau étendu dans les paramètres Windows.
2. Choisir dans Windows l'écran principal à conserver pour le télétravail.
3. Cliquer sur Configurer Télétravail…, vérifier les noms, puis Enregistrer.

Annuler, Échap ou un clic hors de la boîte laissent le fichier intact. Rien n'est modifié dans l'affichage. Les rôles sont relus au moment d'enregistrer : s'ils ont changé entre-temps, l'enregistrement est refusé.

### Fichier `config.json`

```json
{
  "primary_monitor": "MSI MAG342CQR",
  "secondary_monitor": "MAG321CURV"
}
```

Ce sont les noms exacts renvoyés par Windows (majuscules et espaces compris). Le fichier de la version Python est directement réutilisable. Il est relu à chaque action et enregistré de façon atomique : écriture complète dans `config.json.tmp`, puis renommage.

## Commandes de développement

Les commandes servent pendant le développement, depuis le projet :

```powershell
cargo run -- list
cargo run -- check
cargo run -- disable
cargo run -- enable
```

| Commande | Comportement |
| --- | --- |
| `list` | Affiche les écrans, leur état et leur chemin Windows. Ne lit pas la configuration. |
| `check` | Prépare Télétravail et le fait valider par Windows, sans rien appliquer. |
| `disable` | Mode Télétravail. **Modifie l'affichage.** |
| `enable` | Mode Personnel. **Modifie l'affichage.** |

Code de sortie 0 en cas de succès, 1 en cas d'erreur (`Erreur : …`). `cargo run` sans argument ouvre la fenêtre.

En développement, l'exécutable est `target\debug\display-switch.exe` et lit donc `target\debug\config.json`. Copier un `config.json` à cet endroit, ou utiliser Configurer Télétravail… depuis `cargo run`.

### Pourquoi la CLI ne fonctionne pas avec l'exe distribué

Un exécutable Windows est lié soit au sous-système « console » (une fenêtre noire s'ouvre au double-clic), soit au sous-système « windows » (pas de console, la sortie texte est perdue et le terminal n'attend pas la fin). `src/main.rs` choisit selon le profil :

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
```

- `cargo build` / `cargo run` (debug) : application console, les commandes s'affichent.
- `cargo build --release` : application fenêtrée sans console. Les commandes s'exécutent encore mais n'affichent rien. Il faut donc les utiliser avec `cargo run`.

## Contrôles de sécurité

**Télétravail** (`check` s'arrête après l'étape 4) :

1. Retrouver les deux écrans par nom exact. Refus si un nom est absent ou porté par plusieurs écrans, ou si les deux rôles désignent le même écran.
2. Exiger que l'écran à conserver soit actif **et** principal dans Windows (surface en position 0, 0). Si le principal a été changé dans Windows sans reconfigurer, l'action est refusée.
3. Écran ciblé déjà inactif : simple information. Écrans en duplication : refus.
4. Garder tous les chemins actifs sauf le secondaire. La liste doit contenir le principal, il reste donc toujours un écran actif. Windows valide cette proposition (`SDC_VALIDATE`).
5. Appliquer (`SDC_APPLY`), puis relire : principal toujours principal, secondaire inactif.

Télétravail n'a pas de retour arrière automatique, comme la version Python. Une erreur à l'étape 5 ne signifie pas qu'aucun changement n'a eu lieu.

**Personnel** :

1. Exiger exactement deux écrans disponibles, retrouvés par nom, avec le principal configuré actif et principal.
2. Secondaire déjà actif : information (ou refus en cas de duplication).
3. Garder en mémoire les chemins actifs actuels.
4. Demander à Windows sa **dernière topologie étendue mémorisée** (`SDC_TOPOLOGY_EXTEND`, sans fournir de tableaux) : validation, puis application.
5. Relire : secondaire actif, principal conservé, pas de duplication.
6. En cas d'échec : réappliquer les chemins mémorisés, puis relire pour dire si l'état précédent est bien revenu. Si ce retour arrière échoue, les deux erreurs sont affichées.

Le programme ne reconstruit pas la disposition lui-même : positions, résolutions et principal viennent de la base de configuration de Windows. Télétravail n'utilise pas `SDC_SAVE_TO_DATABASE`, donc la configuration à un seul écran ne remplace pas la disposition étendue mémorisée. Ce n'est pas une garantie absolue, d'où les vérifications et le retour arrière.

## Organisation du code

| Fichier | Rôle |
| --- | --- |
| `src/main.rs` | Choix entre fenêtre et commande. |
| `src/win.rs` | Seule couche `unsafe` : appels `GetDisplayConfigBufferSizes`, `QueryDisplayConfig`, `DisplayConfigGetDeviceInfo`, `SetDisplayConfig`, boîte d'erreur. |
| `src/displays.rs` | Transforme la lecture Windows en liste de `Display` simples (un par écran). |
| `src/switch.rs` | Règles de sécurité, Télétravail, Personnel, détection des rôles, et leurs tests. |
| `src/config.rs` | Chemin, validation, lecture et écriture atomique de `config.json`, et leurs tests. |
| `src/cli.rs` | Commandes `list`, `check`, `disable`, `enable`. |
| `src/ui.rs` | Fenêtre egui et boîte de confirmation. |
| `src/theme.rs` | Couleurs, tailles et espacements du design system, appliqués à egui au démarrage. |
| `.cargo/config.toml` | Liaison statique du runtime C (voir plus bas). |

Pour relire : `main.rs`, puis `switch.rs` (le comportement), `displays.rs`, et enfin `win.rs` pour les détails Windows.

### Quelques notions Rust rencontrées

- **`Result<T, String>`** : chaque fonction qui peut échouer renvoie soit une valeur, soit un message d'erreur. L'opérateur `?` remonte l'erreur à l'appelant, jusqu'à la CLI ou la fenêtre qui l'affiche.
- **Emprunts (`&`)** : les règles reçoivent `&[Display]` et `&Settings` ; elles lisent sans devenir propriétaires. `find_display` renvoie une référence `&Display` vers un élément de la liste, valable tant que la liste existe (c'est le sens de `'a`).
- **Ownership** : `Snapshot` possède les tableaux remplis par Windows. Ils vivent tant que le `Snapshot` existe, donc les pointeurs passés à Windows restent valides pendant l'appel.
- **Unions** : Windows range plusieurs variantes dans la même zone mémoire (`DISPLAYCONFIG_MODE_INFO`). Lire une union est `unsafe` : le code vérifie d'abord `infoType` avant de lire `sourceMode`.
- **`unsafe`** : limité à `win.rs`. Chaque bloc porte un commentaire « Sûr : … » qui donne la condition rendant l'appel correct.
- **egui** : la fenêtre est redessinée à chaque image à partir de la struct `MainWindow`. Un clic modifie ses champs, l'image suivante l'affiche.

## Taille et dépendances

Mesuré sur la machine de développement avec `cargo build --release` (Rust 1.99.0, eframe 0.36.2, windows 0.62.2) : **`DisplaySwitch.exe` = 5 006 848 octets (environ 4,8 Mo)**, en un seul fichier. La version Python fait environ 45 Mo.

Ce qui réduit la taille, dans `Cargo.toml` : optimisation pour la taille, LTO, symboles retirés, et le moteur de rendu `glow` (OpenGL) au lieu de `wgpu`, celui d'eframe par défaut.

Dépendances sur le PC cible, toutes fournies par Windows :

- DLL système (`user32`, `kernel32`, `gdi32`, `dwmapi`, `uxtheme`, etc.).
- `opengl32.dll` et le pilote graphique, pour le rendu de la fenêtre.
- Pas de Visual C++ Redistributable : le runtime C est lié dans l'exécutable (`+crt-static` dans `.cargo/config.toml`).

Crates Rust utilisées : `windows` (bindings officiels Microsoft), `serde` et `serde_json` (JSON), `eframe` (egui et sa fenêtre).

## Limites

- Identification par nom exact. Deux écrans du même modèle portant le même nom provoquent un refus.
- Personnel exige exactement deux écrans disponibles, comme la version Python.
- Pas de surveillance des branchements : utiliser Actualiser.
- Le programme contrôle ce que Windows déclare actif. Il ne sait pas quelle entrée le moniteur affiche réellement.
- Hors périmètre : choix libre de l'écran à désactiver, profils multiples, icône de notification, démarrage automatique, raccourcis globaux, mises à jour automatiques.

## Dépannage

| Message ou symptôme | Que faire |
| --- | --- |
| `Configuration absente` | Utiliser Configurer Télétravail… avec les deux écrans actifs, ou placer `config.json` à côté de l'exe. |
| `Configuration invalide` | Corriger le JSON, ou le recréer avec Configurer Télétravail…. |
| `Moniteur introuvable` | Vérifier les noms avec `cargo run -- list` et le branchement de l'écran. |
| `Plusieurs moniteurs portent le nom` | Cas non géré : identification par nom uniquement. |
| `… n'est pas principal dans Windows` | Remettre le principal attendu dans Windows, ou réenregistrer avec Configurer Télétravail…. |
| `duplication` | Passer en bureau étendu dans les paramètres Windows (Windows + P, « Étendre »). |
| `Enregistrement impossible` | Le dossier de l'exe doit être accessible en écriture. |
| `Impossible d'ouvrir la fenêtre` | Le rendu OpenGL n'a pas démarré : vérifier le pilote graphique (session Bureau à distance ou machine virtuelle sans accélération). |
| Erreur après une bascule | Actualiser. Si besoin, rétablir le bureau étendu dans les paramètres Windows. |

## Références

- [QueryDisplayConfig](https://learn.microsoft.com/fr-fr/windows/win32/api/winuser/nf-winuser-querydisplayconfig)
- [SetDisplayConfig](https://learn.microsoft.com/fr-fr/windows/win32/api/winuser/nf-winuser-setdisplayconfig)
- [DisplayConfigGetDeviceInfo](https://learn.microsoft.com/fr-fr/windows/win32/api/winuser/nf-winuser-displayconfiggetdeviceinfo)
- [Crate windows](https://microsoft.github.io/windows-docs-rs/)
- [egui / eframe](https://github.com/emilk/egui)
