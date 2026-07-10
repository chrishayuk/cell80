//! The interpreter entries: parse → lower → run on the [`cell80_core::Interp`]
//! engine (the reference IR interpreter, A4). The engine — semantics, memory
//! image, fuel — lives in `cell80-core`; these wrappers own the syn/lowering
//! front half and mirror the `compile_*` entry shapes.

use crate::codegen::Target;
use cell80_core::interp::Interp;

// ── public entries ──────────────────────────────────────────────────────────────

/// Interpret a single `fn` (the `compile_fn` shape): result = the function's value.
pub fn interp_fn(src: &str) -> Result<u16, String> {
    interp_fn_args(src, &[], &[])
}

/// [`interp_fn`] with 16-bit register args and pre-laid data blobs (the `run_str`
/// buffer pattern).
pub fn interp_fn_args(src: &str, args: &[u16], data: &[(u16, &[u8])]) -> Result<u16, String> {
    let item: syn::ItemFn = syn::parse_str(src).map_err(|e| format!("parse error: {e}"))?;
    let name = item.sig.ident.to_string();
    let func = crate::lower::lower(&item)?;
    let funcs = [(name.clone(), func)];
    if let Some(cycle) = crate::dce::find_recursion(&funcs) {
        return Err(format!("recursion is not supported (cycle: {cycle})"));
    }
    let mut it = Interp::new(&funcs, std::iter::empty(), Target::Cell.descriptor());
    for (addr, bytes) in data {
        it.plant(*addr, bytes);
    }
    let out = it.run(&name, args)?;
    Ok(out.first().copied().unwrap_or(0))
}

/// Interpret a multi-`fn` program from `entry` (the `compile_program` shape):
/// result = the entry's result registers (a wide return as `[low, high]`).
pub fn interp_program(src: &str, entry: &str) -> Result<Vec<u16>, String> {
    Ok(interp_program_run(src, entry)?.1)
}

/// [`interp_program`], returning the final 64 KiB memory image for memory-effect
/// comparison (mask the execution substrate the interpreter doesn't have: the
/// compiled code region and the hardware stack).
pub fn interp_program_mem(src: &str, entry: &str) -> Result<Vec<u8>, String> {
    Ok(interp_program_run(src, entry)?.0)
}

fn interp_program_run(src: &str, entry: &str) -> Result<(Vec<u8>, Vec<u16>), String> {
    let file: syn::File = syn::parse_str(src).map_err(|e| format!("parse error: {e}"))?;
    let lowered = crate::lower::lower_program_full(&file, &crate::lower::PreludeConfig::default())?;
    // The same shaping as `compile_file`: inline single-call-site helpers so the
    // frame layout (slot base assignment) matches the compiled program's.
    let funcs = crate::inline::inline(lowered.funcs, &[]);
    let consts = lowered
        .consts
        .data
        .iter()
        .map(|d| (d.name.as_str(), d.bytes.as_slice()));
    let mut it = Interp::new(&funcs, consts, Target::Cell.descriptor());
    let out = it.run(entry, &[])?;
    let mem = std::mem::take(&mut it.mem);
    Ok((mem, out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_arithmetic_and_calls() {
        let v = interp_fn("fn f() -> u16 { let a = 6; let b = 7; a * b + 1 }").unwrap();
        assert_eq!(v, 43);
        let out = interp_program(
            "fn double(x: u16) -> u16 { x * 2 }\nfn main() -> u16 { double(21) }",
            "main",
        )
        .unwrap();
        assert_eq!(out, vec![42]);
    }

    #[test]
    fn args_bind_to_param_slots() {
        let v = interp_fn_args("fn f(a: u16, b: u16) -> u16 { a - b }", &[10, 3], &[]).unwrap();
        assert_eq!(v, 7);
    }

    #[test]
    fn fuel_guard_stops_a_runaway_loop() {
        let e = interp_fn("fn f() -> u16 { let mut x = 1; while x > 0 { x = 1; } x }").unwrap_err();
        assert!(e.contains("fuel"), "unexpected error: {e}");
    }
}
