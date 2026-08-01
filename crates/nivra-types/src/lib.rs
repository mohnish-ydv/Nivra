//! Type representation and first static type-checking pass for Nivra Edition 2026.
//!
//! D6 validates primitive and nominal types, function signatures, local inference,
//! operators, calls, conditions, arrays, assignments, and returns. Unknown types are
//! retained as recovery values so one unsupported member or generic operation does
//! not produce a wall of follow-on diagnostics.

use std::collections::{HashMap, HashSet};
use std::fmt::{self, Write as _};

use nivra_diagnostics::Diagnostic;
use nivra_lexer::{Keyword, TokenKind};
use nivra_sema::{Namespace, SemanticResult, SymbolKind};
use nivra_source::{SourceFile, Span};
use nivra_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

/// Static type used by the D6 checker.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    Unknown,
    Error,
    Unit,
    Never,
    Bool,
    Char,
    String,
    Int,
    Float,
    Named(String, Vec<Type>),
    Optional(Box<Type>),
    Reference { mutable: bool, inner: Box<Type> },
    Pointer { mutable: bool, inner: Box<Type> },
    Tuple(Vec<Type>),
    Function(Vec<Type>, Box<Type>),
}

impl Type {
    /// Returns whether this is an error-recovery type.
    #[must_use]
    pub const fn is_recovery(&self) -> bool {
        matches!(self, Self::Unknown | Self::Error)
    }

    /// Returns whether arithmetic operators can consume this type.
    #[must_use]
    pub const fn is_numeric(&self) -> bool {
        matches!(self, Self::Int | Self::Float)
    }

    /// Returns a stable source-like spelling.
    #[must_use]
    pub fn display_name(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for Type {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => formatter.write_str("Unknown"),
            Self::Error => formatter.write_str("<error>"),
            Self::Unit => formatter.write_str("Unit"),
            Self::Never => formatter.write_str("Never"),
            Self::Bool => formatter.write_str("Bool"),
            Self::Char => formatter.write_str("Char"),
            Self::String => formatter.write_str("String"),
            Self::Int => formatter.write_str("Int"),
            Self::Float => formatter.write_str("Float"),
            Self::Named(name, arguments) => {
                formatter.write_str(name)?;
                if !arguments.is_empty() {
                    formatter.write_str("<")?;
                    for (index, argument) in arguments.iter().enumerate() {
                        if index > 0 {
                            formatter.write_str(", ")?;
                        }
                        write!(formatter, "{argument}")?;
                    }
                    formatter.write_str(">")?;
                }
                Ok(())
            }
            Self::Optional(inner) => write!(formatter, "{inner}?"),
            Self::Reference { mutable, inner } => {
                if *mutable {
                    write!(formatter, "&mut {inner}")
                } else {
                    write!(formatter, "&{inner}")
                }
            }
            Self::Pointer { mutable, inner } => {
                if *mutable {
                    write!(formatter, "*mut {inner}")
                } else {
                    write!(formatter, "*const {inner}")
                }
            }
            Self::Tuple(items) => {
                formatter.write_str("(")?;
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(", ")?;
                    }
                    write!(formatter, "{item}")?;
                }
                formatter.write_str(")")
            }
            Self::Function(parameters, result) => {
                formatter.write_str("fn(")?;
                for (index, parameter) in parameters.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(", ")?;
                    }
                    write!(formatter, "{parameter}")?;
                }
                write!(formatter, ") -> {result}")
            }
        }
    }
}

/// One typed function parameter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParameterType {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

/// Indexed callable signature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionSignature {
    pub name: String,
    pub parameters: Vec<ParameterType>,
    pub return_type: Type,
    pub span: Span,
    pub is_async: bool,
    pub is_extern: bool,
}

/// Type inferred or declared for one binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindingType {
    pub name: String,
    pub ty: Type,
    pub span: Span,
    pub mutable: bool,
}

/// Type assigned to one expression node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedExpression {
    pub kind: SyntaxKind,
    pub ty: Type,
    pub span: Span,
}

/// Complete D6 type-check result.
#[derive(Clone, Debug)]
pub struct TypeCheckResult {
    pub functions: Vec<FunctionSignature>,
    pub bindings: Vec<BindingType>,
    pub expressions: Vec<TypedExpression>,
    pub diagnostics: Vec<Diagnostic>,
}

impl TypeCheckResult {
    /// Returns whether any type error was produced.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }

    /// Produces a deterministic function-signature report.
    #[must_use]
    pub fn function_report(&self) -> String {
        let mut output = String::new();
        for signature in &self.functions {
            let _ = write!(output, "fn {}(", signature.name);
            for (index, parameter) in signature.parameters.iter().enumerate() {
                if index > 0 {
                    output.push_str(", ");
                }
                let _ = write!(output, "{}: {}", parameter.name, parameter.ty);
            }
            let _ = writeln!(
                output,
                ") -> {}{}{}",
                signature.return_type,
                if signature.is_async { " async" } else { "" },
                if signature.is_extern { " extern" } else { "" }
            );
        }
        output
    }

    /// Produces a deterministic binding report.
    #[must_use]
    pub fn binding_report(&self) -> String {
        let mut output = String::new();
        for binding in &self.bindings {
            let _ = writeln!(
                output,
                "{} {}: {} @ {}..{}",
                if binding.mutable { "var" } else { "let" },
                binding.name,
                binding.ty,
                binding.span.start(),
                binding.span.end()
            );
        }
        output
    }
}

/// Runs D6 type checking after parsing and name resolution have succeeded.
#[must_use]
pub fn check(
    source: &SourceFile,
    root: &SyntaxNode,
    semantic: &SemanticResult,
) -> TypeCheckResult {
    Checker::new(source, semantic).run(root)
}

#[derive(Clone, Debug)]
struct LocalBinding {
    ty: Type,
    mutable: bool,
    span: Span,
}

struct Checker<'a> {
    source: &'a SourceFile,
    known_types: HashSet<String>,
    functions: Vec<FunctionSignature>,
    function_lookup: HashMap<String, usize>,
    constants: HashMap<String, Type>,
    scopes: Vec<HashMap<String, LocalBinding>>,
    bindings: Vec<BindingType>,
    expressions: Vec<TypedExpression>,
    diagnostics: Vec<Diagnostic>,
    expected_return: Type,
    saw_explicit_return: bool,
}

impl<'a> Checker<'a> {
    fn new(source: &'a SourceFile, semantic: &SemanticResult) -> Self {
        let mut known_types = builtin_type_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect::<HashSet<_>>();
        for symbol in &semantic.symbols {
            if symbol.namespace == Namespace::Type
                && !matches!(symbol.kind, SymbolKind::Import)
            {
                let _ = known_types.insert(symbol.name.clone());
            }
            if symbol.namespace == Namespace::Type && symbol.kind == SymbolKind::Import {
                let _ = known_types.insert(symbol.name.clone());
            }
        }
        Self {
            source,
            known_types,
            functions: Vec::new(),
            function_lookup: HashMap::new(),
            constants: HashMap::new(),
            scopes: Vec::new(),
            bindings: Vec::new(),
            expressions: Vec::new(),
            diagnostics: Vec::new(),
            expected_return: Type::Unit,
            saw_explicit_return: false,
        }
    }

    fn run(mut self, root: &SyntaxNode) -> TypeCheckResult {
        self.collect_signatures(root);
        self.collect_constants(root);
        for node in root.child_nodes() {
            match node.kind() {
                SyntaxKind::FunctionDeclaration => self.check_function(node),
                SyntaxKind::ExternBlock => {
                    for function in node.child_nodes() {
                        if function.kind() == SyntaxKind::ExternFunction {
                            self.check_function(function);
                        }
                    }
                }
                SyntaxKind::ConstDeclaration => self.check_constant(node),
                SyntaxKind::TraitDeclaration | SyntaxKind::ImplDeclaration => {
                    for function in node.child_nodes() {
                        if function.kind() == SyntaxKind::FunctionDeclaration {
                            self.check_function(function);
                        }
                    }
                }
                _ => {}
            }
        }
        TypeCheckResult {
            functions: self.functions,
            bindings: self.bindings,
            expressions: self.expressions,
            diagnostics: self.diagnostics,
        }
    }

    fn collect_signatures(&mut self, root: &SyntaxNode) {
        self.collect_signatures_in(root, false);
    }

    fn collect_signatures_in(&mut self, parent: &SyntaxNode, extern_context: bool) {
        for node in parent.child_nodes() {
            match node.kind() {
                SyntaxKind::FunctionDeclaration | SyntaxKind::ExternFunction => {
                    if let Some(signature) = self.signature_from_node(
                        node,
                        extern_context || node.kind() == SyntaxKind::ExternFunction,
                    ) {
                        let index = self.functions.len();
                        let _ = self.function_lookup.insert(signature.name.clone(), index);
                        self.functions.push(signature);
                    }
                }
                SyntaxKind::ExternBlock => self.collect_signatures_in(node, true),
                SyntaxKind::TraitDeclaration | SyntaxKind::ImplDeclaration => {
                    self.collect_signatures_in(node, false);
                }
                _ => {}
            }
        }
    }

    fn signature_from_node(
        &mut self,
        node: &SyntaxNode,
        is_extern: bool,
    ) -> Option<FunctionSignature> {
        let (name, span) = function_name(node, self.source)?;
        let generic_names = generic_parameter_names(node, self.source);
        let mut parameters = Vec::new();
        if let Some(list) = node
            .child_nodes()
            .find(|child| child.kind() == SyntaxKind::ParameterList)
        {
            for parameter in list
                .child_nodes()
                .filter(|child| child.kind() == SyntaxKind::Parameter)
            {
                let Some((parameter_name, parameter_span, type_text)) =
                    parameter_parts(parameter, self.source)
                else {
                    continue;
                };
                let ty = if type_text.trim().is_empty() {
                    Type::Unknown
                } else {
                    self.parse_declared_type(&type_text, parameter.span(), &generic_names)
                };
                parameters.push(ParameterType {
                    name: parameter_name,
                    ty,
                    span: parameter_span,
                });
            }
        }
        let return_type = node
            .child_nodes()
            .find(|child| child.kind() == SyntaxKind::TypeReference)
            .map_or(Type::Unit, |type_node| {
                self.parse_type_node(type_node, &generic_names)
            });
        let is_async = significant_direct_tokens(node).iter().any(|token| {
            token.kind() == TokenKind::Keyword(Keyword::Async)
        });
        Some(FunctionSignature {
            name,
            parameters,
            return_type,
            span,
            is_async,
            is_extern,
        })
    }

    fn collect_constants(&mut self, root: &SyntaxNode) {
        for node in root
            .child_nodes()
            .filter(|child| child.kind() == SyntaxKind::ConstDeclaration)
        {
            let Some((name, _)) = first_direct_identifier(node, self.source) else {
                continue;
            };
            let ty = node
                .child_nodes()
                .find(|child| child.kind() == SyntaxKind::TypeReference)
                .map_or(Type::Unknown, |child| self.parse_type_node(child, &HashSet::new()));
            let _ = self.constants.insert(name, ty);
        }
    }

    fn check_constant(&mut self, node: &SyntaxNode) {
        let annotation = node
            .child_nodes()
            .find(|child| child.kind() == SyntaxKind::TypeReference)
            .map(|child| self.parse_type_node(child, &HashSet::new()));
        let initializer = node
            .child_nodes()
            .find(|child| is_expression_kind(child.kind()));
        if let Some(initializer) = initializer {
            self.push_scope();
            let actual = self.infer_expression(initializer);
            self.pop_scope();
            if let Some(expected) = annotation {
                self.require_assignable(
                    &expected,
                    &actual,
                    initializer.span(),
                    "constant initializer",
                    "TYP001",
                );
            }
        }
    }

    fn check_function(&mut self, node: &SyntaxNode) {
        let Some((name, _)) = function_name(node, self.source) else {
            return;
        };
        let signature = self
            .function_lookup
            .get(&name)
            .and_then(|index| self.functions.get(*index))
            .cloned();
        let Some(signature) = signature else {
            return;
        };
        let Some(block) = node
            .child_nodes()
            .find(|child| child.kind() == SyntaxKind::Block)
        else {
            return;
        };

        let previous_return = self.expected_return.clone();
        let previous_saw_return = self.saw_explicit_return;
        self.expected_return = signature.return_type.clone();
        self.saw_explicit_return = false;
        self.push_scope();
        for parameter in &signature.parameters {
            self.define_local(&parameter.name, parameter.ty.clone(), false, parameter.span);
        }
        let tail = self.infer_block(block, false);
        let saw_explicit = self.saw_explicit_return;
        self.pop_scope();

        if signature.return_type != Type::Unit {
            if tail != Type::Unit || !saw_explicit {
                self.require_assignable(
                    &signature.return_type,
                    &tail,
                    block.span(),
                    "function body result",
                    "TYP005",
                );
            }
        } else if tail != Type::Unit && !tail.is_recovery() {
            self.require_assignable(
                &Type::Unit,
                &tail,
                block.span(),
                "function declared to return Unit",
                "TYP005",
            );
        }

        self.expected_return = previous_return;
        self.saw_explicit_return = previous_saw_return;
    }

    fn infer_block(&mut self, block: &SyntaxNode, nested: bool) -> Type {
        if nested {
            self.push_scope();
        }
        let statements = block.child_nodes().collect::<Vec<_>>();
        let mut tail = Type::Unit;
        for (index, statement) in statements.iter().enumerate() {
            let is_last = index + 1 == statements.len();
            let statement_type = self.check_statement(statement);
            if is_last && statement.kind() == SyntaxKind::ExpressionStatement {
                tail = statement_type;
            }
        }
        if nested {
            self.pop_scope();
        }
        tail
    }

    fn check_statement(&mut self, node: &SyntaxNode) -> Type {
        match node.kind() {
            SyntaxKind::LetStatement | SyntaxKind::VarStatement => {
                self.check_binding(node);
                Type::Unit
            }
            SyntaxKind::ReturnStatement => {
                self.saw_explicit_return = true;
                let actual = node
                    .child_nodes()
                    .find(|child| is_expression_kind(child.kind()))
                    .map_or(Type::Unit, |child| self.infer_expression(child));
                let expected = self.expected_return.clone();
                self.require_assignable(
                    &expected,
                    &actual,
                    node.span(),
                    "return expression",
                    "TYP005",
                );
                Type::Never
            }
            SyntaxKind::WhileStatement => {
                let mut children = node.child_nodes();
                if let Some(condition) = children.next() {
                    let actual = self.infer_expression(condition);
                    self.require_bool(&actual, condition.span(), "while condition");
                }
                if let Some(block) = children.find(|child| child.kind() == SyntaxKind::Block) {
                    self.infer_block(block, true);
                }
                Type::Unit
            }
            SyntaxKind::ForStatement => self.check_for(node),
            SyntaxKind::EnsureStatement => {
                let expressions = node.child_nodes().collect::<Vec<_>>();
                if let Some(condition) = expressions.first() {
                    let actual = self.infer_expression(condition);
                    self.require_bool(&actual, condition.span(), "ensure condition");
                }
                if let Some(error) = expressions.get(1) {
                    self.infer_expression(error);
                }
                Type::Unit
            }
            SyntaxKind::DeferStatement => {
                for child in node.child_nodes() {
                    self.infer_expression(child);
                }
                Type::Unit
            }
            SyntaxKind::ExpressionStatement => node
                .child_nodes()
                .next()
                .map_or(Type::Unit, |child| self.infer_expression(child)),
            _ if is_expression_kind(node.kind()) => self.infer_expression(node),
            _ => Type::Unit,
        }
    }

    fn check_for(&mut self, node: &SyntaxNode) -> Type {
        let pattern = node
            .child_nodes()
            .find(|child| child.kind() == SyntaxKind::Pattern);
        let iterable = node
            .child_nodes()
            .find(|child| is_expression_kind(child.kind()));
        let block = node
            .child_nodes()
            .find(|child| child.kind() == SyntaxKind::Block);
        let item_type = iterable.map_or(Type::Unknown, |expression| {
            match self.infer_expression(expression) {
                Type::Named(name, arguments) if name == "List" && arguments.len() == 1 => {
                    arguments[0].clone()
                }
                Type::Unknown | Type::Error => Type::Unknown,
                other => {
                    self.diagnostics.push(
                        Diagnostic::error("TYP002", format!("type `{other}` is not iterable"))
                            .with_primary(expression.span(), "`for` requires an iterable value")
                            .with_help("use a List<T> value or an iterator-producing expression"),
                    );
                    Type::Error
                }
            }
        });
        self.push_scope();
        if let Some(pattern) = pattern {
            if let Some((name, span)) = pattern_name(pattern, self.source) {
                self.define_local(&name, item_type, false, span);
            }
        }
        if let Some(block) = block {
            self.infer_block(block, false);
        }
        self.pop_scope();
        Type::Unit
    }

    fn check_binding(&mut self, node: &SyntaxNode) {
        let Some(pattern) = node
            .child_nodes()
            .find(|child| child.kind() == SyntaxKind::Pattern)
        else {
            return;
        };
        let Some((name, span)) = pattern_name(pattern, self.source) else {
            return;
        };
        let annotation = node
            .child_nodes()
            .find(|child| child.kind() == SyntaxKind::TypeReference)
            .map(|child| self.parse_type_node(child, &HashSet::new()));
        let initializer = node
            .child_nodes()
            .find(|child| is_expression_kind(child.kind()));
        let actual = initializer.map_or(Type::Unknown, |child| self.infer_expression(child));
        let final_type = match annotation {
            Some(expected) => {
                if let Some(initializer) = initializer {
                    self.require_assignable(
                        &expected,
                        &actual,
                        initializer.span(),
                        "binding initializer",
                        "TYP001",
                    );
                }
                expected
            }
            None => {
                if cannot_infer_without_context(&actual) {
                    self.diagnostics.push(
                        Diagnostic::error("TYP006", format!("cannot infer the type of `{name}`"))
                            .with_primary(span, "this binding needs an explicit type")
                            .with_help(format!("write `{}: Type = ...`", name)),
                    );
                    Type::Error
                } else {
                    actual
                }
            }
        };
        self.define_local(
            &name,
            final_type,
            node.kind() == SyntaxKind::VarStatement,
            span,
        );
    }

    fn infer_expression(&mut self, node: &SyntaxNode) -> Type {
        let ty = match node.kind() {
            SyntaxKind::LiteralExpression => self.infer_literal(node),
            SyntaxKind::NameExpression => self.infer_name(node),
            SyntaxKind::BinaryExpression => self.infer_binary(node),
            SyntaxKind::AssignmentExpression => self.infer_assignment(node),
            SyntaxKind::PrefixExpression => self.infer_prefix(node),
            SyntaxKind::CallExpression => self.infer_call(node),
            SyntaxKind::ParenthesizedExpression => node
                .child_nodes()
                .next()
                .map_or(Type::Unit, |child| self.infer_expression(child)),
            SyntaxKind::TupleExpression => Type::Tuple(
                node.child_nodes()
                    .map(|child| self.infer_expression(child))
                    .collect(),
            ),
            SyntaxKind::ArrayExpression => self.infer_array(node),
            SyntaxKind::Block => self.infer_block(node, true),
            SyntaxKind::IfExpression => self.infer_if(node),
            SyntaxKind::MatchExpression => self.infer_match(node),
            SyntaxKind::IndexExpression => self.infer_index(node),
            SyntaxKind::TryExpression => self.infer_try(node),
            SyntaxKind::AwaitExpression | SyntaxKind::SpawnExpression => node
                .child_nodes()
                .next()
                .map_or(Type::Unknown, |child| self.infer_expression(child)),
            SyntaxKind::AsyncExpression => {
                let inner = node
                    .child_nodes()
                    .next()
                    .map_or(Type::Unit, |child| self.infer_expression(child));
                Type::Named("Task".to_owned(), vec![inner])
            }
            SyntaxKind::TaskGroupExpression | SyntaxKind::UnsafeExpression => node
                .child_nodes()
                .find(|child| child.kind() == SyntaxKind::Block)
                .map_or(Type::Unit, |child| self.infer_block(child, true)),
            SyntaxKind::MemberExpression
            | SyntaxKind::ClosureExpression
            | SyntaxKind::RecordExpression
            | SyntaxKind::RangeExpression => {
                for child in node.child_nodes() {
                    self.infer_expression(child);
                }
                Type::Unknown
            }
            SyntaxKind::ExpressionStatement => node
                .child_nodes()
                .next()
                .map_or(Type::Unit, |child| self.infer_expression(child)),
            SyntaxKind::Error => Type::Error,
            _ => Type::Unknown,
        };
        self.expressions.push(TypedExpression {
            kind: node.kind(),
            ty: ty.clone(),
            span: node.span(),
        });
        ty
    }

    fn infer_literal(&self, node: &SyntaxNode) -> Type {
        let Some(token) = significant_direct_tokens(node).first().copied() else {
            return Type::Unknown;
        };
        match token.kind() {
            TokenKind::IntegerLiteral => Type::Int,
            TokenKind::FloatLiteral => Type::Float,
            TokenKind::StringLiteral => Type::String,
            TokenKind::CharLiteral => Type::Char,
            TokenKind::Keyword(Keyword::True | Keyword::False) => Type::Bool,
            TokenKind::Keyword(Keyword::None) => Type::Optional(Box::new(Type::Unknown)),
            _ => Type::Unknown,
        }
    }

    fn infer_name(&self, node: &SyntaxNode) -> Type {
        let text = significant_text(node, self.source);
        if text.starts_with('.') || text.contains("::") {
            return Type::Unknown;
        }
        if let Some(binding) = self.lookup_local(&text) {
            return binding.ty.clone();
        }
        if let Some(constant) = self.constants.get(&text) {
            return constant.clone();
        }
        if let Some(index) = self.function_lookup.get(&text) {
            if let Some(signature) = self.functions.get(*index) {
                return Type::Function(
                    signature
                        .parameters
                        .iter()
                        .map(|parameter| parameter.ty.clone())
                        .collect(),
                    Box::new(signature.return_type.clone()),
                );
            }
        }
        match text.as_str() {
            "print" | "println" | "assert" | "dbg" | "panic" | "todo" => {
                Type::Function(vec![Type::Unknown], Box::new(Type::Unit))
            }
            "ok" => Type::Function(
                vec![Type::Unknown],
                Box::new(Type::Named(
                    "Result".to_owned(),
                    vec![Type::Unknown, Type::Unknown],
                )),
            ),
            "err" => Type::Function(
                vec![Type::Unknown],
                Box::new(Type::Named(
                    "Result".to_owned(),
                    vec![Type::Unknown, Type::Unknown],
                )),
            ),
            _ => Type::Unknown,
        }
    }

    fn infer_binary(&mut self, node: &SyntaxNode) -> Type {
        let children = node.child_nodes().collect::<Vec<_>>();
        if children.len() < 2 {
            return Type::Error;
        }
        let left = self.infer_expression(children[0]);
        let right = self.infer_expression(children[1]);
        let operator = direct_operator(node).unwrap_or(TokenKind::Unknown);
        if left.is_recovery() || right.is_recovery() {
            return Type::Unknown;
        }
        match operator {
            TokenKind::Plus | TokenKind::Minus | TokenKind::Star | TokenKind::Slash | TokenKind::Percent => {
                if left == right && left.is_numeric() {
                    left
                } else if operator == TokenKind::Plus && left == Type::String && right == Type::String {
                    Type::String
                } else {
                    self.unsupported_operator(node.span(), operator, &left, &right);
                    Type::Error
                }
            }
            TokenKind::ShiftLeft
            | TokenKind::ShiftRight
            | TokenKind::Ampersand
            | TokenKind::Pipe
            | TokenKind::Caret => {
                if left == Type::Int && right == Type::Int {
                    Type::Int
                } else {
                    self.unsupported_operator(node.span(), operator, &left, &right);
                    Type::Error
                }
            }
            TokenKind::AmpersandAmpersand | TokenKind::PipePipe => {
                if left == Type::Bool && right == Type::Bool {
                    Type::Bool
                } else {
                    self.unsupported_operator(node.span(), operator, &left, &right);
                    Type::Error
                }
            }
            TokenKind::EqualEqual | TokenKind::BangEqual => {
                if types_compatible(&left, &right) || types_compatible(&right, &left) {
                    Type::Bool
                } else {
                    self.unsupported_operator(node.span(), operator, &left, &right);
                    Type::Error
                }
            }
            TokenKind::Less | TokenKind::LessEqual | TokenKind::Greater | TokenKind::GreaterEqual => {
                if left == right && (left.is_numeric() || matches!(left, Type::Char | Type::String)) {
                    Type::Bool
                } else {
                    self.unsupported_operator(node.span(), operator, &left, &right);
                    Type::Error
                }
            }
            _ => Type::Unknown,
        }
    }

    fn infer_assignment(&mut self, node: &SyntaxNode) -> Type {
        let children = node.child_nodes().collect::<Vec<_>>();
        if children.len() < 2 {
            return Type::Error;
        }
        let left_type = self.infer_expression(children[0]);
        let right_type = self.infer_expression(children[1]);
        if children[0].kind() == SyntaxKind::NameExpression {
            let name = significant_text(children[0], self.source);
            let immutable_span = self
                .lookup_local(&name)
                .and_then(|binding| (!binding.mutable).then_some(binding.span));
            if let Some(binding_span) = immutable_span {
                self.diagnostics.push(
                    Diagnostic::error("TYP010", format!("cannot assign to immutable binding `{name}`"))
                        .with_primary(children[0].span(), "this binding was declared with `let`")
                        .with_secondary(binding_span, "binding declared here")
                        .with_help("declare it with `var` when mutation is required"),
                );
            }
        }
        self.require_assignable(
            &left_type,
            &right_type,
            children[1].span(),
            "assignment value",
            "TYP001",
        );
        Type::Unit
    }

    fn infer_prefix(&mut self, node: &SyntaxNode) -> Type {
        let operand = node
            .child_nodes()
            .next()
            .map_or(Type::Unknown, |child| self.infer_expression(child));
        let operator = significant_direct_tokens(node)
            .into_iter()
            .find(|token| !token.kind().is_trivia())
            .map_or(TokenKind::Unknown, SyntaxToken::kind);
        if operand.is_recovery() {
            return operand;
        }
        match operator {
            TokenKind::Bang if operand == Type::Bool => Type::Bool,
            TokenKind::Plus | TokenKind::Minus if operand.is_numeric() => operand,
            TokenKind::Tilde if operand == Type::Int => Type::Int,
            TokenKind::Ampersand => Type::Reference {
                mutable: significant_text(node, self.source).starts_with("&mut"),
                inner: Box::new(operand),
            },
            _ => {
                self.diagnostics.push(
                    Diagnostic::error(
                        "TYP002",
                        format!("prefix operator `{}` is not defined for `{operand}`", operator.name()),
                    )
                    .with_primary(node.span(), "unsupported prefix operation")
                    .with_help("change the operand type or use an operator supported by that type"),
                );
                Type::Error
            }
        }
    }

    fn infer_call(&mut self, node: &SyntaxNode) -> Type {
        let children = node.child_nodes().collect::<Vec<_>>();
        if children.len() < 2 {
            return Type::Error;
        }
        let callee = children[0];
        let arguments = children[1]
            .child_nodes()
            .map(|argument| (argument, self.infer_expression(argument)))
            .collect::<Vec<_>>();

        if callee.kind() == SyntaxKind::NameExpression {
            let name = significant_text(callee, self.source);
            if matches!(name.as_str(), "print" | "println" | "dbg" | "panic" | "todo") {
                return Type::Unit;
            }
            if name == "assert" {
                if let Some((argument, ty)) = arguments.first() {
                    self.require_bool(ty, argument.span(), "assert argument");
                }
                return Type::Unit;
            }
            if name == "ok" {
                let value = arguments.first().map_or(Type::Unknown, |(_, ty)| ty.clone());
                return Type::Named("Result".to_owned(), vec![value, Type::Unknown]);
            }
            if name == "err" {
                let error = arguments.first().map_or(Type::Unknown, |(_, ty)| ty.clone());
                return Type::Named("Result".to_owned(), vec![Type::Unknown, error]);
            }
            if let Some(index) = self.function_lookup.get(&name).copied() {
                if let Some(signature) = self.functions.get(index).cloned() {
                    if arguments.len() != signature.parameters.len() {
                        self.diagnostics.push(
                            Diagnostic::error(
                                "TYP003",
                                format!(
                                    "function `{name}` expects {} argument(s), found {}",
                                    signature.parameters.len(),
                                    arguments.len()
                                ),
                            )
                            .with_primary(node.span(), "wrong number of arguments")
                            .with_secondary(signature.span, "function declared here")
                            .with_help("add or remove arguments to match the function signature"),
                        );
                    }
                    for ((argument_node, actual), expected) in
                        arguments.iter().zip(signature.parameters.iter())
                    {
                        self.require_assignable(
                            &expected.ty,
                            actual,
                            argument_node.span(),
                            &format!("argument `{}`", expected.name),
                            "TYP004",
                        );
                    }
                    return signature.return_type;
                }
            }
        }

        let callee_type = self.infer_expression(callee);
        if let Type::Function(parameters, result) = callee_type {
            if parameters.len() == arguments.len() {
                for ((argument, actual), expected) in arguments.iter().zip(parameters.iter()) {
                    self.require_assignable(expected, actual, argument.span(), "call argument", "TYP004");
                }
            }
            *result
        } else {
            Type::Unknown
        }
    }

    fn infer_array(&mut self, node: &SyntaxNode) -> Type {
        let items = node
            .child_nodes()
            .map(|child| (child, self.infer_expression(child)))
            .collect::<Vec<_>>();
        let Some((_, first)) = items.first() else {
            return Type::Named("List".to_owned(), vec![Type::Unknown]);
        };
        let element = first.clone();
        for (child, item) in items.iter().skip(1) {
            if !types_compatible(&element, item) || !types_compatible(item, &element) {
                self.diagnostics.push(
                    Diagnostic::error("TYP009", "array elements have incompatible types")
                        .with_primary(child.span(), format!("found `{item}` here"))
                        .with_note(format!("the first element has type `{element}`"))
                        .with_help("use one element type or add explicit conversions"),
                );
                return Type::Named("List".to_owned(), vec![Type::Error]);
            }
        }
        Type::Named("List".to_owned(), vec![element])
    }

    fn infer_if(&mut self, node: &SyntaxNode) -> Type {
        let has_pattern = node
            .child_nodes()
            .any(|child| child.kind() == SyntaxKind::Pattern);
        let children = node.child_nodes().collect::<Vec<_>>();
        if !has_pattern {
            if let Some(condition) = children
                .iter()
                .copied()
                .find(|child| child.kind() != SyntaxKind::Block && child.kind() != SyntaxKind::IfExpression)
            {
                let actual = self.infer_expression(condition);
                self.require_bool(&actual, condition.span(), "if condition");
            }
        } else {
            for child in &children {
                if child.kind() != SyntaxKind::Pattern && child.kind() != SyntaxKind::Block {
                    self.infer_expression(child);
                    break;
                }
            }
        }
        let branches = children
            .iter()
            .copied()
            .filter(|child| matches!(child.kind(), SyntaxKind::Block | SyntaxKind::IfExpression))
            .map(|child| {
                if child.kind() == SyntaxKind::Block {
                    self.infer_block(child, true)
                } else {
                    self.infer_expression(child)
                }
            })
            .collect::<Vec<_>>();
        unify_branch_types(&branches)
    }

    fn infer_match(&mut self, node: &SyntaxNode) -> Type {
        let mut arm_types = Vec::new();
        for child in node.child_nodes() {
            if child.kind() == SyntaxKind::MatchArm {
                if let Some(expression) = child
                    .child_nodes()
                    .find(|arm_child| arm_child.kind() != SyntaxKind::Pattern)
                {
                    arm_types.push(self.infer_expression(expression));
                }
            } else {
                self.infer_expression(child);
            }
        }
        unify_branch_types(&arm_types)
    }

    fn infer_index(&mut self, node: &SyntaxNode) -> Type {
        let children = node.child_nodes().collect::<Vec<_>>();
        let base = children
            .first()
            .map_or(Type::Unknown, |child| self.infer_expression(child));
        if let Some(index) = children.get(1) {
            let index_type = self.infer_expression(index);
            self.require_assignable(&Type::Int, &index_type, index.span(), "index expression", "TYP001");
        }
        match base {
            Type::Named(name, arguments) if name == "List" && arguments.len() == 1 => arguments[0].clone(),
            Type::String => Type::Char,
            Type::Unknown | Type::Error => Type::Unknown,
            other => {
                self.diagnostics.push(
                    Diagnostic::error("TYP002", format!("type `{other}` cannot be indexed"))
                        .with_primary(node.span(), "indexing is not supported for this value")
                        .with_help("index a List<T>, String, or another indexable type"),
                );
                Type::Error
            }
        }
    }

    fn infer_try(&mut self, node: &SyntaxNode) -> Type {
        let inner = node
            .child_nodes()
            .next()
            .map_or(Type::Unknown, |child| self.infer_expression(child));
        match inner {
            Type::Named(name, mut arguments) if name == "Result" && arguments.len() == 2 => {
                arguments.remove(0)
            }
            Type::Unknown | Type::Error => Type::Unknown,
            other => {
                self.diagnostics.push(
                    Diagnostic::error("TYP002", format!("`try` requires Result<T, E>, found `{other}`"))
                        .with_primary(node.span(), "this expression cannot be propagated")
                        .with_help("return a Result from the called operation or handle the value directly"),
                );
                Type::Error
            }
        }
    }

    fn parse_type_node(&mut self, node: &SyntaxNode, generics: &HashSet<String>) -> Type {
        self.parse_declared_type(&node.lossless_text(self.source), node.span(), generics)
    }

    fn parse_declared_type(
        &mut self,
        text: &str,
        span: Span,
        generics: &HashSet<String>,
    ) -> Type {
        let mut parser = TypeTextParser::new(text);
        let ty = parser.parse().unwrap_or(Type::Error);
        if ty == Type::Error {
            self.diagnostics.push(
                Diagnostic::error("TYP008", "invalid type syntax")
                    .with_primary(span, "the D6 type parser could not understand this type")
                    .with_help("use a named type, T?, &T, &mut T, tuple, or generic type"),
            );
            return ty;
        }
        let mut unknown = Vec::new();
        collect_unknown_type_names(&ty, &self.known_types, generics, &mut unknown);
        if let Some(name) = unknown.first() {
            self.diagnostics.push(
                Diagnostic::error("TYP008", format!("unknown type `{name}`"))
                    .with_primary(span, "this type is not declared or imported")
                    .with_help(format!("declare `{name}`, import it, or correct its spelling")),
            );
        }
        ty
    }

    fn require_bool(&mut self, actual: &Type, span: Span, context: &str) {
        if actual.is_recovery() || actual == &Type::Bool {
            return;
        }
        self.diagnostics.push(
            Diagnostic::error("TYP007", format!("{context} must have type Bool"))
                .with_primary(span, format!("found `{actual}`"))
                .with_note("Nivra has no implicit truthiness")
                .with_help("produce a Bool expression such as a comparison"),
        );
    }

    fn require_assignable(
        &mut self,
        expected: &Type,
        actual: &Type,
        span: Span,
        context: &str,
        code: &'static str,
    ) {
        if types_compatible(expected, actual) {
            return;
        }
        self.diagnostics.push(
            Diagnostic::error(
                code,
                format!("type mismatch in {context}: expected `{expected}`, found `{actual}`"),
            )
            .with_primary(span, format!("this value has type `{actual}`"))
            .with_note("Nivra does not insert lossy numeric conversions")
            .with_help(format!("provide a `{expected}` value or convert explicitly")),
        );
    }

    fn unsupported_operator(
        &mut self,
        span: Span,
        operator: TokenKind,
        left: &Type,
        right: &Type,
    ) {
        self.diagnostics.push(
            Diagnostic::error(
                "TYP002",
                format!(
                    "operator `{}` is not defined for `{left}` and `{right}`",
                    operator.name()
                ),
            )
            .with_primary(span, "unsupported operand types")
            .with_help("use compatible operands or add an explicit conversion"),
        );
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        let _ = self.scopes.pop();
    }

    fn define_local(&mut self, name: &str, ty: Type, mutable: bool, span: Span) {
        if let Some(scope) = self.scopes.last_mut() {
            let _ = scope.insert(
                name.to_owned(),
                LocalBinding {
                    ty: ty.clone(),
                    mutable,
                    span,
                },
            );
        }
        self.bindings.push(BindingType {
            name: name.to_owned(),
            ty,
            span,
            mutable,
        });
    }

    fn lookup_local(&self, name: &str) -> Option<&LocalBinding> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }
}

fn builtin_type_names() -> Vec<&'static str> {
    vec![
        "Bool", "Char", "Float", "F32", "F64", "I8", "I16", "I32", "I64",
        "Int", "List", "Map", "Never", "Option", "Path", "Result", "Set",
        "Shared", "String", "Task", "U8", "U16", "U32", "U64", "Unit", "Usize",
        "Weak", "Self",
    ]
}

fn types_compatible(expected: &Type, actual: &Type) -> bool {
    if expected.is_recovery() || actual.is_recovery() || expected == actual {
        return true;
    }
    match (expected, actual) {
        (Type::Optional(expected_inner), Type::Optional(actual_inner)) => {
            types_compatible(expected_inner, actual_inner)
        }
        (Type::Named(expected_name, expected_args), Type::Named(actual_name, actual_args)) => {
            expected_name == actual_name
                && expected_args.len() == actual_args.len()
                && expected_args
                    .iter()
                    .zip(actual_args.iter())
                    .all(|(left, right)| types_compatible(left, right))
        }
        (
            Type::Reference {
                mutable: expected_mutable,
                inner: expected_inner,
            },
            Type::Reference {
                mutable: actual_mutable,
                inner: actual_inner,
            },
        )
        | (
            Type::Pointer {
                mutable: expected_mutable,
                inner: expected_inner,
            },
            Type::Pointer {
                mutable: actual_mutable,
                inner: actual_inner,
            },
        ) => {
            expected_mutable == actual_mutable && types_compatible(expected_inner, actual_inner)
        }
        (Type::Tuple(expected_items), Type::Tuple(actual_items)) => {
            expected_items.len() == actual_items.len()
                && expected_items
                    .iter()
                    .zip(actual_items.iter())
                    .all(|(left, right)| types_compatible(left, right))
        }
        _ => false,
    }
}

fn cannot_infer_without_context(ty: &Type) -> bool {
    match ty {
        Type::Unknown | Type::Error => true,
        Type::Optional(inner) => inner.is_recovery(),
        Type::Named(_, arguments) => arguments.iter().any(cannot_infer_without_context),
        _ => false,
    }
}

fn unify_branch_types(types: &[Type]) -> Type {
    let mut concrete = types.iter().filter(|ty| !ty.is_recovery());
    let Some(first) = concrete.next() else {
        return Type::Unit;
    };
    if concrete.all(|candidate| types_compatible(first, candidate) && types_compatible(candidate, first)) {
        first.clone()
    } else {
        Type::Unknown
    }
}

fn is_expression_kind(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::IfExpression
            | SyntaxKind::MatchExpression
            | SyntaxKind::LoopExpression
            | SyntaxKind::UnsafeExpression
            | SyntaxKind::TaskGroupExpression
            | SyntaxKind::AsyncExpression
            | SyntaxKind::ClosureExpression
            | SyntaxKind::BinaryExpression
            | SyntaxKind::PrefixExpression
            | SyntaxKind::AssignmentExpression
            | SyntaxKind::CallExpression
            | SyntaxKind::MemberExpression
            | SyntaxKind::IndexExpression
            | SyntaxKind::TryExpression
            | SyntaxKind::AwaitExpression
            | SyntaxKind::SpawnExpression
            | SyntaxKind::RangeExpression
            | SyntaxKind::ParenthesizedExpression
            | SyntaxKind::TupleExpression
            | SyntaxKind::ArrayExpression
            | SyntaxKind::RecordExpression
            | SyntaxKind::LiteralExpression
            | SyntaxKind::NameExpression
            | SyntaxKind::Block
            | SyntaxKind::Error
    )
}

fn significant_direct_tokens(node: &SyntaxNode) -> Vec<SyntaxToken> {
    node.child_tokens()
        .filter(|token| !token.kind().is_trivia())
        .collect()
}

fn descendant_tokens(node: &SyntaxNode) -> Vec<SyntaxToken> {
    let mut output = Vec::new();
    collect_descendant_tokens(node, &mut output);
    output
}

fn collect_descendant_tokens(node: &SyntaxNode, output: &mut Vec<SyntaxToken>) {
    for child in node.children_with_tokens() {
        match child {
            SyntaxElement::Node(child_node) => collect_descendant_tokens(child_node, output),
            SyntaxElement::Token(token) => {
                if !token.kind().is_trivia() {
                    output.push(*token);
                }
            }
        }
    }
}

fn significant_text(node: &SyntaxNode, source: &SourceFile) -> String {
    descendant_tokens(node)
        .iter()
        .filter_map(|token| token.text(source))
        .collect::<Vec<_>>()
        .join("")
}

fn first_direct_identifier(node: &SyntaxNode, source: &SourceFile) -> Option<(String, Span)> {
    significant_direct_tokens(node)
        .into_iter()
        .find(|token| token.kind() == TokenKind::Identifier)
        .and_then(|token| token.text(source).map(|text| (text.to_owned(), token.span())))
}

fn function_name(node: &SyntaxNode, source: &SourceFile) -> Option<(String, Span)> {
    let tokens = significant_direct_tokens(node);
    let fn_index = tokens.iter().position(|token| {
        token.kind() == TokenKind::Keyword(Keyword::Fn)
    })?;
    tokens[fn_index + 1..]
        .iter()
        .copied()
        .find(|token| token.kind() == TokenKind::Identifier)
        .and_then(|token| token.text(source).map(|text| (text.to_owned(), token.span())))
}

fn generic_parameter_names(node: &SyntaxNode, source: &SourceFile) -> HashSet<String> {
    let mut output = HashSet::new();
    let Some(generics) = node
        .child_nodes()
        .find(|child| child.kind() == SyntaxKind::GenericParameterList)
    else {
        return output;
    };
    for token in descendant_tokens(generics) {
        if token.kind() == TokenKind::Identifier {
            if let Some(name) = token.text(source) {
                let _ = output.insert(name.to_owned());
            }
        }
    }
    output
}

fn parameter_parts(
    node: &SyntaxNode,
    source: &SourceFile,
) -> Option<(String, Span, String)> {
    let tokens = descendant_tokens(node);
    let name_token = tokens.iter().copied().find(|token| {
        matches!(
            token.kind(),
            TokenKind::Identifier | TokenKind::Keyword(Keyword::SelfValue)
        )
    })?;
    let name = name_token.text(source)?.to_owned();
    let colon = tokens.iter().position(|token| token.kind() == TokenKind::Colon);
    let Some(colon) = colon else {
        return Some((name, name_token.span(), String::new()));
    };
    let mut type_text = String::new();
    for token in &tokens[colon + 1..] {
        if token.kind() == TokenKind::Equal || token.kind() == TokenKind::Comma {
            break;
        }
        if let Some(text) = token.text(source) {
            if needs_type_space(&type_text, text) {
                type_text.push(' ');
            }
            type_text.push_str(text);
        }
    }
    Some((name, name_token.span(), type_text))
}

fn needs_type_space(current: &str, next: &str) -> bool {
    (current.ends_with("mut") || current.ends_with("const")) && next.chars().next().map_or(false, |character| character.is_alphanumeric())
}

fn pattern_name(node: &SyntaxNode, source: &SourceFile) -> Option<(String, Span)> {
    descendant_tokens(node)
        .into_iter()
        .find(|token| token.kind() == TokenKind::Identifier)
        .and_then(|token| token.text(source).map(|text| (text.to_owned(), token.span())))
}

fn direct_operator(node: &SyntaxNode) -> Option<TokenKind> {
    significant_direct_tokens(node)
        .into_iter()
        .find_map(|token| match token.kind() {
            kind @ (TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Percent
            | TokenKind::EqualEqual
            | TokenKind::BangEqual
            | TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::Greater
            | TokenKind::GreaterEqual
            | TokenKind::ShiftLeft
            | TokenKind::ShiftRight
            | TokenKind::Ampersand
            | TokenKind::AmpersandAmpersand
            | TokenKind::Pipe
            | TokenKind::PipePipe
            | TokenKind::Caret) => Some(kind),
            _ => None,
        })
}

fn collect_unknown_type_names(
    ty: &Type,
    known: &HashSet<String>,
    generics: &HashSet<String>,
    output: &mut Vec<String>,
) {
    match ty {
        Type::Named(name, arguments) => {
            if !known.contains(name) && !generics.contains(name) {
                output.push(name.clone());
            }
            for argument in arguments {
                collect_unknown_type_names(argument, known, generics, output);
            }
        }
        Type::Optional(inner)
        | Type::Reference { inner, .. }
        | Type::Pointer { inner, .. } => {
            collect_unknown_type_names(inner, known, generics, output);
        }
        Type::Tuple(items) | Type::Function(items, _) => {
            for item in items {
                collect_unknown_type_names(item, known, generics, output);
            }
            if let Type::Function(_, result) = ty {
                collect_unknown_type_names(result, known, generics, output);
            }
        }
        _ => {}
    }
}

struct TypeTextParser<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> TypeTextParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }

    fn parse(&mut self) -> Option<Type> {
        let ty = self.parse_type()?;
        self.skip_whitespace();
        (self.position == self.input.len()).then_some(ty)
    }

    fn parse_type(&mut self) -> Option<Type> {
        self.skip_whitespace();
        if self.consume("&") {
            self.skip_whitespace();
            let mutable = self.consume_word("mut");
            self.skip_whitespace();
            let inner = self.parse_type()?;
            return Some(Type::Reference {
                mutable,
                inner: Box::new(inner),
            });
        }
        if self.consume("*") {
            self.skip_whitespace();
            let mutable = if self.consume_word("mut") {
                true
            } else {
                let _ = self.consume_word("const");
                false
            };
            self.skip_whitespace();
            let inner = self.parse_type()?;
            return Some(Type::Pointer {
                mutable,
                inner: Box::new(inner),
            });
        }
        if self.consume("(") {
            let mut items = Vec::new();
            self.skip_whitespace();
            if self.consume(")") {
                return Some(Type::Unit);
            }
            loop {
                items.push(self.parse_type()?);
                self.skip_whitespace();
                if self.consume(")") {
                    break;
                }
                if !self.consume(",") {
                    return None;
                }
            }
            return Some(if items.len() == 1 {
                items.remove(0)
            } else {
                Type::Tuple(items)
            });
        }

        let name = self.parse_name()?;
        let mut arguments = Vec::new();
        self.skip_whitespace();
        if self.consume("<") {
            loop {
                arguments.push(self.parse_type()?);
                self.skip_whitespace();
                if self.consume(">") {
                    break;
                }
                if !self.consume(",") {
                    return None;
                }
            }
        }
        let mut ty = primitive_or_named(name, arguments);
        self.skip_whitespace();
        if self.consume("?") {
            ty = Type::Optional(Box::new(ty));
        }
        Some(ty)
    }

    fn parse_name(&mut self) -> Option<String> {
        self.skip_whitespace();
        let start = self.position;
        while let Some(character) = self.remaining().chars().next() {
            if character.is_alphanumeric() || matches!(character, '_' | ':' | '.') {
                self.position += character.len_utf8();
            } else {
                break;
            }
        }
        (self.position > start).then(|| self.input[start..self.position].to_owned())
    }

    fn consume_word(&mut self, word: &str) -> bool {
        let remaining = self.remaining();
        if !remaining.starts_with(word) {
            return false;
        }
        let boundary = remaining[word.len()..]
            .chars()
            .next()
            .map_or(true, |character| !character.is_alphanumeric() && character != '_');
        if boundary {
            self.position += word.len();
        }
        boundary
    }

    fn consume(&mut self, text: &str) -> bool {
        if self.remaining().starts_with(text) {
            self.position += text.len();
            true
        } else {
            false
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(character) = self.remaining().chars().next() {
            if character.is_whitespace() {
                self.position += character.len_utf8();
            } else {
                break;
            }
        }
    }

    fn remaining(&self) -> &str {
        &self.input[self.position..]
    }
}

fn primitive_or_named(name: String, arguments: Vec<Type>) -> Type {
    if !arguments.is_empty() {
        return Type::Named(name, arguments);
    }
    match name.as_str() {
        "Unit" => Type::Unit,
        "Never" => Type::Never,
        "Bool" => Type::Bool,
        "Char" => Type::Char,
        "String" => Type::String,
        "Int" | "I8" | "I16" | "I32" | "I64" | "U8" | "U16" | "U32" | "U64" | "Usize" => Type::Int,
        "Float" | "F32" | "F64" => Type::Float,
        _ => Type::Named(name, Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use nivra_parser::parse;
    use nivra_sema::analyze;
    use nivra_source::SourceManager;

    use super::{Type, TypeTextParser, check};

    fn check_text(text: &str) -> super::TypeCheckResult {
        let mut sources = SourceManager::new();
        let id = sources
            .add_virtual("test.nva", text)
            .unwrap_or_else(|error| panic!("{error}"));
        let source = sources
            .get(id)
            .unwrap_or_else(|| panic!("source disappeared"));
        let parsed = parse(source);
        assert!(!parsed.has_errors(), "{:?}", parsed.diagnostics);
        let semantic = analyze(source, &parsed.root);
        assert!(!semantic.has_errors(), "{:?}", semantic.diagnostics);
        check(source, &parsed.root, &semantic)
    }

    #[test]
    fn parses_optional_and_generic_types() {
        let mut parser = TypeTextParser::new("List<String?>");
        assert_eq!(
            parser.parse(),
            Some(Type::Named(
                "List".to_owned(),
                vec![Type::Optional(Box::new(Type::String))]
            ))
        );
    }

    #[test]
    fn infers_primitive_bindings() {
        let result = check_text(
            "module test\nfn main() { let count = 2\n let ratio = 1.5\n let enabled = true\n }\n",
        );
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert!(result.bindings.iter().any(|binding| binding.name == "count" && binding.ty == Type::Int));
        assert!(result.bindings.iter().any(|binding| binding.name == "ratio" && binding.ty == Type::Float));
        assert!(result.bindings.iter().any(|binding| binding.name == "enabled" && binding.ty == Type::Bool));
    }

    #[test]
    fn rejects_binding_type_mismatch() {
        let result = check_text("module test\nfn main() { let count: Int = \"two\"\n }\n");
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "TYP001"));
    }

    #[test]
    fn validates_function_calls() {
        let result = check_text(
            "module test\nfn add(a: Int, b: Int) -> Int { a + b }\nfn main() { let total = add(1, 2)\n }\n",
        );
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
    }

    #[test]
    fn rejects_wrong_call_arity() {
        let result = check_text(
            "module test\nfn add(a: Int, b: Int) -> Int { a + b }\nfn main() { add(1) }\n",
        );
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "TYP003"));
    }

    #[test]
    fn rejects_wrong_argument_type() {
        let result = check_text(
            "module test\nfn echo(value: Int) -> Int { value }\nfn main() { echo(\"wrong\") }\n",
        );
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "TYP004"));
    }

    #[test]
    fn rejects_non_boolean_condition() {
        let result = check_text("module test\nfn main() { if 1 { print(1) } }\n");
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "TYP007"));
    }

    #[test]
    fn rejects_bad_return_type() {
        let result = check_text("module test\nfn answer() -> Int { \"forty-two\" }\n");
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "TYP005"));
    }

    #[test]
    fn rejects_none_without_context() {
        let result = check_text("module test\nfn main() { let value = none\n }\n");
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "TYP006"));
    }

    #[test]
    fn accepts_none_with_optional_context() {
        let result = check_text("module test\nfn main() { let value: String? = none\n }\n");
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
    }

    #[test]
    fn rejects_heterogeneous_array() {
        let result = check_text("module test\nfn main() { let values = [1, \"two\"]\n }\n");
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "TYP009"));
    }

    #[test]
    fn rejects_assignment_to_let() {
        let result = check_text("module test\nfn main() { let value = 1\n value = 2\n }\n");
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "TYP010"));
    }
}
