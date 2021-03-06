//! Checks for needless address of operations (`&`)
//!
//! This lint is **warn** by default

use rustc::lint::*;
use rustc::{declare_lint, lint_array};
use if_chain::if_chain;
use rustc::hir::{BindingAnnotation, Expr, ExprKind, MutImmutable, Pat, PatKind};
use rustc::ty;
use rustc::ty::adjustment::{Adjust, Adjustment};
use crate::utils::{in_macro, snippet_opt, span_lint_and_then};

/// **What it does:** Checks for address of operations (`&`) that are going to
/// be dereferenced immediately by the compiler.
///
/// **Why is this bad?** Suggests that the receiver of the expression borrows
/// the expression.
///
/// **Example:**
/// ```rust
/// let x: &i32 = &&&&&&5;
/// ```
///
/// **Known problems:** This will cause false positives in code generated by `derive`.
/// For instance in the following snippet:
/// ```rust
/// #[derive(Debug)]
/// pub enum Error {
///     Type(
///         &'static str,
///     ),
/// }
/// ```
/// A warning will be emitted that `&'static str` should be replaced with `&'static str`,
/// however there is nothing that can or should be done to fix this.
declare_clippy_lint! {
    pub NEEDLESS_BORROW,
    nursery,
    "taking a reference that is going to be automatically dereferenced"
}

#[derive(Copy, Clone)]
pub struct NeedlessBorrow;

impl LintPass for NeedlessBorrow {
    fn get_lints(&self) -> LintArray {
        lint_array!(NEEDLESS_BORROW)
    }
}

impl<'a, 'tcx> LateLintPass<'a, 'tcx> for NeedlessBorrow {
    fn check_expr(&mut self, cx: &LateContext<'a, 'tcx>, e: &'tcx Expr) {
        if in_macro(e.span) {
            return;
        }
        if let ExprKind::AddrOf(MutImmutable, ref inner) = e.node {
            if let ty::TyRef(..) = cx.tables.expr_ty(inner).sty {
                for adj3 in cx.tables.expr_adjustments(e).windows(3) {
                    if let [Adjustment {
                        kind: Adjust::Deref(_),
                        ..
                    }, Adjustment {
                        kind: Adjust::Deref(_),
                        ..
                    }, Adjustment {
                        kind: Adjust::Borrow(_),
                        ..
                    }] = *adj3
                    {
                        span_lint_and_then(
                            cx,
                            NEEDLESS_BORROW,
                            e.span,
                            "this expression borrows a reference that is immediately dereferenced \
                             by the compiler",
                            |db| {
                                if let Some(snippet) = snippet_opt(cx, inner.span) {
                                    db.span_suggestion(e.span, "change this to", snippet);
                                }
                            },
                        );
                    }
                }
            }
        }
    }
    fn check_pat(&mut self, cx: &LateContext<'a, 'tcx>, pat: &'tcx Pat) {
        if in_macro(pat.span) {
            return;
        }
        if_chain! {
            if let PatKind::Binding(BindingAnnotation::Ref, _, name, _) = pat.node;
            if let ty::TyRef(_, tam, mutbl) = cx.tables.pat_ty(pat).sty;
            if mutbl == MutImmutable;
            if let ty::TyRef(_, _, mutbl) = tam.sty;
            // only lint immutable refs, because borrowed `&mut T` cannot be moved out
            if mutbl == MutImmutable;
            then {
                span_lint_and_then(
                    cx,
                    NEEDLESS_BORROW,
                    pat.span,
                    "this pattern creates a reference to a reference",
                    |db| {
                        if let Some(snippet) = snippet_opt(cx, name.span) {
                            db.span_suggestion(pat.span, "change this to", snippet);
                        }
                    }
                )
            }
        }
    }
}
