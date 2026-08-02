//! Lossless concrete syntax tree (CST) and typed AST foundations for Nivra.
//!
//! The CST owns the complete lexer token stream, including whitespace and comments.
//! Typed AST wrappers are zero-copy views over selected CST node kinds.

use std::fmt::{self, Write as _};

use nivra_lexer::{Token, TokenKind};
use nivra_source::{SourceFile, Span};

/// Stable syntactic node categories emitted by the D4 parser.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SyntaxKind {
    SourceFile,
    Error,
    Attribute,
    ModuleDeclaration,
    UseDeclaration,
    ConstDeclaration,
    TypeAliasDeclaration,
    NewtypeDeclaration,
    RecordDeclaration,
    StructDeclaration,
    EnumDeclaration,
    EnumVariant,
    TraitDeclaration,
    ImplDeclaration,
    FunctionDeclaration,
    ExternBlock,
    ExternFunction,
    GenericParameterList,
    ParameterList,
    Parameter,
    FieldList,
    Field,
    WhereClause,
    TypeReference,
    Path,
    Block,
    LetStatement,
    VarStatement,
    ReturnStatement,
    BreakStatement,
    ContinueStatement,
    DeferStatement,
    EnsureStatement,
    WhileStatement,
    ForStatement,
    ExpressionStatement,
    IfExpression,
    MatchExpression,
    MatchArm,
    LoopExpression,
    UnsafeExpression,
    TaskGroupExpression,
    AsyncExpression,
    ClosureExpression,
    BinaryExpression,
    PrefixExpression,
    AssignmentExpression,
    CallExpression,
    MemberExpression,
    IndexExpression,
    TryExpression,
    AwaitExpression,
    SpawnExpression,
    RangeExpression,
    ParenthesizedExpression,
    TupleExpression,
    ArrayExpression,
    RecordExpression,
    RecordFieldInitializer,
    LiteralExpression,
    NameExpression,
    Pattern,
    ArgumentList,
}

impl SyntaxKind {
    /// Returns a stable snake-case name used by CLI tree and JSON output.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::SourceFile => "source_file",
            Self::Error => "error",
            Self::Attribute => "attribute",
            Self::ModuleDeclaration => "module_declaration",
            Self::UseDeclaration => "use_declaration",
            Self::ConstDeclaration => "const_declaration",
            Self::TypeAliasDeclaration => "type_alias_declaration",
            Self::NewtypeDeclaration => "newtype_declaration",
            Self::RecordDeclaration => "record_declaration",
            Self::StructDeclaration => "struct_declaration",
            Self::EnumDeclaration => "enum_declaration",
            Self::EnumVariant => "enum_variant",
            Self::TraitDeclaration => "trait_declaration",
            Self::ImplDeclaration => "impl_declaration",
            Self::FunctionDeclaration => "function_declaration",
            Self::ExternBlock => "extern_block",
            Self::ExternFunction => "extern_function",
            Self::GenericParameterList => "generic_parameter_list",
            Self::ParameterList => "parameter_list",
            Self::Parameter => "parameter",
            Self::FieldList => "field_list",
            Self::Field => "field",
            Self::WhereClause => "where_clause",
            Self::TypeReference => "type_reference",
            Self::Path => "path",
            Self::Block => "block",
            Self::LetStatement => "let_statement",
            Self::VarStatement => "var_statement",
            Self::ReturnStatement => "return_statement",
            Self::BreakStatement => "break_statement",
            Self::ContinueStatement => "continue_statement",
            Self::DeferStatement => "defer_statement",
            Self::EnsureStatement => "ensure_statement",
            Self::WhileStatement => "while_statement",
            Self::ForStatement => "for_statement",
            Self::ExpressionStatement => "expression_statement",
            Self::IfExpression => "if_expression",
            Self::MatchExpression => "match_expression",
            Self::MatchArm => "match_arm",
            Self::LoopExpression => "loop_expression",
            Self::UnsafeExpression => "unsafe_expression",
            Self::TaskGroupExpression => "task_group_expression",
            Self::AsyncExpression => "async_expression",
            Self::ClosureExpression => "closure_expression",
            Self::BinaryExpression => "binary_expression",
            Self::PrefixExpression => "prefix_expression",
            Self::AssignmentExpression => "assignment_expression",
            Self::CallExpression => "call_expression",
            Self::MemberExpression => "member_expression",
            Self::IndexExpression => "index_expression",
            Self::TryExpression => "try_expression",
            Self::AwaitExpression => "await_expression",
            Self::SpawnExpression => "spawn_expression",
            Self::RangeExpression => "range_expression",
            Self::ParenthesizedExpression => "parenthesized_expression",
            Self::TupleExpression => "tuple_expression",
            Self::ArrayExpression => "array_expression",
            Self::RecordExpression => "record_expression",
            Self::RecordFieldInitializer => "record_field_initializer",
            Self::LiteralExpression => "literal_expression",
            Self::NameExpression => "name_expression",
            Self::Pattern => "pattern",
            Self::ArgumentList => "argument_list",
        }
    }
}

/// A token retained in the lossless CST.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SyntaxToken {
    token: Token,
}

impl SyntaxToken {
    /// Wraps a lexer token.
    #[must_use]
    pub const fn new(token: Token) -> Self {
        Self { token }
    }

    /// Returns the lexer token category.
    #[must_use]
    pub const fn kind(self) -> TokenKind {
        self.token.kind
    }

    /// Returns the token source span.
    #[must_use]
    pub const fn span(self) -> Span {
        self.token.span
    }

    /// Returns the exact token text.
    #[must_use]
    pub fn text<'a>(self, source: &'a SourceFile) -> Option<&'a str> {
        self.token.text(source)
    }
}

/// One node or token in a CST node's ordered child list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyntaxElement {
    Node(SyntaxNode),
    Token(SyntaxToken),
}

impl SyntaxElement {
    /// Returns this element's source span.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Node(node) => node.span,
            Self::Token(token) => token.span(),
        }
    }
}

/// Immutable lossless CST node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxNode {
    kind: SyntaxKind,
    span: Span,
    children: Vec<SyntaxElement>,
}

impl SyntaxNode {
    /// Creates a node. `fallback` is used when the node has no children.
    #[must_use]
    pub fn new(kind: SyntaxKind, children: Vec<SyntaxElement>, fallback: Span) -> Self {
        let span = children
            .first()
            .zip(children.last())
            .and_then(|(first, last)| first.span().cover(last.span()))
            .unwrap_or(fallback);
        Self {
            kind,
            span,
            children,
        }
    }

    /// Returns the node kind.
    #[must_use]
    pub const fn kind(&self) -> SyntaxKind {
        self.kind
    }

    /// Returns the covered source span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    /// Returns all ordered lossless children.
    #[must_use]
    pub fn children_with_tokens(&self) -> &[SyntaxElement] {
        &self.children
    }

    /// Iterates direct child nodes.
    pub fn child_nodes(&self) -> impl Iterator<Item = &SyntaxNode> {
        self.children.iter().filter_map(|element| match element {
            SyntaxElement::Node(node) => Some(node),
            SyntaxElement::Token(_) => None,
        })
    }

    /// Iterates direct tokens.
    pub fn child_tokens(&self) -> impl Iterator<Item = SyntaxToken> + '_ {
        self.children.iter().filter_map(|element| match element {
            SyntaxElement::Node(_) => None,
            SyntaxElement::Token(token) => Some(*token),
        })
    }

    /// Reconstructs the exact source represented by this subtree.
    #[must_use]
    pub fn lossless_text(&self, source: &SourceFile) -> String {
        let mut output = String::new();
        self.write_lossless(source, &mut output);
        output
    }

    fn write_lossless(&self, source: &SourceFile, output: &mut String) {
        for child in &self.children {
            match child {
                SyntaxElement::Node(node) => node.write_lossless(source, output),
                SyntaxElement::Token(token) => {
                    if let Some(text) = token.text(source) {
                        output.push_str(text);
                    }
                }
            }
        }
    }

    /// Renders an indentation-based parser tree.
    #[must_use]
    pub fn debug_tree(&self, source: &SourceFile, include_trivia: bool) -> String {
        let mut output = String::new();
        self.write_debug_tree(source, include_trivia, 0, &mut output);
        output
    }

    fn write_debug_tree(
        &self,
        source: &SourceFile,
        include_trivia: bool,
        depth: usize,
        output: &mut String,
    ) {
        let indent = "  ".repeat(depth);
        let _ = writeln!(
            output,
            "{indent}{} {}..{}",
            self.kind.name(),
            self.span.start(),
            self.span.end()
        );
        for child in &self.children {
            match child {
                SyntaxElement::Node(node) => {
                    node.write_debug_tree(source, include_trivia, depth + 1, output);
                }
                SyntaxElement::Token(token) => {
                    if !include_trivia && token.kind().is_trivia() {
                        continue;
                    }
                    let text = token.text(source).unwrap_or("");
                    let _ = writeln!(
                        output,
                        "{}  token {} {}..{} {}",
                        indent,
                        token.kind().name(),
                        token.span().start(),
                        token.span().end(),
                        quoted(text)
                    );
                }
            }
        }
    }

    /// Returns the first direct child with the requested kind.
    #[must_use]
    pub fn child_by_kind(&self, kind: SyntaxKind) -> Option<&SyntaxNode> {
        self.child_nodes().find(|node| node.kind() == kind)
    }

    /// Iterates direct children with the requested kind.
    pub fn children_by_kind(
        &self,
        kind: SyntaxKind,
    ) -> impl Iterator<Item = &SyntaxNode> {
        self.child_nodes().filter(move |node| node.kind() == kind)
    }

    /// Returns the first direct non-trivia token.
    #[must_use]
    pub fn first_significant_token(&self) -> Option<SyntaxToken> {
        self.child_tokens()
            .find(|token| !token.kind().is_trivia() && token.kind() != TokenKind::Eof)
    }

    /// Collects all descendant tokens in source order.
    #[must_use]
    pub fn descendant_tokens(&self) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();
        self.collect_descendant_tokens(&mut tokens);
        tokens
    }

    fn collect_descendant_tokens(&self, output: &mut Vec<SyntaxToken>) {
        for child in &self.children {
            match child {
                SyntaxElement::Node(node) => node.collect_descendant_tokens(output),
                SyntaxElement::Token(token) => output.push(*token),
            }
        }
    }

    /// Counts nodes recursively, including this node.
    #[must_use]
    pub fn descendant_node_count(&self) -> usize {
        1 + self
            .child_nodes()
            .map(|node| node.descendant_node_count())
            .sum::<usize>()
    }

    /// Counts tokens recursively.
    #[must_use]
    pub fn descendant_token_count(&self) -> usize {
        self.children
            .iter()
            .map(|element| match element {
                SyntaxElement::Node(node) => node.descendant_token_count(),
                SyntaxElement::Token(_) => 1,
            })
            .sum()
    }
}

fn quoted(text: &str) -> String {
    let mut output = String::from("\"");
    for character in text.chars() {
        match character {
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            value if value.is_control() => {
                let _ = write!(output, "\\u{{{:x}}}", u32::from(value));
            }
            value => output.push(value),
        }
    }
    output.push('"');
    output
}

/// Common behavior for typed AST wrappers.
pub trait AstNode<'a>: Sized {
    /// Returns the syntax kind represented by the wrapper.
    const KIND: SyntaxKind;

    /// Casts a CST node when it has the expected kind.
    fn cast(node: &'a SyntaxNode) -> Option<Self>;

    /// Returns the wrapped CST node.
    fn syntax(&self) -> &'a SyntaxNode;
}

macro_rules! ast_wrapper {
    ($name:ident, $kind:ident) => {
        #[doc = concat!("Typed AST view for `", stringify!($kind), "` nodes.")]
        #[derive(Clone, Copy, Debug)]
        pub struct $name<'a> {
            syntax: &'a SyntaxNode,
        }

        impl<'a> AstNode<'a> for $name<'a> {
            const KIND: SyntaxKind = SyntaxKind::$kind;

            fn cast(node: &'a SyntaxNode) -> Option<Self> {
                (node.kind() == Self::KIND).then_some(Self { syntax: node })
            }

            fn syntax(&self) -> &'a SyntaxNode {
                self.syntax
            }
        }
    };
}

ast_wrapper!(SourceFileAst, SourceFile);
ast_wrapper!(ModuleAst, ModuleDeclaration);
ast_wrapper!(UseAst, UseDeclaration);
ast_wrapper!(ConstAst, ConstDeclaration);
ast_wrapper!(TypeAliasAst, TypeAliasDeclaration);
ast_wrapper!(NewtypeAst, NewtypeDeclaration);
ast_wrapper!(FunctionAst, FunctionDeclaration);
ast_wrapper!(ExternFunctionAst, ExternFunction);
ast_wrapper!(RecordAst, RecordDeclaration);
ast_wrapper!(StructAst, StructDeclaration);
ast_wrapper!(EnumAst, EnumDeclaration);
ast_wrapper!(EnumVariantAst, EnumVariant);
ast_wrapper!(TraitAst, TraitDeclaration);
ast_wrapper!(ImplAst, ImplDeclaration);
ast_wrapper!(ParameterAst, Parameter);
ast_wrapper!(FieldAst, Field);
ast_wrapper!(PatternAst, Pattern);
ast_wrapper!(NameExpressionAst, NameExpression);
ast_wrapper!(TypeReferenceAst, TypeReference);
ast_wrapper!(BlockAst, Block);
ast_wrapper!(LetStatementAst, LetStatement);
ast_wrapper!(VarStatementAst, VarStatement);
ast_wrapper!(ExpressionStatementAst, ExpressionStatement);

/// Shared accessor for AST nodes whose declaration starts with a direct identifier.
pub trait NamedAstNode<'a>: AstNode<'a> {
    /// Returns the declaration name token.
    #[must_use]
    fn name_token(&self) -> Option<SyntaxToken> {
        self.syntax()
            .child_tokens()
            .find(|token| token.kind() == TokenKind::Identifier)
    }
}

macro_rules! named_ast {
    ($($name:ident),+ $(,)?) => {
        $(impl<'a> NamedAstNode<'a> for $name<'a> {})+
    };
}

named_ast!(
    ModuleAst,
    UseAst,
    ConstAst,
    TypeAliasAst,
    NewtypeAst,
    FunctionAst,
    ExternFunctionAst,
    RecordAst,
    StructAst,
    EnumAst,
    EnumVariantAst,
    TraitAst,
    ParameterAst,
    FieldAst,
    PatternAst,
    NameExpressionAst,
);

impl<'a> SourceFileAst<'a> {
    /// Iterates top-level declaration nodes.
    pub fn declarations(self) -> impl Iterator<Item = &'a SyntaxNode> {
        self.syntax.child_nodes().filter(|node| {
            matches!(
                node.kind(),
                SyntaxKind::ModuleDeclaration
                    | SyntaxKind::UseDeclaration
                    | SyntaxKind::ConstDeclaration
                    | SyntaxKind::TypeAliasDeclaration
                    | SyntaxKind::NewtypeDeclaration
                    | SyntaxKind::RecordDeclaration
                    | SyntaxKind::StructDeclaration
                    | SyntaxKind::EnumDeclaration
                    | SyntaxKind::TraitDeclaration
                    | SyntaxKind::ImplDeclaration
                    | SyntaxKind::FunctionDeclaration
                    | SyntaxKind::ExternBlock
            )
        })
    }
}

impl fmt::Display for SyntaxKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use nivra_lexer::{Token, TokenKind};
    use nivra_source::{SourceId, SourceManager, Span};

    use super::{AstNode, SourceFileAst, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

    #[test]
    fn reconstructs_lossless_text() {
        let mut sources = SourceManager::new();
        let id = sources
            .add_virtual("test.nva", "let value = 1\n")
            .unwrap_or_else(|error| panic!("{error}"));
        let source = sources.get(id).unwrap_or_else(|| panic!("missing source"));
        let children = vec![SyntaxElement::Token(SyntaxToken::new(Token {
            kind: TokenKind::Identifier,
            span: source.full_span(),
        }))];
        let root = SyntaxNode::new(SyntaxKind::SourceFile, children, Span::empty(id, 0));
        assert_eq!(root.lossless_text(source), source.text());
    }

    #[test]
    fn typed_ast_cast_checks_kind() {
        let id = SourceId::from_raw(0);
        let root = SyntaxNode::new(
            SyntaxKind::SourceFile,
            Vec::new(),
            Span::empty(id, 0),
        );
        assert!(SourceFileAst::cast(&root).is_some());
    }
}
