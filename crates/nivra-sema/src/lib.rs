//! Semantic indexing and first-pass name resolution for Nivra Edition 2026.
//!
//! D5 intentionally stops before type checking. It lowers the lossless CST into a
//! semantic index of modules, scopes, symbols, and value-name resolutions. The pass
//! is deterministic, recovery-friendly, and keeps every diagnostic attached to the
//! original source span.

use std::collections::{HashMap, HashSet};
use std::fmt::{self, Write as _};

use nivra_diagnostics::Diagnostic;
use nivra_lexer::{Keyword, TokenKind};
use nivra_parser::ParseResult;
use nivra_source::{SourceFile, Span};
use nivra_syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

/// Stable symbol-table identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SymbolId(u32);

impl SymbolId {
    /// Returns the compact numeric representation used by reports.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// Stable lexical-scope identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ScopeId(u32);

impl ScopeId {
    /// Returns the compact numeric representation used by reports.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// Independent lookup namespaces available before full type checking.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Namespace {
    /// Runtime values, functions, constants, locals, and parameters.
    Value,
    /// Nominal types, aliases, traits, and generic parameters.
    Type,
}

impl Namespace {
    /// Returns the stable report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Value => "value",
            Self::Type => "type",
        }
    }
}

/// Source visibility recorded by the module index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Visibility {
    /// Visible only inside the declaring module.
    Private,
    /// Exported from the declaring module.
    Public,
}

impl Visibility {
    /// Returns the stable report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Public => "public",
        }
    }
}

/// Semantic category assigned to an indexed name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Builtin,
    Import,
    Constant,
    TypeAlias,
    Newtype,
    Record,
    Struct,
    Enum,
    Trait,
    Function,
    ExternFunction,
    Method,
    GenericParameter,
    Parameter,
    Local,
    Field,
    EnumVariant,
    TaskGroup,
}

impl SymbolKind {
    /// Returns the stable report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::Import => "import",
            Self::Constant => "constant",
            Self::TypeAlias => "type_alias",
            Self::Newtype => "newtype",
            Self::Record => "record",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Function => "function",
            Self::ExternFunction => "extern_function",
            Self::Method => "method",
            Self::GenericParameter => "generic_parameter",
            Self::Parameter => "parameter",
            Self::Local => "local",
            Self::Field => "field",
            Self::EnumVariant => "enum_variant",
            Self::TaskGroup => "task_group",
        }
    }
}

/// Why a symbol exists in the semantic index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SymbolOrigin {
    Prelude,
    Import,
    Source,
}

impl SymbolOrigin {
    /// Returns the stable report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Prelude => "prelude",
            Self::Import => "import",
            Self::Source => "source",
        }
    }
}

/// One indexed declaration or binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Symbol {
    pub id: SymbolId,
    pub name: String,
    pub kind: SymbolKind,
    pub namespace: Namespace,
    pub visibility: Visibility,
    pub origin: SymbolOrigin,
    pub scope: ScopeId,
    pub span: Span,
}

/// Lexical scope category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScopeKind {
    Prelude,
    Module,
    TypeBody,
    Function,
    Block,
    Loop,
    MatchArm,
    Closure,
    TaskGroup,
}

impl ScopeKind {
    /// Returns the stable report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Prelude => "prelude",
            Self::Module => "module",
            Self::TypeBody => "type_body",
            Self::Function => "function",
            Self::Block => "block",
            Self::Loop => "loop",
            Self::MatchArm => "match_arm",
            Self::Closure => "closure",
            Self::TaskGroup => "task_group",
        }
    }
}

/// One lexical scope and the symbols declared directly inside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
    pub id: ScopeId,
    pub parent: Option<ScopeId>,
    pub kind: ScopeKind,
    pub span: Span,
    pub symbols: Vec<SymbolId>,
}

/// Result of resolving one source name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolution {
    pub name: String,
    pub namespace: Namespace,
    pub span: Span,
    pub scope: ScopeId,
    pub symbol: Option<SymbolId>,
}

/// Indexed source module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleIndex {
    pub name: String,
    pub root_scope: ScopeId,
    pub span: Span,
}

/// Complete D5 semantic output.
#[derive(Clone, Debug)]
pub struct SemanticResult {
    pub module: ModuleIndex,
    pub scopes: Vec<Scope>,
    pub symbols: Vec<Symbol>,
    pub resolutions: Vec<Resolution>,
    pub diagnostics: Vec<Diagnostic>,
}

impl SemanticResult {
    /// Returns whether semantic analysis produced an error.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }

    /// Returns source and import symbols, excluding compiler prelude entries.
    pub fn user_symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols
            .iter()
            .filter(|symbol| symbol.origin != SymbolOrigin::Prelude)
    }

    /// Counts successful name resolutions.
    #[must_use]
    pub fn resolved_name_count(&self) -> usize {
        self.resolutions
            .iter()
            .filter(|resolution| resolution.symbol.is_some())
            .count()
    }

    /// Counts unresolved value names tracked by D5.
    #[must_use]
    pub fn unresolved_name_count(&self) -> usize {
        self.resolutions
            .iter()
            .filter(|resolution| resolution.symbol.is_none())
            .count()
    }

    /// Looks up a symbol by its stable identifier.
    #[must_use]
    pub fn symbol(&self, id: SymbolId) -> Option<&Symbol> {
        self.symbols.get(id.raw() as usize)
    }

    /// Produces a deterministic symbol-table report.
    #[must_use]
    pub fn symbol_report(&self, include_prelude: bool) -> String {
        let mut output = String::new();
        for symbol in &self.symbols {
            if !include_prelude && symbol.origin == SymbolOrigin::Prelude {
                continue;
            }
            let _ = writeln!(
                output,
                "#{:<3} {:<18} {:<10} {:<16} {:<8} scope={} {}..{}",
                symbol.id.raw(),
                symbol.name,
                symbol.namespace.as_str(),
                symbol.kind.as_str(),
                symbol.visibility.as_str(),
                symbol.scope.raw(),
                symbol.span.start(),
                symbol.span.end()
            );
        }
        output
    }

    /// Produces a deterministic scope tree.
    #[must_use]
    pub fn scope_report(&self) -> String {
        let mut output = String::new();
        self.write_scope(self.scopes[0].id, 0, &mut output);
        output
    }

    fn write_scope(&self, id: ScopeId, depth: usize, output: &mut String) {
        let Some(scope) = self.scopes.get(id.raw() as usize) else {
            return;
        };
        let _ = writeln!(
            output,
            "{}scope #{} {} {}..{} symbols={}",
            "  ".repeat(depth),
            scope.id.raw(),
            scope.kind.as_str(),
            scope.span.start(),
            scope.span.end(),
            scope.symbols.len()
        );
        for child in self
            .scopes
            .iter()
            .filter(|candidate| candidate.parent == Some(id))
        {
            self.write_scope(child.id, depth + 1, output);
        }
    }
}

/// Runs the D5 semantic pass over an already parsed source file.
#[must_use]
pub fn analyze(source: &SourceFile, root: &SyntaxNode) -> SemanticResult {
    Analyzer::new(source, root.span()).run(root)
}

/// Runs semantic analysis only when parsing succeeded.
#[must_use]
pub fn analyze_parse(source: &SourceFile, parsed: &ParseResult) -> Option<SemanticResult> {
    (!parsed.has_errors()).then(|| analyze(source, &parsed.root))
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct BindingKey {
    scope: ScopeId,
    namespace: Namespace,
    name: String,
}

struct Analyzer<'a> {
    source: &'a SourceFile,
    module_name: String,
    module_span: Span,
    module_declarations: usize,
    scopes: Vec<Scope>,
    symbols: Vec<Symbol>,
    bindings: HashMap<BindingKey, SymbolId>,
    resolutions: Vec<Resolution>,
    diagnostics: Vec<Diagnostic>,
    prelude_scope: ScopeId,
    module_scope: ScopeId,
}

impl<'a> Analyzer<'a> {
    fn new(source: &'a SourceFile, root_span: Span) -> Self {
        let prelude_scope = ScopeId(0);
        let module_scope = ScopeId(1);
        let scopes = vec![
            Scope {
                id: prelude_scope,
                parent: None,
                kind: ScopeKind::Prelude,
                span: Span::empty(source.id(), 0),
                symbols: Vec::new(),
            },
            Scope {
                id: module_scope,
                parent: Some(prelude_scope),
                kind: ScopeKind::Module,
                span: root_span,
                symbols: Vec::new(),
            },
        ];
        let mut analyzer = Self {
            source,
            module_name: source
                .path()
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("anonymous")
                .to_owned(),
            module_span: root_span,
            module_declarations: 0,
            scopes,
            symbols: Vec::new(),
            bindings: HashMap::new(),
            resolutions: Vec::new(),
            diagnostics: Vec::new(),
            prelude_scope,
            module_scope,
        };
        analyzer.install_prelude();
        analyzer
    }

    fn run(mut self, root: &SyntaxNode) -> SemanticResult {
        self.index_module(root);
        self.analyze_declarations(root, self.module_scope, false);
        SemanticResult {
            module: ModuleIndex {
                name: self.module_name,
                root_scope: self.module_scope,
                span: self.module_span,
            },
            scopes: self.scopes,
            symbols: self.symbols,
            resolutions: self.resolutions,
            diagnostics: self.diagnostics,
        }
    }

    fn install_prelude(&mut self) {
        for name in [
            "Bool", "Char", "Float", "F32", "F64", "I8", "I16", "I32", "I64", "Int", "List", "Map",
            "Never", "Option", "Path", "Result", "Set", "Shared", "String", "Task", "U8", "U16",
            "U32", "U64", "Unit", "Usize", "Weak",
        ] {
            self.define_unchecked(
                self.prelude_scope,
                name,
                SymbolKind::Builtin,
                Namespace::Type,
                Visibility::Public,
                SymbolOrigin::Prelude,
                Span::empty(self.source.id(), 0),
            );
        }
        for name in [
            "assert", "dbg", "err", "http", "ok", "panic", "print", "println", "todo",
        ] {
            self.define_unchecked(
                self.prelude_scope,
                name,
                SymbolKind::Builtin,
                Namespace::Value,
                Visibility::Public,
                SymbolOrigin::Prelude,
                Span::empty(self.source.id(), 0),
            );
        }
    }

    fn index_module(&mut self, root: &SyntaxNode) {
        for declaration in root.child_nodes() {
            match declaration.kind() {
                SyntaxKind::ModuleDeclaration => self.index_module_declaration(declaration),
                SyntaxKind::UseDeclaration => self.index_import(declaration),
                SyntaxKind::ExternBlock => self.index_extern_functions(declaration),
                kind => {
                    if let Some((symbol_kind, namespace)) = declaration_symbol_kind(kind) {
                        if let Some((name, span)) = first_direct_name(declaration, self.source) {
                            self.define(
                                self.module_scope,
                                &name,
                                symbol_kind,
                                namespace,
                                visibility(declaration),
                                SymbolOrigin::Source,
                                span,
                            );
                        }
                    }
                }
            }
        }
    }

    fn index_module_declaration(&mut self, node: &SyntaxNode) {
        self.module_declarations += 1;
        let path = path_after_keyword(node, self.source, Keyword::Module);
        if let Some((name, span)) = path {
            if self.module_declarations == 1 {
                self.module_name = name;
                self.module_span = span;
            } else {
                self.diagnostics.push(
                    Diagnostic::error("SEM004", "multiple module declarations")
                        .with_primary(span, "a source file may declare exactly one module")
                        .with_secondary(self.module_span, "the module was first declared here")
                        .with_help("remove the extra `module` declaration"),
                );
            }
        }
    }

    fn index_import(&mut self, node: &SyntaxNode) {
        for (name, span) in import_bindings(node, self.source) {
            self.define(
                self.module_scope,
                &name,
                SymbolKind::Import,
                Namespace::Value,
                Visibility::Private,
                SymbolOrigin::Import,
                span,
            );
            if starts_like_type(&name) {
                self.define(
                    self.module_scope,
                    &name,
                    SymbolKind::Import,
                    Namespace::Type,
                    Visibility::Private,
                    SymbolOrigin::Import,
                    span,
                );
            }
        }
    }

    fn index_extern_functions(&mut self, node: &SyntaxNode) {
        for function in node
            .child_nodes()
            .filter(|child| child.kind() == SyntaxKind::ExternFunction)
        {
            if let Some((name, span)) = first_direct_name(function, self.source) {
                self.define(
                    self.module_scope,
                    &name,
                    SymbolKind::ExternFunction,
                    Namespace::Value,
                    visibility(function),
                    SymbolOrigin::Source,
                    span,
                );
            }
        }
    }

    fn analyze_declarations(&mut self, parent: &SyntaxNode, scope: ScopeId, methods: bool) {
        for node in parent.child_nodes() {
            match node.kind() {
                SyntaxKind::FunctionDeclaration => self.analyze_function(node, scope, methods),
                SyntaxKind::ExternBlock => {
                    for function in node.child_nodes() {
                        if function.kind() == SyntaxKind::ExternFunction {
                            self.analyze_function(function, scope, false);
                        }
                    }
                }
                SyntaxKind::TraitDeclaration | SyntaxKind::ImplDeclaration => {
                    let type_scope = self.new_scope(scope, ScopeKind::TypeBody, node.span());
                    self.index_methods(node, type_scope);
                    self.analyze_declarations(node, type_scope, true);
                }
                SyntaxKind::RecordDeclaration | SyntaxKind::StructDeclaration => {
                    self.index_fields(node, scope);
                }
                SyntaxKind::EnumDeclaration => self.index_variants(node, scope),
                SyntaxKind::ConstDeclaration
                | SyntaxKind::TypeAliasDeclaration
                | SyntaxKind::NewtypeDeclaration => {
                    for child in node.child_nodes() {
                        self.analyze_node(child, scope);
                    }
                }
                _ => {}
            }
        }
    }

    fn index_methods(&mut self, node: &SyntaxNode, scope: ScopeId) {
        for method in node
            .child_nodes()
            .filter(|child| child.kind() == SyntaxKind::FunctionDeclaration)
        {
            if let Some((name, span)) = first_direct_name(method, self.source) {
                self.define(
                    scope,
                    &name,
                    SymbolKind::Method,
                    Namespace::Value,
                    visibility(method),
                    SymbolOrigin::Source,
                    span,
                );
            }
        }
    }

    fn index_fields(&mut self, node: &SyntaxNode, parent_scope: ScopeId) {
        let scope = self.new_scope(parent_scope, ScopeKind::TypeBody, node.span());
        if let Some(fields) = node
            .child_nodes()
            .find(|child| child.kind() == SyntaxKind::FieldList)
        {
            for field in fields
                .child_nodes()
                .filter(|child| child.kind() == SyntaxKind::Field)
            {
                if let Some((name, span)) = first_direct_name(field, self.source) {
                    self.define(
                        scope,
                        &name,
                        SymbolKind::Field,
                        Namespace::Value,
                        visibility(field),
                        SymbolOrigin::Source,
                        span,
                    );
                }
            }
        }
    }

    fn index_variants(&mut self, node: &SyntaxNode, parent_scope: ScopeId) {
        let scope = self.new_scope(parent_scope, ScopeKind::TypeBody, node.span());
        for variant in node
            .child_nodes()
            .filter(|child| child.kind() == SyntaxKind::EnumVariant)
        {
            if let Some((name, span)) = first_direct_name(variant, self.source) {
                self.define(
                    scope,
                    &name,
                    SymbolKind::EnumVariant,
                    Namespace::Value,
                    Visibility::Public,
                    SymbolOrigin::Source,
                    span,
                );
            }
        }
    }

    fn analyze_function(&mut self, node: &SyntaxNode, parent_scope: ScopeId, _method: bool) {
        let function_scope = self.new_scope(parent_scope, ScopeKind::Function, node.span());
        self.define_generic_parameters(node, function_scope);

        if let Some(parameters) = node
            .child_nodes()
            .find(|child| child.kind() == SyntaxKind::ParameterList)
        {
            for parameter in parameters
                .child_nodes()
                .filter(|child| child.kind() == SyntaxKind::Parameter)
            {
                if let Some((name, span)) = parameter_name(parameter, self.source) {
                    self.define(
                        function_scope,
                        &name,
                        SymbolKind::Parameter,
                        Namespace::Value,
                        Visibility::Private,
                        SymbolOrigin::Source,
                        span,
                    );
                }
                for child in parameter.child_nodes() {
                    if child.kind() == SyntaxKind::TypeReference {
                        self.resolve_type_reference(child, function_scope);
                    }
                }
            }
        }

        for type_reference in node
            .child_nodes()
            .filter(|child| child.kind() == SyntaxKind::TypeReference)
        {
            self.resolve_type_reference(type_reference, function_scope);
        }

        if let Some(block) = node
            .child_nodes()
            .find(|child| child.kind() == SyntaxKind::Block)
        {
            self.analyze_block(block, function_scope, false);
        }
    }

    fn define_generic_parameters(&mut self, node: &SyntaxNode, scope: ScopeId) {
        let Some(generics) = node
            .child_nodes()
            .find(|child| child.kind() == SyntaxKind::GenericParameterList)
        else {
            return;
        };
        let tokens = significant_direct_tokens(generics);
        let mut expect_name = false;
        let mut seen = HashSet::new();
        for token in tokens {
            match token.kind() {
                TokenKind::Less | TokenKind::Comma => expect_name = true,
                TokenKind::Identifier if expect_name => {
                    if let Some(name) = token.text(self.source) {
                        // D8 owns duplicate generic-parameter diagnostics through GEN005.
                        // Keep the first symbol visible to name resolution, but do not emit
                        // the older generic SEM005 diagnostic and stop type checking early.
                        if seen.insert(name.to_owned()) {
                            self.define(
                                scope,
                                name,
                                SymbolKind::GenericParameter,
                                Namespace::Type,
                                Visibility::Private,
                                SymbolOrigin::Source,
                                token.span(),
                            );
                        }
                    }
                    expect_name = false;
                }
                TokenKind::Greater => break,
                _ => {}
            }
        }
    }

    fn analyze_block(&mut self, block: &SyntaxNode, parent_scope: ScopeId, nested: bool) {
        let scope = if nested {
            self.new_scope(parent_scope, ScopeKind::Block, block.span())
        } else {
            parent_scope
        };

        for statement in block.child_nodes() {
            match statement.kind() {
                SyntaxKind::LetStatement | SyntaxKind::VarStatement => {
                    self.analyze_binding(statement, scope);
                }
                _ => self.analyze_node(statement, scope),
            }
        }
    }

    fn analyze_binding(&mut self, node: &SyntaxNode, scope: ScopeId) {
        for child in node.child_nodes() {
            if child.kind() != SyntaxKind::Pattern && child.kind() != SyntaxKind::TypeReference {
                self.analyze_node(child, scope);
            } else if child.kind() == SyntaxKind::TypeReference {
                self.resolve_type_reference(child, scope);
            }
        }
        if let Some(pattern) = node
            .child_nodes()
            .find(|child| child.kind() == SyntaxKind::Pattern)
        {
            self.define_pattern(pattern, scope, SymbolKind::Local);
        }
    }

    fn analyze_node(&mut self, node: &SyntaxNode, scope: ScopeId) {
        match node.kind() {
            SyntaxKind::NameExpression => self.resolve_name_expression(node, scope),
            SyntaxKind::TypeReference => self.resolve_type_reference(node, scope),
            SyntaxKind::Block => self.analyze_block(node, scope, true),
            SyntaxKind::WhileStatement => self.analyze_while(node, scope),
            SyntaxKind::ForStatement => self.analyze_for(node, scope),
            SyntaxKind::IfExpression => self.analyze_if(node, scope),
            SyntaxKind::MatchExpression => self.analyze_match(node, scope),
            SyntaxKind::ClosureExpression => self.analyze_closure(node, scope),
            SyntaxKind::TaskGroupExpression => self.analyze_task_group(node, scope),
            SyntaxKind::Pattern | SyntaxKind::Parameter | SyntaxKind::Field => {}
            _ => {
                for child in node.child_nodes() {
                    self.analyze_node(child, scope);
                }
            }
        }
    }

    fn analyze_while(&mut self, node: &SyntaxNode, scope: ScopeId) {
        for child in node.child_nodes() {
            if child.kind() == SyntaxKind::Block {
                self.analyze_block(child, scope, true);
            } else {
                self.analyze_node(child, scope);
            }
        }
    }

    fn analyze_for(&mut self, node: &SyntaxNode, scope: ScopeId) {
        let mut pattern = None;
        let mut block = None;
        for child in node.child_nodes() {
            match child.kind() {
                SyntaxKind::Pattern => pattern = Some(child),
                SyntaxKind::Block => block = Some(child),
                _ => self.analyze_node(child, scope),
            }
        }
        let loop_scope = self.new_scope(scope, ScopeKind::Loop, node.span());
        if let Some(pattern) = pattern {
            self.define_pattern(pattern, loop_scope, SymbolKind::Local);
        }
        if let Some(block) = block {
            self.analyze_block(block, loop_scope, false);
        }
    }

    fn analyze_if(&mut self, node: &SyntaxNode, scope: ScopeId) {
        let children = node.child_nodes().collect::<Vec<_>>();
        let pattern = children
            .iter()
            .copied()
            .find(|child| child.kind() == SyntaxKind::Pattern);
        let mut blocks = children
            .iter()
            .copied()
            .filter(|child| child.kind() == SyntaxKind::Block);

        for child in &children {
            if child.kind() != SyntaxKind::Pattern && child.kind() != SyntaxKind::Block {
                self.analyze_node(child, scope);
            }
        }

        if let Some(then_block) = blocks.next() {
            let then_scope = self.new_scope(scope, ScopeKind::Block, then_block.span());
            if let Some(pattern) = pattern {
                self.define_pattern(pattern, then_scope, SymbolKind::Local);
            }
            self.analyze_block(then_block, then_scope, false);
        }
        for else_block in blocks {
            self.analyze_block(else_block, scope, true);
        }
    }

    fn analyze_match(&mut self, node: &SyntaxNode, scope: ScopeId) {
        for child in node.child_nodes() {
            if child.kind() == SyntaxKind::MatchArm {
                let arm_scope = self.new_scope(scope, ScopeKind::MatchArm, child.span());
                for arm_child in child.child_nodes() {
                    if arm_child.kind() == SyntaxKind::Pattern {
                        self.define_pattern(arm_child, arm_scope, SymbolKind::Local);
                    } else {
                        self.analyze_node(arm_child, arm_scope);
                    }
                }
            } else {
                self.analyze_node(child, scope);
            }
        }
    }

    fn analyze_closure(&mut self, node: &SyntaxNode, scope: ScopeId) {
        let closure_scope = self.new_scope(scope, ScopeKind::Closure, node.span());
        let tokens = significant_direct_tokens(node);
        let mut inside = false;
        let mut previous = None;
        for token in tokens {
            if token.kind() == TokenKind::Pipe {
                inside = !inside;
                previous = Some(token.kind());
                continue;
            }
            if inside && token.kind() == TokenKind::Identifier {
                let skip = matches!(
                    previous,
                    Some(TokenKind::Colon) | Some(TokenKind::ColonColon)
                );
                if !skip {
                    if let Some(name) = token.text(self.source) {
                        if is_binding_name(name) {
                            self.define(
                                closure_scope,
                                name,
                                SymbolKind::Parameter,
                                Namespace::Value,
                                Visibility::Private,
                                SymbolOrigin::Source,
                                token.span(),
                            );
                        }
                    }
                }
            }
            previous = Some(token.kind());
        }
        for child in node.child_nodes() {
            self.analyze_node(child, closure_scope);
        }
    }

    fn analyze_task_group(&mut self, node: &SyntaxNode, scope: ScopeId) {
        let group_scope = self.new_scope(scope, ScopeKind::TaskGroup, node.span());
        let tokens = significant_direct_tokens(node);
        let name_token = tokens
            .iter()
            .skip_while(|token| token.kind() != TokenKind::Keyword(Keyword::TaskGroup))
            .skip(1)
            .find(|token| token.kind() == TokenKind::Identifier)
            .copied();
        if let Some(token) = name_token {
            if let Some(name) = token.text(self.source) {
                self.define(
                    group_scope,
                    name,
                    SymbolKind::TaskGroup,
                    Namespace::Value,
                    Visibility::Private,
                    SymbolOrigin::Source,
                    token.span(),
                );
            }
        }
        for child in node.child_nodes() {
            if child.kind() == SyntaxKind::Block {
                self.analyze_block(child, group_scope, false);
            } else {
                self.analyze_node(child, group_scope);
            }
        }
    }

    fn define_pattern(&mut self, pattern: &SyntaxNode, scope: ScopeId, kind: SymbolKind) {
        for token in pattern_binding_tokens(pattern, self.source) {
            if let Some(name) = token.text(self.source) {
                self.define(
                    scope,
                    name,
                    kind,
                    Namespace::Value,
                    Visibility::Private,
                    SymbolOrigin::Source,
                    token.span(),
                );
            }
        }
    }

    fn resolve_name_expression(&mut self, node: &SyntaxNode, scope: ScopeId) {
        let tokens = significant_direct_tokens(node);
        if tokens
            .first()
            .is_some_and(|token| token.kind() == TokenKind::Dot)
        {
            return;
        }
        let Some(token) = tokens.iter().find(|token| {
            matches!(
                token.kind(),
                TokenKind::Identifier
                    | TokenKind::Keyword(Keyword::SelfValue)
                    | TokenKind::Keyword(Keyword::Ok)
                    | TokenKind::Keyword(Keyword::Err)
            )
        }) else {
            return;
        };
        let Some(name) = token.text(self.source) else {
            return;
        };
        let namespace = if starts_like_type(name) {
            Namespace::Type
        } else {
            Namespace::Value
        };
        let symbol = self.lookup(scope, namespace, name);

        // Edition 2026 defers unknown type-path diagnostics to the type checker. Known
        // type paths are still linked into the D5 resolution graph.
        if namespace == Namespace::Type && symbol.is_none() {
            return;
        }

        self.resolutions.push(Resolution {
            name: name.to_owned(),
            namespace,
            span: token.span(),
            scope,
            symbol,
        });

        if symbol.is_none() {
            let mut diagnostic = Diagnostic::error(
                "SEM003",
                format!("cannot resolve value name `{name}`"),
            )
            .with_primary(token.span(), "no visible declaration has this name")
            .with_note("D5 resolves lexical values; member lookup and full type lookup arrive with type checking");
            if let Some(suggestion) = self.closest_visible_name(scope, namespace, name) {
                diagnostic = diagnostic.with_help(format!("did you mean `{suggestion}`?"));
            } else {
                diagnostic = diagnostic.with_help(format!(
                    "declare `{name}` before this use, add it as a parameter, or import it"
                ));
            }
            self.diagnostics.push(diagnostic);
        }
    }

    fn resolve_type_reference(&mut self, node: &SyntaxNode, scope: ScopeId) {
        let mut seen = HashSet::new();
        for token in descendant_tokens(node) {
            if token.kind() != TokenKind::Identifier {
                continue;
            }
            let Some(name) = token.text(self.source) else {
                continue;
            };
            if !starts_like_type(name) || !seen.insert(name.to_owned()) {
                continue;
            }
            if let Some(symbol) = self.lookup(scope, Namespace::Type, name) {
                self.resolutions.push(Resolution {
                    name: name.to_owned(),
                    namespace: Namespace::Type,
                    span: token.span(),
                    scope,
                    symbol: Some(symbol),
                });
            }
        }
    }

    fn new_scope(&mut self, parent: ScopeId, kind: ScopeKind, span: Span) -> ScopeId {
        let id = ScopeId(u32::try_from(self.scopes.len()).unwrap_or(u32::MAX));
        self.scopes.push(Scope {
            id,
            parent: Some(parent),
            kind,
            span,
            symbols: Vec::new(),
        });
        id
    }

    #[allow(clippy::too_many_arguments)]
    fn define(
        &mut self,
        scope: ScopeId,
        name: &str,
        kind: SymbolKind,
        namespace: Namespace,
        visibility: Visibility,
        origin: SymbolOrigin,
        span: Span,
    ) -> Option<SymbolId> {
        if name == "_" || name.is_empty() {
            return None;
        }
        let key = BindingKey {
            scope,
            namespace,
            name: name.to_owned(),
        };
        if let Some(previous) = self.bindings.get(&key).copied() {
            let previous_span = self
                .symbols
                .get(previous.raw() as usize)
                .map_or(span, |symbol| symbol.span);
            let code = duplicate_code(kind);
            self.diagnostics.push(
                Diagnostic::error(
                    code,
                    format!("duplicate {} name `{name}`", namespace.as_str()),
                )
                .with_primary(
                    span,
                    "this declaration conflicts with an earlier name in the same scope",
                )
                .with_secondary(previous_span, "the first declaration is here")
                .with_help("rename or remove one of the declarations"),
            );
            return None;
        }
        Some(self.define_unchecked(scope, name, kind, namespace, visibility, origin, span))
    }

    #[allow(clippy::too_many_arguments)]
    fn define_unchecked(
        &mut self,
        scope: ScopeId,
        name: &str,
        kind: SymbolKind,
        namespace: Namespace,
        visibility: Visibility,
        origin: SymbolOrigin,
        span: Span,
    ) -> SymbolId {
        let id = SymbolId(u32::try_from(self.symbols.len()).unwrap_or(u32::MAX));
        self.symbols.push(Symbol {
            id,
            name: name.to_owned(),
            kind,
            namespace,
            visibility,
            origin,
            scope,
            span,
        });
        self.bindings.insert(
            BindingKey {
                scope,
                namespace,
                name: name.to_owned(),
            },
            id,
        );
        if let Some(target_scope) = self.scopes.get_mut(scope.raw() as usize) {
            target_scope.symbols.push(id);
        }
        id
    }

    fn lookup(&self, mut scope: ScopeId, namespace: Namespace, name: &str) -> Option<SymbolId> {
        loop {
            let key = BindingKey {
                scope,
                namespace,
                name: name.to_owned(),
            };
            if let Some(symbol) = self.bindings.get(&key) {
                return Some(*symbol);
            }
            let parent = self
                .scopes
                .get(scope.raw() as usize)
                .and_then(|entry| entry.parent);
            let Some(parent) = parent else {
                return None;
            };
            scope = parent;
        }
    }

    fn closest_visible_name(
        &self,
        mut scope: ScopeId,
        namespace: Namespace,
        requested: &str,
    ) -> Option<String> {
        let mut best: Option<(usize, String)> = None;
        let mut visited = HashSet::new();
        loop {
            if let Some(entry) = self.scopes.get(scope.raw() as usize) {
                for symbol_id in &entry.symbols {
                    let Some(symbol) = self.symbols.get(symbol_id.raw() as usize) else {
                        continue;
                    };
                    if symbol.namespace != namespace || !visited.insert(symbol.name.clone()) {
                        continue;
                    }
                    let distance = edit_distance(requested, &symbol.name);
                    let improves = match best.as_ref() {
                        Some((current, _)) => distance < *current,
                        None => true,
                    };
                    if distance <= 3 && improves {
                        best = Some((distance, symbol.name.clone()));
                    }
                }
                if let Some(parent) = entry.parent {
                    scope = parent;
                    continue;
                }
            }
            break;
        }
        best.map(|(_, name)| name)
    }
}

fn declaration_symbol_kind(kind: SyntaxKind) -> Option<(SymbolKind, Namespace)> {
    Some(match kind {
        SyntaxKind::ConstDeclaration => (SymbolKind::Constant, Namespace::Value),
        SyntaxKind::TypeAliasDeclaration => (SymbolKind::TypeAlias, Namespace::Type),
        SyntaxKind::NewtypeDeclaration => (SymbolKind::Newtype, Namespace::Type),
        SyntaxKind::RecordDeclaration => (SymbolKind::Record, Namespace::Type),
        SyntaxKind::StructDeclaration => (SymbolKind::Struct, Namespace::Type),
        SyntaxKind::EnumDeclaration => (SymbolKind::Enum, Namespace::Type),
        SyntaxKind::TraitDeclaration => (SymbolKind::Trait, Namespace::Type),
        SyntaxKind::FunctionDeclaration => (SymbolKind::Function, Namespace::Value),
        _ => return None,
    })
}

fn duplicate_code(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Local | SymbolKind::TaskGroup => "SEM002",
        SymbolKind::Parameter | SymbolKind::GenericParameter => "SEM005",
        SymbolKind::Field | SymbolKind::EnumVariant | SymbolKind::Method => "SEM006",
        _ => "SEM001",
    }
}

fn visibility(node: &SyntaxNode) -> Visibility {
    if significant_direct_tokens(node)
        .iter()
        .any(|token| token.kind() == TokenKind::Keyword(Keyword::Pub))
    {
        Visibility::Public
    } else {
        Visibility::Private
    }
}

fn first_direct_name(node: &SyntaxNode, source: &SourceFile) -> Option<(String, Span)> {
    significant_direct_tokens(node)
        .into_iter()
        .find(|token| token.kind() == TokenKind::Identifier)
        .and_then(|token| {
            token
                .text(source)
                .map(|text| (text.to_owned(), token.span()))
        })
}

fn parameter_name(node: &SyntaxNode, source: &SourceFile) -> Option<(String, Span)> {
    significant_direct_tokens(node)
        .into_iter()
        .find(|token| {
            matches!(
                token.kind(),
                TokenKind::Identifier | TokenKind::Keyword(Keyword::SelfValue)
            )
        })
        .and_then(|token| {
            token
                .text(source)
                .map(|text| (text.to_owned(), token.span()))
        })
}

fn path_after_keyword(
    node: &SyntaxNode,
    source: &SourceFile,
    keyword: Keyword,
) -> Option<(String, Span)> {
    let tokens = significant_direct_tokens(node);
    let start = tokens
        .iter()
        .position(|token| token.kind() == TokenKind::Keyword(keyword))?;
    let path_tokens = &tokens[start + 1..];
    let first = path_tokens.first()?.span();
    let last = path_tokens.last()?.span();
    let span = first.cover(last)?;
    let mut name = String::new();
    for token in path_tokens {
        match token.kind() {
            TokenKind::Identifier | TokenKind::Dot | TokenKind::ColonColon => {
                name.push_str(token.text(source).unwrap_or(""));
            }
            _ => break,
        }
    }
    (!name.is_empty()).then_some((name, span))
}

fn import_bindings(node: &SyntaxNode, source: &SourceFile) -> Vec<(String, Span)> {
    let tokens = significant_direct_tokens(node);
    if let Some(as_index) = tokens
        .iter()
        .position(|token| token.kind() == TokenKind::Keyword(Keyword::As))
    {
        if let Some(alias) = tokens[as_index + 1..]
            .iter()
            .find(|token| token.kind() == TokenKind::Identifier)
        {
            if let Some(name) = alias.text(source) {
                return vec![(name.to_owned(), alias.span())];
            }
        }
    }

    if let Some(open) = tokens
        .iter()
        .position(|token| token.kind() == TokenKind::LeftBrace)
    {
        return tokens[open + 1..]
            .iter()
            .take_while(|token| token.kind() != TokenKind::RightBrace)
            .filter(|token| token.kind() == TokenKind::Identifier)
            .filter_map(|token| {
                token
                    .text(source)
                    .map(|name| (name.to_owned(), token.span()))
            })
            .collect();
    }

    tokens
        .iter()
        .rev()
        .find(|token| token.kind() == TokenKind::Identifier)
        .and_then(|token| {
            token
                .text(source)
                .map(|name| vec![(name.to_owned(), token.span())])
        })
        .unwrap_or_default()
}

fn significant_direct_tokens(node: &SyntaxNode) -> Vec<SyntaxToken> {
    node.child_tokens()
        .filter(|token| !token.kind().is_trivia() && token.kind() != TokenKind::Eof)
        .collect()
}

fn descendant_tokens(node: &SyntaxNode) -> Vec<SyntaxToken> {
    let mut output = Vec::new();
    collect_descendant_tokens(node, &mut output);
    output
}

fn collect_descendant_tokens(node: &SyntaxNode, output: &mut Vec<SyntaxToken>) {
    for element in node.children_with_tokens() {
        match element {
            nivra_syntax::SyntaxElement::Node(child) => collect_descendant_tokens(child, output),
            nivra_syntax::SyntaxElement::Token(token) => output.push(*token),
        }
    }
}

fn pattern_binding_tokens(node: &SyntaxNode, source: &SourceFile) -> Vec<SyntaxToken> {
    let tokens = significant_direct_tokens(node);
    let mut output = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.kind() != TokenKind::Identifier {
            continue;
        }
        let previous = index.checked_sub(1).and_then(|value| tokens.get(value));
        if previous
            .is_some_and(|value| matches!(value.kind(), TokenKind::Dot | TokenKind::ColonColon))
        {
            continue;
        }
        let Some(name) = token.text(source) else {
            continue;
        };
        if is_binding_name(name) {
            output.push(*token);
        }
    }
    output
}

fn is_binding_name(name: &str) -> bool {
    name != "_" && !starts_like_type(name)
}

fn starts_like_type(name: &str) -> bool {
    name.chars().next().is_some_and(char::is_uppercase)
}

fn edit_distance(left: &str, right: &str) -> usize {
    let right_chars = right.chars().collect::<Vec<_>>();
    let mut previous = (0..=right_chars.len()).collect::<Vec<_>>();
    for (left_index, left_char) in left.chars().enumerate() {
        let mut current = vec![left_index + 1];
        for (right_index, right_char) in right_chars.iter().enumerate() {
            let insertion = current[right_index] + 1;
            let deletion = previous[right_index + 1] + 1;
            let substitution_cost = if left_char == *right_char { 0 } else { 1 };
            let replacement = previous[right_index] + substitution_cost;
            current.push(insertion.min(deletion).min(replacement));
        }
        previous = current;
    }
    previous[right_chars.len()]
}

impl fmt::Display for Namespace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use nivra_parser::parse;
    use nivra_source::SourceManager;

    use super::{analyze, Namespace, SymbolKind, SymbolOrigin};

    fn semantic(source_text: &str) -> super::SemanticResult {
        let mut sources = SourceManager::new();
        let id = sources
            .add_virtual("test.nva", source_text)
            .unwrap_or_else(|error| panic!("{error}"));
        let source = sources.get(id).unwrap_or_else(|| panic!("missing source"));
        let parsed = parse(source);
        assert!(
            !parsed.has_errors(),
            "parser diagnostics: {:?}",
            parsed.diagnostics
        );
        analyze(source, &parsed.root)
    }

    #[test]
    fn indexes_module_and_declarations() {
        let result = semantic(
            "module demo.core\npub record User { name: String }\nfn main() { print(1) }\n",
        );
        assert_eq!(result.module.name, "demo.core");
        assert!(result.user_symbols().any(|symbol| symbol.name == "User"));
        assert!(result.user_symbols().any(|symbol| symbol.name == "main"));
    }

    #[test]
    fn reports_duplicate_top_level_values() {
        let result = semantic("module demo\nfn run() {}\nfn run() {}\n");
        assert!(result.diagnostics.iter().any(|item| item.code == "SEM001"));
    }

    #[test]
    fn keeps_type_and_value_names_independent() {
        let result = semantic("module demo\nrecord Item {}\nfn Item() {}\n");
        assert!(!result.diagnostics.iter().any(|item| item.code == "SEM001"));
    }

    #[test]
    fn reports_duplicate_locals() {
        let result = semantic("module demo\nfn main() { let value = 1\n let value = 2\n }\n");
        assert!(result.diagnostics.iter().any(|item| item.code == "SEM002"));
    }

    #[test]
    fn reports_unknown_value_name() {
        let result = semantic("module demo\nfn main() { missing_service() }\n");
        assert!(result.diagnostics.iter().any(|item| item.code == "SEM003"));
    }

    #[test]
    fn suggests_nearby_visible_name() {
        let result = semantic("module demo\nfn main() { let message = 1\n mesage }\n");
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|item| item.code == "SEM003")
            .unwrap_or_else(|| panic!("missing semantic diagnostic"));
        assert!(diagnostic
            .help
            .as_deref()
            .is_some_and(|help| help.contains("message")));
    }

    #[test]
    fn parameters_resolve_inside_function() {
        let result = semantic("module demo\nfn echo(value: Int) { print(value) }\n");
        assert!(!result.has_errors());
        assert!(result
            .resolutions
            .iter()
            .any(|item| item.name == "value" && item.symbol.is_some()));
    }

    #[test]
    fn nested_blocks_resolve_parent_bindings() {
        let result =
            semantic("module demo\nfn main() { let outer = 1\n if true { print(outer) } }\n");
        assert!(!result.has_errors());
    }

    #[test]
    fn later_local_is_not_visible_early() {
        let result = semantic("module demo\nfn main() { print(later)\n let later = 1\n }\n");
        assert!(result.diagnostics.iter().any(|item| item.code == "SEM003"));
    }

    #[test]
    fn for_pattern_is_visible_in_loop_body() {
        let result = semantic(
            "module demo\nfn main(values: List<Int>) { for value in values { print(value) } }\n",
        );
        assert!(!result.has_errors());
    }

    #[test]
    fn match_arm_pattern_is_visible_in_arm() {
        let result =
            semantic("module demo\nfn main(value: Int) { match value { item => print(item), } }\n");
        assert!(!result.has_errors());
    }

    #[test]
    fn closure_parameter_is_visible_in_body() {
        let result =
            semantic("module demo\nfn main(values: List<Int>) { values.map(|item| item) }\n");
        assert!(!result.has_errors());
    }

    #[test]
    fn task_group_handle_is_visible_in_body() {
        let result = semantic(
            "module demo\nfn main() { task_group group { group.spawn async { print(1) } } }\n",
        );
        assert!(!result.has_errors());
    }

    #[test]
    fn imports_create_visible_symbols() {
        let result = semantic("module demo\nuse std.fs\nfn main() { fs.read_text(\"a\") }\n");
        assert!(!result.has_errors());
        assert!(result
            .user_symbols()
            .any(|symbol| { symbol.name == "fs" && symbol.origin == SymbolOrigin::Import }));
    }

    #[test]
    fn duplicate_parameters_are_rejected() {
        let result = semantic("module demo\nfn add(value: Int, value: Int) { value }\n");
        assert!(result.diagnostics.iter().any(|item| item.code == "SEM005"));
    }

    #[test]
    fn duplicate_generic_parameters_are_deferred_to_type_checker() {
        let result = semantic("module demo\nfn choose<T, T>(value: T) -> T { value }\n");
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert_eq!(
            result
                .symbols
                .iter()
                .filter(|symbol| {
                    symbol.kind == SymbolKind::GenericParameter && symbol.name == "T"
                })
                .count(),
            1
        );
    }

    #[test]
    fn duplicate_fields_are_rejected() {
        let result = semantic("module demo\nrecord User { name: String\n name: String }\n");
        assert!(result.diagnostics.iter().any(|item| item.code == "SEM006"));
    }

    #[test]
    fn prelude_names_are_not_user_symbols() {
        let result = semantic("module demo\nfn main() { print(1) }\n");
        assert!(result
            .symbols
            .iter()
            .any(|symbol| { symbol.name == "print" && symbol.kind == SymbolKind::Builtin }));
        assert!(!result.user_symbols().any(|symbol| symbol.name == "print"));
    }

    #[test]
    fn type_references_link_to_known_types() {
        let result =
            semantic("module demo\nrecord User {}\nfn load(user: User) -> User { user }\n");
        assert!(result.resolutions.iter().any(|resolution| {
            resolution.name == "User" && resolution.namespace == Namespace::Type
        }));
    }
}
