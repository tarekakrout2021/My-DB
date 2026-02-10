//! Information Units (IUs) - metadata for columns in operator trees.
//!
//! Information Units represent columns flowing through the relational algebra
//! operator tree. They track column names, types, and are used in query
//! optimization and code generation.
//!
//! ## IU vs IURef
//!
//! - [`IU`] - The actual column (name and type)
//! - [`IURef`] - A reference-counted pointer to an IU
//!
//! IURef uses pointer instead of value equality, meaning two IURefs are equal
//! only if they point to the same IU object in memory.

use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::Rc;

use crate::types::Type;

/// Information Unit representing column metadata in the algebra tree.
///
/// Contains the essential information about a column as it flows through
/// relational operators - its name and data type. IUs are created in
/// table scans, for example, and flow through the operator tree.
///
/// # Examples
/// ```
/// use imlab::algebra::iu::IU;
/// use imlab::types::Type;
///
/// let column = IU {
///     name: "customer_id".to_string(),
///     r#type: Type::new_integer(),
/// };
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct IU {
    /// Column name (e.g., "customer_id") for debugging purposes
    pub name: String,
    /// Data type of the column
    pub r#type: Type,
}

/// Reference-counted pointer to an IU.
///
/// IURef wraps `Rc<IU>` but uses pointer equality instead of value equality.
/// Two IURef instances are considered equal only if they point to the exact
/// same IU object in memory. This is essential for tracking column identity
/// through query optimization.
///
/// # Why pointer equality?
///
/// Consider a self-join: `SELECT a.id, b.id FROM table a, table b`
/// Both columns have the same name ("id") and type, but they're logically
/// different. IURef's identity semantics distinguish them correctly.
#[derive(Debug, Clone)]
pub struct IURef(pub Rc<IU>);

impl PartialEq for IURef {
    fn eq(&self, other: &IURef) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for IURef {}

impl Hash for IURef {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        hasher.write_usize(Rc::as_ptr(&self.0) as usize);
    }
}

impl Deref for IURef {
    type Target = Rc<IU>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_iu_ref_same_iu() {
        let iu = Rc::new(IU {
            name: "a".to_string(),
            r#type: Type::new_integer(),
        });

        let mut set: HashSet<IURef> = HashSet::new();

        set.insert(IURef(iu.clone()));
        set.insert(IURef(iu));

        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_iu_ref_different_ius() {
        let iu1 = Rc::new(IU {
            name: "a".to_string(),
            r#type: Type::new_integer(),
        });
        let iu2 = Rc::new(IU {
            name: "a".to_string(),
            r#type: Type::new_integer(),
        });

        let mut set: HashSet<IURef> = HashSet::new();

        set.insert(IURef(iu1));
        set.insert(IURef(iu2));

        assert_eq!(set.len(), 2);
    }
}
