use std::fmt;

#[derive(Debug)]
pub struct CompileError {
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub kind: ErrorKind,
    pub message: String,
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Parse,
    Validate,
    Type,
    Unsupported,
    Internal,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let kind_str = match self.kind {
            ErrorKind::Parse => "parse",
            ErrorKind::Validate => "validate",
            ErrorKind::Type => "type",
            ErrorKind::Unsupported => "unsupported",
            ErrorKind::Internal => "internal",
        };
        write!(f, "{}:{}:{}: {}: {}", self.file, self.line, self.col, kind_str, self.message)?;
        if let Some(ref hint) = self.hint {
            write!(f, "\n  hint: {hint}")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct ErrorCollector {
    errors: Vec<CompileError>,
    file: String,
    max_errors: usize,
}

impl ErrorCollector {
    pub fn new(file: &str) -> Self {
        Self {
            errors: Vec::new(),
            file: file.to_string(),
            max_errors: 50,
        }
    }

    pub fn add(&mut self, line: usize, col: usize, kind: ErrorKind, message: String) {
        self.errors.push(CompileError {
            file: self.file.clone(),
            line, col, kind,
            message,
            hint: None,
        });
    }

    pub fn add_with_hint(&mut self, line: usize, col: usize, kind: ErrorKind, message: String, hint: String) {
        self.errors.push(CompileError {
            file: self.file.clone(),
            line, col, kind,
            message,
            hint: Some(hint),
        });
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn count(&self) -> usize {
        self.errors.len()
    }

    pub fn report(&self) {
        for err in &self.errors {
            eprintln!("{err}");
        }
        if self.errors.len() >= self.max_errors {
            eprintln!("... and more errors (showing first {})", self.max_errors);
        }
        eprintln!("{} error(s) total", self.errors.len());
    }

    pub fn errors(&self) -> &[CompileError] {
        &self.errors
    }
}
