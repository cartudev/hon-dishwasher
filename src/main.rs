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

/// Caché de la última consulta + consulta en vivo. La lista aparece al instante con la
/// caché y despues el estado fresco la reemplaza.
fn inspect(me: rune::AppHandle<State>, with_cache: bool) {
    let _ = rune::task::spawn("hon-inspect", move || {
        if with_cache {
            let cached = backend::call(&["cached"], None).and_then(|value| {
                serde_json::from_value::<model::Snapshot>(value)
                    .map_err(|_| "El backend respondió en un formato inesperado.".to_string())
            });
            let snapshot = cached.into_iter().find(|s| s.ok && s.cached);
            if let Some(snapshot) = snapshot {
                me.update(move |s, tx| {
                    if s.snapshot.is_none() {
                        s.snapshot = Some(snapshot);
                        s.remaining_new = true;
                        s.status = "Datos de la última consulta; actualizando en vivo…".to_string();
                        s.reconcile_selection();
                        tx.instant();
                    }
                });
            }
        }
        let result = backend::call(&["inspect"], None).and_then(|value| {
            serde_json::from_value::<model::Snapshot>(value)
                .map_err(|_| "El backend respondió en un formato inesperado.".to_string())
        });
        me.update(move |s, tx| {
            s.busy = false;
            match result {
                Ok(snapshot) => {
                    s.snapshot = Some(snapshot);
                    s.remaining_new = true;
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

/// Conteo local: sella el reloj cuando aterriza un dato de tiempo y muestra los minutos
/// que quedan, restando el paso del reloj (sin sincronía con hOn; el botón Actualizar
/// trae un valor fresco).
fn poll(s: &mut State, clock: f64, tx: &mut Tx) -> bool {
    let mut dirty = false;
    if s.remaining_new {
        s.landed_at = clock;
        s.remaining_new = false;
        dirty = true;
    }
    if s.running() {
        if let Some(minutes) = s.remaining_minutes() {
            let left = (minutes as f64 * 60.0 - (clock - s.landed_at)).max(0.0);
            let text = format!("Quedan {} min", (left / 60.0).ceil() as u32);
            if text != s.countdown {
                s.countdown = text;
                dirty = true;
            }
        }
        // Un despertar por minuto mantiene el conteo aunque la ventana esté quieta.
        tx.wake_at(clock + 30.0);
    }
    dirty
}

/// Guarda una elección recordada (opción ON/OFF o visibilidad de un ciclo) en el
/// backend y muta el snapshot local al instante: la UI no espera al subprocess.
fn apply_choice(s: &mut State, program: String, key: String, on: bool) {
    let Some(snapshot) = s.snapshot.as_mut() else { return };
    let Some(info) = snapshot.profiles.get_mut(&program) else { return };
    if let Some(option) = info.required.iter_mut().find(|o| o.key == key) {
        option.enabled = on;
    }
    let me = s.me.clone();
    rune::task::spawn("hon-choice", move || {
        let _ = backend::call(&["choice", "set", &program, &key, if on { "1" } else { "0" }], None);
        let _ = me.wake();
    })
    .ok();
}

fn apply_visible(s: &mut State, program: String, visible: bool) {
    let Some(snapshot) = s.snapshot.as_mut() else { return };
    if visible {
        snapshot.hidden.retain(|id| id != &program);
    } else if !snapshot.hidden.iter().any(|h| h == &program) {
        snapshot.hidden.push(program.clone());
    }
    let me = s.me.clone();
    rune::task::spawn("hon-visible", move || {
        let _ = backend::call(&["visible", "set", &program, if visible { "1" } else { "0" }], None);
        let _ = me.wake();
    })
    .ok();
}

/// Detiene el ciclo en curso (tarea de fondo; el UI jamás espera la red).
fn stop_cycle(s: &mut State) {
    if s.busy || s.confirmation.text.trim() != "DETENER" {
        return;
    }
    s.busy = true;
    s.status = "Preparando la detención…".to_string();
    let me = s.me.clone();
    rune::task::spawn("hon-stop", move || {
        let result = backend::call(&["stop", "DETENER"], None);
        let refresh = me.clone();
        me.update(move |s, tx| {
            s.busy = false;
            s.confirmation.clear();
            match result {
                Ok(_) => {
                    s.status = "hOn aceptó la detención del ciclo.".to_string();
                    state::go(s, Screen::Home, tx);
                    inspect(refresh, false);
                }
                Err(error) => s.status = error,
            }
        });
    })
    .ok();
}

fn refresh(s: &mut State) {
    if s.busy {
        return;
    }
    s.busy = true;
    s.status = "Consultando el aparato (solo lectura)…".to_string();
    inspect(s.me.clone(), false);
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
        me.update(move |s, tx| {
            s.busy = false;
            s.confirmation.clear();
            match result {
                Ok(_) => {
                    s.status = format!("hOn aceptó el inicio de {label}.");
                    state::go(s, Screen::Home, tx);
                    s.basket = Basket::Off;
                    inspect(refresh, false);
                }
                Err(error) => s.status = error,
            }
        });
    });
}

fn save_credentials(s: &mut State) {
    if s.busy {
        return;
    }
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
        me.update(move |s, tx| {
            s.busy = false;
            s.password = TextEdit::new("");
            match result {
                Ok(_) => {
                    s.status = "Credenciales guardadas en el keyring del sistema.".to_string();
                    state::go(s, Screen::Home, tx);
                    inspect(refresh, false);
                }
                Err(error) => s.status = error,
            }
        });
    })
    .ok();
}

/// Escape vuelve atrás; la revisión descarta lo escrito para no dejar un INICIAR listo.
fn on_key(s: &mut State, event: &KeyEvent, tx: &mut Tx) -> bool {
    if event.key != Key::Escape {
        return false;
    }
    // Escape vuelve atrás; desde el inicio, esconde la ventana al tray (la app sigue viva).
    match s.screen {
        Screen::Review => {
            state::go(s, Screen::Profile, tx);
            s.confirmation.clear();
            true
        }
        Screen::Stop => {
            state::go(s, Screen::Home, tx);
            s.confirmation.clear();
            true
        }
        Screen::Profile | Screen::Config => {
            state::go(s, Screen::Home, tx);
            true
        }
        Screen::Home => {
            s.me.hide();
            true
        }
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
        TrayEvent::Activate { token } => {
            match token {
                Some(token) if !token.is_empty() => click.activate(token),
                _ => click.open(),
            }
            // Tras reabrir (o enfocar), la altura vuelve a la de la pantalla activa.
            click.update(|s, _| state::reapply_size(s));
        }
        TrayEvent::Menu(id) => match id.as_str() {
            "open" => {
                menu.open();
                menu.update(|s, _| state::reapply_size(s));
            }
            "refresh" => {
                menu.open();
                menu.update(|s, _| crate::refresh(s));
            }
            "config" => {
                menu.open();
                menu.update(|s, tx| state::go(s, Screen::Config, tx));
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
    // El app_id deja que el gestor de ventanas (niri) le aplique reglas: flotar y
    // abrir cerca de la bandeja (`default-floating-position` en el config.kdl).
    rune::set_app_id("hon-dishwasher");
    let app = App::new(AppSpec::<State> {
        scenes: &[
            ("config", |s| {
                s.screen = Screen::Config;
                s.busy = false;
                s.status = "Cuenta hOn".to_string();
            }),
            ("config-typed", |s| {
                s.snapshot = Some(model::sample());
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
            ("cycle", |s| {
                s.snapshot = Some(model::sample_cycle());
                s.status = "Equipo consultado · solo lectura".to_string();
                s.busy = false;
                s.reconcile_selection();
            }),
            ("stop", |s| {
                s.snapshot = Some(model::sample_cycle());
                s.busy = false;
                s.status = "Equipo consultado · solo lectura".to_string();
                s.remaining_new = true;
                s.reconcile_selection();
                s.screen = Screen::Stop;
                s.confirmation.clear();
            }),
        ],
        on_key: Some(on_key),
        poll: Some(poll),
        ..AppSpec::new("Candy — Lavavajillas", state::screen_size(Screen::Home), || unreachable!(), view)
    })
    // La ventana vive aparte del tray: cerrarla no apaga la app.
    .keep_running()
    // Widget flotante: el gestor de ventanas (niri incluido) la hace flotar. El tamaño
    // lo pide la app al cambiar de pantalla (`state::go`); el usuario puede redimensionar.
    .float_on_open()
    .window_options(rune::WindowOptions {
        min_size: Some((380.0, 0.0)),
        max_size: Some((560.0, 760.0)),
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
        inspect(boot, true);
    }
    app.run();
}
