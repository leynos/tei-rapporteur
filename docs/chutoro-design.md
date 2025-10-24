# Chutoro CPU Index Design Notes

_Implementation update (2024-07-02)._ The initial CPU index is now realised in
`CpuHnsw`, which wraps the shared graph in `Arc<RwLock<_>>`. Insertion follows
a strict two-phase protocol: worker threads hold a read lock while performing
the HNSW search, drop it, and then acquire a write lock to apply the insertion
plan. Rayon drives batch construction, seeding the entry point synchronously
before the parallel phase to avoid races. Random level assignment is handled by
a `SmallRng` guarded with a `Mutex`, trading a short critical section for
deterministic tests while preserving the geometric tail induced by the
`1/ln(M)` multiplier. The graph limits neighbour fan-out eagerly, pruning edges
under the write lock using the caller-provided `DataSource` for distance
ordering. Trimming now batches by endpoint: each layer collects the nodes whose
adjacency changed, computes their distance orderings once via
`batch_distances(query, candidates)`, and reapplies the truncated lists,
keeping the write critical section short even when multiple neighbours are
added.
