//! Deterministic source-formatting policy facade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    Crlf,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatError {
    InvalidIndent,
    InvalidInput,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatPolicy {
    pub indent: usize,
    pub line_ending: LineEnding,
    pub trailing_newline: bool,
}
impl FormatPolicy {
    pub fn new(indent: usize) -> Result<Self, FormatError> {
        if indent == 0 || indent > 8 {
            return Err(FormatError::InvalidIndent);
        }
        Ok(Self {
            indent,
            line_ending: LineEnding::Lf,
            trailing_newline: true,
        })
    }
    pub fn line_ending(mut self, ending: LineEnding) -> Self {
        self.line_ending = ending;
        self
    }
    pub fn trailing_newline(mut self, enabled: bool) -> Self {
        self.trailing_newline = enabled;
        self
    }
    pub fn normalize(&self, input: &str) -> Result<String, FormatError> {
        if input.chars().any(|c| c == '\0') {
            return Err(FormatError::InvalidInput);
        }
        let mut out = input.replace("\r\n", "\n").replace('\r', "\n");
        let ending = match self.line_ending {
            LineEnding::Lf => "\n",
            LineEnding::Crlf => "\r\n",
        };
        if self.trailing_newline {
            while out.ends_with('\n') {
                out.pop();
            }
            out.push('\n');
        } else {
            while out.ends_with('\n') {
                out.pop();
            }
        }
        if ending != "\n" {
            out = out.replace('\n', ending);
        }
        Ok(out)
    }
}
impl Default for FormatPolicy {
    fn default() -> Self {
        Self::new(2).unwrap()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalizes_line_endings_and_trailing_newline() {
        let p = FormatPolicy::new(2).unwrap().line_ending(LineEnding::Crlf);
        assert_eq!(p.normalize("a\r\nb\r\n").unwrap(), "a\r\nb\r\n");
        assert_eq!(p.trailing_newline(false).normalize("a\n\n").unwrap(), "a")
    }
    #[test]
    fn validates_indent_and_input() {
        assert!(FormatPolicy::new(0).is_err());
        assert!(FormatPolicy::new(9).is_err());
        assert!(FormatPolicy::default().normalize("a\0").is_err())
    }
}
