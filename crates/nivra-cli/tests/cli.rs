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
fn version_reports_d3_foundation() {
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("--version")
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("nivra 0.3.0"));
    assert!(stdout.contains("D3"));
}

#[test]
fn check_accepts_lexically_valid_source() {
    let path = temporary_source(
        "valid.nva",
        "module test\nfn main() {\n    let message = \"hello\"\n}\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .arg("check")
        .arg(&path)
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(output.status.success());
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
fn explain_describes_known_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_nivra"))
        .args(["explain", "LEX005"])
        .output()
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Malformed number"));
}
