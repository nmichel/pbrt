# pbrt — directives de projet

Renderer *physically based* écrit en Rust. **Projet d'apprentissage** : la structure, le
découplage des concepts et la lisibilité passent avant la performance. L'objectif à long terme
est de produire des images de haute qualité en restant au plus près du modèle physique.

Quand un arbitrage se présente entre « plus rapide » et « plus clair / plus juste
physiquement », choisir la clarté et la justesse, et signaler le coût en performance.

## 0. État des lieux

[IDEAS.md](IDEAS.md) tient la liste des défauts connus et l'ordre de travail retenu, avec les
références précises dans le code. **Le consulter avant de proposer un chantier**, et y cocher
ou amender les entrées au fur et à mesure — c'est la mémoire longue du projet.

Un sujet reste une ligne dans `IDEAS.md` tant qu'une ligne suffit ; dès qu'il porte une analyse,
il prend son propre fichier dans [ideas/](ideas/) et `IDEAS.md` n'en garde que l'entrée d'index.
Ce répertoire n'est pas `docs/` : `docs/` décrit le code tel qu'il est, `ideas/` ce qui n'est pas
fait. Le fichier d'un sujet disparaît quand le sujet atterrit.

**Chantier en cours** — aucun. Le sujet de la branche `chore/revamp_bvh_for_trimesh` est clos :
SAH de maillage corrigé, `intersect_p` descendu dans les formes, arbre de scène à plat, traversée
ordonnée avec resserrement de l'intervalle. L'ordre de travail d'`IDEAS.md` désigne `AreaLight`
comme suite — les surfaces émissives ne contribuent aujourd'hui à aucun éclairage indirect.

Trois chantiers restent *voisins* du BVH et n'en font délibérément pas partie, chacun documenté
dans `IDEAS.md` : `get_bounding_box` recalculé à chaque comparaison (à traiter avant qu'une scène
grossisse), les feuilles de maillage à un seul triangle, et le SAH binné de la scène, mis de côté
avec ses raisons.

**Lancer les tests** — `cargo test` est vert en entier : 74 tests de bibliothèque, 8 doc-tests, et
les `examples/` compilent. Y ajouter `cargo fmt --check`. Pour tout changement de
construction ou de traversée d'un accélérateur, `cargo run --release --bin bvh_stats -- <scène>`
donne les compteurs à comparer ; ils ne voient que les rayons primaires et d'ombre, donc le
chronomètre d'un rendu complet reste une mesure distincte et parfois divergente.

## 1. Structure

Un module = un concept. L'idiome en vigueur dans tout le projet, à respecter pour tout
nouveau module :

```
src/materials.rs        → le trait Material, les types partagés (ScatterInfo), les `pub use`
src/materials/*.rs      → une implémentation par fichier, module privé, re-exportée
```

Le fichier « module » doit se lire comme une **interface** : le trait, sa documentation, les
types qu'il échange, rien d'autre. Les fichiers enfants sont les implémentations. Cet idiome
est appliqué à `shapes`, `objects`, `materials`, `lights`, `textures`, `cameras`,
`integrators`, `pdfs`, `renderers` — s'y conformer plutôt qu'introduire une variante.

Corollaires :
- Une implémentation par fichier. Si un fichier contient deux concepts, il faut le scinder.
- Pas de `pub` sur les modules d'implémentation ; l'accès passe par le re-export.
- Le nom du fichier est le nom du concept, en `snake_case` (`thin_lens.rs` → `ThinLensCamera`).

## 2. Découplage

Les traits sont les coutures du système, et chaque couture doit rester franche :

| Trait | Responsabilité, et rien d'autre |
|---|---|
| `Intersectable` | géométrie pure : toutes les intersections d'un rayon, `contain_point` |
| `AABound` | fournir une AABB pour les structures d'accélération |
| `Shape` | `Intersectable + AABound` — une forme, sans matériau |
| `Object` | associer géométrie et matériau, produire une `Interaction` |
| `Material` | échantillonner (`scatter`) et évaluer (`f`) une BSDF, émettre (`emit`) |
| `Light` | échantillonner une source (`sample_li`), radiance incidente (`le`) |
| `Texture` | valeur spectrale en un point de surface |
| `Camera` | générer le rayon d'un pixel |
| `Integrator` | résoudre l'équation de transport |
| `Pdf` | échantillonner une direction et donner sa densité |

Règles qui découlent de ce découpage :
- **Une `Shape` ne connaît jamais de matériau**, un `Material` jamais de géométrie concrète.
- **L'intégrateur ne connaît pas le renderer** : il ne sait rien des pixels, des threads ni du
  film. Réciproquement, un renderer ne fait aucun choix de transport de lumière.
- Le chargement de scène est isolé derrière le patron **Visitor** (`loader/visitors.rs`) :
  l'AST ne construit rien, les visiteurs ne parsent rien. Ajouter une primitive au langage
  `.stage` implique de traverser lexer → parser → nœud d'AST → méthode de `Visitor` → les deux
  visiteurs. Ne pas court-circuiter cette chaîne.
- Une nouvelle variante d'un concept existant s'ajoute par une implémentation de trait, pas par
  un `match` ou un `enum` dans le code appelant.

## 3. Lisibilité

- Le formatage est fixé par [rustfmt.toml](rustfmt.toml) — `max_width = 150`,
  `control_brace_style = "ClosingNextLine"` (le `else` est donc sur sa propre ligne, après
  l'accolade fermante). Lancer `cargo fmt --check` avant de conclure ; ne pas reformater du
  code que l'on ne modifie pas.
- **Nommer d'après le domaine, pas d'après l'implémentation** : `wo`, `wi`, `beta`,
  `ni_over_nt`, `abs_cos_theta` — la nomenclature de la littérature PBR (pbrt, PBR Book) est
  la convention, y compris ses notations courtes quand elles sont standard.
- Préférer une étape intermédiaire nommée à une expression dense. Les grandeurs physiques
  méritent un nom même utilisées une seule fois.
- Les commentaires et doc-comments du code sont **en anglais**, comme le reste du code.
- Éviter le `unsafe`. Il n'a aujourd'hui aucune justification dans ce projet ; toute
  introduction doit être argumentée par une mesure, pas par une intuition.
- Pas de constante magique nue : une tolérance, un epsilon, un décalage anti-acné doit être
  nommé et son choix expliqué.

## 4. Documenter les concepts — « presque du literate programming »

**C'est la directive la plus importante de ce fichier.** Dès qu'une fonction ou un module
implémente un concept physique ou mathématique, le code doit porter sa propre dérivation. Un
lecteur doit pouvoir comprendre *pourquoi* la formule est celle-là sans ouvrir un livre.

Un bloc de documentation de concept contient, dans cet ordre :

1. **La référence** — lien vers le PBR Book, un article, un cours, avec la section précise.
2. **Le cadre** — les repères et conventions utilisés (repère local où `z` est la normale,
   sens de `wo`, unités, quel côté de l'interface est ηᵢ…). La plupart des erreurs de rendu
   viennent d'une convention implicite : l'énoncer explicitement.
3. **La dérivation** — les étapes du calcul, en notation mathématique Unicode (θ, φ, π, δ, ηᵢ,
   ηₜ, α², √, ⋅), chaque étape numérotée `[1]`, `[2]`… quand une étape ultérieure s'y réfère,
   et l'argument invoqué annoté en marge (`[Chain Rule]`).
4. **Le lien avec le code** — quand la source est un article numéroté, référencer ses équations
   au point d'usage (`// (27a)`, `// (29a)`).

Exemples de référence à imiter, déjà dans le projet :

- [shapes/sphere.rs](src/shapes/sphere.rs) — dérivation complète du passage en coordonnées
  sphériques, du mapping (u, v) et des dérivées ∂p/∂u, ∂p/∂v par la règle de chaîne. C'est le
  mètre-étalon du niveau de détail attendu.
- [materials/dielectric.rs](src/materials/dielectric.rs) — réfraction et Fresnel, avec le lien
  vers l'article source et le renvoi à ses équations numérotées ; explicite le raisonnement
  « entre-t-on ou sort-on du volume » avant de l'implémenter.
- [geom/intersectable.rs](src/geom/intersectable.rs) — `world_to_local` / `local_to_world`
  posent la matrice de changement de repère en commentaire et justifient
  `inv(M) == transpose(M)` avant de l'exploiter.
- [materials/metal.rs](src/materials/metal.rs) — dérive la réflexion dans le repère local
  jusqu'à `[-wox, -woy, woz]` au lieu de livrer le résultat brut.
- [integrators/path.rs](src/integrators/path.rs) — justifie le traitement de la radiance de
  fond après un rebond spéculaire par un renvoi au PBR Book.

Deux exigences supplémentaires :

- **Documenter les écarts au modèle physique.** Toute approximation, tout biais, tout hack
  doit être signalé comme tel, avec sa raison et sa conséquence sur l'image. Exemple existant :
  la division par `|cos θ|` dans les BRDF spéculaires, qui compense le cosinus appliqué par
  l'intégrateur à une distribution de Dirac — le commentaire dit *pourquoi* et cite la source.
- **Les tests font partie de la documentation.** Les tests de conservation d'énergie de
  [pdfs/cosine.rs](src/pdfs/cosine.rs) et [pdfs/hemisphere.rs](src/pdfs/hemisphere.rs)
  démontrent qu'un estimateur Monte-Carlo est non biaisé. Toute nouvelle BSDF ou pdf doit venir
  avec un test de cette nature ; c'est la seule façon de vérifier la justesse physique.
