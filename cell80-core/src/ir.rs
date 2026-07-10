//! The compiler's own small typed IR — decoupled from `syn`, and (A2,
//! `docs/13-multi-target-spec.md` §2.2) carrying **target-independent semantics**:
//!
//! - **Every value's width is statically explicit** — on the node family (the
//!   `*32` siblings are `DWord` by construction) or a [`Width`] parameter. There
//!   is no implicit width anywhere; a backend may compute at its native width so
//!   long as results wrap (mod 2^width) at every step a program can observe.
//! - **The slot ABI is family-wide**: locals, array elements, and struct fields
//!   occupy 2-byte little-endian slots (`u8` in the low byte, `u32`/f32-bits as
//!   two consecutive slots, low word first), byte-addressed. This is the frozen
//!   `StateCell`/manifest ABI — a wider-word backend loads/stores 2-byte slots,
//!   it does not get a wider slot.
//! - **Width bridges are explicit ops** — [`Expr::Trunc`] (to u8),
//!   [`Expr::Trunc32`] (u32 → its low u16), [`Expr::Widen`] (zero-extend to u32),
//!   [`Expr::SignExtend`] (sign-extend to u32). Nothing converts implicitly.
//! - **Evaluation order is left-to-right wherever observable** (A2a): an operand
//!   pair containing a side effect evaluates in source order; effect-free pairs
//!   may reorder.
//!
//! The 16-bit/32-bit node-family *split* (`Lit`/`Lit32`, `Bin`/`Bin32`, …) is not
//! a Z80-ism to erase: it is how widths stay static without a type checker over
//! the IR. Merging the families into width-parameterised nodes is deferred until
//! a second backend supplies evidence the merge is the right shape (the spec's
//! "an abstraction that still fits backend zero has earned it").

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Or,
    And,
    Xor,
    /// Left/right shift by a constant amount (the RHS is always a [`Expr::Lit`]).
    Shl,
    Shr,
}

/// Value width. `u8`/`u16`/`i16` occupy one 2-byte slot (`u8` zero-extends on load;
/// `i16` — `SWord` — is two's-complement: add/sub/mul/bitwise share the unsigned bit
/// patterns, only compare / divide / arithmetic-shift-right differ); `u32` (`DWord`)
/// occupies two slots and is computed in the `HL:DE` pair (`HL` = low word, `DE` =
/// high word) by the dedicated 32-bit codegen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Width {
    Byte,
    Word,
    SWord,
    DWord,
    /// `i32` — two's-complement over `DWord` storage (two slots, same wrapping
    /// add/sub/mul/bitwise bit patterns); only compare / divide / arithmetic-`>>`
    /// differ, carried as `signed` flags on the 32-bit nodes. **IR + interpreter
    /// only until a backend lands** (Phase 5 A3: the signed-32 ops are gated out of
    /// Z80 codegen with an instructive error; RV32 gets them natively at WS-B).
    SDWord,
    /// IEEE binary32 bits riding a u32 — a **lowering-only** type: it storage-plumbs
    /// exactly like `DWord` (two slots, `HL:DE`, wide call convention) but routes
    /// arithmetic/comparisons through the owned softfloat kernels instead of `Bin32`/
    /// `Cmp32`, and never mixes with integer widths implicitly. Codegen receives only
    /// the baked-in wide nodes; `F32` itself never reaches an IR node's semantics.
    F32,
}

impl Width {
    /// Two-slot values: `u32`/`i32`, and f32 bits riding u32. Use this for
    /// storage/call plumbing; use [`Width::is_int_wide`] when the *operation* is
    /// 32-bit integer arithmetic.
    pub fn is_wide(self) -> bool {
        matches!(self, Width::DWord | Width::SDWord | Width::F32)
    }
    /// A 32-bit *integer* lane (`u32` or `i32`) — the `Bin32`/`Cmp32`/`Shift32`
    /// node family, signed or not.
    pub fn is_int_wide(self) -> bool {
        matches!(self, Width::DWord | Width::SDWord)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmp {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

#[derive(Debug, Clone)]
pub enum Expr {
    /// An integer literal.
    Lit(u16),
    /// A local variable, by slot index.
    Var(usize),
    /// A binary arithmetic op; `Width::Byte` masks the result to 8 bits (u8 wrap).
    Bin(BinOp, Box<Expr>, Box<Expr>, Width),
    /// A call to another function by name (args by the calling convention).
    Call(String, Vec<Expr>),
    /// Read array element `base[index]` (`base` is the array's first slot).
    Index(usize, Box<Expr>, Width),
    /// Truncate to 8 bits (`expr as u8`).
    Trunc(Box<Expr>),
    /// Read a byte from a raw address: `peek(addr)` (intrinsic).
    Peek(Box<Expr>),
    /// Read a byte from an I/O port: `inport(port)` (intrinsic, e.g. the keyboard).
    InPort(Box<Expr>),
    /// Absolute address of a local slot (`&local`) — for passing `&self`.
    AddrOf(usize),
    /// The absolute address of a named **const-data** item (`&TILE`, an interned
    /// string literal) — symbolic here, resolved against the data section laid
    /// after the code at encode.
    ConstAddr(String),
    /// Read a `u16` at `*(ptr + byte_offset)` — field access through a pointer
    /// (`self.field`).
    Deref(Box<Expr>, usize),
    /// Read a `u16` array element through a pointer: `*(ptr + off + index*2)` — an
    /// array *field* reached through a pointer receiver (`self.arr[index]`).
    PtrIndex {
        ptr: Box<Expr>,
        off: usize,
        index: Box<Expr>,
    },
    /// Multiply by a compile-time constant (`expr * k`) — used to scale an index by a
    /// struct element's byte stride. Powers of two shift; else the mul micro-runtime.
    MulConst(Box<Expr>, u16),
    /// Load a value (zero-extended for `Width::Byte`) at the byte address in `expr` —
    /// used to read a field of a struct-array element at a computed address.
    LoadAt(Box<Expr>, Width),
    /// A comparison as a **value**: `lhs <cmp> rhs` materialised to `1`/`0` in `HL`
    /// (a `bool`, `Width::Byte`). In condition position a comparison stays a [`Cond`]
    /// (a direct branch); this node is the value form, e.g. `(a < b) as u16`.
    Cmp {
        cmp: Cmp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        /// Two's-complement (`i16`) comparison — `<`/`>` order by sign (S ⊕ V), not
        /// magnitude. `==`/`!=` are sign-agnostic either way.
        signed: bool,
    },
    /// Short-circuit logical op on boolean (`0`/`1`) operands: `&&` (`and = true`) or
    /// `||` (`and = false`). The right operand is only evaluated when the left doesn't
    /// already decide the result (Rust short-circuit semantics).
    Logic {
        and: bool,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    /// A 32-bit comparison as a value: `1`/`0` in `HL` (`Width::Byte` bool).
    /// Unsigned ordering rides the 32-bit `SBC` chain's borrow; equality tests the
    /// difference's four bytes; `signed` (`i32`) orders by two's complement.
    /// In condition position this materialises and branches on `!= 0` (the
    /// compound-`Cond` pattern) — `Cond` itself stays 16-bit.
    Cmp32 {
        cmp: Cmp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        signed: bool,
    },
    /// Shift by a **runtime** amount: `e << amount` (`left = true`) or `e >> amount`.
    /// The amount's low byte is the count (a count ≥ 16 shifts a `u16` out to `0`);
    /// a *literal* amount uses [`BinOp::Shl`]/[`BinOp::Shr`] (unrolled) instead.
    ShiftVar {
        left: bool,
        e: Box<Expr>,
        amount: Box<Expr>,
        w: Width,
    },

    // --- 32-bit (`u32`) nodes — evaluated into the `HL:DE` pair by `gen_expr32` ---
    /// A `u32` literal.
    Lit32(u32),
    /// A `u32` local, by slot index (occupies `slot` and `slot + 1`).
    Var32(usize),
    /// A 32-bit binary op: `+ - * / %` (add/sub as an inline carry chain; mul/div via
    /// the software runtime on Spectrum or the `ED FE` trap on Cell) and `| & ^`.
    /// `signed` (`i32`) changes only `/`/`%` (truncate toward zero, remainder takes
    /// the dividend's sign); add/sub/mul/bitwise share the unsigned bit patterns.
    Bin32(BinOp, Box<Expr>, Box<Expr>, bool),
    /// A 32-bit shift by a constant: `e << k` (`left`) or `e >> k`. `signed` makes
    /// the right shift arithmetic (`i32 >> k` sign-propagates); left shifts and
    /// unsigned right shifts ignore it.
    Shift32 {
        left: bool,
        e: Box<Expr>,
        k: u8,
        signed: bool,
    },
    /// Truncate a `u32` to its low `u16` (`x as u16`) — the bridge back to 16-bit.
    Trunc32(Box<Expr>),
    /// Read a `u32` at `*(ptr + byte_offset)` — a wide field access through a pointer
    /// (`self.total` where `total: u32`; two little-endian slots, low word first).
    Deref32(Box<Expr>, usize),
    /// Widen a 16-bit expr to `u32` (`x as u32`) — zero-extend into the high word. The bridge
    /// *up* to 32-bit, so a `u16` can feed a `u32` op (e.g. a wide intermediate).
    Widen(Box<Expr>),
    /// Widen a **signed** 16-bit expr to `u32` (`i16 as u32`) — the high word takes the
    /// sign fill (rustc's `as` semantics). With [`Expr::Trunc`]/[`Expr::Trunc32`]/
    /// [`Expr::Widen`] this completes the explicit width-bridge family (A2:
    /// truncate / zero-extend / sign-extend are IR ops, never implicit).
    SignExtend(Box<Expr>),
    /// `halt(code)` — a Cell80 intrinsic: stop the run with a status code (the `ED FE`
    /// HALT trap). A no-op on real hardware / the Spectrum target.
    Halt(Box<Expr>),
}

/// A boolean condition driving a branch: a comparison `lhs <cmp> rhs`. A compound or
/// non-comparison condition (`a && b`, `flag`) lowers to `<expr> != 0` — the bool
/// `<expr>` (an [`Expr::Cmp`]/[`Expr::Logic`]) materialised to `0`/`1`, then tested.
#[derive(Debug, Clone)]
pub struct Cond {
    pub cmp: Cmp,
    pub lhs: Expr,
    pub rhs: Expr,
    /// Signed (`i16`) ordering — see [`Expr::Cmp`].
    pub signed: bool,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    /// Store an expression into a local slot (covers `let` and reassignment).
    Assign(usize, Expr),
    /// Store into array element `base[index] = value`.
    StoreIndex(usize, Expr, Expr, Width),
    /// Write a byte to a raw address: `poke(addr, val)` (intrinsic).
    Poke(Expr, Expr),
    /// Write a `u16` to `*(ptr + byte_offset)` — field store through a pointer
    /// (`self.field = v`).
    Store(Expr, usize, Expr),
    /// Write a `u16` array element through a pointer: `*(ptr + off + index*2) = value`
    /// — an array *field* store through a pointer receiver (`self.arr[index] = v`).
    PtrStoreIndex {
        ptr: Box<Expr>,
        off: usize,
        index: Box<Expr>,
        value: Expr,
    },
    /// Store a value at the byte address in the first `Expr` (the low byte only for
    /// `Width::Byte`) — write a field of a struct-array element at a computed address.
    StoreAt(Expr, Expr, Width),
    /// Store a `u32` expression (evaluated in `HL:DE`) into a two-slot local.
    Assign32(usize, Expr),
    /// Write a `u32` to `*(ptr + byte_offset)` — a wide field store through a pointer
    /// (`self.total = v` where `total: u32`; two little-endian slots, low word first).
    Store32(Expr, usize, Expr),
    /// Fill `count` consecutive slots from local slot `base` with `value` (a `[v; N]`
    /// array initialiser — every element is one 2-byte slot, `u8` in the low byte). A
    /// block op: Spectrum lowers it to a first store + `LDIR`; Cell to an `ED FE` fill
    /// trap (host-native). `value` is evaluated once.
    Fill {
        base: usize,
        count: usize,
        value: Expr,
    },
    /// Evaluate an expression for its side effect, discarding the result
    /// (e.g. a `void` function call as a statement).
    Eval(Expr),
    /// Destructure a multi-value return into slots: evaluate the call (which leaves
    /// its tuple in `HL`/`DE`/`BC`) and store each register into `slots[i]`.
    /// `let (a, b) = f(…)`.
    AssignTuple(Vec<usize>, Expr),
    /// `if cond { then } else { els }`.
    If(Cond, Vec<Stmt>, Vec<Stmt>),
    /// `while cond { body }`.
    While(Cond, Vec<Stmt>),
    /// `loop { body }` — an unconditional loop, exited via [`Stmt::Break`] or
    /// [`Stmt::Return`].
    Loop(Vec<Stmt>),
    /// `for var in start..end { body }`. The loop variable's slot is initialised to
    /// `start` *before* this node; `end` is the bound, pre-evaluated into a temp slot
    /// (Rust evaluates a range bound once) and compared each iteration. `inclusive`
    /// selects `<=` over `<`. The induction step (`var += 1`, masked to `width`) runs
    /// at the `continue` target, after the body.
    ForRange {
        var: usize,
        end: Expr,
        inclusive: bool,
        width: Width,
        body: Vec<Stmt>,
    },
    /// `break` — jump past the innermost enclosing loop.
    Break,
    /// `continue` — jump to the innermost enclosing loop's step/condition.
    Continue,
    /// `return` — leave the optional value in `HL` and jump to the function epilogue.
    Return(Option<Expr>),
}

/// A lowered function. Parameters occupy local slots `0..params` (loaded from
/// the calling-convention registers in the prologue — `params` counts **slots**,
/// which equals registers: a wide first param's `[low, high]` pair is `[HL, DE]`).
#[derive(Debug, Clone)]
pub struct Func {
    pub params: usize,
    pub n_locals: usize,
    pub body: Vec<Stmt>,
    /// Return values, in the result convention `HL`/`DE`/`BC`: empty for a void fn,
    /// one entry for a scalar, two or three for a tuple return.
    pub ret: Vec<Expr>,
    /// The first parameter is a `u32` riding `HL:DE` (the one-wide-param call
    /// convention). Excluded from inlining (slot plans assume 1 slot/param).
    pub wide_param: bool,
    /// The *second* parameter is also a `u32` — it rides the **stack** (the
    /// `__mul32` shape: caller pushes high then low; the callee's prologue pops
    /// it under the return address). Implies `wide_param`; at most one more
    /// 16-bit parameter may follow in `BC`.
    pub wide_second: bool,
    /// The single return value is a `u32` in `HL:DE` (evaluated by `gen_expr32`).
    pub wide_ret: bool,
}
