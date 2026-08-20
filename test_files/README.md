# PLY files

[The Stanford 3D Scanning Repository](https://graphics.stanford.edu/data/3Dscanrep/)

# Scenes

Most `.stage` files here are there to be looked at. Two are there to be measured, and hold far more
primitives than a picture needs:

- `many_spheres.stage` — the final scene of "Ray Tracing in One Weekend", 445 spheres. One draw of
  `examples/test_scene.rs`, whose spheres come from an unseeded generator, frozen so that the
  geometry belongs to the file rather than to the run. The only scene whose load time is dominated
  by the tree build rather than by PLY parsing, which is what makes a build change measurable on it.
- `many_meshes.stage` — the same bunny instanced 100 times, the only scene whose accelerator holds a
  large number of *meshes*.

Both are described in `docs/mesures_bvh.md` §2.3, and their reference figures are in its §4.
