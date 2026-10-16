//! Settings › Machines (DESIGN §13 P5-123): the other machines in config.toml, a row each with how
//! it stands, the name it is shown by (clicked, typed over in place), Show, and Remove…, which asks
//! first. Each change is written over config.toml and takes effect in the main window at once
//! (`windows::edit_machine`); a machine is still added by hand.
use super::*;
use crate::{config::Machine, settings::MachineEdit, windows::Standing};

/// The page's state in the Settings window.
#[derive(Default)]
pub(super) struct Machines {
    /// The machine whose name is being typed over.
    renaming: Option<Renaming>,
    /// The machine Remove… asks about.
    removing: Option<String>,
    /// How each machine stands is read again every second while the page shows.
    ticking: bool,
}

struct Renaming {
    name: String,
    input: Entity<TextInput>,
    _blur: Subscription,
}

const ABOUT: &str = "Other machines whose corral agents paddock shows, each in a view of its own. \
A hidden machine is not connected or read.";

/// Another machine `others` names the same as `machine` is shown: its name, for the row's hint.
fn same_name<'a>(machine: &Machine, others: &'a [Machine]) -> Option<&'a str> {
    others
        .iter()
        .find(|other| other.name != machine.name && other.shown_name() == machine.shown_name())
        .map(|other| other.name.as_str())
}

impl SettingsView {
    /// While the page shows, how each machine stands is read again every second.
    pub(super) fn tick_machines(&mut self, cx: &mut Context<Self>) {
        if self.machines.ticking {
            return;
        }
        self.machines.ticking = true;
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let on = this.update(cx, |this, cx| {
                    let on = this.page == Page::Machines;
                    if on {
                        cx.notify();
                    } else {
                        this.machines.ticking = false;
                    }
                    on
                });
                if !matches!(on, Ok(true)) {
                    break;
                }
            }
        })
        .detach();
    }

    /// The name of machine `name` as a field: Return writes it, Esc leaves it as it was, and leaving
    /// the field writes it too, as Settings' other fields do.
    pub fn rename_machine(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.reload(cx);
        let Some(machine) = self.draft.machines().iter().find(|m| m.name == name) else {
            return;
        };
        let shown = machine.shown_name().to_owned();
        let colors = self.input_colors();
        let input = cx.new(|cx| TextInput::new(shown, name, colors, cx));
        let handle = input.read(cx).focus_handle(cx);
        window.focus(&handle, cx);
        let blur = cx.on_blur(&handle, window, |this, _, cx| this.finish_rename(true, cx));
        self.machines.renaming = Some(Renaming {
            name: name.to_owned(),
            input,
            _blur: blur,
        });
        cx.notify();
    }

    /// Ends the typing over a machine's name: what was typed is written (`keep`), or left.
    fn finish_rename(&mut self, keep: bool, cx: &mut Context<Self>) {
        let Some(renaming) = self.machines.renaming.take() else {
            return;
        };
        if keep {
            let text = renaming.input.read(cx).text().to_owned();
            let unchanged = self
                .draft
                .machines()
                .iter()
                .any(|m| m.name == renaming.name && m.shown_name() == text.trim());
            if !unchanged {
                self.change_machine(renaming.name, MachineEdit::Rename(text), cx);
            }
        }
        cx.notify();
    }

    /// Remove…: asks first.
    pub fn ask_remove_machine(&mut self, name: &str, cx: &mut Context<Self>) {
        self.machines.removing = Some(name.to_owned());
        cx.notify();
    }

    fn cancel_remove(&mut self, cx: &mut Context<Self>) {
        self.machines.removing = None;
        cx.notify();
    }

    /// The question's answer: `edit` made to the machine it asked about.
    fn answer_remove(&mut self, edit: MachineEdit, cx: &mut Context<Self>) {
        if let Some(name) = self.machines.removing.take() {
            self.change_machine(name, edit, cx);
        }
        cx.notify();
    }

    /// `edit` written over config.toml and put into effect in the main window, once this window
    /// is done with the event; why not, in the bar.
    fn change_machine(&mut self, name: String, edit: MachineEdit, cx: &mut Context<Self>) {
        let this = cx.entity().downgrade();
        cx.defer(move |cx| {
            if let Err(error) = crate::windows::edit_machine(&name, &edit, cx) {
                let _ = this.update(cx, |this, cx| this.say(error, true, cx));
            }
        });
    }

    /// config.toml as a machine change wrote it: the page reads it from now on.
    pub fn machines_written(&mut self, text: String, cx: &mut Context<Self>) {
        if let Err(error) = self.draft.rebase(Some(text)) {
            self.say(broken(&error), true, cx);
        }
        self.refresh_inputs(cx);
    }

    /// The page: what it is for, a row per machine (or that there is none), and how to add one.
    pub(super) fn machines_page(&self, ui: &UiFont, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let (dim, dimmer) = (self.fg(|t| t.agents_dim), self.fg(|t| t.agents_dimmer));
        let machines = self.draft.machines();
        let about = div()
            .px(px(4.0))
            .text_size(ui.px(12.5))
            .line_height(relative(1.5))
            .text_color(dim)
            .child(ABOUT);
        let rows: Vec<Div> = machines
            .iter()
            .map(|machine| self.machine_row(machine, machines, ui, cx))
            .collect();
        let list = if rows.is_empty() {
            self.card(
                None,
                vec![
                    div()
                        .px(px(16.0))
                        .py(ui.px(14.0))
                        .text_size(ui.px(13.0))
                        .text_color(dim)
                        .child("No other machines are set up."),
                ],
            )
        } else {
            self.card(None, rows)
        };
        let add = div()
            .px(px(4.0))
            .flex()
            .flex_wrap()
            .text_size(ui.px(12.0))
            .text_color(dimmer)
            .child("To add a machine, write a ")
            .child(
                div()
                    .font_family(MONO)
                    .text_color(dim)
                    .child("[[machines]]"),
            )
            .child(" entry in config.toml.");
        let problem = self.draft.machines_problem().map(|problem| {
            div()
                .px(px(4.0))
                .text_size(ui.px(12.0))
                .text_color(self.fg(|t| t.agents_red))
                .child(problem.to_owned())
        });
        let mut items = vec![about, list];
        items.extend(problem);
        items.push(add);
        items
            .into_iter()
            .map(IntoElement::into_any_element)
            .collect()
    }

    /// A machine's row: a dot for how it stands, its name (clicked, a field) over its ssh target
    /// and, when it has a label, its id; how it stands in words; Show; Remove….
    fn machine_row(
        &self,
        machine: &Machine,
        machines: &[Machine],
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Div {
        let (text, dim, dimmer) = (
            self.fg(|t| t.agents_text),
            self.fg(|t| t.agents_dim),
            self.fg(|t| t.agents_dimmer),
        );
        let name = machine.name.clone();
        let standing = if machine.show {
            crate::windows::machine_standing(&name, cx)
        } else {
            Standing::Hidden
        };
        let (dot, words, words_color) = match &standing {
            Standing::Online(agents) => {
                let agents = match agents {
                    0 => "no agents".to_owned(),
                    1 => "1 agent".to_owned(),
                    n => format!("{n} agents"),
                };
                (
                    self.fg(|t| t.agents_green),
                    format!("Online · {agents}"),
                    dim,
                )
            }
            Standing::Waiting(words) => (dimmer, words.clone(), dim),
            Standing::Down(words) => (
                self.fg(|t| t.agents_red),
                words.clone(),
                self.fg(|t| t.agents_red),
            ),
            Standing::Hidden => (dimmer.opacity(0.6), "Hidden".to_owned(), dimmer),
            Standing::AfterRestart => (dimmer, "Connects after a restart".to_owned(), dimmer),
        };
        let hidden = !machine.show;
        let renaming = self
            .machines
            .renaming
            .as_ref()
            .filter(|renaming| renaming.name == machine.name);
        let title: AnyElement = match renaming {
            Some(renaming) => {
                let accent = self.fg(|t| t.agents_accent);
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .id(ElementId::Name(format!("machine-{name}-field").into()))
                            .flex_shrink_0()
                            .w(ui.px(180.0))
                            .h(ui.px(24.0))
                            .px(px(8.0))
                            .flex()
                            .items_center()
                            .rounded(px(6.0))
                            .bg(self.field_ground())
                            .border_1()
                            .border_color(accent.opacity(0.6))
                            .overflow_hidden()
                            .font_weight(FontWeight::SEMIBOLD)
                            .key_context(menu::DIALOG)
                            .on_action(cx.listener(|this, _: &menu::OpenSelected, _, cx| {
                                this.finish_rename(true, cx)
                            }))
                            .on_action(cx.listener(|this, _: &menu::Cancel, _, cx| {
                                this.finish_rename(false, cx)
                            }))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .overflow_hidden()
                                    .child(renaming.input.clone()),
                            ),
                    )
                    .child(
                        div()
                            .min_w(px(0.0))
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .text_size(ui.px(11.0))
                            .text_color(dimmer)
                            .child(format!("Return to save · Esc to cancel · empty for {name}")),
                    )
                    .into_any_element()
            }
            None => {
                let rename = name.clone();
                div()
                    .id(ElementId::Name(format!("machine-{name}-name").into()))
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(if hidden { dim } else { text })
                    .cursor_text()
                    .hover(move |style| style.text_color(text))
                    .child(machine.shown_name().to_owned())
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.rename_machine(&rename, window, cx)
                    }))
                    .into_any_element()
            }
        };
        let mut place = div()
            .flex()
            .min_w(px(0.0))
            .overflow_hidden()
            .whitespace_nowrap()
            .text_size(ui.px(11.5))
            .text_color(if hidden { dimmer } else { dim })
            .child(div().font_family(MONO).child(machine.ssh.clone()));
        if machine.label.is_some() {
            place = place
                .child(div().px(px(5.0)).child("·"))
                .child("id ")
                .child(div().font_family(MONO).child(name.clone()));
        }
        let same = same_name(machine, machines).map(|other| {
            div()
                .text_size(ui.px(11.5))
                .text_color(self.fg(|t| t.agents_yellow))
                .child(format!("{other} is shown by this name too."))
        });
        let show = {
            let name = name.clone();
            let track = if hidden {
                self.lifted(0.16)
            } else {
                self.fg(|t| t.agents_accent)
            };
            div()
                .id(ElementId::Name(format!("machine-{name}-show").into()))
                .flex_shrink_0()
                .w(ui.px(34.0))
                .h(ui.px(20.0))
                .p(ui.px(2.0))
                .rounded(ui.px(10.0))
                .bg(track)
                .flex()
                .items_center()
                .when(!hidden, |track| track.justify_end())
                .cursor_pointer()
                .child(
                    div()
                        .size(ui.px(16.0))
                        .rounded(ui.px(8.0))
                        .bg(self.fg(|t| t.agents_text))
                        .shadow_sm(),
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.change_machine(name.clone(), MachineEdit::Show(hidden), cx)
                }))
        };
        let remove = {
            let name = name.clone();
            let (rest, lit) = (self.faint(0.07), self.faint(0.12));
            div()
                .id(ElementId::Name(format!("machine-{name}-remove").into()))
                .flex_shrink_0()
                .h(ui.px(26.0))
                .px(px(11.0))
                .flex()
                .items_center()
                .rounded(px(7.0))
                .bg(rest)
                .border_1()
                .border_color(self.faint(0.14))
                .text_size(ui.px(12.0))
                .cursor_pointer()
                .hover(move |style| style.bg(lit))
                .child("Remove…")
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.ask_remove_machine(&name, cx)
                }))
        };
        div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .px(px(14.0))
            .py(ui.px(12.0))
            .child(div().flex_shrink_0().size(px(8.0)).rounded_full().bg(dot))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap(px(3.0))
                    .child(title)
                    .child(place)
                    .children(same),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .w(ui.px(150.0))
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .text_size(ui.px(12.0))
                    .text_color(words_color)
                    .child(words),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(ui.px(12.0))
                    .text_color(self.fg(|t| t.agents_branch))
                    .child("Show"),
            )
            .child(show)
            .child(remove)
    }

    /// Remove…'s question over the window: what removing does and does not do, with Hide instead
    /// (for a machine not hidden yet), Cancel and Remove.
    pub(super) fn machines_confirm(
        &self,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Option<Stateful<Div>> {
        let name = self.machines.removing.as_ref()?;
        let machine = self.draft.machines().iter().find(|m| &m.name == name)?;
        let shown = machine.shown_name().to_owned();
        let (dim, dimmer, red) = (
            self.fg(|t| t.agents_branch),
            self.fg(|t| t.agents_dim),
            self.fg(|t| t.agents_red),
        );
        let button = |id: &'static str, label: &'static str| {
            let (rest, lit) = (self.faint(0.07), self.faint(0.12));
            div()
                .id(id)
                .h(ui.px(28.0))
                .px(px(13.0))
                .flex()
                .items_center()
                .rounded(px(7.0))
                .bg(rest)
                .border_1()
                .border_color(self.faint(0.14))
                .text_size(ui.px(12.5))
                .cursor_pointer()
                .hover(move |style| style.bg(lit))
                .child(label)
        };
        let hide = machine.show.then(|| {
            button("machine-hide-instead", "Hide instead").on_click(cx.listener(
                |this, _: &ClickEvent, _, cx| this.answer_remove(MachineEdit::Show(false), cx),
            ))
        });
        let panel = div()
            .id("machine-remove")
            .w(ui.px(400.0))
            .flex()
            .flex_col()
            .gap(px(10.0))
            .px(px(20.0))
            .pt(px(18.0))
            .pb(px(16.0))
            .rounded(ui.px(12.0))
            .border_1()
            .border_color(self.faint(0.12))
            .bg(crate::popover::ground(&self.theme))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .text_size(ui.px(14.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(format!("Remove {shown}?")),
            )
            .child(
                div()
                    .text_size(ui.px(12.5))
                    .line_height(relative(1.55))
                    .text_color(dim)
                    .child(format!(
                        "paddock stops showing {shown}, takes its entry out of config.toml and \
                         forgets its saved panes. The agents on {shown} keep running; nothing \
                         there is touched."
                    )),
            )
            .when(machine.show, |panel| {
                panel.child(
                    div()
                        .text_size(ui.px(12.0))
                        .text_color(dimmer)
                        .child("To keep it for later, hide it instead."),
                )
            })
            .child(
                div()
                    .mt(px(6.0))
                    .flex()
                    .justify_end()
                    .gap(px(8.0))
                    .children(hide)
                    .child(button("machine-remove-cancel", "Cancel").on_click(
                        cx.listener(|this, _: &ClickEvent, _, cx| this.cancel_remove(cx)),
                    ))
                    .child(
                        button("machine-remove-go", "Remove")
                            .bg(red.opacity(0.16))
                            .border_color(red.opacity(0.5))
                            .text_color(red)
                            .font_weight(FontWeight::SEMIBOLD)
                            .hover(move |style| style.bg(red.opacity(0.24)))
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.answer_remove(MachineEdit::Remove, cx)
                            })),
                    ),
            );
        Some(
            div()
                .id("machine-remove-backdrop")
                .absolute()
                .inset_0()
                .occlude()
                .bg(gpui::black().opacity(0.45))
                .flex()
                .items_center()
                .justify_center()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| this.cancel_remove(cx)),
                )
                .child(panel),
        )
    }
}
