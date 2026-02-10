//! TableScan operator for reading base table data.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::rc::{Rc, Weak};

use crate::algebra::iu::IURef;
use crate::algebra::{Op, Operator, OptimizerPass, TupleCtx};
use crate::codegen::Codegen;

/// Table scan operator that reads data from base tables.
///
/// As a leaf operator, TableScan has no children and serves as the starting
/// point for data flow in query execution. It accesses the underlying table
/// storage and produces tuples that flow up through the operator tree.
#[derive(Debug)]
pub struct TableScan {
    /// Reference to parent operator
    pub parent: RefCell<Weak<Op>>,
    // TODO: More fields, e.g., for IURefs of columns to produce
    pub iu_refs_set: HashSet<IURef>,
    // for codegen to know the table name
    pub table_name: String,
}

impl TableScan {
    /// Creates a new 'EMPTY' TableScan operator
    pub fn new() -> Self {
        Self {
            parent: RefCell::new(Weak::new()),
            iu_refs_set: HashSet::new(),
            table_name: String::new(),
        }
    }
}

impl Operator for TableScan {
    fn prepare(&self, _op: &Rc<Op>, _required_ius: HashSet<IURef>) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn ius(&self) -> HashSet<IURef> {
        self.iu_refs_set.clone()
    }

    fn produce(&self, codegen: &mut Codegen) -> Result<(), Box<dyn Error>> {
        codegen.line("// TableScan ");

        let table_var = codegen.new_var("table_");
        let rows_var = codegen.new_var("rows_");
        let row_var = codegen.new_var("row_");
        let view_var = codegen.new_var("view_");

        codegen.line(&format!(
            "let {} = db.get_table(\"{}\").unwrap();",
            table_var, self.table_name
        ));

        codegen.open(format!(
            "(0..{}.num_rows).into_par_iter().chunks(MORSEL_SIZE).for_each( | {} |",
            table_var, rows_var
        ));
        codegen.open(format!("for {} in {} ", row_var, rows_var));
        codegen.line(format!(
            "let {} = RowView{{ table: &{}, row_id: {} }};",
            view_var, table_var, row_var
        ));

        let mut ctx = TupleCtx {
            exprs: HashMap::new(),
        };
        for iu in self.iu_refs_set.iter() {
            let expr = format!(
                "{}.get({}.get_col_index(String::from(\"{}\")).unwrap() as usize).unwrap().clone()",
                view_var, table_var, iu.name
            );
            ctx.exprs.insert(iu.clone(), expr);
        }

        if let Some(parent_rc) = self.parent.borrow().upgrade() {
            parent_rc.consume(codegen, self as &dyn Operator, ctx)?;
        }

        codegen.close(); // end for
        codegen.close();
        codegen.line(");");

        Ok(())
    }

    fn consume(
        &self,
        _codegen: &mut Codegen,
        _caller: &dyn Operator,
        _tuple_ctx: TupleCtx,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn parent(&self) -> Weak<Op> {
        self.parent.borrow().clone()
    }

    fn set_parent(&self, parent: Weak<Op>) {
        *self.parent.borrow_mut() = parent;
    }

    fn child(&self, _child: &dyn Operator) -> Rc<Op> {
        panic!("TableScan has no children");
    }

    fn replace_child(&self, _child: &dyn Operator, _new: Rc<Op>) {
        panic!("TableScan has no children");
    }

    fn optimize(&self, _pass: OptimizerPass) {}
}
