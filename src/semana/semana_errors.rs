use thiserror::Error;

#[derive(Error, Debug, Eq, PartialEq)]
pub enum SemanticError {
    // create and copy errors
    #[error("Table already exists")]
    TableExists,
    #[error("Some column name got repeated at least twice")]
    RepeatedColumnNames,
    #[error("The key '{0}' is not found in columns")]
    KeyNotInColumns(String),
    #[error("The column '{0}' has to be defined with not null")]
    ColumnNotNull(String),
    #[error("File '{0}' is not found.")]
    FileNotFound(String),
    #[error("Table '{0}' is not found the database.")]
    TableNotExists(String),

    //select errors
    #[error("Unknown table '{0}'")]
    UnknownTable(String),
    #[error("Unknown column '{0}'")]
    UnknownColumn(String),
    #[error("Unknown column '{1}' in table '{0}'")]
    UnknownColumnInTable(String, String),
    #[error("Ambiguous column reference '{0}'")]
    AmbiguousColumn(String),
    #[error("Duplicate table alias '{0}'")]
    DuplicateAlias(String),
    #[error("Type mismatch between '{0}' and '{1}' in expression")]
    TypeMismatch(String, String),
}
