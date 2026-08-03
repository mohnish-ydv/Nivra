use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temporary_source(name: &str, content: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|error| panic!("{error}"))
        .as_nanos();
    let directory =
        std::env::temp_dir().join(format!("nivra-cli-test-{}-{nonce}", std::process::id()));
    fs::create_dir_all(&directory).unwrap_or_else(|error| panic!("{error}"));
    let path = directory.join(name);
    fs::write(&path, content).unwrap_or_else(|error| panic!("{error}"));
    path
}

#[test]
fn version_reports_d9_foundation() {
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("--version")
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("nivra 0.9.0"));
    assert!(stdout.contains("D9"));
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

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
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

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
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

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
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

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
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

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
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

#[test]
fn typecheck_reports_nominal_members() {
    let path = temporary_source(
        "nominals.nva",
        "module test\nrecord User {\n name: String\n age: Int = 0\n}\nimpl User {\n fn label(self: &Self) -> String { self.name }\n}\nfn main() { let user = User { name: \"M\" }\n let label = user.label()\n }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("typecheck")
        .arg(&path)
        .arg("--nominals")
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("record User"));
    assert!(stdout.contains("field name: String"));
    assert!(stdout.contains("method label"));
}

#[test]
fn check_rejects_missing_record_field() {
    let path = temporary_source(
        "missing_field.nva",
        "module test\nrecord Pair {\n left: Int\n right: Int\n}\nfn main() { let pair = Pair { left: 1 } }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("check")
        .arg(&path)
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("NOM003"));
}

#[test]
fn check_accepts_enum_variant_payload() {
    let path = temporary_source(
        "enum_variant.nva",
        "module test\nenum State {\n idle\n ready(String)\n}\nfn main() { let state = State.ready(\"done\")\n }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("check")
        .arg(&path)
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn check_rejects_enum_record_construction_syntax() {
    let path = temporary_source(
        "enum_record_syntax.nva",
        "module test\nenum State { idle }\nfn main() { let value = State { } }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("check")
        .arg(&path)
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("NOM010"));
}

#[test]
fn explain_supports_nominal_diagnostics() {
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("explain")
        .arg("NOM001")
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("member"));
}

#[test]
fn check_accepts_inferred_generic_function_call() {
    let path = temporary_source(
        "generic_call.nva",
        "module test\nfn identity<T>(value: T) -> T { value }\nfn main() { let number = identity(7)\n }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("check")
        .arg(&path)
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("0 errors"));
}

#[test]
fn check_rejects_unsatisfied_generic_trait_bound() {
    let path = temporary_source(
        "trait_bound.nva",
        "module test\ntrait Display { fn display(self: &Self) -> String }\nfn render<T: Display>(value: T) -> String { value.display() }\nfn main() { render(7) }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("check")
        .arg(&path)
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("GEN004"));
}

#[test]
fn typecheck_reports_traits_and_implementations() {
    let path = temporary_source(
        "traits.nva",
        "module test\ntrait Display { fn display(self: &Self) -> String }\nrecord User { name: String }\nimpl Display for User { fn display(self: &Self) -> String { self.name } }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("typecheck")
        .arg(&path)
        .arg("--traits")
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("trait Display"));
    assert!(stdout.contains("impl Display for User"));
}

#[test]
fn typecheck_json_contains_generic_and_trait_graphs() {
    let path = temporary_source(
        "generic_trait_json.nva",
        "module test\ntrait Display { fn display(self: &Self) -> String }\nfn identity<T>(value: T) -> T { value }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("typecheck")
        .arg(&path)
        .arg("--json")
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"generic_parameters\":["));
    assert!(stdout.contains("\"traits\":["));
    assert!(stdout.contains("\"implementations\":["));
}

#[test]
fn explain_supports_d8_generic_and_trait_diagnostics() {
    for code in ["GEN001", "GEN004", "GEN006", "TRT003", "TRT006"] {
        let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
            .arg("explain")
            .arg(code)
            .output()
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(output.status.success());
        assert!(!output.stdout.is_empty());
    }
}

#[test]
fn check_accepts_explicit_generic_function_call() {
    let path = temporary_source(
        "explicit_generic_call.nva",
        "module test\nfn identity<T>(value: T) -> T { value }\nfn main() { let number = identity<Int>(7)\n }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("check")
        .arg(&path)
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("0 errors"));
}

#[test]
fn check_rejects_invalid_generic_constraint_parameter() {
    let path = temporary_source(
        "invalid_generic_constraint.nva",
        "module test\ntrait Display { fn display(self: &Self) -> String }\nfn choose<T>(value: T) -> T where U: Display { value }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("check")
        .arg(&path)
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("GEN005"));
}

#[test]
fn check_reports_gen005_for_duplicate_generic_parameters() {
    let path = temporary_source(
        "duplicate_generic_parameters.nva",
        "module test\nfn choose<T, T>(value: T) -> T { value }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("check")
        .arg(&path)
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("GEN005"), "{stderr}");
    assert!(!stderr.contains("SEM005"), "{stderr}");
}

#[test]
fn check_reports_unknown_enum_variant_with_suggestion() {
    let path = temporary_source(
        "unknown_enum_variant.nva",
        "module test\nenum State { ready(String) }\nfn main() { let state = State.redy(\"done\") }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("check")
        .arg(&path)
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("NOM001"), "{stderr}");
    assert!(stderr.contains("ready"), "{stderr}");
}

#[test]
fn check_accepts_nested_explicit_generic_argument() {
    let path = temporary_source(
        "nested_generic_argument.nva",
        "module test\nextern \"C\" { fn make<T>() -> T }\nfn main() { let items = make<List<Int>>() }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("check")
        .arg(&path)
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn check_accepts_concrete_default_trait_method() {
    let path = temporary_source(
        "default_trait_method.nva",
        "module test\ntrait Display { fn display(self: &Self) -> String\n fn debug(self: &Self) -> String { self.display() } }\nrecord User { name: String }\nimpl Display for User { fn display(self: &Self) -> String { self.name } }\nfn main() { let user = User { name: \"Nivra\" }\n let text = user.debug()\n }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("check")
        .arg(&path)
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn check_rejects_generic_trait_declaration() {
    let path = temporary_source(
        "generic_trait.nva",
        "module test\ntrait Convert<T> { fn convert(self: &Self) -> T }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("check")
        .arg(&path)
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("GEN006"));
}

#[test]
fn ownership_command_reports_moves_borrows_and_drops() {
    let path = temporary_source(
        "ownership_valid.nva",
        "module test\nfn main() { let text = \"hello\"\n let view = &text\n print(view)\n }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("ownership")
        .arg(&path)
        .args(["--bindings", "--events", "--drops"])
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("OWNERSHIP SUMMARY"));
    assert!(stdout.contains("borrow_shared"));
    assert!(stdout.contains("DEFER AND DROP PLAN"));
}

#[test]
fn ownership_json_contains_machine_readable_flow_graph() {
    let path = temporary_source(
        "ownership_json.nva",
        "module test\nfn consume(value: String) {}\nfn main() { let text = \"hello\"\n consume(text)\n }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("ownership")
        .arg(&path)
        .arg("--json")
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with('{'));
    assert!(stdout.contains("\"bindings\":["));
    assert!(stdout.contains("\"events\":["));
    assert!(stdout.contains("\"exit_actions\":["));
    assert!(stdout.contains("\"moves\":1"));
}

#[test]
fn check_reports_use_after_move_from_ownership_phase() {
    let path = temporary_source(
        "use_after_move.nva",
        "module test\nfn consume(value: String) {}\nfn main() { let text = \"hello\"\n consume(text)\n print(text)\n }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("check")
        .arg(&path)
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("OWN001"));
}

#[test]
fn check_accepts_explicit_move_and_rejects_source_reuse() {
    let path = temporary_source(
        "explicit_move.nva",
        "module test\nfn main() { let source = \"owned\"\n let target = move source\n print(target)\n print(source)\n }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("check")
        .arg(&path)
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("OWN001"));
}

#[test]
fn explain_supports_d9_ownership_and_borrow_diagnostics() {
    for code in ["OWN001", "OWN007", "BOR001", "BOR006", "BOR009"] {
        let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
            .arg("explain")
            .arg(code)
            .output()
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(output.status.success());
        assert!(!output.stdout.is_empty());
    }
}
