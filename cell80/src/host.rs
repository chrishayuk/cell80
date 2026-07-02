//! A persistent **cell host** — the long-lived, in-process session layer that keeps the
//! library index + warm runners alive *across* calls. This is what the per-call CLI can't
//! be: a process spawn (~ms) re-pays startup every call and throws away the warm runner,
//! defeating the whole compile-once/run-many advantage. A host holds a catalog of compiled
//! cartridges (searchable via [`CellIndex`]) and a slab of **loaded** runners drawn from a
//! [`CellPool`], so `load` once → `run` many reuses one warm machine (an O(touched) reset
//! between runs, no re-instantiate). Transport-agnostic: an MCP server, a daemon, or a
//! library caller all sit on top of the same `search`/`inspect`/`load`/`run` verbs.
use super::*;
use std::collections::HashMap;

/// A loaded cell: a warm runner plus the entry to invoke on it, and (for a state cell) the
/// `(field, address)` map that lets it be driven by name.
struct Loaded {
    runner: Runner,
    entry: String,
    state_addrs: Vec<(String, u16, Ty)>,
}

/// A persistent host over a library of cells: discover (`search`/`manifest`), then
/// `load` → `run` many → `unload`, keeping runners warm between calls.
#[derive(Default)]
pub struct CellHost {
    catalog: HashMap<String, Cartridge>,
    /// Lazily-(re)built TF-IDF search index over the catalog. `None` means stale; the next
    /// `search` rebuilds it from the catalog's manifests, and `add` invalidates it. TF-IDF
    /// fits IDF over the *whole* corpus (unlike [`CellIndex`]'s incremental `add`), so we
    /// cache-and-rebuild rather than update in place — O(n) per rebuild is cheap at library
    /// scale, and a warm host is typically filled once at startup then served from.
    index: std::cell::RefCell<Option<TfidfIndex>>,
    pool: CellPool,
    live: Vec<Option<Loaded>>,
}

impl CellHost {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a cartridge in the catalog (keyed by its manifest id) and invalidate the
    /// search index, which is rebuilt from the catalog on the next [`search`](Self::search).
    pub fn add(&mut self, cart: Cartridge) {
        *self.index.borrow_mut() = None;
        self.catalog.insert(cart.manifest.id.clone(), cart);
    }

    /// How many cells are in the catalog.
    pub fn len(&self) -> usize {
        self.catalog.len()
    }
    pub fn is_empty(&self) -> bool {
        self.catalog.is_empty()
    }

    /// Discover: rank the catalog by relevance to `query` (returns manifests, not cells).
    /// Rebuilds the TF-IDF index from the catalog if it went stale since the last search
    /// (deterministic: ranking is by cosine then id, so catalog iteration order can't leak in).
    pub fn search(&self, query: &str, limit: usize) -> Vec<&Manifest> {
        let stale = self.index.borrow().is_none();
        if stale {
            let manifests: Vec<Manifest> =
                self.catalog.values().map(|c| c.manifest.clone()).collect();
            *self.index.borrow_mut() = Some(TfidfIndex::build(manifests));
        }
        // Pull the ranked ids out (dropping the borrow), then resolve them to manifests that
        // borrow from `catalog` — so the returned references live as long as `&self`.
        let ids: Vec<String> = self
            .index
            .borrow()
            .as_ref()
            .expect("index built above")
            .search(query, limit)
            .into_iter()
            .map(|m| m.id.clone())
            .collect();
        ids.into_iter()
            .filter_map(|id| self.catalog.get(&id).map(|c| &c.manifest))
            .collect()
    }

    /// Discover **by behaviour**: rank the catalog by how many `(inputs, expected_output)`
    /// examples each cell reproduces on the VM, best first (ties by id), positive only. This is
    /// the phrasing- and language-independent signal text search can't give — it tells `min`
    /// from `max` (on `(3,7)` one returns 3, the other 7) where their manifests are identical.
    /// An empty result means *no cell in the library reproduces these examples*.
    pub fn route_by_examples(&self, examples: &[(Vec<u16>, u16)], limit: usize) -> Vec<&Manifest> {
        let mut hits = crate::fingerprint::rank_examples_iter(
            self.catalog.values(),
            examples,
            crate::DEFAULT_CYCLES,
        );
        hits.truncate(limit);
        hits
    }

    /// Inspect a cell's manifest by id (the typed signature, caps, tags, …).
    pub fn manifest(&self, id: &str) -> Option<&Manifest> {
        self.catalog.get(id).map(|c| &c.manifest)
    }

    /// Instantiate a **warm** runner for `id` (from the pool) and return a handle. Cheap to
    /// `run` repeatedly; release it with [`unload`](Self::unload).
    pub fn load(&mut self, id: &str) -> Result<usize, String> {
        let cart = self
            .catalog
            .get(id)
            .ok_or_else(|| format!("no cell `{id}`"))?;
        let loaded = Loaded {
            runner: self.pool.acquire(&cart.program),
            entry: cart.manifest.entry.clone(),
            state_addrs: cart.manifest.state_addrs.clone(),
        };
        // Reuse a freed handle slot if there is one.
        match self.live.iter().position(Option::is_none) {
            Some(h) => {
                self.live[h] = Some(loaded);
                Ok(h)
            }
            None => {
                self.live.push(Some(loaded));
                Ok(self.live.len() - 1)
            }
        }
    }

    fn loaded(&mut self, handle: usize) -> Result<&mut Loaded, String> {
        self.live
            .get_mut(handle)
            .and_then(Option::as_mut)
            .ok_or_else(|| format!("invalid cell handle {handle}"))
    }

    /// Run a loaded cell with typed `inputs` (and `args` in the convention registers),
    /// returning the rich [`Report`]. Reuses the warm runner — no re-instantiate.
    pub fn run(
        &mut self,
        handle: usize,
        args: &[u16],
        inputs: &[(u16, Ty, u64)],
        budget: u64,
    ) -> Result<Report, String> {
        let l = self.loaded(handle)?;
        let entry = l.entry.clone();
        l.runner.run_with_inputs(Some(&entry), args, inputs, budget)
    }

    /// The hot path: run a loaded cell for just the result registers/cycles/halt.
    pub fn run_fast(&mut self, handle: usize, args: &[u16], budget: u64) -> Result<Fast, String> {
        let l = self.loaded(handle)?;
        let entry = l.entry.clone();
        l.runner.run_fast(Some(&entry), args, budget)
    }

    /// Drive a **state cell by field name**: write the given `fields` into the state struct,
    /// run the entry with `&mut self` at [`STATE_BASE`], and read **every** scalar state field
    /// back. Returns the [`Report`] plus the post-run state as `(name, value)` in declaration
    /// order. This is the JSON↔state surface an agent (or a peer cell in a graph) drives — no
    /// raw addresses. Errors if the cell has no named state or a field name is unknown.
    pub fn run_state(
        &mut self,
        handle: usize,
        fields: &[(String, u64)],
        budget: u64,
    ) -> Result<(Report, Vec<(String, u64)>), String> {
        let l = self.loaded(handle)?;
        if l.state_addrs.is_empty() {
            return Err("cell has no named state (not a state cell)".into());
        }
        let entry = l.entry.clone();
        let addrs = l.state_addrs.clone();
        // Resolve each named input to its address, at the field's own width — a `u32`
        // field takes (and reads back) the full 32-bit value.
        let mut inputs = Vec::with_capacity(fields.len());
        for (name, val) in fields {
            let (addr, ty) = addrs
                .iter()
                .find(|(n, _, _)| n == name)
                .map(|(_, a, t)| (*a, *t))
                .ok_or_else(|| format!("no state field `{name}`"))?;
            inputs.push((addr, ty, *val));
        }
        let report = l
            .runner
            .run_with_inputs(Some(&entry), &[STATE_BASE], &inputs, budget)?;
        // Read all fields back so the caller sees the full post-run state.
        let reads: Vec<(String, u16, Ty)> =
            addrs.iter().map(|(n, a, t)| (n.clone(), *a, *t)).collect();
        let state = l.runner.read_named(&reads);
        Ok((report, state))
    }

    /// Read a named typed field from a loaded cell's post-run memory.
    pub fn read_named(
        &mut self,
        handle: usize,
        fields: &[(String, u16, Ty)],
    ) -> Result<Vec<(String, u64)>, String> {
        Ok(self.loaded(handle)?.runner.read_named(fields))
    }

    /// Release a loaded cell, returning its bus to the pool for reuse.
    pub fn unload(&mut self, handle: usize) -> Result<(), String> {
        let l = self
            .live
            .get_mut(handle)
            .and_then(Option::take)
            .ok_or_else(|| format!("invalid cell handle {handle}"))?;
        self.pool.release(l.runner);
        Ok(())
    }

    /// How many cells are currently loaded (warm).
    pub fn live_count(&self) -> usize {
        self.live.iter().filter(|s| s.is_some()).count()
    }
}
