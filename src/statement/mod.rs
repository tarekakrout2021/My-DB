//! Executable SQL statement implementations.
//!
//! This module defines the Statement trait and concrete implementations for
//! different types of SQL statements. Statements are the result of semantic
//! analysis - they represent validated SQL operations ready for execution.
//!
//! The execution model follows a two-phase approach:
//! 1. **Prepare**: Set up data structures, generate and compile code, etc.
//! 2. **Execute**: Run the prepared code against the database
//!
//! This separation will become clearer in task 4, where SQL operations are
//! compiled to native Rust code.

pub mod copy;
pub mod create;
pub mod select;

use std::error::Error;
use std::io::Write;

use crate::db::Database;

/// Trait for executable SQL statements.
pub trait Statement: std::fmt::Debug {
    /// Prepares the statement for execution.
    ///
    /// This phase performs any necessary setup including:
    /// - Code generation and compilation (Task 4)
    /// - Query optimization (Task 5)
    /// - Data structure initialization
    /// - Resource allocation
    fn prepare(&mut self, db: &mut Database) -> Result<(), Box<dyn Error>>;

    /// Executes the prepared statement.
    ///
    /// Performs the actual database operation using any code or structures
    /// set up during the prepare phase. For DDL statements like CREATE TABLE,
    /// this modifies the database schema. For DML operations, this manipulates data.
    fn execute(&mut self, db: &mut Database, out: &mut dyn Write) -> Result<(), Box<dyn Error>>;
}
