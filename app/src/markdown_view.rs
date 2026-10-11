//! Markdown's blocks (`markdown.rs`) drawn, as the Kanban task dialog's Preview and the Inbox's
//! reply show them; and the table alone for the Cairn tab, which draws the rest of its text its
//! own way.
use crate::{
    fonts::UiFont,
    markdown::{self, Block, Marker, Span},
    popover,
    theme::Theme,
    view::hsla,
};
use gpui::{
    Div, Font, FontStyle, FontWeight, HighlightStyle, Hsla, SharedString, StyledText,
    UnderlineStyle, div, prelude::*, px, relative,
};

/// The colours the blocks are drawn in.
#[derive(Clone, Copy)]
pub struct Colors {
    /// Headings, and a table's head.
    pub bright: Hsla,
    /// Everything else's words.
    pub text: Hsla,
    /// List markers.
    pub marker: Hsla,
    /// Links.
    pub link: Hsla,
    /// The ground under code.
    pub code: Hsla,
    /// The lines between a table's rows.
    pub rule: Hsla,
}

impl Colors {
    pub fn of(theme: &Theme) -> Self {
        let fg =
            |pick: fn(&crate::preset::Theme) -> crate::preset::Color| hsla(theme.fg(pick), 1.0);
        Colors {
            bright: fg(|t| t.bright),
            text: fg(|t| t.agents_branch),
            marker: fg(|t| t.agents_dimmer),
            link: fg(|t| t.agents_accent),
            code: popover::lit(theme),
            rule: fg(|t| t.agents_rule),
        }
    }
}

/// `blocks` one under another, their words wrapping; empty for none.
pub fn blocks(blocks: &[Block], mono: &Font, colors: Colors, ui: &UiFont) -> Div {
    let words = |spans: &[Span]| styled(spans, &mono.family, colors.code, colors.link);
    let mut out = div()
        .flex()
        .flex_col()
        .gap(ui.px(6.0))
        .text_size(ui.px(13.0))
        .line_height(relative(1.55))
        .text_color(colors.text);
    for (n, block) in blocks.iter().enumerate() {
        out = out.child(match block {
            Block::Heading(level, spans) => div()
                .when(n > 0, |heading| heading.mt(ui.px(6.0)))
                .text_size(ui.px(match level {
                    1 => 16.0,
                    2 => 14.5,
                    _ => 13.5,
                }))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(colors.bright)
                .child(words(spans)),
            Block::Paragraph(spans) => div().child(words(spans)),
            Block::Item {
                depth,
                marker,
                spans,
            } => {
                let marker = match marker {
                    Some(Marker::Bullet) if *depth == 0 => "•".to_owned(),
                    Some(Marker::Bullet) => "◦".to_owned(),
                    Some(Marker::Number(n)) => format!("{n}."),
                    None => String::new(),
                };
                div()
                    .flex()
                    .gap(ui.px(8.0))
                    .pl(ui.px(18.0 * *depth as f32))
                    .child(
                        div()
                            .flex_shrink_0()
                            .min_w(ui.px(12.0))
                            .text_color(colors.marker)
                            .child(marker),
                    )
                    .child(div().flex_1().min_w(px(0.0)).child(words(spans)))
            }
            Block::Code(text) => div()
                .px(ui.px(10.0))
                .py(ui.px(8.0))
                .rounded(ui.px(6.0))
                .bg(colors.code)
                .font(mono.clone())
                .text_size(ui.px(12.0))
                .child(text.clone()),
            Block::Table { head, rows } => table(head, rows, &mono.family, colors, ui),
        });
    }
    out
}

/// A table: its head in bold on a faint ground, a thin line between its rows, and each cell's
/// words wrapping in its column, so it is never wider than its place. The columns share the
/// width as [`markdown::shares`] says.
pub fn table(
    head: &[Vec<Span>],
    rows: &[Vec<Vec<Span>>],
    mono: &SharedString,
    colors: Colors,
    ui: &UiFont,
) -> Div {
    let shares = markdown::shares(head, rows);
    let line = |cells: &[Vec<Span>]| {
        let cells = shares.iter().enumerate().map(|(column, share)| {
            let spans = cells.get(column).map(Vec::as_slice).unwrap_or_default();
            div()
                .flex_shrink_0()
                .w(relative(*share))
                .min_w(px(0.0))
                .px(ui.px(8.0))
                .py(ui.px(5.0))
                .child(styled(spans, mono, colors.code, colors.link))
        });
        div().flex().items_start().children(cells)
    };
    div()
        .flex()
        .flex_col()
        .rounded(ui.px(6.0))
        .border_1()
        .border_color(colors.rule)
        .overflow_hidden()
        .child(
            line(head)
                .bg(colors.code.opacity(0.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(colors.bright),
        )
        .children(
            rows.iter()
                .map(|row| line(row).border_t_1().border_color(colors.rule)),
        )
}

/// Markdown's words in their styles: bold, italic, inline code in `mono` on `code`, and links in
/// `link`, underlined.
pub fn styled(spans: &[Span], mono: &SharedString, code: Hsla, link: Hsla) -> StyledText {
    let mut text = String::new();
    let (mut highlights, mut fonts) = (Vec::new(), Vec::new());
    for span in spans {
        let start = text.len();
        text.push_str(&span.text);
        let range = start..text.len();
        let mut style = HighlightStyle::default();
        if span.bold {
            style.font_weight = Some(FontWeight::BOLD);
        }
        if span.italic {
            style.font_style = Some(FontStyle::Italic);
        }
        if span.link {
            style.color = Some(link);
            style.underline = Some(UnderlineStyle {
                thickness: px(1.0),
                color: Some(link),
                wavy: false,
            });
        }
        if span.code {
            style.background_color = Some(code);
            fonts.push((range.clone(), mono.clone()));
        }
        if style != HighlightStyle::default() {
            highlights.push((range, style));
        }
    }
    StyledText::new(text)
        .with_highlights(highlights)
        .with_font_family_overrides(fonts)
}
