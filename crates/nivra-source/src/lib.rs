//! Source loading, stable source identifiers, byte spans, and Unicode-aware line maps.
//!
//! The compiler uses byte offsets internally because Rust strings are UTF-8. User-facing
//! line and column numbers are one-based and count Unicode scalar values rather than bytes.

use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// Stable identifier assigned by a [`SourceManager`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceId(u32);

impl SourceId {
    /// Creates an identifier from its raw representation.
    #[must_use]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the raw identifier.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// Half-open byte range in one source file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Span {
    source: SourceId,
    start: usize,
    end: usize,
}

impl Span {
    /// Creates a span when `start <= end`.
    #[must_use]
    pub const fn new(source: SourceId, start: usize, end: usize) -> Self {
        Self { source, start, end }
    }

    /// Creates an empty span at `offset`.
    #[must_use]
    pub const fn empty(source: SourceId, offset: usize) -> Self {
        Self::new(source, offset, offset)
    }

    /// Returns the source identifier.
    #[must_use]
    pub const fn source(self) -> SourceId {
        self.source
    }

    /// Returns the first byte offset.
    #[must_use]
    pub const fn start(self) -> usize {
        self.start
    }

    /// Returns the byte offset immediately after the span.
    #[must_use]
    pub const fn end(self) -> usize {
        self.end
    }

    /// Returns the byte length.
    #[must_use]
    pub const fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Returns whether the span is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Returns a span covering both spans when they belong to the same source.
    #[must_use]
    pub fn cover(self, other: Self) -> Option<Self> {
        (self.source == other.source).then(|| {
            Self::new(
                self.source,
                self.start.min(other.start),
                self.end.max(other.end),
            )
        })
    }
}

/// One-based source position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineColumn {
    /// One-based line number.
    pub line: usize,
    /// One-based Unicode scalar column.
    pub column: usize,
}

/// Loaded UTF-8 source file with a precomputed line index.
#[derive(Clone, Debug)]
pub struct SourceFile {
    id: SourceId,
    path: PathBuf,
    text: String,
    line_starts: Vec<usize>,
}

impl SourceFile {
    fn new(id: SourceId, path: PathBuf, text: String) -> Self {
        let mut line_starts = vec![0];
        for (offset, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(offset + 1);
            }
        }
        Self {
            id,
            path,
            text,
            line_starts,
        }
    }

    /// Returns the stable source identifier.
    #[must_use]
    pub const fn id(&self) -> SourceId {
        self.id
    }

    /// Returns the display path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the complete UTF-8 source text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the source length in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// Returns whether the source is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Returns the number of logical lines.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Converts a byte offset into a one-based line and Unicode scalar column.
    ///
    /// Returns `None` when the offset is outside the source or not on a UTF-8 boundary.
    #[must_use]
    pub fn line_column(&self, offset: usize) -> Option<LineColumn> {
        if offset > self.text.len() || !self.text.is_char_boundary(offset) {
            return None;
        }

        let line_index = match self.line_starts.binary_search(&offset) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        };
        let line_start = self.line_starts[line_index];
        let column = self.text[line_start..offset].chars().count() + 1;

        Some(LineColumn {
            line: line_index + 1,
            column,
        })
    }

    /// Returns a one-based line without its trailing newline or carriage return.
    #[must_use]
    pub fn line_text(&self, line: usize) -> Option<&str> {
        let index = line.checked_sub(1)?;
        let start = *self.line_starts.get(index)?;
        let end = self
            .line_starts
            .get(index + 1)
            .copied()
            .unwrap_or(self.text.len());

        let mut content = &self.text[start..end];
        if let Some(stripped) = content.strip_suffix('\n') {
            content = stripped;
        }
        if let Some(stripped) = content.strip_suffix('\r') {
            content = stripped;
        }
        Some(content)
    }

    /// Returns source text covered by a valid span.
    #[must_use]
    pub fn slice(&self, span: Span) -> Option<&str> {
        if span.source != self.id
            || span.start > span.end
            || span.end > self.text.len()
            || !self.text.is_char_boundary(span.start)
            || !self.text.is_char_boundary(span.end)
        {
            return None;
        }
        Some(&self.text[span.start..span.end])
    }

    /// Returns a full-file span.
    #[must_use]
    pub fn full_span(&self) -> Span {
        Span::new(self.id, 0, self.text.len())
    }
}

/// Source loading failure.
#[derive(Debug)]
pub enum SourceError {
    /// File I/O failed.
    Io {
        /// Requested path.
        path: PathBuf,
        /// Original operating-system error.
        source: std::io::Error,
    },
    /// More than `u32::MAX` source files were added.
    TooManySources,
}

impl fmt::Display for SourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(formatter, "could not read `{}`: {source}", path.display())
            }
            Self::TooManySources => formatter.write_str("source manager exhausted SourceId space"),
        }
    }
}

impl Error for SourceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::TooManySources => None,
        }
    }
}

/// Owns all source files used by one compiler invocation.
#[derive(Clone, Debug, Default)]
pub struct SourceManager {
    files: Vec<SourceFile>,
}

impl SourceManager {
    /// Creates an empty source manager.
    #[must_use]
    pub const fn new() -> Self {
        Self { files: Vec::new() }
    }

    /// Returns the number of loaded sources.
    #[must_use]
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Returns whether no sources are loaded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Adds an in-memory source file.
    pub fn add_virtual(
        &mut self,
        path: impl Into<PathBuf>,
        text: impl Into<String>,
    ) -> Result<SourceId, SourceError> {
        let raw = u32::try_from(self.files.len()).map_err(|_| SourceError::TooManySources)?;
        let id = SourceId::from_raw(raw);
        self.files
            .push(SourceFile::new(id, path.into(), text.into()));
        Ok(id)
    }

    /// Reads and adds a UTF-8 source file.
    pub fn load_path(&mut self, path: impl AsRef<Path>) -> Result<SourceId, SourceError> {
        let path = path.as_ref();
        let text = fs::read_to_string(path).map_err(|source| SourceError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        self.add_virtual(path.to_path_buf(), text)
    }

    /// Returns a source file by identifier.
    #[must_use]
    pub fn get(&self, id: SourceId) -> Option<&SourceFile> {
        self.files.get(id.raw() as usize)
    }

    /// Iterates over loaded files in insertion order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &SourceFile> {
        self.files.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::{LineColumn, SourceManager, Span};

    #[test]
    fn maps_unicode_columns_using_characters_not_bytes() {
        let mut manager = SourceManager::new();
        let id = manager
            .add_virtual("unicode.nva", "let नाम = \"ok\"\n")
            .unwrap_or_else(|error| panic!("{error}"));
        let source = manager
            .get(id)
            .unwrap_or_else(|| panic!("source should exist"));
        let offset = source
            .text()
            .find('न')
            .unwrap_or_else(|| panic!("test character should exist"));

        assert_eq!(
            source.line_column(offset),
            Some(LineColumn { line: 1, column: 5 })
        );
    }

    #[test]
    fn handles_crlf_and_final_empty_line() {
        let mut manager = SourceManager::new();
        let id = manager
            .add_virtual("lines.nva", "first\r\nsecond\n")
            .unwrap_or_else(|error| panic!("{error}"));
        let source = manager
            .get(id)
            .unwrap_or_else(|| panic!("source should exist"));

        assert_eq!(source.line_count(), 3);
        assert_eq!(source.line_text(1), Some("first"));
        assert_eq!(source.line_text(2), Some("second"));
        assert_eq!(source.line_text(3), Some(""));
    }

    #[test]
    fn rejects_spans_from_another_source() {
        let mut manager = SourceManager::new();
        let first = manager
            .add_virtual("a.nva", "abc")
            .unwrap_or_else(|error| panic!("{error}"));
        let second = manager
            .add_virtual("b.nva", "xyz")
            .unwrap_or_else(|error| panic!("{error}"));
        let source = manager
            .get(first)
            .unwrap_or_else(|| panic!("source should exist"));

        assert_eq!(source.slice(Span::new(second, 0, 1)), None);
    }

    #[test]
    fn cover_requires_same_source() {
        let left = Span::new(super::SourceId::from_raw(1), 3, 5);
        let right = Span::new(super::SourceId::from_raw(1), 1, 8);
        let other = Span::new(super::SourceId::from_raw(2), 0, 2);

        assert_eq!(left.cover(right), Some(Span::new(left.source(), 1, 8)));
        assert_eq!(left.cover(other), None);
    }
}
