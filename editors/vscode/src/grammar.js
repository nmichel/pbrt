// The `.stage` language, described once, for both the hover and the completion
// providers.
//
// One entry per keyword of `src/loader/parser/lexer.rs`. Keys stay quoted and each one
// opens its entry on its own line: `tests/vscode_grammar_sync.rs` scans them as plain
// text and fails the Cargo test suite when this table and the lexer drift apart.
//
// Each entry describes what the *implementation* does, not what the AST node calls its
// fields — `rectangle` is the cautionary tale. Prose is French, like `docs/`.

const PRODUCTIONS = {
  // ---------------------------------------------------------------- structure

  "camera": {
    group: "structure",
    signature: "camera <pin_hole|thin_lens>",
    summary:
      "Ouvre la déclaration de caméra. C'est la **première** chose d'un fichier `.stage` : " +
      "le parser lit la caméra, puis la scène, dans cet ordre imposé.",
    params: [{ name: "type", type: "pin_hole | thin_lens", doc: "modèle de caméra" }],
    next: "camera_type",
    example: "camera pin_hole\n  pos 0.0 0.0 800.0\n  look 0.0 0.0 0.0\n  up 0.0 1.0 0.0",
    source: "src/loader/parser.rs:34",
    snippet: "camera pin_hole\n  pos ${1:0.0} ${2:0.0} ${3:800.0}\n  look ${4:0.0} ${5:0.0} ${6:0.0}\n  up ${7:0.0} ${8:1.0} ${9:0.0}\n\nscene\n  $0",
  },

  "scene": {
    group: "structure",
    signature: "scene <object>…",
    summary:
      "Ouvre la liste des objets. Le bloc n'a **pas de délimiteur de fin** : il court jusqu'à " +
      "la fin du fichier, et tout jeton qui n'est pas `object` y est une erreur.",
    params: [],
    next: "object",
    example: "scene\n  object simple\n    sphere 1.0\n    lambertian color 0.8 0.3 0.3",
    source: "src/loader/parser.rs:84",
  },

  "object": {
    group: "structure",
    signature: "object <simple|transformed|compound>",
    summary: "Introduit un objet, c'est-à-dire l'association d'une géométrie et d'un matériau.",
    params: [{ name: "sorte", type: "simple | transformed | compound", doc: "forme de l'objet" }],
    next: "object_kind",
    example: "object simple\n  sphere 1.0\n  metal 0.0 color 0.8 0.8 0.9",
    source: "src/loader/parser.rs:104",
  },

  "transform": {
    group: "structure",
    signature: "transform { <étape>… }",
    summary:
      "Bloc de placement. Les étapes s'appliquent au point dans l'**ordre où elles sont écrites** : " +
      "on écrit donc la rotation d'abord, la translation ensuite. Le bloc est obligatoire dans un " +
      "`object transformed` et dans chaque `elem` d'un `csg`, même vide — `transform { }` vaut " +
      "l'identité. Il n'y a **pas** de mise à l'échelle dans le langage.",
    params: [],
    next: "transform_step",
    example: "transform {\n  rotate_y -0.2618\n  translate 80.0 -150.0 -100.0\n}",
    source: "src/loader/visitors/scene_builder.rs:195",
    snippet: "transform {\n  $0\n}",
  },

  "elem": {
    group: "structure",
    signature: "elem <forme> <transform>",
    summary:
      "Membre d'un bloc `csg`. Une forme, puis son bloc `transform` — les deux sont obligatoires, " +
      "car c'est le seul moyen de placer les formes les unes par rapport aux autres.",
    params: [
      { name: "forme", type: "forme", doc: "`sphere`, `aabox`, `csg` imbriqué…" },
      { name: "transform", type: "bloc", doc: "placement de cette forme" },
    ],
    next: "shape",
    example: "elem\n  sphere 0.4\n  transform {\n    translate -0.5 0 -0.5\n  }",
    source: "src/loader/parser.rs:200",
    snippet: "elem\n  ${1:sphere 1.0}\n  transform {\n    $0\n  }",
  },

  // ------------------------------------------------------------------ caméras

  "pin_hole": {
    group: "camera_type",
    signature: "pin_hole pos <x y z> look <x y z> up <x y z>",
    summary:
      "Sténopé : profondeur de champ infinie, tout est net. Le champ de vision, les plans de " +
      "coupe et la résolution ne sont **pas** dans le fichier — ils viennent de la ligne de " +
      "commande (`--fov`, `--near`, `--far`).",
    params: [
      { name: "pos", type: "vecteur", doc: "position de l'œil" },
      { name: "look", type: "vecteur", doc: "point visé" },
      { name: "up", type: "vecteur", doc: "verticale de l'image" },
    ],
    next: "pos",
    example: "camera pin_hole\n  pos 0.0 0.0 800.0\n  look 0.0 0.0 0.0\n  up 0.0 1.0 0.0",
    source: "src/loader/visitors/scene_builder.rs:61",
  },

  "thin_lens": {
    group: "camera_type",
    signature: "thin_lens pos <x y z> look <x y z> up <x y z> radius <r> focal_length <f>",
    summary:
      "Lentille mince : donne une profondeur de champ. Le plan de netteté est à `focal_length` " +
      "de l'œil, et `radius` règle l'ouverture — à 0, l'image redevient celle d'un sténopé.",
    params: [
      { name: "pos", type: "vecteur", doc: "position de l'œil" },
      { name: "look", type: "vecteur", doc: "point visé" },
      { name: "up", type: "vecteur", doc: "verticale de l'image" },
      { name: "radius", type: "nombre", doc: "rayon de la lentille ; 0 = tout net" },
      { name: "focal_length", type: "nombre", doc: "distance du plan de netteté" },
    ],
    next: "pos",
    example:
      "camera thin_lens\n  pos 0.0 0.0 10.0\n  look 0.0 0.0 0.0\n  up 0.0 1.0 0.0\n" +
      "  radius 0.1\n  focal_length 10.0",
    source: "src/loader/visitors/scene_builder.rs:75",
  },

  "pos": {
    group: "label",
    signature: "pos <x> <y> <z>",
    summary: "Position de l'œil, en coordonnées du monde. Étiquette obligatoire, la première des trois.",
    params: [{ name: "position", type: "vecteur", doc: "trois nombres, sans séparateur" }],
    next: "look",
    example: "pos 0.0 0.0 800.0",
    source: "src/loader/parser.rs:44",
  },

  "look": {
    group: "label",
    signature: "look <x> <y> <z>",
    summary:
      "Point visé, et non une direction : la caméra regarde de `pos` vers `look`. Deuxième " +
      "étiquette de la caméra.",
    params: [{ name: "cible", type: "vecteur", doc: "point du monde regardé" }],
    next: "up",
    example: "look 0.0 0.0 0.0",
    source: "src/loader/parser.rs:44",
  },

  "up": {
    group: "label",
    signature: "up <x> <y> <z>",
    summary:
      "Verticale de l'image. Colinéaire à la direction de visée, la matrice de vue dégénère — " +
      "c'est le piège classique de `look_at`.",
    params: [{ name: "verticale", type: "vecteur", doc: "généralement `0 1 0`" }],
    next: "scene",
    example: "up 0.0 1.0 0.0",
    source: "src/loader/parser.rs:44",
  },

  "radius": {
    group: "label",
    signature: "radius <r>",
    summary: "Rayon de la lentille d'une caméra `thin_lens`. Plus il est grand, plus le flou d'arrière-plan est marqué.",
    params: [{ name: "r", type: "nombre", doc: "rayon d'ouverture" }],
    next: "focal_length",
    example: "radius 0.1",
    source: "src/loader/parser.rs:60",
  },

  "focal_length": {
    group: "label",
    signature: "focal_length <f>",
    summary:
      "Distance entre l'œil et le plan de netteté d'une caméra `thin_lens`. Ce n'est pas une " +
      "focale d'objectif : ce qui est à cette distance est net, le reste ne l'est pas.",
    params: [{ name: "f", type: "nombre", doc: "distance du plan net" }],
    next: "scene",
    example: "focal_length 10.0",
    source: "src/loader/parser.rs:60",
  },

  // ------------------------------------------------------------------- objets

  "simple": {
    group: "object_kind",
    signature: "simple <forme> <matériau>",
    summary: "Une forme et son matériau, sans placement — la forme reste dans son repère de référence.",
    params: [
      { name: "forme", type: "forme", doc: "`sphere`, `rectangle`, `mesh`…" },
      { name: "matériau", type: "matériau", doc: "`lambertian`, `metal`, `dielectric`, `diffuse_light`" },
    ],
    next: "shape",
    example: "object simple\n  sphere 1.0\n  lambertian color 0.8 0.3 0.3",
    source: "src/loader/parser.rs:104",
    snippet: "simple\n  ${1:sphere 1.0}\n  ${2:lambertian color 0.8 0.3 0.3}",
  },

  "transformed": {
    group: "object_kind",
    signature: "transformed object <objet> transform { … }",
    summary:
      "Enveloppe un objet d'un placement. Noter le second `object` : c'est un objet complet qui " +
      "est transformé, donc la construction s'imbrique — un `transformed` peut envelopper un " +
      "`compound`, lui-même fait de `transformed`.",
    params: [
      { name: "objet", type: "objet", doc: "l'objet placé" },
      { name: "transform", type: "bloc", doc: "rotations et translations" },
    ],
    next: "object",
    example:
      "object transformed\n  object simple\n    aabox 165.0 330.0 165.0\n" +
      "    lambertian color 1.0 1.0 1.0\n  transform {\n    rotate_y -0.2618\n" +
      "    translate 80.0 -150.0 -100.0\n  }",
    source: "src/loader/parser.rs:104",
    snippet: "transformed\n  object ${1:simple\n    sphere 1.0\n    lambertian color 0.8 0.3 0.3}\n  transform {\n    $0\n  }",
  },

  "compound": {
    group: "object_kind",
    signature: "compound { object <objet>… }",
    summary:
      "Regroupe plusieurs objets en un seul, chacun gardant son propre matériau. Le bloc peut " +
      "être vide.",
    params: [{ name: "objets", type: "liste", doc: "zéro ou plusieurs `object`" }],
    next: "object",
    example: "object compound {\n  object simple\n    sphere 1.0\n    lambertian color 1 1 1\n}",
    source: "src/loader/parser.rs:104",
    snippet: "compound {\n  object $0\n}",
  },

  // ------------------------------------------------------------------- formes

  "sphere": {
    group: "shape",
    signature: "sphere <radius>",
    summary: "Sphère centrée sur l'origine de son repère. Un `transform` la déplace.",
    params: [{ name: "radius", type: "nombre", doc: "rayon" }],
    arity: 1,
    next: "material",
    example: "sphere 1.0",
    source: "src/shapes/sphere.rs",
  },

  "rectangle": {
    group: "shape",
    signature: "rectangle <width> <height>",
    summary:
      "Quadrilatère dans le plan **y = 0**, centré sur l'origine, normale vers **+y**. `width` " +
      "s'étend le long de x, `height` le long de z.\n\n" +
      "Les deux nombres sont les dimensions **totales** : `rectangle 555 555` mesure bien 555 de " +
      "côté. Le nœud d'AST les nomme `half_width`/`half_height`, mais `Rectangle::new` les divise " +
      "par deux — c'est le nœud qui est mal nommé, pas le langage.",
    params: [
      { name: "width", type: "nombre", doc: "dimension totale selon x" },
      { name: "height", type: "nombre", doc: "dimension totale selon z" },
    ],
    arity: 2,
    next: "material",
    example: "rectangle 555.0 555.0\nlambertian color 1.0 1.0 1.0",
    source: "src/shapes/rectangle.rs:16",
  },

  "plane": {
    group: "shape",
    signature: "plane",
    summary:
      "Plan infini **y = 0**, normale vers **+y**. Seule forme sans paramètre. Comme solide, il " +
      "vaut le demi-espace situé au-dessous, ce qui le rend utilisable dans un `csg`.",
    params: [],
    arity: 0,
    next: "material",
    example: "plane\nlambertian checkerboard 0.2 0.3 0.1 0.9 0.9 0.9 2000.0",
    source: "src/shapes/plane.rs",
  },

  "cylinder": {
    group: "shape",
    signature: "cylinder <radius> <height>",
    summary:
      "Cylindre d'axe **y**, centré sur l'origine : il s'étend de `-height/2` à `+height/2`. " +
      "`height` est la hauteur **totale**.",
    params: [
      { name: "radius", type: "nombre", doc: "rayon" },
      { name: "height", type: "nombre", doc: "hauteur totale" },
    ],
    arity: 2,
    next: "material",
    example: "cylinder 0.5 2.0",
    source: "src/shapes/cylinder.rs:16",
  },

  "aabox": {
    group: "shape",
    signature: "aabox <x> <y> <z>",
    summary:
      "Boîte alignée sur les axes, centrée sur l'origine. Les trois nombres sont les dimensions " +
      "**totales** : `aabox 165 330 165` s'étend de `-82.5` à `+82.5` en x.",
    params: [{ name: "dimensions", type: "vecteur", doc: "largeur, hauteur, profondeur totales" }],
    arity: 3,
    next: "material",
    example: "aabox 165.0 330.0 165.0",
    source: "src/shapes/aabox.rs:15",
  },

  "mesh": {
    group: "shape",
    signature: 'mesh file "<chemin.ply>" [reverse]',
    summary:
      "Maillage de triangles lu dans un fichier PLY. Le chemin est résolu depuis le **répertoire " +
      "courant du rendu**, pas depuis le fichier `.stage` — d'où les `\"test_files/…\"` des " +
      "scènes d'exemple. Aucune mise à l'échelle n'étant disponible, le maillage arrive à la " +
      "taille où il a été exporté.",
    params: [
      { name: "file", type: "chaîne", doc: "chemin du `.ply`, relatif au répertoire courant" },
      { name: "reverse", type: "drapeau", doc: "optionnel — retourne les faces" },
    ],
    next: "file",
    example: 'mesh file "test_files/cube.ply" reverse',
    source: "src/loader/mesh_loader.rs:13",
    snippet: 'mesh file "${1:test_files/bunny.ply}"',
  },

  "file": {
    group: "label",
    signature: 'file "<chemin.ply>"',
    summary:
      "Étiquette obligatoire du chemin d'un `mesh`. La chaîne n'admet **aucun échappement** et ne " +
      "peut pas être vide : `\"\"` arrête le lexer sans un mot.",
    params: [{ name: "chemin", type: "chaîne", doc: "relatif au répertoire courant" }],
    next: "reverse",
    example: 'file "test_files/dragon.ply"',
    source: "src/loader/parser.rs:215",
  },

  "reverse": {
    group: "flag",
    signature: "reverse",
    summary:
      "Retourne l'orientation de chaque face du maillage. Un PLY ne porte aucune convention " +
      "d'orientation : un modèle exporté en sens horaire arrive retourné, et ce drapeau est le " +
      "correctif. Symptôme : un objet noir ou éclairé à l'envers.",
    params: [],
    next: "material",
    example: 'mesh file "test_files/cube.ply" reverse',
    source: "src/loader/mesh_loader.rs:9",
  },

  "csg": {
    group: "shape",
    signature: "csg <union|intersection|substraction> { elem… }",
    summary:
      "Géométrie de construction : combine des formes en une seule. Chaque membre est un `elem` " +
      "portant sa forme et son `transform`.",
    params: [{ name: "opération", type: "union | intersection | substraction", doc: "combinaison appliquée" }],
    next: "csg_op",
    example: "csg union {\n  elem\n    sphere 0.4\n    transform {\n      translate -0.5 0 -0.5\n    }\n}",
    source: "src/shapes/csg.rs",
    snippet: "csg ${1|union,intersection,substraction|} {\n  elem\n    ${2:sphere 0.4}\n    transform {\n      $0\n    }\n}",
  },

  "union": {
    group: "csg_op",
    signature: "union { elem… }",
    summary: "Réunion des volumes : un point appartient au résultat s'il appartient à au moins un membre.",
    params: [],
    next: "elem",
    example: "csg union {\n  elem sphere 0.4 transform { translate -0.5 0 0 }\n  elem sphere 0.4 transform { translate 0.5 0 0 }\n}",
    source: "src/shapes/csg/union.rs",
  },

  "intersection": {
    group: "csg_op",
    signature: "intersection { elem… }",
    summary: "Intersection des volumes : un point n'appartient au résultat que s'il appartient à **tous** les membres.",
    params: [],
    next: "elem",
    example: "csg intersection {\n  elem sphere 0.5 transform { }\n  elem aabox 0.8 0.8 0.8 transform { }\n}",
    source: "src/shapes/csg/intersection.rs",
  },

  "substraction": {
    group: "csg_op",
    signature: "substraction { elem… }",
    summary:
      "Le **premier** `elem` moins tous les suivants. L'ordre compte donc, contrairement à " +
      "`union` et `intersection`.\n\n" +
      "Le mot est orthographié à la française ; la forme anglaise `subtraction` **n'est pas " +
      "reconnue** (dette suivie dans `IDEAS.md`).",
    params: [],
    next: "elem",
    example: "csg substraction {\n  elem aabox 1 1 1 transform { }\n  elem sphere 0.65 transform { }\n}",
    source: "src/shapes/csg/substraction.rs:23",
  },

  // ---------------------------------------------------------------- matériaux

  "lambertian": {
    group: "material",
    signature: "lambertian <texture>",
    summary: "Diffus parfait : la lumière repart dans toutes les directions selon un cosinus. Le matériau à tout faire.",
    params: [{ name: "texture", type: "texture", doc: "albédo — `color` ou `checkerboard`" }],
    next: "texture",
    example: "lambertian color 0.65 0.05 0.05",
    source: "src/materials/lambertian.rs",
  },

  "metal": {
    group: "material",
    signature: "metal <fuzz> <texture>",
    summary:
      "Réflexion spéculaire. Le flou vient **avant** la texture. À `fuzz` nul, c'est un miroir " +
      "parfait ; au-delà, la direction réfléchie est déplacée d'un vecteur de longueur `fuzz`.",
    params: [
      { name: "fuzz", type: "nombre", doc: "flou de la réflexion ; 0 = miroir" },
      { name: "texture", type: "texture", doc: "albédo" },
    ],
    leadingNumbers: 1,
    next: "texture",
    example: "metal 0.3 color 0.8 0.8 0.9",
    source: "src/materials/metal.rs:18",
  },

  "dielectric": {
    group: "material",
    signature: "dielectric <index> <texture>",
    summary:
      "Transmetteur : réfraction et réflexion de Fresnel. L'indice vient **avant** la texture. " +
      "Verre ≈ 1.5, eau ≈ 1.33, diamant ≈ 2.4.\n\n" +
      "Un transmetteur dans un autre est aujourd'hui rendu faux — voir `ideas/nested_dielectrics.md`.",
    params: [
      { name: "index", type: "nombre", doc: "indice de réfraction" },
      { name: "texture", type: "texture", doc: "teinte de la transmission" },
    ],
    leadingNumbers: 1,
    next: "texture",
    example: "dielectric 1.5 color 1 1 1",
    source: "src/materials/dielectric.rs",
  },

  "diffuse_light": {
    group: "material",
    signature: "diffuse_light <texture>",
    summary:
      "Surface émissive, et **seule** façon de déclarer de la lumière dans le langage — il n'y a " +
      "pas de production `light`. La texture est une radiance, non bornée à 1 : une source " +
      "s'écrit `color 15 15 15`.\n\n" +
      "Attention : une telle surface n'est pas encore une source échantillonnable, donc elle " +
      "n'éclaire rien par voie indirecte (`ideas/area_light.md`).",
    params: [{ name: "texture", type: "texture", doc: "radiance émise" }],
    next: "texture",
    example: "diffuse_light color 15.0 15.0 15.0",
    source: "src/materials/diffuse_light.rs",
  },

  // ----------------------------------------------------------------- textures

  "color": {
    group: "texture",
    signature: "color <r> <g> <b>",
    summary:
      "Couleur uniforme, en RVB **linéaire** — pas de correction gamma, pas de bornes. Des " +
      "valeurs au-dessus de 1 n'ont de sens que pour une émission.",
    params: [{ name: "rvb", type: "spectre", doc: "trois nombres, rouge vert bleu" }],
    arity: 3,
    next: "end_of_object",
    example: "color 0.65 0.05 0.05",
    source: "src/textures/plain_color.rs",
  },

  "checkerboard": {
    group: "texture",
    signature: "checkerboard <r1 g1 b1> <r2 g2 b2> <scale>",
    summary:
      "Damier des deux couleurs sur les coordonnées (u, v) de la surface. `scale` multiplie ces " +
      "coordonnées : une case mesure `0.5 / scale`, donc **plus `scale` est grand, plus les cases " +
      "sont petites**. Les (u, v) étant en unités du monde pour un plan ou un rectangle, `scale` " +
      "se règle d'après la taille de la scène — `2000.0` sur un sol de 1000, `0.002` sur un dragon.",
    params: [
      { name: "couleur 1", type: "spectre", doc: "trois nombres" },
      { name: "couleur 2", type: "spectre", doc: "trois nombres" },
      { name: "scale", type: "nombre", doc: "fréquence ; case = 0.5 / scale" },
    ],
    arity: 7,
    next: "end_of_object",
    example: "checkerboard\n  0.65 0.0 0.0\n  0.65 0.65 0.65\n  2.0",
    source: "src/textures/checker_board.rs:19",
    snippet: "checkerboard ${1:0.2 0.3 0.1} ${2:0.9 0.9 0.9} ${3:2.0}",
  },

  // ------------------------------------------------------- étapes de placement

  "translate": {
    group: "transform_step",
    signature: "translate <x> <y> <z>",
    summary: "Déplace la forme. Dans un bloc `transform`, la translation s'écrit **après** les rotations qu'elle doit suivre.",
    params: [{ name: "décalage", type: "vecteur", doc: "trois nombres" }],
    arity: 3,
    next: "transform_step",
    example: "transform {\n  rotate_y -0.2618\n  translate 80.0 -150.0 -100.0\n}",
    source: "src/loader/parser.rs:299",
  },

  "rotate_x": {
    group: "transform_step",
    signature: "rotate_x <angle>",
    summary:
      "Rotation autour de l'axe x, en **radians**. `3.14159` retourne une forme d'un demi-tour — " +
      "c'est ainsi qu'on fait pointer le plafond d'une Cornell box vers le bas.",
    params: [{ name: "angle", type: "nombre", doc: "radians ; 1.5708 vaut 90°" }],
    arity: 1,
    next: "transform_step",
    example: "rotate_x -3.14159",
    source: "src/loader/parser.rs:299",
  },

  "rotate_y": {
    group: "transform_step",
    signature: "rotate_y <angle>",
    summary: "Rotation autour de l'axe y, en **radians**. `-0.2618` vaut -15°.",
    params: [{ name: "angle", type: "nombre", doc: "radians ; 1.5708 vaut 90°" }],
    arity: 1,
    next: "transform_step",
    example: "rotate_y -0.2618",
    source: "src/loader/parser.rs:299",
  },

  "rotate_z": {
    group: "transform_step",
    signature: "rotate_z <angle>",
    summary: "Rotation autour de l'axe z, en **radians**.",
    params: [{ name: "angle", type: "nombre", doc: "radians ; 1.5708 vaut 90°" }],
    arity: 1,
    next: "transform_step",
    example: "rotate_z 3.14159",
    source: "src/loader/parser.rs:299",
  },
};

// Keywords of a given group, in the order the language reads best.
function group(name) {
  return Object.keys(PRODUCTIONS).filter((kw) => PRODUCTIONS[kw].group === name);
}

module.exports = { PRODUCTIONS, group };
