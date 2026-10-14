//! Settings › Agents › Models (DESIGN §13 P5-100, drawn anew in P5-101): the models New Agent
//! offers for Claude and Codex as tags, where they came from, and Refresh, which asks codex now.
use super::*;
use crate::{
    models::{self, Lists, Source},
    new_agent::Tool,
};

/// The section's state in the Settings window.
#[derive(Default)]
pub(super) struct Models {
    lists: Option<Lists>,
    /// Why codex could not be asked last time.
    error: Option<String>,
    asking: bool,
}

/// The column with the kind's name and where its list came from.
const NAME_WIDTH: f32 = 150.0;
/// The column Refresh sits in, kept on the Claude row too so the tags line up.
const ACTION_WIDTH: f32 = 96.0;

const NOTE: &str = "Codex is asked once a month. To use your own lists, add \
[agent_models.claude] or [agent_models.codex] to config.toml.";

impl SettingsView {
    /// Reads the config and what paddock kept from codex again.
    pub(super) fn load_models(&mut self) {
        let configured = Config::load(&self.path).unwrap_or_default().agent_models;
        let kept = models::kept();
        self.models.error = kept.as_ref().and_then(|cache| cache.error.clone());
        self.models.lists = Some(Lists::new(&configured, kept.as_ref()));
    }

    /// Refresh: asks codex now, in the background.
    fn refresh_models(&mut self, cx: &mut Context<Self>) {
        if self.models.asking || models::asking() {
            return;
        }
        self.models.asking = true;
        let task = cx.background_spawn(async { models::ask_codex(true) });
        cx.spawn(async move |this, cx| {
            task.await;
            let _ = this.update(cx, |this, cx| {
                this.models.asking = false;
                this.load_models();
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn models_section(&self, ui: &UiFont, cx: &mut Context<Self>) -> AnyElement {
        let lists = self.models.lists.clone().unwrap_or_default();
        let asking = self.models.asking || models::asking();
        let claude = self.models_row(Tool::Claude, &lists, false, ui, cx);
        let codex = self.models_row(Tool::Codex, &lists, asking, ui, cx);
        let body = div()
            .flex()
            .flex_col()
            .child(self.card(None, vec![claude, codex]))
            .child(
                self.under(NOTE, self.fg(|t| t.agents_dimmer), ui)
                    .pl(px(4.0)),
            );
        self.section("Models", body, ui).into_any_element()
    }

    /// A kind's row: its name with where its list came from under it, its models as tags (the
    /// default one lit), and for Codex, Refresh.
    fn models_row(
        &self,
        tool: Tool,
        lists: &Lists,
        asking: bool,
        ui: &UiFont,
        cx: &mut Context<Self>,
    ) -> Div {
        let dimmer = self.fg(|t| t.agents_dimmer);
        let yellow = self.fg(|t| t.agents_yellow);
        let Some(list) = lists.of(tool) else {
            return div();
        };
        let codex = tool == Tool::Codex;
        let error = self.models.error.as_ref().filter(|_| codex && !asking);
        // Codex says whether its list is codex's own: green when it is and the last asking went
        // well, yellow when the last asking failed.
        let dot = if !codex || asking {
            None
        } else if error.is_some() {
            Some(yellow)
        } else if matches!(list.source, Source::Codex { .. }) {
            Some(self.fg(|t| t.agents_green))
        } else {
            None
        };
        let source = if asking {
            "Asking codex…".to_owned()
        } else {
            list.source.label()
        };
        let name = div()
            .flex_shrink_0()
            .w(ui.px(NAME_WIDTH))
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(div().text_size(ui.px(13.5)).child(tool.label()))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .text_size(ui.px(11.5))
                    .text_color(dimmer)
                    .children(dot.map(|color| div().size(px(6.0)).rounded_full().bg(color)))
                    .child(source),
            )
            .children(error.map(|error| {
                div()
                    .text_size(ui.px(11.5))
                    .text_color(yellow)
                    .child(format!("Couldn't ask: {error}"))
            }));
        let default = lists.default_model(tool);
        let accent = self.fg(|t| t.agents_accent);
        let tags = list.models.iter().map(|model| {
            let lit = default == Some(model.name.as_str());
            div()
                .h(ui.px(24.0))
                .px(px(9.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .rounded(px(7.0))
                .font_family(MONO)
                .text_size(ui.px(11.5))
                .map(|tag| {
                    if lit {
                        tag.bg(accent.opacity(0.12)).text_color(accent)
                    } else {
                        tag.bg(self.faint(0.05))
                            .text_color(self.fg(|t| t.agents_text).opacity(0.8))
                    }
                })
                .child(model.name.clone())
                .when(lit, |tag| {
                    tag.child(
                        div()
                            .font_family(
                                ui.family.clone().unwrap_or_else(|| ".SystemUIFont".into()),
                            )
                            .text_size(ui.px(10.5))
                            .text_color(self.fg(|t| t.agents_dim))
                            .child("default"),
                    )
                })
        });
        let action = div()
            .flex_shrink_0()
            .w(ui.px(ACTION_WIDTH))
            .flex()
            .justify_end()
            .when(codex, |action| {
                action.child(self.refresh_button(asking, ui, cx))
            });
        div()
            .flex()
            .items_start()
            .gap(px(18.0))
            .py(px(16.0))
            .pl(px(18.0))
            .pr(px(16.0))
            .child(name)
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_wrap()
                    .gap(px(6.0))
                    .when(asking, |tags| tags.opacity(0.55))
                    .children(tags),
            )
            .child(action)
    }

    /// Refresh with its icon; grey and still while codex is being asked.
    fn refresh_button(&self, asking: bool, ui: &UiFont, cx: &mut Context<Self>) -> Stateful<Div> {
        let color = if asking {
            self.fg(|t| t.agents_dimmer).opacity(0.7)
        } else {
            self.fg(|t| t.agents_branch)
        };
        let hover = self.faint(0.07);
        let button = div()
            .id("models-refresh")
            .flex_shrink_0()
            .h(ui.px(26.0))
            .px(px(10.0))
            .flex()
            .items_center()
            .gap(px(6.0))
            .rounded_full()
            .text_size(ui.px(12.0))
            .text_color(color)
            .child(footer_icon::icon(
                Icon::Reload,
                color,
                ui.scale(12.0) / footer_icon::SIZE,
            ))
            .child("Refresh");
        if asking {
            button
        } else {
            button
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.refresh_models(cx)))
        }
    }
}
