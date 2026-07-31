//! Measures the quality of the triangle-mesh BVH.
//!
//! ```text
//! cargo run --release --bin bvh_stats -- test_files/cube.ply test_files/bunny.ply test_files/dragon_vrip.ply
//! ```
//!
//! # Why a tool and not a test
//!
//! A change to the split heuristic does not change the image — a badly built tree renders
//! the same picture, only more slowly. So the only way to tell an improvement from a
//! regression is to count the work a fixed set of rays costs, before and after. This tool
//! prints those counts; the figures belong in the commit message of any change to the
//! build.
//!
//! It is not a test: it asserts nothing. There is no reference value to compare against,
//! only the previous run's.
//!
//! # The ray set, and why it is reproducible
//!
//! Reproducibility is what makes this measurable today: the renderer's sampler is not
//! seeded yet, so no lighting result can be compared between two runs — but a traversal
//! draws no random numbers. Given the same rays, the counters are identical to the unit.
//!
//! The rays are the primary rays of `VIEW_COUNT` pinhole cameras placed on a circle around
//! the mesh, one ray through the centre of each pixel. Several viewpoints rather than one,
//! because a single one can be accidentally favourable — looking straight down an axis of
//! an axis-aligned mesh, for instance.
//!
//! # What it does *not* measure
//!
//! Only primary rays, which are coherent: they all start from one point and diverge
//! slowly. Secondary rays — the bulk of the work in a path tracer — start anywhere and
//! point everywhere, and stress a tree differently. Measuring those needs a seeded
//! sampler, so it has to wait.

use std::env;
use std::f64::consts::PI;

use pbrt::cameras::{Camera, PinHoleCamera};
use pbrt::geom::aabound::AABound;
use pbrt::geom::matrix4::Matrix4;
use pbrt::geom::vector2::Vector2u;
use pbrt::geom::vector3::Vector3f;
use pbrt::loader::load_ply_mesh;
use pbrt::shapes::triangle_mesh::TraversalStats;
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

fn main() {
    let paths: Vec<String> = env::args().skip(1).collect();

    if paths.is_empty() {
        eprintln!("usage: bvh_stats <mesh.ply> [mesh.ply ...]");
        return;
    }

    for path in &paths {
        report(path);
    }
}

fn report(path: &str) {
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
    stats: TraversalStats,
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
        stats: TraversalStats::default(),
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
