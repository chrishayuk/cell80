#!/usr/bin/env python3
"""Parse model-emitted ARITHMETIC (assignment lines / expressions, PAL-style) into
cell80's Plan IR via Python's `ast`. The whole point: models are good at writing
arithmetic and bad at writing our JSON DAG, and every JSON-shape error in the pilot
(object-ops, int-ids, nested arrays, bare-literal operands, inline exprs) is an
artifact of flattening a DAG into tuples. A real expression parser makes that class
of error structurally impossible — precedence, parens, nesting all parse natively.

SSA-style: reassignment (`x = x + 1`) is fine; each binding gets a fresh IR id and
`env` tracks the current one. Leaf integer literals become quantities; every BinOp
becomes one PlanOp; target = the last assignment (or an explicit `answer`/`result`).
"""
import ast
import re
from fractions import Fraction

OPMAP = {ast.Add: "add", ast.Sub: "sub", ast.Mult: "mul",
         ast.Div: "div", ast.FloorDiv: "div"}
ASSIGN_RE = re.compile(r"^\s*(?:let\s+)?([A-Za-z_]\w*)\s*=\s*(.+?)\s*;?\s*$", re.I)


class ParseFail(Exception):
    pass


def _assignment_lines(text):
    text = text.strip()
    # strip a single ```...``` fence if present
    m = re.search(r"```(?:\w+)?\s*(.*?)\s*```", text, re.DOTALL)
    if m:
        text = m.group(1)
    out = []
    for raw in text.splitlines():
        m = ASSIGN_RE.match(raw)
        if not m:
            continue
        lhs, rhs = m.group(1), m.group(2)
        # RHS must parse as a pure arithmetic expression
        try:
            node = ast.parse(rhs, mode="eval").body
        except SyntaxError:
            continue
        out.append((lhs, node))
    return out


def equations_to_plan(text, default_unit="scalar"):
    lines = _assignment_lines(text)
    if not lines:
        raise ParseFail("no assignment lines found")

    env = {}            # model var name -> current IR id
    quantities = []     # {id, value, unit}
    ops = []            # [op, a, b, out]
    n = [0]

    def fresh(base):
        n[0] += 1
        return f"{base}{n[0]}"

    def const(v):
        v = int(v)
        if v < 0:
            raise ParseFail(f"negative literal {v}")
        uid = fresh("k")
        quantities.append({"id": uid, "value": v, "unit": default_unit})
        return uid

    def emit_mul(ids):
        acc = ids[0]
        for f in ids[1:]:
            out = fresh("t")
            ops.append(["mul", acc, f, out])
            acc = out
        return acc

    # Flatten a */ / subtree into (numerator_ids, denominator_ids) so that ALL
    # multiplications happen before the single final division — this is what
    # keeps integer division from truncating a correct fraction to zero
    # (`2/3*x` -> `2*x/3`). Decimal literals become integer fractions (`0.9`->9/10).
    def factors(node):
        if isinstance(node, ast.BinOp) and isinstance(node.op, ast.Mult):
            n1, d1 = factors(node.left)
            n2, d2 = factors(node.right)
            return n1 + n2, d1 + d2
        if isinstance(node, ast.BinOp) and isinstance(node.op, (ast.Div, ast.FloorDiv)):
            n1, d1 = factors(node.left)
            n2, d2 = factors(node.right)
            return n1 + d2, d1 + n2   # (a/b)/(c/d) = ad/bc
        if isinstance(node, ast.Constant) and isinstance(node.value, (int, float)):
            v = node.value
            if isinstance(v, float):
                fr = Fraction(v).limit_denominator(1000)
                num = [const(fr.numerator)]
                den = [] if fr.denominator == 1 else [const(fr.denominator)]
                return num, den
            return [const(int(v))], []
        return [eval_expr(node)], []   # Name or an additive subtree: evaluate as a unit

    def eval_expr(node):
        if isinstance(node, ast.BinOp) and isinstance(node.op, (ast.Add, ast.Sub)):
            a = eval_expr(node.left)
            b = eval_expr(node.right)
            out = fresh("t")
            ops.append(["add" if isinstance(node.op, ast.Add) else "sub", a, b, out])
            return out
        if isinstance(node, ast.BinOp) and isinstance(node.op, (ast.Mult, ast.Div, ast.FloorDiv)):
            num, den = factors(node)
            acc = emit_mul(num)
            if den:
                out = fresh("t")
                ops.append(["div", acc, emit_mul(den), out])
                acc = out
            return acc
        if isinstance(node, ast.UnaryOp) and isinstance(node.op, ast.UAdd):
            return eval_expr(node.operand)
        if isinstance(node, ast.Constant) and isinstance(node.value, (int, float)):
            num, den = factors(node)
            acc = emit_mul(num)
            if den:
                out = fresh("t")
                ops.append(["div", acc, emit_mul(den), out])
                acc = out
            return acc
        if isinstance(node, ast.Name):
            if node.id not in env:
                raise ParseFail(f"undefined name {node.id!r}")
            return env[node.id]
        raise ParseFail(f"unsupported expression node {type(node).__name__}")

    last = None
    for name, node in lines:
        env[name] = eval_expr(node)
        last = name

    target = None
    for cue in ("answer", "result", "total", "ans"):
        if cue in env:
            target = env[cue]
    if target is None:
        target = env[last]

    return {"quantities": quantities, "ops": ops, "target": target}


if __name__ == "__main__":
    import json
    import sys
    print(json.dumps(equations_to_plan(sys.stdin.read()), indent=2))
