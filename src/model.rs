//! Lo que el backend Python (pyhOn) responde, en tipos Rust.

use std::collections::HashMap;

use serde::Deserialize;

#[derive(Deserialize, Clone, Default)]
pub struct Snapshot {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub remote_ready: bool,
    #[serde(default)]
    pub remote: HashMap<String, bool>,
    #[serde(default)]
    pub door_status: Option<String>,
    #[serde(default)]
    pub machine_mode: Option<String>,
    /// Orden de los programas según la configuración (`config/programs.json`).
    #[serde(rename = "profileOrder", default)]
    pub profile_order: Vec<String>,
    #[serde(default)]
    pub profiles: HashMap<String, ProfileInfo>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
}

#[derive(Deserialize, Clone, Default)]
pub struct ProfileInfo {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub available: bool,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub reason: String,
    /// Opciones requeridas, en el orden de la configuración, con sus rótulos.
    #[serde(default)]
    pub required: Vec<RequiredOption>,
    #[serde(default)]
    pub basket: BasketInfo,
}

#[derive(Deserialize, Clone, Default)]
pub struct RequiredOption {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub supported: bool,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub reason: String,
}

#[derive(Deserialize, Clone, Default)]
pub struct BasketInfo {
    #[serde(default)]
    pub available: bool,
    #[serde(default)]
    pub choices: Vec<String>,
    #[serde(default)]
    pub reason: String,
}

impl Snapshot {
    pub fn profile(&self, id: &str) -> Option<&ProfileInfo> {
        self.profiles.get(id)
    }
}

/// Datos de ejemplo para las escenas headless (nunca de un aparato real).
pub fn sample() -> Snapshot {
    let required = |labels: &[&str]| {
        labels
            .iter()
            .map(|label| RequiredOption {
                key: label.to_string(),
                label: (*label).to_string(),
                supported: true,
                pinned: false,
                reason: String::new(),
            })
            .collect::<Vec<_>>()
    };
    let profiles = HashMap::from([
        (
            "intensive".to_string(),
            ProfileInfo {
                id: "intensive".to_string(),
                label: "Intensivo".to_string(),
                available: true,
                code: Some("7".to_string()),
                reason: String::new(),
                required: required(&["Intensivo+", "Extra Dry", "Smart Door"]),
                basket: BasketInfo {
                    available: true,
                    choices: vec!["off".into(), "upper".into(), "lower".into()],
                    reason: String::new(),
                },
            },
        ),
        (
            "prewash".to_string(),
            ProfileInfo {
                id: "prewash".to_string(),
                label: "Prelavado".to_string(),
                available: true,
                code: Some("1".to_string()),
                reason: String::new(),
                required: required(&["Smart Door"]),
                basket: BasketInfo {
                    available: false,
                    choices: vec![],
                    reason: "Canasta superior/inferior no confirmada para este programa.".to_string(),
                },
            },
        ),
    ]);
    Snapshot {
        ok: true,
        model: "CF 6C4F1PS-12".to_string(),
        remote_ready: true,
        remote: HashMap::from([
            ("remoteCtrValid".to_string(), true),
            ("remoteActionable".to_string(), true),
            ("remoteVisible".to_string(), true),
        ]),
        door_status: Some("0".to_string()),
        machine_mode: Some("1".to_string()),
        profile_order: vec!["intensive".to_string(), "prewash".to_string()],
        profiles,
        error: None,
        code: None,
    }
}
