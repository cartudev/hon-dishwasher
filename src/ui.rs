//! Las cuatro pantallas: inicio, programa, revisión/confirmación y configuración.

use rune::tok::{self, Theme, motion, radius, space, type_};
use rune::{Color, Cx, El, ScrollSpec, Tx, col, el, row, text, text_input};

use crate::state::{Basket, Screen, State};

pub fn view(s: &State, cx: &mut Cx<State>) -> El {
    let dark = cx.system_dark();
    let t = tok::theme(dark);
    let accent = tok::accent(0, dark);
    let model = s
        .snapshot
        .as_ref()
        .map(|x| x.model.clone())
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| "CF 6C4F1PS-12".to_string());
    col()
        .key("window")
        .w_full()
        .fill(t.bg)
        .px(space::S6)
        .py(space::S3)
        .gap(space::S3)
        .child(
            col()
                .key("header")
                .gap(2.0)
                .child(text("Candy · hOn").key("title").font(type_::TITLE).color(t.text))
                .child(text(model).key("model").font(type_::LABEL).color(t.text_secondary)),
        )
        .child(
            col()
                .key("body")
                .w_full()
                .shrink()
                .max_h(520.0)
                .scroll(ScrollSpec { stick_bottom: false })
                .child(match s.screen {
                    Screen::Home => home(s, t, accent),
                    Screen::Profile => profile_screen(s, t, accent),
                    Screen::Review => review(s, t, accent),
                    Screen::Config => config(s, t, accent),
                }),
        )
        .child(status_line(s, t, accent))
}

fn status_line(s: &State, t: Theme, accent: Color) -> El {
    row()
        .key("status")
        .gap(space::S2)
        .center_y()
        .child(
            text(if s.busy { "◌" } else { "●" })
                .key("dot")
                .font(type_::BODY)
                .color(if s.busy { accent } else { t.text_secondary }),
        )
        .child(text(s.status.clone()).key("status-text").font(type_::LABEL).color(t.text_secondary))
}

fn action(key: &'static str, label: &'static str, t: Theme, enabled: bool, primary: bool, accent: Color, f: impl Fn(&mut State, &mut Tx) + 'static) -> El {
    let on = enabled && primary;
    row()
        .key(key)
        .role("button")
        .label(label)
        .h(34.0)
        .px(space::S3)
        .center()
        .radius(radius::MD)
        .fill(if on { accent } else if enabled { t.card } else { t.control_off })
        .border(1.0, t.card_border)
        .hover(if enabled { t.hover } else { Color::TRANSPARENT })
        .on_press_tx(move |s: &mut State, tx| if !s.busy { f(s, tx) })
        .child(
            text(label)
                .key("label")
                .font(type_::BODY)
                .color(if on { Color::WHITE } else if enabled { t.text } else { t.text_secondary }),
        )
}

fn program_card(id: &str, s: &State, t: Theme, accent: Color) -> El {
    let ready = s.snapshot_ready();
    let info = s.profile_info(id);
    let available = info.is_some_and(|i| i.available);
    let label = info.map(|i| i.label.clone()).unwrap_or_else(|| id.to_string());
    let state_text = if !ready {
        "Requiere consulta"
    } else if available {
        "Disponible en el esquema"
    } else {
        "No disponible en el esquema"
    };
    let pressed = id.to_string();
    row()
        .key(format!("card-{id}"))
        .role("button")
        .label(label.clone())
        .w_full()
        .h(64.0)
        .px(space::S3)
        .center_y()
        .justify_between()
        .radius(radius::LG)
        .fill(t.card)
        .border(1.0, t.card_border)
        .hover(if ready { t.hover } else { Color::TRANSPARENT })
        .motion(motion::SNAPPY)
        .on_press_tx(move |st: &mut State, _| if st.snapshot_ready() { st.open_profile(pressed.clone()) })
        .child(text(label).key("name").font(type_::BODY).color(t.text))
        .child(
            text(state_text)
                .key("state")
                .font(type_::LABEL)
                .color(if available { accent } else { t.text_secondary }),
        )
}

fn home(s: &State, t: Theme, accent: Color) -> El {
    let ready = s.snapshot_ready();
    let mut programs = col().key("programs").gap(space::S2);
    for id in s.program_order() {
        programs = programs.child(program_card(&id, s, t, accent));
    }
    col()
        .key("home")
        .gap(space::S3)
        .child(text("Elegí un programa (no inicia nada)").key("hint").font(type_::SECTION).color(t.text_secondary))
        .child(if ready {
            programs
        } else {
            text("Consultando el aparato para listar los programas…")
                .key("programs-loading")
                .font(type_::BODY)
                .color(t.text_secondary)
        })
        .child(
            row()
                .key("home-actions")
                .gap(space::S2)
                .child(action("refresh", "Actualizar", t, true, false, accent, |s, _| crate::refresh(s)))
                .child(action("config", "Configuración", t, true, false, accent, |s, _| s.screen = Screen::Config))
                .child(action("quit", "Salir", t, true, false, accent, |_, tx| tx.quit())),
        )
}

fn field_shell(key: &'static str, input: El, t: Theme) -> El {
    el()
        .key(key)
        .w_full()
        .h(40.0)
        .px(space::S3)
        .radius(radius::MD)
        .fill(t.card)
        .border(1.0, t.card_border)
        .center_y()
        .child(input)
}

fn required_rows(id: &str, s: &State, t: Theme, accent: Color) -> El {
    let info = s.profile_info(id);
    let options = info.map(|i| i.required.as_slice()).unwrap_or(&[]);
    let mut list = col()
        .key("required")
        .gap(space::S1)
        .child(text("OPCIONES REQUERIDAS (SE ENVIÁN ACTIVADAS)").key("req-title").font(type_::SECTION).color(t.text_secondary));
    if options.is_empty() {
        list = list.child(
            text("Actualizá el equipo para ver las opciones.")
                .key("req-empty")
                .font(type_::BODY)
                .color(t.text_secondary),
        );
    }
    for option in options {
        let ok = option.supported;
        list = list.child(
            row()
                .key(option.key.clone())
                .w_full()
                .center_y()
                .justify_between()
                .child(text(option.label.clone()).key("name").font(type_::BODY).color(t.text))
                .child(
                    text(if ok { "ON" } else { "NO DISPONIBLE" })
                        .key("state")
                        .font(type_::MONO_CAPTION)
                        .color(if ok { accent } else { t.text_secondary }),
                ),
        );
    }
    list
}

fn basket_area(s: &State, t: Theme, accent: Color) -> El {
    let allowed = s.basket_allowed();
    let reason = s
        .selected_info()
        .map(|i| i.basket.reason.clone())
        .filter(|r| !r.is_empty())
        .unwrap_or_else(|| "Canasta superior/inferior no confirmada para este programa.".to_string());
    let options: [(Basket, &'static str, &'static str); 3] = [
        (Basket::Off, "Sin media carga", "opt-off"),
        (Basket::Upper, "Canasta superior", "opt-upper"),
        (Basket::Lower, "Canasta inferior", "opt-lower"),
    ];
    let mut pick = row().key("basket").gap(space::S2);
    for (value, label, key) in options {
        let on = s.basket == value;
        pick = pick.child(
            row()
                .key(key)
                .role("radio")
                .label(label)
                .checked(on && allowed)
                .focusable(allowed)
                .h(30.0)
                .px(space::S3)
                .center()
                .radius(radius::MD)
                .fill(if on && allowed { accent } else { t.card })
                .border(1.0, t.card_border)
                .hover(if allowed { t.hover } else { Color::TRANSPARENT })
                .on_press_tx(move |st: &mut State, _| if st.basket_allowed() { st.basket = value })
                .child(
                    text(label)
                        .key("label")
                        .font(type_::LABEL)
                        .color(if on && allowed { Color::WHITE } else { t.text_secondary }),
                ),
        );
    }
    let mut area = col()
        .key("basket-area")
        .gap(space::S2)
        .child(text("MEDIA CARGA (OPCIONAL)").key("basket-title").font(type_::SECTION).color(t.text_secondary))
        .child(pick);
    if !allowed {
        area = area.child(text(reason).key("basket-reason").w_full().wrap().font(type_::LABEL).color(t.text_secondary));
    }
    area
}

fn profile_screen(s: &State, t: Theme, accent: Color) -> El {
    let id = s.selected.clone();
    let info = s.profile_info(&id);
    let label = info.map(|i| i.label.clone()).unwrap_or_else(|| id.clone());
    let available = info.is_some_and(|i| i.available);
    let detail = match info.and_then(|i| i.code.clone()) {
        Some(code) if available => format!("Código hOn del programa: {code}"),
        _ => info
            .map(|i| i.reason.clone())
            .filter(|r| !r.is_empty())
            .unwrap_or_else(|| "Actualizá el equipo para ver el esquema actual.".to_string()),
    };
    col()
        .key("profile")
        .gap(space::S3)
        .child(
            col()
                .key("program-card")
                .padding(space::S3)
                .gap(space::S1)
                .fill(t.card)
                .radius(radius::LG)
                .border(1.0, t.card_border)
                .child(text(format!("PROGRAMA — {label}")).key("ptitle").font(type_::BODY).color(t.text))
                .child(text(detail).key("pdetail").w_full().wrap().font(type_::LABEL).color(t.text_secondary)),
        )
        .child(required_rows(&id, s, t, accent))
        .child(basket_area(s, t, accent))
        .child(
            row()
                .key("profile-actions")
                .gap(space::S2)
                .child(action("back", "Volver", t, true, false, accent, |s, _| s.screen = Screen::Home))
                .child(action(
                    "review",
                    "Revisar…",
                    t,
                    available,
                    true,
                    accent,
                    |s, _| {
                        s.screen = Screen::Review;
                        s.confirmation.clear();
                    },
                )),
        )
}

fn review(s: &State, t: Theme, accent: Color) -> El {
    let id = s.selected.clone();
    let info = s.profile_info(&id);
    let label = info.map(|i| i.label.clone()).unwrap_or_else(|| id.clone());
    let remote_ready = s.snapshot.as_ref().is_some_and(|x| x.remote_ready);
    let options = info
        .map(|i| i.required.iter().map(|o| o.label.clone()).collect::<Vec<_>>().join(" · "))
        .unwrap_or_default();
    let code = info
        .and_then(|i| i.code.clone())
        .unwrap_or_else(|| "no confirmado".to_string());
    let confirmed = s.confirmation.text.trim() == "INICIAR";
    col()
        .key("review")
        .gap(space::S3)
        .child(
            text("REVISIÓN — TODAVÍA NO SE ENVIÓ NADA")
                .key("rtitle")
                .font(type_::SECTION)
                .color(t.text_secondary),
        )
        .child(
            col()
                .key("summary")
                .padding(space::S3)
                .gap(space::S1)
                .fill(t.card)
                .radius(radius::LG)
                .border(1.0, t.card_border)
                .child(text(format!("Programa: {label} ({code})")).key("sum-program").font(type_::BODY).color(t.text))
                .child(text(format!("Opciones: {options}")).key("sum-options").font(type_::BODY).color(t.text))
                .child(text(format!("Media carga: {}", s.basket.label())).key("sum-basket").font(type_::BODY).color(t.text))
                .child(
                    text(format!("Control remoto: {}", if remote_ready { "listo" } else { "no verificado / no listo" }))
                        .key("sum-remote")
                        .font(type_::BODY)
                        .color(t.text),
                )
                .child(text(s.door_text()).key("sum-door").font(type_::BODY).color(t.text)),
        )
        .child(
            text("Comprobá que el equipo esté cargado, con detergente y la puerta cerrada. Al enviar, la app revalida el esquema y el estado contra hOn.")
                .key("warn")
                .w_full()
                .wrap()
                .font(type_::LABEL)
                .color(t.text_secondary),
        )
        .child(
            col()
                .key("confirm-field")
                .gap(4.0)
                .child(
                    text("Para habilitar el envío, escribí INICIAR en el campo")
                        .key("confirm-label")
                        .font(type_::LABEL)
                        .color(t.text_secondary),
                )
                .child(
                    field_shell(
                        "confirm-shell",
                        text_input(&s.confirmation, type_::BODY)
                            .key("confirmation")
                            .single_line()
                            .color(t.text)
                            .on_edit(|s: &mut State| &mut s.confirmation),
                        t,
                    ),
                ),
        )
        .child(
            row()
                .key("review-actions")
                .gap(space::S2)
                .child(action(
                    "cancel",
                    "Cancelar",
                    t,
                    true,
                    false,
                    accent,
                    |s, _| {
                        s.screen = Screen::Profile;
                        s.confirmation.clear();
                    },
                ))
                .child(action("start", "Iniciar ciclo", t, confirmed, true, accent, |s, _| crate::start_cycle(s))),
        )
}

fn config(s: &State, t: Theme, accent: Color) -> El {
    let can_save = s.email.text.contains('@') && !s.password.text.is_empty();
    col()
        .key("config")
        .gap(space::S3)
        .child(
            text("CONFIGURACIÓN — CUENTA hOn")
                .key("ctitle")
                .font(type_::SECTION)
                .color(t.text_secondary),
        )
        .child(
            col()
                .key("email-field")
                .gap(4.0)
                .child(text("Correo de hOn").key("email-label").font(type_::LABEL).color(t.text_secondary))
                .child(
                    field_shell(
                        "email-shell",
                        text_input(&s.email, type_::BODY)
                            .key("email")
                            .single_line()
                            .color(t.text)
                            .on_edit(|s: &mut State| &mut s.email),
                        t,
                    ),
                ),
        )
        .child(
            col()
                .key("password-field")
                .gap(4.0)
                .child(text("Contraseña").key("password-label").font(type_::LABEL).color(t.text_secondary))
                .child(
                    field_shell(
                        "password-shell",
                        text_input(&s.password, type_::BODY)
                            .key("password")
                            .single_line()
                            .secure()
                            .color(t.text)
                            .on_edit(|s: &mut State| &mut s.password),
                        t,
                    ),
                ),
        )
        .child(
            row()
                .key("config-actions")
                .gap(space::S2)
                .child(action("back", "Volver", t, true, false, accent, |s, _| s.screen = Screen::Home))
                .child(action("save", "Guardar credenciales", t, can_save, true, accent, |s, _| crate::save_credentials(s))),
        )
}
