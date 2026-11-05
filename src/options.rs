use crate::{Changelog, DetectionError, ErrorKind, Format, MAX_INPUT_BYTES, ParseError};

/// Input-size policy shared by parsing and format detection.
///
/// Defaults to 16 MiB. Disabling this cap does not change Markdown nesting,
/// repeated-metadata expansion, or CLI output limits. Input is still held in memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParseOptions {
    /// Maximum UTF-8 input bytes; `None` permits any size the caller can supply.
    pub max_input_bytes: Option<usize>,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            max_input_bytes: Some(MAX_INPUT_BYTES),
        }
    }
}

impl ParseOptions {
    /// Remove the input-byte cap while retaining all other parser limits.
    ///
    /// ```
    /// use logloom::{Format, ParseOptions};
    /// let options = ParseOptions::without_input_limit();
    /// let changelog = options.parse("## [Unreleased]\n", Format::KeepAChangelog)?;
    /// assert_eq!(changelog.releases.len(), 1);
    /// # Ok::<(), logloom::ParseError>(())
    /// ```
    #[must_use]
    pub const fn without_input_limit() -> Self {
        Self {
            max_input_bytes: None,
        }
    }

    /// Parse with this input-size policy, without I/O.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`crate::parse`], using this cap instead of the default.
    pub fn parse(self, input: &str, format: Format) -> Result<Changelog, ParseError> {
        crate::parse_internal(input, format, None, self)
    }

    /// Parse with this policy and a Towncrier filename hint, without opening it.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`crate::parse_named`], using this input-size policy.
    pub fn parse_named(
        self,
        input: &str,
        format: Format,
        name: &str,
    ) -> Result<Changelog, ParseError> {
        crate::parse_internal(input, format, Some(name), self)
    }

    /// Detect content signatures using this input-size policy.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`crate::detect`], using this cap instead of the default.
    pub fn detect(self, input: &str) -> Result<Format, DetectionError> {
        crate::detection::detect_internal(input, None, self)
    }

    /// Detect with this policy and a Towncrier filename hint, without I/O.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`crate::detect_named`], using this input-size policy.
    pub fn detect_named(self, input: &str, name: &str) -> Result<Format, DetectionError> {
        crate::detection::detect_internal(input, Some(name), self)
    }

    pub(crate) fn check_input(self, input: &str) -> Result<(), ParseError> {
        if let Some(limit) = self.max_input_bytes
            && input.len() > limit
        {
            return Err(ParseError::new(
                ErrorKind::ResourceLimit,
                None,
                if limit == MAX_INPUT_BYTES {
                    "input exceeds the 16 MiB limit".to_owned()
                } else {
                    format!("input exceeds the {limit}-byte limit")
                },
            ));
        }
        Ok(())
    }
}
