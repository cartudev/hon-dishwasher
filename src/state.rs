//! Estado de la app: pantalla activa, programa elegido, media carga y los campos de texto.

use rune::{AppHandle, TextEdit, Tx};

use crate::model::{ProfileInfo, Snapshot};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Profile,
    Review,
    Config,
    Stop,
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
    /// Minutos restantes de la última consulta, sellados en el reloj local por `poll`.
    pub remaining_min: Option<u32>,
    /// Hay un dato de tiempo sin sellar (poll asigna `landed_at` en su próximo tick).
    pub remaining_new: bool,
    pub landed_at: f64,
    pub countdown: String,
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
            remaining_min: None,
            remaining_new: false,
            landed_at: 0.0,
            countdown: String::new(),
        }
    }

    /// El aparato reporta un ciclo activo.
    pub fn running(&self) -> bool {
        self.snapshot.as_ref().is_some_and(|s| s.running)
    }

    /// Minutos restantes de la última consulta, ya parseados.
    pub fn remaining_minutes(&self) -> Option<u32> {
        self.snapshot
            .as_ref()
            .and_then(|s| s.remaining_time.as_deref())
            .and_then(|value| value.trim().parse::<u32>().ok())
    }

    /// Ciclos visibles en el inicio (orden del backend, filtrando los ocultos).
    pub fn visible_order(&self) -> Vec<String> {
        let hidden = self.snapshot.as_ref().map(|s| s.hidden.clone()).unwrap_or_default();
        self.program_order()
            .into_iter()
            .filter(|id| !hidden.iter().any(|h| h == id))
            .collect()
    }

    pub fn program_visible(&self, id: &str) -> bool {
        self.snapshot
            .as_ref()
            .is_some_and(|s| !s.hidden.iter().any(|h| h == id))
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

/// Altura natural de cada pantalla (ancho 460): el resize del framework no aplica en
/// Wayland (winit rechaza `request_inner_size` con los estados del compositor), así que
/// el tamaño lo fija niri por IPC sobre nuestra ventana.
pub fn screen_size(screen: Screen) -> (f32, f32) {
    let height = match screen {
        Screen::Home => 320.0,
        Screen::Profile => 390.0,
        Screen::Review => 430.0,
        Screen::Config => 450.0,
        Screen::Stop => 340.0,
    };
    (460.0, height)
}

/// Cambia de pantalla; el alto lo sigue `fit_content_height` del framework.
pub fn go(s: &mut State, screen: Screen, _tx: &mut Tx) {
    s.screen = screen;
    if screen == Screen::Review {
        s.confirmation.clear();
    }
}

/// Fija la altura de nuestra ventana en niri (busca el id por PID; reintenta porque
/// la ventana puede estar mapeándose todavía). Sin niri, queda en silencio.
/// Reabierta la ventana desde la bandeja, el framework re-ajusta el alto al contenido
/// (build_for_show de `fit_content_height`); nada que hacer desde acá.
pub fn reapply_size(_s: &State) {}
