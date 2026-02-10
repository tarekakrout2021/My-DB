//! Scope management for column name resolution in SQL queries.
//!
//! This module handles the complex task of resolving column names to their
//! corresponding Information Units (IUs) during semantic analysis. It manages
//! table aliases, column aliases, and ensures unambiguous column references.

use crate::algebra::iu::IURef;
use crate::parser::ast::ColId;
use crate::semana::semana_errors::SemanticError;

use indexmap::IndexMap;

/// Maps a column alias to its corresponding Information Unit.
///
/// IUMap represents the binding between a column name (as referenced in the query)
/// and the actual IU object that provides the column's metadata and identity.
/// The alias might be different from the IU's actual name due to renaming columns.
pub struct IUMap {
    /// The alias name used to reference this column (e.g., "customer_id", "id")
    pub alias: String,
    /// The actual Information Unit containing column metadata
    pub iu: IURef,
}

/// Manages column name resolution and scope during semantic analysis.
///
/// The Scope tracks which columns are available in the current query context
/// and resolves column references to their corresponding IUs.
#[derive(Default)]
pub struct Scope {
    /// Maps table names/aliases to their available columns.
    ///
    /// The HashMap key is the table name or alias (e.g., "customers", "c").
    /// The table name/alias may be empty, e.g., for unnamed subqueries.
    /// The inner Vec maintains column order and maps column aliases to IUs.
    pub ius: IndexMap<String, Vec<IUMap>>,
}

impl Scope {
    /// Resolves a column reference to its corresponding Information Unit.
    ///
    /// Searches the scope for a column matching the given reference and returns
    /// its IURef if found and unambiguous.
    pub fn iu(&self, c: &ColId) -> Result<IURef, SemanticError> {
        let table = c.table.trim();
        let column = c.column.trim();

        // Case 1: Qualified reference (table.column)
        if !table.is_empty() {
            if let Some(cols) = self.ius.get(table) {
                for m in cols {
                    if m.alias == column {
                        return Ok(m.iu.clone());
                    }
                }
                return Err(SemanticError::UnknownColumnInTable(
                    table.to_string(),
                    column.to_string(),
                ));
            } else {
                return Err(SemanticError::UnknownTable(table.to_string()));
            }
        }

        // Case 2: Unqualified reference (column only)
        let mut found: Option<IURef> = None;
        for (_tbl_alias, cols) in &self.ius {
            for m in cols {
                if m.alias == column {
                    if found.is_some() {
                        // Ambiguous across multiple tables
                        return Err(SemanticError::AmbiguousColumn(column.to_string()));
                    }
                    found = Some(m.iu.clone());
                }
            }
        }

        found.ok_or_else(|| SemanticError::UnknownColumn(column.to_string()))
    }

    /// Adds IUs of a table or subquery to the scope. Errors if the table alias
    /// exists in the scope already.
    pub fn add_ius(&mut self, alias: &str, ius: Vec<IUMap>) -> Result<(), SemanticError> {
        if self.ius.contains_key(alias) {
            return Err(SemanticError::DuplicateAlias(alias.to_string()));
        }
        self.ius.insert(alias.to_string(), ius);
        Ok(())
    }

    /// Returns all visible IUs (flattened across all aliases).
    pub fn all_ius(&self) -> Vec<IURef> {
        self.ius
            .values()
            .flat_map(|cols| cols.iter().map(|m| m.iu.clone()))
            .collect()
    }
}
