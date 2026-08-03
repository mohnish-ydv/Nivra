//! Flow-sensitive ownership, borrow, move, and deterministic-drop analysis for Nivra.
//!
//! D9 operates after type checking. Edition 2026 intentionally avoids user-written
//! lifetime parameters, so borrow regions are inferred from lexical ownership plus the
//! last use of a local reference binding. Control-flow joins are conservative: a value
//! moved on only some paths becomes `maybe_moved` until it is reinitialized.

use std::collections::{HashMap, HashSet};
use std::fmt::{self, Write as _};

use nivra_diagnostics::Diagnostic;
use nivra_lexer::{Keyword, TokenKind};
use nivra_source::{SourceFile, Span};
use nivra_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};
use nivra_types::{FunctionSignature, Type, TypeCheckResult};

/// Whether an assignment or argument transfer copies or moves a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OwnershipClass {
    /// Transfer duplicates the bits and leaves the source available.
    Copy,
    /// Transfer changes ownership and invalidates the source place.
    Move,
}

impl OwnershipClass {
    /// Stable report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Copy => "copy",
            Self::Move => "move",
        }
    }
}

/// Flow state of an owned binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ValueState {
    Available,
    Moved,
    MaybeMoved,
}

impl ValueState {
    /// Stable report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Moved => "moved",
            Self::MaybeMoved => "maybe_moved",
        }
    }
}

/// Kind of inferred borrow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BorrowKind {
    Shared,
    Mutable,
}

impl BorrowKind {
    /// Stable report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Shared => "shared",
            Self::Mutable => "mutable",
        }
    }
}

/// Observable ownership-flow event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OwnershipEventKind {
    Declare,
    Read,
    Copy,
    Move,
    BorrowShared,
    BorrowMutable,
    Assign,
    Reinitialize,
    Defer,
    Drop,
}

impl OwnershipEventKind {
    /// Stable report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Declare => "declare",
            Self::Read => "read",
            Self::Copy => "copy",
            Self::Move => "move",
            Self::BorrowShared => "borrow_shared",
            Self::BorrowMutable => "borrow_mutable",
            Self::Assign => "assign",
            Self::Reinitialize => "reinitialize",
            Self::Defer => "defer",
            Self::Drop => "drop",
        }
    }
}

/// One source-ordered ownership event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnershipEvent {
    pub kind: OwnershipEventKind,
    pub place: String,
    pub ty: Type,
    pub span: Span,
    pub scope_id: usize,
}

/// One final binding summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindingOwnership {
    pub name: String,
    pub ty: Type,
    pub class: OwnershipClass,
    pub mutable: bool,
    pub state: ValueState,
    pub declaration_span: Span,
    pub scope_id: usize,
    pub partial_moves: Vec<String>,
}

/// Scope-exit operation category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExitActionKind {
    Defer,
    Drop,
}

impl ExitActionKind {
    /// Stable report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Defer => "defer",
            Self::Drop => "drop",
        }
    }
}

/// Deterministic scope-exit plan entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExitAction {
    pub scope_id: usize,
    pub order: usize,
    pub kind: ExitActionKind,
    pub name: String,
    pub ty: Type,
    pub span: Span,
    /// True when a control-flow or partial-move drop flag is required.
    pub conditional: bool,
}

/// Complete D9 analysis result.
#[derive(Clone, Debug)]
pub struct OwnershipResult {
    pub bindings: Vec<BindingOwnership>,
    pub events: Vec<OwnershipEvent>,
    pub exit_actions: Vec<ExitAction>,
    pub diagnostics: Vec<Diagnostic>,
}

impl OwnershipResult {
    /// Returns whether ownership analysis produced an error.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }

    /// Number of explicit move events.
    #[must_use]
    pub fn move_count(&self) -> usize {
        self.events
            .iter()
            .filter(|event| event.kind == OwnershipEventKind::Move)
            .count()
    }

    /// Number of inferred borrow events.
    #[must_use]
    pub fn borrow_count(&self) -> usize {
        self.events
            .iter()
            .filter(|event| {
                matches!(
                    event.kind,
                    OwnershipEventKind::BorrowShared | OwnershipEventKind::BorrowMutable
                )
            })
            .count()
    }

    /// Deterministic, phone-friendly event report.
    #[must_use]
    pub fn event_report(&self) -> String {
        let mut output = String::new();
        for event in &self.events {
            let _ = writeln!(
                output,
                "{:<16} {:<24} {:<20} scope={} {}..{}",
                event.kind.as_str(),
                event.place,
                event.ty,
                event.scope_id,
                event.span.start(),
                event.span.end()
            );
        }
        output
    }

    /// Deterministic binding-state report.
    #[must_use]
    pub fn binding_report(&self) -> String {
        let mut output = String::new();
        for binding in &self.bindings {
            let partial = if binding.partial_moves.is_empty() {
                "-".to_owned()
            } else {
                binding.partial_moves.join(",")
            };
            let _ = writeln!(
                output,
                "{:<20} {:<20} {:<6} {:<12} mutable={} scope={} partial={}",
                binding.name,
                binding.ty,
                binding.class.as_str(),
                binding.state.as_str(),
                binding.mutable,
                binding.scope_id,
                partial
            );
        }
        output
    }

    /// Deterministic defer/drop plan report.
    #[must_use]
    pub fn drop_report(&self) -> String {
        let mut output = String::new();
        for action in &self.exit_actions {
            let _ = writeln!(
                output,
                "scope={} order={:<3} {:<6} {:<24} {:<20} conditional={}",
                action.scope_id,
                action.order,
                action.kind.as_str(),
                action.name,
                action.ty,
                action.conditional
            );
        }
        output
    }
}

/// Runs D9 ownership analysis after successful type checking.
#[must_use]
pub fn analyze(source: &SourceFile, root: &SyntaxNode, typed: &TypeCheckResult) -> OwnershipResult {
    Analyzer::new(source, typed).run(root)
}

#[derive(Clone, Debug)]
struct BindingRecord {
    name: String,
    ty: Type,
    class: OwnershipClass,
    mutable: bool,
    state: ValueState,
    declaration_span: Span,
    scope_id: usize,
    is_parameter: bool,
    borrow_origin: Option<usize>,
    moved_places: HashSet<String>,
    maybe_moved_places: HashSet<String>,
}

#[derive(Clone, Debug)]
struct Loan {
    owner_binding: usize,
    borrow_scope_id: usize,
    place: String,
    kind: BorrowKind,
    span: Span,
    end: usize,
    active: bool,
}

#[derive(Clone, Debug)]
struct DeferredAction {
    text: String,
    span: Span,
}

#[derive(Clone, Debug)]
struct ScopeFrame {
    id: usize,
    bindings: Vec<usize>,
    names: HashMap<String, usize>,
    defers: Vec<DeferredAction>,
}

#[derive(Clone, Debug)]
struct FlowSnapshot {
    state: ValueState,
    moved_places: HashSet<String>,
    maybe_moved_places: HashSet<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UseMode {
    Read,
    Consume,
}

struct Analyzer<'a> {
    source: &'a SourceFile,
    typed: &'a TypeCheckResult,
    bindings: Vec<BindingRecord>,
    scopes: Vec<ScopeFrame>,
    loans: Vec<Loan>,
    events: Vec<OwnershipEvent>,
    exit_actions: Vec<ExitAction>,
    diagnostics: Vec<Diagnostic>,
    expression_types: HashMap<(usize, usize), Type>,
    next_scope_id: usize,
    name_uses: HashMap<String, Vec<usize>>,
}

impl<'a> Analyzer<'a> {
    fn new(source: &'a SourceFile, typed: &'a TypeCheckResult) -> Self {
        let expression_types = typed
            .expressions
            .iter()
            .map(|expression| {
                (
                    (expression.span.start(), expression.span.end()),
                    expression.ty.clone(),
                )
            })
            .collect();
        Self {
            source,
            typed,
            bindings: Vec::new(),
            scopes: Vec::new(),
            loans: Vec::new(),
            events: Vec::new(),
            exit_actions: Vec::new(),
            diagnostics: Vec::new(),
            expression_types,
            next_scope_id: 0,
            name_uses: HashMap::new(),
        }
    }

    fn run(mut self, root: &SyntaxNode) -> OwnershipResult {
        self.reject_borrowed_fields();
        self.visit_functions(root);
        let bindings = self
            .bindings
            .into_iter()
            .map(|binding| {
                let mut partial_moves = binding
                    .moved_places
                    .union(&binding.maybe_moved_places)
                    .cloned()
                    .collect::<Vec<_>>();
                partial_moves.sort();
                BindingOwnership {
                    name: binding.name,
                    ty: binding.ty,
                    class: binding.class,
                    mutable: binding.mutable,
                    state: binding.state,
                    declaration_span: binding.declaration_span,
                    scope_id: binding.scope_id,
                    partial_moves,
                }
            })
            .collect();
        OwnershipResult {
            bindings,
            events: self.events,
            exit_actions: self.exit_actions,
            diagnostics: self.diagnostics,
        }
    }

    fn reject_borrowed_fields(&mut self) {
        for nominal in &self.typed.nominals {
            for field in &nominal.fields {
                if contains_reference(&field.ty) {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "BOR006",
                            format!(
                                "borrowed field `{}` is not supported in Edition 2026",
                                field.name
                            ),
                        )
                        .with_primary(field.span, "this field stores a non-owning borrow")
                        .with_note("Edition 2026 keeps borrows local to functions and expressions")
                        .with_help("store an owned value, Box<T>, Shared<T>, or an index/handle instead"),
                    );
                }
            }
            for variant in &nominal.variants {
                if variant.payload.iter().any(contains_reference) {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "BOR006",
                            format!(
                                "borrowed payload in variant `{}` is not supported in Edition 2026",
                                variant.name
                            ),
                        )
                        .with_primary(variant.span, "this variant stores a non-owning borrow")
                        .with_note("Edition 2026 keeps borrows local to functions and expressions")
                        .with_help("store an owned value, Box<T>, Shared<T>, or an index/handle instead"),
                    );
                }
            }
        }
    }

    fn visit_functions(&mut self, node: &SyntaxNode) {
        if node.kind() == SyntaxKind::FunctionDeclaration {
            self.analyze_function(node);
            return;
        }
        for child in node.child_nodes() {
            self.visit_functions(child);
        }
    }

    fn analyze_function(&mut self, node: &SyntaxNode) {
        self.name_uses.clear();
        collect_name_uses(node, self.source, &mut self.name_uses);
        let signature = self
            .typed
            .functions
            .iter()
            .find(|candidate| candidate.span == node.span())
            .cloned();
        if let Some(signature) = &signature {
            self.validate_return_borrow_origin(signature);
        }

        self.push_scope();
        if let Some(signature) = &signature {
            for parameter in &signature.parameters {
                let binding_id = self.declare_binding(
                    &parameter.name,
                    parameter.ty.clone(),
                    matches!(&parameter.ty, Type::Reference { mutable: true, .. }),
                    parameter.span,
                );
                self.bindings[binding_id].is_parameter = true;
                if matches!(&parameter.ty, Type::Reference { .. }) {
                    self.bindings[binding_id].borrow_origin = Some(binding_id);
                }
            }
        }

        if let Some(block) = node
            .child_nodes()
            .find(|child| child.kind() == SyntaxKind::Block)
        {
            self.analyze_block(block, false, signature.as_ref());
        }
        self.pop_scope(node.span().end());
        self.loans.clear();
    }

    fn validate_return_borrow_origin(&mut self, signature: &FunctionSignature) {
        if !matches!(&signature.return_type, Type::Reference { .. }) {
            return;
        }
        let borrowed_inputs = signature
            .parameters
            .iter()
            .filter(|parameter| matches!(&parameter.ty, Type::Reference { .. }))
            .count();
        if borrowed_inputs != 1 {
            self.diagnostics.push(
                Diagnostic::error(
                    "BOR007",
                    format!(
                        "borrowed return from `{}` needs exactly one borrowed input",
                        signature.name
                    ),
                )
                .with_primary(signature.span, "borrow origin is not unambiguous")
                .with_note(format!("found {borrowed_inputs} borrowed input parameters"))
                .with_help("return an owned value or accept exactly one source borrow"),
            );
        }
    }

    fn analyze_block(
        &mut self,
        block: &SyntaxNode,
        creates_scope: bool,
        signature: Option<&FunctionSignature>,
    ) {
        if creates_scope {
            self.push_scope();
        }
        let statements = block.child_nodes().collect::<Vec<_>>();
        let last_index = statements.len().saturating_sub(1);
        for (index, statement) in statements.into_iter().enumerate() {
            self.expire_loans(statement.span().start());
            if index == last_index
                && signature.is_some_and(|function| {
                    matches!(&function.return_type, Type::Reference { .. })
                })
                && statement.kind() == SyntaxKind::ExpressionStatement
            {
                if let Some(expression) = statement.child_nodes().next() {
                    if matches!(self.expression_type(expression), Type::Reference { .. }) {
                        self.check_returned_borrow(expression, signature);
                    }
                }
            }
            self.analyze_statement(statement, signature);
        }
        if creates_scope {
            self.pop_scope(block.span().end());
        }
    }

    fn analyze_statement(&mut self, node: &SyntaxNode, signature: Option<&FunctionSignature>) {
        match node.kind() {
            SyntaxKind::LetStatement | SyntaxKind::VarStatement => self.analyze_binding(node),
            SyntaxKind::ReturnStatement => {
                if let Some(expression) = node.child_nodes().next() {
                    if matches!(self.expression_type(expression), Type::Reference { .. }) {
                        self.check_returned_borrow(expression, signature);
                    }
                    self.eval_expression(expression, UseMode::Consume, node.span().end());
                }
            }
            SyntaxKind::DeferStatement => {
                if let Some(expression) = node.child_nodes().next() {
                    self.eval_expression(expression, UseMode::Read, usize::MAX);
                    if let Some(scope) = self.scopes.last_mut() {
                        scope.defers.push(DeferredAction {
                            text: significant_text(expression, self.source),
                            span: node.span(),
                        });
                    }
                    self.record_event(
                        OwnershipEventKind::Defer,
                        significant_text(expression, self.source),
                        Type::Unit,
                        node.span(),
                    );
                }
            }
            SyntaxKind::WhileStatement => self.analyze_while(node, signature),
            SyntaxKind::ForStatement => self.analyze_for(node, signature),
            SyntaxKind::ExpressionStatement => {
                if let Some(expression) = node.child_nodes().next() {
                    self.eval_expression(expression, UseMode::Read, node.span().end());
                }
            }
            _ if is_expression_kind(node.kind()) => {
                self.eval_expression(node, UseMode::Read, node.span().end());
            }
            _ => {
                for child in node.child_nodes() {
                    self.analyze_statement(child, signature);
                }
            }
        }
    }

    fn analyze_binding(&mut self, node: &SyntaxNode) {
        let pattern = node
            .child_nodes()
            .find(|child| child.kind() == SyntaxKind::Pattern);
        let Some(pattern) = pattern else {
            return;
        };
        let Some((name, name_span)) = first_identifier(pattern, self.source) else {
            return;
        };
        let initializer = node
            .child_nodes()
            .find(|child| is_expression_kind(child.kind()));
        if let Some(initializer) = initializer {
            self.eval_expression(initializer, UseMode::Consume, node.span().end());
        }
        let ty = self
            .typed
            .bindings
            .iter()
            .find(|binding| binding.name == name && binding.span == name_span)
            .map_or_else(
                || initializer.map_or(Type::Unknown, |value| self.expression_type(value)),
                |binding| binding.ty.clone(),
            );
        let binding_id = self.declare_binding(
            &name,
            ty.clone(),
            node.kind() == SyntaxKind::VarStatement,
            name_span,
        );

        if let Some(initializer) = initializer {
            if let Some((kind, place, borrow_span)) = borrow_expression(initializer, self.source) {
                if let Some(owner_id) = self.lookup_place_binding(&place) {
                    let last_use = self
                        .name_uses
                        .get(&name)
                        .and_then(|uses| {
                            uses.iter()
                                .copied()
                                .filter(|offset| *offset > initializer.span().end())
                                .max()
                        })
                        .unwrap_or(initializer.span().end());
                    self.attach_or_extend_loan(owner_id, &place, kind, borrow_span, last_use);
                    self.bindings[binding_id].borrow_origin = Some(owner_id);
                }
            } else if let Some(place) = place_text(initializer, self.source) {
                if let Some(source_id) = self.lookup_place_binding(&place) {
                    if matches!(&self.bindings[source_id].ty, Type::Reference { .. }) {
                        let origin = self.bindings[source_id].borrow_origin.or(Some(source_id));
                        self.bindings[binding_id].borrow_origin = origin;
                    }
                }
            }
        }
    }

    fn analyze_while(&mut self, node: &SyntaxNode, signature: Option<&FunctionSignature>) {
        let children = node.child_nodes().collect::<Vec<_>>();
        if let Some(condition) = children.first() {
            self.eval_expression(condition, UseMode::Read, condition.span().end());
        }
        let before = self.snapshot();
        if let Some(block) = children
            .iter()
            .copied()
            .find(|child| child.kind() == SyntaxKind::Block)
        {
            self.analyze_block(block, true, signature);
        }
        let after = self.snapshot();
        self.restore(&before);
        self.merge_snapshots(&before, &after);
    }

    fn analyze_for(&mut self, node: &SyntaxNode, signature: Option<&FunctionSignature>) {
        let iterable = node
            .child_nodes()
            .find(|child| is_expression_kind(child.kind()));
        if let Some(iterable) = iterable {
            self.eval_expression(iterable, UseMode::Read, iterable.span().end());
        }
        let before = self.snapshot();
        self.push_scope();
        if let Some(pattern) = node
            .child_nodes()
            .find(|child| child.kind() == SyntaxKind::Pattern)
        {
            if let Some((name, span)) = first_identifier(pattern, self.source) {
                self.declare_binding(&name, Type::Unknown, false, span);
            }
        }
        if let Some(block) = node
            .child_nodes()
            .find(|child| child.kind() == SyntaxKind::Block)
        {
            self.analyze_block(block, false, signature);
        }
        self.pop_scope(node.span().end());
        let after = self.snapshot();
        self.restore(&before);
        self.merge_snapshots(&before, &after);
    }

    fn eval_expression(&mut self, node: &SyntaxNode, mode: UseMode, loan_end: usize) -> Type {
        self.expire_loans(node.span().start());
        match node.kind() {
            SyntaxKind::NameExpression | SyntaxKind::MemberExpression | SyntaxKind::IndexExpression => {
                if let Some(place) = place_text(node, self.source) {
                    self.use_place(&place, node.span(), mode);
                } else {
                    for child in node.child_nodes() {
                        self.eval_expression(child, UseMode::Read, loan_end);
                    }
                }
            }
            SyntaxKind::PrefixExpression => {
                let direct = significant_direct_tokens(node);
                if direct.first().is_some_and(|token| token.kind() == TokenKind::Ampersand) {
                    if let Some(operand) = node.child_nodes().next() {
                        if let Some(place) = place_text(operand, self.source) {
                            let mutable = significant_text(node, self.source).starts_with("&mut");
                            self.borrow_place(
                                &place,
                                if mutable {
                                    BorrowKind::Mutable
                                } else {
                                    BorrowKind::Shared
                                },
                                node.span(),
                                loan_end,
                            );
                        } else {
                            self.eval_expression(operand, UseMode::Read, loan_end);
                        }
                    }
                } else if direct
                    .first()
                    .is_some_and(|token| token.kind() == TokenKind::Keyword(Keyword::Move))
                {
                    for child in node.child_nodes() {
                        self.eval_expression(child, UseMode::Consume, loan_end);
                    }
                } else {
                    for child in node.child_nodes() {
                        self.eval_expression(child, mode, loan_end);
                    }
                }
            }
            SyntaxKind::AssignmentExpression => self.eval_assignment(node),
            SyntaxKind::CallExpression => self.eval_call(node, loan_end),
            SyntaxKind::IfExpression => self.eval_if(node),
            SyntaxKind::MatchExpression => self.eval_match(node),
            SyntaxKind::AwaitExpression => {
                self.reject_borrow_across_await(node.span());
                for child in node.child_nodes() {
                    self.eval_expression(child, UseMode::Consume, node.span().end());
                }
            }
            SyntaxKind::Block => self.analyze_block(node, true, None),
            SyntaxKind::RecordExpression
            | SyntaxKind::ArrayExpression
            | SyntaxKind::TupleExpression => {
                for child in node.child_nodes() {
                    if child.kind() == SyntaxKind::RecordFieldInitializer {
                        for value in child.child_nodes() {
                            self.eval_expression(value, UseMode::Consume, child.span().end());
                        }
                    } else if is_expression_kind(child.kind()) {
                        self.eval_expression(child, UseMode::Consume, node.span().end());
                    }
                }
            }
            SyntaxKind::BinaryExpression => {
                for child in node.child_nodes() {
                    self.eval_expression(child, UseMode::Read, node.span().end());
                }
            }
            SyntaxKind::ParenthesizedExpression
            | SyntaxKind::TryExpression
            | SyntaxKind::AsyncExpression
            | SyntaxKind::SpawnExpression => {
                for child in node.child_nodes() {
                    self.eval_expression(child, mode, loan_end);
                }
            }
            SyntaxKind::ClosureExpression => {
                for child in node.child_nodes() {
                    self.eval_expression(child, UseMode::Read, node.span().end());
                }
            }
            _ => {
                for child in node.child_nodes() {
                    if is_expression_kind(child.kind()) {
                        self.eval_expression(child, mode, loan_end);
                    }
                }
            }
        }
        self.expression_type(node)
    }

    fn eval_assignment(&mut self, node: &SyntaxNode) {
        let children = node.child_nodes().collect::<Vec<_>>();
        if children.len() < 2 {
            return;
        }
        self.eval_expression(children[1], UseMode::Consume, node.span().end());
        if let Some(place) = place_text(children[0], self.source) {
            self.assign_place(&place, children[0].span());
        } else {
            self.eval_expression(children[0], UseMode::Read, node.span().end());
        }
    }

    fn eval_call(&mut self, node: &SyntaxNode, loan_end: usize) {
        let children = node.child_nodes().collect::<Vec<_>>();
        let Some(callee) = children.first().copied() else {
            return;
        };
        let Some(argument_list) = children
            .iter()
            .copied()
            .find(|child| child.kind() == SyntaxKind::ArgumentList)
        else {
            return;
        };
        let arguments = argument_list.child_nodes().collect::<Vec<_>>();
        let callee_name = callable_name(callee, self.source);

        if callee.kind() == SyntaxKind::MemberExpression {
            if let Some(base) = callee.child_nodes().next() {
                self.eval_expression(base, UseMode::Read, loan_end);
            }
        }

        let parameter_types = callee_name
            .as_ref()
            .and_then(|name| self.function_parameters(name));
        for (index, argument) in arguments.iter().enumerate() {
            let parameter = parameter_types
                .as_ref()
                .and_then(|parameters| parameters.get(index));
            let mode = match parameter {
                Some(Type::Reference { .. }) => UseMode::Read,
                _ if callee_name.as_deref().is_some_and(is_observer_builtin) => UseMode::Read,
                _ => UseMode::Consume,
            };
            self.eval_expression(argument, mode, loan_end);
        }
        self.expire_loans(node.span().end());
    }

    fn eval_if(&mut self, node: &SyntaxNode) {
        let children = node.child_nodes().collect::<Vec<_>>();
        if let Some(condition) = children
            .iter()
            .copied()
            .find(|child| is_expression_kind(child.kind()) && child.kind() != SyntaxKind::Block)
        {
            self.eval_expression(condition, UseMode::Read, condition.span().end());
        }
        let branches = children
            .iter()
            .copied()
            .filter(|child| {
                child.kind() == SyntaxKind::Block || child.kind() == SyntaxKind::IfExpression
            })
            .collect::<Vec<_>>();
        let before = self.snapshot();
        let mut outcomes = Vec::new();
        for branch in &branches {
            self.restore(&before);
            if branch.kind() == SyntaxKind::Block {
                self.analyze_block(branch, true, None);
            } else {
                self.eval_if(branch);
            }
            outcomes.push(self.snapshot_prefix(before.len()));
        }
        if branches.len() < 2 {
            outcomes.push(before.clone());
        }
        if let Some((first, rest)) = outcomes.split_first() {
            self.restore(first);
            for outcome in rest {
                let current = self.snapshot();
                self.merge_snapshots(&current, outcome);
            }
        } else {
            self.restore(&before);
        }
    }

    fn eval_match(&mut self, node: &SyntaxNode) {
        let mut children = node.child_nodes();
        if let Some(scrutinee) = children.next() {
            self.eval_expression(scrutinee, UseMode::Consume, scrutinee.span().end());
        }
        let before = self.snapshot();
        let arms = node
            .children_by_kind(SyntaxKind::MatchArm)
            .collect::<Vec<_>>();
        let mut outcomes = Vec::new();
        for arm in arms {
            self.restore(&before);
            self.push_scope();
            if let Some(expression) = arm
                .child_nodes()
                .find(|child| is_expression_kind(child.kind()))
            {
                self.eval_expression(expression, UseMode::Read, arm.span().end());
            }
            self.pop_scope(arm.span().end());
            outcomes.push(self.snapshot_prefix(before.len()));
        }
        if let Some((first, rest)) = outcomes.split_first() {
            self.restore(first);
            for outcome in rest {
                let current = self.snapshot();
                self.merge_snapshots(&current, outcome);
            }
        } else {
            self.restore(&before);
        }
    }

    fn use_place(&mut self, place: &str, span: Span, mode: UseMode) {
        let Some(binding_id) = self.lookup_place_binding(place) else {
            return;
        };
        self.expire_loans(span.start());
        let state = self.bindings[binding_id].state;
        let declaration_span = self.bindings[binding_id].declaration_span;
        let root_name = self.bindings[binding_id].name.clone();
        let class = self.place_class(binding_id, place);

        if state == ValueState::Moved {
            self.diagnostics.push(
                Diagnostic::error("OWN001", format!("use of moved value `{root_name}`"))
                    .with_primary(span, "value used here after ownership was transferred")
                    .with_secondary(declaration_span, "value declared here")
                    .with_help("borrow the value, clone it explicitly, or reinitialize the binding"),
            );
            return;
        }
        if state == ValueState::MaybeMoved {
            self.diagnostics.push(
                Diagnostic::error(
                    "OWN007",
                    format!("value `{root_name}` may have been moved on another control-flow path"),
                )
                .with_primary(span, "this use is not valid on every path")
                .with_secondary(declaration_span, "value declared here")
                .with_help("reinitialize the value on every path or borrow it instead"),
            );
            return;
        }
        if self.place_was_moved(binding_id, place) {
            self.diagnostics.push(
                Diagnostic::error("OWN006", format!("use of moved place `{place}`"))
                    .with_primary(span, "this field or indexed place was already moved")
                    .with_secondary(declaration_span, "owner declared here")
                    .with_help("reassign the moved place or move/borrow the complete value only once"),
            );
            return;
        }
        if self.bindings[binding_id].moved_places.len() > 0 && place == root_name {
            self.diagnostics.push(
                Diagnostic::error("OWN006", format!("use of partially moved value `{root_name}`"))
                    .with_primary(span, "one or more fields were moved earlier")
                    .with_secondary(declaration_span, "owner declared here")
                    .with_help("reinitialize the moved fields before using the complete value"),
            );
            return;
        }
        if self.has_active_mutable_loan(place) {
            self.diagnostics.push(
                Diagnostic::error("BOR005", format!("cannot use `{place}` while it is mutably borrowed"))
                    .with_primary(span, "owner use conflicts with an active exclusive borrow")
                    .with_help("finish using the mutable reference before accessing the owner"),
            );
            return;
        }

        match mode {
            UseMode::Read => self.record_event(
                OwnershipEventKind::Read,
                place.to_owned(),
                self.place_type(binding_id, place),
                span,
            ),
            UseMode::Consume if class == OwnershipClass::Copy => self.record_event(
                OwnershipEventKind::Copy,
                place.to_owned(),
                self.place_type(binding_id, place),
                span,
            ),
            UseMode::Consume => {
                if let Some(loan) = self.active_overlapping_loan(place).cloned() {
                    self.diagnostics.push(
                        Diagnostic::error("OWN002", format!("cannot move `{place}` while it is borrowed"))
                            .with_primary(span, "ownership transfer occurs here")
                            .with_secondary(loan.span, format!("{} borrow starts here", loan.kind.as_str()))
                            .with_help("finish using the borrow before moving the value"),
                    );
                    return;
                }
                if place == root_name {
                    self.bindings[binding_id].state = ValueState::Moved;
                } else {
                    let _ = self.bindings[binding_id].moved_places.insert(place.to_owned());
                }
                self.record_event(
                    OwnershipEventKind::Move,
                    place.to_owned(),
                    self.place_type(binding_id, place),
                    span,
                );
            }
        }
    }

    fn borrow_place(&mut self, place: &str, kind: BorrowKind, span: Span, end: usize) {
        let Some(binding_id) = self.lookup_place_binding(place) else {
            return;
        };
        if self.bindings[binding_id].state != ValueState::Available
            || self.place_was_moved(binding_id, place)
        {
            self.diagnostics.push(
                Diagnostic::error("OWN001", format!("cannot borrow moved value `{place}`"))
                    .with_primary(span, "borrow begins after the value became unavailable")
                    .with_secondary(
                        self.bindings[binding_id].declaration_span,
                        "owner declared here",
                    )
                    .with_help("reinitialize the owner before borrowing it"),
            );
            return;
        }
        if kind == BorrowKind::Mutable && !self.bindings[binding_id].mutable {
            self.diagnostics.push(
                Diagnostic::error("BOR003", format!("cannot mutably borrow immutable binding `{place}`"))
                    .with_primary(span, "exclusive mutation was requested here")
                    .with_secondary(
                        self.bindings[binding_id].declaration_span,
                        "binding declared immutable here",
                    )
                    .with_help("declare the owner with `var` or use a shared `&` borrow"),
            );
            return;
        }
        if let Some(conflict) = self.borrow_conflict(place, kind).cloned() {
            let code = match (kind, conflict.kind) {
                (BorrowKind::Shared, BorrowKind::Mutable) => "BOR002",
                (BorrowKind::Mutable, BorrowKind::Mutable) => "BOR001",
                (BorrowKind::Mutable, BorrowKind::Shared) => "BOR001",
                (BorrowKind::Shared, BorrowKind::Shared) => return,
            };
            self.diagnostics.push(
                Diagnostic::error(
                    code,
                    format!(
                        "{} borrow of `{place}` conflicts with an active {} borrow",
                        kind.as_str(),
                        conflict.kind.as_str()
                    ),
                )
                .with_primary(span, "conflicting borrow starts here")
                .with_secondary(conflict.span, "earlier borrow remains live here")
                .with_help("end the earlier reference use before creating this borrow"),
            );
            return;
        }
        self.loans.push(Loan {
            owner_binding: binding_id,
            borrow_scope_id: self.current_scope_id(),
            place: place.to_owned(),
            kind,
            span,
            end,
            active: true,
        });
        self.record_event(
            if kind == BorrowKind::Mutable {
                OwnershipEventKind::BorrowMutable
            } else {
                OwnershipEventKind::BorrowShared
            },
            place.to_owned(),
            self.place_type(binding_id, place),
            span,
        );
    }

    fn attach_or_extend_loan(
        &mut self,
        owner_id: usize,
        place: &str,
        kind: BorrowKind,
        span: Span,
        end: usize,
    ) {
        if let Some(loan) = self.loans.iter_mut().rev().find(|loan| {
            loan.owner_binding == owner_id
                && loan.place == place
                && loan.kind == kind
                && loan.span == span
        }) {
            loan.end = loan.end.max(end);
        }
    }

    fn assign_place(&mut self, place: &str, span: Span) {
        let Some(binding_id) = self.lookup_place_binding(place) else {
            return;
        };
        if let Some(loan) = self.active_overlapping_loan(place).cloned() {
            self.diagnostics.push(
                Diagnostic::error("BOR004", format!("cannot assign to `{place}` while it is borrowed"))
                    .with_primary(span, "write occurs here")
                    .with_secondary(loan.span, "active borrow starts here")
                    .with_help("finish using the reference before assigning to its owner"),
            );
            return;
        }
        if !self.bindings[binding_id].mutable {
            return;
        }
        let root_name = self.bindings[binding_id].name.clone();
        let was_unavailable = self.bindings[binding_id].state != ValueState::Available
            || self.place_was_moved(binding_id, place);
        if place == root_name {
            self.bindings[binding_id].state = ValueState::Available;
            self.bindings[binding_id].moved_places.clear();
            self.bindings[binding_id].maybe_moved_places.clear();
        } else {
            self.bindings[binding_id]
                .moved_places
                .retain(|moved| !places_overlap(moved, place));
            self.bindings[binding_id]
                .maybe_moved_places
                .retain(|moved| !places_overlap(moved, place));
        }
        self.record_event(
            if was_unavailable {
                OwnershipEventKind::Reinitialize
            } else {
                OwnershipEventKind::Assign
            },
            place.to_owned(),
            self.place_type(binding_id, place),
            span,
        );
    }

    fn check_returned_borrow(
        &mut self,
        expression: &SyntaxNode,
        signature: Option<&FunctionSignature>,
    ) {
        let direct_origin = borrow_expression(expression, self.source)
            .and_then(|(_, place, _)| self.lookup_place_binding(&place));
        let alias_origin = place_text(expression, self.source)
            .and_then(|place| self.lookup_place_binding(&place))
            .and_then(|binding_id| {
                matches!(&self.bindings[binding_id].ty, Type::Reference { .. })
                    .then_some(self.bindings[binding_id].borrow_origin.unwrap_or(binding_id))
            });
        let Some(origin_id) = direct_origin.or(alias_origin) else {
            return;
        };
        let origin_name = self.bindings[origin_id].name.clone();
        let is_parameter = self.bindings[origin_id].is_parameter
            || signature.is_some_and(|function| {
                function
                    .parameters
                    .iter()
                    .any(|parameter| parameter.name == origin_name)
            });
        if !is_parameter {
            self.diagnostics.push(
                Diagnostic::error(
                    "BOR008",
                    format!("cannot return a borrow of local `{origin_name}`"),
                )
                .with_primary(expression.span(), "this borrow would outlive its owner")
                .with_secondary(
                    self.bindings[origin_id].declaration_span,
                    "local owner is destroyed at function exit",
                )
                .with_help("return an owned value or borrow from the function's single borrowed input"),
            );
        }
    }

    fn reject_borrow_across_await(&mut self, span: Span) {
        self.expire_loans(span.start());
        if let Some(loan) = self.loans.iter().find(|loan| loan.active) {
            self.diagnostics.push(
                Diagnostic::error("BOR009", "borrow cannot cross an `await` suspension point")
                    .with_primary(span, "task may suspend here while this borrow is live")
                    .with_secondary(loan.span, format!("borrow of `{}` begins here", loan.place))
                    .with_help("finish using the reference before `await` or move owned data into the task"),
            );
        }
    }

    fn declare_binding(&mut self, name: &str, ty: Type, mutable: bool, span: Span) -> usize {
        let scope_id = self.current_scope_id();
        let class = classify_type(&ty, self.typed, &mut HashSet::new());
        let id = self.bindings.len();
        self.bindings.push(BindingRecord {
            name: name.to_owned(),
            ty: ty.clone(),
            class,
            mutable,
            state: ValueState::Available,
            declaration_span: span,
            scope_id,
            is_parameter: false,
            borrow_origin: None,
            moved_places: HashSet::new(),
            maybe_moved_places: HashSet::new(),
        });
        if let Some(scope) = self.scopes.last_mut() {
            scope.bindings.push(id);
            let _ = scope.names.insert(name.to_owned(), id);
        }
        self.record_event(OwnershipEventKind::Declare, name.to_owned(), ty, span);
        id
    }

    fn push_scope(&mut self) {
        let id = self.next_scope_id;
        self.next_scope_id += 1;
        self.scopes.push(ScopeFrame {
            id,
            bindings: Vec::new(),
            names: HashMap::new(),
            defers: Vec::new(),
        });
    }

    fn pop_scope(&mut self, end: usize) {
        self.expire_loans(end);
        let Some(scope) = self.scopes.pop() else {
            return;
        };
        let mut order = 0usize;
        for deferred in scope.defers.iter().rev() {
            self.exit_actions.push(ExitAction {
                scope_id: scope.id,
                order,
                kind: ExitActionKind::Defer,
                name: deferred.text.clone(),
                ty: Type::Unit,
                span: deferred.span,
                conditional: false,
            });
            order += 1;
        }
        for binding_id in scope.bindings.iter().rev().copied() {
            let binding = &self.bindings[binding_id];
            if !type_needs_drop(&binding.ty, self.typed, &mut HashSet::new())
                || binding.state == ValueState::Moved
            {
                continue;
            }
            let conditional = binding.state == ValueState::MaybeMoved
                || !binding.moved_places.is_empty()
                || !binding.maybe_moved_places.is_empty();
            self.exit_actions.push(ExitAction {
                scope_id: scope.id,
                order,
                kind: ExitActionKind::Drop,
                name: binding.name.clone(),
                ty: binding.ty.clone(),
                span: binding.declaration_span,
                conditional,
            });
            self.events.push(OwnershipEvent {
                kind: OwnershipEventKind::Drop,
                place: binding.name.clone(),
                ty: binding.ty.clone(),
                span: binding.declaration_span,
                scope_id: scope.id,
            });
            order += 1;
        }
        let scope_bindings = scope.bindings.into_iter().collect::<HashSet<_>>();
        for loan in &mut self.loans {
            if loan.borrow_scope_id == scope.id
                || scope_bindings.contains(&loan.owner_binding)
                || loan.end <= end
            {
                loan.active = false;
            }
        }
    }

    fn current_scope_id(&self) -> usize {
        self.scopes.last().map_or(0, |scope| scope.id)
    }

    fn record_event(&mut self, kind: OwnershipEventKind, place: String, ty: Type, span: Span) {
        self.events.push(OwnershipEvent {
            kind,
            place,
            ty,
            span,
            scope_id: self.current_scope_id(),
        });
    }

    fn lookup_place_binding(&self, place: &str) -> Option<usize> {
        let root = place_root(place);
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.names.get(root).copied())
    }

    fn expression_type(&self, node: &SyntaxNode) -> Type {
        self.expression_types
            .get(&(node.span().start(), node.span().end()))
            .cloned()
            .unwrap_or(Type::Unknown)
    }

    fn place_type(&self, binding_id: usize, place: &str) -> Type {
        if place == self.bindings[binding_id].name {
            return self.bindings[binding_id].ty.clone();
        }
        let mut ty = self.bindings[binding_id].ty.clone();
        let suffix = &place[self.bindings[binding_id].name.len()..];
        for segment in suffix.split('.').filter(|segment| !segment.is_empty()) {
            let field_name = segment.split('[').next().unwrap_or(segment);
            let Some((nominal_name, arguments)) = nominal_instance(&ty) else {
                return Type::Unknown;
            };
            let Some(nominal) = self
                .typed
                .nominals
                .iter()
                .find(|candidate| candidate.name == nominal_name)
            else {
                return Type::Unknown;
            };
            let Some(field) = nominal.fields.iter().find(|field| field.name == field_name) else {
                return Type::Unknown;
            };
            let substitutions = nominal
                .generic_parameters
                .iter()
                .cloned()
                .zip(arguments.iter().cloned())
                .collect::<HashMap<_, _>>();
            ty = substitute_type(&field.ty, &substitutions);
        }
        ty
    }

    fn place_class(&self, binding_id: usize, place: &str) -> OwnershipClass {
        classify_type(
            &self.place_type(binding_id, place),
            self.typed,
            &mut HashSet::new(),
        )
    }

    fn place_was_moved(&self, binding_id: usize, place: &str) -> bool {
        self.bindings[binding_id]
            .moved_places
            .iter()
            .any(|moved| places_overlap(moved, place))
            || self.bindings[binding_id]
                .maybe_moved_places
                .iter()
                .any(|moved| places_overlap(moved, place))
    }

    fn borrow_conflict(&self, place: &str, requested: BorrowKind) -> Option<&Loan> {
        self.loans.iter().find(|loan| {
            loan.active
                && places_overlap(&loan.place, place)
                && !(loan.kind == BorrowKind::Shared && requested == BorrowKind::Shared)
        })
    }

    fn active_overlapping_loan(&self, place: &str) -> Option<&Loan> {
        self.loans
            .iter()
            .find(|loan| loan.active && places_overlap(&loan.place, place))
    }

    fn has_active_mutable_loan(&self, place: &str) -> bool {
        self.loans.iter().any(|loan| {
            loan.active && loan.kind == BorrowKind::Mutable && places_overlap(&loan.place, place)
        })
    }

    fn expire_loans(&mut self, offset: usize) {
        for loan in &mut self.loans {
            if loan.active && loan.end <= offset {
                loan.active = false;
            }
        }
    }

    fn function_parameters(&self, name: &str) -> Option<Vec<Type>> {
        self.typed
            .functions
            .iter()
            .find(|function| function.owner.is_none() && function.name == name)
            .map(|function| {
                function
                    .parameters
                    .iter()
                    .map(|parameter| parameter.ty.clone())
                    .collect()
            })
    }

    fn snapshot(&self) -> Vec<FlowSnapshot> {
        self.snapshot_prefix(self.bindings.len())
    }

    fn snapshot_prefix(&self, count: usize) -> Vec<FlowSnapshot> {
        self.bindings
            .iter()
            .take(count)
            .map(|binding| FlowSnapshot {
                state: binding.state,
                moved_places: binding.moved_places.clone(),
                maybe_moved_places: binding.maybe_moved_places.clone(),
            })
            .collect()
    }

    fn restore(&mut self, snapshot: &[FlowSnapshot]) {
        for (binding, state) in self.bindings.iter_mut().zip(snapshot.iter()) {
            binding.state = state.state;
            binding.moved_places = state.moved_places.clone();
            binding.maybe_moved_places = state.maybe_moved_places.clone();
        }
    }

    fn merge_snapshots(&mut self, left: &[FlowSnapshot], right: &[FlowSnapshot]) {
        let count = left.len().min(right.len()).min(self.bindings.len());
        for index in 0..count {
            let left_state = left[index].state;
            let right_state = right[index].state;
            self.bindings[index].state = if left_state == right_state {
                left_state
            } else {
                ValueState::MaybeMoved
            };
            let intersection = left[index]
                .moved_places
                .intersection(&right[index].moved_places)
                .cloned()
                .collect::<HashSet<_>>();
            let union = left[index]
                .moved_places
                .union(&right[index].moved_places)
                .cloned()
                .collect::<HashSet<_>>();
            self.bindings[index].moved_places = intersection;
            self.bindings[index].maybe_moved_places = union
                .difference(&self.bindings[index].moved_places)
                .cloned()
                .chain(left[index].maybe_moved_places.iter().cloned())
                .chain(right[index].maybe_moved_places.iter().cloned())
                .collect();
        }
    }
}

/// Computes structural Copy eligibility. Unknown and generic parameter types are
/// conservatively move-only until concrete substitution or later trait solving proves `Copy`.
#[must_use]
pub fn classify_type(
    ty: &Type,
    typed: &TypeCheckResult,
    visiting: &mut HashSet<String>,
) -> OwnershipClass {
    match ty {
        Type::Unit
        | Type::Never
        | Type::Bool
        | Type::Char
        | Type::Int
        | Type::Float
        | Type::Pointer { .. }
        | Type::Function(_, _) => OwnershipClass::Copy,
        Type::Reference { mutable: false, .. } => OwnershipClass::Copy,
        Type::Reference { mutable: true, .. }
        | Type::String
        | Type::Unknown
        | Type::Error
        | Type::Parameter(_) => OwnershipClass::Move,
        Type::Optional(inner) => classify_type(inner, typed, visiting),
        Type::Tuple(items) => {
            if items
                .iter()
                .all(|item| classify_type(item, typed, visiting) == OwnershipClass::Copy)
            {
                OwnershipClass::Copy
            } else {
                OwnershipClass::Move
            }
        }
        Type::Named(name, arguments) => {
            if matches!(name.as_str(), "Box" | "Shared" | "Weak" | "List" | "Map" | "Set") {
                return OwnershipClass::Move;
            }
            let visit_key = ty.to_string();
            if !visiting.insert(visit_key.clone()) {
                return OwnershipClass::Move;
            }
            let result = typed
                .nominals
                .iter()
                .find(|nominal| nominal.name == *name)
                .map_or(OwnershipClass::Move, |nominal| {
                    let substitutions = nominal
                        .generic_parameters
                        .iter()
                        .cloned()
                        .zip(arguments.iter().cloned())
                        .collect::<HashMap<_, _>>();
                    let fields_copy = nominal.fields.iter().all(|field| {
                        let field_type = substitute_type(&field.ty, &substitutions);
                        classify_type(&field_type, typed, visiting) == OwnershipClass::Copy
                    });
                    let variants_copy = nominal.variants.iter().all(|variant| {
                        variant.payload.iter().all(|payload| {
                            let payload_type = substitute_type(payload, &substitutions);
                            classify_type(&payload_type, typed, visiting) == OwnershipClass::Copy
                        })
                    });
                    if fields_copy && variants_copy {
                        OwnershipClass::Copy
                    } else {
                        OwnershipClass::Move
                    }
                });
            let _ = visiting.remove(&visit_key);
            result
        }
    }
}

/// Returns whether a value needs an executable scope-exit drop action.
/// Move-only references remain non-dropping even though they cannot be copied.
#[must_use]
pub fn type_needs_drop(
    ty: &Type,
    typed: &TypeCheckResult,
    visiting: &mut HashSet<String>,
) -> bool {
    match ty {
        Type::Unit
        | Type::Never
        | Type::Bool
        | Type::Char
        | Type::Int
        | Type::Float
        | Type::Reference { .. }
        | Type::Pointer { .. }
        | Type::Function(_, _) => false,
        Type::String | Type::Unknown | Type::Error | Type::Parameter(_) => true,
        Type::Optional(inner) => type_needs_drop(inner, typed, visiting),
        Type::Tuple(items) => items
            .iter()
            .any(|item| type_needs_drop(item, typed, visiting)),
        Type::Named(name, arguments) => {
            if matches!(name.as_str(), "Box" | "Shared" | "Weak" | "List" | "Map" | "Set") {
                return true;
            }
            let visit_key = ty.to_string();
            if !visiting.insert(visit_key.clone()) {
                return true;
            }
            let result = typed
                .nominals
                .iter()
                .find(|nominal| nominal.name == *name)
                .map_or(true, |nominal| {
                    let substitutions = nominal
                        .generic_parameters
                        .iter()
                        .cloned()
                        .zip(arguments.iter().cloned())
                        .collect::<HashMap<_, _>>();
                    nominal.fields.iter().any(|field| {
                        let field_type = substitute_type(&field.ty, &substitutions);
                        type_needs_drop(&field_type, typed, visiting)
                    }) || nominal.variants.iter().any(|variant| {
                        variant.payload.iter().any(|payload| {
                            let payload_type = substitute_type(payload, &substitutions);
                            type_needs_drop(&payload_type, typed, visiting)
                        })
                    })
                });
            let _ = visiting.remove(&visit_key);
            result
        }
    }
}

fn substitute_type(ty: &Type, substitutions: &HashMap<String, Type>) -> Type {
    match ty {
        Type::Parameter(name) => substitutions
            .get(name)
            .cloned()
            .unwrap_or_else(|| ty.clone()),
        Type::Named(name, arguments) => Type::Named(
            name.clone(),
            arguments
                .iter()
                .map(|argument| substitute_type(argument, substitutions))
                .collect(),
        ),
        Type::Optional(inner) => {
            Type::Optional(Box::new(substitute_type(inner, substitutions)))
        }
        Type::Reference { mutable, inner } => Type::Reference {
            mutable: *mutable,
            inner: Box::new(substitute_type(inner, substitutions)),
        },
        Type::Pointer { mutable, inner } => Type::Pointer {
            mutable: *mutable,
            inner: Box::new(substitute_type(inner, substitutions)),
        },
        Type::Tuple(items) => Type::Tuple(
            items
                .iter()
                .map(|item| substitute_type(item, substitutions))
                .collect(),
        ),
        Type::Function(parameters, result) => Type::Function(
            parameters
                .iter()
                .map(|parameter| substitute_type(parameter, substitutions))
                .collect(),
            Box::new(substitute_type(result, substitutions)),
        ),
        _ => ty.clone(),
    }
}

fn contains_reference(ty: &Type) -> bool {
    match ty {
        Type::Reference { .. } => true,
        Type::Optional(inner) | Type::Pointer { inner, .. } => contains_reference(inner),
        Type::Named(_, arguments) | Type::Tuple(arguments) => {
            arguments.iter().any(contains_reference)
        }
        Type::Function(parameters, result) => {
            parameters.iter().any(contains_reference) || contains_reference(result)
        }
        _ => false,
    }
}

fn nominal_instance(ty: &Type) -> Option<(&str, &[Type])> {
    match ty {
        Type::Named(name, arguments) => Some((name, arguments)),
        Type::Reference { inner, .. } | Type::Optional(inner) => nominal_instance(inner),
        _ => None,
    }
}

fn is_observer_builtin(name: &str) -> bool {
    matches!(name, "print" | "println" | "assert" | "dbg")
}

fn callable_name(node: &SyntaxNode, source: &SourceFile) -> Option<String> {
    match node.kind() {
        SyntaxKind::NameExpression => Some(significant_text(node, source)),
        SyntaxKind::MemberExpression => significant_direct_tokens(node)
            .into_iter()
            .rev()
            .find(|token| {
                matches!(
                    token.kind(),
                    TokenKind::Identifier | TokenKind::Keyword(Keyword::SelfValue)
                )
            })
            .and_then(|token| token.text(source).map(str::to_owned)),
        _ => None,
    }
}

fn borrow_expression(
    node: &SyntaxNode,
    source: &SourceFile,
) -> Option<(BorrowKind, String, Span)> {
    if node.kind() != SyntaxKind::PrefixExpression {
        return None;
    }
    let tokens = significant_direct_tokens(node);
    if !tokens
        .first()
        .is_some_and(|token| token.kind() == TokenKind::Ampersand)
    {
        return None;
    }
    let operand = node.child_nodes().next()?;
    let place = place_text(operand, source)?;
    let kind = if significant_text(node, source).starts_with("&mut") {
        BorrowKind::Mutable
    } else {
        BorrowKind::Shared
    };
    Some((kind, place, node.span()))
}

fn place_text(node: &SyntaxNode, source: &SourceFile) -> Option<String> {
    match node.kind() {
        SyntaxKind::NameExpression => {
            let text = significant_text(node, source);
            (!text.is_empty() && !text.contains("::") && !text.starts_with('.')).then_some(text)
        }
        SyntaxKind::MemberExpression => {
            let base = node.child_nodes().next().and_then(|child| place_text(child, source))?;
            let member = significant_direct_tokens(node)
                .into_iter()
                .rev()
                .find(|token| {
                    matches!(
                        token.kind(),
                        TokenKind::Identifier | TokenKind::Keyword(Keyword::SelfValue)
                    )
                })
                .and_then(|token| token.text(source).map(str::to_owned))?;
            Some(format!("{base}.{member}"))
        }
        SyntaxKind::IndexExpression => {
            let base = node.child_nodes().next().and_then(|child| place_text(child, source))?;
            Some(format!("{base}[]"))
        }
        SyntaxKind::ParenthesizedExpression => node
            .child_nodes()
            .next()
            .and_then(|child| place_text(child, source)),
        _ => None,
    }
}

fn place_root(place: &str) -> &str {
    place
        .split(['.', '['])
        .next()
        .unwrap_or(place)
}

fn places_overlap(left: &str, right: &str) -> bool {
    left == right
        || left
            .strip_prefix(right)
            .is_some_and(|suffix| suffix.starts_with('.') || suffix.starts_with('['))
        || right
            .strip_prefix(left)
            .is_some_and(|suffix| suffix.starts_with('.') || suffix.starts_with('['))
}

fn collect_name_uses(
    node: &SyntaxNode,
    source: &SourceFile,
    uses: &mut HashMap<String, Vec<usize>>,
) {
    if node.kind() == SyntaxKind::NameExpression {
        let name = significant_text(node, source);
        if !name.is_empty() && !name.contains("::") && !name.starts_with('.') {
            uses.entry(name).or_default().push(node.span().end());
        }
    }
    for child in node.child_nodes() {
        collect_name_uses(child, source, uses);
    }
}

fn first_identifier(node: &SyntaxNode, source: &SourceFile) -> Option<(String, Span)> {
    node.descendant_tokens()
        .into_iter()
        .find(|token| token.kind() == TokenKind::Identifier)
        .and_then(|token| token.text(source).map(|text| (text.to_owned(), token.span())))
}

fn significant_direct_tokens(node: &SyntaxNode) -> Vec<SyntaxToken> {
    node.child_tokens()
        .filter(|token| !token.kind().is_trivia())
        .collect()
}

fn significant_text(node: &SyntaxNode, source: &SourceFile) -> String {
    node.descendant_tokens()
        .iter()
        .filter(|token| !token.kind().is_trivia())
        .filter_map(|token| token.text(source))
        .collect::<Vec<_>>()
        .join("")
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

impl fmt::Display for OwnershipClass {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use nivra_parser::parse;
    use nivra_sema::analyze as analyze_semantics;
    use nivra_source::SourceManager;
    use nivra_types::check as check_types;

    use super::{analyze, ExitActionKind, OwnershipClass, OwnershipEventKind};

    fn ownership(source_text: &str) -> super::OwnershipResult {
        let mut sources = SourceManager::new();
        let id = sources
            .add_virtual("test.nva", source_text)
            .unwrap_or_else(|error| panic!("{error}"));
        let source = sources.get(id).unwrap_or_else(|| panic!("missing source"));
        let parsed = parse(source);
        assert!(!parsed.has_errors(), "{:?}", parsed.diagnostics);
        let semantic = analyze_semantics(source, &parsed.root);
        assert!(!semantic.has_errors(), "{:?}", semantic.diagnostics);
        let typed = check_types(source, &parsed.root, &semantic);
        assert!(!typed.has_errors(), "{:?}", typed.diagnostics);
        analyze(source, &parsed.root, &typed)
    }

    #[test]
    fn classifies_scalars_as_copy_and_strings_as_move() {
        let result = ownership(
            "module demo\nfn main() { let count = 1\n let text = \"hello\"\n print(count)\n print(text)\n }\n",
        );
        assert!(result
            .bindings
            .iter()
            .any(|binding| binding.name == "count" && binding.class == OwnershipClass::Copy));
        assert!(result
            .bindings
            .iter()
            .any(|binding| binding.name == "text" && binding.class == OwnershipClass::Move));
    }

    #[test]
    fn rejects_use_after_move() {
        let result = ownership(
            "module demo\nfn consume(value: String) {}\nfn main() { let text = \"hello\"\n consume(text)\n print(text)\n }\n",
        );
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "OWN001"));
    }

    #[test]
    fn accepts_copy_after_transfer() {
        let result = ownership(
            "module demo\nfn consume(value: Int) {}\nfn main() { let count = 1\n consume(count)\n print(count)\n }\n",
        );
        assert!(!result.has_errors());
        assert!(result
            .events
            .iter()
            .any(|event| event.kind == OwnershipEventKind::Copy && event.place == "count"));
    }

    #[test]
    fn rejects_shared_then_mutable_borrow_conflict() {
        let result = ownership(
            "module demo\nrecord Note { text: String }\nfn main() { var note = Note(text: \"a\")\n let first = &note\n let second = &mut note\n print(first)\n }\n",
        );
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "BOR001"));
    }

    #[test]
    fn last_use_ends_local_borrow_without_lifetime_syntax() {
        let result = ownership(
            "module demo\nrecord Note { text: String }\nfn consume(value: Note) {}\nfn main() { var note = Note(text: \"a\")\n let view = &note\n print(view)\n consume(note)\n }\n",
        );
        assert!(!result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "OWN002"));
    }

    #[test]
    fn mutable_borrow_requires_var() {
        let result = ownership(
            "module demo\nrecord Note { text: String }\nfn main() { let note = Note(text: \"a\")\n let view = &mut note\n print(view)\n }\n",
        );
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "BOR003"));
    }

    #[test]
    fn moved_var_can_be_reinitialized() {
        let result = ownership(
            "module demo\nfn consume(value: String) {}\nfn main() { var text = \"a\"\n consume(text)\n text = \"b\"\n print(text)\n }\n",
        );
        assert!(!result.has_errors());
        assert!(result.events.iter().any(|event| {
            event.kind == OwnershipEventKind::Reinitialize && event.place == "text"
        }));
    }

    #[test]
    fn plans_defers_before_reverse_local_drops() {
        let result = ownership(
            "module demo\nfn clean(value: &String) {}\nfn main() { let first = \"a\"\n let second = \"b\"\n defer clean(&first)\n print(second)\n }\n",
        );
        let function_actions = result
            .exit_actions
            .iter()
            .filter(|action| action.kind == ExitActionKind::Defer || action.kind == ExitActionKind::Drop)
            .collect::<Vec<_>>();
        assert!(function_actions.iter().any(|action| action.kind == ExitActionKind::Defer));
        let drop_names = function_actions
            .iter()
            .filter(|action| action.kind == ExitActionKind::Drop)
            .map(|action| action.name.as_str())
            .collect::<Vec<_>>();
        assert!(drop_names.windows(2).any(|pair| pair == ["second", "first"]));
    }

    #[test]
    fn rejects_borrow_across_await() {
        let result = ownership(
            "module demo\nasync fn pause() -> Int { 1 }\nasync fn main() { let text = \"a\"\n let view = &text\n await pause()\n print(view)\n }\n",
        );
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "BOR009"));
    }
    #[test]
    fn explicit_move_invalidates_even_outside_a_consuming_call() {
        let result = ownership(
            "module demo\nfn main() { let source = \"owned\"\n let target = move source\n print(target)\n print(source)\n }\n",
        );
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "OWN001"));
    }

    #[test]
    fn both_moving_branches_join_as_moved_not_maybe_moved() {
        let result = ownership(
            "module demo\nfn consume(value: String) {}\nfn main(flag: Bool) { let text = \"owned\"\n if flag { consume(text) } else { consume(text) }\n print(text)\n }\n",
        );
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "OWN001"));
        assert!(!result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "OWN007"));
    }

    #[test]
    fn move_on_only_one_branch_is_maybe_moved() {
        let result = ownership(
            "module demo\nfn consume(value: String) {}\nfn main(flag: Bool) { let text = \"owned\"\n if flag { consume(text) }\n print(text)\n }\n",
        );
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "OWN007"));
    }

    #[test]
    fn inner_scope_borrow_of_outer_owner_ends_with_reference_scope() {
        let result = ownership(
            "module demo\nrecord Note { text: String }\nfn consume(value: Note) {}\nfn main() { let note = Note(text: \"a\")\n { let view = &note\n print(view) }\n consume(note)\n }\n",
        );
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
    }

    #[test]
    fn rejects_complete_use_after_partial_field_move() {
        let result = ownership(
            "module demo\nrecord User { name: String, age: Int }\nfn consume(value: String) {}\nfn main() { let user = User(name: \"Nivra\", age: 1)\n consume(user.name)\n print(user)\n }\n",
        );
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "OWN006"));
    }


    #[test]
    fn concrete_generic_copy_fields_make_the_nominal_copy() {
        let result = ownership(
            "module demo\nrecord Holder<T> { value: T }\nfn consume(value: Holder<Int>) {}\nfn main() { let holder = Holder<Int> { value: 7 }\n consume(holder)\n print(holder.value)\n }\n",
        );
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert!(result.bindings.iter().any(|binding| {
            binding.name == "holder" && binding.class == OwnershipClass::Copy
        }));
    }

    #[test]
    fn concrete_generic_move_fields_make_the_nominal_move() {
        let result = ownership(
            "module demo\nrecord Holder<T> { value: T }\nfn consume(value: Holder<String>) {}\nfn main() { let holder = Holder<String> { value: \"owned\" }\n consume(holder)\n print(holder.value)\n }\n",
        );
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "OWN001"));
    }

    #[test]
    fn explicit_generic_call_still_consumes_move_arguments() {
        let result = ownership(
            "module demo\nfn identity<T>(value: T) -> T { value }\nfn main() { let text = \"owned\"\n let other = identity<String>(text)\n print(other)\n print(text)\n }\n",
        );
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "OWN001"));
    }

    #[test]
    fn mutable_reference_is_move_only_but_has_no_drop_action() {
        let result = ownership(
            "module demo\nfn main() { var text = \"owned\"\n let view = &mut text\n print(view)\n }\n",
        );
        assert!(result.bindings.iter().any(|binding| {
            binding.name == "view" && binding.class == OwnershipClass::Move
        }));
        assert!(!result.exit_actions.iter().any(|action| {
            action.kind == ExitActionKind::Drop && action.name == "view"
        }));
    }

    #[test]
    fn deferred_borrow_keeps_owner_live_until_scope_exit() {
        let result = ownership(
            "module demo\nfn clean(value: &String) {}\nfn consume(value: String) {}\nfn main() { let text = \"owned\"\n defer clean(&text)\n consume(text)\n }\n",
        );
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "OWN002"));
    }

    #[test]
    fn rejects_returning_a_local_borrow_through_an_alias() {
        let result = ownership(
            "module demo\nfn invalid(source: &String) -> &String { let local = \"temporary\"\n let view = &local\n return view\n }\n",
        );
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "BOR008"));
    }

    #[test]
    fn rejects_borrowed_enum_variant_payloads() {
        let result = ownership("module demo\nenum View { text(&String) }\n");
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "BOR006"));
    }


    #[test]
    fn rejects_tail_return_of_a_local_borrow_alias() {
        let result = ownership(
            "module demo\nfn invalid(source: &String) -> &String { let local = \"temporary\"\n let view = &local\n view\n }\n",
        );
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "BOR008"));
    }

}
