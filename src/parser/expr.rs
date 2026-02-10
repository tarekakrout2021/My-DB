//! Expression types for SQL predicates and computations.

use std::error::Error;

use crate::algebra::TupleCtx;
use crate::algebra::iu::IURef;
use crate::parser::ast::ColId;
use crate::parser::ast::Query;

/// Represents any SQL expression that can be evaluated.
///
/// Expressions form a tree structure for complex computations and predicates.
/// The same Expression type is used everywhere for simplicity, though it
/// transitions from parser-created to semantically-analyzed Expression.
#[derive(Clone, Debug, PartialEq)]
pub enum Expression {
    /// String constant literal
    String(String),
    /// Column reference from parser (e.g., "table.column" or "column")
    ColumnRef(ColId),
    /// Resolved column reference after semantic analysis
    IURef(IURef),
    /// Binary operation between two sub-expressions
    BinaryExpr(Box<BinaryExpression>),
    /// Subquery expression (e.g., (SELECT ...))
    Subquery(Box<Query>),
}

impl Expression {
    /// Generates code that reflects the expression tree.
    pub fn produce(&self, ctx: &TupleCtx) -> String {
        match self {
            Expression::IURef(iu) => ctx
                .exprs
                .get(iu)
                .unwrap_or_else(|| panic!("IURef not found in TupleCtx for predicate"))
                .clone(),
            Expression::String(s) => format!("ValueRef::Integer({})", s),

            Expression::ColumnRef(col) => {
                panic!(
                    "ColumnRef {:?} in Select::expr_to_code – expected IURef after semana",
                    col
                );
            }
            Expression::BinaryExpr(_) => {
                unimplemented!("Nested BinaryExpr in WHERE not supported yet");
            }
            Expression::Subquery(_) => {
                panic!("Subquery expression should be removed by semantic analysis before codegen");
            }
        }
    }
}

/// Binary operators supported in expressions.
///
/// Currently limited to equality for basic WHERE clause support.
/// Could be extended with arithmetic and comparison operators.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum BinaryOperator {
    /// Equality comparison (=)
    Eq,
}

/// Binary expression combining two operands with an operator.
#[derive(Clone, Debug, PartialEq)]
pub struct BinaryExpression {
    /// Left operand
    pub l: Expression,
    /// Right operand
    pub r: Expression,
    /// Binary operator
    pub op: BinaryOperator,
}

impl BinaryExpression {
    /// Creates a new binary expression and wraps it in an Expression.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        op: BinaryOperator,
        left: Expression,
        right: Expression,
    ) -> Result<Expression, Box<dyn Error>> {
        Ok(Expression::from(BinaryExpression {
            l: left,
            r: right,
            op,
        }))
    }
}

impl From<BinaryExpression> for Expression {
    /// Converts a BinaryExpression into an Expression::BinaryExpr variant.
    fn from(value: BinaryExpression) -> Self {
        Self::BinaryExpr(Box::new(value))
    }
}
