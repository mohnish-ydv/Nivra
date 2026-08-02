//! Type representation and first static type-checking pass for Nivra Edition 2026.
//!
//! D7 extends the D6 checker with record/struct bodies, constructors, enum variants,
//! fields, methods, `Self`, and mutable receivers. Unknown types are
//! retained as recovery values so one unsupported member or generic operation does
//! not produce a wall of follow-on diagnostics.

use std::collections::{HashMap, HashSet};
use std::fmt::{self, Write as _};

use nivra_diagnostics::Diagnostic;
use nivra_lexer::{Keyword, TokenKind};
use nivra_sema::{Namespace, SemanticResult, SymbolKind};
use nivra_source::{SourceFile, Span};
use nivra_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

/// Static type used by the D7 checker.
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
    pub owner: Option<String>,
    pub trait_name: Option<String>,
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

/// Kind of a declared nominal type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NominalKind {
    Record,
    Struct,
    Enum,
}

impl NominalKind {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Record => "record",
            Self::Struct => "struct",
            Self::Enum => "enum",
        }
    }
}

/// One record or struct field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldInfo {
    pub name: String,
    pub ty: Type,
    pub span: Span,
    pub has_default: bool,
    pub public: bool,
}

/// One enum variant and its positional payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariantInfo {
    pub name: String,
    pub payload: Vec<Type>,
    pub span: Span,
}

/// One method attached through an inherent or trait implementation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MethodInfo {
    pub name: String,
    pub parameters: Vec<ParameterType>,
    pub return_type: Type,
    pub span: Span,
    pub mutable_receiver: bool,
    pub trait_name: Option<String>,
}

/// Indexed nominal type body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NominalTypeInfo {
    pub name: String,
    pub kind: NominalKind,
    pub generic_parameters: Vec<String>,
    pub fields: Vec<FieldInfo>,
    pub variants: Vec<VariantInfo>,
    pub methods: Vec<MethodInfo>,
    pub span: Span,
}

/// Complete D7 type-check result.
#[derive(Clone, Debug)]
pub struct TypeCheckResult {
    pub functions: Vec<FunctionSignature>,
    pub nominals: Vec<NominalTypeInfo>,
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
            if let Some(owner) = &signature.owner {
                let _ = write!(output, "fn {owner}.{}(", signature.name);
            } else {
                let _ = write!(output, "fn {}(", signature.name);
            }
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

    /// Produces a deterministic nominal-type and member report.
    #[must_use]
    pub fn nominal_report(&self) -> String {
        let mut output = String::new();
        for nominal in &self.nominals {
            let _ = write!(output, "{} {}", nominal.kind.name(), nominal.name);
            if !nominal.generic_parameters.is_empty() {
                let _ = write!(output, "<{}>", nominal.generic_parameters.join(", "));
            }
            let _ = writeln!(output, " @ {}..{}", nominal.span.start(), nominal.span.end());
            for field in &nominal.fields {
                let _ = writeln!(
                    output,
                    "  field {}: {}{}{}",
                    field.name,
                    field.ty,
                    if field.has_default { " = <default>" } else { "" },
                    if field.public { " pub" } else { "" }
                );
            }
            for variant in &nominal.variants {
                if variant.payload.is_empty() {
                    let _ = writeln!(output, "  variant {}", variant.name);
                } else {
                    let payload = variant
                        .payload
                        .iter()
                        .map(Type::display_name)
                        .collect::<Vec<_>>()
                        .join(", ");
                    let _ = writeln!(output, "  variant {}({payload})", variant.name);
                }
            }
            for method in &nominal.methods {
                let parameters = method
                    .parameters
                    .iter()
                    .map(|parameter| format!("{}: {}", parameter.name, parameter.ty))
                    .collect::<Vec<_>>()
                    .join(", ");
                let _ = writeln!(
                    output,
                    "  method {}({parameters}) -> {}{}{}",
                    method.name,
                    method.return_type,
                    if method.mutable_receiver { " mut-self" } else { "" },
                    method
                        .trait_name
                        .as_ref()
                        .map_or(String::new(), |name| format!(" trait={name}"))
                );
            }
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
    nominals: Vec<NominalTypeInfo>,
    nominal_lookup: HashMap<String, usize>,
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
            nominals: Vec::new(),
            nominal_lookup: HashMap::new(),
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
        self.collect_nominals(root);
        self.collect_signatures(root);
        self.attach_methods();
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
            nominals: self.nominals,
            bindings: self.bindings,
            expressions: self.expressions,
            diagnostics: self.diagnostics,
        }
    }

    fn collect_nominals(&mut self, root: &SyntaxNode) {
        for node in root.child_nodes() {
            let kind = match node.kind() {
                SyntaxKind::RecordDeclaration => NominalKind::Record,
                SyntaxKind::StructDeclaration => NominalKind::Struct,
                SyntaxKind::EnumDeclaration => NominalKind::Enum,
                _ => continue,
            };
            let Some((name, span)) = first_direct_identifier(node, self.source) else {
                continue;
            };
            let mut generic_parameters = generic_parameter_names(node, self.source)
                .into_iter()
                .collect::<Vec<_>>();
            generic_parameters.sort();
            let generic_set = generic_parameters.iter().cloned().collect::<HashSet<_>>();
            let mut fields = Vec::new();
            let mut variants = Vec::new();

            if matches!(kind, NominalKind::Record | NominalKind::Struct) {
                if let Some(field_list) = node
                    .child_nodes()
                    .find(|child| child.kind() == SyntaxKind::FieldList)
                {
                    for field in field_list
                        .child_nodes()
                        .filter(|child| child.kind() == SyntaxKind::Field)
                    {
                        if let Some((field_name, field_span, type_text, has_default, public)) =
                            field_declaration_parts(field, self.source)
                        {
                            let ty = self.parse_declared_type(
                                &type_text,
                                field.span(),
                                &generic_set,
                            );
                            fields.push(FieldInfo {
                                name: field_name,
                                ty,
                                span: field_span,
                                has_default,
                                public,
                            });
                        }
                    }
                }
            } else {
                for variant in node
                    .child_nodes()
                    .filter(|child| child.kind() == SyntaxKind::EnumVariant)
                {
                    if let Some((variant_name, variant_span, payload_texts)) =
                        enum_variant_parts(variant, self.source)
                    {
                        let payload = payload_texts
                            .iter()
                            .map(|text| {
                                self.parse_declared_type(text, variant.span(), &generic_set)
                            })
                            .collect();
                        variants.push(VariantInfo {
                            name: variant_name,
                            payload,
                            span: variant_span,
                        });
                    }
                }
            }

            let index = self.nominals.len();
            let _ = self.nominal_lookup.insert(name.clone(), index);
            self.nominals.push(NominalTypeInfo {
                name,
                kind,
                generic_parameters,
                fields,
                variants,
                methods: Vec::new(),
                span,
            });
        }
    }

    fn collect_signatures(&mut self, root: &SyntaxNode) {
        for node in root.child_nodes() {
            match node.kind() {
                SyntaxKind::FunctionDeclaration => {
                    self.collect_one_signature(node, false, None, None, true);
                }
                SyntaxKind::ExternBlock => {
                    for function in node.child_nodes() {
                        if function.kind() == SyntaxKind::ExternFunction {
                            self.collect_one_signature(function, true, None, None, true);
                        }
                    }
                }
                SyntaxKind::TraitDeclaration => {
                    let owner = first_direct_identifier(node, self.source)
                        .map(|(name, _)| name);
                    for function in node.child_nodes() {
                        if function.kind() == SyntaxKind::FunctionDeclaration {
                            self.collect_one_signature(
                                function,
                                false,
                                owner.clone(),
                                owner.clone(),
                                false,
                            );
                        }
                    }
                }
                SyntaxKind::ImplDeclaration => {
                    let (target, trait_name) = impl_header(node, self.source);
                    for function in node.child_nodes() {
                        if function.kind() == SyntaxKind::FunctionDeclaration {
                            self.collect_one_signature(
                                function,
                                false,
                                target.clone(),
                                trait_name.clone(),
                                false,
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn collect_one_signature(
        &mut self,
        node: &SyntaxNode,
        is_extern: bool,
        owner: Option<String>,
        trait_name: Option<String>,
        top_level: bool,
    ) {
        if let Some(signature) = self.signature_from_node(node, is_extern, owner, trait_name) {
            let index = self.functions.len();
            if top_level {
                let _ = self.function_lookup.insert(signature.name.clone(), index);
            }
            self.functions.push(signature);
        }
    }

    fn signature_from_node(
        &mut self,
        node: &SyntaxNode,
        is_extern: bool,
        owner: Option<String>,
        trait_name: Option<String>,
    ) -> Option<FunctionSignature> {
        let (name, _name_span) = function_name(node, self.source)?;
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
                let mut ty = if type_text.trim().is_empty() {
                    Type::Unknown
                } else {
                    self.parse_declared_type(&type_text, parameter.span(), &generic_names)
                };
                if let Some(owner_name) = &owner {
                    ty = replace_self_type(ty, owner_name);
                }
                parameters.push(ParameterType {
                    name: parameter_name,
                    ty,
                    span: parameter_span,
                });
            }
        }
        let mut return_type = node
            .child_nodes()
            .find(|child| child.kind() == SyntaxKind::TypeReference)
            .map_or(Type::Unit, |type_node| {
                self.parse_type_node(type_node, &generic_names)
            });
        if let Some(owner_name) = &owner {
            return_type = replace_self_type(return_type, owner_name);
        }
        let is_async = significant_direct_tokens(node).iter().any(|token| {
            token.kind() == TokenKind::Keyword(Keyword::Async)
        });
        Some(FunctionSignature {
            name,
            owner,
            trait_name,
            parameters,
            return_type,
            span: node.span(),
            is_async,
            is_extern,
        })
    }

    fn attach_methods(&mut self) {
        let signatures = self.functions.clone();
        for signature in signatures {
            let Some(owner) = signature.owner.clone() else {
                continue;
            };
            let Some(index) = self.nominal_lookup.get(&owner).copied() else {
                continue;
            };
            let receiver = signature
                .parameters
                .first()
                .filter(|parameter| parameter.name == "self");
            let mutable_receiver = receiver.is_some_and(|parameter| {
                matches!(
                    &parameter.ty,
                    Type::Reference {
                        mutable: true,
                        ..
                    }
                )
            });
            let parameters = if receiver.is_some() {
                signature.parameters.iter().skip(1).cloned().collect()
            } else {
                signature.parameters.clone()
            };
            if let Some(nominal) = self.nominals.get_mut(index) {
                nominal.methods.push(MethodInfo {
                    name: signature.name,
                    parameters,
                    return_type: signature.return_type,
                    span: signature.span,
                    mutable_receiver,
                    trait_name: signature.trait_name,
                });
            }
        }
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
            .functions
            .iter()
            .find(|signature| signature.name == name && signature.span == node.span())
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
            SyntaxKind::MemberExpression => self.infer_member(node),
            SyntaxKind::RecordExpression => self.infer_record_expression(node),
            SyntaxKind::ClosureExpression | SyntaxKind::RangeExpression => {
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
        } else if children[0].kind() == SyntaxKind::MemberExpression
            && !self.is_mutable_place(children[0])
        {
            self.diagnostics.push(
                Diagnostic::error("NOM008", "cannot assign through an immutable member access")
                    .with_primary(children[0].span(), "the receiver is not mutable")
                    .with_help("bind the record with `var` or use an `&mut` receiver"),
            );
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

        if let Some(result) = self.infer_enum_variant_call(callee, &arguments, node.span()) {
            return result;
        }

        let callee_type = self.infer_expression(callee);
        if let Type::Function(parameters, result) = callee_type {
            if parameters.len() != arguments.len() {
                self.diagnostics.push(
                    Diagnostic::error(
                        "TYP003",
                        format!(
                            "call expects {} argument(s), found {}",
                            parameters.len(),
                            arguments.len()
                        ),
                    )
                    .with_primary(node.span(), "wrong number of arguments")
                    .with_help("add or remove arguments to match the callable signature"),
                );
            }
            for ((argument, actual), expected) in arguments.iter().zip(parameters.iter()) {
                self.require_assignable(
                    expected,
                    actual,
                    argument.span(),
                    "call argument",
                    "TYP004",
                );
            }
            *result
        } else {
            Type::Unknown
        }
    }

    fn infer_enum_variant_call(
        &mut self,
        callee: &SyntaxNode,
        arguments: &[(&SyntaxNode, Type)],
        call_span: Span,
    ) -> Option<Type> {
        if callee.kind() != SyntaxKind::MemberExpression {
            return None;
        }
        let base = callee.child_nodes().next()?;
        if base.kind() != SyntaxKind::NameExpression {
            return None;
        }
        let type_name = significant_text(base, self.source);
        let index = self.nominal_lookup.get(&type_name).copied()?;
        let nominal = self.nominals.get(index)?.clone();
        if nominal.kind != NominalKind::Enum {
            return None;
        }
        let (variant_name, _) = member_name(callee, self.source)?;
        let variant = nominal
            .variants
            .iter()
            .find(|variant| variant.name == variant_name)?
            .clone();
        let generic_names = nominal
            .generic_parameters
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        let payload = variant
            .payload
            .iter()
            .cloned()
            .map(|ty| erase_generic_parameters(ty, &generic_names))
            .collect::<Vec<_>>();

        if payload.len() != arguments.len() {
            self.diagnostics.push(
                Diagnostic::error(
                    "NOM007",
                    format!(
                        "enum variant `{}.{}` expects {} payload value(s), found {}",
                        nominal.name,
                        variant.name,
                        payload.len(),
                        arguments.len()
                    ),
                )
                .with_primary(call_span, "wrong enum variant payload")
                .with_secondary(variant.span, "variant declared here")
                .with_help("match the number and types of the declared variant payload"),
            );
        }
        for ((argument, actual), expected) in arguments.iter().zip(payload.iter()) {
            if !types_compatible(expected, actual) {
                self.diagnostics.push(
                    Diagnostic::error(
                        "NOM007",
                        format!(
                            "enum variant `{}.{}` expects `{expected}`, found `{actual}`",
                            nominal.name, variant.name
                        ),
                    )
                    .with_primary(argument.span(), "wrong enum variant payload type")
                    .with_secondary(variant.span, "variant declared here")
                    .with_help(format!("provide a `{expected}` payload value")),
                );
            }
        }

        Some(Type::Named(
            nominal.name,
            vec![Type::Unknown; nominal.generic_parameters.len()],
        ))
    }

    fn infer_member(&mut self, node: &SyntaxNode) -> Type {
        let children = node.child_nodes().collect::<Vec<_>>();
        let Some(base_node) = children.first().copied() else {
            return Type::Error;
        };
        let Some((member, member_span)) = member_name(node, self.source) else {
            return Type::Error;
        };

        if base_node.kind() == SyntaxKind::NameExpression {
            let type_name = significant_text(base_node, self.source);
            if let Some(index) = self.nominal_lookup.get(&type_name).copied() {
                if let Some(nominal) = self.nominals.get(index).cloned() {
                    if nominal.kind == NominalKind::Enum {
                        if let Some(variant) = nominal
                            .variants
                            .iter()
                            .find(|variant| variant.name == member)
                        {
                            let generic_names = nominal
                                .generic_parameters
                                .iter()
                                .cloned()
                                .collect::<HashSet<_>>();
                            let result = Type::Named(
                                nominal.name.clone(),
                                vec![Type::Unknown; nominal.generic_parameters.len()],
                            );
                            if variant.payload.is_empty() {
                                return result;
                            }
                            let payload = variant
                                .payload
                                .iter()
                                .cloned()
                                .map(|ty| erase_generic_parameters(ty, &generic_names))
                                .collect();
                            return Type::Function(payload, Box::new(result));
                        }
                        let candidates = nominal
                            .variants
                            .iter()
                            .map(|variant| variant.name.as_str())
                            .collect::<Vec<_>>();
                        let mut diagnostic = Diagnostic::error(
                            "NOM001",
                            format!("enum `{}` has no variant `{member}`", nominal.name),
                        )
                        .with_primary(member_span, "unknown enum variant");
                        if let Some(suggestion) = closest_name(&candidates, &member) {
                            diagnostic =
                                diagnostic.with_help(format!("did you mean `{suggestion}`?"));
                        } else {
                            diagnostic = diagnostic.with_help("use a declared enum variant");
                        }
                        self.diagnostics.push(diagnostic);
                        return Type::Error;
                    }
                }
            }
        }

        let base_type = self.infer_expression(base_node);
        if base_type.is_recovery() {
            return Type::Unknown;
        }
        if matches!(base_type, Type::Optional(_)) {
            self.diagnostics.push(
                Diagnostic::error(
                    "NOM002",
                    format!("cannot access member `{member}` through an optional value"),
                )
                .with_primary(member_span, "optional values must be handled before member access")
                .with_help("use `if let`, `match`, or another explicit optional operation"),
            );
            return Type::Error;
        }
        let Some(nominal_name) = nominal_name_from_type(&base_type) else {
            self.diagnostics.push(
                Diagnostic::error(
                    "NOM002",
                    format!("type `{base_type}` does not expose nominal members"),
                )
                .with_primary(member_span, "member lookup requires a record, struct, or enum value")
                .with_help("access a declared nominal type or use a supported built-in operation"),
            );
            return Type::Error;
        };
        let Some(index) = self.nominal_lookup.get(&nominal_name).copied() else {
            return Type::Unknown;
        };
        let Some(nominal) = self.nominals.get(index).cloned() else {
            return Type::Unknown;
        };

        if let Some(field) = nominal.fields.iter().find(|field| field.name == member) {
            return field.ty.clone();
        }
        if let Some(method) = nominal.methods.iter().find(|method| method.name == member) {
            if method.mutable_receiver && !self.is_mutable_place(base_node) {
                self.diagnostics.push(
                    Diagnostic::error(
                        "NOM008",
                        format!("method `{}` requires a mutable receiver", method.name),
                    )
                    .with_primary(base_node.span(), "this receiver is immutable")
                    .with_secondary(method.span, "mutable receiver declared here")
                    .with_help("bind the value with `var` or pass it through `&mut`"),
                );
            }
            return Type::Function(
                method
                    .parameters
                    .iter()
                    .map(|parameter| parameter.ty.clone())
                    .collect(),
                Box::new(method.return_type.clone()),
            );
        }

        let available = nominal
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .chain(nominal.methods.iter().map(|method| method.name.as_str()))
            .collect::<Vec<_>>();
        let mut diagnostic = Diagnostic::error(
            "NOM001",
            format!("type `{}` has no member `{member}`", nominal.name),
        )
        .with_primary(member_span, "unknown member");
        if let Some(suggestion) = closest_name(&available, &member) {
            diagnostic = diagnostic.with_help(format!("did you mean `{suggestion}`?"));
        } else {
            diagnostic = diagnostic.with_help("use a declared field or method");
        }
        self.diagnostics.push(diagnostic);
        Type::Error
    }

    fn infer_record_expression(&mut self, node: &SyntaxNode) -> Type {
        let children = node.child_nodes().collect::<Vec<_>>();
        let Some(target) = children.first().copied() else {
            return Type::Error;
        };
        let type_name = significant_text(target, self.source);
        let Some(index) = self.nominal_lookup.get(&type_name).copied() else {
            self.diagnostics.push(
                Diagnostic::error(
                    "NOM009",
                    format!("cannot construct unknown nominal type `{type_name}`"),
                )
                .with_primary(target.span(), "no local record or struct has this name")
                .with_help("declare or import the type before constructing it"),
            );
            for initializer in children.iter().skip(1) {
                for expression in initializer.child_nodes() {
                    self.infer_expression(expression);
                }
            }
            return Type::Error;
        };
        let Some(nominal) = self.nominals.get(index).cloned() else {
            return Type::Error;
        };
        if nominal.kind == NominalKind::Enum {
            self.diagnostics.push(
                Diagnostic::error(
                    "NOM010",
                    format!("enum `{}` cannot be constructed with record syntax", nominal.name),
                )
                .with_primary(target.span(), "use a declared enum variant")
                .with_help(format!("construct a variant such as `{}.<variant>(...)`", nominal.name)),
            );
            return Type::Error;
        }

        let generic_names = nominal
            .generic_parameters
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        let mut initialized = HashSet::new();
        for initializer in children
            .iter()
            .skip(1)
            .copied()
            .filter(|child| child.kind() == SyntaxKind::RecordFieldInitializer)
        {
            let Some((field_name, field_span)) =
                first_direct_identifier(initializer, self.source)
            else {
                continue;
            };
            let expression = initializer
                .child_nodes()
                .find(|child| is_expression_kind(child.kind()));
            let actual = expression.map_or(Type::Unknown, |child| self.infer_expression(child));
            if !initialized.insert(field_name.clone()) {
                self.diagnostics.push(
                    Diagnostic::error(
                        "NOM005",
                        format!("field `{field_name}` is initialized more than once"),
                    )
                    .with_primary(field_span, "duplicate field initializer")
                    .with_help("remove one of the repeated initializers"),
                );
                continue;
            }
            let Some(field) = nominal.fields.iter().find(|field| field.name == field_name) else {
                self.diagnostics.push(
                    Diagnostic::error(
                        "NOM004",
                        format!("type `{}` has no field `{field_name}`", nominal.name),
                    )
                    .with_primary(field_span, "unknown constructor field")
                    .with_help("use one of the fields declared on the type"),
                );
                continue;
            };
            if !type_contains_generic(&field.ty, &generic_names) {
                if let Some(expression) = expression {
                    self.require_assignable(
                        &field.ty,
                        &actual,
                        expression.span(),
                        &format!("field `{field_name}`"),
                        "NOM006",
                    );
                }
            }
        }

        for field in &nominal.fields {
            if !field.has_default && !initialized.contains(&field.name) {
                self.diagnostics.push(
                    Diagnostic::error(
                        "NOM003",
                        format!("missing required field `{}` for `{}`", field.name, nominal.name),
                    )
                    .with_primary(node.span(), "record construction is incomplete")
                    .with_secondary(field.span, "required field declared here")
                    .with_help(format!("add `{}: <value>`", field.name)),
                );
            }
        }

        Type::Named(
            nominal.name,
            vec![Type::Unknown; nominal.generic_parameters.len()],
        )
    }

    fn is_mutable_place(&self, node: &SyntaxNode) -> bool {
        match node.kind() {
            SyntaxKind::NameExpression => {
                let name = significant_text(node, self.source);
                self.lookup_local(&name).is_some_and(|binding| {
                    binding.mutable
                        || matches!(
                            &binding.ty,
                            Type::Reference {
                                mutable: true,
                                ..
                            }
                        )
                })
            }
            SyntaxKind::MemberExpression => node
                .child_nodes()
                .next()
                .is_some_and(|base| self.is_mutable_place(base)),
            SyntaxKind::PrefixExpression => {
                significant_text(node, self.source).starts_with("&mut")
            }
            _ => false,
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

fn field_declaration_parts(
    node: &SyntaxNode,
    source: &SourceFile,
) -> Option<(String, Span, String, bool, bool)> {
    let tokens = descendant_tokens(node);
    let public = tokens
        .iter()
        .any(|token| token.kind() == TokenKind::Keyword(Keyword::Pub));
    let name_token = tokens
        .iter()
        .copied()
        .find(|token| token.kind() == TokenKind::Identifier)?;
    let name = name_token.text(source)?.to_owned();
    let colon = tokens.iter().position(|token| token.kind() == TokenKind::Colon)?;
    let mut type_text = String::new();
    let mut has_default = false;
    for token in &tokens[colon + 1..] {
        if token.kind() == TokenKind::Equal {
            has_default = true;
            break;
        }
        if let Some(text) = token.text(source) {
            if needs_type_space(&type_text, text) {
                type_text.push(' ');
            }
            type_text.push_str(text);
        }
    }
    if type_text.trim().is_empty() {
        return None;
    }
    Some((name, name_token.span(), type_text, has_default, public))
}

fn enum_variant_parts(
    node: &SyntaxNode,
    source: &SourceFile,
) -> Option<(String, Span, Vec<String>)> {
    let tokens = descendant_tokens(node);
    let name_token = tokens
        .iter()
        .copied()
        .find(|token| token.kind() == TokenKind::Identifier)?;
    let name = name_token.text(source)?.to_owned();
    let name_index = tokens
        .iter()
        .position(|token| token.span() == name_token.span())?;
    let remaining = &tokens[name_index + 1..];
    let open = remaining
        .iter()
        .position(|token| token.kind() == TokenKind::LeftParen);
    let Some(open) = open else {
        return Some((name, name_token.span(), Vec::new()));
    };
    let close = remaining
        .iter()
        .rposition(|token| token.kind() == TokenKind::RightParen);
    let Some(close) = close else {
        return Some((name, name_token.span(), Vec::new()));
    };
    if close <= open {
        return Some((name, name_token.span(), Vec::new()));
    }
    let mut payload = Vec::new();
    let mut current = String::new();
    let mut angle_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    for token in &remaining[open + 1..close] {
        match token.kind() {
            TokenKind::Less => angle_depth += 1,
            TokenKind::Greater if angle_depth > 0 => angle_depth -= 1,
            TokenKind::ShiftRight if angle_depth > 0 => {
                angle_depth = angle_depth.saturating_sub(2);
            }
            TokenKind::LeftParen => paren_depth += 1,
            TokenKind::RightParen if paren_depth > 0 => paren_depth -= 1,
            TokenKind::LeftBracket => bracket_depth += 1,
            TokenKind::RightBracket if bracket_depth > 0 => bracket_depth -= 1,
            TokenKind::Comma if angle_depth == 0 && paren_depth == 0 && bracket_depth == 0 => {
                if !current.trim().is_empty() {
                    payload.push(current.trim().to_owned());
                }
                current.clear();
                continue;
            }
            _ => {}
        }
        if let Some(text) = token.text(source) {
            if needs_type_space(&current, text) {
                current.push(' ');
            }
            current.push_str(text);
        }
    }
    if !current.trim().is_empty() {
        payload.push(current.trim().to_owned());
    }
    Some((name, name_token.span(), payload))
}

fn impl_header(node: &SyntaxNode, source: &SourceFile) -> (Option<String>, Option<String>) {
    let tokens = significant_direct_tokens(node);
    let identifiers = tokens
        .iter()
        .filter(|token| token.kind() == TokenKind::Identifier)
        .filter_map(|token| token.text(source).map(str::to_owned))
        .collect::<Vec<_>>();
    let has_for = tokens
        .iter()
        .any(|token| token.kind() == TokenKind::Keyword(Keyword::For));
    if has_for && identifiers.len() >= 2 {
        (
            identifiers.last().cloned(),
            identifiers.first().cloned(),
        )
    } else {
        (identifiers.first().cloned(), None)
    }
}

fn replace_self_type(ty: Type, owner: &str) -> Type {
    match ty {
        Type::Named(name, arguments) if name == "Self" && arguments.is_empty() => {
            Type::Named(owner.to_owned(), Vec::new())
        }
        Type::Named(name, arguments) => Type::Named(
            name,
            arguments
                .into_iter()
                .map(|argument| replace_self_type(argument, owner))
                .collect(),
        ),
        Type::Optional(inner) => {
            Type::Optional(Box::new(replace_self_type(*inner, owner)))
        }
        Type::Reference { mutable, inner } => Type::Reference {
            mutable,
            inner: Box::new(replace_self_type(*inner, owner)),
        },
        Type::Pointer { mutable, inner } => Type::Pointer {
            mutable,
            inner: Box::new(replace_self_type(*inner, owner)),
        },
        Type::Tuple(items) => Type::Tuple(
            items
                .into_iter()
                .map(|item| replace_self_type(item, owner))
                .collect(),
        ),
        Type::Function(parameters, result) => Type::Function(
            parameters
                .into_iter()
                .map(|parameter| replace_self_type(parameter, owner))
                .collect(),
            Box::new(replace_self_type(*result, owner)),
        ),
        other => other,
    }
}

fn erase_generic_parameters(ty: Type, generics: &HashSet<String>) -> Type {
    match ty {
        Type::Named(name, arguments) if generics.contains(&name) && arguments.is_empty() => {
            Type::Unknown
        }
        Type::Named(name, arguments) => Type::Named(
            name,
            arguments
                .into_iter()
                .map(|argument| erase_generic_parameters(argument, generics))
                .collect(),
        ),
        Type::Optional(inner) => {
            Type::Optional(Box::new(erase_generic_parameters(*inner, generics)))
        }
        Type::Reference { mutable, inner } => Type::Reference {
            mutable,
            inner: Box::new(erase_generic_parameters(*inner, generics)),
        },
        Type::Pointer { mutable, inner } => Type::Pointer {
            mutable,
            inner: Box::new(erase_generic_parameters(*inner, generics)),
        },
        Type::Tuple(items) => Type::Tuple(
            items
                .into_iter()
                .map(|item| erase_generic_parameters(item, generics))
                .collect(),
        ),
        Type::Function(parameters, result) => Type::Function(
            parameters
                .into_iter()
                .map(|parameter| erase_generic_parameters(parameter, generics))
                .collect(),
            Box::new(erase_generic_parameters(*result, generics)),
        ),
        other => other,
    }
}

fn type_contains_generic(ty: &Type, generics: &HashSet<String>) -> bool {
    match ty {
        Type::Named(name, arguments) => {
            (generics.contains(name) && arguments.is_empty())
                || arguments
                    .iter()
                    .any(|argument| type_contains_generic(argument, generics))
        }
        Type::Optional(inner)
        | Type::Reference { inner, .. }
        | Type::Pointer { inner, .. } => type_contains_generic(inner, generics),
        Type::Tuple(items) => items
            .iter()
            .any(|item| type_contains_generic(item, generics)),
        Type::Function(parameters, result) => {
            parameters
                .iter()
                .any(|parameter| type_contains_generic(parameter, generics))
                || type_contains_generic(result, generics)
        }
        _ => false,
    }
}

fn nominal_name_from_type(ty: &Type) -> Option<String> {
    match ty {
        Type::Named(name, _) => Some(name.clone()),
        Type::Reference { inner, .. } => nominal_name_from_type(inner),
        _ => None,
    }
}

fn member_name(node: &SyntaxNode, source: &SourceFile) -> Option<(String, Span)> {
    significant_direct_tokens(node)
        .into_iter()
        .rev()
        .find(|token| {
            matches!(
                token.kind(),
                TokenKind::Identifier | TokenKind::Keyword(_)
            )
        })
        .and_then(|token| token.text(source).map(|text| (text.to_owned(), token.span())))
}

fn closest_name<'a>(candidates: &[&'a str], requested: &str) -> Option<&'a str> {
    candidates
        .iter()
        .copied()
        .map(|candidate| (edit_distance(candidate, requested), candidate))
        .filter(|(distance, candidate)| {
            *distance <= 3 || *distance * 2 <= candidate.len().max(requested.len())
        })
        .min_by_key(|(distance, candidate)| (*distance, candidate.len()))
        .map(|(_, candidate)| candidate)
}

fn edit_distance(left: &str, right: &str) -> usize {
    let right_chars = right.chars().collect::<Vec<_>>();
    let mut previous = (0..=right_chars.len()).collect::<Vec<_>>();
    for (left_index, left_char) in left.chars().enumerate() {
        let mut current = vec![left_index + 1];
        for (right_index, right_char) in right_chars.iter().enumerate() {
            let deletion = previous[right_index + 1] + 1;
            let insertion = current[right_index] + 1;
            let substitution =
                previous[right_index] + usize::from(left_char != *right_char);
            current.push(deletion.min(insertion).min(substitution));
        }
        previous = current;
    }
    previous[right_chars.len()]
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
    }    #[test]
    fn indexes_record_fields_and_methods() {
        let result = check_text(
            "module test\nrecord User {\n name: String\n age: Int = 0\n}\nimpl User {\n fn label(self: &Self) -> String { self.name }\n}\nfn main() { let user = User { name: \"M\" }\n let label = user.label()\n let age = user.age\n }\n",
        );
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        let user = result
            .nominals
            .iter()
            .find(|nominal| nominal.name == "User")
            .unwrap_or_else(|| panic!("User nominal missing"));
        assert_eq!(user.fields.len(), 2);
        assert!(user.methods.iter().any(|method| method.name == "label"));
        assert!(result.bindings.iter().any(|binding| {
            binding.name == "label" && binding.ty == Type::String
        }));
    }

    #[test]
    fn accepts_mutable_field_assignment_and_mutating_method() {
        let result = check_text(
            "module test\nrecord User {\n name: String\n}\nimpl User {\n fn rename(self: &mut Self, name: String) -> Unit { self.name = name }\n}\nfn main() { var user = User { name: \"before\" }\n user.rename(\"after\")\n user.name = \"done\"\n }\n",
        );
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
    }

    #[test]
    fn rejects_missing_record_field() {
        let result = check_text(
            "module test\nrecord Pair {\n left: Int\n right: Int\n}\nfn main() { let pair = Pair { left: 1 } }\n",
        );
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "NOM003"));
    }

    #[test]
    fn rejects_unknown_record_field() {
        let result = check_text(
            "module test\nrecord User {\n name: String\n}\nfn main() { let user = User { title: \"x\", name: \"M\" } }\n",
        );
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "NOM004"));
    }

    #[test]
    fn rejects_duplicate_record_initializer() {
        let result = check_text(
            "module test\nrecord User {\n name: String\n}\nfn main() { let user = User { name: \"A\", name: \"B\" } }\n",
        );
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "NOM005"));
    }

    #[test]
    fn rejects_wrong_record_field_type() {
        let result = check_text(
            "module test\nrecord User {\n age: Int\n}\nfn main() { let user = User { age: \"young\" } }\n",
        );
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "NOM006"));
    }

    #[test]
    fn resolves_enum_unit_and_payload_variants() {
        let result = check_text(
            "module test\nenum State {\n idle\n ready(String)\n}\nfn main() { let first = State.idle\n let second = State.ready(\"done\")\n }\n",
        );
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert!(result.bindings.iter().any(|binding| {
            binding.name == "first"
                && binding.ty == Type::Named("State".to_owned(), Vec::new())
        }));
        assert!(result.bindings.iter().any(|binding| {
            binding.name == "second"
                && binding.ty == Type::Named("State".to_owned(), Vec::new())
        }));
    }

    #[test]
    fn rejects_wrong_enum_payload_type() {
        let result = check_text(
            "module test\nenum State {\n ready(String)\n}\nfn main() { let state = State.ready(1) }\n",
        );
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "NOM007"));
    }

    #[test]
    fn rejects_unknown_member_with_suggestion() {
        let result = check_text(
            "module test\nrecord User {\n name: String\n}\nfn main() { let user = User { name: \"M\" }\n let value = user.nam\n }\n",
        );
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "NOM001")
            .unwrap_or_else(|| panic!("NOM001 missing"));
        assert!(diagnostic.help.iter().any(|help| help.contains("name")));
    }

    #[test]
    fn rejects_unknown_enum_variant_with_suggestion() {
        let result = check_text(
            "module test\nenum State { ready(String) }\nfn main() { let state = State.redy("done") }\n",
        );
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "NOM001")
            .unwrap_or_else(|| panic!("NOM001 missing"));
        assert!(diagnostic.help.iter().any(|help| help.contains("ready")));
    }

    #[test]
    fn rejects_optional_member_access() {
        let result = check_text(
            "module test\nrecord User {\n name: String\n}\nfn main() { let user: User? = none\n let name = user.name\n }\n",
        );
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "NOM002"));
    }

    #[test]
    fn rejects_mutation_through_immutable_record() {
        let result = check_text(
            "module test\nrecord User {\n name: String\n}\nfn main() { let user = User { name: \"M\" }\n user.name = \"N\"\n }\n",
        );
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "NOM008"));
    }

    #[test]
    fn rejects_unknown_constructor_target() {
        let result = check_text(
            "module test\nfn main() { let value = Missing { name: \"x\" } }\n",
        );
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "NOM009"));
    }

    #[test]
    fn rejects_record_syntax_for_enum() {
        let result = check_text(
            "module test\nenum State { idle }\nfn main() { let value = State { } }\n",
        );
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "NOM010"));
    }

    #[test]
    fn checks_method_bodies_against_their_signatures() {
        let result = check_text(
            "module test\nrecord User { name: String }\nimpl User { fn broken(self: &Self) -> Int { self.name } }\n",
        );
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "TYP005"));
    }
}

