//! Type representation and first static type-checking pass for Nivra Edition 2026.
//!
//! D8 extends the nominal checker with generic substitution, trait constraints,
//! implementation validation, and deterministic method selection. Unknown types remain
//! recovery values so one unsupported operation does not produce cascading diagnostics.

use std::collections::{HashMap, HashSet};
use std::fmt::{self, Write as _};

use nivra_diagnostics::Diagnostic;
use nivra_lexer::{Keyword, TokenKind};
use nivra_sema::{Namespace, SemanticResult, SymbolKind};
use nivra_source::{SourceFile, Span};
use nivra_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

/// Static type used by the D8 checker.
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
    Parameter(String),
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
            Self::Parameter(name) => formatter.write_str(name),
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

/// One declared generic parameter and its trait bounds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenericParameterInfo {
    pub name: String,
    pub bounds: Vec<String>,
    pub span: Span,
}

/// One normalized trait constraint such as `T: Display`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraitConstraint {
    pub parameter: String,
    pub trait_name: String,
    pub span: Span,
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
    pub owner_type: Option<Type>,
    pub trait_name: Option<String>,
    pub generic_parameters: Vec<GenericParameterInfo>,
    pub constraints: Vec<TraitConstraint>,
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
    pub owner_type: Type,
    pub generic_parameters: Vec<GenericParameterInfo>,
    pub constraints: Vec<TraitConstraint>,
}

/// One method declared by a trait.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraitMethodInfo {
    pub name: String,
    pub parameters: Vec<ParameterType>,
    pub return_type: Type,
    pub span: Span,
    pub mutable_receiver: bool,
    pub has_default: bool,
}

/// Indexed local trait declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraitInfo {
    pub name: String,
    pub generic_parameters: Vec<String>,
    pub constraints: Vec<TraitConstraint>,
    pub methods: Vec<TraitMethodInfo>,
    pub span: Span,
}

/// Indexed inherent or trait implementation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImplementationInfo {
    pub trait_name: Option<String>,
    pub target: Type,
    pub generic_parameters: Vec<String>,
    pub constraints: Vec<TraitConstraint>,
    pub methods: Vec<MethodInfo>,
    pub span: Span,
}

/// Indexed nominal type body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NominalTypeInfo {
    pub name: String,
    pub kind: NominalKind,
    pub generic_parameters: Vec<String>,
    pub constraints: Vec<TraitConstraint>,
    pub fields: Vec<FieldInfo>,
    pub variants: Vec<VariantInfo>,
    pub methods: Vec<MethodInfo>,
    pub span: Span,
}

/// Complete D8 type-check result.
#[derive(Clone, Debug)]
pub struct TypeCheckResult {
    pub functions: Vec<FunctionSignature>,
    pub nominals: Vec<NominalTypeInfo>,
    pub traits: Vec<TraitInfo>,
    pub implementations: Vec<ImplementationInfo>,
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
                let _ = write!(output, "fn {owner}.{}", signature.name);
            } else {
                let _ = write!(output, "fn {}", signature.name);
            }
            if !signature.generic_parameters.is_empty() {
                let generics = signature
                    .generic_parameters
                    .iter()
                    .map(|parameter| {
                        if parameter.bounds.is_empty() {
                            parameter.name.clone()
                        } else {
                            format!("{}: {}", parameter.name, parameter.bounds.join(" + "))
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let _ = write!(output, "<{generics}>");
            }
            output.push('(');
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
            let _ = writeln!(
                output,
                " @ {}..{}",
                nominal.span.start(),
                nominal.span.end()
            );
            for field in &nominal.fields {
                let _ = writeln!(
                    output,
                    "  field {}: {}{}{}",
                    field.name,
                    field.ty,
                    if field.has_default {
                        " = <default>"
                    } else {
                        ""
                    },
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
                    if method.mutable_receiver {
                        " mut-self"
                    } else {
                        ""
                    },
                    method
                        .trait_name
                        .as_ref()
                        .map_or(String::new(), |name| format!(" trait={name}"))
                );
            }
        }
        output
    }

    /// Produces a deterministic trait and implementation report.
    #[must_use]
    pub fn trait_report(&self) -> String {
        let mut output = String::new();
        for trait_info in &self.traits {
            let _ = write!(output, "trait {}", trait_info.name);
            if !trait_info.generic_parameters.is_empty() {
                let _ = write!(
                    output,
                    "<{}>",
                    trait_info.generic_parameters.join(", ")
                );
            }
            let _ = writeln!(
                output,
                " @ {}..{}",
                trait_info.span.start(),
                trait_info.span.end()
            );
            for method in &trait_info.methods {
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
                    if method.mutable_receiver {
                        " mut-self"
                    } else {
                        ""
                    },
                    if method.has_default { " default" } else { " required" }
                );
            }
        }
        for implementation in &self.implementations {
            match &implementation.trait_name {
                Some(trait_name) => {
                    let _ = writeln!(output, "impl {trait_name} for {}", implementation.target);
                }
                None => {
                    let _ = writeln!(output, "impl {}", implementation.target);
                }
            }
            for constraint in &implementation.constraints {
                let _ = writeln!(
                    output,
                    "  where {}: {}",
                    constraint.parameter,
                    constraint.trait_name
                );
            }
        }
        output
    }
}

/// Runs D8 type checking after parsing and name resolution have succeeded.
#[must_use]
pub fn check(source: &SourceFile, root: &SyntaxNode, semantic: &SemanticResult) -> TypeCheckResult {
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
    imported_types: HashSet<String>,
    functions: Vec<FunctionSignature>,
    function_lookup: HashMap<String, usize>,
    nominals: Vec<NominalTypeInfo>,
    nominal_lookup: HashMap<String, usize>,
    traits: Vec<TraitInfo>,
    trait_lookup: HashMap<String, usize>,
    implementations: Vec<ImplementationInfo>,
    constants: HashMap<String, Type>,
    scopes: Vec<HashMap<String, LocalBinding>>,
    bindings: Vec<BindingType>,
    expressions: Vec<TypedExpression>,
    diagnostics: Vec<Diagnostic>,
    expected_return: Type,
    saw_explicit_return: bool,
    active_constraints: Vec<TraitConstraint>,
}

impl<'a> Checker<'a> {
    fn new(source: &'a SourceFile, semantic: &SemanticResult) -> Self {
        let imported_types = semantic
            .symbols
            .iter()
            .filter(|symbol| {
                symbol.namespace == Namespace::Type && symbol.kind == SymbolKind::Import
            })
            .map(|symbol| symbol.name.clone())
            .collect::<HashSet<_>>();
        let mut known_types = builtin_type_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect::<HashSet<_>>();
        for symbol in &semantic.symbols {
            if symbol.namespace == Namespace::Type && !matches!(symbol.kind, SymbolKind::Import) {
                let _ = known_types.insert(symbol.name.clone());
            }
            if symbol.namespace == Namespace::Type && symbol.kind == SymbolKind::Import {
                let _ = known_types.insert(symbol.name.clone());
            }
        }
        Self {
            source,
            known_types,
            imported_types,
            functions: Vec::new(),
            function_lookup: HashMap::new(),
            nominals: Vec::new(),
            nominal_lookup: HashMap::new(),
            traits: Vec::new(),
            trait_lookup: HashMap::new(),
            implementations: Vec::new(),
            constants: HashMap::new(),
            scopes: Vec::new(),
            bindings: Vec::new(),
            expressions: Vec::new(),
            diagnostics: Vec::new(),
            expected_return: Type::Unit,
            saw_explicit_return: false,
            active_constraints: Vec::new(),
        }
    }

    fn run(mut self, root: &SyntaxNode) -> TypeCheckResult {
        self.collect_nominals(root);
        self.collect_traits(root);
        self.collect_signatures(root);
        self.collect_implementations(root);
        self.validate_generic_constraints();
        self.validate_implementations();
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
            traits: self.traits,
            implementations: self.implementations,
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
            let generic_infos = generic_parameter_infos(node, self.source);
            self.report_duplicate_generic_parameters(&generic_infos);
            let generic_parameters = generic_infos
                .iter()
                .map(|parameter| parameter.name.clone())
                .collect::<Vec<_>>();
            let generic_set = generic_parameters.iter().cloned().collect::<HashSet<_>>();
            let constraints = declaration_constraints(node, self.source, &generic_infos);
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
                            let ty =
                                self.parse_declared_type(&type_text, field.span(), &generic_set);
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
                constraints,
                fields,
                variants,
                methods: Vec::new(),
                span,
            });
        }
    }

    fn collect_traits(&mut self, root: &SyntaxNode) {
        for node in root
            .child_nodes()
            .filter(|child| child.kind() == SyntaxKind::TraitDeclaration)
        {
            let Some((name, span)) = first_direct_identifier(node, self.source) else {
                continue;
            };
            let generic_infos = generic_parameter_infos(node, self.source);
            self.report_duplicate_generic_parameters(&generic_infos);
            if !generic_infos.is_empty() {
                self.diagnostics.push(
                    Diagnostic::error(
                        "GEN006",
                        "generic trait declarations are deferred beyond D8",
                    )
                    .with_primary(node.span(), "generic trait parameters are not enabled")
                    .with_help(
                        "use a non-generic trait with generic functions or nominal types in D8",
                    ),
                );
            }
            let generic_parameters = generic_infos
                .iter()
                .map(|parameter| parameter.name.clone())
                .collect::<Vec<_>>();
            let constraints = declaration_constraints(node, self.source, &generic_infos);
            let generic_names = generic_parameters.iter().cloned().collect::<HashSet<_>>();
            let methods = node
                .child_nodes()
                .filter(|child| child.kind() == SyntaxKind::FunctionDeclaration)
                .filter_map(|method| self.trait_method_from_node(method, &generic_names))
                .collect::<Vec<_>>();
            let index = self.traits.len();
            let _ = self.trait_lookup.insert(name.clone(), index);
            self.traits.push(TraitInfo {
                name,
                generic_parameters,
                constraints,
                methods,
                span,
            });
        }
    }

    fn trait_method_from_node(
        &mut self,
        node: &SyntaxNode,
        inherited_generics: &HashSet<String>,
    ) -> Option<TraitMethodInfo> {
        let (name, _) = function_name(node, self.source)?;
        let mut generic_names = inherited_generics.clone();
        let _ = generic_names.insert("Self".to_owned());
        let method_generics = generic_parameter_infos(node, self.source);
        if !method_generics.is_empty() {
            self.diagnostics.push(
                Diagnostic::error(
                    "GEN006",
                    "generic trait methods are deferred beyond D8",
                )
                .with_primary(node.span(), "generic trait methods are not enabled")
                .with_help("move the generic parameter to a free function or nominal impl"),
            );
        }
        for parameter in method_generics {
            let _ = generic_names.insert(parameter.name);
        }
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
        let receiver = parameters
            .first()
            .filter(|parameter| parameter.name == "self");
        let mutable_receiver = receiver.is_some_and(|parameter| {
            matches!(&parameter.ty, Type::Reference { mutable: true, .. })
        });
        if receiver.is_some() {
            parameters.remove(0);
        }
        let return_type = node
            .child_nodes()
            .find(|child| child.kind() == SyntaxKind::TypeReference)
            .map_or(Type::Unit, |type_node| {
                self.parse_type_node(type_node, &generic_names)
            });
        Some(TraitMethodInfo {
            name,
            parameters,
            return_type,
            span: node.span(),
            mutable_receiver,
            has_default: node
                .child_nodes()
                .any(|child| child.kind() == SyntaxKind::Block),
        })
    }

    fn collect_implementations(&mut self, root: &SyntaxNode) {
        for node in root
            .child_nodes()
            .filter(|child| child.kind() == SyntaxKind::ImplDeclaration)
        {
            let header = impl_header_parts(node, self.source);
            self.report_duplicate_generic_parameters(&header.generic_parameters);
            let generic_names = header
                .generic_parameters
                .iter()
                .map(|parameter| parameter.name.clone())
                .collect::<HashSet<_>>();
            let target =
                self.parse_declared_type(&header.target_text, node.span(), &generic_names);
            let mut methods = Vec::new();
            for function in node
                .child_nodes()
                .filter(|child| child.kind() == SyntaxKind::FunctionDeclaration)
            {
                let signature = self
                    .functions
                    .iter()
                    .find(|signature| signature.span == function.span())
                    .cloned();
                let Some(signature) = signature else {
                    continue;
                };
                let receiver = signature
                    .parameters
                    .first()
                    .filter(|parameter| parameter.name == "self");
                let mutable_receiver = receiver.is_some_and(|parameter| {
                    matches!(&parameter.ty, Type::Reference { mutable: true, .. })
                });
                let parameters = if receiver.is_some() {
                    signature.parameters.iter().skip(1).cloned().collect()
                } else {
                    signature.parameters.clone()
                };
                methods.push(MethodInfo {
                    name: signature.name,
                    parameters,
                    return_type: signature.return_type,
                    span: signature.span,
                    mutable_receiver,
                    trait_name: signature.trait_name,
                    owner_type: signature.owner_type.unwrap_or_else(|| target.clone()),
                    generic_parameters: signature.generic_parameters,
                    constraints: signature.constraints,
                });
            }
            self.implementations.push(ImplementationInfo {
                trait_name: header.trait_name,
                target,
                generic_parameters: header
                    .generic_parameters
                    .iter()
                    .map(|parameter| parameter.name.clone())
                    .collect(),
                constraints: header.constraints,
                methods,
                span: node.span(),
            });
        }
    }

    fn validate_generic_constraints(&mut self) {
        let mut declarations = Vec::new();
        for nominal in &self.nominals {
            declarations.push((
                nominal.generic_parameters.clone(),
                nominal.constraints.clone(),
            ));
        }
        for trait_info in &self.traits {
            declarations.push((
                trait_info.generic_parameters.clone(),
                trait_info.constraints.clone(),
            ));
        }
        for signature in &self.functions {
            let names = signature
                .generic_parameters
                .iter()
                .map(|parameter| parameter.name.clone())
                .collect::<Vec<_>>();
            declarations.push((names, signature.constraints.clone()));
        }
        for implementation in &self.implementations {
            declarations.push((
                implementation.generic_parameters.clone(),
                implementation.constraints.clone(),
            ));
        }

        let mut seen = HashSet::new();
        for (parameters, constraints) in declarations {
            let names = parameters.into_iter().collect::<HashSet<_>>();
            for constraint in constraints {
                let key = (
                    constraint.span.start(),
                    constraint.parameter.clone(),
                    constraint.trait_name.clone(),
                );
                if !seen.insert(key) {
                    continue;
                }
                if constraint.parameter != "Self" && !names.contains(&constraint.parameter) {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "GEN005",
                            format!(
                                "constraint refers to undeclared generic parameter `{}`",
                                constraint.parameter
                            ),
                        )
                        .with_primary(constraint.span, "unknown generic parameter")
                        .with_help("declare the parameter in `<...>` or correct the where clause"),
                    );
                }
                if !self.trait_lookup.contains_key(&constraint.trait_name)
                    && !self.imported_types.contains(&constraint.trait_name)
                {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "TRT001",
                            format!("unknown trait `{}`", constraint.trait_name),
                        )
                        .with_primary(constraint.span, "this trait is not declared or imported")
                        .with_help("declare the trait, import it, or correct its spelling"),
                    );
                }
            }
        }

        let mut checked = HashSet::new();
        let nominals = self.nominals.clone();
        for nominal in nominals {
            for field in nominal.fields {
                if checked.insert((field.span.start(), field.ty.display_name())) {
                    self.validate_type_arity(&field.ty, field.span);
                }
            }
            for variant in nominal.variants {
                for payload in variant.payload {
                    if checked.insert((variant.span.start(), payload.display_name())) {
                        self.validate_type_arity(&payload, variant.span);
                    }
                }
            }
        }
        let functions = self.functions.clone();
        for signature in functions {
            for parameter in signature.parameters {
                if checked.insert((parameter.span.start(), parameter.ty.display_name())) {
                    self.validate_type_arity(&parameter.ty, parameter.span);
                }
            }
            if checked.insert((signature.span.start(), signature.return_type.display_name())) {
                self.validate_type_arity(&signature.return_type, signature.span);
            }
        }
        let implementations = self.implementations.clone();
        for implementation in implementations {
            if checked.insert((implementation.span.start(), implementation.target.display_name())) {
                self.validate_type_arity(&implementation.target, implementation.span);
            }
        }
    }

    fn validate_implementations(&mut self) {
        let implementations = self.implementations.clone();
        let mut seen = HashMap::<String, Span>::new();
        for implementation in implementations {
            let Some(trait_name) = implementation.trait_name.clone() else {
                continue;
            };
            let target_is_local = nominal_name_from_type(&implementation.target)
                .is_some_and(|name| self.nominal_lookup.contains_key(&name));
            let trait_is_local = self.trait_lookup.contains_key(&trait_name);
            let trait_is_imported = self.imported_types.contains(&trait_name);

            if !trait_is_local && !trait_is_imported {
                self.diagnostics.push(
                    Diagnostic::error("TRT001", format!("unknown trait `{trait_name}`"))
                        .with_primary(implementation.span, "this implementation names no known trait")
                        .with_help("declare or import the trait before implementing it"),
                );
                continue;
            }
            if !trait_is_local && !target_is_local {
                self.diagnostics.push(
                    Diagnostic::error(
                        "TRT006",
                        format!(
                            "implementation of external trait `{trait_name}` for external type `{}` violates the orphan rule",
                            implementation.target
                        ),
                    )
                    .with_primary(implementation.span, "neither side belongs to this package")
                    .with_help("implement a local trait or target a local nominal type"),
                );
            }

            let key = format!(
                "{}|{}",
                trait_name,
                canonical_type_pattern(
                    &implementation.target,
                    &implementation.generic_parameters
                )
            );
            if let Some(previous) = seen.insert(key, implementation.span) {
                self.diagnostics.push(
                    Diagnostic::error(
                        "TRT002",
                        format!(
                            "conflicting implementation of `{trait_name}` for `{}`",
                            implementation.target
                        ),
                    )
                    .with_primary(implementation.span, "overlapping implementation")
                    .with_secondary(previous, "previous implementation declared here")
                    .with_help("keep exactly one applicable implementation"),
                );
            }

            let Some(trait_index) = self.trait_lookup.get(&trait_name).copied() else {
                continue;
            };
            let Some(trait_info) = self.traits.get(trait_index).cloned() else {
                continue;
            };
            for required in trait_info.methods {
                let implemented = implementation
                    .methods
                    .iter()
                    .find(|method| method.name == required.name);
                let Some(implemented) = implemented else {
                    if !required.has_default {
                        self.diagnostics.push(
                            Diagnostic::error(
                                "TRT003",
                                format!(
                                    "implementation of `{trait_name}` is missing required method `{}`",
                                    required.name
                                ),
                            )
                            .with_primary(implementation.span, "required method is absent")
                            .with_secondary(required.span, "method required by this trait")
                            .with_help(format!("implement `fn {}(...)`", required.name)),
                        );
                    }
                    continue;
                };
                let mut substitutions = HashMap::new();
                let _ = substitutions.insert("Self".to_owned(), implementation.target.clone());
                let expected_parameters = required
                    .parameters
                    .iter()
                    .map(|parameter| substitute_type(&parameter.ty, &substitutions))
                    .collect::<Vec<_>>();
                let expected_return = substitute_type(&required.return_type, &substitutions);
                let parameter_match = expected_parameters.len() == implemented.parameters.len()
                    && expected_parameters
                        .iter()
                        .zip(implemented.parameters.iter())
                        .all(|(expected, actual)| {
                            types_equivalent_strict(expected, &actual.ty)
                        });
                if required.mutable_receiver != implemented.mutable_receiver
                    || !parameter_match
                    || !types_equivalent_strict(&expected_return, &implemented.return_type)
                {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "TRT004",
                            format!(
                                "method `{}` does not match trait `{trait_name}`",
                                required.name
                            ),
                        )
                        .with_primary(implemented.span, "incompatible implementation signature")
                        .with_secondary(required.span, "trait method declared here")
                        .with_note(format!(
                            "expected ({}) -> {expected_return}",
                            expected_parameters
                                .iter()
                                .map(Type::display_name)
                                .collect::<Vec<_>>()
                                .join(", ")
                        ))
                        .with_help("match the receiver, parameter, and return types exactly"),
                    );
                }
            }
        }
    }

    fn report_duplicate_generic_parameters(&mut self, parameters: &[GenericParameterInfo]) {
        let mut seen = HashMap::<String, Span>::new();
        for parameter in parameters {
            if let Some(previous) = seen.insert(parameter.name.clone(), parameter.span) {
                self.diagnostics.push(
                    Diagnostic::error(
                        "GEN005",
                        format!("generic parameter `{}` is declared more than once", parameter.name),
                    )
                    .with_primary(parameter.span, "duplicate generic parameter")
                    .with_secondary(previous, "first declaration is here")
                    .with_help("rename or remove one of the duplicate parameters"),
                );
            }
        }
    }

    fn validate_type_arity(&mut self, ty: &Type, span: Span) {
        match ty {
            Type::Named(name, arguments) => {
                if let Some(expected) = self.expected_generic_arity(name) {
                    if expected != arguments.len() {
                        self.diagnostics.push(
                            Diagnostic::error(
                                "GEN001",
                                format!(
                                    "type `{name}` expects {expected} generic argument(s), found {}",
                                    arguments.len()
                                ),
                            )
                            .with_primary(span, "wrong generic argument count")
                            .with_help(format!(
                                "write `{name}<...>` with exactly {expected} argument(s)"
                            )),
                        );
                    }
                }
                for argument in arguments {
                    self.validate_type_arity(argument, span);
                }
            }
            Type::Optional(inner)
            | Type::Reference { inner, .. }
            | Type::Pointer { inner, .. } => self.validate_type_arity(inner, span),
            Type::Tuple(items) => {
                for item in items {
                    self.validate_type_arity(item, span);
                }
            }
            Type::Function(parameters, result) => {
                for parameter in parameters {
                    self.validate_type_arity(parameter, span);
                }
                self.validate_type_arity(result, span);
            }
            _ => {}
        }
    }

    fn expected_generic_arity(&self, name: &str) -> Option<usize> {
        if let Some(index) = self.nominal_lookup.get(name).copied() {
            return self
                .nominals
                .get(index)
                .map(|nominal| nominal.generic_parameters.len());
        }
        builtin_generic_arity(name)
    }

    fn collect_signatures(&mut self, root: &SyntaxNode) {
        for node in root.child_nodes() {
            match node.kind() {
                SyntaxKind::FunctionDeclaration => {
                    self.collect_one_signature(
                        node,
                        false,
                        None,
                        None,
                        None,
                        &[],
                        &[],
                        true,
                    );
                }
                SyntaxKind::ExternBlock => {
                    for function in node.child_nodes() {
                        if function.kind() == SyntaxKind::ExternFunction {
                            self.collect_one_signature(
                                function,
                                true,
                                None,
                                None,
                                None,
                                &[],
                                &[],
                                true,
                            );
                        }
                    }
                }
                SyntaxKind::TraitDeclaration => {
                    let owner = first_direct_identifier(node, self.source).map(|(name, _)| name);
                    let generic_parameters = generic_parameter_infos(node, self.source);
                    let mut constraints =
                        declaration_constraints(node, self.source, &generic_parameters);
                    if let Some(trait_name) = &owner {
                        constraints.push(TraitConstraint {
                            parameter: "Self".to_owned(),
                            trait_name: trait_name.clone(),
                            span: node.span(),
                        });
                    }
                    deduplicate_constraints(&mut constraints);
                    for function in node.child_nodes() {
                        if function.kind() == SyntaxKind::FunctionDeclaration {
                            self.collect_one_signature(
                                function,
                                false,
                                owner.clone(),
                                Some(Type::Parameter("Self".to_owned())),
                                owner.clone(),
                                &generic_parameters,
                                &constraints,
                                false,
                            );
                        }
                    }
                }
                SyntaxKind::ImplDeclaration => {
                    let header = impl_header_parts(node, self.source);
                    let generic_names = header
                        .generic_parameters
                        .iter()
                        .map(|parameter| parameter.name.clone())
                        .collect::<HashSet<_>>();
                    let target_type = self.parse_declared_type(
                        &header.target_text,
                        node.span(),
                        &generic_names,
                    );
                    let owner = nominal_name_from_type(&target_type);
                    for function in node.child_nodes() {
                        if function.kind() == SyntaxKind::FunctionDeclaration {
                            self.collect_one_signature(
                                function,
                                false,
                                owner.clone(),
                                Some(target_type.clone()),
                                header.trait_name.clone(),
                                &header.generic_parameters,
                                &header.constraints,
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
        owner_type: Option<Type>,
        trait_name: Option<String>,
        inherited_generics: &[GenericParameterInfo],
        inherited_constraints: &[TraitConstraint],
        top_level: bool,
    ) {
        if let Some(signature) = self.signature_from_node(
            node,
            is_extern,
            owner,
            owner_type,
            trait_name,
            inherited_generics,
            inherited_constraints,
        ) {
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
        owner_type: Option<Type>,
        trait_name: Option<String>,
        inherited_generics: &[GenericParameterInfo],
        inherited_constraints: &[TraitConstraint],
    ) -> Option<FunctionSignature> {
        let (name, _name_span) = function_name(node, self.source)?;
        let mut generic_parameters = inherited_generics.to_vec();
        let own_generics = generic_parameter_infos(node, self.source);
        self.report_duplicate_generic_parameters(&own_generics);
        generic_parameters.extend(own_generics);
        let mut constraints = inherited_constraints.to_vec();
        constraints.extend(declaration_constraints(
            node,
            self.source,
            &generic_parameters,
        ));
        deduplicate_constraints(&mut constraints);
        let generic_names = generic_parameters
            .iter()
            .map(|parameter| parameter.name.clone())
            .collect::<HashSet<_>>();
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
                if let Some(owner_ty) = &owner_type {
                    ty = replace_self_type(ty, owner_ty);
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
        if let Some(owner_ty) = &owner_type {
            return_type = replace_self_type(return_type, owner_ty);
        }
        let is_async = significant_direct_tokens(node)
            .iter()
            .any(|token| token.kind() == TokenKind::Keyword(Keyword::Async));
        Some(FunctionSignature {
            name,
            owner,
            owner_type,
            trait_name,
            generic_parameters,
            constraints,
            parameters,
            return_type,
            span: node.span(),
            is_async,
            is_extern,
        })
    }

    fn attach_methods(&mut self) {
        for nominal in &mut self.nominals {
            nominal.methods.clear();
        }
        let implementations = self.implementations.clone();
        for implementation in implementations {
            let Some(owner) = nominal_name_from_type(&implementation.target) else {
                continue;
            };
            let Some(index) = self.nominal_lookup.get(&owner).copied() else {
                continue;
            };
            if let Some(nominal) = self.nominals.get_mut(index) {
                nominal.methods.extend(implementation.methods);
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
                .map_or(Type::Unknown, |child| {
                    self.parse_type_node(child, &HashSet::new())
                });
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
        let previous_constraints = self.active_constraints.clone();
        self.expected_return = signature.return_type.clone();
        self.saw_explicit_return = false;
        self.active_constraints = signature.constraints.clone();
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
        self.active_constraints = previous_constraints;
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
        let (text, explicit_arguments) = name_expression_parts(node, self.source);
        if text.starts_with('.') || text.contains("::") {
            return Type::Unknown;
        }
        if explicit_arguments.is_empty() {
            if let Some(binding) = self.lookup_local(&text) {
                return binding.ty.clone();
            }
            if let Some(constant) = self.constants.get(&text) {
                return constant.clone();
            }
        }
        if let Some(index) = self.function_lookup.get(&text) {
            if let Some(signature) = self.functions.get(*index) {
                let generic_names = signature
                    .generic_parameters
                    .iter()
                    .map(|parameter| parameter.name.clone())
                    .collect::<HashSet<_>>();
                let parameters = signature
                    .parameters
                    .iter()
                    .map(|parameter| erase_generic_parameters(parameter.ty.clone(), &generic_names))
                    .collect();
                let result = erase_generic_parameters(signature.return_type.clone(), &generic_names);
                return Type::Function(parameters, Box::new(result));
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
            TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Percent => {
                if left == right && left.is_numeric() {
                    left
                } else if operator == TokenKind::Plus
                    && left == Type::String
                    && right == Type::String
                {
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
            TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::Greater
            | TokenKind::GreaterEqual => {
                if left == right && (left.is_numeric() || matches!(left, Type::Char | Type::String))
                {
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
                    Diagnostic::error(
                        "TYP010",
                        format!("cannot assign to immutable binding `{name}`"),
                    )
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
                        format!(
                            "prefix operator `{}` is not defined for `{operand}`",
                            operator.name()
                        ),
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
            let (name, explicit_arguments) = name_expression_parts(callee, self.source);
            if matches!(
                name.as_str(),
                "print" | "println" | "dbg" | "panic" | "todo"
            ) {
                return Type::Unit;
            }
            if name == "assert" {
                if let Some((argument, ty)) = arguments.first() {
                    self.require_bool(ty, argument.span(), "assert argument");
                }
                return Type::Unit;
            }
            if name == "ok" {
                let value = arguments
                    .first()
                    .map_or(Type::Unknown, |(_, ty)| ty.clone());
                return Type::Named("Result".to_owned(), vec![value, Type::Unknown]);
            }
            if name == "err" {
                let error = arguments
                    .first()
                    .map_or(Type::Unknown, |(_, ty)| ty.clone());
                return Type::Named("Result".to_owned(), vec![Type::Unknown, error]);
            }
            if let Some(index) = self.function_lookup.get(&name).copied() {
                if let Some(signature) = self.functions.get(index).cloned() {
                    return self.check_callable(
                        &format!("function `{name}`"),
                        &signature.generic_parameters,
                        &signature.constraints,
                        &signature.parameters,
                        &signature.return_type,
                        &explicit_arguments,
                        &arguments,
                        node.span(),
                        signature.span,
                    );
                }
            }
        }

        if let Some(result) = self.infer_enum_variant_call(callee, &arguments, node.span()) {
            return result;
        }
        if let Some(result) = self.infer_method_call(callee, &arguments, node.span()) {
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

    fn check_callable(
        &mut self,
        label: &str,
        generic_parameters: &[GenericParameterInfo],
        constraints: &[TraitConstraint],
        parameters: &[ParameterType],
        return_type: &Type,
        explicit_arguments: &[String],
        arguments: &[(&SyntaxNode, Type)],
        call_span: Span,
        declaration_span: Span,
    ) -> Type {
        if parameters.len() != arguments.len() {
            self.diagnostics.push(
                Diagnostic::error(
                    "TYP003",
                    format!(
                        "{label} expects {} argument(s), found {}",
                        parameters.len(),
                        arguments.len()
                    ),
                )
                .with_primary(call_span, "wrong number of arguments")
                .with_secondary(declaration_span, "callable declared here")
                .with_help("add or remove arguments to match the callable signature"),
            );
        }

        let generic_names = generic_parameters
            .iter()
            .map(|parameter| parameter.name.clone())
            .collect::<HashSet<_>>();
        let mut substitutions = HashMap::<String, Type>::new();
        if !explicit_arguments.is_empty() {
            if explicit_arguments.len() != generic_parameters.len() {
                self.diagnostics.push(
                    Diagnostic::error(
                        "GEN001",
                        format!(
                            "{label} expects {} generic argument(s), found {}",
                            generic_parameters.len(),
                            explicit_arguments.len()
                        ),
                    )
                    .with_primary(call_span, "wrong generic argument count")
                    .with_secondary(declaration_span, "generic callable declared here")
                    .with_help("provide exactly the declared number of generic arguments"),
                );
            }
            for (parameter, text) in generic_parameters.iter().zip(explicit_arguments.iter()) {
                let ty = self.parse_declared_type(text, call_span, &HashSet::new());
                let _ = substitutions.insert(parameter.name.clone(), ty);
            }
        }
        for ((_, actual), expected) in arguments.iter().zip(parameters.iter()) {
            if let Some((parameter, previous, found)) = infer_type_substitutions(
                &expected.ty,
                actual,
                &generic_names,
                &mut substitutions,
            ) {
                self.diagnostics.push(
                    Diagnostic::error(
                        "GEN003",
                        format!(
                            "conflicting inference for `{parameter}`: `{previous}` and `{found}`"
                        ),
                    )
                    .with_primary(call_span, "generic arguments imply incompatible types")
                    .with_secondary(declaration_span, "generic callable declared here")
                    .with_help("use compatible arguments or specify generic arguments explicitly"),
                );
            }
        }
        for parameter in generic_parameters {
            if !substitutions.contains_key(&parameter.name) {
                self.diagnostics.push(
                    Diagnostic::error(
                        "GEN002",
                        format!("cannot infer generic argument `{}` for {label}", parameter.name),
                    )
                    .with_primary(call_span, "generic argument is not determined by this call")
                    .with_secondary(parameter.span, "generic parameter declared here")
                    .with_help(format!(
                        "provide an explicit argument such as `<{}>`",
                        parameter.name
                    )),
                );
                let _ = substitutions.insert(parameter.name.clone(), Type::Unknown);
            }
        }
        self.validate_substitution_constraints(constraints, &substitutions, call_span);

        let instantiated_parameters = parameters
            .iter()
            .map(|parameter| substitute_type(&parameter.ty, &substitutions))
            .collect::<Vec<_>>();
        for ((argument, actual), expected) in arguments.iter().zip(instantiated_parameters.iter()) {
            self.require_assignable(
                expected,
                actual,
                argument.span(),
                "call argument",
                "TYP004",
            );
        }
        substitute_type(return_type, &substitutions)
    }

    fn validate_substitution_constraints(
        &mut self,
        constraints: &[TraitConstraint],
        substitutions: &HashMap<String, Type>,
        span: Span,
    ) {
        for constraint in constraints {
            let Some(actual) = substitutions.get(&constraint.parameter) else {
                continue;
            };
            if actual.is_recovery() {
                continue;
            }
            let mut visiting = HashSet::new();
            if !self.type_implements_trait(actual, &constraint.trait_name, &mut visiting) {
                self.diagnostics.push(
                    Diagnostic::error(
                        "GEN004",
                        format!(
                            "type `{actual}` does not satisfy trait bound `{}`",
                            constraint.trait_name
                        ),
                    )
                    .with_primary(span, "unsatisfied generic trait bound")
                    .with_secondary(constraint.span, "bound declared here")
                    .with_help(format!(
                        "implement `{}` for `{actual}` or use a compatible type",
                        constraint.trait_name
                    )),
                );
            }
        }
    }

    fn type_implements_trait(
        &self,
        actual: &Type,
        trait_name: &str,
        visiting: &mut HashSet<(String, String)>,
    ) -> bool {
        if actual.is_recovery() {
            return true;
        }
        if let Type::Parameter(parameter) = actual {
            return self.active_constraints.iter().any(|constraint| {
                constraint.parameter == *parameter && constraint.trait_name == trait_name
            });
        }
        let key = (actual.display_name(), trait_name.to_owned());
        if !visiting.insert(key.clone()) {
            return true;
        }
        let mut matched = false;
        for implementation in &self.implementations {
            if implementation.trait_name.as_deref() != Some(trait_name) {
                continue;
            }
            let names = implementation
                .generic_parameters
                .iter()
                .cloned()
                .collect::<HashSet<_>>();
            let mut substitutions = HashMap::new();
            if infer_type_substitutions(
                &implementation.target,
                actual,
                &names,
                &mut substitutions,
            )
            .is_some()
            {
                continue;
            }
            let target = substitute_type(&implementation.target, &substitutions);
            if !types_equivalent_strict(&target, actual) {
                continue;
            }
            let constraints_hold = implementation.constraints.iter().all(|constraint| {
                substitutions.get(&constraint.parameter).map_or(false, |ty| {
                    self.type_implements_trait(ty, &constraint.trait_name, visiting)
                })
            });
            if constraints_hold {
                matched = true;
                break;
            }
        }
        let _ = visiting.remove(&key);
        matched
    }

    fn infer_method_call(
        &mut self,
        callee: &SyntaxNode,
        arguments: &[(&SyntaxNode, Type)],
        call_span: Span,
    ) -> Option<Type> {
        if callee.kind() != SyntaxKind::MemberExpression {
            return None;
        }
        let base_node = callee.child_nodes().next()?;
        let (member, _) = member_name(callee, self.source)?;
        let base_type = self.infer_expression(base_node);
        if base_type.is_recovery() {
            return Some(Type::Unknown);
        }
        let dispatch_type = match &base_type {
            Type::Reference { inner, .. } => inner.as_ref().clone(),
            _ => base_type.clone(),
        };

        let mut candidates = Vec::<(MethodInfo, HashMap<String, Type>)>::new();
        if let Type::Parameter(parameter) = &dispatch_type {
            for constraint in &self.active_constraints {
                if constraint.parameter != *parameter {
                    continue;
                }
                let Some(index) = self.trait_lookup.get(&constraint.trait_name).copied() else {
                    continue;
                };
                let Some(trait_info) = self.traits.get(index) else {
                    continue;
                };
                for method in trait_info.methods.iter().filter(|method| method.name == member) {
                    let mut substitutions = HashMap::new();
                    let _ = substitutions.insert("Self".to_owned(), dispatch_type.clone());
                    candidates.push((
                        MethodInfo {
                            name: method.name.clone(),
                            parameters: method.parameters.clone(),
                            return_type: method.return_type.clone(),
                            span: method.span,
                            mutable_receiver: method.mutable_receiver,
                            trait_name: Some(trait_info.name.clone()),
                            owner_type: dispatch_type.clone(),
                            generic_parameters: Vec::new(),
                            constraints: Vec::new(),
                        },
                        substitutions,
                    ));
                }
            }
        } else {
            for implementation in &self.implementations {
                let names = implementation
                    .generic_parameters
                    .iter()
                    .cloned()
                    .collect::<HashSet<_>>();
                let mut substitutions = HashMap::new();
                if infer_type_substitutions(
                    &implementation.target,
                    &dispatch_type,
                    &names,
                    &mut substitutions,
                )
                .is_some()
                {
                    continue;
                }
                let target = substitute_type(&implementation.target, &substitutions);
                if !types_equivalent_strict(&target, &dispatch_type) {
                    continue;
                }
                let mut visiting = HashSet::new();
                let applicable = implementation.constraints.iter().all(|constraint| {
                    substitutions.get(&constraint.parameter).map_or(true, |ty| {
                        self.type_implements_trait(ty, &constraint.trait_name, &mut visiting)
                    })
                });
                if !applicable {
                    continue;
                }
                let has_override = implementation
                    .methods
                    .iter()
                    .any(|method| method.name == member);
                for method in implementation
                    .methods
                    .iter()
                    .filter(|method| method.name == member)
                {
                    candidates.push((method.clone(), substitutions.clone()));
                }
                if !has_override {
                    if let Some(trait_name) = implementation.trait_name.as_deref() {
                        if let Some(trait_index) = self.trait_lookup.get(trait_name).copied() {
                            if let Some(default_method) = self
                                .traits
                                .get(trait_index)
                                .and_then(|trait_info| {
                                    trait_info.methods.iter().find(|method| {
                                        method.name == member && method.has_default
                                    })
                                })
                                .cloned()
                            {
                                let mut default_substitutions = substitutions.clone();
                                let _ = default_substitutions
                                    .insert("Self".to_owned(), dispatch_type.clone());
                                candidates.push((
                                    MethodInfo {
                                        name: default_method.name,
                                        parameters: default_method.parameters,
                                        return_type: default_method.return_type,
                                        span: default_method.span,
                                        mutable_receiver: default_method.mutable_receiver,
                                        trait_name: Some(trait_name.to_owned()),
                                        owner_type: implementation.target.clone(),
                                        generic_parameters: Vec::new(),
                                        constraints: implementation.constraints.clone(),
                                    },
                                    default_substitutions,
                                ));
                            }
                        }
                    }
                }
            }
        }
        if candidates.is_empty() {
            return None;
        }
        let inherent = candidates
            .iter()
            .filter(|(method, _)| method.trait_name.is_none())
            .cloned()
            .collect::<Vec<_>>();
        if inherent.len() == 1 {
            candidates = inherent;
        } else if inherent.len() > 1 || candidates.len() > 1 {
            self.diagnostics.push(
                Diagnostic::error(
                    "TRT005",
                    format!("method call `{member}` is ambiguous for type `{dispatch_type}`"),
                )
                .with_primary(call_span, "multiple applicable methods were found")
                .with_help("use an explicit trait-qualified call or remove the overlapping implementation"),
            );
            return Some(Type::Error);
        }
        let Some((method, owner_substitutions)) = candidates.into_iter().next() else {
            return Some(Type::Unknown);
        };
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
        let parameters = method
            .parameters
            .iter()
            .map(|parameter| ParameterType {
                name: parameter.name.clone(),
                ty: substitute_type(&parameter.ty, &owner_substitutions),
                span: parameter.span,
            })
            .collect::<Vec<_>>();
        let result = substitute_type(&method.return_type, &owner_substitutions);
        let callable_generics = method
            .generic_parameters
            .iter()
            .filter(|parameter| !owner_substitutions.contains_key(&parameter.name))
            .cloned()
            .collect::<Vec<_>>();
        let callable_constraints = method
            .constraints
            .iter()
            .filter(|constraint| !owner_substitutions.contains_key(&constraint.parameter))
            .cloned()
            .collect::<Vec<_>>();
        let (_, explicit_arguments) = member_expression_parts(callee, self.source);
        Some(self.check_callable(
            &format!("method `{}`", method.name),
            &callable_generics,
            &callable_constraints,
            &parameters,
            &result,
            &explicit_arguments,
            arguments,
            call_span,
            method.span,
        ))
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
        let (type_name, explicit_arguments) = name_expression_parts(base, self.source);
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
        let mut substitutions = HashMap::<String, Type>::new();
        self.apply_explicit_nominal_arguments(
            &nominal,
            &explicit_arguments,
            &mut substitutions,
            call_span,
        );
        for ((_, actual), pattern) in arguments.iter().zip(variant.payload.iter()) {
            if let Some((parameter, previous, found)) = infer_type_substitutions(
                pattern,
                actual,
                &generic_names,
                &mut substitutions,
            ) {
                self.diagnostics.push(
                    Diagnostic::error(
                        "GEN003",
                        format!(
                            "conflicting inference for `{parameter}`: `{previous}` and `{found}`"
                        ),
                    )
                    .with_primary(call_span, "variant payload implies incompatible generic types")
                    .with_secondary(variant.span, "variant declared here")
                    .with_help("provide compatible payload values or explicit type arguments"),
                );
            }
        }
        self.complete_nominal_substitutions(&nominal, &mut substitutions, call_span);
        let payload = variant
            .payload
            .iter()
            .map(|ty| substitute_type(ty, &substitutions))
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

        Some(self.instantiated_nominal_type(&nominal, &substitutions))
    }

    fn apply_explicit_nominal_arguments(
        &mut self,
        nominal: &NominalTypeInfo,
        explicit_arguments: &[String],
        substitutions: &mut HashMap<String, Type>,
        span: Span,
    ) {
        if explicit_arguments.is_empty() {
            return;
        }
        if explicit_arguments.len() != nominal.generic_parameters.len() {
            self.diagnostics.push(
                Diagnostic::error(
                    "GEN001",
                    format!(
                        "type `{}` expects {} generic argument(s), found {}",
                        nominal.name,
                        nominal.generic_parameters.len(),
                        explicit_arguments.len()
                    ),
                )
                .with_primary(span, "wrong generic argument count")
                .with_secondary(nominal.span, "nominal type declared here")
                .with_help("provide exactly the declared number of generic arguments"),
            );
        }
        for (parameter, text) in nominal
            .generic_parameters
            .iter()
            .zip(explicit_arguments.iter())
        {
            let ty = self.parse_declared_type(text, span, &HashSet::new());
            let _ = substitutions.insert(parameter.clone(), ty);
        }
    }

    fn complete_nominal_substitutions(
        &mut self,
        nominal: &NominalTypeInfo,
        substitutions: &mut HashMap<String, Type>,
        span: Span,
    ) {
        for parameter in &nominal.generic_parameters {
            if !substitutions.contains_key(parameter) {
                self.diagnostics.push(
                    Diagnostic::error(
                        "GEN002",
                        format!(
                            "cannot infer generic argument `{parameter}` for `{}`",
                            nominal.name
                        ),
                    )
                    .with_primary(span, "generic argument is not determined here")
                    .with_secondary(nominal.span, "generic nominal type declared here")
                    .with_help(format!(
                        "provide explicit arguments such as `{}<...>`",
                        nominal.name
                    )),
                );
                let _ = substitutions.insert(parameter.clone(), Type::Unknown);
            }
        }
        self.validate_substitution_constraints(&nominal.constraints, substitutions, span);
    }

    fn instantiated_nominal_type(
        &self,
        nominal: &NominalTypeInfo,
        substitutions: &HashMap<String, Type>,
    ) -> Type {
        Type::Named(
            nominal.name.clone(),
            nominal
                .generic_parameters
                .iter()
                .map(|parameter| substitutions.get(parameter).cloned().unwrap_or(Type::Unknown))
                .collect(),
        )
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
            let (type_name, explicit_arguments) = name_expression_parts(base_node, self.source);
            if let Some(index) = self.nominal_lookup.get(&type_name).copied() {
                if let Some(nominal) = self.nominals.get(index).cloned() {
                    if nominal.kind == NominalKind::Enum {
                        if let Some(variant) = nominal
                            .variants
                            .iter()
                            .find(|variant| variant.name == member)
                        {
                            let mut substitutions = HashMap::new();
                            self.apply_explicit_nominal_arguments(
                                &nominal,
                                &explicit_arguments,
                                &mut substitutions,
                                base_node.span(),
                            );
                            if variant.payload.is_empty() {
                                self.complete_nominal_substitutions(
                                    &nominal,
                                    &mut substitutions,
                                    base_node.span(),
                                );
                                return self.instantiated_nominal_type(&nominal, &substitutions);
                            }
                            let generic_names = nominal
                                .generic_parameters
                                .iter()
                                .cloned()
                                .collect::<HashSet<_>>();
                            let payload = variant
                                .payload
                                .iter()
                                .cloned()
                                .map(|ty| {
                                    if explicit_arguments.is_empty() {
                                        erase_generic_parameters(ty, &generic_names)
                                    } else {
                                        substitute_type(&ty, &substitutions)
                                    }
                                })
                                .collect();
                            let result = if explicit_arguments.is_empty() {
                                Type::Named(
                                    nominal.name.clone(),
                                    vec![Type::Unknown; nominal.generic_parameters.len()],
                                )
                            } else {
                                self.instantiated_nominal_type(&nominal, &substitutions)
                            };
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
                .with_primary(
                    member_span,
                    "optional values must be handled before member access",
                )
                .with_help("use `if let`, `match`, or another explicit optional operation"),
            );
            return Type::Error;
        }
        if let Type::Parameter(parameter) = &base_type {
            let mut methods = Vec::new();
            for constraint in &self.active_constraints {
                if constraint.parameter != *parameter {
                    continue;
                }
                if let Some(index) = self.trait_lookup.get(&constraint.trait_name).copied() {
                    if let Some(trait_info) = self.traits.get(index) {
                        methods.extend(
                            trait_info
                                .methods
                                .iter()
                                .filter(|method| method.name == member)
                                .cloned(),
                        );
                    }
                }
            }
            if methods.len() == 1 {
                let method = &methods[0];
                let mut substitutions = HashMap::new();
                let _ = substitutions.insert("Self".to_owned(), base_type.clone());
                return Type::Function(
                    method
                        .parameters
                        .iter()
                        .map(|parameter| substitute_type(&parameter.ty, &substitutions))
                        .collect(),
                    Box::new(substitute_type(&method.return_type, &substitutions)),
                );
            }
            if methods.len() > 1 {
                self.diagnostics.push(
                    Diagnostic::error(
                        "TRT005",
                        format!("member `{member}` is ambiguous for generic parameter `{parameter}`"),
                    )
                    .with_primary(member_span, "multiple trait bounds provide this method")
                    .with_help("use an explicit trait-qualified call"),
                );
                return Type::Error;
            }
        }
        let Some(nominal_name) = nominal_name_from_type(&base_type) else {
            self.diagnostics.push(
                Diagnostic::error(
                    "NOM002",
                    format!("type `{base_type}` does not expose nominal members"),
                )
                .with_primary(
                    member_span,
                    "member lookup requires a record, struct, enum, or bounded generic value",
                )
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
        let substitutions = nominal_substitutions(&nominal, &base_type);

        if let Some(field) = nominal.fields.iter().find(|field| field.name == member) {
            return substitute_type(&field.ty, &substitutions);
        }
        let applicable = nominal
            .methods
            .iter()
            .filter(|method| method.name == member)
            .filter_map(|method| {
                let names = implementation_generic_names(method);
                let mut method_substitutions = HashMap::new();
                if infer_type_substitutions(
                    &method.owner_type,
                    &base_type,
                    &names,
                    &mut method_substitutions,
                )
                .is_some()
                {
                    return None;
                }
                let owner = substitute_type(&method.owner_type, &method_substitutions);
                types_equivalent_strict(&owner, &base_type)
                    .then_some((method.clone(), method_substitutions))
            })
            .collect::<Vec<_>>();
        if applicable.len() > 1 {
            self.diagnostics.push(
                Diagnostic::error(
                    "TRT005",
                    format!("member `{member}` is ambiguous for type `{base_type}`"),
                )
                .with_primary(member_span, "multiple applicable methods were found")
                .with_help("use an explicit trait-qualified call or remove an overlapping impl"),
            );
            return Type::Error;
        }
        if let Some((method, method_substitutions)) = applicable.into_iter().next() {
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
                    .map(|parameter| substitute_type(&parameter.ty, &method_substitutions))
                    .collect(),
                Box::new(substitute_type(
                    &method.return_type,
                    &method_substitutions,
                )),
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
        let (type_name, explicit_arguments) = name_expression_parts(target, self.source);
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
                    format!(
                        "enum `{}` cannot be constructed with record syntax",
                        nominal.name
                    ),
                )
                .with_primary(target.span(), "use a declared enum variant")
                .with_help(format!(
                    "construct a variant such as `{}.<variant>(...)`",
                    nominal.name
                )),
            );
            return Type::Error;
        }

        let generic_names = nominal
            .generic_parameters
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        let mut substitutions = HashMap::<String, Type>::new();
        self.apply_explicit_nominal_arguments(
            &nominal,
            &explicit_arguments,
            &mut substitutions,
            target.span(),
        );
        let mut initialized = HashSet::new();
        let mut pending = Vec::<(String, Span, Option<&SyntaxNode>, Type)>::new();
        for initializer in children
            .iter()
            .skip(1)
            .copied()
            .filter(|child| child.kind() == SyntaxKind::RecordFieldInitializer)
        {
            let Some((field_name, field_span)) = first_direct_identifier(initializer, self.source)
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
            if let Some((parameter, previous, found)) = infer_type_substitutions(
                &field.ty,
                &actual,
                &generic_names,
                &mut substitutions,
            ) {
                self.diagnostics.push(
                    Diagnostic::error(
                        "GEN003",
                        format!(
                            "conflicting inference for `{parameter}`: `{previous}` and `{found}`"
                        ),
                    )
                    .with_primary(field_span, "constructor fields imply incompatible types")
                    .with_secondary(field.span, "field declared here")
                    .with_help("use compatible field values or explicit generic arguments"),
                );
            }
            pending.push((field_name, field_span, expression, actual));
        }
        self.complete_nominal_substitutions(&nominal, &mut substitutions, target.span());

        for (field_name, _field_span, expression, actual) in pending {
            let Some(field) = nominal.fields.iter().find(|field| field.name == field_name) else {
                continue;
            };
            let expected = substitute_type(&field.ty, &substitutions);
            if let Some(expression) = expression {
                self.require_assignable(
                    &expected,
                    &actual,
                    expression.span(),
                    &format!("field `{field_name}`"),
                    "NOM006",
                );
            }
        }

        for field in &nominal.fields {
            if !field.has_default && !initialized.contains(&field.name) {
                self.diagnostics.push(
                    Diagnostic::error(
                        "NOM003",
                        format!(
                            "missing required field `{}` for `{}`",
                            field.name, nominal.name
                        ),
                    )
                    .with_primary(node.span(), "constructor is incomplete")
                    .with_secondary(field.span, "required field declared here")
                    .with_help(format!("add `{}: <value>`", field.name)),
                );
            }
        }

        self.instantiated_nominal_type(&nominal, &substitutions)
    }

    fn is_mutable_place(&self, node: &SyntaxNode) -> bool {
        match node.kind() {
            SyntaxKind::NameExpression => {
                let name = significant_text(node, self.source);
                self.lookup_local(&name).is_some_and(|binding| {
                    binding.mutable || matches!(&binding.ty, Type::Reference { mutable: true, .. })
                })
            }
            SyntaxKind::MemberExpression => node
                .child_nodes()
                .next()
                .is_some_and(|base| self.is_mutable_place(base)),
            SyntaxKind::PrefixExpression => significant_text(node, self.source).starts_with("&mut"),
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
            if let Some(condition) = children.iter().copied().find(|child| {
                child.kind() != SyntaxKind::Block && child.kind() != SyntaxKind::IfExpression
            }) {
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
            self.require_assignable(
                &Type::Int,
                &index_type,
                index.span(),
                "index expression",
                "TYP001",
            );
        }
        match base {
            Type::Named(name, arguments) if name == "List" && arguments.len() == 1 => {
                arguments[0].clone()
            }
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
                    Diagnostic::error(
                        "TYP002",
                        format!("`try` requires Result<T, E>, found `{other}`"),
                    )
                    .with_primary(node.span(), "this expression cannot be propagated")
                    .with_help(
                        "return a Result from the called operation or handle the value directly",
                    ),
                );
                Type::Error
            }
        }
    }

    fn parse_type_node(&mut self, node: &SyntaxNode, generics: &HashSet<String>) -> Type {
        self.parse_declared_type(&node.lossless_text(self.source), node.span(), generics)
    }

    fn parse_declared_type(&mut self, text: &str, span: Span, generics: &HashSet<String>) -> Type {
        let mut parser = TypeTextParser::new(text);
        let parsed = parser.parse().unwrap_or(Type::Error);
        let ty = mark_generic_parameters(parsed, generics);
        if ty == Type::Error {
            self.diagnostics.push(
                Diagnostic::error("TYP008", "invalid type syntax")
                    .with_primary(span, "the D8 type parser could not understand this type")
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
                    .with_help(format!(
                        "declare `{name}`, import it, or correct its spelling"
                    )),
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
            .with_help(format!(
                "provide a `{expected}` value or convert explicitly"
            )),
        );
    }

    fn unsupported_operator(&mut self, span: Span, operator: TokenKind, left: &Type, right: &Type) {
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
    let colon = tokens
        .iter()
        .position(|token| token.kind() == TokenKind::Colon)?;
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

#[derive(Clone, Debug)]
struct ImplHeaderParts {
    trait_name: Option<String>,
    target_text: String,
    generic_parameters: Vec<GenericParameterInfo>,
    constraints: Vec<TraitConstraint>,
}

fn impl_header_parts(node: &SyntaxNode, source: &SourceFile) -> ImplHeaderParts {
    let generic_parameters = generic_parameter_infos(node, source);
    let constraints = declaration_constraints(node, source, &generic_parameters);
    let tokens = significant_direct_tokens(node);
    let impl_index = tokens
        .iter()
        .position(|token| token.kind() == TokenKind::Keyword(Keyword::Impl))
        .unwrap_or(0);
    let end_index = tokens
        .iter()
        .position(|token| token.kind() == TokenKind::LeftBrace)
        .unwrap_or(tokens.len());
    let header = &tokens[impl_index.saturating_add(1)..end_index];
    let for_index = header
        .iter()
        .position(|token| token.kind() == TokenKind::Keyword(Keyword::For));
    let (trait_tokens, target_tokens) = if let Some(index) = for_index {
        (&header[..index], &header[index + 1..])
    } else {
        (&header[0..0], header)
    };
    let trait_text = token_text(trait_tokens, source);
    let target_text = token_text(target_tokens, source);
    ImplHeaderParts {
        trait_name: (!trait_text.is_empty()).then(|| base_type_name(&trait_text)),
        target_text,
        generic_parameters,
        constraints,
    }
}

fn token_text(tokens: &[SyntaxToken], source: &SourceFile) -> String {
    let mut output = String::new();
    for token in tokens {
        if let Some(text) = token.text(source) {
            if needs_type_space(&output, text) {
                output.push(' ');
            }
            output.push_str(text);
        }
    }
    output
}

fn base_type_name(text: &str) -> String {
    text.split(['<', ':', '.'])
        .find(|part| !part.is_empty())
        .unwrap_or(text)
        .to_owned()
}

fn replace_self_type(ty: Type, owner: &Type) -> Type {
    match ty {
        Type::Named(name, arguments) if name == "Self" && arguments.is_empty() => owner.clone(),
        Type::Parameter(name) if name == "Self" => owner.clone(),
        Type::Named(name, arguments) => Type::Named(
            name,
            arguments
                .into_iter()
                .map(|argument| replace_self_type(argument, owner))
                .collect(),
        ),
        Type::Optional(inner) => Type::Optional(Box::new(replace_self_type(*inner, owner))),
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
        Type::Parameter(name) if generics.contains(&name) => Type::Unknown,
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
        Type::Parameter(name) => generics.contains(name),
        Type::Named(name, arguments) => {
            (generics.contains(name) && arguments.is_empty())
                || arguments
                    .iter()
                    .any(|argument| type_contains_generic(argument, generics))
        }
        Type::Optional(inner) | Type::Reference { inner, .. } | Type::Pointer { inner, .. } => {
            type_contains_generic(inner, generics)
        }
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

fn mark_generic_parameters(ty: Type, generics: &HashSet<String>) -> Type {
    match ty {
        Type::Named(name, arguments) if generics.contains(&name) && arguments.is_empty() => {
            Type::Parameter(name)
        }
        Type::Named(name, arguments) => Type::Named(
            name,
            arguments
                .into_iter()
                .map(|argument| mark_generic_parameters(argument, generics))
                .collect(),
        ),
        Type::Optional(inner) => {
            Type::Optional(Box::new(mark_generic_parameters(*inner, generics)))
        }
        Type::Reference { mutable, inner } => Type::Reference {
            mutable,
            inner: Box::new(mark_generic_parameters(*inner, generics)),
        },
        Type::Pointer { mutable, inner } => Type::Pointer {
            mutable,
            inner: Box::new(mark_generic_parameters(*inner, generics)),
        },
        Type::Tuple(items) => Type::Tuple(
            items
                .into_iter()
                .map(|item| mark_generic_parameters(item, generics))
                .collect(),
        ),
        Type::Function(parameters, result) => Type::Function(
            parameters
                .into_iter()
                .map(|parameter| mark_generic_parameters(parameter, generics))
                .collect(),
            Box::new(mark_generic_parameters(*result, generics)),
        ),
        other => other,
    }
}

fn substitute_type(ty: &Type, substitutions: &HashMap<String, Type>) -> Type {
    match ty {
        Type::Parameter(name) => substitutions.get(name).cloned().unwrap_or_else(|| ty.clone()),
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
        other => other.clone(),
    }
}

fn collect_type_parameters(ty: &Type, output: &mut HashSet<String>) {
    match ty {
        Type::Parameter(name) => {
            let _ = output.insert(name.clone());
        }
        Type::Named(_, arguments) | Type::Tuple(arguments) => {
            for argument in arguments {
                collect_type_parameters(argument, output);
            }
        }
        Type::Optional(inner) | Type::Reference { inner, .. } | Type::Pointer { inner, .. } => {
            collect_type_parameters(inner, output);
        }
        Type::Function(parameters, result) => {
            for parameter in parameters {
                collect_type_parameters(parameter, output);
            }
            collect_type_parameters(result, output);
        }
        _ => {}
    }
}

fn infer_type_substitutions(
    pattern: &Type,
    actual: &Type,
    generic_names: &HashSet<String>,
    substitutions: &mut HashMap<String, Type>,
) -> Option<(String, Type, Type)> {
    match pattern {
        Type::Parameter(name) if generic_names.contains(name) => {
            if actual.is_recovery() {
                return None;
            }
            if let Some(existing) = substitutions.get(name) {
                if !types_compatible(existing, actual) || !types_compatible(actual, existing) {
                    return Some((name.clone(), existing.clone(), actual.clone()));
                }
            } else {
                let _ = substitutions.insert(name.clone(), actual.clone());
            }
        }
        Type::Named(pattern_name, pattern_arguments) => {
            if let Type::Named(actual_name, actual_arguments) = actual {
                if pattern_name == actual_name && pattern_arguments.len() == actual_arguments.len() {
                    for (left, right) in pattern_arguments.iter().zip(actual_arguments.iter()) {
                        if let Some(conflict) = infer_type_substitutions(
                            left,
                            right,
                            generic_names,
                            substitutions,
                        ) {
                            return Some(conflict);
                        }
                    }
                }
            }
        }
        Type::Optional(pattern_inner) => {
            if let Type::Optional(actual_inner) = actual {
                return infer_type_substitutions(
                    pattern_inner,
                    actual_inner,
                    generic_names,
                    substitutions,
                );
            }
        }
        Type::Reference {
            mutable: pattern_mutable,
            inner: pattern_inner,
        } => {
            if let Type::Reference {
                mutable: actual_mutable,
                inner: actual_inner,
            } = actual
            {
                if pattern_mutable == actual_mutable {
                    return infer_type_substitutions(
                        pattern_inner,
                        actual_inner,
                        generic_names,
                        substitutions,
                    );
                }
            }
        }
        Type::Pointer {
            mutable: pattern_mutable,
            inner: pattern_inner,
        } => {
            if let Type::Pointer {
                mutable: actual_mutable,
                inner: actual_inner,
            } = actual
            {
                if pattern_mutable == actual_mutable {
                    return infer_type_substitutions(
                        pattern_inner,
                        actual_inner,
                        generic_names,
                        substitutions,
                    );
                }
            }
        }
        Type::Tuple(pattern_items) => {
            if let Type::Tuple(actual_items) = actual {
                if pattern_items.len() == actual_items.len() {
                    for (left, right) in pattern_items.iter().zip(actual_items.iter()) {
                        if let Some(conflict) = infer_type_substitutions(
                            left,
                            right,
                            generic_names,
                            substitutions,
                        ) {
                            return Some(conflict);
                        }
                    }
                }
            }
        }
        Type::Function(pattern_parameters, pattern_result) => {
            if let Type::Function(actual_parameters, actual_result) = actual {
                if pattern_parameters.len() == actual_parameters.len() {
                    for (left, right) in pattern_parameters.iter().zip(actual_parameters.iter()) {
                        if let Some(conflict) = infer_type_substitutions(
                            left,
                            right,
                            generic_names,
                            substitutions,
                        ) {
                            return Some(conflict);
                        }
                    }
                    return infer_type_substitutions(
                        pattern_result,
                        actual_result,
                        generic_names,
                        substitutions,
                    );
                }
            }
        }
        _ => {}
    }
    None
}


fn builtin_generic_arity(name: &str) -> Option<usize> {
    match name {
        "List" | "Option" | "Set" | "Shared" | "Task" | "Weak" => Some(1),
        "Map" | "Result" => Some(2),
        _ => None,
    }
}

fn types_equivalent_strict(left: &Type, right: &Type) -> bool {
    left == right
}

fn canonical_type_pattern(ty: &Type, generic_parameters: &[String]) -> String {
    let names = generic_parameters.iter().cloned().collect::<HashSet<_>>();
    fn render(ty: &Type, names: &HashSet<String>) -> String {
        match ty {
            Type::Parameter(name) if names.contains(name) => "_".to_owned(),
            Type::Named(name, arguments) => {
                if arguments.is_empty() {
                    name.clone()
                } else {
                    format!(
                        "{}<{}>",
                        name,
                        arguments
                            .iter()
                            .map(|argument| render(argument, names))
                            .collect::<Vec<_>>()
                            .join(",")
                    )
                }
            }
            Type::Optional(inner) => format!("{}?", render(inner, names)),
            Type::Reference { mutable, inner } => format!(
                "&{}{}",
                if *mutable { "mut " } else { "" },
                render(inner, names)
            ),
            Type::Pointer { mutable, inner } => format!(
                "*{} {}",
                if *mutable { "mut" } else { "const" },
                render(inner, names)
            ),
            Type::Tuple(items) => format!(
                "({})",
                items
                    .iter()
                    .map(|item| render(item, names))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Type::Function(parameters, result) => format!(
                "fn({})->{}",
                parameters
                    .iter()
                    .map(|parameter| render(parameter, names))
                    .collect::<Vec<_>>()
                    .join(","),
                render(result, names)
            ),
            other => other.display_name(),
        }
    }
    render(ty, &names)
}

fn nominal_substitutions(
    nominal: &NominalTypeInfo,
    actual: &Type,
) -> HashMap<String, Type> {
    let mut substitutions = HashMap::new();
    match actual {
        Type::Named(name, arguments) if name == &nominal.name => {
            for (parameter, argument) in nominal.generic_parameters.iter().zip(arguments.iter()) {
                let _ = substitutions.insert(parameter.clone(), argument.clone());
            }
        }
        Type::Reference { inner, .. } => return nominal_substitutions(nominal, inner),
        _ => {}
    }
    substitutions
}

fn implementation_generic_names(method: &MethodInfo) -> HashSet<String> {
    let mut output = method
        .generic_parameters
        .iter()
        .map(|parameter| parameter.name.clone())
        .collect::<HashSet<_>>();
    collect_type_parameters(&method.owner_type, &mut output);
    output
}

fn member_expression_parts(node: &SyntaxNode, source: &SourceFile) -> (String, Vec<String>) {
    let member = significant_direct_tokens(node)
        .into_iter()
        .rev()
        .find(|token| matches!(token.kind(), TokenKind::Identifier | TokenKind::Keyword(_)))
        .and_then(|token| token.text(source))
        .unwrap_or_default()
        .to_owned();
    (member, generic_argument_texts(node, source))
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
        .find(|token| matches!(token.kind(), TokenKind::Identifier | TokenKind::Keyword(_)))
        .and_then(|token| {
            token
                .text(source)
                .map(|text| (text.to_owned(), token.span()))
        })
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
            let substitution = previous[right_index] + usize::from(left_char != *right_char);
            current.push(deletion.min(insertion).min(substitution));
        }
        previous = current;
    }
    previous[right_chars.len()]
}

fn builtin_type_names() -> Vec<&'static str> {
    vec![
        "Bool", "Char", "Float", "F32", "F64", "I8", "I16", "I32", "I64", "Int", "List", "Map",
        "Never", "Option", "Path", "Result", "Set", "Shared", "String", "Task", "U8", "U16", "U32",
        "U64", "Unit", "Usize", "Weak", "Self",
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
        ) => expected_mutable == actual_mutable && types_compatible(expected_inner, actual_inner),
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
    if concrete
        .all(|candidate| types_compatible(first, candidate) && types_compatible(candidate, first))
    {
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
        .and_then(|token| {
            token
                .text(source)
                .map(|text| (text.to_owned(), token.span()))
        })
}

fn function_name(node: &SyntaxNode, source: &SourceFile) -> Option<(String, Span)> {
    let tokens = significant_direct_tokens(node);
    let fn_index = tokens
        .iter()
        .position(|token| token.kind() == TokenKind::Keyword(Keyword::Fn))?;
    tokens[fn_index + 1..]
        .iter()
        .copied()
        .find(|token| token.kind() == TokenKind::Identifier)
        .and_then(|token| {
            token
                .text(source)
                .map(|text| (text.to_owned(), token.span()))
        })
}

fn generic_parameter_infos(node: &SyntaxNode, source: &SourceFile) -> Vec<GenericParameterInfo> {
    let Some(generics) = node
        .child_nodes()
        .find(|child| child.kind() == SyntaxKind::GenericParameterList)
    else {
        return Vec::new();
    };
    let tokens = descendant_tokens(generics);
    let inner = strip_delimiters(&tokens, TokenKind::Less, TokenKind::Greater);
    split_top_level_tokens(inner, TokenKind::Comma)
        .into_iter()
        .filter_map(|segment| generic_parameter_from_tokens(segment, source))
        .collect()
}

fn generic_parameter_from_tokens(
    tokens: &[SyntaxToken],
    source: &SourceFile,
) -> Option<GenericParameterInfo> {
    let name_token = tokens
        .iter()
        .copied()
        .find(|token| token.kind() == TokenKind::Identifier)?;
    let name = name_token.text(source)?.to_owned();
    let colon = tokens
        .iter()
        .position(|token| token.kind() == TokenKind::Colon);
    let bounds = colon.map_or_else(Vec::new, |index| {
        tokens[index + 1..]
            .split(|token| token.kind() == TokenKind::Plus)
            .filter_map(|bound| {
                bound
                    .iter()
                    .copied()
                    .find(|token| token.kind() == TokenKind::Identifier)
                    .and_then(|token| token.text(source).map(str::to_owned))
            })
            .collect::<Vec<_>>()
    });
    Some(GenericParameterInfo {
        name,
        bounds,
        span: name_token.span(),
    })
}

fn declaration_constraints(
    node: &SyntaxNode,
    source: &SourceFile,
    parameters: &[GenericParameterInfo],
) -> Vec<TraitConstraint> {
    let mut constraints = parameters
        .iter()
        .flat_map(|parameter| {
            parameter.bounds.iter().map(|bound| TraitConstraint {
                parameter: parameter.name.clone(),
                trait_name: bound.clone(),
                span: parameter.span,
            })
        })
        .collect::<Vec<_>>();
    if let Some(where_clause) = node
        .child_nodes()
        .find(|child| child.kind() == SyntaxKind::WhereClause)
    {
        constraints.extend(where_clause_constraints(where_clause, source));
    }
    deduplicate_constraints(&mut constraints);
    constraints
}

fn where_clause_constraints(node: &SyntaxNode, source: &SourceFile) -> Vec<TraitConstraint> {
    let tokens = descendant_tokens(node);
    let body = tokens
        .iter()
        .position(|token| token.kind() == TokenKind::Keyword(Keyword::Where))
        .map_or(tokens.as_slice(), |index| &tokens[index + 1..]);
    let mut output = Vec::new();
    for segment in split_top_level_tokens(body, TokenKind::Comma) {
        let Some(colon) = segment
            .iter()
            .position(|token| token.kind() == TokenKind::Colon)
        else {
            continue;
        };
        let Some(parameter_token) = segment[..colon]
            .iter()
            .copied()
            .find(|token| token.kind() == TokenKind::Identifier)
        else {
            continue;
        };
        let Some(parameter) = parameter_token.text(source).map(str::to_owned) else {
            continue;
        };
        for bound in segment[colon + 1..].split(|token| token.kind() == TokenKind::Plus) {
            let Some(trait_token) = bound
                .iter()
                .copied()
                .find(|token| token.kind() == TokenKind::Identifier)
            else {
                continue;
            };
            if let Some(trait_name) = trait_token.text(source) {
                output.push(TraitConstraint {
                    parameter: parameter.clone(),
                    trait_name: trait_name.to_owned(),
                    span: trait_token.span(),
                });
            }
        }
    }
    output
}

fn deduplicate_constraints(constraints: &mut Vec<TraitConstraint>) {
    let mut seen = HashSet::new();
    constraints.retain(|constraint| {
        seen.insert((
            constraint.parameter.clone(),
            constraint.trait_name.clone(),
        ))
    });
}

fn strip_delimiters(
    tokens: &[SyntaxToken],
    open: TokenKind,
    close: TokenKind,
) -> &[SyntaxToken] {
    let start = usize::from(tokens.first().is_some_and(|token| token.kind() == open));
    let end = tokens
        .len()
        .saturating_sub(usize::from(tokens.last().is_some_and(|token| {
            token.kind() == close || token.kind() == TokenKind::ShiftRight
        })));
    if start <= end {
        &tokens[start..end]
    } else {
        &[]
    }
}

fn split_top_level_tokens(tokens: &[SyntaxToken], separator: TokenKind) -> Vec<&[SyntaxToken]> {
    let mut output = Vec::new();
    let mut start = 0usize;
    let mut angle = 0usize;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        match token.kind() {
            TokenKind::Less => angle += 1,
            TokenKind::Greater if angle > 0 => angle -= 1,
            TokenKind::ShiftRight if angle > 0 => angle = angle.saturating_sub(2),
            TokenKind::LeftParen => paren += 1,
            TokenKind::RightParen if paren > 0 => paren -= 1,
            TokenKind::LeftBracket => bracket += 1,
            TokenKind::RightBracket if bracket > 0 => bracket -= 1,
            kind if kind == separator && angle == 0 && paren == 0 && bracket == 0 => {
                output.push(&tokens[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    output.push(&tokens[start..]);
    output
}

fn generic_argument_texts(node: &SyntaxNode, source: &SourceFile) -> Vec<String> {
    let Some(arguments) = node
        .child_nodes()
        .find(|child| child.kind() == SyntaxKind::GenericArgumentList)
    else {
        return Vec::new();
    };
    let Some(text) = source.slice(arguments.span()) else {
        return Vec::new();
    };
    let inner = text
        .strip_prefix('<')
        .and_then(|value| value.strip_suffix('>'))
        .unwrap_or(text);
    split_top_level_type_text(inner)
}

fn split_top_level_type_text(text: &str) -> Vec<String> {
    let mut output = Vec::new();
    let mut start = 0usize;
    let mut angle = 0usize;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    for (index, character) in text.char_indices() {
        match character {
            '<' => angle += 1,
            '>' if angle > 0 => angle -= 1,
            '(' => paren += 1,
            ')' if paren > 0 => paren -= 1,
            '[' => bracket += 1,
            ']' if bracket > 0 => bracket -= 1,
            ',' if angle == 0 && paren == 0 && bracket == 0 => {
                let item = text[start..index].trim();
                if !item.is_empty() {
                    output.push(item.to_owned());
                }
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    let item = text[start..].trim();
    if !item.is_empty() {
        output.push(item.to_owned());
    }
    output
}

fn name_expression_parts(node: &SyntaxNode, source: &SourceFile) -> (String, Vec<String>) {
    let path = significant_direct_tokens(node)
        .iter()
        .filter_map(|token| token.text(source))
        .collect::<Vec<_>>()
        .join("");
    (path, generic_argument_texts(node, source))
}

fn parameter_parts(node: &SyntaxNode, source: &SourceFile) -> Option<(String, Span, String)> {
    let tokens = descendant_tokens(node);
    let name_token = tokens.iter().copied().find(|token| {
        matches!(
            token.kind(),
            TokenKind::Identifier | TokenKind::Keyword(Keyword::SelfValue)
        )
    })?;
    let name = name_token.text(source)?.to_owned();
    let colon = tokens
        .iter()
        .position(|token| token.kind() == TokenKind::Colon);
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
    (current.ends_with("mut") || current.ends_with("const"))
        && next
            .chars()
            .next()
            .map_or(false, |character| character.is_alphanumeric())
}

fn pattern_name(node: &SyntaxNode, source: &SourceFile) -> Option<(String, Span)> {
    descendant_tokens(node)
        .into_iter()
        .find(|token| token.kind() == TokenKind::Identifier)
        .and_then(|token| {
            token
                .text(source)
                .map(|text| (text.to_owned(), token.span()))
        })
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
        Type::Parameter(_) => {}
        Type::Named(name, arguments) => {
            if !known.contains(name) && !generics.contains(name) {
                output.push(name.clone());
            }
            for argument in arguments {
                collect_unknown_type_names(argument, known, generics, output);
            }
        }
        Type::Optional(inner) | Type::Reference { inner, .. } | Type::Pointer { inner, .. } => {
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
            .map_or(true, |character| {
                !character.is_alphanumeric() && character != '_'
            });
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

    use super::{check, Type, TypeTextParser};

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
        assert!(result
            .bindings
            .iter()
            .any(|binding| binding.name == "count" && binding.ty == Type::Int));
        assert!(result
            .bindings
            .iter()
            .any(|binding| binding.name == "ratio" && binding.ty == Type::Float));
        assert!(result
            .bindings
            .iter()
            .any(|binding| binding.name == "enabled" && binding.ty == Type::Bool));
    }

    #[test]
    fn rejects_binding_type_mismatch() {
        let result = check_text("module test\nfn main() { let count: Int = \"two\"\n }\n");
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "TYP001"));
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
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "TYP003"));
    }

    #[test]
    fn rejects_wrong_argument_type() {
        let result = check_text(
            "module test\nfn echo(value: Int) -> Int { value }\nfn main() { echo(\"wrong\") }\n",
        );
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "TYP004"));
    }

    #[test]
    fn rejects_non_boolean_condition() {
        let result = check_text("module test\nfn main() { if 1 { print(1) } }\n");
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "TYP007"));
    }

    #[test]
    fn rejects_bad_return_type() {
        let result = check_text("module test\nfn answer() -> Int { \"forty-two\" }\n");
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "TYP005"));
    }

    #[test]
    fn rejects_none_without_context() {
        let result = check_text("module test\nfn main() { let value = none\n }\n");
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "TYP006"));
    }

    #[test]
    fn accepts_none_with_optional_context() {
        let result = check_text("module test\nfn main() { let value: String? = none\n }\n");
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
    }

    #[test]
    fn rejects_heterogeneous_array() {
        let result = check_text("module test\nfn main() { let values = [1, \"two\"]\n }\n");
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "TYP009"));
    }

    #[test]
    fn rejects_assignment_to_let() {
        let result = check_text("module test\nfn main() { let value = 1\n value = 2\n }\n");
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "TYP010"));
    }

    #[test]
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
        assert!(result
            .bindings
            .iter()
            .any(|binding| { binding.name == "label" && binding.ty == Type::String }));
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
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "NOM003"));
    }

    #[test]
    fn rejects_unknown_record_field() {
        let result = check_text(
            "module test\nrecord User {\n name: String\n}\nfn main() { let user = User { title: \"x\", name: \"M\" } }\n",
        );
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "NOM004"));
    }

    #[test]
    fn rejects_duplicate_record_initializer() {
        let result = check_text(
            "module test\nrecord User {\n name: String\n}\nfn main() { let user = User { name: \"A\", name: \"B\" } }\n",
        );
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "NOM005"));
    }

    #[test]
    fn rejects_wrong_record_field_type() {
        let result = check_text(
            "module test\nrecord User {\n age: Int\n}\nfn main() { let user = User { age: \"young\" } }\n",
        );
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "NOM006"));
    }

    #[test]
    fn resolves_enum_unit_and_payload_variants() {
        let result = check_text(
            "module test\nenum State {\n idle\n ready(String)\n}\nfn main() { let first = State.idle\n let second = State.ready(\"done\")\n }\n",
        );
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert!(result.bindings.iter().any(|binding| {
            binding.name == "first" && binding.ty == Type::Named("State".to_owned(), Vec::new())
        }));
        assert!(result.bindings.iter().any(|binding| {
            binding.name == "second" && binding.ty == Type::Named("State".to_owned(), Vec::new())
        }));
    }

    #[test]
    fn rejects_wrong_enum_payload_type() {
        let result = check_text(
            "module test\nenum State {\n ready(String)\n}\nfn main() { let state = State.ready(1) }\n",
        );
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "NOM007"));
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
            "module test\nenum State { ready(String) }\nfn main() { let state = State.redy(\"done\") }\n",
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
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "NOM002"));
    }

    #[test]
    fn rejects_mutation_through_immutable_record() {
        let result = check_text(
            "module test\nrecord User {\n name: String\n}\nfn main() { let user = User { name: \"M\" }\n user.name = \"N\"\n }\n",
        );
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "NOM008"));
    }

    #[test]
    fn rejects_unknown_constructor_target() {
        let result = check_text("module test\nfn main() { let value = Missing { name: \"x\" } }\n");
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "NOM009"));
    }

    #[test]
    fn rejects_record_syntax_for_enum() {
        let result =
            check_text("module test\nenum State { idle }\nfn main() { let value = State { } }\n");
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "NOM010"));
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
    #[test]
    fn infers_generic_function_arguments() {
        let result = check_text(
            "module test\nfn identity<T>(value: T) -> T { value }\nfn main() { let number = identity(7)\n let text = identity(\"seven\")\n }\n",
        );
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert!(result.bindings.iter().any(|binding| binding.name == "number" && binding.ty == Type::Int));
        assert!(result.bindings.iter().any(|binding| binding.name == "text" && binding.ty == Type::String));
    }

    #[test]
    fn accepts_explicit_generic_function_arguments() {
        let result = check_text(
            "module test\nfn identity<T>(value: T) -> T { value }\nfn main() { let number = identity<Int>(7)\n }\n",
        );
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert!(result.bindings.iter().any(|binding| binding.name == "number" && binding.ty == Type::Int));
    }

    #[test]
    fn rejects_wrong_generic_argument_count() {
        let result = check_text(
            "module test\nfn identity<T>(value: T) -> T { value }\nfn main() { identity<Int, String>(7) }\n",
        );
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "GEN001"));
    }

    #[test]
    fn rejects_conflicting_generic_inference() {
        let result = check_text(
            "module test\nfn same<T>(left: T, right: T) -> T { left }\nfn main() { same(1, \"one\") }\n",
        );
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "GEN003"));
    }

    #[test]
    fn substitutes_generic_record_fields_and_methods() {
        let result = check_text(
            "module test\nrecord Box<T> { value: T }\nimpl<T> Box<T> { fn get(self: &Self) -> T { self.value } }\nfn main() { let boxed = Box { value: 7 }\n let value = boxed.get()\n }\n",
        );
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert!(result.bindings.iter().any(|binding| binding.name == "boxed" && binding.ty == Type::Named("Box".to_owned(), vec![Type::Int])));
        assert!(result.bindings.iter().any(|binding| binding.name == "value" && binding.ty == Type::Int));
    }

    #[test]
    fn infers_generic_enum_payload() {
        let result = check_text(
            "module test\nenum Maybe<T> { empty, some(T) }\nfn main() { let value = Maybe.some(7) }\n",
        );
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert!(result.bindings.iter().any(|binding| binding.name == "value" && binding.ty == Type::Named("Maybe".to_owned(), vec![Type::Int])));
    }

    #[test]
    fn validates_trait_bound_and_selects_method() {
        let result = check_text(
            "module test\ntrait Display { fn display(self: &Self) -> String }\nrecord User { name: String }\nimpl Display for User { fn display(self: &Self) -> String { self.name } }\nfn render<T: Display>(value: T) -> String { value.display() }\nfn main() { let user = User { name: \"Nivra\" }\n let text = render(user)\n }\n",
        );
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert!(result.bindings.iter().any(|binding| binding.name == "text" && binding.ty == Type::String));
    }

    #[test]
    fn rejects_unsatisfied_trait_bound() {
        let result = check_text(
            "module test\ntrait Display { fn display(self: &Self) -> String }\nfn render<T: Display>(value: T) -> String { value.display() }\nfn main() { render(7) }\n",
        );
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "GEN004"));
    }

    #[test]
    fn rejects_missing_required_trait_method() {
        let result = check_text(
            "module test\ntrait Display { fn display(self: &Self) -> String }\nrecord User { name: String }\nimpl Display for User { }\n",
        );
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "TRT003"));
    }

    #[test]
    fn rejects_trait_method_signature_mismatch() {
        let result = check_text(
            "module test\ntrait Display { fn display(self: &Self) -> String }\nrecord User { name: String }\nimpl Display for User { fn display(self: &Self) -> Int { 1 } }\n",
        );
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "TRT004"));
    }

    #[test]
    fn rejects_conflicting_trait_implementations() {
        let result = check_text(
            "module test\ntrait Display { fn display(self: &Self) -> String }\nrecord User { name: String }\nimpl Display for User { fn display(self: &Self) -> String { self.name } }\nimpl Display for User { fn display(self: &Self) -> String { self.name } }\n",
        );
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "TRT002"));
    }

    #[test]
    fn rejects_generic_function_without_inference_context() {
        let result = check_text(
            "module test\nextern \"C\" { fn make<T>() -> T }\nfn main() { let value = make() }\n",
        );
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "GEN002"));
    }

    #[test]
    fn rejects_duplicate_generic_parameters() {
        let result = check_text(
            "module test\nfn choose<T, T>(value: T) -> T { value }\n",
        );
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "GEN005"));
    }

    #[test]
    fn rejects_unknown_trait_constraint() {
        let result = check_text(
            "module test\nfn render<T: Missing>(value: T) -> T { value }\n",
        );
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "TRT001"));
    }

    #[test]
    fn rejects_ambiguous_trait_method_selection() {
        let result = check_text(
            "module test\ntrait LeftShow { fn show(self: &Self) -> String }\ntrait RightShow { fn show(self: &Self) -> String }\nrecord User { name: String }\nimpl LeftShow for User { fn show(self: &Self) -> String { self.name } }\nimpl RightShow for User { fn show(self: &Self) -> String { self.name } }\nfn main() { let user = User { name: \"Nivra\" }\n let text = user.show()\n }\n",
        );
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "TRT005"));
    }

    #[test]
    fn rejects_external_trait_for_external_type() {
        let result = check_text(
            "module test\nuse external.Display\nuse external.User\nimpl Display for User {}\n",
        );
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "TRT006"));
    }

    #[test]
    fn accepts_where_clause_trait_bound() {
        let result = check_text(
            "module test\ntrait Display { fn display(self: &Self) -> String }\nrecord User { name: String }\nimpl Display for User { fn display(self: &Self) -> String { self.name } }\nfn render<T>(value: T) -> String where T: Display { value.display() }\nfn main() { let user = User { name: \"Nivra\" }\n let text = render(user)\n }\n",
        );
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
    }

    #[test]
    fn accepts_default_trait_method_using_required_method() {
        let result = check_text(
            "module test\ntrait Display { fn display(self: &Self) -> String\n fn debug(self: &Self) -> String { self.display() } }\nrecord User { name: String }\nimpl Display for User { fn display(self: &Self) -> String { self.name } }\nfn main() { let user = User { name: \"Nivra\" }\n let text = user.debug()\n }\n",
        );
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert!(result.bindings.iter().any(|binding| {
            binding.name == "text" && binding.ty == Type::String
        }));
    }

    #[test]
    fn accepts_explicit_generic_record_construction() {
        let result = check_text(
            "module test\nrecord Box<T> { value: T }\nfn main() { let boxed = Box<Int> { value: 7 } }\n",
        );
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert!(result.bindings.iter().any(|binding| {
            binding.name == "boxed"
                && binding.ty == Type::Named("Box".to_owned(), vec![Type::Int])
        }));
    }

    #[test]
    fn preserves_nested_explicit_generic_argument_types() {
        let result = check_text(
            "module test\nextern \"C\" { fn make<T>() -> T }\nfn main() { let items = make<List<Int>>() }\n",
        );
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert!(result.bindings.iter().any(|binding| {
            binding.name == "items"
                && binding.ty
                    == Type::Named(
                        "List".to_owned(),
                        vec![Type::Int],
                    )
        }));
    }

    #[test]
    fn rejects_generic_traits_until_the_feature_is_defined() {
        let result = check_text(
            "module test\ntrait Convert<T> { fn convert(self: &Self) -> T }\n",
        );
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "GEN006"));
    }

}
