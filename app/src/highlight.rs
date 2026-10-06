//! The Changes tab's colours for code: what each piece of a line is, by syntect's grammars, and
//! which words of a changed line changed, by `similar`. The pieces are named, not coloured, so the
//! tab maps them to the theme in effect and a theme change needs nothing worked out again.
use crate::diff::{Kind, Line};
use similar::{Algorithm, DiffOp, capture_diff_slices};
use std::{ops::Range, str::FromStr, sync::OnceLock};
use syntect::{
    highlighting::{
        Color, FontStyle, HighlightIterator, HighlightState, Highlighter, ScopeSelectors,
        StyleModifier, Theme, ThemeItem, ThemeSettings,
    },
    parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet},
};

/// What a piece of code is, for its colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Token {
    Plain,
    Keyword,
    Function,
    Type,
    String,
    Number,
    Comment,
    Punctuation,
}

impl Token {
    const ALL: [Token; 8] = [
        Token::Plain,
        Token::Keyword,
        Token::Function,
        Token::Type,
        Token::String,
        Token::Number,
        Token::Comment,
        Token::Punctuation,
    ];
}

/// A piece of a line: its bytes and what it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub range: Range<usize>,
    pub token: Token,
}

/// The bundled grammars, loaded the first time they are needed.
pub fn syntaxes() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

/// A theme whose colours are tokens: the red channel holds the token's place in `Token::ALL`.
fn theme() -> &'static Theme {
    static THEME: OnceLock<Theme> = OnceLock::new();
    THEME.get_or_init(|| {
        let item = |selectors: &str, token: Token, italic: bool| ThemeItem {
            scope: ScopeSelectors::from_str(selectors).expect("valid selectors"),
            style: StyleModifier {
                foreground: Some(code(token)),
                background: None,
                font_style: italic.then_some(FontStyle::ITALIC),
            },
        };
        Theme {
            settings: ThemeSettings {
                foreground: Some(code(Token::Plain)),
                ..ThemeSettings::default()
            },
            scopes: vec![
                item("comment, punctuation.definition.comment", Token::Comment, true),
                item(
                    "string, constant.character, punctuation.definition.string",
                    Token::String,
                    false,
                ),
                item("constant.numeric, constant.language", Token::Number, false),
                item(
                    "keyword, storage, keyword.control, variable.language",
                    Token::Keyword,
                    false,
                ),
                item(
                    "keyword.operator, punctuation, meta.attribute, meta.annotation",
                    Token::Punctuation,
                    false,
                ),
                item(
                    "entity.name.function, support.function, variable.function, \
                     meta.function-call entity.name, support.macro, entity.name.macro, \
                     entity.name.tag, entity.other.attribute-name, meta.mapping.key string",
                    Token::Function,
                    false,
                ),
                item(
                    "entity.name.type, entity.name.class, entity.name.struct, entity.name.enum, \
                     entity.name.trait, entity.name.interface, entity.name.namespace, \
                     entity.other.inherited-class, support.type, support.class, storage.type.primitive, \
                     storage.type.numeric, storage.type.builtin",
                    Token::Type,
                    false,
                ),
            ],
            ..Theme::default()
        }
    })
}

fn code(token: Token) -> Color {
    let place = Token::ALL.iter().position(|t| *t == token).unwrap_or(0);
    Color {
        r: place as u8,
        g: 0,
        b: 0,
        a: 0xff,
    }
}

/// The grammar for a file: by its name, its extension, or a `#!` first line.
pub fn syntax_for(path: &str, first_line: Option<&str>) -> &'static SyntaxReference {
    let set = syntaxes();
    let name = path.rsplit('/').next().unwrap_or(path);
    let extension = name.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
    set.find_syntax_by_extension(name)
        .or_else(|| set.find_syntax_by_extension(extension))
        .or_else(|| first_line.and_then(|line| set.find_syntax_by_first_line(line)))
        .unwrap_or_else(|| set.find_syntax_plain_text())
}

/// Reads lines one after another, each in the state the ones before left.
pub struct Stream {
    parse: ParseState,
    highlight: HighlightState,
    highlighter: Highlighter<'static>,
}

impl Stream {
    pub fn new(syntax: &SyntaxReference) -> Self {
        let highlighter = Highlighter::new(theme());
        Stream {
            parse: ParseState::new(syntax),
            highlight: HighlightState::new(&highlighter, ScopeStack::new()),
            highlighter,
        }
    }

    /// The pieces of the next line, which has no line ending. A line the grammar chokes on is
    /// plain, and so are the ones after it.
    pub fn line(&mut self, text: &str) -> Vec<Span> {
        let mut with_end = String::with_capacity(text.len() + 1);
        with_end.push_str(text);
        with_end.push('\n');
        let Ok(ops) = self.parse.parse_line(&with_end, syntaxes()) else {
            return plain(text);
        };
        let mut spans: Vec<Span> = Vec::new();
        let mut at = 0;
        for (style, piece) in
            HighlightIterator::new(&mut self.highlight, &ops, &with_end, &self.highlighter)
        {
            let end = (at + piece.len()).min(text.len());
            if end > at {
                let token = Token::ALL
                    .get(usize::from(style.foreground.r))
                    .copied()
                    .unwrap_or(Token::Plain);
                match spans.last_mut() {
                    Some(last) if last.token == token && last.range.end == at => {
                        last.range.end = end
                    }
                    _ => spans.push(Span {
                        range: at..end,
                        token,
                    }),
                }
            }
            at += piece.len();
        }
        spans
    }
}

fn plain(text: &str) -> Vec<Span> {
    vec![Span {
        range: 0..text.len(),
        token: Token::Plain,
    }]
}

/// Deleted and added lines that face each other: in each run of deletions followed by additions,
/// the first deleted with the first added, and so on. Indices into `lines`.
pub fn pairs(lines: &[Line]) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].kind != Kind::Deleted {
            i += 1;
            continue;
        }
        let deleted = i;
        while i < lines.len() && lines[i].kind == Kind::Deleted {
            i += 1;
        }
        let added = i;
        while i < lines.len() && lines[i].kind == Kind::Added {
            i += 1;
        }
        let count = (added - deleted).min(i - added);
        pairs.extend((0..count).map(|k| (deleted + k, added + k)));
    }
    pairs
}

/// Byte ranges in a line.
pub type Words = Vec<Range<usize>>;

/// The parts of a deleted line and the added line facing it that changed, as byte ranges in
/// each; `None` when the two have too little in common for words to say more than the lines do.
pub fn changed_words(old: &str, new: &str) -> Option<(Words, Words)> {
    let (old_words, new_words) = (words(old), words(new));
    let old_texts: Vec<&str> = old_words.iter().map(|r| &old[r.clone()]).collect();
    let new_texts: Vec<&str> = new_words.iter().map(|r| &new[r.clone()]).collect();
    let mut gone = Vec::new();
    let mut came = Vec::new();
    let mut kept = 0;
    let span = |words: &[Range<usize>], range: Range<usize>| -> Option<Range<usize>> {
        Some(words.get(range.start)?.start..words.get(range.end.checked_sub(1)?)?.end)
    };
    for op in capture_diff_slices(Algorithm::Myers, &old_texts, &new_texts) {
        match op {
            DiffOp::Equal { old_index, len, .. } => {
                kept += old_words[old_index..old_index + len]
                    .iter()
                    .filter(|r| !old[(*r).clone()].trim().is_empty())
                    .map(|r| r.len())
                    .sum::<usize>();
            }
            DiffOp::Delete {
                old_index, old_len, ..
            } => gone.extend(span(&old_words, old_index..old_index + old_len)),
            DiffOp::Insert {
                new_index, new_len, ..
            } => came.extend(span(&new_words, new_index..new_index + new_len)),
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                gone.extend(span(&old_words, old_index..old_index + old_len));
                came.extend(span(&new_words, new_index..new_index + new_len));
            }
        }
    }
    // Less than a third of the longer line kept: mostly rewritten.
    let longer = old.trim().len().max(new.trim().len());
    if longer == 0 || kept * 3 < longer || (gone.is_empty() && came.is_empty()) {
        return None;
    }
    Some((gone, came))
}

/// A line's words: runs of letters, digits and `_`, runs of spaces, and each other character
/// alone.
fn words(text: &str) -> Vec<Range<usize>> {
    #[derive(PartialEq)]
    enum Class {
        Word,
        Space,
        Other,
    }
    let class = |c: char| {
        if c.is_alphanumeric() || c == '_' {
            Class::Word
        } else if c.is_whitespace() {
            Class::Space
        } else {
            Class::Other
        }
    };
    let mut words: Vec<Range<usize>> = Vec::new();
    let mut last: Option<Class> = None;
    for (i, c) in text.char_indices() {
        let this = class(c);
        match words.last_mut() {
            Some(word) if last.as_ref() == Some(&this) && this != Class::Other => {
                word.end = i + c.len_utf8()
            }
            _ => words.push(i..i + c.len_utf8()),
        }
        last = Some(this);
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(kind: Kind, text: &str) -> Line {
        Line {
            kind,
            old: 0,
            new: 0,
            text: text.into(),
        }
    }

    #[test]
    fn deletions_face_the_additions_after_them_in_order() {
        let lines = [
            line(Kind::Context, "a"),
            line(Kind::Deleted, "b"),
            line(Kind::Deleted, "c"),
            line(Kind::Added, "B"),
            line(Kind::Context, "d"),
            line(Kind::Deleted, "e"),
            line(Kind::Added, "E"),
            line(Kind::Added, "F"),
            line(Kind::Added, "G"),
            line(Kind::Deleted, "h"),
            line(Kind::Context, "i"),
            line(Kind::Added, "J"),
        ];
        assert_eq!(pairs(&lines), [(1, 3), (5, 6)]);
    }

    #[test]
    fn changed_words_are_the_parts_that_differ() {
        let old = r#"    width.clamp(MIN_WIDTH, max.max(MIN_WIDTH))"#;
        let new = r#"    width.clamp(min_width(ui), max.max(min_width(ui)))"#;
        let (gone, came) = changed_words(old, new).unwrap();
        let gone: Vec<&str> = gone.iter().map(|r| &old[r.clone()]).collect();
        let came: Vec<&str> = came.iter().map(|r| &new[r.clone()]).collect();
        assert_eq!(gone, ["MIN_WIDTH", "MIN_WIDTH"]);
        assert_eq!(came, ["min_width(ui)", "min_width(ui)"]);

        // One word in a long line.
        let (gone, came) =
            changed_words("let total = count + 1;", "let total = count + 2;").unwrap();
        assert_eq!(gone, came);
        assert_eq!(gone.first(), Some(&(20..21)));
        assert_eq!(gone.len(), 1);

        // Unrelated lines say nothing more than that they changed.
        assert_eq!(changed_words("fn main() {", "    // a comment here"), None);
        // Nor do equal ones.
        assert_eq!(changed_words("same", "same"), None);
    }

    #[test]
    fn code_is_split_into_named_pieces() {
        let mut stream = Stream::new(syntax_for("src/lib.rs", None));
        let text = "pub fn add(a: u32) -> u32 { 42 } // sum";
        let spans = stream.line(text);
        let token = |piece: &str| {
            let at = text.find(piece).unwrap();
            spans
                .iter()
                .find(|s| s.range.contains(&at))
                .map(|s| s.token)
        };
        assert_eq!(token("pub"), Some(Token::Keyword));
        assert_eq!(token("add"), Some(Token::Function));
        assert_eq!(token("42"), Some(Token::Number));
        assert_eq!(token("// sum"), Some(Token::Comment));
        // The pieces cover the line, in order.
        assert_eq!(spans.first().unwrap().range.start, 0);
        assert_eq!(spans.last().unwrap().range.end, text.len());
        assert!(spans.windows(2).all(|w| w[0].range.end == w[1].range.start));
        // A file of no known kind is plain.
        let mut plain = Stream::new(syntax_for("notes.unknownext", None));
        assert!(plain.line("x = 1").iter().all(|s| s.token == Token::Plain));
    }
}
