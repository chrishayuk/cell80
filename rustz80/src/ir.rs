//! The compiler's own small typed IR — decoupled from `syn`. Stage 0 is `u16`
//! throughout (8-bit narrowing comes later); locals are addressed by slot.

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
    /// A **u32** comparison as a value: `1`/`0` in `HL` (`Width::Byte` bool).
    /// Unsigned only (the dialect has no `i32`). Ordering rides the 32-bit `SBC`
    /// chain's borrow; equality tests the difference's four bytes. In condition
    /// position this materialises and branches on `!= 0` (the compound-`Cond`
    /// pattern) — `Cond` itself stays 16-bit.
    Cmp32 {
        cmp: Cmp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
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
    /// A `u32` binary op: `+ - * / %` (add/sub as an inline carry chain; mul/div via
    /// the software runtime on Spectrum or the `ED FE` trap on Cell) and `| & ^`.
    Bin32(BinOp, Box<Expr>, Box<Expr>),
    /// A `u32` shift by a constant: `e << k` (`left`) or `e >> k`.
    Shift32 { left: bool, e: Box<Expr>, k: u8 },
    /// Truncate a `u32` to its low `u16` (`x as u16`) — the bridge back to 16-bit.
    Trunc32(Box<Expr>),
    /// Read a `u32` at `*(ptr + byte_offset)` — a wide field access through a pointer
    /// (`self.total` where `total: u32`; two little-endian slots, low word first).
    Deref32(Box<Expr>, usize),
    /// Widen a 16-bit expr to `u32` (`x as u32`) — zero-extend into the high word. The bridge
    /// *up* to 32-bit, so a `u16` can feed a `u32` op (e.g. a wide intermediate).
    Widen(Box<Expr>),
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
/// the calling-convention registers in the prologue).
#[derive(Debug, Clone)]
pub struct Func {
    pub params: usize,
    pub n_locals: usize,
    pub body: Vec<Stmt>,
    /// Return values, in the result convention `HL`/`DE`/`BC`: empty for a void fn,
    /// one entry for a scalar, two or three for a tuple return.
    pub ret: Vec<Expr>,
}
