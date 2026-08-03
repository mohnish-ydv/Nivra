//! Lossless, error-recovering parser for the Nivra Edition 2026 surface grammar.
//!
//! D4 parses declarations, statements, types, patterns, and expressions into an
//! immutable CST. Expressions use Pratt parsing. Recovery synchronizes at declaration,
//! statement, comma, and closing-delimiter boundaries so one error does not hide later
//! diagnostics.

use nivra_diagnostics::Diagnostic;
use nivra_lexer::{lex, Keyword, Lexed, Token, TokenKind};
use nivra_source::{SourceFile, Span};
use nivra_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

/// Complete parser output.
#[derive(Clone, Debug)]
pub struct ParseResult {
    /// Lossless source-file CST.
    pub root: SyntaxNode,
    /// Lexical and parser diagnostics in source order.
    pub diagnostics: Vec<Diagnostic>,
    /// Number of diagnostics produced by the lexer.
    pub lexical_diagnostic_count: usize,
    /// Number of parser recovery regions.
    pub recovered_error_count: usize,
}

impl ParseResult {
    /// Returns whether any lexical or parser error occurred.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }

    /// Returns parser diagnostics only.
    #[must_use]
    pub fn parser_diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics[self.lexical_diagnostic_count..]
    }
}

/// Lexes and parses one source file.
#[must_use]
pub fn parse(source: &SourceFile) -> ParseResult {
    parse_lexed(source, lex(source))
}

/// Parses a previously lexed source file.
#[must_use]
pub fn parse_lexed(source: &SourceFile, lexed: Lexed) -> ParseResult {
    let Lexed {
        tokens,
        mut diagnostics,
    } = lexed;
    let lexical_diagnostic_count = diagnostics.len();
    let mut parser = Parser::new(source, &tokens);
    let root = parser.parse_source_file();
    let recovered_error_count = parser.recovered_error_count;
    diagnostics.append(&mut parser.diagnostics);

    ParseResult {
        root,
        diagnostics,
        lexical_diagnostic_count,
        recovered_error_count,
    }
}

struct Parser<'a> {
    source: &'a SourceFile,
    tokens: &'a [Token],
    position: usize,
    diagnostics: Vec<Diagnostic>,
    recovered_error_count: usize,
}

impl<'a> Parser<'a> {
    fn new(source: &'a SourceFile, tokens: &'a [Token]) -> Self {
        Self {
            source,
            tokens,
            position: 0,
            diagnostics: Vec::new(),
            recovered_error_count: 0,
        }
    }

    fn parse_source_file(&mut self) -> SyntaxNode {
        let mut children = Vec::new();
        self.eat_trivia(&mut children);

        while !self.at(TokenKind::Eof) {
            let before = self.position;
            children.push(SyntaxElement::Node(self.parse_item()));
            if self.position == before {
                children.push(SyntaxElement::Node(self.recover_item()));
            }
            self.eat_trivia(&mut children);
        }

        self.bump_raw(&mut children);
        self.node(SyntaxKind::SourceFile, children)
    }

    fn parse_item(&mut self) -> SyntaxNode {
        let mut children = Vec::new();
        while self.at(TokenKind::At) {
            children.push(SyntaxElement::Node(self.parse_attribute()));
            self.eat_trivia(&mut children);
        }

        if self.at_keyword(Keyword::Pub) {
            self.bump_significant(&mut children);
        }

        if self.at_keyword(Keyword::Async) && self.nth_kind(1) == TokenKind::Keyword(Keyword::Fn) {
            self.bump_significant(&mut children);
            return self.parse_function_with_prefix(children, SyntaxKind::FunctionDeclaration);
        }
        if self.at_keyword(Keyword::Unsafe) && self.nth_kind(1) == TokenKind::Keyword(Keyword::Fn) {
            self.bump_significant(&mut children);
            return self.parse_function_with_prefix(children, SyntaxKind::FunctionDeclaration);
        }

        match self.current_kind() {
            TokenKind::Keyword(Keyword::Module) => {
                self.parse_line_declaration(SyntaxKind::ModuleDeclaration, children, "module path")
            }
            TokenKind::Keyword(Keyword::Use) => {
                self.parse_line_declaration(SyntaxKind::UseDeclaration, children, "import path")
            }
            TokenKind::Keyword(Keyword::Const) => {
                self.parse_value_declaration(SyntaxKind::ConstDeclaration, children)
            }
            TokenKind::Keyword(Keyword::Type) => {
                self.parse_value_declaration(SyntaxKind::TypeAliasDeclaration, children)
            }
            TokenKind::Keyword(Keyword::Newtype) => {
                self.parse_value_declaration(SyntaxKind::NewtypeDeclaration, children)
            }
            TokenKind::Keyword(Keyword::Record) => {
                self.parse_nominal_declaration(SyntaxKind::RecordDeclaration, children)
            }
            TokenKind::Keyword(Keyword::Struct) => {
                self.parse_nominal_declaration(SyntaxKind::StructDeclaration, children)
            }
            TokenKind::Keyword(Keyword::Enum) => self.parse_enum(children),
            TokenKind::Keyword(Keyword::Trait) => self.parse_trait(children),
            TokenKind::Keyword(Keyword::Impl) => self.parse_impl(children),
            TokenKind::Keyword(Keyword::Fn) => {
                self.parse_function_with_prefix(children, SyntaxKind::FunctionDeclaration)
            }
            TokenKind::Keyword(Keyword::Extern) => self.parse_extern_block(children),
            _ => {
                self.error_here(
                    "PAR004",
                    "expected a top-level declaration",
                    "start a declaration with `module`, `use`, `fn`, `record`, `enum`, or another Edition 2026 declaration keyword",
                );
                let mut recovered = children;
                self.recover_until(&mut recovered, Recovery::Item);
                self.node(SyntaxKind::Error, recovered)
            }
        }
    }

    fn parse_attribute(&mut self) -> SyntaxNode {
        let mut children = Vec::new();
        self.expect(TokenKind::At, &mut children, "`@`");
        self.expect_identifier(&mut children, "attribute name");
        if self.at(TokenKind::LeftParen) {
            self.consume_balanced(
                &mut children,
                TokenKind::LeftParen,
                TokenKind::RightParen,
                "PAR003",
                "attribute argument list",
            );
        }
        self.node(SyntaxKind::Attribute, children)
    }

    fn parse_line_declaration(
        &mut self,
        kind: SyntaxKind,
        mut children: Vec<SyntaxElement>,
        expected: &'static str,
    ) -> SyntaxNode {
        self.bump_significant(&mut children);
        self.eat_trivia(&mut children);
        let content_start = self.position;
        while !self.at_raw(TokenKind::Newline)
            && !self.at_raw(TokenKind::Semicolon)
            && !self.at_raw(TokenKind::Eof)
        {
            self.bump_raw(&mut children);
        }
        if self.position == content_start {
            self.error_here(
                "PAR002",
                format!("expected {expected}"),
                "add the missing path",
            );
        }
        if self.at_raw(TokenKind::Semicolon) {
            self.bump_raw(&mut children);
        }
        self.node(kind, children)
    }

    fn parse_value_declaration(
        &mut self,
        kind: SyntaxKind,
        mut children: Vec<SyntaxElement>,
    ) -> SyntaxNode {
        self.bump_significant(&mut children);
        self.expect_identifier(&mut children, "declaration name");
        if self.at(TokenKind::Colon) {
            self.bump_significant(&mut children);
            children.push(SyntaxElement::Node(self.parse_type_until(&[
                TokenKind::Equal,
                TokenKind::Semicolon,
                TokenKind::Eof,
            ])));
        }
        self.expect(TokenKind::Equal, &mut children, "`=`");
        if !self.at_line_end() {
            children.push(SyntaxElement::Node(self.parse_expression(0)));
        } else {
            self.error_here(
                "PAR005",
                "expected an initializer expression",
                "write a value after `=`",
            );
        }
        self.eat_inline_trivia(&mut children);
        self.eat(TokenKind::Semicolon, &mut children);
        self.node(kind, children)
    }

    fn parse_nominal_declaration(
        &mut self,
        kind: SyntaxKind,
        mut children: Vec<SyntaxElement>,
    ) -> SyntaxNode {
        self.bump_significant(&mut children);
        self.expect_identifier(&mut children, "type name");
        if self.at(TokenKind::Less) {
            children.push(SyntaxElement::Node(self.parse_generic_parameters()));
        }
        if self.at_keyword(Keyword::Where) {
            children.push(SyntaxElement::Node(self.parse_where_clause()));
        }
        children.push(SyntaxElement::Node(self.parse_field_list()));
        self.node(kind, children)
    }

    fn parse_enum(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        self.bump_significant(&mut children);
        self.expect_identifier(&mut children, "enum name");
        if self.at(TokenKind::Less) {
            children.push(SyntaxElement::Node(self.parse_generic_parameters()));
        }
        self.expect(TokenKind::LeftBrace, &mut children, "`{`");
        self.eat_trivia(&mut children);
        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            let mut variant = Vec::new();
            self.expect_identifier(&mut variant, "enum variant name");
            if self.at(TokenKind::LeftParen) {
                self.consume_balanced(
                    &mut variant,
                    TokenKind::LeftParen,
                    TokenKind::RightParen,
                    "PAR003",
                    "enum payload",
                );
            } else if self.at(TokenKind::LeftBrace) {
                self.consume_balanced(
                    &mut variant,
                    TokenKind::LeftBrace,
                    TokenKind::RightBrace,
                    "PAR003",
                    "enum record payload",
                );
            }
            self.eat_inline_trivia(&mut variant);
            self.eat(TokenKind::Comma, &mut variant);
            children.push(SyntaxElement::Node(
                self.node(SyntaxKind::EnumVariant, variant),
            ));
            self.eat_trivia(&mut children);
        }
        self.expect_closing(TokenKind::RightBrace, &mut children, "enum declaration");
        self.node(SyntaxKind::EnumDeclaration, children)
    }

    fn parse_trait(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        self.bump_significant(&mut children);
        self.expect_identifier(&mut children, "trait name");
        if self.at(TokenKind::Less) {
            children.push(SyntaxElement::Node(self.parse_generic_parameters()));
        }
        if self.at_keyword(Keyword::Where) {
            children.push(SyntaxElement::Node(self.parse_where_clause()));
        }
        self.expect(TokenKind::LeftBrace, &mut children, "`{`");
        self.eat_trivia(&mut children);
        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            let before = self.position;
            children.push(SyntaxElement::Node(self.parse_item()));
            if self.position == before {
                children.push(SyntaxElement::Node(self.recover_item()));
            }
            self.eat_trivia(&mut children);
        }
        self.expect_closing(TokenKind::RightBrace, &mut children, "trait declaration");
        self.node(SyntaxKind::TraitDeclaration, children)
    }

    fn parse_impl(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        self.bump_significant(&mut children);
        if self.at(TokenKind::Less) {
            children.push(SyntaxElement::Node(self.parse_generic_parameters()));
        }
        while !self.at(TokenKind::LeftBrace) && !self.at(TokenKind::Eof) {
            if self.at_keyword(Keyword::Where) {
                children.push(SyntaxElement::Node(self.parse_where_clause()));
                break;
            }
            self.bump_significant(&mut children);
        }
        self.expect(TokenKind::LeftBrace, &mut children, "`{`");
        self.eat_trivia(&mut children);
        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            let before = self.position;
            children.push(SyntaxElement::Node(self.parse_item()));
            if self.position == before {
                children.push(SyntaxElement::Node(self.recover_item()));
            }
            self.eat_trivia(&mut children);
        }
        self.expect_closing(TokenKind::RightBrace, &mut children, "implementation");
        self.node(SyntaxKind::ImplDeclaration, children)
    }

    fn parse_function_with_prefix(
        &mut self,
        mut children: Vec<SyntaxElement>,
        kind: SyntaxKind,
    ) -> SyntaxNode {
        self.expect_keyword(Keyword::Fn, &mut children, "`fn`");
        self.expect_identifier(&mut children, "function name");
        if self.at(TokenKind::Less) {
            children.push(SyntaxElement::Node(self.parse_generic_parameters()));
        }
        children.push(SyntaxElement::Node(self.parse_parameter_list()));
        if self.at(TokenKind::Arrow) {
            self.bump_significant(&mut children);
            children.push(SyntaxElement::Node(self.parse_type_until(&[
                TokenKind::Keyword(Keyword::Where),
                TokenKind::LeftBrace,
                TokenKind::RightBrace,
                TokenKind::Semicolon,
                TokenKind::Eof,
            ])));
        }
        if self.at_keyword(Keyword::Where) {
            children.push(SyntaxElement::Node(self.parse_where_clause()));
        }
        if self.at(TokenKind::LeftBrace) {
            children.push(SyntaxElement::Node(self.parse_block()));
        } else {
            self.eat_inline_trivia(&mut children);
            self.eat(TokenKind::Semicolon, &mut children);
        }
        self.node(kind, children)
    }

    fn parse_extern_block(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        self.bump_significant(&mut children);
        if self.at(TokenKind::StringLiteral) {
            self.bump_significant(&mut children);
        }
        self.expect(TokenKind::LeftBrace, &mut children, "`{`");
        self.eat_trivia(&mut children);
        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            let mut function_prefix = Vec::new();
            if self.at_keyword(Keyword::Unsafe) {
                self.bump_significant(&mut function_prefix);
            }
            if self.at_keyword(Keyword::Fn) {
                children.push(SyntaxElement::Node(self.parse_function_with_prefix(
                    function_prefix,
                    SyntaxKind::ExternFunction,
                )));
            } else {
                children.push(SyntaxElement::Node(self.recover_item()));
            }
            self.eat_trivia(&mut children);
        }
        self.expect_closing(TokenKind::RightBrace, &mut children, "extern block");
        self.node(SyntaxKind::ExternBlock, children)
    }

    fn parse_generic_parameters(&mut self) -> SyntaxNode {
        let mut children = Vec::new();
        self.consume_balanced(
            &mut children,
            TokenKind::Less,
            TokenKind::Greater,
            "PAR003",
            "generic parameter list",
        );
        self.node(SyntaxKind::GenericParameterList, children)
    }

    fn parse_generic_arguments(&mut self) -> SyntaxNode {
        let mut children = Vec::new();
        self.consume_balanced(
            &mut children,
            TokenKind::Less,
            TokenKind::Greater,
            "PAR003",
            "generic argument list",
        );
        self.node(SyntaxKind::GenericArgumentList, children)
    }

    fn parse_parameter_list(&mut self) -> SyntaxNode {
        let mut children = Vec::new();
        self.expect(TokenKind::LeftParen, &mut children, "`(`");
        self.eat_trivia(&mut children);
        while !self.at(TokenKind::RightParen) && !self.at(TokenKind::Eof) {
            let mut parameter = Vec::new();
            self.consume_until_top_level(
                &mut parameter,
                &[TokenKind::Comma, TokenKind::RightParen],
            );
            if parameter.iter().all(is_trivia_element) {
                self.error_here(
                    "PAR002",
                    "expected a function parameter",
                    "remove the extra comma or add a parameter",
                );
            }
            self.eat_inline_trivia(&mut parameter);
            self.eat(TokenKind::Comma, &mut parameter);
            children.push(SyntaxElement::Node(
                self.node(SyntaxKind::Parameter, parameter),
            ));
            self.eat_trivia(&mut children);
        }
        self.expect_closing(TokenKind::RightParen, &mut children, "parameter list");
        self.node(SyntaxKind::ParameterList, children)
    }

    fn parse_field_list(&mut self) -> SyntaxNode {
        let mut children = Vec::new();
        self.expect(TokenKind::LeftBrace, &mut children, "`{`");
        self.eat_trivia(&mut children);
        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            let mut field = Vec::new();
            self.consume_until_field_end(&mut field);
            if field.iter().all(is_trivia_element) {
                self.recover_until(&mut field, Recovery::Field);
            }
            self.eat_inline_trivia(&mut field);
            self.eat(TokenKind::Comma, &mut field);
            children.push(SyntaxElement::Node(self.node(SyntaxKind::Field, field)));
            self.eat_trivia(&mut children);
        }
        self.expect_closing(TokenKind::RightBrace, &mut children, "field list");
        self.node(SyntaxKind::FieldList, children)
    }

    fn parse_where_clause(&mut self) -> SyntaxNode {
        let mut children = Vec::new();
        self.expect_keyword(Keyword::Where, &mut children, "`where`");
        while !self.at(TokenKind::LeftBrace)
            && !self.at(TokenKind::Semicolon)
            && !self.at(TokenKind::Eof)
        {
            self.bump_significant(&mut children);
        }
        self.node(SyntaxKind::WhereClause, children)
    }

    fn parse_type_until(&mut self, stops: &[TokenKind]) -> SyntaxNode {
        let mut children = Vec::new();
        let mut paren = 0usize;
        let mut bracket = 0usize;
        let mut angle = 0usize;

        loop {
            let raw = self.raw_kind();
            let significant = self.current_kind();
            if raw == TokenKind::Eof {
                break;
            }
            if paren == 0 && bracket == 0 && angle == 0 {
                if raw == TokenKind::Newline || stops.contains(&significant) {
                    break;
                }
            }

            match raw {
                TokenKind::LeftParen => paren += 1,
                TokenKind::RightParen if paren > 0 => paren -= 1,
                TokenKind::LeftBracket => bracket += 1,
                TokenKind::RightBracket if bracket > 0 => bracket -= 1,
                TokenKind::Less => angle += 1,
                TokenKind::Greater if angle > 0 => angle -= 1,
                TokenKind::ShiftRight if angle > 0 => angle = angle.saturating_sub(2),
                _ => {}
            }
            self.bump_raw(&mut children);
        }

        if children.iter().all(is_trivia_element) {
            self.error_here(
                "PAR002",
                "expected a type",
                "write an Edition 2026 type here",
            );
        }
        self.node(SyntaxKind::TypeReference, children)
    }

    fn parse_block(&mut self) -> SyntaxNode {
        let mut children = Vec::new();
        self.expect(TokenKind::LeftBrace, &mut children, "`{`");
        self.eat_trivia(&mut children);
        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            let before = self.position;
            children.push(SyntaxElement::Node(self.parse_statement()));
            if self.position == before {
                children.push(SyntaxElement::Node(self.recover_statement()));
            }
            self.eat_trivia(&mut children);
        }
        self.expect_closing(TokenKind::RightBrace, &mut children, "block");
        self.node(SyntaxKind::Block, children)
    }

    fn parse_statement(&mut self) -> SyntaxNode {
        match self.current_kind() {
            TokenKind::Keyword(Keyword::Let) => self.parse_binding(SyntaxKind::LetStatement),
            TokenKind::Keyword(Keyword::Var) => self.parse_binding(SyntaxKind::VarStatement),
            TokenKind::Keyword(Keyword::Return) => {
                self.parse_keyword_expression_statement(SyntaxKind::ReturnStatement)
            }
            TokenKind::Keyword(Keyword::Break) => {
                self.parse_keyword_expression_statement(SyntaxKind::BreakStatement)
            }
            TokenKind::Keyword(Keyword::Continue) => {
                self.parse_keyword_expression_statement(SyntaxKind::ContinueStatement)
            }
            TokenKind::Keyword(Keyword::Defer) => {
                self.parse_keyword_expression_statement(SyntaxKind::DeferStatement)
            }
            TokenKind::Keyword(Keyword::Ensure) => self.parse_ensure_statement(),
            TokenKind::Keyword(Keyword::While) => self.parse_while_statement(),
            TokenKind::Keyword(Keyword::For) => self.parse_for_statement(),
            _ => {
                let mut children = vec![SyntaxElement::Node(self.parse_expression(0))];
                self.eat_inline_trivia(&mut children);
                self.eat(TokenKind::Semicolon, &mut children);
                self.node(SyntaxKind::ExpressionStatement, children)
            }
        }
    }

    fn parse_binding(&mut self, kind: SyntaxKind) -> SyntaxNode {
        let mut children = Vec::new();
        self.bump_significant(&mut children);
        children.push(SyntaxElement::Node(self.parse_pattern_until(&[
            TokenKind::Colon,
            TokenKind::Equal,
            TokenKind::Semicolon,
            TokenKind::Eof,
        ])));
        if self.at(TokenKind::Colon) {
            self.bump_significant(&mut children);
            children.push(SyntaxElement::Node(self.parse_type_until(&[
                TokenKind::Equal,
                TokenKind::Semicolon,
                TokenKind::Eof,
            ])));
        }
        if self.at(TokenKind::Equal) {
            self.bump_significant(&mut children);
            if self.at_line_end() {
                self.error_here(
                    "PAR005",
                    "expected an initializer expression",
                    "write a value after `=`",
                );
            } else {
                children.push(SyntaxElement::Node(self.parse_expression(0)));
            }
        } else {
            self.error_here(
                "PAR002",
                "expected `=` in binding",
                "initialize the binding with `= expression`",
            );
        }
        self.eat_inline_trivia(&mut children);
        self.eat(TokenKind::Semicolon, &mut children);
        self.node(kind, children)
    }

    fn parse_keyword_expression_statement(&mut self, kind: SyntaxKind) -> SyntaxNode {
        let mut children = Vec::new();
        self.bump_significant(&mut children);
        if !self.at_statement_end() {
            children.push(SyntaxElement::Node(self.parse_expression(0)));
        }
        self.eat_inline_trivia(&mut children);
        self.eat(TokenKind::Semicolon, &mut children);
        self.node(kind, children)
    }

    fn parse_ensure_statement(&mut self) -> SyntaxNode {
        let mut children = Vec::new();
        self.bump_significant(&mut children);
        children.push(SyntaxElement::Node(self.parse_expression(0)));
        self.expect_keyword(Keyword::Else, &mut children, "`else`");
        children.push(SyntaxElement::Node(self.parse_expression(0)));
        self.eat_inline_trivia(&mut children);
        self.eat(TokenKind::Semicolon, &mut children);
        self.node(SyntaxKind::EnsureStatement, children)
    }

    fn parse_while_statement(&mut self) -> SyntaxNode {
        let mut children = Vec::new();
        self.bump_significant(&mut children);
        children.push(SyntaxElement::Node(self.parse_expression(0)));
        children.push(SyntaxElement::Node(self.parse_block()));
        self.node(SyntaxKind::WhileStatement, children)
    }

    fn parse_for_statement(&mut self) -> SyntaxNode {
        let mut children = Vec::new();
        self.bump_significant(&mut children);
        children.push(SyntaxElement::Node(self.parse_pattern_until(&[
            TokenKind::Keyword(Keyword::In),
            TokenKind::LeftBrace,
            TokenKind::Eof,
        ])));
        self.expect_keyword(Keyword::In, &mut children, "`in`");
        children.push(SyntaxElement::Node(self.parse_expression(0)));
        children.push(SyntaxElement::Node(self.parse_block()));
        self.node(SyntaxKind::ForStatement, children)
    }

    fn parse_pattern_until(&mut self, stops: &[TokenKind]) -> SyntaxNode {
        let mut children = Vec::new();
        self.consume_until_top_level(&mut children, stops);
        if children.iter().all(is_trivia_element) {
            self.error_here(
                "PAR002",
                "expected a pattern",
                "write a name or destructuring pattern",
            );
        }
        self.node(SyntaxKind::Pattern, children)
    }

    fn parse_expression(&mut self, minimum_binding_power: u8) -> SyntaxNode {
        let mut left = self.parse_prefix_expression();

        loop {
            if self.has_line_break_before_significant() {
                break;
            }
            if self.at(TokenKind::LeftBrace) && self.looks_like_record_expression(&left) {
                left = self.parse_record_expression(left);
                continue;
            }
            if self.at(TokenKind::LeftParen) {
                let mut children = vec![SyntaxElement::Node(left)];
                children.push(SyntaxElement::Node(self.parse_argument_list()));
                left = self.node(SyntaxKind::CallExpression, children);
                continue;
            }
            if self.at(TokenKind::LeftBracket) {
                let mut children = vec![SyntaxElement::Node(left)];
                self.bump_significant(&mut children);
                if !self.at(TokenKind::RightBracket) {
                    children.push(SyntaxElement::Node(self.parse_expression(0)));
                }
                self.expect_closing(TokenKind::RightBracket, &mut children, "index expression");
                left = self.node(SyntaxKind::IndexExpression, children);
                continue;
            }
            if self.at(TokenKind::Dot) {
                let mut children = vec![SyntaxElement::Node(left)];
                self.bump_significant(&mut children);
                self.expect_identifier_or_keyword(&mut children, "member name");
                if self.at(TokenKind::Less) && self.looks_like_generic_argument_list() {
                    children.push(SyntaxElement::Node(self.parse_generic_arguments()));
                }
                left = self.node(SyntaxKind::MemberExpression, children);
                continue;
            }
            if self.at(TokenKind::Question) {
                let mut children = vec![SyntaxElement::Node(left)];
                self.bump_significant(&mut children);
                left = self.node(SyntaxKind::TryExpression, children);
                continue;
            }
            if self.at_keyword(Keyword::Async) {
                let mut children = vec![SyntaxElement::Node(left)];
                children.push(SyntaxElement::Node(self.parse_async_expression()));
                left = self.node(SyntaxKind::CallExpression, children);
                continue;
            }

            let operator = self.current_kind();
            let Some((left_power, right_power, kind)) = infix_binding_power(operator) else {
                break;
            };
            if left_power < minimum_binding_power {
                break;
            }
            let mut children = vec![SyntaxElement::Node(left)];
            self.bump_significant(&mut children);
            children.push(SyntaxElement::Node(self.parse_expression(right_power)));
            left = self.node(kind, children);
        }

        left
    }

    fn parse_prefix_expression(&mut self) -> SyntaxNode {
        match self.current_kind() {
            TokenKind::IntegerLiteral
            | TokenKind::FloatLiteral
            | TokenKind::StringLiteral
            | TokenKind::CharLiteral
            | TokenKind::Keyword(Keyword::True)
            | TokenKind::Keyword(Keyword::False)
            | TokenKind::Keyword(Keyword::None) => {
                let mut children = Vec::new();
                self.bump_significant(&mut children);
                self.node(SyntaxKind::LiteralExpression, children)
            }
            TokenKind::Identifier
            | TokenKind::Keyword(Keyword::SelfValue)
            | TokenKind::Keyword(Keyword::Ok)
            | TokenKind::Keyword(Keyword::Err) => self.parse_name_expression(),
            TokenKind::Dot => self.parse_relative_name_expression(),
            TokenKind::LeftParen => self.parse_parenthesized_or_tuple(),
            TokenKind::LeftBracket => self.parse_array_expression(),
            TokenKind::LeftBrace => self.parse_block(),
            TokenKind::Keyword(Keyword::If) => self.parse_if_expression(),
            TokenKind::Keyword(Keyword::Match) => self.parse_match_expression(),
            TokenKind::Keyword(Keyword::Unsafe) => self.parse_unsafe_expression(),
            TokenKind::Keyword(Keyword::TaskGroup) => self.parse_task_group_expression(),
            TokenKind::Keyword(Keyword::Async) => self.parse_async_expression(),
            TokenKind::Keyword(Keyword::Try) => {
                self.parse_prefix_keyword(SyntaxKind::TryExpression)
            }
            TokenKind::Keyword(Keyword::Await) => {
                self.parse_prefix_keyword(SyntaxKind::AwaitExpression)
            }
            TokenKind::Keyword(Keyword::Spawn) => {
                self.parse_prefix_keyword(SyntaxKind::SpawnExpression)
            }
            TokenKind::Keyword(Keyword::Blocking)
            | TokenKind::Keyword(Keyword::Yield)
            | TokenKind::Keyword(Keyword::Move) => {
                self.parse_prefix_keyword(SyntaxKind::PrefixExpression)
            }
            TokenKind::Ampersand => {
                let mut children = Vec::new();
                self.bump_significant(&mut children);
                if self.at_identifier_text("mut") {
                    self.bump_significant(&mut children);
                }
                children.push(SyntaxElement::Node(self.parse_expression(23)));
                self.node(SyntaxKind::PrefixExpression, children)
            }
            TokenKind::Bang
            | TokenKind::Minus
            | TokenKind::Plus
            | TokenKind::Tilde
            | TokenKind::Star => {
                let mut children = Vec::new();
                self.bump_significant(&mut children);
                children.push(SyntaxElement::Node(self.parse_expression(23)));
                self.node(SyntaxKind::PrefixExpression, children)
            }
            TokenKind::Pipe => self.parse_closure_expression(),
            _ => {
                let mut children = Vec::new();
                self.error_here(
                    "PAR005",
                    "expected an expression",
                    "write a literal, name, call, block, or control-flow expression",
                );
                if !self.at(TokenKind::Eof) {
                    self.bump_significant(&mut children);
                }
                self.node(SyntaxKind::Error, children)
            }
        }
    }

    fn parse_name_expression(&mut self) -> SyntaxNode {
        let mut children = Vec::new();
        self.bump_significant(&mut children);
        while self.at(TokenKind::ColonColon) {
            self.bump_significant(&mut children);
            self.expect_identifier_or_keyword(&mut children, "path segment");
        }
        if self.at(TokenKind::Less) && self.looks_like_generic_argument_list() {
            children.push(SyntaxElement::Node(self.parse_generic_arguments()));
        }
        self.node(SyntaxKind::NameExpression, children)
    }

    fn parse_relative_name_expression(&mut self) -> SyntaxNode {
        let mut children = Vec::new();
        self.bump_significant(&mut children);
        self.expect_identifier_or_keyword(&mut children, "enum variant name");
        self.node(SyntaxKind::NameExpression, children)
    }

    fn parse_parenthesized_or_tuple(&mut self) -> SyntaxNode {
        let mut children = Vec::new();
        self.bump_significant(&mut children);
        let mut tuple = false;
        if !self.at(TokenKind::RightParen) {
            children.push(SyntaxElement::Node(self.parse_expression(0)));
            while self.at(TokenKind::Comma) {
                tuple = true;
                self.bump_significant(&mut children);
                if self.at(TokenKind::RightParen) {
                    break;
                }
                children.push(SyntaxElement::Node(self.parse_expression(0)));
            }
        }
        self.expect_closing(
            TokenKind::RightParen,
            &mut children,
            "parenthesized expression",
        );
        self.node(
            if tuple {
                SyntaxKind::TupleExpression
            } else {
                SyntaxKind::ParenthesizedExpression
            },
            children,
        )
    }

    fn parse_array_expression(&mut self) -> SyntaxNode {
        let mut children = Vec::new();
        self.bump_significant(&mut children);
        while !self.at(TokenKind::RightBracket) && !self.at(TokenKind::Eof) {
            children.push(SyntaxElement::Node(self.parse_expression(0)));
            if !self.eat(TokenKind::Comma, &mut children) {
                break;
            }
        }
        self.expect_closing(TokenKind::RightBracket, &mut children, "array expression");
        self.node(SyntaxKind::ArrayExpression, children)
    }

    fn parse_if_expression(&mut self) -> SyntaxNode {
        let mut children = Vec::new();
        self.bump_significant(&mut children);
        if self.at_keyword(Keyword::Let) {
            self.bump_significant(&mut children);
            children.push(SyntaxElement::Node(self.parse_pattern_until(&[
                TokenKind::Equal,
                TokenKind::LeftBrace,
                TokenKind::Eof,
            ])));
            self.expect(TokenKind::Equal, &mut children, "`=`");
        }
        children.push(SyntaxElement::Node(self.parse_expression(0)));
        children.push(SyntaxElement::Node(self.parse_block()));
        if self.at_keyword(Keyword::Else) {
            self.bump_significant(&mut children);
            if self.at_keyword(Keyword::If) {
                children.push(SyntaxElement::Node(self.parse_if_expression()));
            } else {
                children.push(SyntaxElement::Node(self.parse_block()));
            }
        }
        self.node(SyntaxKind::IfExpression, children)
    }

    fn parse_match_expression(&mut self) -> SyntaxNode {
        let mut children = Vec::new();
        self.bump_significant(&mut children);
        children.push(SyntaxElement::Node(self.parse_expression(0)));
        self.expect(TokenKind::LeftBrace, &mut children, "`{`");
        self.eat_trivia(&mut children);
        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            let mut arm = Vec::new();
            arm.push(SyntaxElement::Node(self.parse_pattern_until(&[
                TokenKind::FatArrow,
                TokenKind::RightBrace,
                TokenKind::Eof,
            ])));
            self.expect(TokenKind::FatArrow, &mut arm, "`=>`");
            arm.push(SyntaxElement::Node(self.parse_expression(0)));
            self.eat_inline_trivia(&mut arm);
            self.eat(TokenKind::Comma, &mut arm);
            children.push(SyntaxElement::Node(self.node(SyntaxKind::MatchArm, arm)));
            self.eat_trivia(&mut children);
        }
        self.expect_closing(TokenKind::RightBrace, &mut children, "match expression");
        self.node(SyntaxKind::MatchExpression, children)
    }

    fn parse_unsafe_expression(&mut self) -> SyntaxNode {
        let mut children = Vec::new();
        self.bump_significant(&mut children);
        if self.at(TokenKind::LeftParen) {
            self.consume_balanced(
                &mut children,
                TokenKind::LeftParen,
                TokenKind::RightParen,
                "PAR003",
                "unsafe capability list",
            );
        }
        children.push(SyntaxElement::Node(self.parse_block()));
        self.node(SyntaxKind::UnsafeExpression, children)
    }

    fn parse_task_group_expression(&mut self) -> SyntaxNode {
        let mut children = Vec::new();
        self.bump_significant(&mut children);
        if self.at(TokenKind::Identifier) {
            self.bump_significant(&mut children);
        }
        children.push(SyntaxElement::Node(self.parse_block()));
        self.node(SyntaxKind::TaskGroupExpression, children)
    }

    fn parse_async_expression(&mut self) -> SyntaxNode {
        let mut children = Vec::new();
        self.bump_significant(&mut children);
        if self.at(TokenKind::LeftBrace) {
            children.push(SyntaxElement::Node(self.parse_block()));
        } else {
            children.push(SyntaxElement::Node(self.parse_expression(23)));
        }
        self.node(SyntaxKind::AsyncExpression, children)
    }

    fn parse_prefix_keyword(&mut self, kind: SyntaxKind) -> SyntaxNode {
        let mut children = Vec::new();
        self.bump_significant(&mut children);
        children.push(SyntaxElement::Node(self.parse_expression(23)));
        self.node(kind, children)
    }

    fn parse_closure_expression(&mut self) -> SyntaxNode {
        let mut children = Vec::new();
        self.bump_significant(&mut children);
        while !self.at(TokenKind::Pipe) && !self.at(TokenKind::Eof) {
            self.bump_significant(&mut children);
        }
        self.expect_closing(TokenKind::Pipe, &mut children, "closure parameter list");
        if self.at(TokenKind::LeftBrace) {
            children.push(SyntaxElement::Node(self.parse_block()));
        } else {
            children.push(SyntaxElement::Node(self.parse_expression(0)));
        }
        self.node(SyntaxKind::ClosureExpression, children)
    }

    fn looks_like_generic_argument_list(&self) -> bool {
        if !self.at(TokenKind::Less) {
            return false;
        }

        let mut index = self.position;
        let mut depth = 0usize;
        while let Some(token) = self.tokens.get(index) {
            if token.kind.is_trivia() {
                index += 1;
                continue;
            }
            match token.kind {
                TokenKind::Less => depth += 1,
                TokenKind::Greater => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return self.tokens[index + 1..]
                            .iter()
                            .find(|next| !next.kind.is_trivia())
                            .is_some_and(|next| {
                                matches!(
                                    next.kind,
                                    TokenKind::LeftParen
                                        | TokenKind::LeftBrace
                                        | TokenKind::Dot
                                        | TokenKind::ColonColon
                                )
                            });
                    }
                }
                TokenKind::ShiftRight if depth >= 2 => {
                    depth -= 2;
                    if depth == 0 {
                        return self.tokens[index + 1..]
                            .iter()
                            .find(|next| !next.kind.is_trivia())
                            .is_some_and(|next| {
                                matches!(
                                    next.kind,
                                    TokenKind::LeftParen
                                        | TokenKind::LeftBrace
                                        | TokenKind::Dot
                                        | TokenKind::ColonColon
                                )
                            });
                    }
                }
                TokenKind::Eof | TokenKind::Newline if depth == 0 => return false,
                _ => {}
            }
            index += 1;
        }
        false
    }

    fn looks_like_record_expression(&self, left: &SyntaxNode) -> bool {
        if left.kind() != SyntaxKind::NameExpression || !self.at(TokenKind::LeftBrace) {
            return false;
        }
        if self.nth_kind(1) == TokenKind::Identifier && self.nth_kind(2) == TokenKind::Colon {
            return true;
        }
        if self.nth_kind(1) != TokenKind::RightBrace {
            return false;
        }
        left.lossless_text(self.source)
            .trim()
            .rsplit("::")
            .next()
            .and_then(|segment| segment.chars().next())
            .is_some_and(char::is_uppercase)
    }

    fn parse_record_expression(&mut self, left: SyntaxNode) -> SyntaxNode {
        let mut children = vec![SyntaxElement::Node(left)];
        self.expect(TokenKind::LeftBrace, &mut children, "`{`");
        self.eat_trivia(&mut children);
        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            let mut field = Vec::new();
            self.expect_identifier(&mut field, "record field name");
            self.expect(TokenKind::Colon, &mut field, "`:`");
            field.push(SyntaxElement::Node(self.parse_expression(0)));
            self.eat_inline_trivia(&mut field);
            self.eat(TokenKind::Comma, &mut field);
            children.push(SyntaxElement::Node(
                self.node(SyntaxKind::RecordFieldInitializer, field),
            ));
            self.eat_trivia(&mut children);
        }
        self.expect_closing(TokenKind::RightBrace, &mut children, "record expression");
        self.node(SyntaxKind::RecordExpression, children)
    }

    fn parse_argument_list(&mut self) -> SyntaxNode {
        let mut children = Vec::new();
        self.bump_significant(&mut children);
        while !self.at(TokenKind::RightParen) && !self.at(TokenKind::Eof) {
            if self.at(TokenKind::Identifier) && self.nth_kind(1) == TokenKind::Colon {
                self.bump_significant(&mut children);
                self.bump_significant(&mut children);
            }
            children.push(SyntaxElement::Node(self.parse_expression(0)));
            if !self.eat(TokenKind::Comma, &mut children) {
                break;
            }
        }
        self.expect_closing(TokenKind::RightParen, &mut children, "argument list");
        self.node(SyntaxKind::ArgumentList, children)
    }

    fn consume_balanced(
        &mut self,
        output: &mut Vec<SyntaxElement>,
        open: TokenKind,
        close: TokenKind,
        code: &'static str,
        context: &'static str,
    ) {
        self.expect(open, output, open.name().as_str());
        let mut depth = 1usize;
        while depth > 0 && !self.at_raw(TokenKind::Eof) {
            let kind = self.raw_kind();
            if kind == open {
                depth += 1;
            } else if kind == close {
                depth -= 1;
            } else if open == TokenKind::Less
                && close == TokenKind::Greater
                && kind == TokenKind::ShiftRight
            {
                depth = depth.saturating_sub(2);
            }
            self.bump_raw(output);
        }
        if depth > 0 {
            self.diagnostics.push(
                Diagnostic::error(code, format!("unclosed {context}"))
                    .with_primary(
                        self.current_span(),
                        format!("expected `{}` before end of file", close.name()),
                    )
                    .with_help(format!("add the missing `{}`", close.name())),
            );
        }
    }

    fn consume_until_top_level(&mut self, output: &mut Vec<SyntaxElement>, stops: &[TokenKind]) {
        let mut paren = 0usize;
        let mut bracket = 0usize;
        let mut brace = 0usize;
        let mut angle = 0usize;
        loop {
            let kind = self.current_kind();
            if kind == TokenKind::Eof {
                break;
            }
            if paren == 0 && bracket == 0 && brace == 0 && angle == 0 && stops.contains(&kind) {
                break;
            }
            match kind {
                TokenKind::LeftParen => paren += 1,
                TokenKind::RightParen if paren > 0 => paren -= 1,
                TokenKind::LeftBracket => bracket += 1,
                TokenKind::RightBracket if bracket > 0 => bracket -= 1,
                TokenKind::LeftBrace => brace += 1,
                TokenKind::RightBrace if brace > 0 => brace -= 1,
                TokenKind::Less => angle += 1,
                TokenKind::Greater if angle > 0 => angle -= 1,
                TokenKind::ShiftRight if angle > 0 => angle = angle.saturating_sub(2),
                _ => {}
            }
            self.bump_significant(output);
        }
    }

    fn consume_until_field_end(&mut self, output: &mut Vec<SyntaxElement>) {
        let mut depth = 0usize;
        while !self.at_raw(TokenKind::Eof) {
            let kind = self.raw_kind();
            if depth == 0
                && matches!(
                    kind,
                    TokenKind::Newline | TokenKind::Comma | TokenKind::RightBrace
                )
            {
                break;
            }
            match kind {
                TokenKind::LeftParen | TokenKind::LeftBracket | TokenKind::LeftBrace => depth += 1,
                TokenKind::RightParen | TokenKind::RightBracket | TokenKind::RightBrace
                    if depth > 0 =>
                {
                    depth -= 1;
                }
                _ => {}
            }
            self.bump_raw(output);
        }
    }

    fn recover_item(&mut self) -> SyntaxNode {
        self.diagnostics.push(
            Diagnostic::error("PAR001", "unexpected token in declaration context")
                .with_primary(self.current_span(), "this token cannot begin a declaration")
                .with_help("remove the token or start a valid declaration"),
        );
        let mut children = Vec::new();
        self.recover_until(&mut children, Recovery::Item);
        self.node(SyntaxKind::Error, children)
    }

    fn recover_statement(&mut self) -> SyntaxNode {
        self.diagnostics.push(
            Diagnostic::error("PAR001", "unexpected token in statement context")
                .with_primary(
                    self.current_span(),
                    "this token cannot continue the statement",
                )
                .with_help("finish the current statement or start a new one"),
        );
        let mut children = Vec::new();
        self.recover_until(&mut children, Recovery::Statement);
        self.node(SyntaxKind::Error, children)
    }

    fn recover_until(&mut self, output: &mut Vec<SyntaxElement>, recovery: Recovery) {
        self.recovered_error_count += 1;
        let start = self.position;
        while !self.at_raw(TokenKind::Eof) {
            let raw = self.raw_kind();
            let significant = self.current_kind();
            let synchronized = match recovery {
                Recovery::Item => {
                    raw == TokenKind::Newline
                        || raw == TokenKind::Semicolon
                        || raw == TokenKind::RightBrace
                        || is_item_start(significant)
                }
                Recovery::Statement => {
                    raw == TokenKind::Newline
                        || raw == TokenKind::Semicolon
                        || raw == TokenKind::RightBrace
                        || is_statement_start(significant)
                }
                Recovery::Field => {
                    raw == TokenKind::Newline
                        || raw == TokenKind::Comma
                        || raw == TokenKind::RightBrace
                }
            };
            if synchronized && self.position > start {
                break;
            }
            self.bump_raw(output);
            if self.position > start && synchronized {
                break;
            }
        }
        if self.position == start && !self.at_raw(TokenKind::Eof) {
            self.bump_raw(output);
        }
    }

    fn expect_identifier(&mut self, output: &mut Vec<SyntaxElement>, expected: &'static str) {
        self.eat_trivia(output);
        if self.at_raw(TokenKind::Identifier) {
            self.bump_raw(output);
        } else {
            self.missing_expected(expected);
        }
    }

    fn expect_identifier_or_keyword(
        &mut self,
        output: &mut Vec<SyntaxElement>,
        expected: &'static str,
    ) {
        self.eat_trivia(output);
        if matches!(
            self.raw_kind(),
            TokenKind::Identifier | TokenKind::Keyword(_)
        ) {
            self.bump_raw(output);
        } else {
            self.missing_expected(expected);
        }
    }

    fn expect_keyword(
        &mut self,
        keyword: Keyword,
        output: &mut Vec<SyntaxElement>,
        expected: &'static str,
    ) {
        self.expect(TokenKind::Keyword(keyword), output, expected);
    }

    fn expect(&mut self, kind: TokenKind, output: &mut Vec<SyntaxElement>, expected: &str) {
        self.eat_trivia(output);
        if self.at_raw(kind) {
            self.bump_raw(output);
        } else {
            self.missing_expected(expected);
        }
    }

    fn expect_closing(
        &mut self,
        kind: TokenKind,
        output: &mut Vec<SyntaxElement>,
        context: &'static str,
    ) {
        self.eat_trivia(output);
        if self.at_raw(kind) {
            self.bump_raw(output);
        } else {
            self.diagnostics.push(
                Diagnostic::error("PAR003", format!("unclosed {context}"))
                    .with_primary(
                        self.current_span(),
                        format!("expected `{}` here", kind.name()),
                    )
                    .with_help(format!("add the missing `{}`", kind.name())),
            );
        }
    }

    fn missing_expected(&mut self, expected: &str) {
        self.diagnostics.push(
            Diagnostic::error("PAR002", format!("expected {expected}"))
                .with_primary(self.current_span(), format!("{expected} is required here"))
                .with_help(format!("add {expected} before this token")),
        );
    }

    fn error_here(
        &mut self,
        code: &'static str,
        message: impl Into<String>,
        help: impl Into<String>,
    ) {
        self.diagnostics.push(
            Diagnostic::error(code, message)
                .with_primary(
                    self.current_span(),
                    "parser could not continue from this token",
                )
                .with_help(help),
        );
    }

    fn eat(&mut self, kind: TokenKind, output: &mut Vec<SyntaxElement>) -> bool {
        self.eat_trivia(output);
        if self.at_raw(kind) {
            self.bump_raw(output);
            true
        } else {
            false
        }
    }

    fn eat_trivia(&mut self, output: &mut Vec<SyntaxElement>) {
        while self.raw_kind().is_trivia() {
            self.bump_raw(output);
        }
    }

    fn eat_inline_trivia(&mut self, output: &mut Vec<SyntaxElement>) {
        while matches!(
            self.raw_kind(),
            TokenKind::Whitespace
                | TokenKind::LineComment
                | TokenKind::DocLineComment
                | TokenKind::BlockComment
                | TokenKind::DocBlockComment
        ) {
            self.bump_raw(output);
        }
    }

    fn bump_significant(&mut self, output: &mut Vec<SyntaxElement>) {
        self.eat_trivia(output);
        self.bump_raw(output);
    }

    fn bump_raw(&mut self, output: &mut Vec<SyntaxElement>) {
        if let Some(token) = self.tokens.get(self.position).copied() {
            output.push(SyntaxElement::Token(SyntaxToken::new(token)));
            self.position += 1;
        }
    }

    fn current_kind(&self) -> TokenKind {
        self.nth_kind(0)
    }

    fn nth_kind(&self, distance: usize) -> TokenKind {
        self.tokens
            .iter()
            .skip(self.position)
            .filter(|token| !token.kind.is_trivia())
            .nth(distance)
            .map_or(TokenKind::Eof, |token| token.kind)
    }

    fn raw_kind(&self) -> TokenKind {
        self.tokens
            .get(self.position)
            .map_or(TokenKind::Eof, |token| token.kind)
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.current_kind() == kind
    }

    fn at_raw(&self, kind: TokenKind) -> bool {
        self.raw_kind() == kind
    }

    fn at_keyword(&self, keyword: Keyword) -> bool {
        self.at(TokenKind::Keyword(keyword))
    }

    fn at_line_end(&self) -> bool {
        let mut index = self.position;
        while let Some(token) = self.tokens.get(index) {
            match token.kind {
                TokenKind::Whitespace
                | TokenKind::LineComment
                | TokenKind::DocLineComment
                | TokenKind::BlockComment
                | TokenKind::DocBlockComment => index += 1,
                TokenKind::Newline | TokenKind::Semicolon | TokenKind::Eof => return true,
                _ => return false,
            }
        }
        true
    }

    fn at_statement_end(&self) -> bool {
        self.at_line_end() || self.at(TokenKind::RightBrace)
    }

    fn has_line_break_before_significant(&self) -> bool {
        self.tokens
            .iter()
            .skip(self.position)
            .take_while(|token| token.kind.is_trivia())
            .any(|token| token.kind == TokenKind::Newline)
    }

    fn at_identifier_text(&self, expected: &str) -> bool {
        self.tokens
            .iter()
            .skip(self.position)
            .find(|token| !token.kind.is_trivia())
            .is_some_and(|token| {
                token.kind == TokenKind::Identifier && token.text(self.source) == Some(expected)
            })
    }

    fn current_span(&self) -> Span {
        self.tokens
            .iter()
            .skip(self.position)
            .find(|token| !token.kind.is_trivia())
            .or_else(|| self.tokens.last())
            .map_or_else(
                || Span::empty(self.source.id(), self.source.len()),
                |token| token.span,
            )
    }

    fn node(&self, kind: SyntaxKind, children: Vec<SyntaxElement>) -> SyntaxNode {
        SyntaxNode::new(kind, children, self.current_span())
    }
}

#[derive(Clone, Copy)]
enum Recovery {
    Item,
    Statement,
    Field,
}

fn is_trivia_element(element: &SyntaxElement) -> bool {
    matches!(element, SyntaxElement::Token(token) if token.kind().is_trivia())
}

fn is_item_start(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::At
            | TokenKind::Keyword(Keyword::Pub)
            | TokenKind::Keyword(Keyword::Async)
            | TokenKind::Keyword(Keyword::Unsafe)
            | TokenKind::Keyword(Keyword::Module)
            | TokenKind::Keyword(Keyword::Use)
            | TokenKind::Keyword(Keyword::Const)
            | TokenKind::Keyword(Keyword::Type)
            | TokenKind::Keyword(Keyword::Newtype)
            | TokenKind::Keyword(Keyword::Record)
            | TokenKind::Keyword(Keyword::Struct)
            | TokenKind::Keyword(Keyword::Enum)
            | TokenKind::Keyword(Keyword::Trait)
            | TokenKind::Keyword(Keyword::Impl)
            | TokenKind::Keyword(Keyword::Fn)
            | TokenKind::Keyword(Keyword::Extern)
    )
}

fn is_statement_start(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Keyword(Keyword::Let)
            | TokenKind::Keyword(Keyword::Var)
            | TokenKind::Keyword(Keyword::Return)
            | TokenKind::Keyword(Keyword::Break)
            | TokenKind::Keyword(Keyword::Continue)
            | TokenKind::Keyword(Keyword::Defer)
            | TokenKind::Keyword(Keyword::Ensure)
            | TokenKind::Keyword(Keyword::While)
            | TokenKind::Keyword(Keyword::For)
            | TokenKind::Keyword(Keyword::If)
            | TokenKind::Keyword(Keyword::Match)
    )
}

fn infix_binding_power(kind: TokenKind) -> Option<(u8, u8, SyntaxKind)> {
    Some(match kind {
        TokenKind::Equal
        | TokenKind::PlusEqual
        | TokenKind::MinusEqual
        | TokenKind::StarEqual
        | TokenKind::SlashEqual
        | TokenKind::PercentEqual
        | TokenKind::AmpersandEqual
        | TokenKind::PipeEqual
        | TokenKind::CaretEqual
        | TokenKind::ShiftLeftEqual
        | TokenKind::ShiftRightEqual => (1, 1, SyntaxKind::AssignmentExpression),
        TokenKind::DotDot | TokenKind::DotDotEqual => (2, 3, SyntaxKind::RangeExpression),
        TokenKind::PipePipe => (4, 5, SyntaxKind::BinaryExpression),
        TokenKind::AmpersandAmpersand => (6, 7, SyntaxKind::BinaryExpression),
        TokenKind::Pipe => (8, 9, SyntaxKind::BinaryExpression),
        TokenKind::Caret => (10, 11, SyntaxKind::BinaryExpression),
        TokenKind::Ampersand => (12, 13, SyntaxKind::BinaryExpression),
        TokenKind::EqualEqual | TokenKind::BangEqual => (14, 15, SyntaxKind::BinaryExpression),
        TokenKind::Less | TokenKind::LessEqual | TokenKind::Greater | TokenKind::GreaterEqual => {
            (16, 17, SyntaxKind::BinaryExpression)
        }
        TokenKind::ShiftLeft | TokenKind::ShiftRight => (18, 19, SyntaxKind::BinaryExpression),
        TokenKind::Plus | TokenKind::Minus => (20, 21, SyntaxKind::BinaryExpression),
        TokenKind::Star | TokenKind::Slash | TokenKind::Percent => {
            (22, 23, SyntaxKind::BinaryExpression)
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use nivra_source::SourceManager;
    use nivra_syntax::{AstNode, SourceFileAst, SyntaxKind};

    use super::parse;

    fn parse_text(text: &str) -> (nivra_source::SourceFile, super::ParseResult) {
        let mut manager = SourceManager::new();
        let id = manager
            .add_virtual("test.nva", text)
            .unwrap_or_else(|error| panic!("{error}"));
        let source = manager
            .get(id)
            .unwrap_or_else(|| panic!("source should exist"))
            .clone();
        let result = parse(&source);
        (source, result)
    }

    fn contains_kind(node: &nivra_syntax::SyntaxNode, kind: SyntaxKind) -> bool {
        node.kind() == kind || node.child_nodes().any(|child| contains_kind(child, kind))
    }

    #[test]
    fn parses_module_function_and_preserves_source() {
        let text = "module demo\n\nfn main() {\n    let value = 1 + 2 * 3\n}\n";
        let (source, result) = parse_text(text);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert_eq!(result.root.lossless_text(&source), text);
        assert!(contains_kind(&result.root, SyntaxKind::FunctionDeclaration));
        assert!(contains_kind(&result.root, SyntaxKind::BinaryExpression));
    }

    #[test]
    fn builds_typed_source_file_view() {
        let (_source, result) = parse_text("module demo\nfn main() {}\n");
        let ast = SourceFileAst::cast(&result.root).unwrap_or_else(|| panic!("root cast"));
        assert_eq!(ast.declarations().count(), 2);
    }

    #[test]
    fn pratt_parser_honors_multiplication_precedence() {
        let (_source, result) = parse_text("fn value() { 1 + 2 * 3 }\n");
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        let tree = format!("{:?}", result.root);
        assert!(tree.matches("BinaryExpression").count() >= 2);
    }

    #[test]
    fn recovers_after_missing_expression() {
        let (_source, result) = parse_text(
            "fn broken() {\n    let first =\n    let second = 2\n    print(second)\n}\n",
        );
        assert!(result.has_errors());
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "PAR005"));
        assert!(contains_kind(&result.root, SyntaxKind::CallExpression));
    }

    #[test]
    fn reports_unclosed_block() {
        let (_source, result) = parse_text("fn main() {\n    let value = 1\n");
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "PAR003"));
    }

    #[test]
    fn parses_records_enums_traits_and_impls() {
        let (_source, result) = parse_text(
            "record User {\n name: String\n}\n\nenum State { idle, ready(User) }\n\ntrait Show { fn show(self: &Self) -> String }\nimpl Show for User { fn show(self: &Self) -> String { self.name } }\n",
        );
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert!(contains_kind(&result.root, SyntaxKind::RecordDeclaration));
        assert!(contains_kind(&result.root, SyntaxKind::EnumDeclaration));
        assert!(contains_kind(&result.root, SyntaxKind::TraitDeclaration));
        assert!(contains_kind(&result.root, SyntaxKind::ImplDeclaration));
    }

    #[test]
    fn parses_control_flow_and_calls() {
        let (_source, result) = parse_text(
            "fn run(items: List<Int>) {\n for item in items {\n  if item > 0 { print(item) } else { print(0) }\n }\n}\n",
        );
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert!(contains_kind(&result.root, SyntaxKind::ForStatement));
        assert!(contains_kind(&result.root, SyntaxKind::IfExpression));
        assert!(contains_kind(&result.root, SyntaxKind::CallExpression));
    }
    #[test]
    fn parses_method_style_spawn_with_async_block() {
        let (_source, result) = parse_text(
            "async fn run(paths: List<Path>) { task_group group { let tasks = paths.map(|path| group.spawn async { try await load(path) }) } }\n",
        );
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert!(contains_kind(&result.root, SyntaxKind::ClosureExpression));
        assert!(contains_kind(&result.root, SyntaxKind::AsyncExpression));
        assert!(contains_kind(&result.root, SyntaxKind::TaskGroupExpression));
    }

    #[test]
    fn parses_record_construction_losslessly() {
        let text =
            "record User {\n name: String\n age: Int\n}\nfn main() { let user = User { name: \"M\", age: 13 } }\n";
        let (source, result) = parse_text(text);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert_eq!(result.root.lossless_text(&source), text);
        assert!(contains_kind(&result.root, SyntaxKind::RecordExpression));
        assert!(contains_kind(
            &result.root,
            SyntaxKind::RecordFieldInitializer
        ));
    }

    #[test]
    fn does_not_confuse_if_blocks_with_record_construction() {
        let (_source, result) =
            parse_text("fn main() { let enabled = true\n if enabled { print(\"yes\") } }\n");
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert!(contains_kind(&result.root, SyntaxKind::IfExpression));
        assert!(!contains_kind(&result.root, SyntaxKind::RecordExpression));
    }

    #[test]
    fn parses_empty_record_construction_after_leading_trivia() {
        let text = "record Empty {}\nfn main() { let value = Empty { } }\n";
        let (source, result) = parse_text(text);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert_eq!(result.root.lossless_text(&source), text);
        assert!(contains_kind(&result.root, SyntaxKind::RecordExpression));
    }

    #[test]
    fn parses_explicit_generic_call_arguments() {
        let text = "fn identity<T>(value: T) -> T { value }\nfn main() { let value = identity<Int>(1) }\n";
        let (source, result) = parse_text(text);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert_eq!(result.root.lossless_text(&source), text);
        assert!(contains_kind(
            &result.root,
            SyntaxKind::GenericArgumentList
        ));
        assert!(contains_kind(&result.root, SyntaxKind::CallExpression));
    }

    #[test]
    fn keeps_comparisons_out_of_generic_argument_parsing() {
        let text = "fn main() { let value = left < right && right > limit }\n";
        let (_source, result) = parse_text(text);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert!(!contains_kind(
            &result.root,
            SyntaxKind::GenericArgumentList
        ));
        assert!(contains_kind(&result.root, SyntaxKind::BinaryExpression));
    }

    #[test]
    fn preserves_impl_generic_parameter_nodes() {
        let text = "record Box<T> { value: T }\nimpl<T> Box<T> { fn get(self: &Self) -> T { self.value } }\n";
        let (source, result) = parse_text(text);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert_eq!(result.root.lossless_text(&source), text);
        let count = format!("{:?}", result.root)
            .matches("GenericParameterList")
            .count();
        assert!(count >= 2);
    }

    #[test]
    fn parses_where_clause_after_return_type() {
        let text = "fn render<T>(value: T) -> String where T: Display { value.display() }\n";
        let (source, result) = parse_text(text);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert_eq!(result.root.lossless_text(&source), text);
        assert!(contains_kind(&result.root, SyntaxKind::WhereClause));
        assert!(contains_kind(&result.root, SyntaxKind::TypeReference));
    }

    #[test]
    fn parses_explicit_generic_method_arguments() {
        let text = "fn main() { value.convert<String>() }\n";
        let (source, result) = parse_text(text);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert_eq!(result.root.lossless_text(&source), text);
        assert!(contains_kind(&result.root, SyntaxKind::MemberExpression));
        assert!(contains_kind(&result.root, SyntaxKind::GenericArgumentList));
    }

    #[test]
    fn parses_nested_generic_arguments_with_shift_right_token() {
        let text = "fn main() { let value = make<Box<List<Int>>>() }\n";
        let (source, result) = parse_text(text);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert_eq!(result.root.lossless_text(&source), text);
        assert!(contains_kind(
            &result.root,
            SyntaxKind::GenericArgumentList
        ));
    }

    #[test]
    fn parses_explicit_move_prefix_expression() {
        let text = "fn main() { let source = \"owned\"\n let target = move source\n }\n";
        let (source, result) = parse_text(text);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert_eq!(result.root.lossless_text(&source), text);
        assert!(contains_kind(&result.root, SyntaxKind::PrefixExpression));
    }

}
