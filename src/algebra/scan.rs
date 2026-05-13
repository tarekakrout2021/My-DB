//! TableScan operator for reading base table data.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::rc::{Rc, Weak};

use crate::algebra::iu::IURef;
use crate::algebra::{Op, Operator, OptimizerPass, TupleCtx};
use crate::codegen::Codegen;

/// Table scan operator that reads data from base tables.
#[derive(Debug)]
pub struct TableScan {
    /// Reference to parent operator
    pub parent: RefCell<Weak<Op>>,
    /// IUs produced by this scan
    pub iu_refs_set: HashSet<IURef>,
    /// Table name
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

        let table_var = codegen.get_and_declare_table(&self.table_name);

        let rows_var = codegen.new_var("rows_");
        let row_var = codegen.new_var("row_");
        let view_var = codegen.new_var("view_");
        let row_ids_var = codegen.new_var("row_ids_");

        let mut ius: Vec<IURef> = self.iu_refs_set.iter().cloned().collect();
        ius.sort_by(|a, b| a.name.cmp(&b.name));

        let mut col_vars: HashMap<IURef, String> = HashMap::new();
        for iu in ius.iter() {
            let col_var = codegen.get_and_declare_col(&table_var, &iu.name);
            col_vars.insert(iu.clone(), col_var);
        }

        // row_ids_expr is the *name of a variable* declared inside the loop.
        let mut ctx = TupleCtx {
            exprs: HashMap::new(),
            access: HashMap::new(),
            row_ids_expr: row_ids_var.clone(),
            row_id_tables: vec![table_var.clone()],
        };

        // Base IU expressions are just RowView.get(col_idx)
        for iu in ius.iter() {
            let col_var = col_vars.get(iu).unwrap();
            let expr = format!("{}.get({}).unwrap()", view_var, col_var);
            ctx.exprs.insert(iu.clone(), expr);
            ctx.access
                .insert(iu.clone(), (table_var.clone(), col_var.clone()));
        }

        codegen.open(format!(
            "(0..{}.num_rows).into_par_iter().chunks(MORSEL_SIZE).for_each( | {} |",
            table_var, rows_var
        ));
        codegen.open(format!("for {} in {} ", row_var, rows_var));

        codegen.line(format!(
            "let {} = RowView{{ table: &{}, row_id: {} }};",
            view_var, table_var, row_var
        ));

        codegen.line(&format!(
            "let {}: Vec<usize> = vec![{}];",
            row_ids_var, row_var
        ));

        if let Some(parent_rc) = self.parent.borrow().upgrade() {
            parent_rc.consume(codegen, self as &dyn Operator, ctx)?;
        }

        codegen.close(); // for row
        codegen.close(); // for_each
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
