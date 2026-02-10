//! Print operator for outputting query results.

use std::cell::RefCell;
use std::collections::HashSet;
use std::error::Error;
use std::rc::{Rc, Weak};

use crate::algebra::iu::IURef;
use crate::algebra::{Op, Operator, OptimizerPass, TupleCtx};
use crate::codegen::Codegen;

/// Print operator that outputs query results.
///
/// Serves as the root operator in our query tree, receiving the final result
/// tuples and formatting them for display.
#[derive(Debug)]
pub struct Print {
    /// Reference to parent operator (itself for root nodes)
    pub parent: RefCell<Weak<Op>>,
    /// Child operator providing tuples
    pub child: RefCell<Rc<Op>>,
    pub iu_refs_set: HashSet<IURef>,
    pub iu_refs_vec: Vec<IURef>,
}

impl Print {
    /// Creates a new Print operator with the specified child.
    pub fn new(child: Rc<Op>) -> Self {
        Self {
            parent: RefCell::new(Weak::new()),
            child: RefCell::new(child),
            iu_refs_set: HashSet::new(),
            iu_refs_vec: Vec::new(),
        }
    }
}

impl Operator for Print {
    fn prepare(&self, op: &Rc<Op>, required_ius: HashSet<IURef>) -> Result<(), Box<dyn Error>> {
        let child_rc = self.child.borrow().clone();
        child_rc.set_parent(Rc::downgrade(op));
        child_rc.prepare(&child_rc, required_ius)?;
        Ok(())
    }

    fn ius(&self) -> HashSet<IURef> {
        self.iu_refs_set.clone()
    }

    fn produce(&self, codegen: &mut Codegen) -> Result<(), Box<dyn Error>> {
        codegen.line("// Print operator produce");
        codegen.line("extern crate imlab;");
        codegen.line("use std::error::Error;");
        codegen.line("use imlab::db::Database;");
        codegen.line("use imlab::db::Value;");
        codegen.line("use imlab::types::integer::Integer;");
        codegen.line("use imlab::types::text::Text;");
        codegen.line("use imlab::types::numeric::Numeric;");
        codegen.line("use imlab::types::timestamp::Timestamp;");
        codegen.line("use imlab::types::bool::Bool;");
        codegen.line("use imlab::db::Key;");
        codegen.line("use imlab::infra::map::LazyMultiMapBuilder;");
        codegen.line("use imlab::infra::map::LazyMultiMapParBuilder;");
        codegen.line("use imlab::db::row::RowView;");
        codegen.line("use imlab::db::row::ValueRef;");
        codegen.line("use imlab::rayon::prelude::*;");
        codegen.line("use imlab::algebra::print::ParOutput;");

        codegen.start_function("main_query");
        codegen.line("let MORSEL_SIZE = 256;");
        codegen.line("let out = ParOutput::new();");

        let child_rc = self.child.borrow().clone();
        child_rc.produce(codegen)?;

        codegen.line("let (text, rows) = out.finalize();");
        codegen.line("print!(\"{}\", text);");
        codegen.line("println!(\"\");");
        codegen.line("println!(\"Total rows is : {}\", rows);");
        codegen.end_function();
        Ok(())
    }

    fn consume(
        &self,
        codegen: &mut Codegen,
        _caller: &dyn Operator,
        tuple_ctx: TupleCtx,
    ) -> Result<(), Box<dyn Error>> {
        // codegen.line("println!(\"{:?}\", row);");
        codegen.line("// Print operator consume");
        let mut curly_braces_line = "".to_string();
        for _ in &self.iu_refs_set {
            curly_braces_line.push_str("{} | ");
        }
        codegen.line(format!("let line = format!(\" {} \", ", curly_braces_line));
        for iu in &self.iu_refs_vec {
            let expr = tuple_ctx.exprs.get(iu).unwrap();
            codegen.line(format!("{},  ", expr));
        }
        codegen.line(");");
        codegen.line("out.push_line(&line);");
        // codegen.line("out_count += 1;");

        Ok(())
    }

    fn parent(&self) -> Weak<Op> {
        self.parent.borrow().clone()
    }

    fn set_parent(&self, parent: Weak<Op>) {
        *self.parent.borrow_mut() = parent;
    }

    fn child(&self, _child: &dyn Operator) -> Rc<Op> {
        self.child.borrow().clone()
    }

    fn replace_child(&self, _child: &dyn Operator, new: Rc<Op>) {
        *self.child.borrow_mut() = new;
    }

    fn optimize(&self, pass: OptimizerPass) {
        let child_rc = self.child.borrow().clone();
        child_rc.optimize(pass);
    }
}

use thread_local::ThreadLocal;

#[derive(Default)]
struct LocalState {
    buf: String,
    count: usize,
}

pub struct ParOutput {
    locals: ThreadLocal<RefCell<LocalState>>,
}

impl ParOutput {
    pub fn new() -> Self {
        Self {
            locals: ThreadLocal::new(),
        }
    }

    pub fn push_str(&self, s: &str) {
        let cell = self.locals.get_or(|| RefCell::new(LocalState::default()));
        cell.borrow_mut().buf.push_str(s);
    }

    pub fn push_line(&self, s: &str) {
        let cell = self.locals.get_or(|| RefCell::new(LocalState::default()));
        let mut st = cell.borrow_mut();
        st.buf.push_str(s);
        st.buf.push('\n');
        st.count += 1;
    }

    /// Consume and merge. Returns (output, total_rows).
    pub fn finalize(self) -> (String, usize) {
        let mut out = String::new();
        let mut total = 0usize;

        for cell in self.locals.into_iter() {
            let st = cell.into_inner();
            total += st.count;
            out.push_str(&st.buf);
        }
        // println!("Total rows printed: {}", total);
        (out, total)
    }
}
