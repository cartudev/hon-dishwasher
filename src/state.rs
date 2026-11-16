//! Estado de la app: pantalla activa, programa elegido, media carga y los campos de texto.

use rune::{AppHandle, TextEdit, Tx};

use crate::model::{ProfileInfo, Snapshot};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Profile,
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
    /// The chosen program's id (from `config/programs.json`).
    pub selected: String,
    pub basket: Basket,
    pub busy: bool,
    pub status: String,
    pub snapshot: Option<Snapshot>,
    pub confirmation: TextEdit,
    pub email: TextEdit,
    pub password: TextEdit,
    /// Remaining minutes from the last query, stamped on the local clock by `poll`.
    pub remaining_min: Option<u32>,
    /// An un-stamped time datum (poll assigns `landed_at` on its next tick).
    pub remaining_new: bool,
    pub landed_at: f64,
    pub countdown: String,
    /// A start is in flight: the window is hidden and the tray icon spins until hOn answers.
    pub sending: bool,
    pub spin_frame: u8,
    pub spin_at: f64,
    /// Minutes from the program card, kept if the follow-up query has no clock yet.
    pub held_estimate: Option<u32>,
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
            sending: false,
            spin_frame: 0,
            spin_at: 0.0,
            held_estimate: None,
        }
    }

    /// The appliance reports an active cycle.
    pub fn running(&self) -> bool {
        self.snapshot.as_ref().is_some_and(|s| s.running)
    }

    /// Remaining minutes from the last query, already parsed.
    pub fn remaining_minutes(&self) -> Option<u32> {
        self.snapshot
            .as_ref()
            .and_then(|s| s.remaining_time.as_deref())
            .and_then(|value| value.trim().parse::<u32>().ok())
    }

    /// Cycles visible on the home screen (backend order, hidden ones filtered out).
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

    /// When a query lands, the chosen program must exist; otherwise the first one.
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

/// Natural height per screen (460 wide): kept as the initial window size only;
/// live sizing comes from the framework's fit (see the Wayland patch in patches/).

pub fn screen_size(screen: Screen) -> (f32, f32) {
    let height = match screen {
        Screen::Home => 320.0,
        Screen::Profile => 390.0,
        Screen::Config => 450.0,
        Screen::Stop => 340.0,
    };
    (460.0, height)
}

/// Switches screens; the height follows the framework's `fit_content_height`.
/// `instant` lands the change and its height fit without a spring: during the
/// transition the drawn layout and the hit test disagreed by a frame and clicks
/// landed displaced ("above") right when entering program/settings.
pub fn go(s: &mut State, screen: Screen, tx: &mut Tx) {
    s.screen = screen;
    tx.instant();
}

/// Reopened from the tray, the framework re-fits the height to the content
/// (`build_for_show` of `fit_content_height`); nothing to do from here.


pub fn reapply_size(_s: &State) {}
