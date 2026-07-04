# Machine text — bounded string handling for cells (Phase S)

*Status: specified, not started. Gated behind the 1K scale curve, the admission gate
(2.2), and `Halt::Escalate` (3.2) — see §9. This spec decides; open questions are
listed as decisions with a chosen default, not as musings.*

## The governing rule: frozen spec in, versioned spec out

The memo table's value is that `(image_hash, args) → result` is true **forever**. Any
cell whose correct answer depends on a spec that *revs* — the Unicode Character
Database, locale rules, timezone tables — produces cache entries that stay internally
consistent while becoming externally wrong: exactly the staleness the architecture
exists to make impossible. So the line is not "strings" and never was:

> **cell80 handles machine text** — frozen-format byte and UTF-8 *structural* work
> over bounded buffers. **Human text** — case beyond ASCII, normalization, collation,
> meaning — **is the escalation path, permanently**: any spec that versions would
> poison the memo table.

What this admits (all frozen): UTF-8 structure (unchanged since 2003), JSON framing
(RFC 8259), base64/32/16 (RFC 4648), percent-encoding (RFC 3986), CSV framing
(RFC 4180), ASCII (1968). What it excludes forever: UCD-dependent anything, locale,
encodings other than UTF-8/ASCII, runtime-compiled regex, `format!`. The division of
labour: **formats are frozen, so they belong to cells; language is living, so it
belongs to the brain.**

Substrate note: this is smaller than it looks. Byte-buffer kernels *already compile*
(`&[u8; N]` params, const data, local `[u8; N]` — digit-count/atoi/FNV/prefix-check
probes all build at 88–156 bytes today). Phase S is mostly **plumbing to the cell I/O
surface plus a stdlib**, not a compiler rewrite. And the Z80 is natively good at this
lane: `LDIR`/`CPIR` *are* string instructions, with honest cycle costs — **no new
traps** (§5).

## 1. Wire format — one convention everywhere

A string value in memory is **`len: u16` (little-endian) followed by `len` bytes**,
UTF-8 valid by contract (§4). One format for const data, params, and state fields.

- **Migration:** the shipped const-string convention is a *u8* prefix
  (`peek(s)` = len). Phase S0 moves interned consts to the u16 prefix. This is
  compiler-internal (the convention lives between emitted code and itself), so old
  *images* are untouched; only user source that hand-reads the prefix via `peek(s)`
  breaks — grep the cell corpus, expect zero hits, note it in the changelog.
- **Capacity cap: `N ≤ 1024` per field/const.** Cells are index cards; a cell that
  needs more is not a cell (`Escalate::CapacityExceeded`). The cap is a lowering
  error, not a runtime surprise.

## 2. Dialect surface (all of it is real Rust — the oracle survives)

Every accepted construct below compiles identically under rustc; `check!` keeps its
guarantee. Nothing in Phase S introduces a semantics rustc doesn't share.

**2.1 Inputs — `&str` parameters.** A `&str` param occupies one register (the address
of a §1 buffer). Accepted methods, each a few instructions:

| construct | lowering | note |
|---|---|---|
| `s.len()` | 16-bit load at `s` | |
| `s.is_empty()` | load, `== 0` | predicate convention |
| `s.as_bytes()[i]` | byte load at `s + 2 + i`, zero-extend | **no bounds check** — same as array fields today; guard with `i < s.len()` (the library idiom) |
| `s.is_char_boundary(i)` | `i == 0 \|\| i >= len \|\| (b[i] & 0xC0) != 0x80` | pure bit test, real Rust semantics |

`&[u8; N]` params stay as shipped (compile-time N, no prefix). `&str` is the
runtime-length sibling. Both fit the 3-param convention; multiple `&str` params are
fine (one register each).

**2.2 Byte literals — `b'a'` and `b"HTTP"`.** Currently a rejection class; both
become features. `b'a'` is a `u8` literal (ASCII work stops being magic numbers —
`c >= b'0' && c <= b'9'`); `b"…"` is a `[u8; N]` const (packs into the const-data
section like any array const). Real Rust, diff-tests for free, and removes the single
ugliest idiom in the probe cells.

**2.3 Outputs — a manifest convention, not a language feature.** A cell builds output
into an ordinary field pair it already (post-`[u8;N]`-fields) can express:

```rust
struct Slug { input: StrIn, out_len: u16, out: [u8; 64], run: … }
// body: self.out[self.out_len as usize] = b; self.out_len += 1;  ← plain Rust
```

The **manifest** annotates `str_out = (out_len, out)`; the **host** promotes the pair
to a string at the boundary after a UTF-8 validity check (§4). No `String`, no push
method, no new semantics — the dialect stays 100% oracle-checkable and output-building
code is boring array code. Overflow is the cell's problem to signal:
`Escalate::CapacityExceeded`, or a truncation flag field — the library standardises on
**escalate** (truncation is silent data loss wearing a flag).

**2.4 Not in the dialect** (each keeps its instructive diagnostic): `String`, `&str`
returns, string concatenation/`+`, `format!`/`write!`, `char` as a type (codepoints
are `u32` — we have `u32`), slicing syntax `&s[a..b]` (a *span* is `(offset, len)` —
two u16s, the composition currency of §6), iterator adapters over strings.

## 3. ABI v3 — one new manifest type, symmetric edges

- Manifest field types grow `str[N]` and `bytes[N]` (`str[N]` = §1 pair with UTF-8
  contract; `bytes[N]` = same layout, no contract). `state_addrs` carries them like
  any width; `.cell` format bumps (back-compat reads, as v4 did).
- **CellGraph edges:** `str[N] → str[M]` checks `M ≥ N` at graph-validation time
  (capacity-widening wires pass, narrowing rejects — the u32→u16 rule's sibling).
  `str → bytes` coerces free (a str *is* bytes). `bytes → str` inserts a **host
  validation step**; failure is a typed graph halt, not a poisoned downstream cell.
- MCP/PyO3: `cell_run(fields=…)` accepts/returns Python `str`/`bytes` for these
  fields; the host does the §1 packing. `entry_signature` renders capacities so an
  agent reads the envelope before wiring.

## 4. The validity contract (the graph-typechecker move)

**Cells assume their `str` inputs are valid UTF-8; the host establishes it once at
the boundary.** Ingest paths (MCP, PyO3, graph `bytes→str` edges, promoted outputs)
validate; a failure is a typed error naming the field — never replacement characters,
never silent lossy repair (a memo table must not cache a lie). Inside the fence,
cells skip re-validation — which is precisely what keeps byte-loop cells at
tens of bytes. `utf8_validate` also ships as an ordinary cell (§6) so a graph can
make the check *explicit and metered* when it wants to.

## 5. Cost: no new traps

String work is byte loops and block moves — the lane the ISA is natively good at.
Copies lower to `LDIR`, scans to `CPIR`-shaped loops, all cycle-honest on both
targets with **zero ABI trap-surface growth**. This is deliberate: every trap added
is a `trapped_ops` weighting question the ABI doc admits it can't answer faithfully.
Frozen trap set = the cost surface stays as honest as it is today. (If profiling ever
argues for a `MEMCMP` trap, that's a Phase S+1 debate with the gate-not-gradient rule
in the room.)

## 6. The stdlib — Phase S packs (confusable by design, eval tax per cell)

The composition currency is the **span**: `(offset: u16, len: u16)` into an input
buffer. Scanners produce spans; parsers/validators consume them; tuple returns
(≤3 registers) carry `(off, len, ok)`. This is what lets `csv_field → trim_span →
parse_u16` wire as a graph without copying.

- **validators** — `all_digits`, `all_hex`, `is_ascii`, `is_ascii_printable`,
  `starts_with`, `ends_with`, `memeq`, `eq_ignore_ascii_case`, shape checks
  (`matches_date_shape` `YYYY-MM-DD`, postcode-style fixed patterns).
- **scanners (span producers)** — `find_byte`, `rfind_byte`, `count_byte`,
  `split_at_byte`, `trim_spaces_span`, `token_next` (delimiter walk, state cell).
- **parsers** — `parse_u16`, `parse_u32`, `parse_hex`, `parse_i16` — all
  `(value, ok)` tuples, overflow → `ok = 0` (never a wrapped value: a garbage parse
  must not flow onward, the div-by-zero lesson).
- **codecs** — `hex_encode`/`hex_decode`, base64 encode/decode (RFC 4648),
  `percent_encode`/`decode` (RFC 3986) — the frozen-RFC shelf.
- **transforms (ASCII envelope)** — `to_upper_ascii`, `to_lower_ascii`,
  `slugify_ascii`, `sanitize_ident`; non-ASCII byte encountered →
  `Escalate::NonAscii` (the fence has a verb, not an approximation).
- **utf8 (structural)** — `utf8_validate` (Höhrmann DFA, ~400 B const table — the
  first showcase of const-data-as-behaviour), `utf8_char_count`, `utf8_next_cp`
  (decode one codepoint → `u32`), `cp_encode_utf8`.
- **json-structural (RFC 8259 framing only)** — `json_skip_ws`, `json_match_brace`,
  `json_string_span`, `json_field_span` (find `"key":` value span), `json_type_of`.
  Framing, never semantics: numbers route to the parsers pack, unescaping to codecs.
- **experimental, own kill gate** — `dfa_run`: a DFA runner whose transition table is
  const data (compile-time-fixed pattern matching, no regex engine). Either the
  elegant outer edge of the contract or the first step off the cliff; the gate is
  size (< 1 KiB with table) + a measured adoption row, and the answer gets banked
  either way.

Every pack lands under the contribution rule (retrieval rows, host oracle, probes at
ingest) — and string families are *savagely* confusable
(`json_field_span`/`json_string_span`/`find_byte`…), which is exactly the retrieval
pressure the 1K curve is supposed to have already survived (§9).

## 7. Testing

- **`check_str!`** joins the harness: same stringify trick, `fn f(s: &str) -> u16` —
  the host calls with a `&'static str`; the z80 side packs the §1 buffer at a fixed
  address and passes it in `HL`. Both targets, rustc oracle, as ever.
- **Boundary fuzz**: the §4 fence gets its own property test — random byte soup
  through every ingest path; invalid input must *never* reach a cell as `str`.
- **Repair rows**: every §2.4 rejection and the two byte-literal classes join
  `cell-eval repair`; the wide-literal fix (`let w: u32 = 100000;`) ships alongside
  since Phase S authors will hit it constantly.

## 8. Explicit non-goals (the versioned-spec shelf, permanent)

Case mapping beyond ASCII · Unicode normalization/collation/segmentation · character
classes beyond ASCII · locale/timezone anything · encodings other than UTF-8/ASCII ·
runtime-compiled patterns · `format!`-style templating (fixed-template fill with
spans is a cell; a template *language* is not) · unbounded growth. The test for any
future proposal is one question: **does its spec version?**

## 9. Sequencing — gates, not dates

| gate | what | why it blocks |
|---|---|---|
| **S-pre** | 1K curve read · admission gate live · `Escalate` shipped | strings multiply the confusable-family load; landing them on an unproven index piles the hardest retrieval problem onto the least evidence. NonAscii/CapacityExceeded need the escalate verb to exist. |
| **S0** | `[u8; N]` state fields (the speccy ask — shared cost) · u16-prefix migration · ABI v3 manifest types | the plumbing everything above rides on |
| **S1** | `&str` params + §2.1 methods · byte literals · `check_str!` | inputs before outputs — read-only kernels are the low-risk half |
| **S2** | validators/scanners/parsers/codecs waves through the gate · **pre-registered adoption question** | ~20 seed cells + validation-shaped adoption tasks; *kill gate:* if agents shortcut to host code on ≥ half the string tasks with the cells on the shelf, stop and diagnose (steering vs retrieval vs envelope) before authoring more |
| **S3** | `str_out` promotion · graph `str` edges · json-structural + utf8 packs | the composition payoff; capstone eval = a `csv_field → trim → parse_u16 → range_check` graph, wired by the model |

Early-warning instrument, cheap and worth running from wave 4 onward: track the
adoption eval's *shortcut rate on string-shaped tasks*. When it starts screaming for
`parse_u16(&str)`, that's the S-pre → S0 start signal — demand-driven, like the
library itself.

## The one-sentence version

Text is a byte convention, not a type; the host guards the UTF-8 fence; the dialect
stays real Rust end to end; frozen specs are in, versioned specs are out, and the
memo table stays true forever.
