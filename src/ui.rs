//! Las cuatro pantallas: inicio, programa, revisión/confirmación y configuración.

use rune::tok::{self, Theme, motion, radius, space, type_};
use rune::Color;
use rune::{Cx, El, ScrollSpec, Tx, col, el, row, text, text_input};
use rune_kit::{Switch, Kit};
use rune_theme_opencode;

use crate::state::{self, Basket, Screen, State};

pub fn view(s: &State, cx: &mut Cx<State>) -> El {
    let dark = cx.system_dark();
    let t = tok::theme(dark);
    let kit = rune_theme_opencode::kit(dark);
    let accent = tok::accent(0, dark);
    let model = s
        .snapshot
        .as_ref()
        .map(|x| x.model.clone())
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| "CF 6C4F1PS-12".to_string());
    let window = col()
        .key("window")
        .w_full()
        // fit_content_height exige que la raíz no tenga altura fija ni fill: la mide
        // del contenido. El fondo cubre igual con fill().
        .fill(t.bg)
        .px(space::S6)
        .py(space::S3)
        .gap(space::S3)
        .child(
            row()
                .key("header")
                .gap(8.0)
                .window_drag()
                .child(
                    // Puntos dibujados (sin texto): un texto es hit target y conservaría
                    // el press, así que el grip no arrastraría.
                    col()
                        .key("drag-grip")
                        .gap(4.0)
                        .center_y()
                        .h(28.0)
                        .child(el().key("dot1").size(4.0, 4.0).radius(2.0).fill(t.text_secondary))
                        .child(el().key("dot2").size(4.0, 4.0).radius(2.0).fill(t.text_secondary))
                        .child(el().key("dot3").size(4.0, 4.0).radius(2.0).fill(t.text_secondary)),
                )
                .child(
                    col()
                        .key("titles")
                        .gap(2.0)
                        .child(text("Candy · hOn").key("title").font(type_::TITLE).color(t.text))
                        .child(text(model).key("model").font(type_::LABEL).color(t.text_secondary)),
                ),
        )
        .child(
            col()
                .key("body")
                .w_full()
                .scroll(ScrollSpec { stick_bottom: false })
                .child(match s.screen {
                    Screen::Home => home(s, t, accent),
                    Screen::Profile => profile_screen(s, t, accent, kit),
                    Screen::Review => review(s, t, accent),
                    Screen::Config => config(s, t, accent, kit),
                    Screen::Stop => stop_screen(s, t, accent),
                }),
        )
        .child(status_line(s, t, accent));
    window
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
        .radius(17.0)
        .fill(if on { accent } else if enabled { t.card } else { t.control_off })
        .border(1.0, t.card_border)
        .hover(if enabled { t.hover } else { Color::TRANSPARENT })
        .on_press_tx(move |s: &mut State, tx| f(s, tx))
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

fn cycle_card(s: &State, t: Theme, accent: Color) -> El {
    let label = s
        .snapshot
        .as_ref()
        .and_then(|x| x.program.clone())
        .map(|name| name.to_uppercase())
        .unwrap_or_else(|| format!("CICLO prCode {}", s.snapshot.as_ref().and_then(|x| x.pr_code.clone()).unwrap_or_else(|| "?".into())));
    let countdown = if !s.countdown.is_empty() {
        s.countdown.clone()
    } else if s.remaining_minutes().is_none() {
        "El aparato no reporta el tiempo restante".to_string()
    } else {
        "Calculando el tiempo…".to_string()
    };
    col()
        .key("cycle-card")
        .gap(space::S1)
        .padding(space::S3)
        .fill(t.card)
        .radius(radius::LG)
        .border(1.0, t.card_border)
        .child(
            row()
                .key("cycle-head")
                .w_full()
                .center_y()
                .justify_between()
                .child(text("CICLO EN CURSO").key("cycle-title").font(type_::SECTION).color(t.text_secondary))
                .child(text(label).key("cycle-code").font(type_::MONO_CAPTION).color(t.text_secondary)),
        )
        .child(text(countdown).key("cycle-count").font(type_::TITLE).color(accent))
        .child(
            row()
                .key("cycle-actions")
                .gap(space::S2)
                .child(action("stop", "Detener ciclo…", t, true, false, accent, |s, tx| state::go(s, Screen::Stop, tx)))
                .child(action("refresh", "Actualizar", t, true, false, accent, |s, _| crate::refresh(s))),
        )
}

fn home(s: &State, t: Theme, accent: Color) -> El {
    let ready = s.snapshot_ready();
    let order = s.visible_order();
    let mut programs = col().key("programs").gap(space::S2);
    for id in &order {
        programs = programs.child(program_card(id, s, t, accent));
    }
    col()
        .key("home")
        .gap(space::S3)
        .child(
            col()
                .key("home-body")
                .gap(space::S3)
                .child(
                    if s.running() {
                        cycle_card(s, t, accent)
                    } else if ready && order.is_empty() {
                        text("No hay ciclos visibles; activá alguno en Configuración.")
                            .key("hint-empty")
                            .font(type_::SECTION)
                            .color(t.text_secondary)
                    } else {
                        text("Elegí un programa (no inicia nada)").key("hint").font(type_::SECTION).color(t.text_secondary)
                    },
                ),
        )
        .child(if ready {
            programs
        } else if s.busy {
            text("Consultando el aparato para listar los programas…")
                .key("programs-loading")
                .font(type_::BODY)
                .color(t.text_secondary)
        } else {
            text("Sin datos del aparato; mirá el estado de abajo.")
                .key("programs-none")
                .font(type_::BODY)
                .color(t.text_secondary)
        })
        .child(
            row()
                .key("home-actions")
                .gap(space::S2)
                .child(action("refresh", "Actualizar", t, true, false, accent, |s, _| crate::refresh(s)))
                .child(action("config", "Configuración", t, true, false, accent, |s, tx| state::go(s, Screen::Config, tx)))
                .child(action("quit", "Salir", t, true, false, accent, |_, tx| tx.quit())),
        )
}

fn field_shell(key: &'static str, input: El, t: Theme) -> El {
    el()
        .key(key)
        .w_full()
        .h(40.0)
        .px(space::S3)
        .radius(20.0)
        .fill(t.card)
        .border(1.0, t.card_border)
        .center_y()
        .child(input)
}

fn required_rows(id: &str, s: &State, t: Theme, accent: Color, kit: &Kit) -> El {
    let _ = accent;
    let info = s.profile_info(id);
    let options = info.map(|i| i.required.as_slice()).unwrap_or(&[]);
    let mut list = col()
        .key("required")
        .gap(space::S1)
        .child(text("OPCIONES RECORDADAS (SE ENVIÁN SEGÚN EL INTERRUPTOR)").key("req-title").font(type_::SECTION).color(t.text_secondary));
    if options.is_empty() {
        list = list.child(
            text("Actualizá el equipo para ver las opciones.")
                .key("req-empty")
                .font(type_::BODY)
                .color(t.text_secondary),
        );
    }
    for option in options {
        let on = option.enabled;
        let (program, key) = (id.to_string(), option.key.clone());
        list = list.child(
            row()
                .key(option.key.clone())
                .w_full()
                .center_y()
                .justify_between()
                .child(text(option.label.clone()).key("name").font(type_::BODY).color(t.text))
                .child(
                    Switch::new(on)
                        .standard()
                        .hide_label()
                        .label(option.label.clone())
                        .on_toggle_tx(move |st: &mut State, _| crate::apply_choice(st, program.clone(), key.clone(), !on))
                        .build(kit)
                        .key(format!("sw-{}", option.key)),
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
                .radius(15.0)
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

fn profile_screen(s: &State, t: Theme, accent: Color, kit: &Kit) -> El {
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
        .child(required_rows(&id, s, t, accent, kit))
        .child(basket_area(s, t, accent))
        .child(
            row()
                .key("profile-actions")
                .gap(space::S2)
                .child(action("back", "Volver", t, true, false, accent, |s, tx| state::go(s, Screen::Home, tx)))
                .child(action(
                    "review",
                    "Revisar…",
                    t,
                    available,
                    true,
                    accent,
                    |s, tx| state::go(s, Screen::Review, tx),
                )),
        )
}

fn review(s: &State, t: Theme, accent: Color) -> El {
    let id = s.selected.clone();
    let info = s.profile_info(&id);
    let label = info.map(|i| i.label.clone()).unwrap_or_else(|| id.clone());
    let remote_ready = s.snapshot.as_ref().is_some_and(|x| x.remote_ready);
    let options = info
        .map(|i| i.required.iter().filter(|o| o.enabled).map(|o| o.label.clone()).collect::<Vec<_>>().join(" · "))
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
                            .w_full()
                            .color(t.text)
                            .label("Confirmación INICIAR")
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
                    |s, tx| {
                        state::go(s, Screen::Profile, tx);
                        s.confirmation.clear();
                    },
                ))
                .child(action("start", "Iniciar ciclo", t, confirmed, true, accent, |s, _| crate::start_cycle(s))),
        )
}

fn stop_screen(s: &State, t: Theme, accent: Color) -> El {
    let code = s
        .snapshot
        .as_ref()
        .and_then(|x| x.pr_code.clone())
        .unwrap_or_else(|| "?".to_string());
    let countdown = if s.countdown.is_empty() {
        "Calculando el tiempo…".to_string()
    } else {
        s.countdown.clone()
    };
    let confirmed = s.confirmation.text.trim() == "DETENER";
    col()
        .key("stop")
        .gap(space::S3)
        .child(
            text("DETENER — TODAVÍA NO SE ENVIÓ NADA")
                .key("stitle")
                .font(type_::SECTION)
                .color(t.text_secondary),
        )
        .child(
            col()
                .key("stop-summary")
                .padding(space::S3)
                .gap(space::S1)
                .fill(t.card)
                .radius(radius::LG)
                .border(1.0, t.card_border)
                .child(text(countdown).key("stop-count").font(type_::TITLE).color(accent))
                .child(text(format!("Ciclo prCode {code} en curso")).key("stop-cycle").font(type_::BODY).color(t.text)),
        )
        .child(
            text("Al detener, hOn cancela el ciclo del aparato. Verificá que no haya vajilla enjuagada a medio hacer antes de confirmar.")
                .key("swarn")
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
                    text("Para habilitar la detención, escribí DETENER en el campo")
                        .key("stop-label")
                        .font(type_::LABEL)
                        .color(t.text_secondary),
                )
                .child(
                    field_shell(
                        "confirm-shell",
                        text_input(&s.confirmation, type_::BODY)
                            .key("confirmation")
                            .single_line()
                            .w_full()
                            .color(t.text)
                            .label("Confirmación DETENER")
                            .on_edit(|s: &mut State| &mut s.confirmation),
                        t,
                    ),
                ),
        )
        .child(
            row()
                .key("stop-actions")
                .gap(space::S2)
                .child(
                    action("back", "Volver", t, true, false, accent, |s, tx| {
                        state::go(s, Screen::Home, tx);
                        s.confirmation.clear();
                    }),
                )
                .child(action("stop", "Detener ciclo", t, confirmed, true, accent, |s, _| crate::stop_cycle(s))),
        )
}

fn config(s: &State, t: Theme, accent: Color, kit: &Kit) -> El {
    let can_save = s.email.text.contains('@') && !s.password.text.is_empty();
    let mut cycles = col().key("cycles").gap(space::S1);
    cycles = cycles.child(
        text("CICLOS EN EL INICIO")
            .key("cycles-title")
            .font(type_::SECTION)
            .color(t.text_secondary),
    );
    for id in s.program_order() {
        let info = s.profile_info(&id);
        let visible = s.program_visible(&id);
        let label = info.map(|i| i.label.clone()).unwrap_or_else(|| id.clone());
        let moved_id = id.clone();
        cycles = cycles.child(
            row()
                .key(format!("vis-{id}"))
                .w_full()
                .center_y()
                .justify_between()
                .child(text(label.clone()).key("name").font(type_::BODY).color(t.text))
                .child(
                    Switch::new(visible)
                        .standard()
                        .hide_label()
                        .label(label)
                        .on_toggle_tx(move |st: &mut State, _| crate::apply_visible(st, moved_id.clone(), !visible))
                        .build(kit)
                        .key(format!("swv-{id}")),
                ),
        );
    }
    if s.program_order().is_empty() {
        cycles = cycles.child(
            text("Disponible tras la primera consulta al aparato.")
                .key("cycles-empty")
                .font(type_::LABEL)
                .color(t.text_secondary),
        );
    }
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
                            .w_full()
                            .color(t.text)
                            .label("Correo de hOn")
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
                            .w_full()
                            .color(t.text)
                            .label("Contraseña")
                            .on_edit(|s: &mut State| &mut s.password),
                        t,
                    ),
                ),
        )
        .child(cycles)
        .child(
            row()
                .key("config-actions")
                .gap(space::S2)
                .child(action("back", "Volver", t, true, false, accent, |s, tx| state::go(s, Screen::Home, tx)))
                .child(action("save", "Guardar credenciales", t, can_save, true, accent, |s, _| crate::save_credentials(s))),
        )
}
