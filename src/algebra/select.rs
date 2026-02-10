//! Select operator for filtering rows based on predicates.

use std::cell::RefCell;
use std::collections::HashSet;
use std::error::Error;
use std::rc::{Rc, Weak};

use crate::algebra::iu::IURef;
use crate::algebra::join::JoinType;
use crate::algebra::{Op, Operator, OptimizerPass, TupleCtx};
use crate::codegen::Codegen;
use crate::parser::expr::{BinaryExpression, Expression};

/// Selection operator that filters tuples based on a predicate.
#[derive(Debug)]
pub struct Select {
    /// Reference to parent operator
    pub parent: RefCell<Weak<Op>>,
    /// Child operator providing tuples
    pub child: RefCell<Rc<Op>>,
    /// Predicate expression
    pub binary_exp: BinaryExpression,
}

impl Select {
    pub fn new(child: Rc<Op>, binary_expression: BinaryExpression) -> Self {
        Self {
            parent: RefCell::new(Weak::new()),
            child: RefCell::new(child),
            binary_exp: binary_expression,
        }
    }

    fn collect_ius_from_expr(expr: &Expression, out: &mut HashSet<IURef>) {
        match expr {
            Expression::IURef(iu) => {
                out.insert(iu.clone());
            }
            Expression::BinaryExpr(b) => {
                Self::collect_ius_from_expr(&b.l, out);
                Self::collect_ius_from_expr(&b.r, out);
            }
            Expression::ColumnRef(_) | Expression::String(_) | Expression::Subquery(_) => {}
        }
    }

    fn predicate_ius(&self) -> HashSet<IURef> {
        let mut set = HashSet::new();
        Self::collect_ius_from_expr(&self.binary_exp.l, &mut set);
        Self::collect_ius_from_expr(&self.binary_exp.r, &mut set);
        set
    }

    fn predicate_to_code(&self, ctx: &TupleCtx) -> String {
        let left = self.binary_exp.l.produce(ctx);
        let right = self.binary_exp.r.produce(ctx);
        //  WHERE only supports '=' so we hardcode '=='
        format!("{} == {}", left, right)
    }

    /// Try to push this select below joins.
    ///
    /// Returns:
    /// - Some(ConvertToJoin(join))
    /// - Some(InsertSelect { parent, on_left, child })
    /// - None if no pushdown is possible
    fn find_push_target(&self, start: Rc<Op>, pred_ius: &HashSet<IURef>) -> Option<PushTarget> {
        let mut current = start;

        // Track the deepest valid insertion point
        let mut insert_parent: Option<Rc<Op>> = None;
        let mut insert_child: Option<Rc<Op>> = None;
        let mut insert_on_left = false;

        loop {
            let Op::Join(join) = current.as_ref() else {
                break;
            };

            let left = join.left_child.borrow().clone();
            let right = join.right_child.borrow().clone();

            let left_ius = left.ius();
            let right_ius = right.ius();

            let in_left = pred_ius.iter().all(|iu| left_ius.contains(iu));
            let in_right = pred_ius.iter().all(|iu| right_ius.contains(iu));

            match (in_left, in_right) {
                (true, false) => {
                    insert_parent = Some(current.clone());
                    insert_child = Some(left.clone());
                    insert_on_left = true;
                    current = left;
                }
                (false, true) => {
                    insert_parent = Some(current.clone());
                    insert_child = Some(right.clone());
                    insert_on_left = false;
                    current = right;
                }
                (true, true) | (false, false) => {
                    return Some(PushTarget::ConvertToJoin(current.clone()));
                }
            }
        }

        if let (Some(parent), Some(child)) = (insert_parent, insert_child) {
            Some(PushTarget::InsertAt {
                parent,
                on_left: insert_on_left,
                target_child: child,
            })
        } else {
            None
        }
    }
}

/// Result of predicate pushdown analysis.
enum PushTarget {
    /// Attach predicate to this join
    ConvertToJoin(Rc<Op>),
    /// Insert a Select above `target_child`
    InsertAt {
        parent: Rc<Op>,
        on_left: bool,
        target_child: Rc<Op>,
    },
}

impl Operator for Select {
    fn prepare(&self, op: &Rc<Op>, required_ius: HashSet<IURef>) -> Result<(), Box<dyn Error>> {
        let child = self.child.borrow().clone();
        child.set_parent(Rc::downgrade(op));

        let mut child_required = required_ius;
        child_required.extend(self.predicate_ius());

        child.prepare(&child, child_required)?;
        Ok(())
    }

    fn ius(&self) -> HashSet<IURef> {
        //self.iu_refs_set.clone()
        let child_rc = self.child.borrow().clone();
        child_rc.ius()
    }

    fn produce(&self, codegen: &mut Codegen) -> Result<(), Box<dyn Error>> {
        let child_rc = self.child.borrow().clone();
        child_rc.produce(codegen)?;
        Ok(())
    }

    fn consume(
        &self,
        codegen: &mut Codegen,
        _caller: &dyn Operator,
        tuple_ctx: TupleCtx,
    ) -> Result<(), Box<dyn Error>> {
        let cond = self.predicate_to_code(&tuple_ctx);

        codegen.line("// Select operator consume");
        codegen.open(format!("if {}", cond));

        if let Some(parent) = self.parent.borrow().upgrade() {
            parent.consume(codegen, self, tuple_ctx)?;
        }

        codegen.close(); // end if

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

    fn replace_child(&self, _old: &dyn Operator, new: Rc<Op>) {
        *self.child.borrow_mut() = new;
    }

    fn optimize(&self, pass: OptimizerPass) {
        // Optimize child first
        let child = self.child.borrow().clone();
        child.optimize(pass.clone());

        let child = self.child.borrow().clone();
        let Op::Join(_) = child.as_ref() else {
            return;
        };

        let pred_ius = self.predicate_ius();
        if pred_ius.is_empty() {
            return;
        }

        let Some(target) = self.find_push_target(child.clone(), &pred_ius) else {
            return;
        };

        match target {
            PushTarget::ConvertToJoin(join_rc) => {
                let Op::Join(join) = join_rc.as_ref() else {
                    return;
                };
                join.set_join_type(JoinType::Inner);
                join.add_join_predicate(self.binary_exp.clone());

                if let Some(parent) = self.parent.borrow().upgrade() {
                    parent.replace_child(self, child);
                }
            }

            PushTarget::InsertAt {
                parent,
                on_left,
                target_child,
            } => {
                let pushed = Rc::new(Op::Select(Select::new(
                    target_child.clone(),
                    self.binary_exp.clone(),
                )));

                target_child.set_parent(Rc::downgrade(&pushed));
                pushed.set_parent(Rc::downgrade(&parent));

                if let Op::Join(join) = parent.as_ref() {
                    if on_left {
                        *join.left_child.borrow_mut() = pushed.clone();
                    } else {
                        *join.right_child.borrow_mut() = pushed.clone();
                    }
                } else {
                    parent.replace_child(target_child.as_ref(), pushed.clone());
                }

                if let Some(parent) = self.parent.borrow().upgrade() {
                    parent.replace_child(self, child);
                }

                pushed.optimize(pass);
            }
        }
    }
}
