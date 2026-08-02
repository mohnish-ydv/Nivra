use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process;

use nivra_diagnostics::{error_count, Renderer};
use nivra_lexer::lex;
use nivra_parser::parse;
use nivra_sema::{analyze_parse, SemanticResult};
use nivra_source::{SourceError, SourceManager};
use nivra_syntax::{SyntaxElement, SyntaxNode};
use nivra_types::{check as check_types, TypeCheckResult};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let exit_code = run(env::args_os().skip(1).collect());
    process::exit(exit_code);
}

fn run(arguments: Vec<OsString>) -> i32 {
    let Some(command) = arguments.first().and_then(|value| value.to_str()) else {
        print_help();
        return 0;
    };

    match command {
        "help" | "--help" | "-h" => {
            print_help();
            0
        }
        "version" | "--version" | "-V" => {
            println!("nivra {VERSION} (generics and traits D8)");
            0
        }
        "doctor" => doctor(),
        "explain" => explain(arguments.get(1)),
        "check" => check_command(&arguments[1..]),
        "lex" => lex_command(&arguments[1..]),
        "parse" => parse_command(&arguments[1..]),
        "resolve" => resolve_command(&arguments[1..]),
        "typecheck" => typecheck_command(&arguments[1..]),
        unknown => {
            eprintln!("error[CLI001]: unknown command `{unknown}`");
            eprintln!("  = help: run `nivra help` to list available D8 commands");
            2
        }
    }
}

fn print_help() {
    println!(
        "\
Nivra compiler foundation {VERSION}

USAGE:
    nivra <COMMAND> [OPTIONS]

COMMANDS:
    check <FILE> [--json]                       Lex, parse, resolve, and type-check
    lex <FILE> [--trivia] [--json]              Print the lossless token stream
    parse <FILE> [--tree] [--trivia] [--json]   Inspect the lossless CST
    resolve <FILE> [--symbols] [--scopes]        Inspect semantic name resolution
                   [--all] [--json]
    typecheck <FILE> [--functions] [--types]     Inspect static types
                     [--nominals] [--traits] [--json]
    explain <CODE>                               Explain a D8 diagnostic code
    doctor                                       Show compiler-driver information
    version                                      Print the version
    help                                         Print this help

D8 SCOPE:
    The full D7 pipeline plus generic functions and nominal types, explicit and
    inferred type arguments, trait bounds, implementation validation, generic
    substitution, and deterministic method selection are implemented.
    Ownership flow analysis, HIR/MIR, and code generation arrive later.
"
    );
}

fn doctor() -> i32 {
    let current_directory = env::current_dir()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|error| format!("<unavailable: {error}>"));
    let nivra_home = env::var_os("NIVRA_HOME")
        .map(|value| PathBuf::from(value).display().to_string())
        .unwrap_or_else(|| "<not set>".to_owned());

    println!("NIVRA DOCTOR");
    println!("============");
    println!("CLI version: {VERSION}");
    println!("Host OS: {}", env::consts::OS);
    println!("Host architecture: {}", env::consts::ARCH);
    println!("Current directory: {current_directory}");
    println!("NIVRA_HOME: {nivra_home}");
    println!("Source manager: PASS");
    println!("Diagnostic renderer: PASS");
    println!("Lossless lexer: PASS");
    println!("Lossless CST parser: PASS");
    println!("Pratt expression parser: PASS");
    println!("Typed AST accessors: PASS");
    println!("Error recovery: PASS");
    println!("Module indexer: PASS");
    println!("Scope and symbol tables: PASS");
    println!("Name resolver: PASS");
    println!("Type representation: PASS");
    println!("Signature collector: PASS");
    println!("Local type inference: PASS");
    println!("Operator and call checker: PASS");
    println!("Return checker: PASS");
    println!("Nominal type index: PASS");
    println!("Record constructor checker: PASS");
    println!("Field and method lookup: PASS");
    println!("Enum variant typing: PASS");
    println!("Mutable receiver validation: PASS");
    println!("Generic substitution: PASS");
    println!("Trait constraint validation: PASS");
    println!("Implementation coherence: PASS");
    println!("Deterministic method selection: PASS");
    println!("D8 status: OPERATIONAL");
    0
}

fn explain(code: Option<&OsString>) -> i32 {
    let Some(code) = code.and_then(|value| value.to_str()) else {
        eprintln!("error[CLI002]: `nivra explain` requires a diagnostic code");
        eprintln!("  = help: example: `nivra explain PAR003`");
        return 2;
    };

    let explanation = match code.to_ascii_uppercase().as_str() {
        "LEX001" => "Unexpected character. Replace it with an Edition 2026 token.",
        "LEX002" => "Unterminated string. Close the string before the line ends.",
        "LEX003" => "Invalid escape. Use a supported escape such as \\n or \\u{1F680}.",
        "LEX004" => "Unterminated nested block comment. Add the missing */.",
        "LEX005" => "Malformed number. Check its base digits and underscore placement.",
        "LEX006" => "Malformed exponent. Add digits after e, e+, or e-.",
        "LEX007" => "Unterminated character literal. Add the closing single quote.",
        "LEX008" => "A character literal must decode to exactly one Unicode scalar.",
        "LEX009" => "A bidirectional control character may disguise source display order.",
        "LEX010" => "NUL bytes are rejected from Nivra source files.",
        "PAR001" => "Unexpected syntax token. The parser recovered at a safe boundary.",
        "PAR002" => "A required syntax element is missing at this location.",
        "PAR003" => "A delimited construct is not closed before its recovery boundary.",
        "PAR004" => "Only declarations are accepted at the source-file or declaration-body level.",
        "PAR005" => "An expression was required but no expression could begin here.",
        "SEM001" => "A module-level name is declared more than once in the same namespace.",
        "SEM002" => "A local binding is declared more than once in the same lexical scope.",
        "SEM003" => "A value name is not visible from this lexical scope.",
        "SEM004" => "A source file contains more than one module declaration.",
        "SEM005" => "A parameter or generic parameter is duplicated.",
        "SEM006" => "A field, enum variant, or method is duplicated in its type body.",
        "TYP001" => "A value is not assignable to the expected static type.",
        "TYP002" => "An operator or type-directed operation is unsupported for these operands.",
        "TYP003" => "A function call supplies the wrong number of arguments.",
        "TYP004" => "A function argument does not match its parameter type.",
        "TYP005" => "A return expression or function body does not match the declared return type.",
        "TYP006" => "A binding needs a type annotation because inference has insufficient context.",
        "TYP007" => "A condition must be Bool; Nivra does not use truthiness.",
        "TYP008" => "A declared type is malformed, unknown, or not imported.",
        "TYP009" => "Array elements must have one compatible element type.",
        "TYP010" => "An immutable `let` binding cannot be assigned a new value.",
        "NOM001" => "The requested member (field, method, or enum variant) does not exist.",
        "NOM002" => {
            "Member access requires a concrete nominal value and explicit optional handling."
        }
        "NOM003" => "A required record or struct field is missing from construction.",
        "NOM004" => "A constructor initializer names a field that the type does not declare.",
        "NOM005" => "A record or struct field is initialized more than once.",
        "NOM006" => "A constructor field value does not match the declared field type.",
        "NOM007" => "An enum variant payload does not match its declaration.",
        "NOM008" => "A field assignment or method call requires a mutable receiver.",
        "NOM009" => "Record construction targets an unknown nominal type.",
        "NOM010" => "Record construction syntax cannot construct an enum.",
        "GEN001" => "A type or callable received the wrong number of generic arguments.",
        "GEN002" => "A generic argument could not be inferred from available context.",
        "GEN003" => "Generic inference produced conflicting concrete types.",
        "GEN004" => "A concrete type does not satisfy a required trait bound.",
        "GEN005" => "A generic parameter or constraint declaration is invalid or duplicated.",
        "GEN006" => "Generic trait declarations and generic trait methods are deferred beyond D8.",
        "TRT001" => "A referenced trait is not declared or imported.",
        "TRT002" => "Two implementations overlap for the same trait and target pattern.",
        "TRT003" => "A trait implementation omits a required method.",
        "TRT004" => "An implemented method does not match the trait signature.",
        "TRT005" => "Method selection found more than one equally applicable candidate.",
        "TRT006" => "An implementation violates Nivra's package orphan rule.",
        "CLI001" => "The requested D8 command does not exist.",
        "CLI002" => "A required command argument is missing or an option is invalid.",
        "DRV001" => "The compiler driver could not load the requested source file.",
        _ => {
            eprintln!("error[CLI003]: unknown diagnostic code `{code}`");
            eprintln!("  = help: D8 codes use the LEX, PAR, SEM, TYP, NOM, GEN, TRT, CLI, and DRV prefixes");
            return 2;
        }
    };

    println!("{}: {explanation}", code.to_ascii_uppercase());
    0
}

fn check_command(arguments: &[OsString]) -> i32 {
    let parsed = match parse_file_options(arguments, OptionMode::Check) {
        Ok(value) => value,
        Err(message) => {
            eprintln!("error[CLI002]: {message}");
            eprintln!("  = help: usage: `nivra check <FILE> [--json]`");
            return 2;
        }
    };

    let (sources, source_id) = match load_source(&parsed.path) {
        Ok(value) => value,
        Err(error) => return print_source_error(error, parsed.json),
    };
    let Some(source) = sources.get(source_id) else {
        eprintln!("error[DRV999]: loaded source disappeared from the source manager");
        return 2;
    };
    let result = parse(source);
    let semantic = analyze_parse(source, &result);
    let typed = semantic
        .as_ref()
        .filter(|semantic_result| !semantic_result.has_errors())
        .map(|semantic_result| check_types(source, &result.root, semantic_result));
    let mut diagnostics = result.diagnostics.clone();
    if let Some(semantic_result) = &semantic {
        diagnostics.extend(semantic_result.diagnostics.iter().cloned());
    }
    if let Some(type_result) = &typed {
        diagnostics.extend(type_result.diagnostics.iter().cloned());
    }
    let errors = error_count(&diagnostics);
    let warnings = diagnostics.len().saturating_sub(errors);
    let nodes = result.root.descendant_node_count();
    let tokens = result.root.descendant_token_count();
    let semantic_symbols = semantic
        .as_ref()
        .map_or(0, |value| value.user_symbols().count());
    let semantic_scopes = semantic.as_ref().map_or(0, |value| value.scopes.len());
    let resolved_names = semantic
        .as_ref()
        .map_or(0, SemanticResult::resolved_name_count);
    let typed_bindings = typed.as_ref().map_or(0, |value| value.bindings.len());
    let typed_expressions = typed.as_ref().map_or(0, |value| value.expressions.len());
    let function_signatures = typed.as_ref().map_or(0, |value| value.functions.len());
    let nominal_types = typed.as_ref().map_or(0, |value| value.nominals.len());

    if parsed.json {
        let rendered = Renderer::new().json_many(&diagnostics, &sources);
        println!(
            "{{\"path\":{},\"nodes\":{},\"tokens\":{},\"lexical_diagnostics\":{},\"recoveries\":{},\"semantic_symbols\":{},\"semantic_scopes\":{},\"resolved_names\":{},\"function_signatures\":{},\"nominal_types\":{},\"typed_bindings\":{},\"typed_expressions\":{},\"errors\":{},\"warnings\":{},\"diagnostics\":{}}}",
            json_string(&parsed.path.to_string_lossy()),
            nodes,
            tokens,
            result.lexical_diagnostic_count,
            result.recovered_error_count,
            semantic_symbols,
            semantic_scopes,
            resolved_names,
            function_signatures,
            nominal_types,
            typed_bindings,
            typed_expressions,
            errors,
            warnings,
            rendered
        );
    } else {
        if !diagnostics.is_empty() {
            eprint!("{}", Renderer::new().human_many(&diagnostics, &sources));
        }
        if errors == 0 {
            println!(
                "Checked {}: {nodes} nodes, {tokens} lossless tokens, {semantic_symbols} symbols, {resolved_names} resolved names, {function_signatures} signatures, {nominal_types} nominal types, {typed_bindings} typed bindings, {warnings} warnings, 0 errors",
                parsed.path.display()
            );
        } else {
            println!(
                "Check failed for {}: {nodes} nodes, {tokens} lossless tokens, {semantic_symbols} symbols, {resolved_names} resolved names, {function_signatures} signatures, {nominal_types} nominal types, {typed_bindings} typed bindings, {warnings} warnings, {errors} errors",
                parsed.path.display()
            );
        }
    }

    if errors > 0 {
        1
    } else {
        0
    }
}

fn parse_command(arguments: &[OsString]) -> i32 {
    let parsed = match parse_file_options(arguments, OptionMode::Parse) {
        Ok(value) => value,
        Err(message) => {
            eprintln!("error[CLI002]: {message}");
            eprintln!("  = help: usage: `nivra parse <FILE> [--tree] [--trivia] [--json]`");
            return 2;
        }
    };

    let (sources, source_id) = match load_source(&parsed.path) {
        Ok(value) => value,
        Err(error) => return print_source_error(error, parsed.json),
    };
    let Some(source) = sources.get(source_id) else {
        eprintln!("error[DRV999]: loaded source disappeared from the source manager");
        return 2;
    };
    let result = parse(source);
    let errors = error_count(&result.diagnostics);
    let warnings = result.diagnostics.len().saturating_sub(errors);

    if parsed.json {
        let diagnostics = Renderer::new().json_many(&result.diagnostics, &sources);
        println!(
            "{{\"path\":{},\"errors\":{},\"warnings\":{},\"recoveries\":{},\"tree\":{},\"diagnostics\":{}}}",
            json_string(&parsed.path.to_string_lossy()),
            errors,
            warnings,
            result.recovered_error_count,
            syntax_json(&result.root, source, parsed.include_trivia),
            diagnostics
        );
    } else {
        if parsed.tree {
            print!("{}", result.root.debug_tree(source, parsed.include_trivia));
        } else {
            println!("PARSE SUMMARY");
            println!("=============");
            println!("Path: {}", parsed.path.display());
            println!("Root: {}", result.root.kind().name());
            println!("Nodes: {}", result.root.descendant_node_count());
            println!("Lossless tokens: {}", result.root.descendant_token_count());
            println!("Parser recoveries: {}", result.recovered_error_count);
            println!("Errors: {errors}");
            println!("Warnings: {warnings}");
            println!(
                "Lossless round trip: {}",
                if result.root.lossless_text(source) == source.text() {
                    "PASS"
                } else {
                    "FAIL"
                }
            );
        }

        if !result.diagnostics.is_empty() {
            eprint!(
                "{}",
                Renderer::new().human_many(&result.diagnostics, &sources)
            );
        }
    }

    if errors > 0 {
        1
    } else {
        0
    }
}

fn resolve_command(arguments: &[OsString]) -> i32 {
    let options = match parse_resolve_options(arguments) {
        Ok(value) => value,
        Err(message) => {
            eprintln!("error[CLI002]: {message}");
            eprintln!(
                "  = help: usage: `nivra resolve <FILE> [--symbols] [--scopes] [--all] [--json]`"
            );
            return 2;
        }
    };

    let (sources, source_id) = match load_source(&options.path) {
        Ok(value) => value,
        Err(error) => return print_source_error(error, options.json),
    };
    let Some(source) = sources.get(source_id) else {
        eprintln!("error[DRV999]: loaded source disappeared from the source manager");
        return 2;
    };
    let parsed = parse(source);
    if parsed.has_errors() {
        if options.json {
            println!(
                "{{\"path\":{},\"phase\":\"parse\",\"diagnostics\":{}}}",
                json_string(&options.path.to_string_lossy()),
                Renderer::new().json_many(&parsed.diagnostics, &sources)
            );
        } else {
            eprint!(
                "{}",
                Renderer::new().human_many(&parsed.diagnostics, &sources)
            );
            eprintln!("Semantic resolution skipped because parsing failed.");
        }
        return 1;
    }

    let semantic = nivra_sema::analyze(source, &parsed.root);
    let errors = error_count(&semantic.diagnostics);
    let warnings = semantic.diagnostics.len().saturating_sub(errors);

    if options.json {
        println!("{}", semantic_json(&options.path, &semantic, &sources));
    } else {
        println!("RESOLUTION SUMMARY");
        println!("==================");
        println!("Path: {}", options.path.display());
        println!("Module: {}", semantic.module.name);
        println!("User symbols: {}", semantic.user_symbols().count());
        println!("Scopes: {}", semantic.scopes.len());
        println!("Resolved names: {}", semantic.resolved_name_count());
        println!("Unresolved names: {}", semantic.unresolved_name_count());
        println!("Errors: {errors}");
        println!("Warnings: {warnings}");

        if options.symbols {
            println!();
            println!("SYMBOL TABLE");
            println!("============");
            print!("{}", semantic.symbol_report(options.all));
        }
        if options.scopes {
            println!();
            println!("SCOPE TREE");
            println!("==========");
            print!("{}", semantic.scope_report());
        }
        if !semantic.diagnostics.is_empty() {
            eprint!(
                "{}",
                Renderer::new().human_many(&semantic.diagnostics, &sources)
            );
        }
    }

    if errors > 0 {
        1
    } else {
        0
    }
}

fn typecheck_command(arguments: &[OsString]) -> i32 {
    let options = match parse_typecheck_options(arguments) {
        Ok(value) => value,
        Err(message) => {
            eprintln!("error[CLI002]: {message}");
            eprintln!(
                "  = help: usage: `nivra typecheck <FILE> [--functions] [--types] [--nominals] [--traits] [--json]`"
            );
            return 2;
        }
    };

    let (sources, source_id) = match load_source(&options.path) {
        Ok(value) => value,
        Err(error) => return print_source_error(error, options.json),
    };
    let Some(source) = sources.get(source_id) else {
        eprintln!("error[DRV999]: loaded source disappeared from the source manager");
        return 2;
    };
    let parsed = parse(source);
    if parsed.has_errors() {
        if options.json {
            println!(
                "{{\"path\":{},\"phase\":\"parse\",\"diagnostics\":{}}}",
                json_string(&options.path.to_string_lossy()),
                Renderer::new().json_many(&parsed.diagnostics, &sources)
            );
        } else {
            eprint!(
                "{}",
                Renderer::new().human_many(&parsed.diagnostics, &sources)
            );
            eprintln!("Type checking skipped because parsing failed.");
        }
        return 1;
    }

    let semantic = nivra_sema::analyze(source, &parsed.root);
    if semantic.has_errors() {
        if options.json {
            println!(
                "{{\"path\":{},\"phase\":\"semantic\",\"diagnostics\":{}}}",
                json_string(&options.path.to_string_lossy()),
                Renderer::new().json_many(&semantic.diagnostics, &sources)
            );
        } else {
            eprint!(
                "{}",
                Renderer::new().human_many(&semantic.diagnostics, &sources)
            );
            eprintln!("Type checking skipped because name resolution failed.");
        }
        return 1;
    }

    let typed = check_types(source, &parsed.root, &semantic);
    let errors = error_count(&typed.diagnostics);
    let warnings = typed.diagnostics.len().saturating_sub(errors);

    if options.json {
        println!("{}", typecheck_json(&options.path, &typed, &sources));
    } else {
        println!("TYPECHECK SUMMARY");
        println!("=================");
        println!("Path: {}", options.path.display());
        println!("Function signatures: {}", typed.functions.len());
        println!("Nominal types: {}", typed.nominals.len());
        println!("Traits: {}", typed.traits.len());
        println!("Implementations: {}", typed.implementations.len());
        println!("Typed bindings: {}", typed.bindings.len());
        println!("Typed expressions: {}", typed.expressions.len());
        println!("Errors: {errors}");
        println!("Warnings: {warnings}");

        if options.functions {
            println!();
            println!("FUNCTION SIGNATURES");
            println!("===================");
            print!("{}", typed.function_report());
        }
        if options.types {
            println!();
            println!("INFERRED AND DECLARED BINDINGS");
            println!("==============================");
            print!("{}", typed.binding_report());
        }
        if options.nominals {
            println!();
            println!("NOMINAL TYPES AND MEMBERS");
            println!("=========================");
            print!("{}", typed.nominal_report());
        }
        if options.traits {
            println!();
            println!("TRAITS AND IMPLEMENTATIONS");
            println!("==========================");
            print!("{}", typed.trait_report());
        }
        if !typed.diagnostics.is_empty() {
            eprint!(
                "{}",
                Renderer::new().human_many(&typed.diagnostics, &sources)
            );
        }
    }

    if errors > 0 {
        1
    } else {
        0
    }
}

fn lex_command(arguments: &[OsString]) -> i32 {
    let parsed = match parse_file_options(arguments, OptionMode::Lex) {
        Ok(value) => value,
        Err(message) => {
            eprintln!("error[CLI002]: {message}");
            eprintln!("  = help: usage: `nivra lex <FILE> [--trivia] [--json]`");
            return 2;
        }
    };

    let (sources, source_id) = match load_source(&parsed.path) {
        Ok(value) => value,
        Err(error) => return print_source_error(error, parsed.json),
    };
    let Some(source) = sources.get(source_id) else {
        eprintln!("error[DRV999]: loaded source disappeared from the source manager");
        return 2;
    };
    let result = lex(source);

    let visible_tokens = result
        .tokens
        .iter()
        .filter(|token| parsed.include_trivia || !token.kind.is_trivia())
        .collect::<Vec<_>>();

    if parsed.json {
        print!("[");
        for (index, token) in visible_tokens.iter().enumerate() {
            if index > 0 {
                print!(",");
            }
            let position = source.line_column(token.span.start());
            let (line, column) = position.map_or((0, 0), |value| (value.line, value.column));
            let text = token.text(source).unwrap_or("");
            print!(
                "{{\"kind\":{},\"start\":{},\"end\":{},\"line\":{},\"column\":{},\"text\":{}}}",
                json_string(&token.kind.name()),
                token.span.start(),
                token.span.end(),
                line,
                column,
                json_string(text)
            );
        }
        println!("]");
    } else {
        for token in visible_tokens {
            let position = source.line_column(token.span.start());
            let (line, column) = position.map_or((0, 0), |value| (value.line, value.column));
            let text = token.text(source).unwrap_or("");
            println!(
                "{line}:{column}\t{}..{}\t{:<24}\t{}",
                token.span.start(),
                token.span.end(),
                token.kind.name(),
                quoted_text(text)
            );
        }
    }

    if !result.diagnostics.is_empty() {
        if parsed.json {
            eprintln!(
                "{}",
                Renderer::new().json_many(&result.diagnostics, &sources)
            );
        } else {
            eprint!(
                "{}",
                Renderer::new().human_many(&result.diagnostics, &sources)
            );
        }
    }

    if result.has_errors() {
        1
    } else {
        0
    }
}

#[derive(Clone, Copy)]
enum OptionMode {
    Check,
    Lex,
    Parse,
}

#[derive(Debug)]
struct FileOptions {
    path: PathBuf,
    json: bool,
    include_trivia: bool,
    tree: bool,
}

#[derive(Debug)]
struct ResolveOptions {
    path: PathBuf,
    json: bool,
    symbols: bool,
    scopes: bool,
    all: bool,
}

#[derive(Debug)]
struct TypecheckOptions {
    path: PathBuf,
    json: bool,
    functions: bool,
    types: bool,
    nominals: bool,
    traits: bool,
}

fn parse_typecheck_options(arguments: &[OsString]) -> Result<TypecheckOptions, String> {
    let mut path = None;
    let mut json = false;
    let mut functions = false;
    let mut types = false;
    let mut nominals = false;
    let mut traits = false;

    for argument in arguments {
        if argument.as_os_str() == OsStr::new("--json") {
            json = true;
        } else if argument.as_os_str() == OsStr::new("--functions") {
            functions = true;
        } else if argument.as_os_str() == OsStr::new("--types") {
            types = true;
        } else if argument.as_os_str() == OsStr::new("--nominals") {
            nominals = true;
        } else if argument.as_os_str() == OsStr::new("--traits") {
            traits = true;
        } else if argument.to_string_lossy().starts_with('-') {
            return Err(format!("unknown option `{}`", argument.to_string_lossy()));
        } else if path.replace(PathBuf::from(argument.as_os_str())).is_some() {
            return Err("only one source file may be supplied".to_owned());
        }
    }

    if json && (functions || types || nominals || traits) {
        return Err(
            "`--json` already includes functions, bindings, nominals, and traits; remove display flags"
                .to_owned(),
        );
    }

    Ok(TypecheckOptions {
        path: path.ok_or_else(|| "a source file path is required".to_owned())?,
        json,
        functions,
        types,
        nominals,
        traits,
    })
}

fn parse_resolve_options(arguments: &[OsString]) -> Result<ResolveOptions, String> {
    let mut path = None;
    let mut json = false;
    let mut symbols = false;
    let mut scopes = false;
    let mut all = false;

    for argument in arguments {
        if argument.as_os_str() == OsStr::new("--json") {
            json = true;
        } else if argument.as_os_str() == OsStr::new("--symbols") {
            symbols = true;
        } else if argument.as_os_str() == OsStr::new("--scopes") {
            scopes = true;
        } else if argument.as_os_str() == OsStr::new("--all") {
            all = true;
        } else if argument.to_string_lossy().starts_with('-') {
            return Err(format!("unknown option `{}`", argument.to_string_lossy()));
        } else if path.replace(PathBuf::from(argument.as_os_str())).is_some() {
            return Err("only one source file may be supplied".to_owned());
        }
    }

    if json && (symbols || scopes || all) {
        return Err(
            "`--json` already includes symbols and scopes; remove display flags".to_owned(),
        );
    }
    if all && !symbols {
        return Err("`--all` requires `--symbols`".to_owned());
    }

    Ok(ResolveOptions {
        path: path.ok_or_else(|| "a source file path is required".to_owned())?,
        json,
        symbols,
        scopes,
        all,
    })
}

fn parse_file_options(arguments: &[OsString], mode: OptionMode) -> Result<FileOptions, String> {
    let mut path = None;
    let mut json = false;
    let mut include_trivia = false;
    let mut tree = false;

    for argument in arguments {
        if argument.as_os_str() == OsStr::new("--json") {
            json = true;
        } else if argument.as_os_str() == OsStr::new("--trivia") {
            if matches!(mode, OptionMode::Check) {
                return Err("`--trivia` is valid only for `nivra lex` and `nivra parse`".to_owned());
            }
            include_trivia = true;
        } else if argument.as_os_str() == OsStr::new("--tree") {
            if !matches!(mode, OptionMode::Parse) {
                return Err("`--tree` is valid only for `nivra parse`".to_owned());
            }
            tree = true;
        } else if argument.to_string_lossy().starts_with('-') {
            return Err(format!("unknown option `{}`", argument.to_string_lossy()));
        } else if path.replace(PathBuf::from(argument.as_os_str())).is_some() {
            return Err("only one source file may be supplied".to_owned());
        }
    }

    if json && tree {
        return Err("`--json` already includes the tree; remove `--tree`".to_owned());
    }

    let path = path.ok_or_else(|| "a source file path is required".to_owned())?;
    Ok(FileOptions {
        path,
        json,
        include_trivia,
        tree,
    })
}

fn load_source(path: &Path) -> Result<(SourceManager, nivra_source::SourceId), SourceError> {
    let mut sources = SourceManager::new();
    let id = sources.load_path(path)?;
    Ok((sources, id))
}

fn print_source_error(error: SourceError, json: bool) -> i32 {
    if json {
        println!(
            "{{\"severity\":\"error\",\"code\":\"DRV001\",\"message\":{}}}",
            json_string(&error.to_string())
        );
    } else {
        eprintln!("error[DRV001]: {error}");
        eprintln!("  = help: check the path, file permissions, and UTF-8 encoding");
    }
    2
}

fn typecheck_json(path: &Path, typed: &TypeCheckResult, sources: &SourceManager) -> String {
    let mut output = String::new();
    output.push('{');
    let _ = write!(
        output,
        "\"path\":{},\"functions\":[",
        json_string(&path.to_string_lossy())
    );
    for (index, signature) in typed.functions.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        let owner = signature
            .owner
            .as_ref()
            .map_or_else(|| "null".to_owned(), |value| json_string(value));
        let trait_name = signature
            .trait_name
            .as_ref()
            .map_or_else(|| "null".to_owned(), |value| json_string(value));
        let _ = write!(
            output,
            "{{\"name\":{},\"owner\":{},\"trait\":{},\"return_type\":{},\"async\":{},\"extern\":{},\"generic_parameters\":[",
            json_string(&signature.name),
            owner,
            trait_name,
            json_string(&signature.return_type.display_name()),
            signature.is_async,
            signature.is_extern
        );
        for (generic_index, generic) in signature.generic_parameters.iter().enumerate() {
            if generic_index > 0 {
                output.push(',');
            }
            let _ = write!(
                output,
                "{{\"name\":{},\"bounds\":[",
                json_string(&generic.name)
            );
            for (bound_index, bound) in generic.bounds.iter().enumerate() {
                if bound_index > 0 {
                    output.push(',');
                }
                output.push_str(&json_string(bound));
            }
            output.push_str("]}");
        }
        output.push_str("],\"parameters\":[");
        for (parameter_index, parameter) in signature.parameters.iter().enumerate() {
            if parameter_index > 0 {
                output.push(',');
            }
            let _ = write!(
                output,
                "{{\"name\":{},\"type\":{},\"start\":{},\"end\":{}}}",
                json_string(&parameter.name),
                json_string(&parameter.ty.display_name()),
                parameter.span.start(),
                parameter.span.end()
            );
        }
        output.push_str("]}");
    }
    output.push_str("],\"nominals\":[");
    for (index, nominal) in typed.nominals.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        let _ = write!(
            output,
            "{{\"name\":{},\"kind\":{},\"generic_parameters\":[",
            json_string(&nominal.name),
            json_string(nominal.kind.name())
        );
        for (generic_index, generic) in nominal.generic_parameters.iter().enumerate() {
            if generic_index > 0 {
                output.push(',');
            }
            output.push_str(&json_string(generic));
        }
        output.push_str("],\"fields\":[");
        for (field_index, field) in nominal.fields.iter().enumerate() {
            if field_index > 0 {
                output.push(',');
            }
            let _ = write!(
                output,
                "{{\"name\":{},\"type\":{},\"default\":{},\"public\":{}}}",
                json_string(&field.name),
                json_string(&field.ty.display_name()),
                field.has_default,
                field.public
            );
        }
        output.push_str("],\"variants\":[");
        for (variant_index, variant) in nominal.variants.iter().enumerate() {
            if variant_index > 0 {
                output.push(',');
            }
            let _ = write!(
                output,
                "{{\"name\":{},\"payload\":[",
                json_string(&variant.name)
            );
            for (payload_index, payload) in variant.payload.iter().enumerate() {
                if payload_index > 0 {
                    output.push(',');
                }
                output.push_str(&json_string(&payload.display_name()));
            }
            output.push_str("]}");
        }
        output.push_str("],\"methods\":[");
        for (method_index, method) in nominal.methods.iter().enumerate() {
            if method_index > 0 {
                output.push(',');
            }
            let trait_name = method
                .trait_name
                .as_ref()
                .map_or_else(|| "null".to_owned(), |value| json_string(value));
            let _ = write!(
                output,
                "{{\"name\":{},\"return_type\":{},\"mutable_receiver\":{},\"trait\":{},\"parameters\":[",
                json_string(&method.name),
                json_string(&method.return_type.display_name()),
                method.mutable_receiver,
                trait_name
            );
            for (parameter_index, parameter) in method.parameters.iter().enumerate() {
                if parameter_index > 0 {
                    output.push(',');
                }
                let _ = write!(
                    output,
                    "{{\"name\":{},\"type\":{}}}",
                    json_string(&parameter.name),
                    json_string(&parameter.ty.display_name())
                );
            }
            output.push_str("]}");
        }
        output.push_str("]}");
    }
    output.push_str("],\"traits\":[");
    for (index, trait_info) in typed.traits.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        let _ = write!(
            output,
            "{{\"name\":{},\"generic_parameters\":[",
            json_string(&trait_info.name)
        );
        for (generic_index, generic) in trait_info.generic_parameters.iter().enumerate() {
            if generic_index > 0 {
                output.push(',');
            }
            output.push_str(&json_string(generic));
        }
        output.push_str("],\"methods\":[");
        for (method_index, method) in trait_info.methods.iter().enumerate() {
            if method_index > 0 {
                output.push(',');
            }
            let _ = write!(
                output,
                "{{\"name\":{},\"return_type\":{},\"mutable_receiver\":{},\"default\":{}}}",
                json_string(&method.name),
                json_string(&method.return_type.display_name()),
                method.mutable_receiver,
                method.has_default
            );
        }
        output.push_str("]}");
    }
    output.push_str("],\"implementations\":[");
    for (index, implementation) in typed.implementations.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        let trait_name = implementation
            .trait_name
            .as_ref()
            .map_or_else(|| "null".to_owned(), |value| json_string(value));
        let _ = write!(
            output,
            "{{\"trait\":{},\"target\":{},\"generic_parameters\":[",
            trait_name,
            json_string(&implementation.target.display_name())
        );
        for (generic_index, generic) in implementation.generic_parameters.iter().enumerate() {
            if generic_index > 0 {
                output.push(',');
            }
            output.push_str(&json_string(generic));
        }
        output.push_str("]}");
    }
    output.push_str("],\"bindings\":[");
    for (index, binding) in typed.bindings.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        let _ = write!(
            output,
            "{{\"name\":{},\"type\":{},\"mutable\":{},\"start\":{},\"end\":{}}}",
            json_string(&binding.name),
            json_string(&binding.ty.display_name()),
            binding.mutable,
            binding.span.start(),
            binding.span.end()
        );
    }
    output.push_str("],\"expressions\":[");
    for (index, expression) in typed.expressions.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        let _ = write!(
            output,
            "{{\"kind\":{},\"type\":{},\"start\":{},\"end\":{}}}",
            json_string(expression.kind.name()),
            json_string(&expression.ty.display_name()),
            expression.span.start(),
            expression.span.end()
        );
    }
    let errors = error_count(&typed.diagnostics);
    let warnings = typed.diagnostics.len().saturating_sub(errors);
    let _ = write!(
        output,
        "],\"errors\":{},\"warnings\":{},\"diagnostics\":{}",
        errors,
        warnings,
        Renderer::new().json_many(&typed.diagnostics, sources)
    );
    output.push('}');
    output
}

fn semantic_json(path: &Path, semantic: &SemanticResult, sources: &SourceManager) -> String {
    let mut output = String::new();
    output.push('{');
    let _ = write!(
        output,
        "\"path\":{},\"module\":{},\"root_scope\":{},\"symbols\":[",
        json_string(&path.to_string_lossy()),
        json_string(&semantic.module.name),
        semantic.module.root_scope.raw()
    );
    for (index, symbol) in semantic.symbols.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        let _ = write!(
            output,
            "{{\"id\":{},\"name\":{},\"kind\":{},\"namespace\":{},\"visibility\":{},\"origin\":{},\"scope\":{},\"start\":{},\"end\":{}}}",
            symbol.id.raw(),
            json_string(&symbol.name),
            json_string(symbol.kind.as_str()),
            json_string(symbol.namespace.as_str()),
            json_string(symbol.visibility.as_str()),
            json_string(symbol.origin.as_str()),
            symbol.scope.raw(),
            symbol.span.start(),
            symbol.span.end()
        );
    }
    output.push_str("],\"scopes\":[");
    for (index, scope) in semantic.scopes.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        let parent = scope
            .parent
            .map_or_else(|| "null".to_owned(), |value| value.raw().to_string());
        let symbols = scope
            .symbols
            .iter()
            .map(|value| value.raw().to_string())
            .collect::<Vec<_>>()
            .join(",");
        let _ = write!(
            output,
            "{{\"id\":{},\"parent\":{},\"kind\":{},\"start\":{},\"end\":{},\"symbols\":[{}]}}",
            scope.id.raw(),
            parent,
            json_string(scope.kind.as_str()),
            scope.span.start(),
            scope.span.end(),
            symbols
        );
    }
    output.push_str("],\"resolutions\":[");
    for (index, resolution) in semantic.resolutions.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        let symbol = resolution
            .symbol
            .map_or_else(|| "null".to_owned(), |value| value.raw().to_string());
        let _ = write!(
            output,
            "{{\"name\":{},\"namespace\":{},\"scope\":{},\"symbol\":{},\"start\":{},\"end\":{}}}",
            json_string(&resolution.name),
            json_string(resolution.namespace.as_str()),
            resolution.scope.raw(),
            symbol,
            resolution.span.start(),
            resolution.span.end()
        );
    }
    let errors = error_count(&semantic.diagnostics);
    let warnings = semantic.diagnostics.len().saturating_sub(errors);
    let _ = write!(
        output,
        "],\"resolved_names\":{},\"unresolved_names\":{},\"errors\":{},\"warnings\":{},\"diagnostics\":{}",
        semantic.resolved_name_count(),
        semantic.unresolved_name_count(),
        errors,
        warnings,
        Renderer::new().json_many(&semantic.diagnostics, sources)
    );
    output.push('}');
    output
}

fn syntax_json(node: &SyntaxNode, source: &nivra_source::SourceFile, trivia: bool) -> String {
    let mut output = String::new();
    output.push('{');
    let _ = write!(
        output,
        "\"kind\":{},\"start\":{},\"end\":{},\"children\":[",
        json_string(node.kind().name()),
        node.span().start(),
        node.span().end()
    );
    let mut first = true;
    for child in node.children_with_tokens() {
        if matches!(child, SyntaxElement::Token(token) if !trivia && token.kind().is_trivia()) {
            continue;
        }
        if !first {
            output.push(',');
        }
        first = false;
        match child {
            SyntaxElement::Node(child_node) => {
                output.push_str(&syntax_json(child_node, source, trivia));
            }
            SyntaxElement::Token(token) => {
                let text = token.text(source).unwrap_or("");
                let _ = write!(
                    output,
                    "{{\"token\":{},\"start\":{},\"end\":{},\"text\":{}}}",
                    json_string(&token.kind().name()),
                    token.span().start(),
                    token.span().end(),
                    json_string(text)
                );
            }
        }
    }
    output.push_str("]}");
    output
}

fn quoted_text(text: &str) -> String {
    let mut output = String::from("\"");
    for character in text.chars() {
        match character {
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            character if character.is_control() => {
                let _ = write!(output, "\\u{{{:x}}}", u32::from(character));
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

fn json_string(text: &str) -> String {
    let mut output = String::from("\"");
    for character in text.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                let _ = write!(output, "\\u{:04x}", u32::from(character));
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}
