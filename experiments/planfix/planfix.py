#!/usr/bin/env python3
"""planfix — a deterministic adapter from messy, model-emitted "plan" JSON to
cell80's strict Plan IR.

No model is in this loop. The premise (see experiments/gsm8k-small-model-pilot
findings): small models don't hit our exact schema, but they *do* emit a
recognisable dialect of it. Across four models the pilot found four distinct
malformed conventions — object-shaped ops, bare-literal operands, integer ids,
flattened/nested-array ops — plus a family of "placeholder for a value I'm about
to derive" antipatterns. Every one of those is a mechanical shape mismatch, not a
comprehension error. So we guess the intended call and coerce the parameters into
our canonical form, or we return a *typed* rejection. Every transform is recorded
as a repair row so the failures remain nameable.

Target IR (cell80/src/plan.rs):
  {"quantities":[{"id","value":u32,"unit"}...],
   "ops":[["add|sub|mul|div","a","b","out"]...],
   "target":"id",
   "constraints":[["nonneg","x"],["exact_div","a","b"]...]}
"""
from __future__ import annotations

import json
import re

# ---------------------------------------------------------------------------
# identifier rules — mirror cell80/src/plan.rs `ident_ok` (plan.rs:196-222)
# ---------------------------------------------------------------------------
RUST_KEYWORDS = set(
    "as break const continue crate dyn else enum extern false fn for if impl in "
    "let loop match mod move mut pub ref return static struct super trait true "
    "type unsafe use where while async await".split()
)
RUST_RESERVED = set(
    "abstract become box do final macro override priv typeof unsized virtual "
    "yield try union".split()
)
RENDER_RESERVED = {"self", "run"}
BAD_IDS = RUST_KEYWORDS | RUST_RESERVED | RENDER_RESERVED
IDENT_RE = re.compile(r"^[a-z_][a-z0-9_]*$")

OPS = {"add", "sub", "mul", "div"}
# common synonyms models reach for
OP_ALIASES = {
    "plus": "add", "add": "add", "sum": "add", "+": "add",
    "minus": "sub", "sub": "sub", "subtract": "sub", "-": "sub", "difference": "sub",
    "times": "mul", "mul": "mul", "multiply": "mul", "product": "mul", "*": "mul", "x": "mul",
    "divide": "div", "div": "div", "quotient": "div", "/": "div",
}


def ident_ok(s: str) -> bool:
    return bool(IDENT_RE.match(s)) and s not in BAD_IDS


class Reject(Exception):
    """A typed, nameable refusal. .code is the harvestable repair-row category."""

    def __init__(self, code: str, msg: str = ""):
        self.code = code
        self.msg = msg
        super().__init__(f"{code}: {msg}")


# ---------------------------------------------------------------------------
# 1. tolerant parse: text -> python object
# ---------------------------------------------------------------------------
def tolerant_parse(text):
    if not isinstance(text, str):
        return text  # already parsed
    t = text.strip()
    m = re.search(r"```(?:json)?\s*(.*?)\s*```", t, re.DOTALL)
    if m:
        t = m.group(1).strip()
    # carve out the first balanced {...} or [...] blob
    t = _first_blob(t) or t
    # bare `?` used as a value ("value": ?) -> null (row89 shape)
    t = re.sub(r":\s*\?", ": null", t)
    # trailing commas before } or ]
    t = re.sub(r",\s*([}\]])", r"\1", t)
    for candidate in (t, t.replace("'", '"')):
        try:
            return json.loads(candidate)
        except Exception:
            continue
    return None


def _first_blob(t: str):
    start = None
    opener = None
    for i, ch in enumerate(t):
        if ch in "{[":
            start, opener = i, ch
            break
    if start is None:
        return None
    closer = "}" if opener == "{" else "]"
    depth = 0
    for i in range(start, len(t)):
        if t[i] == opener:
            depth += 1
        elif t[i] == closer:
            depth -= 1
            if depth == 0:
                return t[start : i + 1]
    return None


# ---------------------------------------------------------------------------
# 2. locate candidate plan dict(s) anywhere in the emitted object
# ---------------------------------------------------------------------------
def locate_plans(obj):
    """Return a list of raw plan-dicts. Handles flat plans, {plans:[...]},
    tool-call {arguments:{...}} / {parameters:{...}} envelopes, and lists."""
    out = []

    def looks_like_plan(d):
        return isinstance(d, dict) and (
            "quantities" in d or "ops" in d or "operations" in d
        )

    def walk(x, depth=0):
        if depth > 4:
            return
        if isinstance(x, list):
            for item in x:
                walk(item, depth + 1)
        elif isinstance(x, dict):
            if looks_like_plan(x):
                out.append(x)
                return
            for key in ("plans", "plan", "arguments", "parameters", "input", "args"):
                if key in x:
                    walk(x[key], depth + 1)

    walk(obj)
    return out


# ---------------------------------------------------------------------------
# 3. per-plan normalisation
# ---------------------------------------------------------------------------
def _as_int(v):
    """Return an int if v is an integer value (int, or a clean integer string)."""
    if isinstance(v, bool):
        return None
    if isinstance(v, int):
        return v
    if isinstance(v, float):
        return int(v) if v.is_integer() else None
    if isinstance(v, str):
        s = v.strip()
        if re.fullmatch(r"-?\d+", s):
            return int(s)
    return None


def _is_numeric_operand(v):
    """True if operand is a literal number (not an identifier)."""
    if isinstance(v, bool):
        return False
    if isinstance(v, (int, float)):
        return True
    if isinstance(v, str) and re.fullmatch(r"-?\d+(\.\d+)?", v.strip()):
        return True
    return False


def sanitize_ident(raw, id_map, repairs):
    """Map an arbitrary id (int, reserved word, bad chars) to a valid, stable ident."""
    key = repr(raw)
    if key in id_map:
        return id_map[key]
    if isinstance(raw, (int, float)) and not isinstance(raw, bool):
        new = f"q{int(raw)}"
        repairs.append(("int_id", raw, new))
    else:
        s = str(raw).strip().lower()
        s = re.sub(r"[^a-z0-9_]", "_", s)
        if not s or s[0].isdigit():
            s = "q_" + s
        if s in BAD_IDS:
            s = s + "_"
            repairs.append(("reserved_keyword", raw, s))
        elif s != str(raw):
            repairs.append(("sanitize_id", raw, s))
    id_map[key] = new if isinstance(raw, (int, float)) and not isinstance(raw, bool) else s
    return id_map[key]


def normalize_op(o, repairs):
    """Coerce one op into a raw 4-tuple (op, a, b, out) with operands still raw."""
    if isinstance(o, dict):
        op = o.get("op") or o.get("operator") or o.get("operation")
        a = o.get("a", o.get("a_id", o.get("left", o.get("x"))))
        b = o.get("b", o.get("b_id", o.get("right", o.get("y"))))
        out = o.get("out", o.get("out_id", o.get("result", o.get("output", o.get("to")))))
        if "args" in o and (a is None or b is None):
            args = o["args"]
            if isinstance(args, list) and len(args) == 2:
                a, b = args
        repairs.append(("op_object_to_array", None, None))
        return [op, a, b, out]
    if isinstance(o, list):
        # nested-array op: [op, [a, b], out]
        if len(o) == 3 and isinstance(o[1], list) and len(o[1]) == 2:
            repairs.append(("op_nested_array", None, None))
            return [o[0], o[1][0], o[1][1], o[2]]
        if len(o) == 4:
            return list(o)
        if len(o) == 3:
            # [op, a, out] with an implied/duplicated operand — cannot safely guess
            raise Reject("op_bad_arity", f"op has 3 elements: {o!r}")
        raise Reject("op_bad_arity", f"op arity {len(o)}: {o!r}")
    raise Reject("op_bad_shape", f"op is {type(o).__name__}: {o!r}")


def normalize_plan(p):
    repairs = []
    quantities = p.get("quantities") or p.get("quantity") or []
    ops_raw = p.get("ops") or p.get("operations") or p.get("op") or []
    if isinstance(ops_raw, dict):
        ops_raw = [ops_raw]
    target = p.get("target") or p.get("answer") or p.get("result") or p.get("goal")

    id_map = {}
    decls = {}   # ident -> (value_or_None, unit)
    for q in quantities:
        if not isinstance(q, dict):
            continue
        raw_id = q.get("id", q.get("name", q.get("label")))
        if raw_id is None:
            raise Reject("quantity_no_id", f"quantity missing id: {q!r}")
        qid = sanitize_ident(raw_id, id_map, repairs)
        decls[qid] = (_as_int(q.get("value")), q.get("unit") or "scalar")

    # normalise op shapes, then resolve operands
    ops = []
    for o in ops_raw:
        op, a, b, out = normalize_op(o, repairs)
        alias = OP_ALIASES.get(str(op).strip().lower()) if op is not None else None
        if alias is None:
            raise Reject("unknown_op", f"op {op!r} not in add/sub/mul/div")
        ops.append([alias, a, b, out])

    produced = set()          # ids created by ops (in order)
    const_pool = {}           # int value -> const ident
    extra_quantities = []     # synthesized const quantities
    inserted_ops = []         # expanded inline-expression ops
    resolved_ops = []

    def resolve_operand(v):
        # already a known declared id or a produced id?
        if isinstance(v, str):
            mapped = id_map.get(repr(v), v)
            if mapped in decls or mapped in produced:
                return mapped
            # inline expression operand, e.g. "(ratio_a+ratio_b)"
            expr = _try_inline_expr(v, decls, produced, const_pool, extra_quantities,
                                    inserted_ops, repairs)
            if expr is not None:
                return expr
        # a bare numeric literal -> promote to a const quantity
        if _is_numeric_operand(v):
            n = _as_int(v)
            if n is None:
                raise Reject("requires_fractional_scale",
                             f"non-integer literal operand {v!r}")
            if n not in const_pool:
                cid = f"c_{n}" if n >= 0 else f"c_neg{abs(n)}"
                const_pool[n] = cid
                extra_quantities.append({"id": cid, "value": n, "unit": "scalar"})
                repairs.append(("promote_literal", v, cid))
            return const_pool[n]
        # a string that maps to nothing and isn't numeric: undefined reference
        if isinstance(v, str):
            mapped = id_map.get(repr(v), v)
            raise Reject("undefined_reference", f"operand {mapped!r} is never defined")
        raise Reject("op_operand_bad", f"operand {v!r}")

    for op, a, b, out in ops:
        ra = resolve_operand(a)
        rb = resolve_operand(b)
        rout = sanitize_ident(out, id_map, repairs) if out is not None else None
        if rout is None:
            raise Reject("op_no_out", f"op {op} has no output id")
        for io in inserted_ops:
            resolved_ops.append(io)
        inserted_ops.clear()
        resolved_ops.append([op, ra, rb, rout])
        produced.add(rout)

    # --- placeholder / derived / double-assignment cleanup ---------------
    # A declared quantity that an op also produces is a placeholder for a
    # derived value: drop the declaration, keep the op. A declared quantity
    # with no usable value that is NOT produced is a genuine undefined.
    final_quantities = []
    for qid, (val, unit) in decls.items():
        if qid in produced:
            repairs.append(("drop_derived_decl", qid, None))
            continue
        if val is None or val < 0:
            # unused junk placeholder we can simply forget?
            if not _id_used(qid, resolved_ops, target, id_map):
                repairs.append(("drop_unused_bad_decl", qid, None))
                continue
            raise Reject("undefined_quantity", f"quantity {qid!r} has no usable value")
        final_quantities.append({"id": qid, "value": val, "unit": unit})
    final_quantities.extend(extra_quantities)

    # --- target ----------------------------------------------------------
    if target is not None:
        target = id_map.get(repr(target), target)
    if not target or (target not in produced and target not in {q["id"] for q in final_quantities}):
        inferred = _infer_target(resolved_ops)
        if inferred is None:
            raise Reject("target_missing", "no target and no op to infer one from")
        repairs.append(("infer_target", target, inferred))
        target = inferred

    plan = {"quantities": final_quantities, "ops": resolved_ops, "target": target}
    return plan, repairs


def _id_used(qid, ops, target, id_map):
    if target is not None and id_map.get(repr(target), target) == qid:
        return True
    for op, a, b, out in ops:
        if qid in (a, b, out):
            return True
    return False


def _infer_target(ops):
    if not ops:
        return None
    produced = [o[3] for o in ops]
    consumed = set()
    for o in ops:
        consumed.add(o[1])
        consumed.add(o[2])
    # the sink: a produced id nobody later consumes; fall back to last op out
    for pid in reversed(produced):
        if pid not in consumed:
            return pid
    return produced[-1]


_EXPR_RE = re.compile(r"^\(?\s*([a-z0-9_]+)\s*([-+*/])\s*([a-z0-9_]+)\s*\)?$")


def _try_inline_expr(v, decls, produced, const_pool, extra_quantities, inserted_ops, repairs):
    """Expand a single-binop operand like "(ratio_a+ratio_b)" into its own op."""
    m = _EXPR_RE.match(v.strip())
    if not m:
        return None
    a, sym, b = m.group(1), m.group(2), m.group(3)
    if a not in decls and a not in produced:
        return None
    if b not in decls and b not in produced:
        return None
    op = OP_ALIASES.get(sym)
    if op is None:
        return None
    out = f"expr_{a}_{op}_{b}"
    inserted_ops.append([op, a, b, out])
    produced.add(out)
    repairs.append(("expand_inline_expr", v, out))
    return out


# ---------------------------------------------------------------------------
# top-level entry
# ---------------------------------------------------------------------------
def normalize(raw):
    """raw: a string (model text) or an already-parsed object.
    Returns (plans:list[dict], repairs:list) on success.
    Raises Reject(code, msg) on a typed, nameable failure."""
    obj = tolerant_parse(raw)
    if obj is None:
        raise Reject("invalid_json", "could not parse any JSON object")
    candidates = locate_plans(obj)
    if not candidates:
        raise Reject("no_plan", "no quantities/ops found anywhere in the object")
    plans = []
    all_repairs = []
    for c in candidates:
        plan, repairs = normalize_plan(c)
        plans.append(plan)
        all_repairs.extend(repairs)
    return plans, all_repairs


if __name__ == "__main__":
    import sys

    data = sys.stdin.read()
    try:
        plans, repairs = normalize(data)
        print(json.dumps({"ok": True, "plans": plans,
                          "repairs": [list(r) for r in repairs]}, indent=2))
    except Reject as e:
        print(json.dumps({"ok": False, "code": e.code, "msg": e.msg}, indent=2))
