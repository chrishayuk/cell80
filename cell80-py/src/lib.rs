//! PyO3 bindings: expose the `rustz80-cell` **host** to Python as a warm `CellHost` class —
//! the persistent, in-process session an MCP server (`cell80-mcp`) drives. The Rust side
//! compiles, runs, and caches warm runners; it returns plain dicts/ints/strings and leaves
//! all MCP-shaping (tool schemas, content blocks) to the Python layer. Mirrors `zxspec_py`:
//! a standalone crate (own workspace, cdylib), built with maturin.
//
// pyo3's `?` ergonomics convert `PyErr -> PyErr` at each `?`, which clippy flags as a
// useless conversion (the sibling `zxspec_py` carries the same noise); silence it here.
#![allow(clippy::useless_conversion)]
use cell80::{Cartridge, CartridgeOpts, CellConfig, CellHost as RsHost, Halt, Manifest};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

/// A stable lowercase tag for a halt reason (the Python layer surfaces it verbatim).
fn halt_str(h: Halt) -> &'static str {
    match h {
        Halt::Returned => "returned",
        Halt::Halted(_) => "halted",
        Halt::CycleBudget => "cycle_budget",
        Halt::MemoryLimit => "memory_limit",
    }
}

/// A brief manifest (what `search` returns — enough to choose a cell).
fn brief<'py>(py: Python<'py>, m: &Manifest) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("id", &m.id)?;
    d.set_item("summary", &m.summary)?;
    d.set_item("tags", m.tags.clone())?;
    d.set_item("signature", m.signature.to_decl(&m.entry))?;
    Ok(d)
}

/// A full manifest (what `inspect` returns — the typed interface + provenance).
fn full<'py>(py: Python<'py>, m: &Manifest) -> PyResult<Bound<'py, PyDict>> {
    let d = brief(py, m)?;
    d.set_item("entry", &m.entry)?;
    d.set_item("abi", m.abi_version)?;
    d.set_item("params", m.signature.params.clone())?;
    d.set_item("ret", &m.signature.ret)?;
    d.set_item("state", m.signature.state.clone())?;
    d.set_item("source_hash", format!("0x{:016x}", m.source_hash))?;
    Ok(d)
}

/// A warm, persistent cell host: register cells, `search`, then `load` → `run` many →
/// `unload`, keeping runners warm across calls (the warm-path a per-invocation CLI can't).
#[pyclass]
struct CellHost {
    host: RsHost,
}

#[pymethods]
impl CellHost {
    #[new]
    fn new() -> Self {
        Self {
            host: RsHost::new(),
        }
    }

    /// Compile a dialect `.rs` source into the catalog. `entry` defaults to `run`/`main`.
    #[pyo3(signature = (id, src, summary="", tags=Vec::new(), entry=None))]
    fn add_source(
        &mut self,
        id: &str,
        src: &str,
        summary: &str,
        tags: Vec<String>,
        entry: Option<String>,
    ) -> PyResult<()> {
        let cart = Cartridge::compile(
            src,
            CellConfig::sandboxed(),
            CartridgeOpts {
                id: Some(id.to_string()),
                summary: summary.to_string(),
                tags,
                entry,
            },
        )
        .map_err(PyValueError::new_err)?;
        self.host.add(cart);
        Ok(())
    }

    /// Register a precompiled `.cell` cartridge (its bytes) into the catalog.
    fn add_cell(&mut self, data: &[u8]) -> PyResult<()> {
        let cart = Cartridge::from_bytes(data).map_err(PyValueError::new_err)?;
        self.host.add(cart);
        Ok(())
    }

    /// How many cells are in the catalog.
    fn __len__(&self) -> usize {
        self.host.len()
    }
    /// How many cells are currently loaded (warm).
    fn live_count(&self) -> usize {
        self.host.live_count()
    }

    /// Rank the catalog by relevance to `query`; returns brief manifests, best first.
    #[pyo3(signature = (query, limit=10))]
    fn search<'py>(
        &self,
        py: Python<'py>,
        query: &str,
        limit: usize,
    ) -> PyResult<Bound<'py, PyList>> {
        let list = PyList::empty_bound(py);
        for m in self.host.search(query, limit) {
            list.append(brief(py, m)?)?;
        }
        Ok(list)
    }

    /// Full manifest for `id` (typed signature, abi, hash) — or `None`.
    fn manifest<'py>(&self, py: Python<'py>, id: &str) -> PyResult<Option<Bound<'py, PyDict>>> {
        self.host.manifest(id).map(|m| full(py, m)).transpose()
    }

    /// Load `id` → a warm handle (cheap to `run` repeatedly).
    fn load(&mut self, id: &str) -> PyResult<usize> {
        self.host.load(id).map_err(PyValueError::new_err)
    }

    /// Run a loaded cell with register args; returns
    /// `{result, regs, cycles, trapped_ops, halt}`.
    #[pyo3(signature = (handle, args=Vec::new(), cycles=2_000_000))]
    fn run<'py>(
        &mut self,
        py: Python<'py>,
        handle: usize,
        args: Vec<u16>,
        cycles: u64,
    ) -> PyResult<Bound<'py, PyDict>> {
        let f = self
            .host
            .run_fast(handle, &args, cycles)
            .map_err(PyValueError::new_err)?;
        let d = PyDict::new_bound(py);
        d.set_item("result", f.result)?;
        d.set_item("regs", vec![f.regs[0], f.regs[1], f.regs[2]])?;
        d.set_item("cycles", f.cycles)?;
        d.set_item("trapped_ops", f.trapped_ops)?;
        d.set_item("halt", halt_str(f.halt))?;
        Ok(d)
    }

    /// Drive a loaded **state cell by field name**: `fields` is `{name: int}`. Returns
    /// `{result, regs, cycles, trapped_ops, halt, state: {name: value}}`, where `state` is the
    /// full post-run struct read back by name. The JSON↔state surface — no raw addresses.
    #[pyo3(signature = (handle, fields, cycles=2_000_000))]
    fn run_state<'py>(
        &mut self,
        py: Python<'py>,
        handle: usize,
        fields: &Bound<'py, PyDict>,
        cycles: u64,
    ) -> PyResult<Bound<'py, PyDict>> {
        let mut named = Vec::with_capacity(fields.len());
        for (k, v) in fields.iter() {
            named.push((k.extract::<String>()?, v.extract::<u64>()?));
        }
        let (rep, state) = self
            .host
            .run_state(handle, &named, cycles)
            .map_err(PyValueError::new_err)?;
        let d = PyDict::new_bound(py);
        d.set_item("result", rep.result)?;
        d.set_item("regs", vec![rep.regs[0], rep.regs[1], rep.regs[2]])?;
        d.set_item("cycles", rep.cycles)?;
        d.set_item("trapped_ops", rep.trapped_ops)?;
        d.set_item("halt", halt_str(rep.halt))?;
        let sd = PyDict::new_bound(py);
        for (name, val) in state {
            sd.set_item(name, val)?;
        }
        d.set_item("state", sd)?;
        Ok(d)
    }

    /// Release a loaded handle (returns its bus to the pool).
    fn unload(&mut self, handle: usize) -> PyResult<()> {
        self.host.unload(handle).map_err(PyValueError::new_err)
    }
}

#[pymodule]
fn cell80_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<CellHost>()?;
    Ok(())
}
