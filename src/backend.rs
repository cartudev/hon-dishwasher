//! Puente con el backend Python (`src/backend.py`, pyhOn). Un proceso por operación, JSON por stdout.
//! La contraseña nunca va por argv: la operación `store` la recibe por stdin.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::Value;

const UNEXPECTED: &str = "El backend respondió en un formato inesperado.";
const MISSING: &str = "Faltan dependencias. Desde la carpeta del proyecto ejecutá ./setup.sh.";

fn locate() -> Option<(PathBuf, PathBuf)> {
    let mut dirs = Vec::new();
    // target/debug/<exe> → tres ancestros arriba está la raíz del proyecto.
    if let Ok(exe) = std::env::current_exe()
        && let Some(root) = exe.ancestors().nth(3)
    {
        dirs.push(root.to_path_buf());
    }
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd);
    }
    dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    for dir in dirs {
        let script = dir.join("src/backend.py");
        if !script.is_file() {
            continue;
        }
        let python = match std::env::var_os("HON_DISHWASHER_PYTHON") {
            Some(value) => PathBuf::from(value),
            None => dir.join(".venv/bin/python"),
        };
        return Some((python, script));
    }
    None
}

/// Llama al backend y devuelve su JSON `ok: true`; cualquier otra cosa es un error
/// con el mensaje seguro que el backend emite (sin datos del aparato ni de la cuenta).
pub fn call(args: &[&str], stdin_data: Option<String>) -> Result<Value, String> {
    let Some((python, script)) = locate() else {
        return Err(MISSING.to_string());
    };
    let mut command = Command::new(&python);
    command
        .arg(&script)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if stdin_data.is_some() {
        command.stdin(Stdio::piped());
    } else {
        command.stdin(Stdio::null());
    }
    let mut child = command.spawn().map_err(|_| "No se pudo ejecutar el backend Python.".to_string())?;
    if let Some(data) = stdin_data
        && let Some(mut pipe) = child.stdin.take()
    {
        let _ = pipe.write_all(data.as_bytes());
    }
    let output = child.wait_with_output().map_err(|_| "El backend no terminó correctamente.".to_string())?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let value: Value = serde_json::from_str(&text).map_err(|_| UNEXPECTED.to_string())?;
    if value.get("ok").and_then(Value::as_bool) != Some(true) {
        let message = value
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("No se pudo completar la operación con hOn.");
        return Err(message.to_string());
    }
    Ok(value)
}
