mod validate;

pub use validate::validate_source;

use tree_sitter::{Parser, Tree};

pub fn parse_c(source: &str) -> Result<Tree, ParseError> {
    let mut parser = Parser::new();
    let language = tree_sitter_c::LANGUAGE;
    parser
        .set_language(&language.into())
        .map_err(|_| ParseError::LanguageError)?;
    parser.parse(source, None).ok_or(ParseError::ParseFailed)
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("failed to load C grammar")]
    LanguageError,
    #[error("tree-sitter parse failed")]
    ParseFailed,
}
