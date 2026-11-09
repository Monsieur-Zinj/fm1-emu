//! macOS desktop firmware chooser. The running emulator is stopped only after a valid selection.
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(target_os = "macos")]
fn dialog(script: &str, args: &[String]) -> Result<Option<String>, String> {
    let output = Command::new("osascript").arg("-e").arg(script).args(args).output()
        .map_err(|e| format!("Impossible d'ouvrir le dialogue : {e}"))?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr);
        if message.contains("-128") { return Ok(None); }
        return Err(format!("Dialogue : {}", message.trim()));
    }
    let value = String::from_utf8_lossy(&output.stdout).trim_end_matches(['\r', '\n']).to_string();
    if value == "false" || value.is_empty() { Ok(None) } else { Ok(Some(value)) }
}

#[cfg(target_os = "macos")]
pub fn choose(catalog: &[(String, String, String, String)], has_flash: bool) -> Result<Option<PathBuf>, String> {
    if has_flash {
        return Err("Changement désactivé avec --flash : chaque firmware doit utiliser sa propre sauvegarde. Relancez sans --flash ou avec une sauvegarde adaptée.".into());
    }
    let mut labels = vec!["Ouvrir un fichier .fwsc…".to_string()];
    labels.extend(catalog.iter().map(|(id, name, version, path)| {
        format!("{name} {version} [{id}]{}", if Path::new(path).is_file() { "" } else { " — à télécharger" })
    }));
    let script = r#"on run argv
activate
set chosen to choose from list argv with title "Changer de firmware" with prompt "Choisis un firmware. Les réglages non sauvegardés seront perdus." OK button name "Ouvrir" cancel button name "Annuler"
if chosen is false then return "false"
return item 1 of chosen
end run"#;
    let Some(selected) = dialog(script, &labels)? else { return Ok(None) };
    let index = labels.iter().position(|label| label == &selected).ok_or("Sélection inconnue")?;
    let path = if index == 0 {
        let script = r#"on run
activate
set chosen to choose file with prompt "Choisis un fichier firmware .fwsc"
return POSIX path of chosen
end run"#;
        let Some(path) = dialog(script, &[])? else { return Ok(None) };
        let path = PathBuf::from(path);
        if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("fwsc")) {
            return Err("Choisis un fichier dont le nom se termine par .fwsc.".into());
        }
        path
    } else {
        let (id, _, _, path) = &catalog[index - 1];
        let path = PathBuf::from(path);
        if !path.is_file() {
            eprintln!("Téléchargement du firmware {id}…");
            let output = Command::new("node").args(["tools/fetch_firmwares.mjs", id]).output()
                .map_err(|e| format!("Téléchargement impossible (Node.js requis) : {e}"))?;
            if !output.status.success() || !path.is_file() {
                return Err(format!("Téléchargement impossible : {} {}", String::from_utf8_lossy(&output.stdout).trim(), String::from_utf8_lossy(&output.stderr).trim()));
            }
        }
        path
    };
    let bytes = std::fs::read(&path).map_err(|e| format!("Lecture de {} : {e}", path.display()))?;
    fm1_soc::stock_from_package(&bytes, None).map_err(|e| format!("Firmware invalide : {e}"))?;
    std::fs::canonicalize(&path).map(Some).map_err(|e| e.to_string())
}

#[cfg(not(target_os = "macos"))]
pub fn choose(_catalog: &[(String, String, String, String)], _has_flash: bool) -> Result<Option<PathBuf>, String> {
    Err("Le sélecteur de firmware est disponible sur macOS uniquement.".into())
}

pub fn report_error(message: &str) {
    eprintln!("{message}");
    #[cfg(target_os = "macos")]
    {
        let _ = dialog("on run argv\nactivate\ndisplay alert \"Changement de firmware\" message (item 1 of argv) as warning\nend run", &[message.to_string()]);
    }
}

/// Carry over emulator options, but not the previous firmware, its bank or flash image.
pub fn restart_args(args: impl IntoIterator<Item = String>, path: &Path) -> Vec<String> {
    let mut result = Vec::new();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if matches!(arg.as_str(), "--fw" | "--fwsc" | "--bin" | "--entry" | "--syx" | "--flash") {
            args.next();
        } else {
            result.push(arg);
        }
    }
    result.push("--fwsc".into());
    result.push(path.to_string_lossy().into_owned());
    result
}

pub fn restart(path: &Path) {
    let result = std::env::current_exe().and_then(|exe| {
        Command::new(exe).args(restart_args(std::env::args().skip(1), path)).spawn()
    });
    if let Err(e) = result { report_error(&format!("Impossible de redémarrer : {e}")); }
}
