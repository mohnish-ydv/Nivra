use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temporary_source(name: &str, content: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|error| panic!("{error}"))
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "nivra-cli-test-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&directory).unwrap_or_else(|error| panic!("{error}"));
    let path = directory.join(name);
    fs::write(&path, content).unwrap_or_else(|error| panic!("{error}"));
    path
}

#[test]
fn version_reports_d6_foundation() {
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("--version")
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("nivra 0.6.0"));
    assert!(stdout.contains("D6"));
}

#[test]
fn check_accepts_syntactically_valid_source() {
    let path = temporary_source(
        "valid.nva",
        "module test\nfn main() {\n    let message = \"hello\"\n    print(message)\n}\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("check")
        .arg(&path)
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("0 errors"));
}

#[test]
fn check_rejects_unterminated_string_with_actionable_code() {
    let path = temporary_source("invalid.nva", "module test\nlet value = \"open\n");
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("check")
        .arg(&path)
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("LEX002"));
    assert!(stderr.contains("close the string"));
}

#[test]
fn check_rejects_missing_block_close() {
    let path = temporary_source("missing_brace.nva", "fn main() {\n let value = 1\n");
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("check")
        .arg(&path)
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("PAR003"));
}

#[test]
fn lex_json_is_machine_readable_in_shape() {
    let path = temporary_source("tokens.nva", "let value = 42\n");
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("lex")
        .arg(&path)
        .arg("--json")
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with('['));
    assert!(stdout.contains("\"kind\":\"keyword(let)\""));
    assert!(stdout.contains("\"kind\":\"integer_literal\""));
    assert!(stdout.trim_end().ends_with(']'));
}

#[test]
fn parse_tree_contains_function_and_binary_nodes() {
    let path = temporary_source("tree.nva", "fn value() { 1 + 2 * 3 }\n");
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("parse")
        .arg(&path)
        .arg("--tree")
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("function_declaration"));
    assert!(stdout.contains("binary_expression"));
}

#[test]
fn parse_summary_confirms_lossless_round_trip() {
    let path = temporary_source(
        "lossless.nva",
        "module demo\n// retained comment\nfn main() {}\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("parse")
        .arg(&path)
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Lossless round trip: PASS"));
}

#[test]
fn parse_json_contains_nested_tree() {
    let path = temporary_source("tree_json.nva", "fn main() { print(1) }\n");
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("parse")
        .arg(&path)
        .arg("--json")
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"kind\":\"source_file\""));
    assert!(stdout.contains("\"kind\":\"call_expression\""));
    assert!(stdout.contains("\"diagnostics\":[]"));
}

#[test]
fn explain_describes_parser_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .args(["explain", "PAR003"])
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("not closed"));
}

#[test]
fn resolve_reports_module_symbols_and_scopes() {
    let path = temporary_source(
        "resolve.nva",
        "module demo.resolve\nuse std.fs\nrecord User { name: String }\nfn load(path: Path) { let text = fs.read_text(path)\n print(text) }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .args(["resolve"])
        .arg(&path)
        .args(["--symbols", "--scopes"])
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Module: demo.resolve"));
    assert!(stdout.contains("SYMBOL TABLE"));
    assert!(stdout.contains("SCOPE TREE"));
    assert!(stdout.contains("User"));
    assert!(stdout.contains("load"));
}

#[test]
fn check_rejects_unresolved_value_name() {
    let path = temporary_source(
        "unresolved.nva",
        "module demo\nfn main() { missing_service() }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("check")
        .arg(&path)
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("SEM003"));
}

#[test]
fn check_rejects_duplicate_local() {
    let path = temporary_source(
        "duplicate.nva",
        "module demo\nfn main() { let value = 1\n let value = 2\n }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("check")
        .arg(&path)
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("SEM002"));
}

#[test]
fn resolve_json_contains_semantic_graph() {
    let path = temporary_source(
        "semantic_json.nva",
        "module demo\nfn echo(value: Int) { print(value) }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("resolve")
        .arg(&path)
        .arg("--json")
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with('{'));
    assert!(stdout.contains("\"module\":\"demo\""));
    assert!(stdout.contains("\"symbols\":["));
    assert!(stdout.contains("\"scopes\":["));
    assert!(stdout.contains("\"resolutions\":["));
}

#[test]
fn explain_describes_semantic_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .args(["explain", "SEM003"])
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("not visible"));
}


#[test]
fn typecheck_reports_functions_and_bindings() {
    let path = temporary_source(
        "types.nva",
        "module demo\nfn add(a: Int, b: Int) -> Int { a + b }\nfn main() { let total = add(1, 2)\n print(total) }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("typecheck")
        .arg(&path)
        .args(["--functions", "--types"])
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("TYPECHECK SUMMARY"));
    assert!(stdout.contains("fn add(a: Int, b: Int) -> Int"));
    assert!(stdout.contains("let total: Int"));
}

#[test]
fn check_rejects_static_type_mismatch() {
    let path = temporary_source(
        "mismatch.nva",
        "module demo\nfn main() { let count: Int = \"two\"\n }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("check")
        .arg(&path)
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("TYP001"));
}

#[test]
fn typecheck_rejects_wrong_arity() {
    let path = temporary_source(
        "arity.nva",
        "module demo\nfn add(a: Int, b: Int) -> Int { a + b }\nfn main() { add(1) }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("typecheck")
        .arg(&path)
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("TYP003"));
}

#[test]
fn typecheck_rejects_non_bool_condition() {
    let path = temporary_source(
        "condition.nva",
        "module demo\nfn main() { if 7 { print(7) } }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("typecheck")
        .arg(&path)
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("TYP007"));
}

#[test]
fn typecheck_json_contains_static_type_graph() {
    let path = temporary_source(
        "types_json.nva",
        "module demo\nfn identity(value: String) -> String { value }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("typecheck")
        .arg(&path)
        .arg("--json")
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with('{'));
    assert!(stdout.contains("\"functions\":["));
    assert!(stdout.contains("\"return_type\":\"String\""));
    assert!(stdout.contains("\"bindings\":["));
    assert!(stdout.contains("\"expressions\":["));
}

#[test]
fn explain_describes_type_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .args(["explain", "TYP004"])
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("parameter type"));
}
