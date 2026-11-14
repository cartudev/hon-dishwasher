//! App Rune nativa (widget flotante con icono en la bandeja) para el lavavajillas
//! Candy CF 6C4F1PS-12 vía hOn. Ninguna operación se envía sin la confirmación
//! explícita "INICIAR" en la pantalla de revisión.

mod backend;
mod model;
mod state;
mod ui;

use rune::tray::{TrayEvent, TrayIcon, TrayItem, TraySpec};
use rune::{App, AppSpec, Key, KeyEvent, TextEdit, Tx};
use state::{Basket, Screen, State};
use ui::view;

/// Consulta de solo lectura, ya en un hilo de tareas con el resultado aplicado al estado.
fn inspect(me: rune::AppHandle<State>) {
    let _ = rune::task::spawn("hon-inspect", move || {
        let result = backend::call(&["inspect"], None).and_then(|value| {
            serde_json::from_value::<model::Snapshot>(value)
                .map_err(|_| "El backend respondió en un formato inesperado.".to_string())
        });
        me.update(move |s, tx| {
            s.busy = false;
            match result {
                Ok(snapshot) => {
                    s.snapshot = Some(snapshot);
                    s.status = "Equipo consultado · solo lectura".to_string();
                    s.reconcile_selection();
                }
                Err(error) => {
                    s.snapshot = None;
                    s.status = error;
                }
            }
            tx.instant();
        });
    });
}

fn refresh(s: &mut State) {
    if s.busy {
        return;
    }
    s.busy = true;
    s.status = "Consultando el aparato (solo lectura)…".to_string();
    inspect(s.me.clone());
}

fn start_cycle(s: &mut State) {
    if s.busy || s.confirmation.text.trim() != "INICIAR" {
        return;
    }
    let Some(info) = s.selected_info() else { return };
    let label = info.label.clone();
    let program = s.selected.clone();
    s.busy = true;
    s.status = "Revalidando opciones y preparando el inicio…".to_string();
    let me = s.me.clone();
    let basket = s.basket;
    let _ = rune::task::spawn("hon-start", move || {
        let result = backend::call(&["start", &program, basket.as_str(), "INICIAR"], None);
        let refresh = me.clone();
        me.update(move |s, _| {
            s.busy = false;
            s.confirmation.clear();
            match result {
                Ok(_) => {
                    s.status = format!("hOn aceptó el inicio de {label}.");
                    s.screen = Screen::Home;
                    s.basket = Basket::Off;
                    inspect(refresh);
                }
                Err(error) => s.status = error,
            }
        });
    });
}

fn save_credentials(s: &mut State) {
    let email = s.email.text.trim().to_string();
    let password = s.password.text.clone();
    if !email.contains('@') || password.is_empty() {
        s.status = "Completá un correo y una contraseña antes de guardar.".to_string();
        return;
    }
    s.busy = true;
    s.status = "Guardando en el keyring del sistema…".to_string();
    let me = s.me.clone();
    rune::task::spawn("hon-store", move || {
        // Solo por stdin: nunca en argv, nunca en disco.
        let payload = serde_json::json!({ "email": email, "password": password });
        let result = backend::call(&["store"], Some(payload.to_string()));
        let refresh = me.clone();
        me.update(move |s, _| {
            s.busy = false;
            s.password = TextEdit::new("");
            match result {
                Ok(_) => {
                    s.status = "Credenciales guardadas en el keyring del sistema.".to_string();
                    s.screen = Screen::Home;
                    inspect(refresh);
                }
                Err(error) => s.status = error,
            }
        });
    })
    .ok();
}

/// Escape vuelve atrás; la revisión descarta lo escrito para no dejar un INICIAR listo.
fn on_key(s: &mut State, event: &KeyEvent, _: &mut Tx) -> bool {
    if event.key != Key::Escape {
        return false;
    }
    match s.screen {
        Screen::Review => {
            s.screen = Screen::Profile;
            s.confirmation.clear();
            true
        }
        Screen::Profile | Screen::Config => {
            s.screen = Screen::Home;
            true
        }
        Screen::Home => false,
    }
}

/// Icono en la barra: clic abre la ventana; el menú da Abrir, Actualizar, Configuración y Salir.
fn build_tray(handle: rune::AppHandle<State>) -> rune::Tray {
    let pixels = include_bytes!("../assets/icon.rgba").to_vec();
    let spec = TraySpec::new(
        "hon-dishwasher",
        "Candy — Lavavajillas",
        TrayIcon::rgba(32, 32, pixels),
    )
    .tooltip("Candy — Lavavajillas · hOn")
    .menu(vec![
        TrayItem::action("open", "Abrir"),
        TrayItem::action("refresh", "Actualizar"),
        TrayItem::action("config", "Configuración"),
        TrayItem::Separator,
        TrayItem::action("quit", "Salir"),
    ]);
    let click = handle.clone();
    let menu = handle.clone();
    rune::Tray::new(spec, move |event| match event {
        TrayEvent::Activate { token } => match token {
            Some(token) if !token.is_empty() => click.activate(token),
            _ => click.open(),
        },
        TrayEvent::Menu(id) => match id.as_str() {
            "open" => menu.open(),
            "refresh" => {
                menu.open();
                menu.update(|s, _| crate::refresh(s));
            }
            "config" => {
                menu.open();
                menu.update(|s, _| s.screen = Screen::Config);
            }
            "quit" => menu.quit(),
            _ => {}
        },
        // `TrayEvent` es non_exhaustive: Rune puede sumar variantes.
        _ => {}
    })
    .expect("system tray")
}

fn main() {
    let app = App::new(AppSpec::<State> {
        scenes: &[
            ("config", |s| {
                s.screen = Screen::Config;
                s.busy = false;
                s.status = "Cuenta hOn".to_string();
            }),
            ("config-typed", |s| {
                s.screen = Screen::Config;
                s.busy = false;
                s.status = "Cuenta hOn".to_string();
                s.email = TextEdit::new("cartu@ejemplo.com");
                s.password = TextEdit::new("secreta123");
            }),
            ("profile", |s| {
                s.snapshot = Some(model::sample());
                s.screen = Screen::Profile;
                s.busy = false;
                s.status = "Equipo consultado · solo lectura".to_string();
                s.reconcile_selection();
            }),
            ("review", |s| {
                s.snapshot = Some(model::sample());
                s.screen = Screen::Review;
                s.busy = false;
                s.status = "Equipo consultado · solo lectura".to_string();
                s.reconcile_selection();
            }),
            ("sample", |s| {
                s.snapshot = Some(model::sample());
                s.status = "Equipo consultado · solo lectura".to_string();
                s.busy = false;
                s.reconcile_selection();
            }),
        ],
        on_key: Some(on_key),
        ..AppSpec::new("Candy — Lavavajillas", (400.0, 160.0), || unreachable!(), view)
    })
    // La ventana vive aparte del tray: cerrarla no apaga la app.
    .keep_running()
    // Widget flotante: el gestor de ventanas (niri incluido) la hace flotar, la altura
    // sigue al contenido y el tamaño queda acotado.
    .float_on_open()
    .fit_content_height()
    .window_options(rune::WindowOptions {
        min_size: Some((320.0, 0.0)),
        max_size: Some((460.0, 720.0)),
        ..Default::default()
    });
    let handle = app.handle();
    let mut app = app.init(move || State::new(handle.clone()));
    // Tooling headless (--snap/--tree/...): escena pura, sin consulta real ni ventana.
    let headless = std::env::args()
        .any(|argument| matches!(argument.as_str(), "--snap" | "--tree" | "--scene" | "--script" | "--video" | "--playground"));
    // Arranque como widget: sin ventana salvo --open; el icono de la bandeja la abre.
    if !headless && !std::env::args().any(|argument| argument == "--open") {
        app = app.windowless();
    }
    let boot = app.handle();
    let _tray = build_tray(boot.clone());
    if !headless {
        inspect(boot);
    }
    app.run();
}
