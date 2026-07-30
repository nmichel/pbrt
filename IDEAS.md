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

- [ ] **SAH cost is computed from the wrong box.**
      [bvh.rs:343](src/shapes/triangle_mesh/bvh.rs#L343) and
      [bvh.rs:349](src/shapes/triangle_mesh/bvh.rs#L349) use `left_bin.bounds.half_area()`
      — the area of the *single* bin — where the SAH needs the area of the union of all
      bins on that side of the plane. `left_box`/`right_box` are accumulated correctly at
      [bvh.rs:334](src/shapes/triangle_mesh/bvh.rs#L334) and then never read. The cost
      being minimised is therefore not the SAH, and the chosen splits are not the best
      ones. Affects tree quality only, not the image.
- [ ] **First candidate plane is degenerate.**
      [bvh.rs:358](src/shapes/triangle_mesh/bvh.rs#L358): `best_pos = bound_min + i * inv_scale`
      with `i` starting at 0 puts the first plane exactly on `bound_min`, so the left side
      is empty; the `left_count == 0` guard then returns and subdivision stops early. The
      boundary of bin `i` is at `bound_min + (i + 1) * scale`.
- [ ] `evaluate_sah` is dead code (build warning) — the binned path replaced it. Remove or
      keep as the reference implementation a test can check the binned version against.

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
      use `new_invalid()` instead of an inverted box fixed up by accident; `Rectangle`
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

