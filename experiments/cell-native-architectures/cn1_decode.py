#!/usr/bin/env python3
"""CN-1 real build, step 2b (`cell-native-architectures-cn1-preregistration.md`): constrained
decoding, the LARQL `OpNameMask` seam ported to the TinyModel PyTorch generate loop.

LARQL masks by per-character op-name prefix because a Gemma op name spans several subword
tokens. CN-1 minted one atomic token per cell (step 2a), so the port collapses to its simplest
form: a **single-step mask over a fixed id set**. The grammar is one transition — immediately
after the `<call>` delimiter the next token must be a `<cell:*>` id — so the sampler seam is
exactly LARQL's `FnMut(generated_ids, &mut logits)`: after the dense LM head, before the
argmax/sample, set every logit outside the allowed set to -inf.

The allowed set is the full 790-cell id range **including axis-A held-out cells**: constrained
decoding must be able to *emit* an unseen cell, or gate (ii) could never observe a preference
for one (the mask measures selection, not vocabulary membership — that is the whole design).

No training here; this validates the decode apparatus (the pilot's discipline).
Run: python3 cn1_decode.py
"""
from __future__ import annotations

import json
from pathlib import Path

import torch

HERE = Path(__file__).resolve().parent
TOKEN_MAP = HERE / "cn1_cell_token_map.json"


def load_call_grammar():
    """Returns (call_open_id, call_close_id, cell_ids sorted list, cell_id_set)."""
    m = json.loads(TOKEN_MAP.read_text())
    call_open = m["<call>"]
    call_close = m["</call>"]
    cell_ids = sorted(v for k, v in m.items() if k.startswith("<cell:"))
    return call_open, call_close, cell_ids, set(cell_ids)


class CellCallMask:
    """The constrained-decode grammar for CN-1's call sites.

    State machine (per the corpus form `... <call> <cell:NAME> <args> </call> ...`):
      - the step whose last generated token is `<call>` is an *op-name step*: mask to cell ids.
      - every other step is free (args are ordinary digit/word tokens; `</call>` and the rest
        of the sentence are unconstrained).
    Optionally restrict the allowed cell set (e.g. to the arity-appropriate subset) — default
    is all cells, held-out included.
    """

    def __init__(self, call_open_id: int, cell_ids, allowed_cell_ids=None):
        self.call_open_id = call_open_id
        self.cell_ids = torch.tensor(sorted(cell_ids), dtype=torch.long)
        allowed = cell_ids if allowed_cell_ids is None else allowed_cell_ids
        self.allowed = torch.tensor(sorted(allowed), dtype=torch.long)

    def is_op_name_step(self, generated_ids) -> bool:
        return len(generated_ids) > 0 and int(generated_ids[-1]) == self.call_open_id

    def apply(self, generated_ids, logits: torch.Tensor) -> torch.Tensor:
        """logits: (vocab,) for the next token. Returns masked logits (in-place)."""
        if not self.is_op_name_step(generated_ids):
            return logits
        keep = torch.full_like(logits, float("-inf"))
        keep[self.allowed.to(logits.device)] = logits[self.allowed.to(logits.device)]
        return keep


@torch.no_grad()
def generate_constrained(model, prompt_ids, mask: CellCallMask, max_new=8, greedy=True):
    """Minimal autoregressive loop with the mask applied at the sampler seam. Mirrors LARQL:
    dense logits -> mask_fn -> pick. Greedy by default (deterministic, like the CN-2 baseline).
    Truncates the growing sequence to the model's max_seq window."""
    max_seq = model.base.rope_freqs.shape[0]
    device = next(model.parameters()).device
    generated = list(prompt_ids)
    for _ in range(max_new):
        window = generated[-max_seq:]
        ids = torch.tensor([window], dtype=torch.long, device=device)
        logits = model(ids)[0, -1].float().cpu()  # (vocab,)
        logits = mask.apply(generated, logits)
        nxt = int(torch.argmax(logits)) if greedy else int(torch.multinomial(torch.softmax(logits, -1), 1))
        generated.append(nxt)
    return generated


# ---- self-test ---------------------------------------------------------------------

def _selftest():
    import cn1_model

    call_open, call_close, cell_ids, cell_set = load_call_grammar()
    print(f"== call grammar: <call>={call_open} </call>={call_close}  {len(cell_ids)} cell ids "
          f"({cell_ids[0]}..{cell_ids[-1]}) ==")

    # 1. mask mechanics on a random logit vector: after <call>, only cell ids survive.
    mask = CellCallMask(call_open, cell_ids)
    vocab = cn1_model.EXTENDED_VOCAB
    logits = torch.randn(vocab)
    # not an op-name step -> unchanged
    assert torch.equal(mask.apply([call_open, 5, 6], logits.clone()), logits)
    # op-name step -> argmax must be a cell id, and every non-cell logit is -inf
    masked = mask.apply([7, call_open], logits.clone())
    assert int(torch.argmax(masked)) in cell_set, "masked argmax is not a cell id"
    finite = torch.isfinite(masked).nonzero().flatten().tolist()
    assert set(finite) == cell_set, "exactly the cell ids must remain finite"
    print(f"  OK: after <call>, exactly the {len(cell_set)} cell ids survive; argmax ∈ cell set")

    # 2. gate-(ii) emittability: every axis-A held-out cell id is in the allowed set.
    held = {h["name"] for h in json.loads((HERE / "cn1_axis_a_heldout.json").read_text())["held_out_cells"]}
    tok_map = json.loads(TOKEN_MAP.read_text())
    held_ids = [tok_map[f"<cell:{n}>"] for n in held]
    assert all(hid in cell_set for hid in held_ids), "a held-out cell is not emittable — gate (ii) impossible"
    print(f"  OK: all {len(held_ids)} axis-A held-out cells are emittable (in the allowed set)")

    # 3. end-to-end: a real (untrained-cell) model emits a valid cell token under constraint.
    print("  == loading model for a constrained generate (arm c) ==", flush=True)
    model, names, _ = cn1_model.build("fingerprint")
    prompt = [2, 388, 21221, call_open]  # bos, two words, <call>  -> next MUST be a cell id
    out = generate_constrained(model, prompt, mask, max_new=1)
    emitted = out[-1]
    assert emitted in cell_set, f"constrained generate emitted non-cell id {emitted}"
    name = next(n for n in names if tok_map[f"<cell:{n}>"] == emitted)
    print(f"  OK: constrained generate after <call> emitted a valid cell token: id {emitted} = <cell:{name}>")

    print("\nconstrained-decode self-test: PASS — single-step mask over the fixed cell id set, "
          "held-out cells emittable, generate loop honors the grammar")


if __name__ == "__main__":
    _selftest()
