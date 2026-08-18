//! Measures the quality of the two bounding volume hierarchies.
//!
//! ```text
//! cargo run --release --bin bvh_stats -- test_files/bunny.ply test_files/bunny_mesh.stage
//! ```
//!
//! Two modes, told apart by the file extension — which is the meaningful distinction and not a
//! shorthand: a `.ply` is a bare mesh and has no camera, so this tool has to invent a ray set for
//! it, whereas a `.stage` carries its own camera and the ray set is simply the render's.
//!
//! - **`.ply`** — the mesh BVH of `TriangleMesh`, over one mesh, no scene, no materials.
//! - **`.stage`** — the scene BVH of `Scene`, over the objects of a real scene file.
//!
//! # Why a tool and not a test
//!
//! A change to either accelerator does not change the image — a badly built tree renders the same
//! picture, only more slowly. So the only way to tell an improvement from a regression is to
//! count the work a fixed set of rays costs, before and after. This tool prints those counts; the
//! figures belong in the commit message of any change to a build or a traversal.
//!
//! It is not a test: it asserts nothing. There is no reference value to compare against, only the
//! previous run's.
//!
//! # The ray sets, and why they are reproducible
//!
//! Reproducibility is what makes this measurable while the renderer's sampler is unseeded, so that
//! no lighting result can be compared between two runs. Neither accelerator draws a random number,
//! in its build or in its traversal, so the same rays over the same geometry give **counts identical
//! to the unit** — in both modes. Every figure printed here can therefore be compared across
//! commits, and a movement of one is a change in the code and not noise.
//!
//! In mesh mode the rays are the primary rays of `VIEW_COUNT` pinhole cameras placed on a circle
//! around the mesh, one ray through the centre of each pixel. Several viewpoints rather than one,
//! because a single one can be accidentally favourable — looking straight down an axis of an
//! axis-aligned mesh, for instance.
//!
//! In scene mode there is nothing to invent: `Loader::load_scene` hands back the camera the scene
//! file declares, and the rays are one per pixel centre of a small render at the renderer's own
//! default settings.
//!
//! # What it does *not* measure
//!
//! Only primary rays, which are coherent: they all start from one point and diverge slowly, plus
//! one shadow ray per hit, which is a segment towards a fixed point. Secondary rays — the bulk of
//! the work in a path tracer — start anywhere and point everywhere, and stress a tree differently.
//! Reaching them requires a seeded sampler, without which their ray set is not reproducible and the
//! counters mean nothing.

use std::f64::consts::PI;
use std::{env, fs};

use pbrt::bvh::TraversalStats as SceneTraversalStats;
use pbrt::cameras::{Camera, PinHoleCamera};
use pbrt::config::default_config;
use pbrt::geom::aabound::AABound;
use pbrt::geom::matrix4::Matrix4;
use pbrt::geom::ray::Ray;
use pbrt::geom::vector2::Vector2u;
use pbrt::geom::vector3::Vector3f;
use pbrt::loader::{load_ply_mesh, Loader};
use pbrt::scene::Scene;
use pbrt::shapes::triangle_mesh::TraversalStats as MeshTraversalStats;
use pbrt::shapes::TriangleMesh;

/// Viewpoints the mesh is measured from, spread evenly in azimuth.
const VIEW_COUNT: usize = 6;

/// Elevation of every viewpoint above the mesh's equator. Deliberately not zero: a
/// viewpoint level with the equator sees the mesh's own symmetry planes edge-on, which is
/// exactly the accidentally-favourable case a single viewpoint would hide.
const VIEW_ELEVATION: f64 = 20.0 * PI / 180.0;

/// Side of the square image cast from each viewpoint, so `VIEW_COUNT × IMAGE_SIDE²` rays
/// in all. Square because a square image needs no aspect-ratio reasoning.
const IMAGE_SIDE: u32 = 200;

/// Field of view. Wide enough to hold the whole mesh, narrow enough that a good share of
/// the rays actually reach it: a ray set that mostly misses measures the tree's root box
/// and little else.
const FOV: f64 = 40.0 * PI / 180.0;

/// Ray interval, mirroring the renderer's defaults (`Config::near`, `Config::far`). The
/// interval is an input to the traversal — it is what lets a box test fail early — so the
/// measurement has to charge the same one the renderer does.
const NEAR: f64 = 0.0001;
const FAR: f64 = 1000.0;

/// Interval start for a shadow ray, mirroring the `0.00001` that `VisibilityTester::unoccluded`
/// uses today. It differs from `NEAR` because it serves a different purpose — pushing the ray off
/// the surface it starts on, rather than clipping the near plane — and the two should not be
/// conflated just because they are both small.
const SHADOW_NEAR: f64 = 0.00001;

/// Resolution of the render whose primary rays make up the scene-mode ray set. The renderer's own
/// default aspect ratio, at a fraction of its size: the point is to cast the rays a render casts,
/// and the aspect ratio is part of that.
const SCENE_IMAGE_WIDTH: usize = 200;
const SCENE_IMAGE_HEIGHT: usize = 150;

fn main() {
    let paths: Vec<String> = env::args().skip(1).collect();

    if paths.is_empty() {
        eprintln!("usage: bvh_stats <mesh.ply | scene.stage> [...]");
        return;
    }

    for path in &paths {
        if path.ends_with(".stage") {
            report_scene(path);
        }
        else {
            report_mesh(path);
        }
    }
}

/// The point the shadow rays aim at: the `PointLight` that `Loader::load_scene` hard-codes into
/// every scene. Aiming elsewhere would measure segments the renderer never casts.
fn shadow_target() -> Vector3f {
    Vector3f::new(0.0, 2.0, 1.0)
}

/// Counters for one ray set.
struct SceneCast {
    stats: SceneTraversalStats,
    ray_count: usize,
    hit_count: usize,
}

impl SceneCast {
    fn new() -> Self {
        Self {
            stats: SceneTraversalStats::default(),
            ray_count: 0,
            hit_count: 0,
        }
    }

    fn report(&self, label: &str) {
        let hit_ratio = self.hit_count as f64 / self.ray_count as f64;
        let per_ray = |total: usize| total as f64 / self.ray_count as f64;

        println!("  {:<24} {} ({} hit, {:.1} %)", label, self.ray_count, self.hit_count, 100.0 * hit_ratio);
        println!("    nodes visited / ray    {:.2}", per_ray(self.stats.nodes_visited));
        println!("    box tests / ray        {:.2}", per_ray(self.stats.box_tests));
        println!("    object tests / ray     {:.2}", per_ray(self.stats.object_tests));
    }
}

fn report_scene(path: &str) {
    let text = fs::read_to_string(path).expect("should be able to read the scene file");

    // The renderer's own defaults, at a smaller resolution: the camera the scene declares is built
    // from a `Config`, so measuring the rays a render would cast means starting from the same
    // settings rather than inventing a parallel set.
    let mut config = default_config();
    config.output_width = SCENE_IMAGE_WIDTH;
    config.output_height = SCENE_IMAGE_HEIGHT;

    let (scene, camera) = Loader::load_scene(&text, &config);
    let (primary, shadow) = cast_scene_ray_sets(&scene, camera.as_ref());

    println!("{}", path);
    println!("  primitives               {}", scene.primitive_count());
    primary.report("primary rays");
    shadow.report("shadow rays");
    println!();
}

/// Casts two ray sets and counts them apart: the camera's primary rays, and one shadow ray from
/// each point they hit.
///
/// The two are reported separately because they are different problems. A primary ray wants the
/// *nearest* hit; a shadow ray only wants to know whether *any* surface stands in the way, and
/// paying for the nearest one — with its shading normal, its texture coordinates and its
/// derivatives — is the cost `intersect_p` exists to remove. Keeping the counters apart is what
/// will make that improvement visible instead of merely plausible.
fn cast_scene_ray_sets(scene: &Scene, camera: &dyn Camera) -> (SceneCast, SceneCast) {
    let mut primary = SceneCast::new();
    let mut shadow = SceneCast::new();
    let target = shadow_target();

    for pixel_y in 0..SCENE_IMAGE_HEIGHT {
        for pixel_x in 0..SCENE_IMAGE_WIDTH {
            let ray = camera.get_ray(pixel_x as f64 + 0.5, pixel_y as f64 + 0.5);

            primary.ray_count += 1;
            let interaction = scene.intersect_instrumented(&ray, NEAR, FAR, &mut primary.stats);

            let hit = match interaction {
                Some(interaction) => interaction,
                None => continue,
            };
            primary.hit_count += 1;

            // Exactly what `VisibilityTester::unoccluded` builds: a segment from the shaded point
            // to the light, bounded by the light's own distance — an occluder beyond the light is
            // not an occluder — and answered by `intersect_p`.
            let shadow_ray = Ray::spawn_from_through(&hit.intersection.p, &target);
            let light_distance = (&target - &hit.intersection.p).length();

            shadow.ray_count += 1;
            if scene.intersect_p_instrumented(&shadow_ray, SHADOW_NEAR, light_distance, &mut shadow.stats) {
                shadow.hit_count += 1;
            }
        }
    }

    (primary, shadow)
}

fn report_mesh(path: &str) {
    let mesh = load_ply_mesh(path, false);
    let build = mesh.build_stats();
    let cast = cast_ray_set(&mesh);

    let triangle_count = mesh.triangle_count();
    let mean_leaf_tri_count = build.total_leaf_tri_count as f64 / build.leaf_count as f64;
    let hit_ratio = cast.hit_count as f64 / cast.ray_count as f64;
    let per_ray = |total: usize| total as f64 / cast.ray_count as f64;

    println!("{}", path);
    println!("  triangles                {}", triangle_count);
    println!("  nodes                    {} ({} leaves)", build.node_count, build.leaf_count);
    println!("  max depth                {}", build.max_depth);
    println!(
        "  leaf triangles           mean {:.1}, max {}",
        mean_leaf_tri_count, build.max_leaf_tri_count
    );

    // Every triangle sits in exactly one leaf. A mismatch means the in-place partitioning
    // lost or duplicated one, which would make every figure below meaningless.
    if build.total_leaf_tri_count != triangle_count {
        println!(
            "  ** partition is broken: {} triangles across leaves, mesh has {}",
            build.total_leaf_tri_count, triangle_count
        );
    }

    // The raw count, not only the rounded percentage: it is the invariant that guards the
    // partition across a change to the build. Which rays reach the mesh is a property of the
    // geometry alone — a better tree changes the work done to find the hit, never the hit. A
    // count that moves means a triangle was lost.
    println!(
        "  rays                     {} ({} hit the mesh, {:.1} %)",
        cast.ray_count,
        cast.hit_count,
        100.0 * hit_ratio
    );
    println!("  nodes visited / ray      {:.2}", per_ray(cast.stats.nodes_visited));
    println!("  box tests / ray          {:.2}", per_ray(cast.stats.box_tests));
    println!("  triangle tests / ray     {:.2}", per_ray(cast.stats.triangle_tests));
    println!();
}

/// Outcome of casting the whole ray set at one mesh.
struct RaySetCast {
    stats: MeshTraversalStats,
    ray_count: usize,

    /// Rays that found a triangle. Reported because it qualifies everything else: a
    /// traversal count averaged over rays that mostly miss says nothing about the tree.
    hit_count: usize,
}

fn cast_ray_set(mesh: &TriangleMesh) -> RaySetCast {
    let bbox = mesh.get_bounding_box();
    let center = bbox.centroid();

    // Frame the mesh through its bounding sphere, of radius half the box diagonal. Seen
    // from a distance d, that sphere subtends a half-angle α with sin α = r / d, so it fits
    // inside a field of view of 2θ as soon as d ≥ r / sin θ. Taking the equality places
    // the sphere tangent to the view cone — the tightest framing that still shows all of
    // the mesh, from every azimuth, whatever its shape.
    let radius = (bbox.bmax - bbox.bmin).length() * 0.5;
    let distance = radius / (FOV / 2.0).sin();

    let mut cast = RaySetCast {
        stats: MeshTraversalStats::default(),
        ray_count: 0,
        hit_count: 0,
    };

    for view in 0..VIEW_COUNT {
        let camera = orbit_camera(view, &center, distance);

        for pixel_y in 0..IMAGE_SIDE {
            for pixel_x in 0..IMAGE_SIDE {
                // Pixel centre, not corner: a corner ray of an axis-aligned mesh can land
                // exactly on a shared triangle edge, a degenerate case that has no reason
                // to be over-represented in a measurement.
                let ray = camera.get_ray(pixel_x as f64 + 0.5, pixel_y as f64 + 0.5);

                cast.ray_count += 1;
                if mesh.intersect_instrumented(&ray, NEAR, FAR, &mut cast.stats).is_some() {
                    cast.hit_count += 1;
                }
            }
        }
    }

    cast
}

/// Camera number `view` of the orbit, looking at `center` from `distance` away.
fn orbit_camera(view: usize, center: &Vector3f, distance: f64) -> PinHoleCamera {
    let azimuth = 2.0 * PI * (view as f64) / (VIEW_COUNT as f64);

    // Spherical coordinates, y up: elevation lifts the viewpoint off the xz plane, azimuth
    // turns it around the y axis.
    let direction = Vector3f::new(
        VIEW_ELEVATION.cos() * azimuth.cos(),
        VIEW_ELEVATION.sin(),
        VIEW_ELEVATION.cos() * azimuth.sin(),
    );
    let position = center + &(&direction * distance);

    let up = Vector3f::new(0.0, 1.0, 0.0);
    let resolution = Vector2u::new(IMAGE_SIDE, IMAGE_SIDE);

    PinHoleCamera::new(&resolution, FOV, NEAR, FAR, Matrix4::look_at(&position, center, &up))
}
