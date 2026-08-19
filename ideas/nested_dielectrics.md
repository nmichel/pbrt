# Nested dielectrics — a transmitter inside another transmitter

Indexed from [IDEAS.md](../IDEAS.md). Not started.

Two scenes a physically based renderer is expected to handle are rendered wrong today, without
crashing and without any warning: **a transparent shape inside another transparent shape** (glass in
water, an air bubble in glass) and **two shapes sharing a face, with different materials** (glass
against water, glass against a mirror). Both are well-posed scenes — nothing about them is
ambiguous or ill-defined — and both fail, for two independent reasons.

## 1. The outer medium is hard-wired to ηᵢ = 1

[materials/dielectric.rs:57-79](../src/materials/dielectric.rs#L57) picks the index pair from the
sign of `local_wo.z` alone:

```rust
if local_wo.z <= 0.0 {          // leaving
    ni = self.ref_idx;
    nt = 1.0;                   // ← the outside is vacuum, by construction
}
else {                          // entering
    ni = 1.0;                   // ← same
    nt = self.ref_idx;
}
```

`Dielectric` knows only *its own* index, and has no way to learn the other one: neither `Ray`
(origin and direction, nothing else) nor `Interaction` (intersection and material) carries the
medium the ray is travelling through. Every interface is therefore computed as if it bordered
vacuum.

For glass (η = 1.517) inside water (η = 1.333), at the two inner interfaces:

| | computed | physical |
|---|---|---|
| ratio water→glass | 1 / 1.517 = 0.659 | 1.333 / 1.517 = 0.879 |
| ratio glass→water | 1.517 | 1.138 |
| critical angle, glass→water | asin(1/1.517) = **41.2°** | asin(1/1.138) = **61.5°** |
| Fresnel at normal incidence | **4.22 %** | **0.42 %** |

Three consequences, worst first: **spurious total internal reflection** across the whole
41.2°–61.5° band, i.e. black regions where the image should transmit; an inner reflection ten times
too bright; and over-strong bending, so the refraction distorts too much.

The case that shows the defect most plainly is an **air bubble in glass** — physically *the*
interface that produces total internal reflection. Here a sphere of index 1.0 gives
`ni_over_nt = 1`: no bending, zero Fresnel, and the bubble is **invisible**. To see it, the scene
has to declare η = 1/1.517 ≈ 0.659, which bakes the surrounding geometry into the material. That
holds for one level of nesting, breaks as soon as a shape is sometimes immersed and sometimes not,
and contradicts `CLAUDE.md` §2: a `Material` never knows concrete geometry.

## 2. Coincident faces: a coin flip, then a skipped interface

Glass on the left, water on the right, shared face at x = 0.

**Two shapes report a crossing on that same plane**, each computed in its own local frame by a
different arithmetic route. `Scene::intersect` keeps the strictly nearest, so the winner is decided
by rounding and **varies from ray to ray**. On screen that is salt-and-pepper noise along the seam,
not a stable error.

**Whichever wins, the other is lost.** Say the glass exit face wins. `scatter` computes
glass→vacuum, then [dielectric.rs:95-96](../src/materials/dielectric.rs#L95) moves the origin:

```rust
let shift_avoid_acne = world_outward_normal * -0.001;
scattered_ray_origin = p + &shift_avoid_acne;
```

The new origin sits 0.001 *past* x = 0, hence already inside the water shape. Water's entry face is
now behind the ray origin and will never be intersected. The path rendered is

> air→glass (right) · glass→**air** at the seam (instead of glass→water) · the water volume crossed
> with **no interface at all** · water→air on the far face

One interface computed with the wrong index pair, one interface silently gone.

Glass against a mirror follows the same shape: if the glass face wins, a glass→air refraction that
has no physical existence, then the metal reflection 0.001 further on; if the metal face wins, the
glass exit vanishes. And `Metal` reads no index at all, so a mirror immersed in glass reflects
exactly as it would in air — while the index pair is precisely what sets the colour of that
reflection.

## The fix, and which seam it moves

The standard algorithm is **nested dielectrics** (Schmidt & Budge, 2002 — as used by RenderMan and
Arnold): each dielectric carries a *priority*, the ray carries a small stack of the dielectrics it
is currently inside, and each hit reads the top of the stack for ηᵢ. It settles both problems at
once — nesting, and coincident surfaces, whose overlap is then resolved by priority instead of by
rounding.

The structural point: **this state belongs to the integrator, not to the material.** The integrator
owns the path, so it owns "where am I". `Dielectric` should declare its own η and be *given* ηᵢ, not
derive it. The seam to change is therefore the signature of `Material::scatter`, not the inside of
`Dielectric`.

## Prerequisite, and how to know it works

- **The anti-acne offset has to move from the position to the ray interval** (`near = ε` rather than
  displacing `p`). Displacing the origin by 0.001 scene units steps through any wall thinner than
  that, so without this the stack reads the right ηᵢ for an interface the ray has already jumped
  over. Small, independent, and required first.
- **Write the test scene before the fix.** No current `.stage` exercises either case, so nothing
  regresses — and nothing would detect the fix either. Two concentric spheres with different
  indices, and two `AABox` sharing a face, are enough; the seam noise of case 2 is visible at any
  sample count.
- The gain is correctness, not speed. It is worth stating in whatever commit lands it that the image
  changes only in scenes that were wrong before.
