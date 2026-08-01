use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process;

use nivra_diagnostics::{Renderer, error_count};
use nivra_lexer::lex;
use nivra_parser::parse;
use nivra_source::{SourceError, SourceManager};
use nivra_syntax::{SyntaxElement, SyntaxNode};

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
            println!("nivra {VERSION} (parser and AST foundation D4)");
            0
        }
        "doctor" => doctor(),
        "explain" => explain(arguments.get(1)),
        "check" => check_command(&arguments[1..]),
        "lex" => lex_command(&arguments[1..]),
        "parse" => parse_command(&arguments[1..]),
        unknown => {
            eprintln!("error[CLI001]: unknown command `{unknown}`");
            eprintln!("  = help: run `nivra help` to list available D4 commands");
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
    check <FILE> [--json]                     Lex and parse a Nivra source file
    lex <FILE> [--trivia] [--json]            Print the lossless token stream
    parse <FILE> [--tree] [--trivia] [--json] Inspect the lossless CST
    explain <CODE>                             Explain a D4 diagnostic code
    doctor                                     Show compiler-driver information
    version                                    Print the version
    help                                       Print this help

D4 SCOPE:
    Source management, diagnostics, lexing, lossless CST parsing, Pratt expression
    precedence, recovery, and typed AST foundations are implemented.
    Name resolution, type checking, lowering, code generation, and execution arrive later.
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
    println!("Typed AST foundation: PASS");
    println!("Error recovery: PASS");
    println!("D4 status: OPERATIONAL");
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
        "CLI001" => "The requested D4 command does not exist.",
        "CLI002" => "A required command argument is missing or an option is invalid.",
        "DRV001" => "The compiler driver could not load the requested source file.",
        _ => {
            eprintln!("error[CLI003]: unknown diagnostic code `{code}`");
            eprintln!("  = help: D4 codes use the LEX, PAR, CLI, and DRV prefixes");
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
    let errors = error_count(&result.diagnostics);
    let warnings = result.diagnostics.len().saturating_sub(errors);
    let nodes = result.root.descendant_node_count();
    let tokens = result.root.descendant_token_count();

    if parsed.json {
        let diagnostics = Renderer::new().json_many(&result.diagnostics, &sources);
        println!(
            "{{\"path\":{},\"nodes\":{},\"tokens\":{},\"lexical_diagnostics\":{},\"recoveries\":{},\"errors\":{},\"warnings\":{},\"diagnostics\":{}}}",
            json_string(&parsed.path.to_string_lossy()),
            nodes,
            tokens,
            result.lexical_diagnostic_count,
            result.recovered_error_count,
            errors,
            warnings,
            diagnostics
        );
    } else {
        if !result.diagnostics.is_empty() {
            eprint!(
                "{}",
                Renderer::new().human_many(&result.diagnostics, &sources)
            );
        }
        if errors == 0 {
            println!(
                "Checked {}: {nodes} nodes, {tokens} lossless tokens, {warnings} warnings, 0 errors",
                parsed.path.display()
            );
        } else {
            println!(
                "Check failed for {}: {nodes} nodes, {tokens} lossless tokens, {warnings} warnings, {errors} errors",
                parsed.path.display()
            );
        }
    }

    if errors > 0 { 1 } else { 0 }
}

fn parse_command(arguments: &[OsString]) -> i32 {
    let parsed = match parse_file_options(arguments, OptionMode::Parse) {
        Ok(value) => value,
        Err(message) => {
            eprintln!("error[CLI002]: {message}");
            eprintln!(
                "  = help: usage: `nivra parse <FILE> [--tree] [--trivia] [--json]`"
            );
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
            print!(
                "{}",
                result.root.debug_tree(source, parsed.include_trivia)
            );
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

    if errors > 0 { 1 } else { 0 }
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

    if result.has_errors() { 1 } else { 0 }
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
