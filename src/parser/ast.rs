//! Abstract Syntax Tree (AST) definitions for parsed statements.
//!
//! This module defines the AST node types that represent parsed SQL statements
//! before semantic analysis. The AST captures the syntactic structure of SQL
//! but doesn't perform validation or type checking - that's done in the semantic
//! analysis phase.

/// Root AST node representing multiple parsed SQL statements.
#[derive(Debug)]
#[allow(clippy::upper_case_acronyms)]
pub struct AST {
    // The semantic analysis will convert these AST statements into executable Statement objects
    pub statements: Vec<Statement>,
}

// ============================================================================
// SQL AST Nodes
// ============================================================================

/// AST node type enumeration for SQL statements.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum Statement {
    CreateTable(CreateTable),
    CopyTable(CopyTable),
    /// SELECT
    Select(Query),
}

/// AST node for CREATE TABLE statements.
#[derive(Debug, Clone, PartialEq)]
pub struct CreateTable {
    pub table: Table,
}

/// A single table definition.
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    /// Name of the table
    pub id: String,
    /// Columns in the table
    pub columns: Vec<Column>,
    /// Primary key columns
    pub primary_key: Vec<String>,
}

/// A single column definition.
#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    /// Name of the column
    pub id: String,
    /// Type of the column
    pub r#type: crate::types::Type,
    pub not_null: bool,
}

/// AST node for COPY TABLE statements.
#[derive(Debug, Clone, PartialEq)]
pub struct CopyTable {
    /// Table to copy data into
    pub table: String,
    /// File to copy data from
    pub file: String,
    /// Delimiter used in the file (default is '|' for TPC-C files)
    pub delimiter: String,
}

/// AST node for SELECT queries.
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    /// Target columns or expressions (SELECT clause)
    pub targets: Option<Vec<ColId>>,
    /// Tables to select from (FROM clause)
    pub from: Vec<TableFactor>,
    /// WHERE clause conditions
    pub r#where: Vec<BinaryExpression>,
}

/// AST node for column identifiers (e.g., table.column or column).
#[derive(Debug, Clone, PartialEq)]
pub struct ColId {
    /// Table name (empty if not qualified)
    pub table: String,
    /// Column name
    pub column: String,
}

/// AST node for table references (e.g., table or table alias).
#[derive(Debug, Clone, PartialEq)]
pub struct TableRef {
    /// Table name
    pub table: String,
    /// Table alias (empty if no alias)
    pub alias: String,
}

/// TableFactor represents either a named table reference or a derived table (subquery with alias).
#[derive(Debug, Clone, PartialEq)]
pub enum TableFactor {
    /// Simple named table (wraps existing TableRef)
    Table(TableRef),
    /// Derived table from a subquery: (SELECT ...) alias
    Derived { query: Box<Query>, alias: String },
}

/// AST node for binary expressions.
use crate::parser::expr::BinaryExpression;
