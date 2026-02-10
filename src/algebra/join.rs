//! Join operator for combining data from two input streams.

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
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

/// Join operator that combines tuples from two child operators.
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

    /// from which side is the consume call coming
    pub current_left_ctx: RefCell<Option<TupleCtx>>,

    /// IUs from the left child that we want to store as payload in the hash table for the hash join.
    pub left_payload_ius: RefCell<Vec<IURef>>,

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
            left_payload_ius: RefCell::new(Vec::new()),
            builder_var: RefCell::new(None),
            map_var: RefCell::new(None),
            in_build_phase: Cell::new(false),
        }
    }

    pub fn join_type(&self) -> JoinType {
        *self.join_type.borrow()
    }

    /// Updates the join type (used by optimizer passes).
    pub fn set_join_type(&self, join_type: JoinType) {
        *self.join_type.borrow_mut() = join_type;
    }

    /// Returns a clone of the current join predicates, if any.
    pub fn join_predicates(&self) -> Vec<BinaryExpression> {
        self.join_predicates.borrow().clone()
    }

    /// Updates the join predicate (e.g., when turning a cross product + filter into an inner join).
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
                    // build side: choose IU from left child
                    if left_ius.contains(iu_l) {
                        ctx.exprs
                            .get(iu_l)
                            .unwrap_or_else(|| panic!("Build IU not found in TupleCtx (left)"))
                            .clone()
                    } else if left_ius.contains(iu_r) {
                        ctx.exprs
                            .get(iu_r)
                            .unwrap_or_else(|| panic!("Build IU not found in TupleCtx (right)"))
                            .clone()
                    } else {
                        panic!("No build-side IU found for predicate in Join");
                    }
                }
                (Expression::IURef(iu_l), Expression::IURef(iu_r), false) => {
                    // probe side: choose IU from right child
                    if right_ius.contains(iu_l) {
                        ctx.exprs
                            .get(iu_l)
                            .unwrap_or_else(|| panic!("Probe IU not found in TupleCtx (left)"))
                            .clone()
                    } else if right_ius.contains(iu_r) {
                        ctx.exprs
                            .get(iu_r)
                            .unwrap_or_else(|| panic!("Probe IU not found in TupleCtx (right)"))
                            .clone()
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

    fn payload_exprs_from_ctx(&self, ctx: &TupleCtx) -> Vec<String> {
        self.left_payload_ius
            .borrow()
            .iter()
            .map(|iu| {
                ctx.exprs
                    .get(iu)
                    .unwrap_or_else(|| panic!("Payload IU not found in TupleCtx"))
                    .clone()
            })
            .collect()
    }
}

impl Operator for Join {
    fn prepare(&self, op: &Rc<Op>, required_ius: HashSet<IURef>) -> Result<(), Box<dyn Error>> {
        let left_rc = self.left_child.borrow().clone();
        let right_rc = self.right_child.borrow().clone();

        left_rc.set_parent(Rc::downgrade(op));
        right_rc.set_parent(Rc::downgrade(op));

        // Figure out which IUs each child can produce
        let left_ius = left_rc.ius();
        let right_ius = right_rc.ius();

        let mut left_required = HashSet::new();
        let mut right_required = HashSet::new();

        for iu in required_ius {
            if left_ius.contains(&iu) {
                left_required.insert(iu.clone());
                // left_payload.push(iu.clone());
            }
            if right_ius.contains(&iu) {
                right_required.insert(iu.clone());
            }
        }

        *self.left_payload_ius.borrow_mut() = left_ius.into_iter().collect();

        // Prepare children with their respective required IUs
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
                // Hash join using LazyMultiMap<Key, Vec<Value>>.

                let builder_name = codegen.new_var("builder_");
                let map_name = codegen.new_var("map_");

                *self.builder_var.borrow_mut() = Some(builder_name.clone());
                *self.map_var.borrow_mut() = Some(map_name.clone());

                // Declare the builder
                codegen.line("// Using LazyMultiMap ");
                codegen.line(&format!(
                    "let mut {}: LazyMultiMapParBuilder<Vec<ValueRef>, Vec<ValueRef>> = LazyMultiMapParBuilder::new();",
                    builder_name
                ));

                // Build phase: consume all left tuples into the builder
                self.in_build_phase.set(true);
                let left_rc = self.left_child.borrow().clone();
                codegen.line("// Build Phase ");
                left_rc.produce(codegen)?;
                self.in_build_phase.set(false);

                // Finalize to get the hash table
                codegen.line(&format!("let {} = {}.finalize();", map_name, builder_name));

                // Probe phase: scan the right side and probe the hash table
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
                    {
                        let mut slot = self.current_left_ctx.borrow_mut();
                        *slot = Some(tuple_ctx); // store this left tuple ctx
                    }

                    let right_rc = self.right_child.borrow().clone();
                    right_rc.produce(codegen)?;

                    // clear the stored left context.
                    let mut slot = self.current_left_ctx.borrow_mut();
                    *slot = None;

                    Ok(())
                } else {
                    let left_ctx = {
                        let slot = self.current_left_ctx.borrow();
                        slot.as_ref().unwrap().clone()
                    };

                    let mut merged = TupleCtx {
                        exprs: left_ctx.exprs.clone(),
                    };
                    for (iu, expr) in tuple_ctx.exprs.into_iter() {
                        merged.exprs.insert(iu, expr);
                    }

                    // Forward the merged tuple to parent
                    if let Some(parent_rc) = self.parent.borrow().upgrade() {
                        parent_rc.consume(codegen, self as &dyn Operator, merged)?;
                    }

                    Ok(())
                }
            }

            JoinType::Inner => {
                if self.in_build_phase.get() {
                    // insert key, payload into the builder
                    let key_exprs = self.key_exprs_from_ctx(&tuple_ctx, true);
                    let payload_exprs = self.payload_exprs_from_ctx(&tuple_ctx);

                    let builder_name = self
                        .builder_var
                        .borrow()
                        .as_ref()
                        .expect("builder_var must be set in the produce phase")
                        .clone();

                    let key_vec = format!("vec![{}]", key_exprs.join(", "));
                    let payload_vec = format!("vec![{}]", payload_exprs.join(", "));

                    codegen.line(&format!(
                        "{}.insert({}, {});",
                        builder_name, key_vec, payload_vec
                    ));

                    Ok(())
                } else {
                    // probe phase
                    let key_exprs = self.key_exprs_from_ctx(&tuple_ctx, false);
                    let map_name = self
                        .map_var
                        .borrow()
                        .as_ref()
                        .expect("map_var must be set in produce phase")
                        .clone();

                    let key_vec = format!("vec![{}]", key_exprs.join(", "));
                    let match_var = codegen.new_var("build_row_");

                    // for build_row_ in map.get(vec![key_components]) {
                    codegen.open(format!(
                        "for {} in {}.get({})",
                        match_var, map_name, key_vec
                    ));

                    // Build merged TupleCtx: left side from `match_var`, right side from tuple_ctx
                    use std::collections::HashMap;
                    let mut merged = TupleCtx {
                        exprs: HashMap::new(),
                    };

                    // Left side: payload stored as Vec<Value> in build_row_
                    for (idx, iu) in self.left_payload_ius.borrow().iter().enumerate() {
                        let expr = format!("{}[{}].clone()", match_var, idx);
                        merged.exprs.insert(iu.clone(), expr);
                    }

                    // Right side: original expressions
                    for (iu, expr) in tuple_ctx.exprs.into_iter() {
                        merged.exprs.insert(iu, expr);
                    }

                    // Forward to parent inside the loop
                    if let Some(parent_rc) = self.parent.borrow().upgrade() {
                        parent_rc.consume(codegen, self as &dyn Operator, merged)?;
                    }

                    codegen.close(); // end for

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
