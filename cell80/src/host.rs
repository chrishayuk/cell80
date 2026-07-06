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
pub(crate) struct Loaded {
    pub(crate) runner: Runner,
    pub(crate) id: String,
    pub(crate) entry: String,
    pub(crate) state_addrs: Vec<(String, u16, Ty)>,
}

/// What a fact-aware behavioural route did — the ranking plus the provenance split
/// (docs/12 §2) of its probe runs. From [`CellHost::route_by_examples_facts`].
#[derive(Debug, Clone, PartialEq)]
pub struct RouteReport {
    /// `(examples reproduced, cell id)`, best first (ties by id), positive scores only —
    /// the [`route_by_examples`](CellHost::route_by_examples) ranking contract.
    pub ranked: Vec<(usize, String)>,
    /// Total probe runs: cells routed × examples (`from_facts + local`).
    pub probe_runs: u64,
    /// Probe runs answered from **imported facts** — claims served without execution.
    pub from_facts: u64,
    /// Probe runs computed locally: executed on the VM, or a repeated example served
    /// by an entry this route itself just computed.
    pub local: u64,
}

/// A persistent host over a library of cells: discover (`search`/`manifest`), then
/// `load` → `run` many → `unload`, keeping runners warm between calls.
#[derive(Default)]
pub struct CellHost {
    pub(crate) catalog: HashMap<String, Cartridge>,
    /// Lazily-(re)built TF-IDF search index over the catalog. `None` means stale; the next
    /// `search_scored` rebuilds it from the catalog's manifests, and `add` invalidates it.
    /// TF-IDF fits IDF over the *whole* corpus (unlike [`CellIndex`]'s incremental `add`), so
    /// we cache-and-rebuild rather than update in place — O(n) per rebuild is cheap at
    /// library scale, and a warm host is typically filled once at startup then served from.
    /// Kept separate from [`type_led`](Self::type_led) deliberately: `search_scored`'s raw
    /// cosine magnitude feeds `cell-eval`'s calibrated tiered-retrieval margin gate — it must
    /// stay exactly plain tf-idf, never re-ranked, or that calibration silently drifts.
    index: std::cell::RefCell<Option<TfidfIndex>>,
    /// Lazily-(re)built **type-led** index (roadmap #3) over the catalog — plain tf-idf
    /// re-ranked by each cell's behavioural predicate/transformer signal (`docs/
    /// library-growth.md`; `TypeLedIndex`'s own module doc has the measured honest lift:
    /// small, ~1-3 points on paraphrase/adversarial, not a fix for the paraphrase ceiling).
    /// Built from cartridges, not just manifests — the predicate label comes from *running*
    /// each cell on a probe bank. Powers plain [`search`](Self::search) only;
    /// [`search_scored`](Self::search_scored)'s calibrated magnitude is untouched.
    type_led: std::cell::RefCell<Option<TypeLedIndex>>,
    pool: CellPool,
    pub(crate) live: Vec<Option<Loaded>>,
    /// When set, every [`load`](Self::load) enables the runner's memoization cache —
    /// repeated `run_fast` calls with the same args become hash lookups (roadmap 3.3).
    cache_loads: bool,
    /// Imported facts staged by artifact hash (docs/12 §3): stamped into a runner's
    /// cache at [`load`](Self::load) (and immediately into already-loaded handles at
    /// import time), so a warm host serves them without re-execution.
    pub(crate) imported: HashMap<[u8; 32], Vec<Fact>>,
}

impl CellHost {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a cartridge in the catalog (keyed by its manifest id) and invalidate both
    /// search indexes, rebuilt from the catalog on the next [`search`](Self::search) /
    /// [`search_scored`](Self::search_scored).
    pub fn add(&mut self, cart: Cartridge) {
        *self.index.borrow_mut() = None;
        *self.type_led.borrow_mut() = None;
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
    /// Rebuilds the type-led index from the catalog if it went stale since the last search
    /// (deterministic: ranking is by score then id, so catalog iteration order can't leak
    /// in). Text-ranked (tf-idf), then re-ranked by each cell's behavioural predicate signal
    /// — a same-shape confusable pair like `range_check`/`clamp` shares every bounds word,
    /// so behaviour breaks the tie where text alone can't (`TypeLedIndex`'s module doc has
    /// the measured lift — modest, not a fix for the paraphrase ceiling).
    ///
    /// **Width routing** (2026-07-07): when the query expresses *width intent*
    /// ("wide", "u32", "32-bit", "65535", or the exact words "large"/"big"/"huge" —
    /// not superlatives like "largest", which are about values), wide cells
    /// (`u32`/`wide`-tagged or `*_u32`) stably move ahead of their u16 siblings.
    /// Every `_u32` slice dilutes the IDF of the width words, so text alone stops
    /// separating `min_u32` from `min` on "the smaller of two large numbers" — the
    /// measured retrieval-curve cost (`cell-eval/baselines/
    /// retrieval-direct-misses-263cells-2026-07-06.txt`). Routing is order-only:
    /// no score is rescaled, and width-neutral queries are untouched.
    pub fn search(&self, query: &str, limit: usize) -> Vec<&Manifest> {
        let stale = self.type_led.borrow().is_none();
        if stale {
            let carts: Vec<Cartridge> = self.catalog.values().cloned().collect();
            *self.type_led.borrow_mut() = Some(TypeLedIndex::build(carts));
        }
        let wide_intent = query
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
            .any(|t| {
                matches!(
                    t.to_ascii_lowercase().as_str(),
                    "wide" | "u32" | "32-bit" | "65535" | "large" | "big" | "huge"
                )
            });
        // Over-fetch when routing so a wide sibling just below the cut can surface.
        let fetch = if wide_intent { limit * 2 } else { limit };
        // Pull the ranked ids out (dropping the borrow), then resolve them to manifests that
        // borrow from `catalog` — so the returned references live as long as `&self`.
        let ids: Vec<String> = self
            .type_led
            .borrow()
            .as_ref()
            .expect("index built above")
            .search(query, fetch)
            .into_iter()
            .map(|m| m.id.clone())
            .collect();
        let mut hits: Vec<&Manifest> = ids
            .into_iter()
            .filter_map(|id| self.catalog.get(&id).map(|c| &c.manifest))
            .collect();
        if wide_intent {
            let is_wide = |m: &Manifest| {
                m.id.ends_with("_u32") || m.tags.iter().any(|t| t == "u32" || t == "wide")
            };
            // Stable partition: wide cells first, relative order preserved on both sides.
            let (wide, narrow): (Vec<&Manifest>, Vec<&Manifest>) =
                hits.into_iter().partition(|m| is_wide(m));
            hits = wide.into_iter().chain(narrow).collect();
        }
        hits.truncate(limit);
        hits
    }

    /// Like [`search`](Self::search) but keeping each hit's tf-idf cosine — the margin
    /// between top-1 and top-2 is the confidence signal a tiered retriever gates on
    /// (small margin → escalate to the next rung instead of answering).
    pub fn search_scored(&self, query: &str, limit: usize) -> Vec<(f32, &Manifest)> {
        let stale = self.index.borrow().is_none();
        if stale {
            let manifests: Vec<Manifest> =
                self.catalog.values().map(|c| c.manifest.clone()).collect();
            *self.index.borrow_mut() = Some(TfidfIndex::build(manifests));
        }
        let hits: Vec<(f32, String)> = self
            .index
            .borrow()
            .as_ref()
            .expect("index built above")
            .scored(query, limit)
            .into_iter()
            .map(|(s, m)| (s, m.id.clone()))
            .collect();
        hits.into_iter()
            .filter_map(|(s, id)| self.catalog.get(&id).map(|c| (s, &c.manifest)))
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

    /// [`route_by_examples`](Self::route_by_examples) riding the fact library (docs/12):
    /// the same ranking contract (reproduced-example count, best first, ties by id,
    /// positive only), but each candidate is **loaded** — so staged imported facts stamp
    /// into its memo cache and answer probe runs as hash lookups instead of executions.
    /// The report carries the provenance split: probe runs answered from imported facts
    /// vs computed locally (executed, or a repeated example served by this route's own
    /// fresh cache entry). With no facts imported — or the cache off — every run is
    /// local and the ranking is identical to the execute-everything path.
    pub fn route_by_examples_facts(
        &mut self,
        examples: &[(Vec<u16>, u16)],
        limit: usize,
    ) -> Result<RouteReport, String> {
        let mut ids: Vec<String> = self.catalog.keys().cloned().collect();
        ids.sort();
        let mut ranked: Vec<(usize, String)> = Vec::new();
        let (mut from_facts, mut local) = (0u64, 0u64);
        for id in ids {
            let h = self.load(&id)?;
            // A pool reuse resets the cache and its counters (`Runner::reset_for`),
            // so this handle's imported-hit count is exactly this route's.
            let mut hits = 0usize;
            for (inputs, want) in examples {
                if matches!(
                    self.run_fast(h, inputs, DEFAULT_CYCLES),
                    Ok(f) if f.halt == Halt::Returned && f.result == *want
                ) {
                    hits += 1;
                }
            }
            let imported = self
                .cache_split(h)?
                .map(|(_, imported)| imported)
                .unwrap_or(0);
            from_facts += imported;
            local += examples.len() as u64 - imported;
            self.unload(h)?;
            if hits > 0 {
                ranked.push((hits, id));
            }
        }
        ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        ranked.truncate(limit);
        Ok(RouteReport {
            probe_runs: from_facts + local,
            ranked,
            from_facts,
            local,
        })
    }

    /// [`route_by_examples`](Self::route_by_examples) for **state cells**: each
    /// example is named fields in → expected `result` out — the structured form
    /// `Struct::run` cells (and the campaign's compiled plans) need, since
    /// register probes can't drive named state.
    pub fn route_by_field_examples(
        &self,
        examples: &[(Vec<(String, u64)>, u16)],
        limit: usize,
    ) -> Vec<&Manifest> {
        let mut hits = crate::fingerprint::rank_field_examples_iter(
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
        let mut runner = self.pool.acquire(&cart.program);
        // Facts key on the shareable content address, not the bare image self-hash
        // (docs/12 §2) — stamp the cartridge's v5 artifact hash.
        runner.set_artifact_hash(cart.artifact_hash());
        if self.cache_loads {
            runner.enable_cache();
        }
        // Stamp any staged imported facts (a fresh runner has no conflicting entries;
        // a fact that doesn't resolve — renamed entry, changed layout — can't happen
        // under the same hash, but is skipped defensively rather than failing a load).
        if let Some(facts) = self.imported.get(&cart.artifact_hash()) {
            if self.cache_loads {
                for f in facts {
                    let _ = runner.insert_fact(f, &cart.manifest.state_addrs);
                }
            }
        }
        let loaded = Loaded {
            runner,
            id: id.to_string(),
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

    /// The live handle for `id`, loading a warm runner only if none exists — the
    /// "one warm runner per schema" shape the solve loop's residue depends on
    /// (an unloaded runner returns to the pool and its memo table with it).
    pub fn handle_for(&mut self, id: &str) -> Result<usize, String> {
        if let Some(h) = self
            .live
            .iter()
            .position(|l| l.as_ref().is_some_and(|l| l.id == id))
        {
            return Ok(h);
        }
        self.load(id)
    }

    pub(crate) fn loaded(&mut self, handle: usize) -> Result<&mut Loaded, String> {
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

    /// The **cached** state-cell hot path (docs/12 §2): [`run_state`](Self::run_state)'s
    /// field-name surface over [`Runner::run_state_fast`] — named inputs in, the
    /// [`Fast`] outcome plus every scalar field back, memoized when the cache is on.
    /// The scoring workhorses ("score 10K candidates through `weighted_sum_wide`")
    /// are state cells; this is the path that makes their repeats hash lookups.
    pub fn run_state_fast(
        &mut self,
        handle: usize,
        fields: &[(String, u64)],
        budget: u64,
    ) -> Result<(Fast, Vec<(String, u64)>), String> {
        let l = self.loaded(handle)?;
        if l.state_addrs.is_empty() {
            return Err("cell has no named state (not a state cell)".into());
        }
        let entry = l.entry.clone();
        let addrs = l.state_addrs.clone();
        let mut inputs = Vec::with_capacity(fields.len());
        for (name, val) in fields {
            let (addr, ty) = addrs
                .iter()
                .find(|(n, _, _)| n == name)
                .map(|(_, a, t)| (*a, *t))
                .ok_or_else(|| format!("no state field `{name}`"))?;
            inputs.push((addr, ty, *val));
        }
        let reads: Vec<(String, u16, Ty)> =
            addrs.iter().map(|(n, a, t)| (n.clone(), *a, *t)).collect();
        l.runner
            .run_state_fast(Some(&entry), &inputs, &reads, budget)
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

    /// Enable (or disable) memoization for cells loaded **from now on** — see
    /// [`Runner::enable_cache`] for what is (and safely isn't) cached. Already-loaded
    /// cells keep their current setting.
    pub fn set_cache(&mut self, on: bool) {
        self.cache_loads = on;
    }

    /// `(hits, lookups)` of a loaded cell's memoization cache — `None` if caching wasn't
    /// enabled when it was loaded.
    pub fn cache_stats(&mut self, handle: usize) -> Result<Option<(u64, u64)>, String> {
        Ok(self.loaded(handle)?.runner.cache_stats())
    }

    /// The provenance split of a loaded cell's cache hits: `(local, imported)` —
    /// the Act-3 number (docs/12 §2).
    pub fn cache_split(&mut self, handle: usize) -> Result<Option<(u64, u64)>, String> {
        Ok(self.loaded(handle)?.runner.cache_split())
    }
}
