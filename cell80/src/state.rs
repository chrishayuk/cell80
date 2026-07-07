//! `StateCell` — typed I/O by field name (the JSON↔state agent surface).
use super::*;
use std::collections::HashMap;

/// Where a [`StateCell`]'s state struct is laid out — clear of code (`ORG`), the scratch
/// register file, the trampoline, and the stack.
pub const STATE_BASE: u16 = 0xB000;

/// A cell bound to a **state struct** at [`STATE_BASE`] — typed I/O by *field name*. The
/// agent/MCP surface for "named inputs in → run → named outputs out": compile once, then
/// `set` fields, `run`, `get` fields; the layout maps names to addresses. The program is a
/// method on the state (`impl State { fn run(&mut self) … }`), reached through `&mut self`.
///
/// ```
/// use cell80::{StateCell, DEFAULT_CYCLES};
/// let src = "struct State { x: u16, score: u16 }
///            impl State { fn run(&mut self) -> u16 { self.score = self.x * 2u16; self.score } }";
/// let mut cell = StateCell::bind(src, "State", None)?;
/// cell.set("x", 10)?;
/// cell.run(DEFAULT_CYCLES)?;
/// assert_eq!(cell.get("score"), Some(20));   // typed, by name — no raw addresses
/// # Ok::<(), String>(())
/// ```
pub struct StateCell {
    runner: Runner,
    addrs: HashMap<String, (u16, Ty)>, // scalar field name -> (byte address, width)
    entry: String,
    pending: Vec<(u16, Ty, u64)>,
}

impl StateCell {
    /// Compile `src`, bind its `state` struct's scalar fields at [`STATE_BASE`], and target
    /// `entry` (default `"<state>::run"`).
    pub fn bind(src: &str, state: &str, entry: Option<&str>) -> Result<Self, String> {
        let layout = rustz80::struct_layout(src, state)?;
        let mut addrs = HashMap::new();
        for f in &layout {
            if let Some(ty) = scalar_ty(f) {
                addrs.insert(f.name.clone(), (STATE_BASE + f.offset * 2, ty));
            }
        }
        Ok(StateCell {
            runner: Runner::compile(src)?,
            addrs,
            entry: entry.map_or_else(|| format!("{state}::run"), String::from),
            pending: Vec::new(),
        })
    }

    /// Queue a named input (written into the state before the next [`run`](StateCell::run))
    /// at the field's own width — a `u32` field takes the full 32-bit value.
    pub fn set(&mut self, field: &str, value: u64) -> Result<(), String> {
        let &(addr, ty) = self
            .addrs
            .get(field)
            .ok_or_else(|| format!("no scalar field `{field}`"))?;
        if ty.capacity().is_some() {
            return Err(format!(
                "field `{field}` is {ty} — a buffer, not a scalar; the byte-buffer \
                 I/O surface arrives with Phase S3"
            ));
        }
        self.pending.push((addr, ty, value));
        Ok(())
    }

    /// Run the entry with `&mut self` state at [`STATE_BASE`], applying then clearing the
    /// queued inputs.
    pub fn run(&mut self, budget: u64) -> Result<Report, String> {
        let pending = std::mem::take(&mut self.pending);
        self.runner
            .run_with_inputs(Some(&self.entry), &[STATE_BASE], &pending, budget)
    }

    /// Read a named **scalar** field from the last run's state, at the field's own
    /// width. A `bytes[N]`/`str[N]` buffer field returns `None` (its byte-I/O
    /// surface is Phase S3).
    pub fn get(&self, field: &str) -> Option<u64> {
        self.addrs.get(field).and_then(|&(a, ty)| match ty {
            Ty::U8 => Some(self.runner.peek_u8(a) as u64),
            Ty::U16 => Some(self.runner.peek_u16(a) as u64),
            Ty::U32 | Ty::F32 => Some(self.runner.peek_u32(a) as u64),
            Ty::Bytes(_) | Ty::Str(_) => None,
        })
    }

    /// The bound (scalar) field names.
    pub fn fields(&self) -> impl Iterator<Item = &str> {
        self.addrs.keys().map(String::as_str)
    }
}

/// The addressable kind of a layout field: one slot → `u16` (a `u8` field also reads
/// fine as its low byte), a two-slot `dword` → `u32`, a byte-packed `[u8; N]` field →
/// `bytes[N]` (ABI v3 — declared with its capacity so a caller reads the envelope;
/// the scalar `set`/`get` paths reject it until the S3 byte-I/O surface). Word-slot
/// arrays and tuples are not name-addressed.
fn scalar_ty(f: &rustz80::FieldLayout) -> Option<Ty> {
    if let Some(n) = f.bytes {
        Some(Ty::Bytes(n))
    } else if f.f32 {
        Some(Ty::F32)
    } else if f.dword {
        Some(Ty::U32)
    } else if f.slots == 1 {
        Some(Ty::U16)
    } else {
        None
    }
}

/// Byte addresses of a state cell's **scalar** fields — `(name, addr, ty)` at
/// [`STATE_BASE`], in declaration order — so a warm host (or a `.cell`) can drive the cell
/// *by name* without the source. `ty` is the field's width: a `u32` field is 4 bytes /
/// two slots (little-endian, low word first). Empty for a free-function entry (no
/// `&mut self` state). Uses the exact compiler [`struct_layout`](rustz80::struct_layout);
/// tolerant of a non-state entry (returns empty).
pub fn state_field_addrs(src: &str, entry: &str) -> Result<Vec<(String, u16, Ty)>, String> {
    // A state entry is `Struct::method`; the receiver struct name is the part before `::`.
    let state_struct = match entry.split_once("::") {
        Some((s, _)) => s,
        None => return Ok(Vec::new()),
    };
    let layout = match rustz80::struct_layout(src, state_struct) {
        Ok(l) => l,
        Err(_) => return Ok(Vec::new()), // not a known struct → no named state
    };
    Ok(layout
        .into_iter()
        .filter_map(|f| {
            let ty = scalar_ty(&f)?;
            Some((f.name, STATE_BASE + f.offset * 2, ty))
        })
        .collect())
}
