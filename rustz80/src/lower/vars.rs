//! The per-function "register file": named locals (and parameters) mapped to scratch
//! slots. Flat scoping; arrays occupy one 2-byte slot per element. A variable also
//! remembers its value/element [`Width`], whether it is a struct (and which), whether
//! it is a pointer receiver (`self`), and whether it is a prelude handle (`Frame`).

use crate::ir::Width;
use std::collections::HashMap;

struct VarInfo {
    base: usize,
    sty: Option<String>,
    ty: Width,    // scalar value type, or array element type
    is_ptr: bool, // a pointer to a struct (e.g. `self`) vs a by-value struct local
    /// A prelude handle type (`"Frame"`/`"Input"`) — methods route to intrinsics.
    handle: Option<String>,
    /// For a struct-element array (`[Cell; N]`), the element struct's name — so element
    /// access (`a[i].x`) knows the element stride + field layout.
    elem_struct: Option<String>,
    /// A read-only pointer to **packed** element data (`t: &[u8; N]` parameter, or
    /// `let t = &CONST;`): the slot holds a byte address; `t[i]` loads through it at
    /// `(element width, byte stride)`. Distinct from a local array (slot-per-element).
    elem_ptr: Option<(Width, u16)>,
    /// A `&str` parameter: the slot holds the address of a length-prefixed buffer
    /// (little-endian u16 length at `s`, bytes at `s + 2` — the Phase S wire format).
    /// Reads route through the string methods (`s.len()`, `s.as_bytes()[i]`, …),
    /// never direct indexing.
    is_str: bool,
    /// A local `[u32; N]` array: two slots per element (`ty` is `DWord`). Element
    /// access goes wide (`Deref32`/`Store32` at `&base + i*4`); the bare name is
    /// not a value.
    wide_array: bool,
}

/// Name → variable info. `next` is the next free slot.
#[derive(Default)]
pub(crate) struct Vars {
    map: HashMap<String, VarInfo>,
    pub(crate) next: usize,
}

impl Vars {
    pub(crate) fn declare(
        &mut self,
        name: &str,
        size: usize,
        sty: Option<String>,
        ty: Width,
    ) -> usize {
        let base = self.next;
        self.map.insert(
            name.to_string(),
            VarInfo {
                base,
                sty,
                ty,
                is_ptr: false,
                handle: None,
                elem_struct: None,
                elem_ptr: None,
                is_str: false,
                wide_array: false,
            },
        );
        self.next += size;
        base
    }
    /// Declare a pointer-to-struct local (one slot holding an address) — `self`.
    pub(crate) fn declare_ptr(&mut self, name: &str, sty: &str) -> usize {
        let base = self.next;
        self.map.insert(
            name.to_string(),
            VarInfo {
                base,
                sty: Some(sty.to_string()),
                ty: Width::Word,
                is_ptr: true,
                handle: None,
                elem_struct: None,
                elem_ptr: None,
                is_str: false,
                wide_array: false,
            },
        );
        self.next += 1;
        base
    }
    /// Declare a prelude-handle param (`frame: &mut Frame`, `input: &Input`).
    pub(crate) fn declare_handle(&mut self, name: &str, handle: &str) -> usize {
        let base = self.next;
        self.map.insert(
            name.to_string(),
            VarInfo {
                base,
                sty: None,
                ty: Width::Word,
                is_ptr: false,
                handle: Some(handle.to_string()),
                elem_struct: None,
                elem_ptr: None,
                is_str: false,
                wide_array: false,
            },
        );
        self.next += 1;
        base
    }
    pub(crate) fn handle_of(&self, name: &str) -> Option<String> {
        self.map.get(name).and_then(|v| v.handle.clone())
    }
    /// Is `name` a declared variable? (A declared local shadows a program const;
    /// an undeclared name falls through to the const table before auto-declaring.)
    pub(crate) fn is_declared(&self, name: &str) -> bool {
        self.map.contains_key(name)
    }
    pub(crate) fn base(&mut self, name: &str) -> usize {
        match self.map.get(name) {
            Some(v) => v.base,
            None => self.declare(name, 1, None, Width::Word),
        }
    }
    /// A struct-typed var as a method receiver: `(base, struct name, is_ptr)`.
    pub(crate) fn receiver(&self, name: &str) -> Option<(usize, String, bool)> {
        self.map
            .get(name)
            .and_then(|v| v.sty.as_ref().map(|s| (v.base, s.clone(), v.is_ptr)))
    }
    /// The variable's value type (scalar) or element type (array).
    pub(crate) fn ty(&self, name: &str) -> Width {
        self.map.get(name).map(|v| v.ty).unwrap_or(Width::Word)
    }
    /// Declare a struct-element array (`[Cell; N]`): `slots` total slots, remembering
    /// the element struct so `a[i].field` can compute the element address.
    pub(crate) fn declare_struct_array(
        &mut self,
        name: &str,
        slots: usize,
        elem_struct: &str,
    ) -> usize {
        let base = self.next;
        self.map.insert(
            name.to_string(),
            VarInfo {
                base,
                sty: None,
                ty: Width::Word,
                is_ptr: false,
                handle: None,
                elem_struct: Some(elem_struct.to_string()),
                elem_ptr: None,
                is_str: false,
                wide_array: false,
            },
        );
        self.next += slots;
        base
    }
    /// The element struct of a struct-element array var, if it is one.
    pub(crate) fn elem_struct(&self, name: &str) -> Option<String> {
        self.map.get(name).and_then(|v| v.elem_struct.clone())
    }
    /// Declare a read-only element pointer (`&[T; N]` param / `let t = &CONST;`):
    /// one slot holding a byte address into packed data.
    pub(crate) fn declare_elem_ptr(&mut self, name: &str, elem: Width, stride: u16) -> usize {
        let base = self.next;
        self.map.insert(
            name.to_string(),
            VarInfo {
                base,
                sty: None,
                ty: Width::Word,
                is_ptr: false,
                handle: None,
                elem_struct: None,
                elem_ptr: Some((elem, stride)),
                is_str: false,
                wide_array: false,
            },
        );
        self.next += 1;
        base
    }
    /// The `(element width, byte stride)` of an element-pointer var, if it is one.
    pub(crate) fn elem_ptr(&self, name: &str) -> Option<(Width, u16)> {
        self.map.get(name).and_then(|v| v.elem_ptr)
    }
    /// Declare a `&str` parameter: one slot holding the address of a
    /// length-prefixed buffer (u16 LE length at `s`, bytes at `s + 2`).
    pub(crate) fn declare_str(&mut self, name: &str) -> usize {
        let base = self.next;
        self.map.insert(
            name.to_string(),
            VarInfo {
                base,
                sty: None,
                ty: Width::Word,
                is_ptr: false,
                handle: None,
                elem_struct: None,
                elem_ptr: None,
                is_str: true,
                wide_array: false,
            },
        );
        self.next += 1;
        base
    }
    /// Is `name` a `&str` parameter?
    pub(crate) fn str_param(&self, name: &str) -> bool {
        self.map.get(name).is_some_and(|v| v.is_str)
    }
    /// Declare a local `[u32; N]` array: `2 * n` slots, wide element access.
    pub(crate) fn declare_wide_array(&mut self, name: &str, n: usize) -> usize {
        let base = self.declare(name, 2 * n, None, Width::DWord);
        self.map.get_mut(name).expect("just declared").wide_array = true;
        base
    }
    /// Is `name` a local `[u32; N]` array?
    pub(crate) fn wide_array(&self, name: &str) -> bool {
        self.map.get(name).is_some_and(|v| v.wide_array)
    }
}
