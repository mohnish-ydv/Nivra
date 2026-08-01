//! Structured diagnostics with deterministic phone-friendly human output and JSON output.

use std::fmt::{self, Write as _};

use nivra_source::{SourceManager, Span};

/// Diagnostic severity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Severity {
    /// Compilation cannot continue successfully.
    Error,
    /// Suspicious code that remains compilable.
    Warning,
    /// Additional context.
    Note,
    /// A direct suggestion.
    Help,
}

impl Severity {
    /// Returns the stable lowercase name used by text and JSON output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Note => "note",
            Self::Help => "help",
        }
    }
}

/// Source label attached to a diagnostic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    /// Labeled source range.
    pub span: Span,
    /// Explanation for the range.
    pub message: String,
    /// Whether this is the primary range.
    pub primary: bool,
}

impl Label {
    /// Creates a primary label.
    #[must_use]
    pub fn primary(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            primary: true,
        }
    }

    /// Creates a secondary label.
    #[must_use]
    pub fn secondary(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            primary: false,
        }
    }
}

/// One compiler diagnostic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// Stable code such as `LEX001`.
    pub code: String,
    /// Severity.
    pub severity: Severity,
    /// Main message.
    pub message: String,
    /// Source labels.
    pub labels: Vec<Label>,
    /// Additional notes.
    pub notes: Vec<String>,
    /// Optional actionable fix guidance.
    pub help: Option<String>,
}

impl Diagnostic {
    /// Creates an error.
    #[must_use]
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(Severity::Error, code, message)
    }

    /// Creates a warning.
    #[must_use]
    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, code, message)
    }

    /// Creates a diagnostic.
    #[must_use]
    pub fn new(
        severity: Severity,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
            help: None,
        }
    }

    /// Adds a primary source label.
    #[must_use]
    pub fn with_primary(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label::primary(span, message));
        self
    }

    /// Adds a secondary source label.
    #[must_use]
    pub fn with_secondary(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label::secondary(span, message));
        self
    }

    /// Adds a note.
    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Adds fix guidance.
    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Returns whether this diagnostic is an error.
    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self.severity, Severity::Error)
    }

    fn primary_label(&self) -> Option<&Label> {
        self.labels
            .iter()
            .find(|label| label.primary)
            .or_else(|| self.labels.first())
    }
}

/// Counts error diagnostics.
#[must_use]
pub fn error_count(diagnostics: &[Diagnostic]) -> usize {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.is_error())
        .count()
}

/// Renders diagnostics without terminal-control codes.
///
/// The output is deterministic so it can be used in tests, CI logs, and phone terminals.
#[derive(Clone, Copy, Debug, Default)]
pub struct Renderer;

impl Renderer {
    /// Creates a renderer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Renders one diagnostic as human-readable text.
    #[must_use]
    pub fn human(self, diagnostic: &Diagnostic, sources: &SourceManager) -> String {
        let mut output = String::new();
        let _ = writeln!(
            output,
            "{}[{}]: {}",
            diagnostic.severity.as_str(),
            diagnostic.code,
            diagnostic.message
        );

        if let Some(label) = diagnostic.primary_label() {
            if let Some(source) = sources.get(label.span.source()) {
                if let Some(position) = source.line_column(label.span.start()) {
                    let _ = writeln!(
                        output,
                        " --> {}:{}:{}",
                        source.path().display(),
                        position.line,
                        position.column
                    );

                    if let Some(line_text) = source.line_text(position.line) {
                        let gutter_width = position.line.to_string().len();
                        let _ = writeln!(output, "{:>width$} |", "", width = gutter_width);
                        let _ = writeln!(
                            output,
                            "{:>width$} | {}",
                            position.line,
                            line_text,
                            width = gutter_width
                        );

                        let caret_width =
                            label_width_on_first_line(source, label.span).max(1);
                        let prefix = " ".repeat(position.column.saturating_sub(1));
                        let carets = "^".repeat(caret_width);
                        let _ = writeln!(
                            output,
                            "{:>width$} | {}{} {}",
                            "",
                            prefix,
                            carets,
                            label.message,
                            width = gutter_width
                        );
                    }
                }
            }
        }

        for label in self.secondary_labels(diagnostic) {
            if let Some(source) = sources.get(label.span.source()) {
                if let Some(position) = source.line_column(label.span.start()) {
                    let _ = writeln!(
                        output,
                        "  = related: {}:{}:{}: {}",
                        source.path().display(),
                        position.line,
                        position.column,
                        label.message
                    );
                }
            }
        }

        for note in &diagnostic.notes {
            let _ = writeln!(output, "  = note: {note}");
        }
        if let Some(help) = &diagnostic.help {
            let _ = writeln!(output, "  = help: {help}");
        }

        output
    }

    /// Renders a list of diagnostics as human-readable text.
    #[must_use]
    pub fn human_many(self, diagnostics: &[Diagnostic], sources: &SourceManager) -> String {
        diagnostics
            .iter()
            .map(|diagnostic| self.human(diagnostic, sources))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Renders one diagnostic as a stable JSON object.
    #[must_use]
    pub fn json(self, diagnostic: &Diagnostic, sources: &SourceManager) -> String {
        let mut output = String::new();
        output.push('{');
        json_field(&mut output, "severity", diagnostic.severity.as_str(), true);
        json_field(&mut output, "code", &diagnostic.code, false);
        json_field(&mut output, "message", &diagnostic.message, false);

        output.push_str(",\"labels\":[");
        for (index, label) in diagnostic.labels.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            output.push('{');
            output.push_str(&format!(
                "\"primary\":{},\"start\":{},\"end\":{}",
                label.primary,
                label.span.start(),
                label.span.end()
            ));
            json_field(&mut output, "message", &label.message, false);

            if let Some(source) = sources.get(label.span.source()) {
                json_field(
                    &mut output,
                    "path",
                    &source.path().to_string_lossy(),
                    false,
                );
                if let Some(position) = source.line_column(label.span.start()) {
                    output.push_str(&format!(
                        ",\"line\":{},\"column\":{}",
                        position.line, position.column
                    ));
                }
            }
            output.push('}');
        }
        output.push(']');

        output.push_str(",\"notes\":[");
        for (index, note) in diagnostic.notes.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            json_string(&mut output, note);
        }
        output.push(']');

        output.push_str(",\"help\":");
        match &diagnostic.help {
            Some(help) => json_string(&mut output, help),
            None => output.push_str("null"),
        }
        output.push('}');
        output
    }

    /// Renders diagnostics as a JSON array.
    #[must_use]
    pub fn json_many(self, diagnostics: &[Diagnostic], sources: &SourceManager) -> String {
        let body = diagnostics
            .iter()
            .map(|diagnostic| self.json(diagnostic, sources))
            .collect::<Vec<_>>()
            .join(",");
        format!("[{body}]")
    }

    fn secondary_labels<'a>(
        self,
        diagnostic: &'a Diagnostic,
    ) -> impl Iterator<Item = &'a Label> {
        diagnostic.labels.iter().filter(|label| !label.primary)
    }
}

fn label_width_on_first_line(source: &nivra_source::SourceFile, span: Span) -> usize {
    let Some(start) = source.line_column(span.start()) else {
        return 1;
    };
    let end_offset = span.end().min(source.len());
    let Some(end) = source.line_column(end_offset) else {
        return 1;
    };

    if start.line == end.line {
        end.column.saturating_sub(start.column)
    } else {
        source
            .line_text(start.line)
            .map_or(1, |line| line.chars().count().saturating_sub(start.column - 1))
    }
}

fn json_field(output: &mut String, key: &str, value: &str, first: bool) {
    if !first {
        output.push(',');
    }
    json_string(output, key);
    output.push(':');
    json_string(output, value);
}

fn json_string(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
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
}

impl fmt::Display for Severity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use nivra_source::{SourceManager, Span};

    use super::{Diagnostic, Renderer, Severity, error_count};

    #[test]
    fn renders_phone_friendly_human_diagnostic() {
        let mut sources = SourceManager::new();
        let id = sources
            .add_virtual("sample.nva", "let answer = @\n")
            .unwrap_or_else(|error| panic!("{error}"));
        let diagnostic = Diagnostic::error("LEX001", "unexpected character")
            .with_primary(Span::new(id, 13, 14), "`@` is not valid here")
            .with_help("remove the character");

        let rendered = Renderer::new().human(&diagnostic, &sources);
        assert!(rendered.contains("error[LEX001]: unexpected character"));
        assert!(rendered.contains("sample.nva:1:14"));
        assert!(rendered.contains("^ `@` is not valid here"));
        assert!(rendered.contains("help: remove the character"));
    }

    #[test]
    fn renders_valid_json_with_escaped_content() {
        let mut sources = SourceManager::new();
        let id = sources
            .add_virtual("quote.nva", "\"")
            .unwrap_or_else(|error| panic!("{error}"));
        let diagnostic = Diagnostic::warning("LEX999", "a \"quoted\" warning")
            .with_primary(Span::new(id, 0, 1), "quote");

        let rendered = Renderer::new().json(&diagnostic, &sources);
        assert!(rendered.starts_with('{'));
        assert!(rendered.contains("\"severity\":\"warning\""));
        assert!(rendered.contains("a \\\"quoted\\\" warning"));
        assert!(rendered.ends_with('}'));
    }

    #[test]
    fn counts_only_errors() {
        let diagnostics = vec![
            Diagnostic::new(Severity::Error, "A", "error"),
            Diagnostic::new(Severity::Warning, "B", "warning"),
        ];
        assert_eq!(error_count(&diagnostics), 1);
    }
}
