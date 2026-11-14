//! Estado de la app: pantalla activa, programa elegido, media carga y los campos de texto.

use rune::{AppHandle, TextEdit};

use crate::model::{ProfileInfo, Snapshot};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Profile,
    Review,
    Config,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Basket {
    Off,
    Upper,
    Lower,
}

impl Basket {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Upper => "upper",
            Self::Lower => "lower",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Sin media carga",
            Self::Upper => "Media carga superior",
            Self::Lower => "Media carga inferior",
        }
    }
}

pub struct State {
    pub me: AppHandle<State>,
    pub screen: Screen,
    /// Id del programa elegido (viene de `config/programs.json`).
    pub selected: String,
    pub basket: Basket,
    pub busy: bool,
    pub status: String,
    pub snapshot: Option<Snapshot>,
    pub confirmation: TextEdit,
    pub email: TextEdit,
    pub password: TextEdit,
}

impl State {
    pub fn new(me: AppHandle<State>) -> Self {
        Self {
            me,
            screen: Screen::Home,
            selected: String::new(),
            basket: Basket::Off,
            busy: true,
            status: "Conectando con hOn…".to_string(),
            snapshot: None,
            confirmation: TextEdit::new(""),
            email: TextEdit::new(""),
            password: TextEdit::new(""),
        }
    }

    pub fn snapshot_ready(&self) -> bool {
        self.snapshot.as_ref().is_some_and(|s| s.ok)
    }

    pub fn program_order(&self) -> Vec<String> {
        self.snapshot
            .as_ref()
            .map(|s| s.profile_order.clone())
            .unwrap_or_default()
    }

    pub fn profile_info(&self, id: &str) -> Option<&ProfileInfo> {
        self.snapshot.as_ref().and_then(|s| s.profile(id))
    }

    pub fn selected_info(&self) -> Option<&ProfileInfo> {
        self.profile_info(&self.selected)
    }

    pub fn basket_allowed(&self) -> bool {
        self.selected_info().is_some_and(|i| i.basket.available)
    }

    pub fn open_profile(&mut self, id: String) {
        self.selected = id;
        self.basket = Basket::Off;
        self.confirmation.clear();
        self.screen = Screen::Profile;
    }

    /// Al aterrizar una consulta, el programa elegido debe existir; si no, el primero.
    pub fn reconcile_selection(&mut self) {
        let order = self.program_order();
        if !order.is_empty() && !order.iter().any(|id| *id == self.selected) {
            self.selected = order[0].clone();
        }
    }

    pub fn door_text(&self) -> &'static str {
        let Some(door) = self.snapshot.as_ref().and_then(|s| s.door_status.clone()) else {
            return "hOn no publica la puerta; comprobá que esté cerrada";
        };
        match door.trim() {
            "0" => "hOn reporta la puerta cerrada",
            "1" => "hOn reporta la puerta abierta (el envío se bloquea)",
            _ => "hOn no publica la puerta; comprobá que esté cerrada",
        }
    }
}
