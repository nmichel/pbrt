# IDEAS

- [x] Add cylinder volume
- [ ] nested transmitter volumes (e.g. glass in water)
- [ ] don't use Arc, but Rc with unsafe wrapping to pass to threads. See this [stackoverflow article](https://stackoverflow.com/questions/63433718/how-to-freeze-an-rc-data-structure-and-send-it-across-threads)
- [x] Make BVH more generic
- [ ] Add cone volume
- [ ] Add a scene from text file loader
- [ ] Add support for triangle based geometry

---

# Code review — 2026-07-28

Findings from a full read of the tree at commit `1859a9e`, on branch
`chore/revamp_bvh_for_trimesh`. Line numbers are from that state and will drift.
Nothing here has been fixed yet.

## Suggested order of work

Rationale: (1) makes every later iteration faster to test, (2) is the largest
departure from a physical model, (3) is a prerequisite to *validating* (2) and (4).

1. Finish the BVH revamp — fix the SAH cost, then port the flat/ordered design to the
   scene BVH and add `intersect_p`.
2. `AreaLight` — emissive primitives registered as sampleable lights.
3. Seedable RNG + stratified samplers (needed to compare two renders at all).
4. MIS, then re-enable Russian roulette.
5. `Film` abstraction + tile-based scheduling.

## Defects

### BVH — mesh (the branch's current subject)

- [x] **SAH cost was computed from the wrong box.** `left_bin.bounds.half_area()` — the area
      of a *single* bin — where the SAH needs the area of the union of all bins on that side
      of the plane. `left_box`/`right_box` were accumulated correctly and then never read.
      *Done.* The prefix/suffix scan now reads the accumulated unions. The derivation is in
      [docs/heuristique_aire_surface.md](docs/heuristique_aire_surface.md), whose §3 carries
      the counter-example that settles it: for eight equally populated bins of growing spread,
      the per-bin cost is not merely imprecise, it is **constant** — it distinguishes nothing,
      ties on all seven planes, and the strict comparison then elects the first, i.e. the
      degenerate one. A `debug_assert!` now guards the invariant the per-bin form lacks: a
      union can only grow, so prefix areas cannot decrease and suffix areas cannot increase.
- [x] **First candidate plane was degenerate.** `bound_min + i * inv_scale` with `i` from 0 put
      the first plane exactly on `bound_min`, left side empty, the `left_count == 0` guard
      returning and subdivision stopping. *Done.* Boundary `i` is at
      `centroid_min + (i + 1) · bin_width`, and `inv_scale` — the misnamed duplicate of
      `scale` that made the wrong formula read as plausible — is gone.
- [x] `evaluate_sah` was dead code, and buggy: `f64::MAX` for a zero cost under a comment about
      a division it does not perform. *Done.* Fixed, renamed `exhaustive_split_cost`, marked
      `#[cfg(test)]`, and used as the oracle of `test_binned_cost_matches_exhaustive_scan` —
      which fails on the per-bin defect, verified by reintroducing it.
- [x] **The partition compared a reconstructed float position** while the cost was derived from
      bin counts, so the two could disagree for a centroid on a boundary and the winning plane
      could be scored on a partition that never happened. *Done.* `SplitCandidate` carries the
      boundary index and the binning parameters rather than a position, and both paths call the
      same `bin_index`. Not in the original review — found while planning the fix.
- [ ] **Each node's box is tested twice.** The traversal tests a child's box before pushing
      it ([bvh.rs:379-380](src/shapes/triangle_mesh/bvh.rs#L379-L380)) and tests the same box
      again after popping it ([bvh.rs:354](src/shapes/triangle_mesh/bvh.rs#L354)). Box tests
      per ray runs at ~2.6× nodes visited per ray. Either push the distance alongside the
      index, or drop the pre-push test and let the pop handle it. **Next up** — deliberately
      kept out of the SAH commit so its before/after stayed attributable.
- [ ] **Leaves hold a single triangle**, so the tree has ~2 nodes per triangle: 1 724 381 nodes
      for `dragon_vrip.ply`. That is the `t_trav = 0` of `[5]`
      ([docs/heuristique_aire_surface.md](docs/heuristique_aire_surface.md) §6) — with
      traversal counted free, splitting always wins. Giving it pbrt's weight of ~1/8 of an
      intersection would shorten the tree and cut the memory, at some cost in triangle tests.
      Worth measuring both ways now that measuring is cheap.
- [ ] **An empty mesh makes `build_stats` recurse into a node that does not exist.** `build`
      leaves a root with `tri_count == 0`, which `is_leaf` reports as an interior node, so a
      walk follows `left_first` into an empty `nodes`. `subdivide` is no longer affected — the
      explicit rejection of empty sides makes it return at once — but the representation
      problem stands. Same family as the `BVHNode::new`-on-empty-vector defect listed under
      *BVH — scene*: emptiness is not represented in `BVHTree` at all.

#### Baseline — 2026-07-30

Instrumentation is in place: `TraversalStats` and `BuildStats` in
[bvh.rs](src/shapes/triangle_mesh/bvh.rs), exposed through
`TriangleMesh::intersect_instrumented` / `build_stats`, driven by
[src/bin/bvh_stats.rs](src/bin/bvh_stats.rs). The ray set is the primary rays of 6 pinhole
cameras on a deterministic orbit, 200×200 each — 240 000 rays, no RNG, reproducible to the
unit.

```
cargo run --release --bin bvh_stats -- test_files/<mesh>.ply
```

| mesh | tris | nodes (leaves) | depth | leaf tris mean / max | nodes/ray | box tests/ray | **tri tests/ray** |
|---|---|---|---|---|---|---|---|
| `cube.ply` | 12 | 1 (1) | 1 | 12.0 / 12 | 1.00 | 1.00 | 5.80 |
| `bun_zipper_res4.ply` | 948 | 3 (2) | 2 | 474.0 / 716 | 1.51 | 2.45 | 254.41 |
| `bunny.ply` | 69 451 | 121 (61) | 14 | 1138.5 / 37 469 | 2.63 | 5.80 | 7464.08 |
| `dragon_vrip_res4.ply` | 11 102 | 351 (176) | 19 | 63.1 / 3143 | 3.20 | 7.36 | 517.28 |
| `dragon_vrip_res3.ply` | 47 794 | 1021 (511) | 22 | 93.5 / 15 949 | 2.98 | 6.68 | 3622.88 |
| `dragon_vrip.ply` | 871 414 | 2301 (1151) | 27 | 757.1 / 316 949 | 2.89 | 6.45 | 66 649.36 |

Hit rate is 18–20 % on the organic meshes, 48 % on the cubes, so the averages are over ray
sets that really do reach the geometry.

**What the numbers say.** The tree barely filters anything: on the full dragon a ray is
charged 66 649 triangle tests, i.e. **7.6 % of the whole mesh**, and a single leaf holds
316 949 of the 871 414 triangles — 36 % of the model. `cube.ply` is not subdivided at all
(1 node). Subdivision stops almost immediately, which is the signature of the degenerate
first candidate plane: `best_pos = bound_min` leaves the left side empty, the
`left_count == 0` guard returns, and the node stays a leaf. The two defects compound — the
cost being minimised is not the SAH, and the winning plane is unusable.

For scale: 240 000 primary rays against `dragon_vrip.ply` take **2 min 17 s**. A correct
binned SAH should bring triangle tests per ray down to the tens.

**Ray set caveat.** Primary rays only, and they are coherent. Secondary rays start anywhere
and point everywhere and stress a tree differently; measuring those needs the seeded
sampler (item 3 of the order of work). So this baseline tracks the right direction but
understates the gain.

#### Effect of making the empty box report an area of 0 — 2026-07-31

`AABoundingBox::half_area()` now returns `0` on an empty box instead of `+inf`. This was
meant as a correctness cleanup, not a tree improvement, but it moves the numbers — worth
recording, because the direction is not the one intuition suggests.

| mesh | nodes (leaves) | depth | max leaf | tri tests/ray |
|---|---|---|---|---|
| `cube.ply` | 1 (1) → **23 (12)** | 1 → 5 | 12 → **1** | 5.80 → **1.31** |
| `bun_zipper_res4.ply` | 3 (2) → 3 (2) | 2 → 2 | 716 → 716 | 254.41 → 254.41 |
| `bunny.ply` | 121 (61) → 217 (109) | 14 → 16 | 37 469 → 37 469 | 7464.08 → 7464.08 |
| `dragon_vrip_res4.ply` | 351 (176) → 935 (468) | 19 → 22 | 3143 → 3143 | 517.28 → 517.13 |
| `dragon_vrip_res3.ply` | 1021 (511) → 2423 (1212) | 22 → 26 | 15 949 → 15 949 | 3622.88 → 3623.01 |
| `dragon_vrip.ply` | 2301 (1151) → 5755 (2878) | 27 → 32 | 316 949 → 316 949 | 66 649.36 → 66 658.07 |

Hit counts unchanged on all six, so the partition is intact.

**Reading.** The `+inf` was not a harmless accident: it *rejected outright* every candidate
plane with an empty bin, which is why subdivision stopped so early. Removing it more than
doubles the node count on the dragon. But it buys nothing per ray, because the areas are
still read per bin: an empty bin now makes a plane look **free** — wrong in the opposite
direction. So the extra nodes carve off crumbs while the dominant leaf, the one a real SAH
would attack, is untouched on every organic mesh. `cube.ply` is the exception that proves the
mechanism: 12 triangles over 8 bins leaves most bins empty, so the poisoning had blocked
every plane there.

Conclusion for the next step: the empty box had to be fixed, but the gain is entirely in the
cost function itself.

#### After the SAH fix — 2026-07-31

The cost now reads the areas of the accumulated **unions** either side of the plane, candidate
planes sit on the right bin boundaries, empty sides are rejected explicitly, and the partition
classifies with the same `bin_index` the cost model binned with.

| mesh | nodes (leaves) | depth | max leaf | **tri tests/ray** | box tests/ray |
|---|---|---|---|---|---|
| `cube.ply` | 11 (6) | 6 | 2 | 5.80 → **1.18** | 1.00 → 8.62 |
| `bun_zipper_res4.ply` | 1811 (906) | 14 | 4 | 254.41 → **0.77** | 2.45 → 15.27 |
| `bunny.ply` | 138 881 (69 441) | 21 | 2 | 7464.08 → **0.58** | 5.80 → 21.48 |
| `dragon_vrip_res4.ply` | 21 137 (10 569) | 19 | 5 | 517.28 → **0.63** | 7.36 → 18.22 |
| `dragon_vrip_res3.ply` | 92 929 (46 465) | 21 | 5 | 3622.88 → **0.60** | 6.68 → 20.87 |
| `dragon_vrip.ply` | 1 724 381 (862 191) | 29 | 6 | 66 649.36 → **0.58** | 6.45 → 24.76 |

Four to five orders of magnitude on the large meshes. The dominant leaf of `dragon_vrip.ply`
goes from 316 949 triangles — 36 % of the model — to 6. Measuring the six meshes took 2 min 17 s
for the dragon alone before; the whole set now runs in seconds.

**The partition is intact, checked two ways.** Raw hit counts are identical *to the unit* on all
six meshes (`cube` 116 004, `bun_zipper_res4` 47 693, `bunny` 46 904, `dragon_res4` 42 797,
`dragon_res3` 43 451, `dragon_vrip` 43 269) — `bvh_stats` now prints the raw count, not only the
rounded percentage, precisely so this comparison is possible. And a unit test compares
`BVHTree::query` against brute-force intersection of every triangle, requiring exact equality of
distance *and* triangle index; the hit count alone would not notice a ray that found a farther
triangle.

**Two costs, both expected.** Box tests per ray rise by a factor of 3 to 4: a deeper tree means
more nodes to reject, which is the trade the SAH makes and wins by a wide margin. And the node
count is now ~2× the triangle count, so leaves hold a single triangle on average — that is the
`t_trav = 0` departure showing (`docs/heuristique_aire_surface.md` §6): with traversal counted
free, splitting always looks worth it. Giving `t_trav` its real weight would shorten the tree
and cut the memory; deliberately left for later so this before/after measures one thing.

**A render is unchanged**, verified by rendering `cube_mesh.stage` from the previous commit and
from this one. A pixel-exact comparison is impossible while the sampler is unseeded, so the
brute-force test above is the real guarantee.

**Where the time goes now.** `bunny_mesh.stage` at 120×90×4 takes 53 s, which is no longer the
mesh's fault — the scene BVH clones its primitives into a `Vec` per ray and tests them all,
unordered. That is the next bottleneck, and it is the *BVH — scene* section below.

#### How to go about it

**Prerequisite, satisfied.** `half_area` now reports the true area (commit `6211906`).
Before that every box was inflated to a 0.01 minimum extent per axis, so repairing the
accumulation alone would have left the cost wrong anyway.

**Measure first — done.** See the baseline above. Re-run `bvh_stats` over the same meshes
after every change to the build and put the before/after figures in the commit message.

**Four traps, all verified while reviewing:**

- ~~**`new_invalid().half_area()` was `+inf`.**~~ *Half done.* `bmax - bmin` overflowed to
  `-inf`, so the sum of products came back `+inf`, and **any** empty bin poisoned its
  candidate with `inf * N = +inf` — not the rare `inf * 0 = NaN` edge case the review
  described, but every plane with an empty bin, discarded systematically. `AABoundingBox` now
  represents emptiness explicitly: `new_invalid()` is renamed `empty()` (the identity element
  of the union, not an invalid state), `is_empty()` is the way to ask, `half_area()` reports
  `0` — the area of the empty set — and `hit()` asserts the box is non-empty rather than
  relying on the overflow landing the right way. Measured effect above: it unblocks
  subdivision but buys nothing per ray. **Still to do in the binned path:** reject empty sides
  explicitly, `left_count[i] == 0 || right_count[i] == 0`, because with `0` an empty side now
  makes a plane look *free*.
- **`inv_scale` is misnamed.** It holds the bin width `(bound_max - bound_min) / 8`,
  identical to the `scale` computed a few lines above, and is the inverse of nothing. The
  misnomer is very likely what made `bound_min + i * inv_scale` read as plausible. Rename
  it and drop the duplicate.
- **Keep the cost convention.** `find_best_split_plane` returns `A_L·N_L + A_R·N_R` and
  `subdivide` compares it against `calculate_node_cost` = `A_node·N`. Neither is normalised
  by `A_node`, so the comparison is consistent as it stands — changing one side alone
  silently redefines the "is splitting worth it" test.
- **`next_node_idx` duplicates `nodes.len()`.** They stay in step only because every
  `push` happens to be paired with an increment. Worth collapsing while in the area.

**Documentation expected**, per `CLAUDE.md` §4: the SAH is a concept, so the surface-area
heuristic itself should be derived in the doc comment — why cost goes as area × primitive
count, what the probability argument behind it is, and why the areas must be those of the
*unions* either side of the plane rather than of the individual bins. That derivation is
precisely what would have made the current bug visible on reading.

### BVH — scene

- [ ] **`query` clones the primitives it finds** into an accumulator `Vec`
      ([bvh.rs:62](src/bvh.rs#L62)) — one allocation plus N atomic refcount bumps per ray.
- [ ] **No ordered traversal, no `far` narrowing, no early-out.** `Scene::intersect`
      collects every candidate, then tests them all. The BVH filters but does not order.
- [ ] **Split plane is a median split on a randomly chosen axis**
      ([bvh.rs:77](src/bvh.rs#L77)). The mesh BVH's binned SAH is the design to port here.
- [ ] **`BVHNode::new` on an empty vector** falls into the `_` arm and recurses forever;
      `Scene::commit` on an empty scene hits this.
- [ ] **No `intersect_p`.** Shadow rays go through the full nearest-hit-plus-material
      search when a boolean with an early-out would do — the main cost of NEE.
- [ ] **`Plane` reports an unbounded box**, `±f64::MAX` on x and z
      ([plane.rs:52-55](src/shapes/plane.rs#L52)), so its `half_area` is `inf` — which
      would poison any SAH cost the moment a `Plane` sits in the scene BVH. Not a bounding
      bug as such but a design question: an unbounded primitive has no business inside an
      acceleration structure (pbrt keeps them out of the accelerator). **Blocks the port of
      the binned SAH to the scene BVH** — decide when doing that work.

### Correctness / robustness

- [ ] **Needless `unsafe`** at [simple.rs:31-34](src/objects/simple.rs#L31) — a raw pointer
      is used to read `intersections[0]`, but `Intersection` is `Copy`. Also assumes the
      first element is the nearest; worth asserting that every `Intersectable` really does
      return a distance-sorted list.
- [ ] **Infinite lights build a degenerate visibility tester** —
      [uniform_infinite_light.rs:36](src/lights/uniform_infinite_light.rs#L36) and
      [background_infinite_light.rs:40](src/lights/background_infinite_light.rs#L40) pass
      `(0,0,0)` for both endpoints and ignore their `_intersection` argument, so the shadow
      ray has a null direction. These lights cast no shadow. `UniformInfiniteLight` is
      unusable as it stands.
- [x] **`AABoundingBox::new` inflated every axis to a minimum extent of 0.01**, which
      biased every `half_area` and therefore every SAH cost. *Done.* The investigation
      showed the clamp was not the guard it looked like: `Plane` and `Rectangle` pad
      themselves by hand, so triangles — the SAH-critical path — were its only clients.
      The real defect was in [`hit`](src/geom/aabound.rs), whose `tmax <= tmin`
      rejected every zero-thickness slab; the clamp merely papered over it. Changes:
      `hit` now rejects on `tmax < tmin` so a tangential hit counts (required: a bounding
      box is a *conservative* bound); rays parallel to a slab are handled explicitly
      instead of relying on `f64::max` silently dropping a NaN; the slab interval is
      widened by the rounding bound 2γ(3); `new` stores the
      bound faithfully with a `debug_assert`; `combine` and `Compound::get_bounding_box`
      use `empty()` instead of an inverted box fixed up by accident; `Rectangle`
      dropped its ±1.0 hand-padding for an exact flat bound. Four tests added for
      degenerate and grazing cases.
      The rounding bound has its own write-up in
      [docs/arithmetique_flottante.md](docs/arithmetique_flottante.md): §0–§3 cover float
      representation, the standard error model and γ(n); §4 derives 2γ(3) step by step,
      with worked intervals and the counter-example that forces the magnitude form. The
      doc comment on `hit` carries a condensed version of the same derivation.
- [ ] **Mesh normals are parsed and never used** (build warning): no interpolated shading
      normals, so meshes are visibly faceted. The geometric normal is derived as
      `cross(dpdv, dpdu)` from default UVs, which is a roundabout route with a fragile
      orientation.

## Departures from a physical model

- [ ] **No area lights — the biggest gap.** `DiffuseLight` is only a *material*; no
      `AreaLight` is ever registered in `Scene::lights`. So NEE can never sample an
      emissive panel, and [path.rs](src/integrators/path.rs) only accumulates emission when
      `is_last_bounce_specular`. Net effect: **an emissive surface contributes nothing to
      indirect lighting** — it is only visible on a direct view. This is why
      [loader.rs](src/loader.rs) hard-codes a `PointLight` and a background light to get
      scenes lit at all.
- [ ] **No MIS.** `Light` has no `pdf_li`, so NEE and BSDF sampling cannot be weighted
      against each other. Blocked on the item above.
- [ ] **Russian roulette is commented out**
      ([path.rs:93](src/integrators/path.rs#L93)); paths are cut dead at `max_depth`, and
      the cut at [path.rs:65](src/integrators/path.rs#L65) happens *before* the last
      vertex's light sampling — systematic energy loss.
- [ ] **No tone mapping.** [`gamma_correct`](src/spectrum.rs#L21) is a `sqrt` (gamma 2.0,
      not sRGB) and [`in_bound`](src/spectrum.rs#L134) hard-clips at 1.0, so all dynamic
      range above 1 is discarded.
- [ ] **`Spectrum` is an RGB triple with no declared colour space** — no primaries, no
      white point. The name promises spectral rendering that does not exist.
- [ ] **Lights are hard-coded in `Loader::load_scene`** and the `.stage` grammar has no
      `light` production at all. Lights should be declarable in the scene file.

## Renderer & infrastructure

- [ ] **Per-pixel round-robin dispatch** in [mt.rs](src/renderers/mt.rs): ~480 000 channel
      messages for an 800×600 image, no load balancing (the schedule is fixed in advance,
      so one thread inheriting a costly region holds up the frame), the main loop spins on
      a non-blocking `try_recv` once the pixel iterator is exhausted, and the channels are
      unbounded. [`Bounds2`](src/geom/bounds2.rs) already supports the tiling that fixes
      all four.
- [ ] **~50 lines duplicated** between [st.rs](src/renderers/st.rs) and
      [mt.rs](src/renderers/mt.rs): `compute_pixel`, `Sampler2`, `image_write` are
      identical. Extract `Film` (accumulate + write) and `Sampler`; the two renderers
      should then differ only in scheduling.
- [ ] **Renders are not reproducible.** `rand 0.3` via `thread_rng` is not seeded here, so
      two runs cannot be compared — which makes every change to the integrator unverifiable.
      Upgrading to `rand 0.8` also buys stratified/Sobol samplers.
- [ ] **The `examples/` directory no longer compiles** — 22 errors, all the same cause: the
      `match config.integrator` in each example predates the `NAIVE` and `NORMAL` variants
      and is now non-exhaustive. `cargo test` therefore fails as a whole; only
      `cargo test --lib` and `cargo test --doc` are green. Either fix the matches or drop
      the examples now that `.stage` files cover the same ground.
- [ ] Dead weight: `src/_keep.rs` and `src/shapes/triangle.cpp` are not compiled;
      `integrators/whitted.rs` no longer compiles and is commented out of the module;
      `crossbeam` is still declared in `Cargo.toml` but unused (`thread::scope` replaced
      it); the build emits 24 warnings.
- [ ] `edition = "2018"` in `Cargo.toml` vs `edition = "2021"` in `rustfmt.toml`.

## Also worth noting

The two oldest unchecked items above — "scene from text file loader" and "support for
triangle based geometry" — look done (`src/loader/`, `src/shapes/triangle_mesh/`). Left
unchecked deliberately: that is the author's call, not the reviewer's.

