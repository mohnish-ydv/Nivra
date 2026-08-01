//! Lossless hand-written lexer for the Nivra Edition 2026 surface syntax.
//!
//! D3 intentionally stops at tokenization. Interpolation is retained inside string tokens;
//! parser-facing interpolation modes are scheduled for the parser delivery.

use std::fmt;

use nivra_diagnostics::Diagnostic;
use nivra_source::{SourceFile, Span};

/// A reserved Edition 2026 keyword.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Keyword {
    As,
    Async,
    Await,
    Blocking,
    Break,
    Const,
    Continue,
    Defer,
    Dyn,
    Else,
    Ensure,
    Enum,
    Err,
    Extern,
    False,
    Fn,
    For,
    If,
    Impl,
    In,
    Let,
    Match,
    Module,
    Move,
    Newtype,
    None,
    Ok,
    Pub,
    Record,
    Ref,
    Return,
    SelfValue,
    Spawn,
    Struct,
    TaskGroup,
    Trait,
    True,
    Try,
    Type,
    Unsafe,
    Use,
    Var,
    Where,
    While,
    Yield,
}

impl Keyword {
    /// Resolves an identifier to a keyword.
    #[must_use]
    pub fn from_identifier(identifier: &str) -> Option<Self> {
        Some(match identifier {
            "as" => Self::As,
            "async" => Self::Async,
            "await" => Self::Await,
            "blocking" => Self::Blocking,
            "break" => Self::Break,
            "const" => Self::Const,
            "continue" => Self::Continue,
            "defer" => Self::Defer,
            "dyn" => Self::Dyn,
            "else" => Self::Else,
            "ensure" => Self::Ensure,
            "enum" => Self::Enum,
            "err" => Self::Err,
            "extern" => Self::Extern,
            "false" => Self::False,
            "fn" => Self::Fn,
            "for" => Self::For,
            "if" => Self::If,
            "impl" => Self::Impl,
            "in" => Self::In,
            "let" => Self::Let,
            "match" => Self::Match,
            "module" => Self::Module,
            "move" => Self::Move,
            "newtype" => Self::Newtype,
            "none" => Self::None,
            "ok" => Self::Ok,
            "pub" => Self::Pub,
            "record" => Self::Record,
            "ref" => Self::Ref,
            "return" => Self::Return,
            "self" => Self::SelfValue,
            "spawn" => Self::Spawn,
            "struct" => Self::Struct,
            "task_group" => Self::TaskGroup,
            "trait" => Self::Trait,
            "true" => Self::True,
            "try" => Self::Try,
            "type" => Self::Type,
            "unsafe" => Self::Unsafe,
            "use" => Self::Use,
            "var" => Self::Var,
            "where" => Self::Where,
            "while" => Self::While,
            "yield" => Self::Yield,
            _ => return None,
        })
    }

    /// Returns the source spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::As => "as",
            Self::Async => "async",
            Self::Await => "await",
            Self::Blocking => "blocking",
            Self::Break => "break",
            Self::Const => "const",
            Self::Continue => "continue",
            Self::Defer => "defer",
            Self::Dyn => "dyn",
            Self::Else => "else",
            Self::Ensure => "ensure",
            Self::Enum => "enum",
            Self::Err => "err",
            Self::Extern => "extern",
            Self::False => "false",
            Self::Fn => "fn",
            Self::For => "for",
            Self::If => "if",
            Self::Impl => "impl",
            Self::In => "in",
            Self::Let => "let",
            Self::Match => "match",
            Self::Module => "module",
            Self::Move => "move",
            Self::Newtype => "newtype",
            Self::None => "none",
            Self::Ok => "ok",
            Self::Pub => "pub",
            Self::Record => "record",
            Self::Ref => "ref",
            Self::Return => "return",
            Self::SelfValue => "self",
            Self::Spawn => "spawn",
            Self::Struct => "struct",
            Self::TaskGroup => "task_group",
            Self::Trait => "trait",
            Self::True => "true",
            Self::Try => "try",
            Self::Type => "type",
            Self::Unsafe => "unsafe",
            Self::Use => "use",
            Self::Var => "var",
            Self::Where => "where",
            Self::While => "while",
            Self::Yield => "yield",
        }
    }
}

/// Token category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenKind {
    // Lossless trivia.
    Whitespace,
    Newline,
    LineComment,
    DocLineComment,
    BlockComment,
    DocBlockComment,

    // Names and literals.
    Identifier,
    Keyword(Keyword),
    IntegerLiteral,
    FloatLiteral,
    StringLiteral,
    CharLiteral,

    // Delimiters and punctuation.
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Colon,
    ColonColon,
    Semicolon,
    Dot,
    DotDot,
    DotDotEqual,
    At,
    Question,

    // Operators.
    Plus,
    PlusEqual,
    Minus,
    MinusEqual,
    Arrow,
    Star,
    StarEqual,
    Slash,
    SlashEqual,
    Percent,
    PercentEqual,
    Equal,
    EqualEqual,
    FatArrow,
    Bang,
    BangEqual,
    Less,
    LessEqual,
    ShiftLeft,
    ShiftLeftEqual,
    Greater,
    GreaterEqual,
    ShiftRight,
    ShiftRightEqual,
    Ampersand,
    AmpersandEqual,
    AmpersandAmpersand,
    Pipe,
    PipeEqual,
    PipePipe,
    Caret,
    CaretEqual,
    Tilde,

    Unknown,
    Eof,
}

impl TokenKind {
    /// Returns whether this token is trivia retained for lossless parsing.
    #[must_use]
    pub const fn is_trivia(self) -> bool {
        matches!(
            self,
            Self::Whitespace
                | Self::Newline
                | Self::LineComment
                | Self::DocLineComment
                | Self::BlockComment
                | Self::DocBlockComment
        )
    }

    /// Returns whether this token is a reserved keyword.
    #[must_use]
    pub const fn is_keyword(self) -> bool {
        matches!(self, Self::Keyword(_))
    }

    /// Returns a stable diagnostic/debug name.
    #[must_use]
    pub fn name(self) -> String {
        match self {
            Self::Keyword(keyword) => format!("keyword({})", keyword.as_str()),
            Self::Whitespace => "whitespace".into(),
            Self::Newline => "newline".into(),
            Self::LineComment => "line_comment".into(),
            Self::DocLineComment => "doc_line_comment".into(),
            Self::BlockComment => "block_comment".into(),
            Self::DocBlockComment => "doc_block_comment".into(),
            Self::Identifier => "identifier".into(),
            Self::IntegerLiteral => "integer_literal".into(),
            Self::FloatLiteral => "float_literal".into(),
            Self::StringLiteral => "string_literal".into(),
            Self::CharLiteral => "char_literal".into(),
            Self::LeftParen => "left_paren".into(),
            Self::RightParen => "right_paren".into(),
            Self::LeftBrace => "left_brace".into(),
            Self::RightBrace => "right_brace".into(),
            Self::LeftBracket => "left_bracket".into(),
            Self::RightBracket => "right_bracket".into(),
            Self::Comma => "comma".into(),
            Self::Colon => "colon".into(),
            Self::ColonColon => "colon_colon".into(),
            Self::Semicolon => "semicolon".into(),
            Self::Dot => "dot".into(),
            Self::DotDot => "dot_dot".into(),
            Self::DotDotEqual => "dot_dot_equal".into(),
            Self::At => "at".into(),
            Self::Question => "question".into(),
            Self::Plus => "plus".into(),
            Self::PlusEqual => "plus_equal".into(),
            Self::Minus => "minus".into(),
            Self::MinusEqual => "minus_equal".into(),
            Self::Arrow => "arrow".into(),
            Self::Star => "star".into(),
            Self::StarEqual => "star_equal".into(),
            Self::Slash => "slash".into(),
            Self::SlashEqual => "slash_equal".into(),
            Self::Percent => "percent".into(),
            Self::PercentEqual => "percent_equal".into(),
            Self::Equal => "equal".into(),
            Self::EqualEqual => "equal_equal".into(),
            Self::FatArrow => "fat_arrow".into(),
            Self::Bang => "bang".into(),
            Self::BangEqual => "bang_equal".into(),
            Self::Less => "less".into(),
            Self::LessEqual => "less_equal".into(),
            Self::ShiftLeft => "shift_left".into(),
            Self::ShiftLeftEqual => "shift_left_equal".into(),
            Self::Greater => "greater".into(),
            Self::GreaterEqual => "greater_equal".into(),
            Self::ShiftRight => "shift_right".into(),
            Self::ShiftRightEqual => "shift_right_equal".into(),
            Self::Ampersand => "ampersand".into(),
            Self::AmpersandEqual => "ampersand_equal".into(),
            Self::AmpersandAmpersand => "ampersand_ampersand".into(),
            Self::Pipe => "pipe".into(),
            Self::PipeEqual => "pipe_equal".into(),
            Self::PipePipe => "pipe_pipe".into(),
            Self::Caret => "caret".into(),
            Self::CaretEqual => "caret_equal".into(),
            Self::Tilde => "tilde".into(),
            Self::Unknown => "unknown".into(),
            Self::Eof => "eof".into(),
        }
    }
}

/// One lossless token.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Token {
    /// Token category.
    pub kind: TokenKind,
    /// Exact byte range in the source.
    pub span: Span,
}

impl Token {
    /// Returns the exact token text when the token belongs to this source.
    #[must_use]
    pub fn text<'a>(self, source: &'a SourceFile) -> Option<&'a str> {
        source.slice(self.span)
    }
}

/// Lexer result.
#[derive(Clone, Debug, Default)]
pub struct Lexed {
    /// Tokens including trivia and one final EOF token.
    pub tokens: Vec<Token>,
    /// Recoverable lexical diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

impl Lexed {
    /// Returns true when at least one error was emitted.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }

    /// Iterates over non-trivia, non-EOF tokens.
    pub fn significant_tokens(&self) -> impl Iterator<Item = &Token> {
        self.tokens
            .iter()
            .filter(|token| !token.kind.is_trivia() && token.kind != TokenKind::Eof)
    }
}

/// Lexes one source file.
#[must_use]
pub fn lex(source: &SourceFile) -> Lexed {
    Lexer::new(source).run()
}

struct Lexer<'a> {
    source: &'a SourceFile,
    text: &'a str,
    position: usize,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a SourceFile) -> Self {
        Self {
            source,
            text: source.text(),
            position: 0,
            tokens: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn run(mut self) -> Lexed {
        while self.position < self.text.len() {
            let start = self.position;
            let Some(character) = self.current() else {
                break;
            };

            match character {
                '\n' | '\r' => self.lex_newline(start),
                character if is_horizontal_whitespace(character) => {
                    self.lex_whitespace(start);
                }
                '/' if self.starts_with("//") => self.lex_line_comment(start),
                '/' if self.starts_with("/*") => self.lex_block_comment(start),
                '"' => self.lex_string(start),
                '\'' => self.lex_char(start),
                character if character.is_ascii_digit() => self.lex_number(start),
                character if is_identifier_start(character) => self.lex_identifier(start),
                '\0' => {
                    self.advance();
                    self.push(TokenKind::Unknown, start);
                    self.diagnostics.push(
                        Diagnostic::error("LEX010", "NUL byte is not allowed in source")
                            .with_primary(self.span(start, self.position), "remove this byte"),
                    );
                }
                character if is_bidi_control(character) => {
                    self.advance();
                    self.push(TokenKind::Unknown, start);
                    self.diagnostics.push(
                        Diagnostic::error(
                            "LEX009",
                            "bidirectional control character outside a literal or comment",
                        )
                        .with_primary(
                            self.span(start, self.position),
                            "this invisible character can disguise source order",
                        )
                        .with_help("remove it or represent it explicitly inside a string"),
                    );
                }
                _ => self.lex_symbol_or_unknown(start),
            }
        }

        self.tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span::empty(self.source.id(), self.position),
        });

        Lexed {
            tokens: self.tokens,
            diagnostics: self.diagnostics,
        }
    }

    fn lex_newline(&mut self, start: usize) {
        if self.starts_with("\r\n") {
            self.position += 2;
        } else {
            self.advance();
        }
        self.push(TokenKind::Newline, start);
    }

    fn lex_whitespace(&mut self, start: usize) {
        while self.current().is_some_and(is_horizontal_whitespace) {
            self.advance();
        }
        self.push(TokenKind::Whitespace, start);
    }

    fn lex_line_comment(&mut self, start: usize) {
        let kind = if self.starts_with("///") && !self.starts_with("////") {
            TokenKind::DocLineComment
        } else {
            TokenKind::LineComment
        };

        while let Some(character) = self.current() {
            if matches!(character, '\n' | '\r') {
                break;
            }
            if is_bidi_control(character) {
                let control_start = self.position;
                self.advance();
                self.warn_bidi(control_start);
            } else {
                self.advance();
            }
        }
        self.push(kind, start);
    }

    fn lex_block_comment(&mut self, start: usize) {
        let kind = if self.starts_with("/**") && !self.starts_with("/***") {
            TokenKind::DocBlockComment
        } else {
            TokenKind::BlockComment
        };

        self.position += 2;
        let mut depth = 1usize;

        while self.position < self.text.len() {
            if self.starts_with("/*") {
                depth += 1;
                self.position += 2;
            } else if self.starts_with("*/") {
                depth -= 1;
                self.position += 2;
                if depth == 0 {
                    break;
                }
            } else if let Some(character) = self.current() {
                if is_bidi_control(character) {
                    let control_start = self.position;
                    self.advance();
                    self.warn_bidi(control_start);
                } else {
                    self.advance();
                }
            }
        }

        if depth != 0 {
            self.diagnostics.push(
                Diagnostic::error("LEX004", "unterminated block comment")
                    .with_primary(self.span(start, self.position), "comment starts here")
                    .with_help("add `*/` before the end of the file"),
            );
        }
        self.push(kind, start);
    }

    fn lex_string(&mut self, start: usize) {
        self.advance();
        let mut terminated = false;

        while let Some(character) = self.current() {
            match character {
                '"' => {
                    self.advance();
                    terminated = true;
                    break;
                }
                '\n' | '\r' => break,
                '\\' => self.lex_escape(),
                character if is_bidi_control(character) => {
                    let control_start = self.position;
                    self.advance();
                    self.warn_bidi(control_start);
                }
                _ => {
                    self.advance();
                }
            }
        }

        if !terminated {
            self.diagnostics.push(
                Diagnostic::error("LEX002", "unterminated string literal")
                    .with_primary(self.span(start, self.position), "string starts here")
                    .with_help("close the string with `\"` before the line ends"),
            );
        }
        self.push(TokenKind::StringLiteral, start);
    }

    fn lex_char(&mut self, start: usize) {
        self.advance();
        let content_start = self.position;
        let mut units = 0usize;
        let mut terminated = false;

        while let Some(character) = self.current() {
            match character {
                '\'' => {
                    self.advance();
                    terminated = true;
                    break;
                }
                '\n' | '\r' => break,
                '\\' => {
                    self.lex_escape();
                    units += 1;
                }
                character if is_bidi_control(character) => {
                    let control_start = self.position;
                    self.advance();
                    self.warn_bidi(control_start);
                    units += 1;
                }
                _ => {
                    self.advance();
                    units += 1;
                }
            }
        }

        if !terminated {
            self.diagnostics.push(
                Diagnostic::error("LEX007", "unterminated character literal")
                    .with_primary(self.span(start, self.position), "character starts here")
                    .with_help("close the character literal with `'`"),
            );
        } else if units != 1 {
            self.diagnostics.push(
                Diagnostic::error(
                    "LEX008",
                    "character literal must contain exactly one character",
                )
                .with_primary(
                    self.span(content_start, self.position.saturating_sub(1)),
                    if units == 0 {
                        "this character literal is empty"
                    } else {
                        "this character literal contains multiple characters"
                    },
                )
                .with_help("use a string literal when more than one character is required"),
            );
        }

        self.push(TokenKind::CharLiteral, start);
    }

    fn lex_escape(&mut self) {
        let escape_start = self.position;
        self.advance();

        let Some(character) = self.current() else {
            self.invalid_escape(escape_start, "escape sequence ends at end of file");
            return;
        };

        match character {
            '\\' | '"' | '\'' | 'n' | 'r' | 't' | '0' | '$' => {
                self.advance();
            }
            'u' => {
                self.advance();
                if self.current() != Some('{') {
                    self.invalid_escape(
                        escape_start,
                        "Unicode escape must use the form `\\u{1F680}`",
                    );
                    return;
                }
                self.advance();
                let digits_start = self.position;
                let mut digit_count = 0usize;
                while self.current().is_some_and(|value| value.is_ascii_hexdigit()) {
                    self.advance();
                    digit_count += 1;
                }

                let closed = self.current() == Some('}');
                if closed {
                    self.advance();
                }

                if digit_count == 0 || digit_count > 6 || !closed {
                    self.invalid_escape(
                        escape_start,
                        "Unicode escape needs 1 to 6 hexadecimal digits and a closing `}`",
                    );
                    return;
                }

                let digits = &self.text[digits_start..digits_start + digit_count];
                let valid_scalar = u32::from_str_radix(digits, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .is_some();
                if !valid_scalar {
                    self.invalid_escape(escape_start, "Unicode escape is not a valid scalar value");
                }
            }
            _ => {
                self.advance();
                self.invalid_escape(
                    escape_start,
                    "supported escapes include `\\n`, `\\t`, `\\\\`, and `\\u{...}`",
                );
            }
        }
    }

    fn invalid_escape(&mut self, start: usize, help: &'static str) {
        self.diagnostics.push(
            Diagnostic::error("LEX003", "invalid escape sequence")
                .with_primary(self.span(start, self.position), "invalid escape")
                .with_help(help),
        );
    }

    fn lex_identifier(&mut self, start: usize) {
        self.advance();
        while self.current().is_some_and(is_identifier_continue) {
            self.advance();
        }
        let text = &self.text[start..self.position];
        let kind = Keyword::from_identifier(text)
            .map(TokenKind::Keyword)
            .unwrap_or(TokenKind::Identifier);
        self.push(kind, start);
    }

    fn lex_number(&mut self, start: usize) {
        if self.starts_with("0x") || self.starts_with("0X") {
            self.lex_based_integer(start, 16);
            return;
        }
        if self.starts_with("0o") || self.starts_with("0O") {
            self.lex_based_integer(start, 8);
            return;
        }
        if self.starts_with("0b") || self.starts_with("0B") {
            self.lex_based_integer(start, 2);
            return;
        }

        self.consume_digits_and_underscores(10);
        let mut kind = TokenKind::IntegerLiteral;

        if self.current() == Some('.')
            && !self.starts_with("..")
            && self.peek_char(1).is_some_and(|character| character.is_ascii_digit())
        {
            kind = TokenKind::FloatLiteral;
            self.advance();
            self.consume_digits_and_underscores(10);
        }

        if matches!(self.current(), Some('e' | 'E')) {
            kind = TokenKind::FloatLiteral;
            self.advance();
            if matches!(self.current(), Some('+' | '-')) {
                self.advance();
            }
            let exponent_start = self.position;
            self.consume_digits_and_underscores(10);
            if exponent_start == self.position {
                self.diagnostics.push(
                    Diagnostic::error("LEX006", "floating-point exponent has no digits")
                        .with_primary(
                            self.span(exponent_start.saturating_sub(1), self.position),
                            "expected exponent digits here",
                        )
                        .with_help("write an exponent such as `e3` or `e-2`"),
                );
            }
        }

        let text = &self.text[start..self.position];
        if invalid_underscore_placement(text) {
            self.malformed_number(
                start,
                "underscores must appear only between digits",
            );
        }

        self.push(kind, start);
    }

    fn lex_based_integer(&mut self, start: usize, radix: u32) {
        self.position += 2;
        let digits_start = self.position;

        while let Some(character) = self.current() {
            if character == '_' || character.is_ascii_alphanumeric() {
                self.advance();
            } else {
                break;
            }
        }

        let body = &self.text[digits_start..self.position];
        let has_digit = body.chars().any(|character| character != '_');
        let valid_digits = body
            .chars()
            .filter(|character| *character != '_')
            .all(|character| character.to_digit(radix).is_some());

        if !has_digit {
            self.malformed_number(start, "base prefix must be followed by digits");
        } else if !valid_digits {
            self.malformed_number(
                start,
                match radix {
                    2 => "binary literals may contain only `0` and `1`",
                    8 => "octal literals may contain only digits `0` through `7`",
                    16 => "hexadecimal literals may contain only hexadecimal digits",
                    _ => "number contains a digit invalid for its base",
                },
            );
        } else if invalid_underscore_placement(body) {
            self.malformed_number(start, "underscores must appear only between digits");
        }

        self.push(TokenKind::IntegerLiteral, start);
    }

    fn consume_digits_and_underscores(&mut self, radix: u32) {
        while self.current().is_some_and(|character| {
            character == '_' || character.to_digit(radix).is_some()
        }) {
            self.advance();
        }
    }

    fn malformed_number(&mut self, start: usize, help: &'static str) {
        self.diagnostics.push(
            Diagnostic::error("LEX005", "malformed numeric literal")
                .with_primary(self.span(start, self.position), "invalid number")
                .with_help(help),
        );
    }

    fn lex_symbol_or_unknown(&mut self, start: usize) {
        let (kind, byte_length) = if self.starts_with("<<=") {
            (TokenKind::ShiftLeftEqual, 3)
        } else if self.starts_with(">>=") {
            (TokenKind::ShiftRightEqual, 3)
        } else if self.starts_with("..=") {
            (TokenKind::DotDotEqual, 3)
        } else if self.starts_with("::") {
            (TokenKind::ColonColon, 2)
        } else if self.starts_with("..") {
            (TokenKind::DotDot, 2)
        } else if self.starts_with("->") {
            (TokenKind::Arrow, 2)
        } else if self.starts_with("=>") {
            (TokenKind::FatArrow, 2)
        } else if self.starts_with("==") {
            (TokenKind::EqualEqual, 2)
        } else if self.starts_with("!=") {
            (TokenKind::BangEqual, 2)
        } else if self.starts_with("<=") {
            (TokenKind::LessEqual, 2)
        } else if self.starts_with(">=") {
            (TokenKind::GreaterEqual, 2)
        } else if self.starts_with("<<") {
            (TokenKind::ShiftLeft, 2)
        } else if self.starts_with(">>") {
            (TokenKind::ShiftRight, 2)
        } else if self.starts_with("&&") {
            (TokenKind::AmpersandAmpersand, 2)
        } else if self.starts_with("||") {
            (TokenKind::PipePipe, 2)
        } else if self.starts_with("+=") {
            (TokenKind::PlusEqual, 2)
        } else if self.starts_with("-=") {
            (TokenKind::MinusEqual, 2)
        } else if self.starts_with("*=") {
            (TokenKind::StarEqual, 2)
        } else if self.starts_with("/=") {
            (TokenKind::SlashEqual, 2)
        } else if self.starts_with("%=") {
            (TokenKind::PercentEqual, 2)
        } else if self.starts_with("&=") {
            (TokenKind::AmpersandEqual, 2)
        } else if self.starts_with("|=") {
            (TokenKind::PipeEqual, 2)
        } else if self.starts_with("^=") {
            (TokenKind::CaretEqual, 2)
        } else {
            let Some(character) = self.current() else {
                return;
            };
            let kind = match character {
                '(' => TokenKind::LeftParen,
                ')' => TokenKind::RightParen,
                '{' => TokenKind::LeftBrace,
                '}' => TokenKind::RightBrace,
                '[' => TokenKind::LeftBracket,
                ']' => TokenKind::RightBracket,
                ',' => TokenKind::Comma,
                ':' => TokenKind::Colon,
                ';' => TokenKind::Semicolon,
                '.' => TokenKind::Dot,
                '@' => TokenKind::At,
                '?' => TokenKind::Question,
                '+' => TokenKind::Plus,
                '-' => TokenKind::Minus,
                '*' => TokenKind::Star,
                '/' => TokenKind::Slash,
                '%' => TokenKind::Percent,
                '=' => TokenKind::Equal,
                '!' => TokenKind::Bang,
                '<' => TokenKind::Less,
                '>' => TokenKind::Greater,
                '&' => TokenKind::Ampersand,
                '|' => TokenKind::Pipe,
                '^' => TokenKind::Caret,
                '~' => TokenKind::Tilde,
                _ => {
                    self.advance();
                    self.push(TokenKind::Unknown, start);
                    self.diagnostics.push(
                        Diagnostic::error("LEX001", "unexpected character")
                            .with_primary(
                                self.span(start, self.position),
                                format!("`{character}` is not recognized by Edition 2026"),
                            )
                            .with_help("remove the character or replace it with a supported token"),
                    );
                    return;
                }
            };
            (kind, character.len_utf8())
        };

        self.position += byte_length;
        self.push(kind, start);
    }

    fn warn_bidi(&mut self, start: usize) {
        self.diagnostics.push(
            Diagnostic::warning("LEX009", "bidirectional control character in source text")
                .with_primary(
                    self.span(start, self.position),
                    "invisible direction control appears here",
                )
                .with_note("direction controls can make reviewed code differ from compiler order")
                .with_help("prefer an explicit Unicode escape when the character is intentional"),
        );
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        self.tokens.push(Token {
            kind,
            span: self.span(start, self.position),
        });
    }

    fn span(&self, start: usize, end: usize) -> Span {
        Span::new(self.source.id(), start, end)
    }

    fn current(&self) -> Option<char> {
        self.text[self.position..].chars().next()
    }

    fn peek_char(&self, distance: usize) -> Option<char> {
        self.text[self.position..].chars().nth(distance)
    }

    fn advance(&mut self) {
        if let Some(character) = self.current() {
            self.position += character.len_utf8();
        }
    }

    fn starts_with(&self, value: &str) -> bool {
        self.text[self.position..].starts_with(value)
    }
}

fn is_horizontal_whitespace(character: char) -> bool {
    character.is_whitespace() && !matches!(character, '\n' | '\r')
}

fn is_identifier_start(character: char) -> bool {
    character == '_' || character.is_alphabetic()
}

fn is_identifier_continue(character: char) -> bool {
    character == '_'
        || character.is_alphanumeric()
        || matches!(
            u32::from(character),
            0x0300..=0x036F
                | 0x0483..=0x0489
                | 0x0591..=0x05BD
                | 0x05BF
                | 0x05C1..=0x05C2
                | 0x05C4..=0x05C5
                | 0x0610..=0x061A
                | 0x064B..=0x065F
                | 0x0670
                | 0x06D6..=0x06ED
                | 0x0900..=0x0903
                | 0x093A..=0x094F
                | 0x0951..=0x0957
                | 0x0962..=0x0963
                | 0x1AB0..=0x1AFF
                | 0x1DC0..=0x1DFF
                | 0x20D0..=0x20FF
                | 0xFE20..=0xFE2F
        )
}

fn is_bidi_control(character: char) -> bool {
    matches!(
        character,
        '\u{061C}'
            | '\u{200E}'
            | '\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2066}'..='\u{2069}'
    )
}

fn invalid_underscore_placement(text: &str) -> bool {
    text.starts_with('_')
        || text.ends_with('_')
        || text.as_bytes().windows(2).any(|pair| pair == b"__")
        || text.contains("._")
        || text.contains("_.")
        || text.contains("e_")
        || text.contains("E_")
        || text.contains("_e")
        || text.contains("_E")
        || text.contains("+_")
        || text.contains("-_")
}

impl fmt::Display for Keyword {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use nivra_source::SourceManager;

    use super::{Keyword, TokenKind, lex};

    fn lex_text(text: &str) -> (nivra_source::SourceFile, super::Lexed) {
        let mut manager = SourceManager::new();
        let id = manager
            .add_virtual("test.nva", text)
            .unwrap_or_else(|error| panic!("{error}"));
        let source = manager
            .get(id)
            .unwrap_or_else(|| panic!("source should exist"))
            .clone();
        let result = lex(&source);
        (source, result)
    }

    #[test]
    fn preserves_trivia_and_recognizes_keywords() {
        let (source, result) = lex_text("module app\n/// docs\nfn main() {}\n");
        let kinds = result
            .tokens
            .iter()
            .map(|token| token.kind)
            .collect::<Vec<_>>();

        assert!(kinds.contains(&TokenKind::Keyword(Keyword::Module)));
        assert!(kinds.contains(&TokenKind::DocLineComment));
        assert!(kinds.contains(&TokenKind::Keyword(Keyword::Fn)));
        assert_eq!(result.tokens.last().map(|token| token.kind), Some(TokenKind::Eof));
        assert_eq!(
            result.tokens[0].text(&source),
            Some("module")
        );
        assert!(!result.has_errors());
    }

    #[test]
    fn supports_unicode_identifiers_with_combining_marks() {
        let (_source, result) = lex_text("let नाम = 1\nlet Δelta = 2\n");
        let identifiers = result
            .tokens
            .iter()
            .filter(|token| token.kind == TokenKind::Identifier)
            .count();

        assert_eq!(identifiers, 2);
        assert!(!result.has_errors());
    }

    #[test]
    fn handles_nested_block_comments() {
        let (_source, result) = lex_text("/* outer /* inner */ done */ let value = 1");
        assert_eq!(
            result
                .tokens
                .iter()
                .filter(|token| token.kind == TokenKind::BlockComment)
                .count(),
            1
        );
        assert!(!result.has_errors());
    }

    #[test]
    fn reports_unterminated_comment_and_string_without_panicking() {
        let (_source, result) = lex_text("/* open\n\"open");
        let codes = result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>();

        assert!(codes.contains(&"LEX004"));
        // The block comment consumes the rest of this input, so the quote is intentionally
        // not lexed as a string. A separate input checks the string case.
        let (_source, string_result) = lex_text("\"open\n");
        assert!(
            string_result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "LEX002")
        );
    }

    #[test]
    fn validates_based_numbers_and_exponents() {
        let (_source, valid) = lex_text("0b1010 0o755 0xCAFE 12.5 1e-3");
        assert!(!valid.has_errors());

        let (_source, invalid) = lex_text("0b102 0x 1e+");
        let codes = invalid
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>();
        assert!(codes.contains(&"LEX005"));
        assert!(codes.contains(&"LEX006"));
    }

    #[test]
    fn rejects_underscores_next_to_exponent_markers_and_signs() {
        let (_source, result) = lex_text("1_e3 1e_3 1e+_3 1e-_3");
        let malformed_count = result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "LEX005")
            .count();

        assert_eq!(malformed_count, 4);
    }

    #[test]
    fn uses_longest_match_for_operators() {
        let (_source, result) = lex_text("<<= >>= ..= -> => == != <= >= && ||");
        let kinds = result
            .significant_tokens()
            .map(|token| token.kind)
            .collect::<Vec<_>>();

        assert_eq!(
            kinds,
            vec![
                TokenKind::ShiftLeftEqual,
                TokenKind::ShiftRightEqual,
                TokenKind::DotDotEqual,
                TokenKind::Arrow,
                TokenKind::FatArrow,
                TokenKind::EqualEqual,
                TokenKind::BangEqual,
                TokenKind::LessEqual,
                TokenKind::GreaterEqual,
                TokenKind::AmpersandAmpersand,
                TokenKind::PipePipe,
            ]
        );
    }

    #[test]
    fn validates_character_length_and_escapes() {
        let (_source, valid) = lex_text("'a' '\\n' '\\u{1F680}'");
        assert!(!valid.has_errors());

        let (_source, invalid) = lex_text("'' 'ab' '\\q'");
        let codes = invalid
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>();
        assert!(codes.contains(&"LEX008"));
        assert!(codes.contains(&"LEX003"));
    }
}
