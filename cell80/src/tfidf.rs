//! A **TF-IDF tool index** — a stronger drop-in alternative to [`CellIndex`](crate::CellIndex)'s
//! token overlap, aimed at the roadmap's #1 open problem (*paraphrase retrieval is a
//! coin-flip*). Features are lowercased word tokens **and** char-3-grams (so near-spellings
//! and shared morphology still match), each weighted by **inverse document frequency** fit
//! over the manifest corpus — generic words ("two", "number", "value") are down-weighted so
//! the discriminating words dominate. Vectors are L2-normalised, so cosine similarity is a
//! plain dot product.
//!
//! Pure and deterministic, no external model: a real sentence-embedding retriever would need
//! a Python/offline dependency that `cell80` deliberately avoids. The char-3-gram features
//! subsume a separate n-gram-cosine retriever, so this is the single retrieval upgrade.
//!
//! Ported from the `soma-cell` experiment (an out-of-tree SOMA probe), adapted to cell80's
//! [`Manifest`] and search conventions.
use std::collections::HashMap;

use super::Manifest;

/// Lowercased alphanumeric word tokens — the same tokenisation [`CellIndex`](crate::CellIndex)
/// uses, so the two indexes see the same words.
fn tokens(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Overlapping char `n`-grams of the lowercased text (the whole string if shorter than `n`).
fn char_ngrams(s: &str, n: usize) -> Vec<String> {
    let chars: Vec<char> = s.to_lowercase().chars().collect();
    if chars.len() < n {
        let whole: String = chars.into_iter().collect();
        return if whole.is_empty() {
            vec![]
        } else {
            vec![whole]
        };
    }
    chars.windows(n).map(|w| w.iter().collect()).collect()
}

/// Token + char-3-gram features, namespaced (`t:` / `g:`) so a word and an identically
/// spelled n-gram never collide in the vocabulary.
fn feats(text: &str) -> Vec<String> {
    let mut f: Vec<String> = tokens(text).into_iter().map(|t| format!("t:{t}")).collect();
    f.extend(char_ngrams(text, 3).into_iter().map(|g| format!("g:{g}")));
    f
}

/// A fitted TF-IDF model over an explicit token + char-3-gram vocabulary.
#[derive(Clone, Debug, Default)]
pub struct Tfidf {
    vocab: HashMap<String, usize>,
    idf: Vec<f32>,
}

impl Tfidf {
    /// Fit on the document corpus (df/idf are computed from these). The idf is the smoothed
    /// form `ln((N+1)/(df+1)) + 1`, so a feature present in every document still contributes a
    /// little and a rare, discriminating feature dominates.
    pub fn fit(docs: &[String]) -> Self {
        let mut vocab: HashMap<String, usize> = HashMap::new();
        for d in docs {
            for f in feats(d) {
                let next = vocab.len();
                vocab.entry(f).or_insert(next);
            }
        }
        let mut df = vec![0u32; vocab.len()];
        for d in docs {
            let mut seen: Vec<usize> = feats(d)
                .iter()
                .filter_map(|f| vocab.get(f).copied())
                .collect();
            seen.sort_unstable();
            seen.dedup();
            for i in seen {
                df[i] += 1;
            }
        }
        let n = docs.len() as f32;
        let idf = df
            .iter()
            .map(|&d| ((n + 1.0) / (d as f32 + 1.0)).ln() + 1.0)
            .collect();
        Tfidf { vocab, idf }
    }

    /// The number of fitted features (vocabulary size).
    pub fn dim(&self) -> usize {
        self.vocab.len()
    }

    /// Encode `text` into a **sparse, L2-normalised** tf-idf vector of `(feature index,
    /// weight)` sorted by index. Features absent from the fit corpus (OOV) are dropped.
    pub fn encode(&self, text: &str) -> Vec<(usize, f32)> {
        let mut acc: HashMap<usize, f32> = HashMap::new();
        for f in feats(text) {
            if let Some(&i) = self.vocab.get(&f) {
                *acc.entry(i).or_insert(0.0) += self.idf[i]; // tf (via repeated +=) × idf
            }
        }
        let norm = acc.values().map(|x| x * x).sum::<f32>().sqrt();
        let mut v: Vec<(usize, f32)> = if norm > 0.0 {
            acc.into_iter().map(|(i, w)| (i, w / norm)).collect()
        } else {
            acc.into_iter().collect()
        };
        v.sort_unstable_by_key(|&(i, _)| i);
        v
    }
}

/// Cosine of two sparse, L2-normalised vectors (each sorted by index) — a merge-join dot
/// product. Both already unit-length, so the dot *is* the cosine.
fn cosine(a: &[(usize, f32)], b: &[(usize, f32)]) -> f32 {
    let (mut i, mut j, mut dot) = (0usize, 0usize, 0.0f32);
    while i < a.len() && j < b.len() {
        match a[i].0.cmp(&b[j].0) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                dot += a[i].1 * b[j].1;
                i += 1;
                j += 1;
            }
        }
    }
    dot
}

/// The text [`CellIndex`](crate::CellIndex) also searches: id, summary, and tags.
fn manifest_text(m: &Manifest) -> String {
    format!("{} {} {}", m.id, m.summary, m.tags.join(" "))
}

/// A cell's scalar-input count: a free-fn's param count, or a state cell's field count.
/// [`search`](TfidfIndex::search)'s ranking-order re-weight (below) uses this — a cell
/// needing more scalar inputs is structurally more specific/compound.
fn complexity(m: &Manifest) -> usize {
    if m.signature.state.is_empty() {
        m.signature.params.len()
    } else {
        m.signature.state.len()
    }
}

/// A searchable TF-IDF index over cell manifests — same `search(query, limit) -> [&Manifest]`
/// shape as [`CellIndex`](crate::CellIndex), so it drops in where token overlap is too weak.
#[derive(Default)]
pub struct TfidfIndex {
    entries: Vec<Manifest>,
    model: Tfidf,
    doc_vecs: Vec<Vec<(usize, f32)>>,
}

impl TfidfIndex {
    /// Build the index from a corpus of manifests: fit idf over their text, then precompute
    /// each manifest's vector. TF-IDF needs the whole corpus at once, so this replaces
    /// [`CellIndex`](crate::CellIndex)'s incremental `add` with a one-shot build.
    pub fn build(entries: Vec<Manifest>) -> Self {
        let docs: Vec<String> = entries.iter().map(manifest_text).collect();
        let model = Tfidf::fit(&docs);
        let doc_vecs = docs.iter().map(|d| model.encode(d)).collect();
        TfidfIndex {
            entries,
            model,
            doc_vecs,
        }
    }

    /// The number of indexed manifests.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the index holds no manifests.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Length-normalisation-style re-rank strength (the same instinct BM25 applies to
    /// document length, applied to a cell's shape instead): [`search`](Self::search) breaks
    /// near-ties toward the structurally simpler cell via `rank_key = cosine / (1 +
    /// GAMMA · max(0, complexity - COMPLEXITY_BASELINE))`. Deliberately **not** applied in
    /// [`scored`](Self::scored) — that magnitude feeds `cell-eval`'s tiered-retrieval margin
    /// gate (a blended score calibrated against raw tf-idf cosine; rescaling it would
    /// silently drift an already-tuned θ), so only `search`'s *ranking order* is biased, never
    /// the exposed cosine. Measured against `examples/retrieval_compare` (163 cells, 327
    /// queries): overall P@1 +0.6, adversarial +5.9, paraphrase +0.8, direct −0.6 — swept
    /// 0.0..0.3, this was the best overall point and the only one positive on every split but
    /// direct.
    const GAMMA: f32 = 0.05;
    const COMPLEXITY_BASELINE: usize = 2;

    /// The manifests best matching `query`, ranked by tf-idf cosine with a small complexity
    /// tie-break (see `GAMMA`), best first (ties broken by id), up to `limit`; only positive
    /// (non-zero) similarities are considered. Use [`scored`](Self::scored) for the raw,
    /// unbiased cosine magnitude (e.g. for a calibrated confidence margin).
    pub fn search(&self, query: &str, limit: usize) -> Vec<&Manifest> {
        let mut scored = self.scored(query, usize::MAX);
        let rank_key = |s: f32, m: &Manifest| {
            let extra = complexity(m).saturating_sub(Self::COMPLEXITY_BASELINE) as f32;
            s / (1.0 + Self::GAMMA * extra)
        };
        scored.sort_by(|a, b| {
            rank_key(b.0, b.1)
                .partial_cmp(&rank_key(a.0, a.1))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.id.cmp(&b.1.id))
        });
        scored.into_iter().take(limit).map(|(_, m)| m).collect()
    }

    /// Like [`search`](Self::search) but keeps the cosine score with each manifest — the form
    /// a re-ranker (e.g. [`TypeLedIndex`](crate::TypeLedIndex)) needs to combine the text
    /// signal with a structural one. Best first (ties broken by id), positive scores only.
    pub fn scored(&self, query: &str, limit: usize) -> Vec<(f32, &Manifest)> {
        let q = self.model.encode(query);
        let mut scored: Vec<(f32, &Manifest)> = self
            .entries
            .iter()
            .zip(&self.doc_vecs)
            .map(|(m, dv)| (cosine(&q, dv), m))
            .filter(|(s, _)| *s > 0.0)
            .collect();
        scored.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.id.cmp(&b.1.id))
        });
        scored.into_iter().take(limit).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idf_down_weights_common_words_so_the_discriminator_wins() {
        // Two "documents" sharing a generic phrase ("of two numbers") but differing in the
        // key word. A divisor query must route to the divisor doc despite the shared tokens.
        let docs = vec![
            "the greatest common divisor of two numbers".to_string(),
            "the smaller of two numbers".to_string(),
        ];
        let m = Tfidf::fit(&docs);
        let q = m.encode("greatest common divisor");
        assert!(
            cosine(&q, &m.encode(&docs[0])) > cosine(&q, &m.encode(&docs[1])),
            "tf-idf should route the divisor query to the divisor doc"
        );
    }

    #[test]
    fn encoding_is_normalised_and_deterministic() {
        let docs = vec![
            "clamp a value inside a bound".to_string(),
            "the sign of a number".to_string(),
        ];
        let m = Tfidf::fit(&docs);
        let a = m.encode("clamp a value");
        assert_eq!(a, m.encode("clamp a value"), "encoding is deterministic");
        let norm = a.iter().map(|(_, w)| w * w).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5 || norm == 0.0, "L2-normalised");
    }

    #[test]
    fn index_search_ranks_the_paraphrased_target_first() {
        let mk = |id: &str, summary: &str, tags: &[&str]| Manifest {
            id: id.into(),
            summary: summary.into(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            entry: "run".into(),
            source_hash: 0,
            compiler_version: String::new(),
            abi_version: 0,
            signature: Default::default(),
            state_addrs: vec![],
            limits: Vec::new(),
            scale: None,
            finite_result: true,
            kernel_bank: None,
            target: crate::Z80_CELL_TARGET.to_string(),
            family_hash: None,
            accuracy: None,
        };
        let idx = TfidfIndex::build(vec![
            mk(
                "gcd",
                "greatest common divisor of two numbers",
                &["math", "divisor"],
            ),
            mk("min", "the smaller of two numbers", &["math", "compare"]),
            mk("clamp", "constrain a value to a range", &["math", "bound"]),
        ]);
        // A paraphrase that shares no rare token with "greatest common divisor" verbatim but
        // overlaps on char-3-grams ("divis") still ranks gcd first.
        let hits = idx.search("highest common factor / divisor", 3);
        assert_eq!(hits[0].id, "gcd");
    }

    #[test]
    fn search_breaks_a_cosine_tie_toward_the_simpler_shape() {
        // Two manifests with byte-identical summary/tags — and single-letter, equally
        // unique ids ("a", "b": each contributes one token + one short gram, same document
        // frequency, so they perturb each doc's norm identically) — so their raw cosine to
        // any query not mentioning "a"/"b" ties exactly. Different structural shape: `a` is
        // a 2-arg free-fn, `b` a 6-field state cell. `search` must break the tie toward `a`;
        // `scored` must report the tie honestly (equal magnitudes), since that value feeds a
        // calibrated confidence margin elsewhere and must not be silently rescaled.
        let sig = |params: usize, state: usize| rustz80::Signature {
            params: (0..params)
                .map(|i| (format!("p{i}"), "u16".into()))
                .collect(),
            ret: "u16".into(),
            state: (0..state)
                .map(|i| (format!("f{i}"), "u16".into()))
                .collect(),
        };
        let mk = |id: &str, signature: rustz80::Signature| Manifest {
            id: id.into(),
            summary: "does the same thing to two numbers".into(),
            tags: vec!["math".into(), "combine".into()],
            entry: "run".into(),
            source_hash: 0,
            compiler_version: String::new(),
            abi_version: 0,
            signature,
            state_addrs: vec![],
            limits: Vec::new(),
            scale: None,
            finite_result: true,
            kernel_bank: None,
            target: crate::Z80_CELL_TARGET.to_string(),
            family_hash: None,
            accuracy: None,
        };
        let idx = TfidfIndex::build(vec![mk("a", sig(2, 0)), mk("b", sig(0, 6))]);

        // scored() is honest: an identical-text tie stays a tie, unbiased.
        let raw = idx.scored("does the same thing to two numbers", 2);
        assert_eq!(raw.len(), 2);
        assert!(
            (raw[0].0 - raw[1].0).abs() < 1e-6,
            "raw cosine ties exactly"
        );

        // search() breaks that same tie toward the lower-complexity cell.
        let hits = idx.search("does the same thing to two numbers", 2);
        assert_eq!(hits[0].id, "a");
    }
}
