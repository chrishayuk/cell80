//! `canon::light` — the dialect normalizer (Light mode): semantics-preserving
//! rewrites only. Strip statement macros, rewrite a trailing `let`/`return` into a
//! tail expression, collapse redundant parens. Byte-identical output when no rule
//! fires (no hash churn for already-clean sources).

use crate::diag::{DiagCode, Repair};
use quote::ToTokens;
use syn::visit_mut::VisitMut;

struct ParenFold {
    fired: bool,
}

impl VisitMut for ParenFold {
    fn visit_expr_mut(&mut self, e: &mut syn::Expr) {
        syn::visit_mut::visit_expr_mut(self, e); // bottom-up
        if let syn::Expr::Paren(p) = e {
            if matches!(
                &*p.expr,
                syn::Expr::Paren(_) | syn::Expr::Lit(_) | syn::Expr::Path(_)
            ) {
                let inner = (*p.expr).clone();
                *e = inner;
                self.fired = true;
            }
        }
    }
}

pub(crate) fn returns_value(sig: &syn::Signature) -> bool {
    !matches!(sig.output, syn::ReturnType::Default)
}

/// Apply the normalizer to one fn body. Returns whether anything fired.
pub(crate) fn normalize_block(
    block: &mut syn::Block,
    value_fn: bool,
    repairs: &mut Vec<Repair>,
) -> bool {
    let mut fired = false;
    // Strip statement macros — the dialect has none; a bare `println!(…);` line is
    // exactly the model-dialect noise the normalizer exists for.
    let before = block.stmts.len();
    block.stmts.retain(|s| {
        if let syn::Stmt::Macro(m) = s {
            repairs.push(Repair::new(
                DiagCode::StatementMacro,
                format!("stripped `{}!`", m.mac.path.to_token_stream()),
            ));
            false
        } else {
            true
        }
    });
    fired |= block.stmts.len() != before;
    // Trailing `let` / trailing `return` → tail expression (the row93 class).
    if value_fn {
        let rewrite = match block.stmts.last() {
            Some(syn::Stmt::Local(l)) => l.init.as_ref().map(|i| ((*i.expr).clone(), "let")),
            Some(syn::Stmt::Expr(syn::Expr::Return(r), _)) => {
                r.expr.as_ref().map(|e| ((**e).clone(), "return"))
            }
            _ => None,
        };
        if let Some((tail, kind)) = rewrite {
            *block.stmts.last_mut().unwrap() = syn::Stmt::Expr(tail, None);
            repairs.push(Repair::new(
                DiagCode::TrailingLet,
                format!("rewrote trailing `{kind}` to a tail expression"),
            ));
            fired = true;
        }
    }
    // Collapse redundant parens.
    let mut fold = ParenFold { fired: false };
    fold.visit_block_mut(block);
    if fold.fired {
        repairs.push(Repair::new(
            DiagCode::RedundantParens,
            "collapsed redundant parentheses",
        ));
        fired = true;
    }
    fired
}
