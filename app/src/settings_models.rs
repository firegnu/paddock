//! Settings › Agents › Models (DESIGN §13 P5-100): the models New Agent offers for Claude and
//! Codex, where they came from, and Refresh, which asks codex now.
use super::*;
use crate::models::{self, Lists, Source};

/// The section's state in the Settings window.
#[derive(Default)]
pub(super) struct Models {
    lists: Option<Lists>,
    /// Why codex could not be asked last time.
    error: Option<String>,
    asking: bool,
}

/// The Agent and From columns; the list takes what is between.
const AGENT_WIDTH: f32 = 110.0;
const FROM_WIDTH: f32 = 260.0;

const NOTE: [&str; 2] = [
    "Codex's models are asked from codex once a month, or when you click Refresh. If that fails, \
paddock uses the last ones it got, then [agent_models.codex] in config.toml, then its own list.",
    "Claude's come from [agent_models.claude] in config.toml, else paddock's own list.",
];

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
        let columns = div()
            .flex()
            .items_center()
            .gap(px(14.0))
            .pt(px(8.0))
            .pb(px(2.0))
            .pl(px(28.0))
            .pr(px(16.0))
            .text_size(ui.px(11.5))
            .text_color(self.fg(|t| t.agents_dimmer))
            .child(div().flex_shrink_0().w(ui.px(AGENT_WIDTH)).child("Agent"))
            .child(div().flex_1().child("In New Agent"))
            .child(div().flex_shrink_0().w(ui.px(FROM_WIDTH)).child("From"));
        let claude = self.models_row("Claude", &lists.claude, None, ui);
        let codex = self.models_row("Codex", &lists.codex, Some(self.codex_from(ui, cx)), ui);
        let note = self.fg(|t| t.agents_dim);
        let body = div()
            .flex()
            .flex_col()
            .child(self.card(Some(columns.into_any_element()), vec![claude, codex]))
            .children(NOTE.map(|line| self.under(line, note, ui)));
        self.section("Models", body, ui).into_any_element()
    }

    /// A kind's row: its name, its models, and where they came from (`from` for Codex, which says
    /// more).
    fn models_row(
        &self,
        name: &'static str,
        list: &models::List,
        from: Option<Div>,
        ui: &UiFont,
    ) -> Div {
        let dim = self.fg(|t| t.agents_dim);
        let from = from.unwrap_or_else(|| {
            div()
                .text_size(ui.px(12.5))
                .text_color(dim)
                .child(list.source.label())
        });
        div()
            .flex()
            .items_center()
            .gap(px(14.0))
            .min_h(ui.px(54.0))
            .py(px(6.0))
            .pl(px(28.0))
            .pr(px(16.0))
            .child(
                div()
                    .flex_shrink_0()
                    .w(ui.px(AGENT_WIDTH))
                    .text_size(ui.px(13.5))
                    .child(name),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .font_family(MONO)
                    .text_size(ui.px(12.0))
                    .text_color(self.fg(|t| t.agents_text).opacity(0.8))
                    .child(list.shown()),
            )
            .child(div().flex_shrink_0().w(ui.px(FROM_WIDTH)).child(from))
    }

    /// Codex's From: where its list came from and why codex could not be asked, or that it is
    /// being asked; then Refresh.
    fn codex_from(&self, ui: &UiFont, cx: &mut Context<Self>) -> Div {
        let dim = self.fg(|t| t.agents_dim);
        let asking = self.models.asking || models::asking();
        let source = self
            .models
            .lists
            .as_ref()
            .map_or(Source::Own, |lists| lists.codex.source.clone());
        let said = if asking {
            div()
                .text_size(ui.px(12.5))
                .text_color(dim)
                .child("Asking codex…")
        } else {
            div()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(ui.px(12.5))
                        .text_color(dim)
                        .child(source.label()),
                )
                .children(self.models.error.as_ref().map(|error| {
                    div()
                        .text_size(ui.px(11.5))
                        .text_color(self.fg(|t| t.agents_yellow))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .child(format!("Couldn't ask codex: {error}"))
                }))
        };
        let hover = self.faint(0.1);
        let refresh = div()
            .id("models-refresh")
            .flex_shrink_0()
            .h(ui.px(24.0))
            .px(px(10.0))
            .flex()
            .items_center()
            .rounded_full()
            .text_size(ui.px(12.0))
            .child("Refresh");
        let refresh = if asking {
            refresh
                .bg(self.faint(0.04))
                .text_color(self.fg(|t| t.agents_dimmer))
        } else {
            refresh
                .bg(self.faint(0.06))
                .text_color(self.fg(|t| t.agents_branch))
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.refresh_models(cx)))
        };
        div()
            .min_w(px(0.0))
            .flex()
            .items_center()
            .gap(px(10.0))
            .child(said)
            .child(refresh)
    }
}
