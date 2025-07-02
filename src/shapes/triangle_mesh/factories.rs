use super::TriangleMesh;

pub fn unit_cube() -> TriangleMesh {
    let coords: Vec<f64> = vec![
        // Sommet 0: arrière-bas-gauche
        -0.5, -0.5, -0.5, // Sommet 1: arrière-bas-droite
        0.5, -0.5, -0.5, // Sommet 2: arrière-haut-droite
        0.5, 0.5, -0.5, // Sommet 3: arrière-haut-gauche
        -0.5, 0.5, -0.5, // Sommet 4: avant-bas-gauche
        -0.5, -0.5, 0.5, // Sommet 5: avant-bas-droite
        0.5, -0.5, 0.5, // Sommet 6: avant-haut-droite
        0.5, 0.5, 0.5, // Sommet 7: avant-haut-gauche
        -0.5, 0.5, 0.5,
    ];

    // Indices des triangles (2 triangles par face, 12 triangles au total)
    // Convention: sens antihoraire vu de l'extérieur
    let indices: Vec<usize> = vec![
        // Face arrière (z = -0.5)
        0, 1, 2, 0, 2, 3, // Face avant (z = +0.5)
        4, 6, 5, 4, 7, 6, // Face gauche (x = -0.5)
        0, 3, 7, 0, 7, 4, // Face droite (x = +0.5)
        1, 5, 6, 1, 6, 2, // Face bas (y = -0.5)
        0, 4, 5, 0, 5, 1, // Face haut (y = +0.5)
        3, 2, 6, 3, 6, 7,
    ];

    TriangleMesh::new(coords, indices, None, None)
}
