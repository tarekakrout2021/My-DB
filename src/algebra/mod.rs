//! Relational algebra operators for query execution.
//!
//! This module implements the core relational algebra operators used to build
//! and execute query plans.

pub mod iu;
pub mod join;
pub mod print;
pub mod scan;
pub mod select;

use enum_dispatch::enum_dispatch;

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::rc::{Rc, Weak};

pub mod prelude {
    pub use crate::algebra::iu::IURef;
    pub use crate::algebra::join::Join;
    pub use crate::algebra::print::Print;
    pub use crate::algebra::scan::TableScan;
    pub use crate::algebra::select::Select;
    pub use crate::algebra::print::ParOutput;
}

use crate::codegen::Codegen;
use prelude::*;

#[derive(Clone, Debug)]
pub struct TupleCtx {
    // For every IURef we know how to refer to it in code
    //  IURef -> row_1.get(table_0.get_col_index(String::from("col2")).unwrap() as usize).unwrap().clone()
    pub exprs: HashMap<IURef, String>,
}

/// Enumeration of all relational algebra operators.
#[derive(Debug)]
#[enum_dispatch]
pub enum Op {
    /// Join operator for combining data from two inputs
    Join(Join),
    /// Print operator for outputting query results
    Print(Print),
    /// Select operator for filtering rows
    Select(Select),
    /// TableScan operator for reading base table data
    TableScan(TableScan),
}

impl Operator for Op {
    fn prepare(&self, op: &Rc<Op>, required_ius: HashSet<IURef>) -> Result<(), Box<dyn Error>> {
        match self {
            Op::Join(x) => x.prepare(op, required_ius),
            Op::Print(x) => x.prepare(op, required_ius),
            Op::Select(x) => x.prepare(op, required_ius),
            Op::TableScan(x) => x.prepare(op, required_ius),
        }
    }

    fn ius(&self) -> HashSet<IURef> {
        match self {
            Op::Join(x) => x.ius(),
            Op::Print(x) => x.ius(),
            Op::Select(x) => x.ius(),
            Op::TableScan(x) => x.ius(),
        }
    }

    fn produce(&self, cg: &mut Codegen) -> Result<(), Box<dyn Error>> {
        match self {
            Op::Join(x) => x.produce(cg),
            Op::Print(x) => x.produce(cg),
            Op::Select(x) => x.produce(cg),
            Op::TableScan(x) => x.produce(cg),
        }
    }

    fn consume(
        &self,
        cg: &mut Codegen,
        caller: &dyn Operator,
        tuple_ctx: TupleCtx,
    ) -> Result<(), Box<dyn Error>> {
        match self {
            Op::Join(x) => x.consume(cg, caller, tuple_ctx),
            Op::Print(x) => x.consume(cg, caller, tuple_ctx),
            Op::Select(x) => x.consume(cg, caller, tuple_ctx),
            Op::TableScan(x) => x.consume(cg, caller, tuple_ctx),
        }
    }

    fn parent(&self) -> Weak<Op> {
        match self {
            Op::Join(x) => x.parent(),
            Op::Print(x) => x.parent(),
            Op::Select(x) => x.parent(),
            Op::TableScan(x) => x.parent(),
        }
    }

    fn set_parent(&self, parent: Weak<Op>) {
        match self {
            Op::Join(x) => x.set_parent(parent),
            Op::Print(x) => x.set_parent(parent),
            Op::Select(x) => x.set_parent(parent),
            Op::TableScan(x) => x.set_parent(parent),
        }
    }

    fn child(&self, child: &dyn Operator) -> Rc<Op> {
        match self {
            Op::Join(x) => x.child(child),
            Op::Print(x) => x.child(child),
            Op::Select(x) => x.child(child),
            Op::TableScan(x) => x.child(child),
        }
    }

    fn replace_child(&self, child: &dyn Operator, new: Rc<Op>) {
        match self {
            Op::Join(x) => x.replace_child(child, new),
            Op::Print(x) => x.replace_child(child, new),
            Op::Select(x) => x.replace_child(child, new),
            Op::TableScan(x) => x.replace_child(child, new),
        }
    }

    fn optimize(&self, pass: OptimizerPass) {
        match self {
            Op::Join(x) => x.optimize(pass),
            Op::Print(x) => x.optimize(pass),
            Op::Select(x) => x.optimize(pass),
            Op::TableScan(x) => x.optimize(pass),
        }
    }
}
#[derive(PartialEq, Clone)]
pub enum OptimizerPass {
    PredicatePushdown,
}

/// Core trait for all relational algebra operators.
// #[enum_dispatch(Op)]
pub trait Operator: std::fmt::Debug {
    /// Prepares the operator for execution. `op` is the operator itself.
    fn prepare(&self, op: &Rc<Op>, required_ius: HashSet<IURef>) -> Result<(), Box<dyn Error>>;

    /// Returns all IUs of this operator and its child operators.
    fn ius(&self) -> HashSet<IURef>;

    /// Generates code to produce tuples.
    fn produce(&self, codegen: &mut Codegen) -> Result<(), Box<dyn Error>>;

    /// Generates code to consume tuples from a child operator.
    fn consume(
        &self,
        codegen: &mut Codegen,
        caller: &dyn Operator,
        tuple_ctx: TupleCtx,
    ) -> Result<(), Box<dyn Error>>;

    /// Returns a weak reference to this operator's parent.
    fn parent(&self) -> Weak<Op>;

    /// Sets this operator's parent reference.
    fn set_parent(&self, parent: Weak<Op>);

    /// Finds and returns a child operator.
    fn child(&self, child: &dyn Operator) -> Rc<Op>;

    /// Replaces a child operator with a new one.
    fn replace_child(&self, child: &dyn Operator, new: Rc<Op>);

    /// Returns this operator as an Operator trait reference.
    fn as_op(&self) -> &dyn Operator
    where
        Self: Sized,
    {
        self
    }

    /// Returns this operator as a strong reference.
    ///
    /// # Panics
    /// Panics if called before `prepare()` has set up parent references
    fn as_rc(&self) -> Rc<Op>
    where
        Self: Sized,
    {
        let p = self.parent().upgrade().unwrap();
        p.child(self)
    }

    /// Optimizes this operator and its child operators.
    fn optimize(&self, pass: OptimizerPass);
}
