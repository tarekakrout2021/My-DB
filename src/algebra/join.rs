//! Join operator for combining data from two input streams.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::rc::{Rc, Weak};

use crate::algebra::iu::IURef;
use crate::algebra::{Op, Operator, OptimizerPass, TupleCtx};
use crate::codegen::Codegen;
use crate::parser::expr::{BinaryExpression, Expression};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinType {
    Cross,
    Inner,
}

#[derive(Debug)]
pub struct Join {
    /// Reference to parent operator
    pub parent: RefCell<Weak<Op>>,
    /// Left child operator (build side for hash joins)
    pub left_child: RefCell<Rc<Op>>,
    /// Right child operator (probe side for hash joins)
    pub right_child: RefCell<Rc<Op>>,

    pub join_type: RefCell<JoinType>,
    pub join_predicates: RefCell<Vec<BinaryExpression>>,

    /// Used for CROSS join to remember the left tuple.
    pub current_left_ctx: RefCell<Option<TupleCtx>>,

    /// Captured from build side so probe phase can reconstruct IUs.
    left_access_template: RefCell<Option<HashMap<IURef, (String, String)>>>,
    left_row_id_tables_template: RefCell<Option<Vec<String>>>,

    /// Name of the LazyMultiMapBuilder variable in generated code (e.g., "builder_0").
    builder_var: RefCell<Option<String>>,
    /// Name of the LazyMultiMap variable in generated code (e.g., "map_0").
    map_var: RefCell<Option<String>>,
    /// Flag to indicate whether we are currently in the build phase (consuming left child) or probe phase (consuming right child) of the hash join.
    in_build_phase: Cell<bool>,
}

impl Join {
    /// Creates a new Join operator with the specified children.
    pub fn new(left_child: Rc<Op>, right_child: Rc<Op>) -> Self {
        Self {
            parent: RefCell::new(Weak::new()),
            left_child: RefCell::new(left_child),
            right_child: RefCell::new(right_child),
            join_type: RefCell::new(JoinType::Cross),
            join_predicates: RefCell::new(Vec::new()),
            current_left_ctx: RefCell::new(None),
            left_access_template: RefCell::new(None),
            left_row_id_tables_template: RefCell::new(None),
            builder_var: RefCell::new(None),
            map_var: RefCell::new(None),
            in_build_phase: Cell::new(false),
        }
    }

    pub fn join_type(&self) -> JoinType {
        *self.join_type.borrow()
    }

    pub fn set_join_type(&self, join_type: JoinType) {
        *self.join_type.borrow_mut() = join_type;
    }

    pub fn add_join_predicate(&self, predicate: BinaryExpression) {
        self.join_predicates.borrow_mut().push(predicate);
        *self.join_type.borrow_mut() = JoinType::Inner;
    }

    fn key_exprs_from_ctx(&self, ctx: &TupleCtx, build_side: bool) -> Vec<String> {
        let left_ius = self.left_child.borrow().ius();
        let right_ius = self.right_child.borrow().ius();

        let mut res = Vec::new();
        for pred in self.join_predicates.borrow().iter() {
            let expr = match (&pred.l, &pred.r, build_side) {
                (Expression::IURef(iu_l), Expression::IURef(iu_r), true) => {
                    if left_ius.contains(iu_l) {
                        ctx.exprs.get(iu_l).unwrap().clone()
                    } else if left_ius.contains(iu_r) {
                        ctx.exprs.get(iu_r).unwrap().clone()
                    } else {
                        panic!("No build-side IU found for predicate in Join");
                    }
                }
                (Expression::IURef(iu_l), Expression::IURef(iu_r), false) => {
                    if right_ius.contains(iu_l) {
                        ctx.exprs.get(iu_l).unwrap().clone()
                    } else if right_ius.contains(iu_r) {
                        ctx.exprs.get(iu_r).unwrap().clone()
                    } else {
                        panic!("No probe-side IU found for predicate in Join");
                    }
                }
                _ => unreachable!(),
            };
            res.push(expr);
        }
        res
    }

    /// Emit: out = left.clone(); out.extend(right.iter().copied());
    /// with *explicit* SmallVec backing array type to avoid SmallVec<_> inference failures.
    fn emit_row_ids_concat(
        &self,
        codegen: &mut Codegen,
        out_var: &str,
        left_expr: &str,
        right_expr: &str,
    ) {
        codegen.line(&format!(
            "let {out}: Vec<usize> = {{ \
                let mut tmp: Vec<usize> = {left}.clone(); \
                tmp.extend({right}.iter().copied()); \
                tmp \
            }};",
            out = out_var,
            left = left_expr,
            right = right_expr
        ));
    }

    /// Build a TupleCtx where all IU exprs are reconstructed via RowView from joined row ids.
    fn rebuild_ctx_from_row_ids(
        &self,
        joined_ids_var: &str,
        merged_tables: &Vec<String>,
        merged_access: &HashMap<IURef, (String, String)>,
    ) -> TupleCtx {
        let mut merged_exprs: HashMap<IURef, String> = HashMap::new();

        for (iu, (tbl_var, col_var)) in merged_access.iter() {
            let idx = merged_tables
                .iter()
                .position(|t| t == tbl_var)
                .unwrap_or_else(|| panic!("Table var {} not found in merged_tables", tbl_var));

            let rid_expr = format!("*{}.get({}).unwrap()", joined_ids_var, idx);
            let view_expr = format!("RowView{{ table: &{}, row_id: {} }}", tbl_var, rid_expr);
            let expr = format!("{}.get({}).unwrap()", view_expr, col_var);
            merged_exprs.insert(iu.clone(), expr);
        }

        TupleCtx {
            exprs: merged_exprs,
            access: merged_access.clone(),
            row_ids_expr: joined_ids_var.to_string(),
            row_id_tables: merged_tables.clone(),
        }
    }
}

impl Operator for Join {
    fn prepare(&self, op: &Rc<Op>, required_ius: HashSet<IURef>) -> Result<(), Box<dyn Error>> {
        let left_rc = self.left_child.borrow().clone();
        let right_rc = self.right_child.borrow().clone();

        left_rc.set_parent(Rc::downgrade(op));
        right_rc.set_parent(Rc::downgrade(op));

        // Split required IUs per child
        let left_ius = left_rc.ius();
        let right_ius = right_rc.ius();

        let mut left_required = HashSet::new();
        let mut right_required = HashSet::new();

        for iu in required_ius {
            if left_ius.contains(&iu) {
                left_required.insert(iu.clone());
            }
            if right_ius.contains(&iu) {
                right_required.insert(iu.clone());
            }
        }

        left_rc.prepare(&left_rc, left_required)?;
        right_rc.prepare(&right_rc, right_required)?;
        Ok(())
    }

    fn ius(&self) -> HashSet<IURef> {
        let mut res = self.left_child.borrow().ius();
        res.extend(self.right_child.borrow().ius());
        res
    }

    fn produce(&self, codegen: &mut Codegen) -> Result<(), Box<dyn Error>> {
        match self.join_type() {
            JoinType::Cross => {
                let left_rc = self.left_child.borrow().clone();
                left_rc.produce(codegen)?;
                Ok(())
            }
            JoinType::Inner => {
                let builder_name = codegen.new_var("builder_");
                let map_name = codegen.new_var("map_");

                *self.builder_var.borrow_mut() = Some(builder_name.clone());
                *self.map_var.borrow_mut() = Some(map_name.clone());

                codegen.line("// Using LazyMultiMap ");
                codegen.line(&format!(
                    "let mut {b}: LazyMultiMapParBuilder<Vec<ValueRef>, Vec<usize>> = LazyMultiMapParBuilder::new();",
                    b = builder_name
                ));

                // Build phase
                self.in_build_phase.set(true);
                let left_rc = self.left_child.borrow().clone();
                codegen.line("// Build Phase ");
                left_rc.produce(codegen)?;
                self.in_build_phase.set(false);

                // Finalize hash table
                codegen.line(&format!(
                    "let {m} = {b}.finalize();",
                    m = map_name,
                    b = builder_name
                ));

                // Probe phase
                let right_rc = self.right_child.borrow().clone();
                codegen.line("// Probe Phase ");
                right_rc.produce(codegen)?;
                Ok(())
            }
        }
    }

    fn consume(
        &self,
        codegen: &mut Codegen,
        _caller: &dyn Operator,
        tuple_ctx: TupleCtx,
    ) -> Result<(), Box<dyn Error>> {
        match self.join_type() {
            JoinType::Cross => {
                let is_left_call = self.current_left_ctx.borrow().is_none();

                if is_left_call {
                    *self.current_left_ctx.borrow_mut() = Some(tuple_ctx);
                    let right_rc = self.right_child.borrow().clone();
                    right_rc.produce(codegen)?;
                    *self.current_left_ctx.borrow_mut() = None;
                    Ok(())
                } else {
                    let left_ctx = self.current_left_ctx.borrow().as_ref().unwrap().clone();

                    let joined_ids_var = codegen.new_var("row_ids_");
                    self.emit_row_ids_concat(
                        codegen,
                        &joined_ids_var,
                        &left_ctx.row_ids_expr,
                        &tuple_ctx.row_ids_expr,
                    );

                    let mut merged_tables = left_ctx.row_id_tables.clone();
                    merged_tables.extend(tuple_ctx.row_id_tables.iter().cloned());

                    let mut merged_access = left_ctx.access.clone();
                    for (iu, acc) in tuple_ctx.access.iter() {
                        merged_access.insert(iu.clone(), acc.clone());
                    }

                    let merged = self.rebuild_ctx_from_row_ids(
                        &joined_ids_var,
                        &merged_tables,
                        &merged_access,
                    );

                    if let Some(parent_rc) = self.parent.borrow().upgrade() {
                        parent_rc.consume(codegen, self as &dyn Operator, merged)?;
                    }

                    Ok(())
                }
            }

            JoinType::Inner => {
                if self.in_build_phase.get() {
                    // Build: store only row ids as payload
                    let key_exprs = self.key_exprs_from_ctx(&tuple_ctx, true);

                    if self.left_access_template.borrow().is_none() {
                        *self.left_access_template.borrow_mut() = Some(tuple_ctx.access.clone());
                    }
                    if self.left_row_id_tables_template.borrow().is_none() {
                        *self.left_row_id_tables_template.borrow_mut() =
                            Some(tuple_ctx.row_id_tables.clone());
                    }

                    let builder_name = self.builder_var.borrow().as_ref().unwrap().clone();
                    let key = format!("vec![{}]", key_exprs.join(", "));
                    let payload = tuple_ctx.row_ids_expr.clone(); // variable name (typed)

                    codegen.line(&format!(
                        "{b}.insert({k}, {p});",
                        b = builder_name,
                        k = key,
                        p = payload
                    ));
                    Ok(())
                } else {
                    // Probe: for each match, concatenate row ids and rebuild expressions
                    let key_exprs = self.key_exprs_from_ctx(&tuple_ctx, false);
                    let map_name = self.map_var.borrow().as_ref().unwrap().clone();

                    let key = format!("vec![{}]", key_exprs.join(", "));
                    let match_var = codegen.new_var("build_row_");

                    codegen.open(format!(
                        "for {br} in {m}.get({k})",
                        br = match_var,
                        m = map_name,
                        k = key
                    ));

                    let joined_ids_var = codegen.new_var("row_ids_");
                    self.emit_row_ids_concat(
                        codegen,
                        &joined_ids_var,
                        &match_var,
                        &tuple_ctx.row_ids_expr,
                    );

                    let mut merged_tables = self
                        .left_row_id_tables_template
                        .borrow()
                        .clone()
                        .expect("left_row_id_tables_template should be set during build phase");
                    merged_tables.extend(tuple_ctx.row_id_tables.iter().cloned());

                    let mut merged_access = self
                        .left_access_template
                        .borrow()
                        .clone()
                        .expect("left_access_template should be set during build phase");
                    for (iu, acc) in tuple_ctx.access.iter() {
                        merged_access.insert(iu.clone(), acc.clone());
                    }

                    let merged = self.rebuild_ctx_from_row_ids(
                        &joined_ids_var,
                        &merged_tables,
                        &merged_access,
                    );

                    if let Some(parent_rc) = self.parent.borrow().upgrade() {
                        parent_rc.consume(codegen, self as &dyn Operator, merged)?;
                    }

                    codegen.close(); // end for build_row in map.get(...)
                    Ok(())
                }
            }
        }
    }

    fn parent(&self) -> Weak<Op> {
        self.parent.borrow().clone()
    }

    fn set_parent(&self, parent: Weak<Op>) {
        *self.parent.borrow_mut() = parent;
    }

    fn child(&self, _child: &dyn Operator) -> Rc<Op> {
        self.left_child.borrow().clone()
    }

    fn replace_child(&self, _child: &dyn Operator, new: Rc<Op>) {
        {
            let left = self.left_child.borrow();
            if matches!(left.as_ref(), Op::Select(_)) {
                drop(left);
                *self.left_child.borrow_mut() = new;
                return;
            }
        }

        {
            let right = self.right_child.borrow();
            if matches!(right.as_ref(), Op::Select(_)) {
                drop(right);
                *self.right_child.borrow_mut() = new;
                return;
            }
        }
        panic!("Unexpected") // TODO
    }

    fn optimize(&self, pass: OptimizerPass) {
        let left_rc = self.left_child.borrow().clone();
        let right_rc = self.right_child.borrow().clone();

        left_rc.optimize(pass.clone());
        right_rc.optimize(pass);
    }
}
