//! Tray widget for a Candy CF 6C4F1PS-12 via hOn.
//! A tray click opens the window; the next click hides it. Starting a cycle hides
//! the window and spins the tray icon until hOn answers.

mod backend;
mod model;
mod state;
mod ui;

use rune::tray::{TrayEvent, TrayIcon, TrayItem, TraySpec};
use rune::{App, AppSpec, Key, KeyEvent, TextEdit, Tx};
use state::{Basket, Screen, State};
use std::sync::Mutex;
use ui::view;

const ICON: u32 = 64;
static TRAY: Mutex<Option<rune::Tray>> = Mutex::new(None);

fn base_pixels() -> &'static [u8] {
    include_bytes!("../assets/icon.rgba")
}

fn scaled_icon() -> Vec<u8> {
    let src = base_pixels();
    let mut out = vec![0u8; (ICON * ICON * 4) as usize];
    for y in 0..32 {
        for x in 0..32 {
            let s = (y * 32 + x) * 4;
            for dy in 0..2 {
                for dx in 0..2 {
                    let d = ((y * 2 + dy) * ICON as usize + x * 2 + dx) * 4;
                    out[d..d + 4].copy_from_slice(&src[s..s + 4]);
                }
            }
        }
    }
    out
}

fn idle_icon() -> TrayIcon {
    TrayIcon::rgba(ICON, ICON, scaled_icon())
}

/// A small arc in a corner disc, not dots painted over the dishwasher.
fn spin_icon(frame: u8) -> TrayIcon {
    let mut pixels = scaled_icon();
    let (cx, cy, radius) = (48.0, 48.0, 10.5);
    for y in 34..64 {
        for x in 34..64 {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance > 13.2 {
                continue;
            }
            let i = (y * ICON as usize + x) * 4;
            pixels[i] = 14;
            pixels[i + 1] = 15;
            pixels[i + 2] = 19;
            pixels[i + 3] = 255;
        }
    }
    let start = frame as f32 / 8.0 * std::f32::consts::TAU;
    let sweep = 1.7;
    for y in 34..64 {
        for x in 34..64 {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            let distance = (dx * dx + dy * dy).sqrt();
            let ring = (distance - radius).abs();
            if ring > 1.8 {
                continue;
            }
            let mut delta = dy.atan2(dx) - start;
            delta = delta.rem_euclid(std::f32::consts::TAU);
            if delta > sweep {
                continue;
            }
            let cover = ((1.8 - ring) / 1.8).clamp(0.0, 1.0) * (1.0 - delta / sweep * 0.25);
            let i = (y * ICON as usize + x) * 4;
            pixels[i] = (255.0 * cover + pixels[i] as f32 * (1.0 - cover)) as u8;
            pixels[i + 1] = (186.0 * cover + pixels[i + 1] as f32 * (1.0 - cover)) as u8;
            pixels[i + 2] = (92.0 * cover + pixels[i + 2] as f32 * (1.0 - cover)) as u8;
            pixels[i + 3] = 255;
        }
    }
    TrayIcon::rgba(ICON, ICON, pixels)
}

fn with_tray(f: impl FnOnce(&rune::Tray)) {
    if let Ok(guard) = TRAY.lock() {
        if let Some(tray) = guard.as_ref() {
            f(tray);
        }
    }
}

fn show_spin(frame: u8) {
    with_tray(|tray| {
        tray.set_icon(spin_icon(frame));
        tray.set_tooltip("Enviando el programa…");
    });
}

fn show_idle() {
    with_tray(|tray| {
        tray.set_icon(idle_icon());
        tray.set_tooltip("Candy — Lavavajillas · hOn");
    });
}

/// Last query's cache + live query. The list shows instantly from the cache and
/// the fresh state replaces it afterwards.
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
                Ok(mut snapshot) => {
                    let reported = snapshot
                        .remaining_time
                        .as_deref()
                        .and_then(|value| value.trim().parse::<u32>().ok())
                        .filter(|minutes| *minutes > 0);
                    if reported.is_none() {
                        if let Some(held) = s.held_estimate {
                            snapshot.remaining_time = Some(held.to_string());
                            snapshot.running = true;
                        }
                    } else {
                        s.held_estimate = None;
                    }
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
/// left, discounted by the clock's passage (no live sync with hOn; the Refresh button
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
        // A wake once a minute keeps the countdown going while the window sits idle.
        tx.wake_at(clock + 30.0);
    }
    if s.sending {
        if s.spin_at == 0.0 {
            s.spin_at = clock;
        } else if clock - s.spin_at >= 0.08 {
            s.spin_frame = (s.spin_frame + 1) % 8;
            s.spin_at = clock;
            show_spin(s.spin_frame);
        }
        tx.wake_at(clock + 0.08);
    }
    dirty
}

/// Persists a remembered choice (option ON/OFF or cycle visibility) in the
/// backend and mutates the local snapshot at once: the UI never waits on the subprocess.
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

/// Stops the running cycle (background task; the UI never waits on the network).
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
    if s.busy {
        return;
    }
    let Some(info) = s.selected_info() else { return };
    if !info.available {
        return;
    }
    let label = info.label.clone();
    let program = s.selected.clone();
    s.busy = true;
    s.sending = true;
    s.spin_frame = 0;
    s.spin_at = 0.0;
    s.status = format!("Enviando el inicio de {label}…");
    s.me.hide();
    show_spin(0);
    s.me.wake();
    let me = s.me.clone();
    let basket = s.basket;
    let _ = rune::task::spawn("hon-start", move || {
        let result = backend::call(&["start", &program, basket.as_str(), "INICIAR"], None);
        let refresh = me.clone();
        me.update(move |s, tx| {
            s.busy = false;
            s.sending = false;
            s.confirmation.clear();
            show_idle();
            match result {
                Ok(payload) => {
                    let minutes = payload
                        .get("estimated_minutes")
                        .and_then(|value| value.as_u64().map(|n| n as u32).or_else(|| value.as_str().and_then(|text| text.parse().ok())))
                        .filter(|minutes| *minutes > 0);
                    if let Some(minutes) = minutes {
                        s.held_estimate = Some(minutes);
                        if let Some(snapshot) = s.snapshot.as_mut() {
                            snapshot.running = true;
                            snapshot.program = Some(label.clone());
                            snapshot.remaining_time = Some(minutes.to_string());
                        }
                        s.remaining_new = true;
                        s.countdown = format!("Quedan {minutes} min");
                        with_tray(|tray| tray.set_tooltip(format!("{label} · quedan {minutes} min")));
                        s.status = format!("hOn aceptó {label}. Quedan {minutes} min.");
                    } else {
                        s.status = format!("hOn aceptó el inicio de {label}.");
                    }
                    state::go(s, Screen::Home, tx);
                    s.basket = Basket::Off;
                    inspect(refresh, false);
                }
                Err(error) => {
                    s.status = error;
                    s.me.open();
                }
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
        // Stdin only: never argv, never disk.
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

/// Escape goes back; from home it hides the window to the tray.
fn on_key(s: &mut State, event: &KeyEvent, tx: &mut Tx) -> bool {
    if event.key != Key::Escape {
        return false;
    }
    match s.screen {
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

/// Tray icon: a click opens the window; the menu offers Open, Refresh, Settings and Quit.
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
            if click.is_visible() {
                click.hide();
            } else {
                match token {
                    Some(token) if !token.is_empty() => click.activate(token),
                    _ => click.open(),
                }
                click.update(|s, _| state::reapply_size(s));
            }
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
                s.email = TextEdit::new("tu@correo.com");
                s.password = TextEdit::new("secreta123");
            }),
            ("profile", |s| {
                s.snapshot = Some(model::sample());
                s.screen = Screen::Profile;
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
    // Widget flotante: el gestor de ventanas (niri incluido) la hace flotar y el alto
    // follows each screen's content (the framework's fit, via winit).
    .float_on_open()
    .fit_content_height()
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
    if let Ok(mut slot) = TRAY.lock() {
        *slot = Some(build_tray(boot.clone()));
    }
    if !headless {
        inspect(boot, true);
    }
    app.run();
}
