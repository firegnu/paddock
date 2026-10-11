//! Markdown read as the blocks paddock draws (pulldown-cmark, its CommonMark core and tables): a task
//! file's body in the Kanban task dialog's Preview, an agent's reply in the Inbox, cairn's text.
//! Headings, paragraphs, list items (nested lists a step further in), code blocks, tables, and
//! within a line bold, italic, inline code and links (by their words; they are not followed).
//! Anything else is shown as a paragraph of its words. `markdown_view.rs` draws them.
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

/// Words in one style.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    /// A link's words; where it goes is not kept.
    pub link: bool,
}

/// What a list item starts with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Marker {
    Bullet,
    Number(u64),
}

/// One block of the preview, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    /// A heading and its level, 1 to 6.
    Heading(u8, Vec<Span>),
    Paragraph(Vec<Span>),
    /// A list item's line, `depth` lists in (0 for the outermost). Without a marker, a further
    /// paragraph of the item above.
    Item {
        depth: usize,
        marker: Option<Marker>,
        spans: Vec<Span>,
    },
    /// A code block's lines.
    Code(String),
    /// A table: its head's cells, then each row's, a cell its words in their styles.
    Table {
        head: Vec<Vec<Span>>,
        rows: Vec<Vec<Vec<Span>>>,
    },
}

/// `source`'s blocks.
pub fn blocks(source: &str) -> Vec<Block> {
    read(source, " ")
}

/// As [`blocks`], a line break within a paragraph or a list item kept as one: the Cairn tab's
/// text puts a fact on each line.
pub fn blocks_by_line(source: &str) -> Vec<Block> {
    read(source, "\n")
}

/// The least and the most a table's column counts for, in characters' widths: a column of single
/// letters still has room, and one of long sentences does not squeeze the others out.
const COLUMN: (usize, usize) = (6, 40);

/// How a table's width is shared among its columns, as parts of one: each by its widest cell,
/// within bounds. A cell's words wrap in what its column gets.
pub fn shares(head: &[Vec<Span>], rows: &[Vec<Vec<Span>>]) -> Vec<f32> {
    let columns = rows.iter().map(Vec::len).fold(head.len(), usize::max);
    let wide = |cell: &Vec<Span>| -> usize {
        cell.iter()
            .map(|span| unicode_width::UnicodeWidthStr::width(span.text.as_str()))
            .sum()
    };
    let widths: Vec<usize> = (0..columns)
        .map(|column| {
            let cells = std::iter::once(head).chain(rows.iter().map(Vec::as_slice));
            let widest = cells.filter_map(|row| row.get(column)).map(wide).max();
            widest.unwrap_or(0).clamp(COLUMN.0, COLUMN.1)
        })
        .collect();
    let all: usize = widths.iter().sum();
    widths
        .iter()
        .map(|&width| width as f32 / all as f32)
        .collect()
}

/// `source`'s blocks, a line break within one read as `soft`.
fn read(source: &str, soft: &str) -> Vec<Block> {
    let mut read = Reader::default();
    for (event, range) in Parser::new_ext(source, Options::ENABLE_TABLES).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                read.line();
                read.heading = Some(level as u8);
            }
            Event::End(TagEnd::Heading(_)) => {
                read.line();
                read.heading = None;
            }
            Event::Start(Tag::List(first)) => {
                read.line();
                read.lists.push(first);
            }
            Event::End(TagEnd::List(_)) => {
                read.lists.pop();
            }
            Event::Start(Tag::Item) => {
                read.line();
                let next = read.lists.last_mut().and_then(Option::as_mut);
                read.marker = Some(match next {
                    Some(n) => {
                        *n += 1;
                        Marker::Number(*n - 1)
                    }
                    None => Marker::Bullet,
                });
                read.items += 1;
            }
            Event::End(TagEnd::Item) => {
                read.line();
                // An item with no words still shows its marker.
                if read.marker.is_some() {
                    read.push(Vec::new());
                }
                read.items -= 1;
            }
            Event::Start(Tag::CodeBlock(_)) => {
                read.line();
                read.code = Some(String::new());
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(code) = read.code.take() {
                    read.out
                        .push(Block::Code(code.trim_end_matches('\n').to_owned()));
                }
            }
            Event::Start(Tag::Table(_)) => {
                read.line();
                read.table = Some(Table::default());
            }
            Event::End(TagEnd::TableCell) => read.cell(),
            Event::End(TagEnd::TableHead) => {
                if let Some(table) = &mut read.table {
                    table.head = std::mem::take(&mut table.row);
                }
            }
            Event::End(TagEnd::TableRow) => {
                if let Some(table) = &mut read.table {
                    let row = std::mem::take(&mut table.row);
                    table.rows.push(row);
                }
            }
            Event::End(TagEnd::Table) => {
                if let Some(Table { head, rows, .. }) = read.table.take() {
                    read.out.push(Block::Table { head, rows });
                }
            }
            Event::End(TagEnd::Paragraph | TagEnd::HtmlBlock) => read.line(),
            Event::Start(Tag::Emphasis) => read.italic += 1,
            Event::End(TagEnd::Emphasis) => read.italic -= 1,
            Event::Start(Tag::Strong) => read.bold += 1,
            Event::End(TagEnd::Strong) => read.bold -= 1,
            Event::Start(Tag::Link { .. }) => read.link += 1,
            Event::End(TagEnd::Link) => read.link -= 1,
            Event::Text(text) => match &mut read.code {
                Some(code) => code.push_str(&text),
                None => read.words(&text, false),
            },
            Event::Code(text) => read.words(&text, true),
            Event::Html(text) | Event::InlineHtml(text) => read.words(&text, false),
            Event::SoftBreak => read.words(soft, false),
            Event::HardBreak => read.words("\n", false),
            Event::Rule => {
                read.line();
                read.words(source[range].trim(), false);
                read.line();
            }
            // Block quotes, images (their words come as text) and what only options turn on.
            _ => {}
        }
    }
    read.line();
    read.out
}

/// A table as it is read: its head, the rows done, and the cells of the row not yet done.
#[derive(Default)]
struct Table {
    head: Vec<Vec<Span>>,
    rows: Vec<Vec<Vec<Span>>>,
    row: Vec<Vec<Span>>,
}

/// Where the reading is: the blocks so far, what is open, and the words of the line not yet out.
#[derive(Default)]
struct Reader {
    out: Vec<Block>,
    spans: Vec<Span>,
    /// The heading being read, by level.
    heading: Option<u8>,
    /// The lists open, outermost first: each one's next number, `None` for bullets.
    lists: Vec<Option<u64>>,
    /// How many list items are open.
    items: usize,
    /// The open item's marker, until its first line is out.
    marker: Option<Marker>,
    /// The code block being read.
    code: Option<String>,
    /// The table being read.
    table: Option<Table>,
    bold: u32,
    italic: u32,
    link: u32,
}

impl Reader {
    /// Words in the style open now, run on with the last ones in the same style.
    fn words(&mut self, text: &str, code: bool) {
        let span = Span {
            text: text.to_owned(),
            bold: self.bold > 0,
            italic: self.italic > 0,
            code,
            link: self.link > 0,
        };
        let style = |s: &Span| (s.bold, s.italic, s.code, s.link);
        match self.spans.last_mut() {
            Some(last) if style(last) == style(&span) => last.text.push_str(text),
            _ => self.spans.push(span),
        }
    }

    /// The line read so far out as its block, unless it has no words.
    fn line(&mut self) {
        if let Some(last) = self.spans.last_mut() {
            let kept = last.text.trim_end_matches('\n').len();
            last.text.truncate(kept);
        }
        self.spans.retain(|span| !span.text.is_empty());
        if self.spans.is_empty() {
            return;
        }
        let spans = std::mem::take(&mut self.spans);
        self.push(spans);
    }

    /// The words read so far out as a cell of the table's row, an empty one too.
    fn cell(&mut self) {
        let mut spans = std::mem::take(&mut self.spans);
        spans.retain(|span| !span.text.is_empty());
        if let Some(table) = &mut self.table {
            table.row.push(spans);
        }
    }

    /// `spans` out as the block they are in.
    fn push(&mut self, spans: Vec<Span>) {
        let block = match self.heading {
            Some(level) => Block::Heading(level, spans),
            None if self.items > 0 => Block::Item {
                depth: self.lists.len().saturating_sub(1),
                marker: self.marker.take(),
                spans,
            },
            None => Block::Paragraph(spans),
        };
        self.out.push(block);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(text: &str) -> Span {
        Span {
            text: text.to_owned(),
            ..Span::default()
        }
    }

    fn styled(text: &str, style: impl FnOnce(&mut Span)) -> Span {
        let mut span = plain(text);
        style(&mut span);
        span
    }

    fn item(depth: usize, marker: Option<Marker>, text: &str) -> Block {
        Block::Item {
            depth,
            marker,
            spans: if text.is_empty() {
                Vec::new()
            } else {
                vec![plain(text)]
            },
        }
    }

    #[test]
    fn headings_and_paragraphs() {
        assert_eq!(blocks(""), []);
        assert_eq!(
            blocks("## 用户原话\n看板上\n卡片  \n推错列\n\n# 一\n###### 六\n"),
            [
                Block::Heading(2, vec![plain("用户原话")]),
                Block::Paragraph(vec![plain("看板上 卡片\n推错列")]),
                Block::Heading(1, vec![plain("一")]),
                Block::Heading(6, vec![plain("六")]),
            ]
        );
    }

    #[test]
    fn a_line_break_within_a_block_is_a_space_or_kept_when_asked() {
        let source = "### 现场对比\n相对记录 01：\nHEAD 多 2 个提交\n没有变化。\n\n- a\n  b\n";
        assert_eq!(
            blocks(source),
            [
                Block::Heading(3, vec![plain("现场对比")]),
                Block::Paragraph(vec![plain("相对记录 01： HEAD 多 2 个提交 没有变化。")]),
                item(0, Some(Marker::Bullet), "a b"),
            ]
        );
        assert_eq!(
            blocks_by_line(source),
            [
                Block::Heading(3, vec![plain("现场对比")]),
                Block::Paragraph(vec![plain("相对记录 01：\nHEAD 多 2 个提交\n没有变化。")]),
                item(0, Some(Marker::Bullet), "a\nb"),
            ]
        );
        // Nothing else differs.
        let other = "## 停点\n一行。\n\n1. x\n\n```\ncode\nmore\n```\n";
        assert_eq!(blocks_by_line(other), blocks(other));
    }

    #[test]
    fn list_items_and_the_list_nested_in_one() {
        use Marker::*;
        assert_eq!(
            blocks("- a\n- b\n  - c\n  - d\n- e\n\n3. x\n4. y\n   1. z\n"),
            [
                item(0, Some(Bullet), "a"),
                item(0, Some(Bullet), "b"),
                item(1, Some(Bullet), "c"),
                item(1, Some(Bullet), "d"),
                item(0, Some(Bullet), "e"),
                item(0, Some(Number(3)), "x"),
                item(0, Some(Number(4)), "y"),
                item(1, Some(Number(1)), "z"),
            ]
        );
        // Items apart, a second paragraph in one, and one left empty, as a new task's sections are.
        assert_eq!(
            blocks("- a\n\n  more\n\n- b\n"),
            [
                item(0, Some(Bullet), "a"),
                item(0, None, "more"),
                item(0, Some(Bullet), "b"),
            ]
        );
        assert_eq!(
            blocks("## 要做的\n-\n"),
            [
                Block::Heading(2, vec![plain("要做的")]),
                item(0, Some(Bullet), ""),
            ]
        );
    }

    #[test]
    fn styles_within_a_line_and_code_blocks() {
        assert_eq!(
            blocks("**粗** *斜* ***都*** `cargo test` [链接 `x`](https://x.dev) 后\n"),
            [Block::Paragraph(vec![
                styled("粗", |s| s.bold = true),
                plain(" "),
                styled("斜", |s| s.italic = true),
                plain(" "),
                styled("都", |s| (s.bold, s.italic) = (true, true)),
                plain(" "),
                styled("cargo test", |s| s.code = true),
                plain(" "),
                styled("链接 ", |s| s.link = true),
                styled("x", |s| (s.link, s.code) = (true, true)),
                plain(" 后"),
            ])]
        );
        assert_eq!(
            blocks("```sh\ncargo test\n  cargo fmt\n```\n\n    indented\n"),
            [
                Block::Code("cargo test\n  cargo fmt".into()),
                Block::Code("indented".into()),
            ]
        );
        // In a list item.
        assert_eq!(
            blocks("- run `cargo test`\n"),
            [Block::Item {
                depth: 0,
                marker: Some(Marker::Bullet),
                spans: vec![plain("run "), styled("cargo test", |s| s.code = true)],
            }]
        );
    }

    #[test]
    fn a_table_is_its_head_and_rows_with_the_styles_in_its_cells() {
        let source = "之前\n\n| 档位 | 命令 | 说明 |\n| --- | :-: | --: |\n| **常规** | `cargo test` | 改了 *两处* |\n| 加强 | | [文档](https://x.dev) 见 |\n\n之后\n";
        let table = Block::Table {
            head: vec![
                vec![plain("档位")],
                vec![plain("命令")],
                vec![plain("说明")],
            ],
            rows: vec![
                vec![
                    vec![styled("常规", |s| s.bold = true)],
                    vec![styled("cargo test", |s| s.code = true)],
                    vec![plain("改了 "), styled("两处", |s| s.italic = true)],
                ],
                // An empty cell keeps its place.
                vec![
                    vec![plain("加强")],
                    vec![],
                    vec![styled("文档", |s| s.link = true), plain(" 见")],
                ],
            ],
        };
        let around = [
            Block::Paragraph(vec![plain("之前")]),
            table,
            Block::Paragraph(vec![plain("之后")]),
        ];
        assert_eq!(blocks(source), around);
        // The same by line, as the Cairn tab reads its text.
        assert_eq!(blocks_by_line(source), around);
        // Its columns share the width by their widest cells: 4, 10 and 9 characters wide, the
        // narrow one counted as 6.
        let Block::Table { head, rows } = &around[1] else {
            unreachable!()
        };
        assert_eq!(shares(head, rows), [6.0 / 25.0, 10.0 / 25.0, 9.0 / 25.0]);
        // A head alone, and a table in a list item.
        assert_eq!(
            blocks("- 见下\n\n  | a | b |\n  |---|---|\n"),
            [
                item(0, Some(Marker::Bullet), "见下"),
                Block::Table {
                    head: vec![vec![plain("a")], vec![plain("b")]],
                    rows: vec![],
                },
            ]
        );
    }

    #[test]
    fn anything_else_is_a_paragraph_of_its_words() {
        assert_eq!(
            blocks("> 引用\n\n---\n\n| a | b |\n\n<div>x</div>\n\n![图](a.png)\n\n- [ ] 待办\n"),
            [
                Block::Paragraph(vec![plain("引用")]),
                Block::Paragraph(vec![plain("---")]),
                // A row of cells with no rule under it is no table.
                Block::Paragraph(vec![plain("| a | b |")]),
                Block::Paragraph(vec![plain("<div>x</div>")]),
                Block::Paragraph(vec![plain("图")]),
                item(0, Some(Marker::Bullet), "[ ] 待办"),
            ]
        );
    }
}
