# RNG graine — ce qui reste à brancher

Indexé depuis [IDEAS.md](../IDEAS.md). **En cours** sur la branche `feat/rework_sampling` : le
sampler existe, rien ne l'appelle encore.

Ce qui a atterri est décrit par [docs/rendu_reproductible.md](../docs/rendu_reproductible.md) et
par les doc-comments de [`samplers.rs`](../src/samplers.rs) — ce fichier-ci ne le répète pas. Il ne
tient que le travail restant, ses décisions, et ce qu'on a écarté en chemin pour ne pas le
reprendre à zéro.

L'objectif est étroit : **une même scène, mêmes options de rendu, même image**, pour le même
binaire sur la même machine. Deux sujets l'attendent nommément, la validation d'
[`AreaLight`](area_light.md) et le balayage de `t_trav`
([cout_traversee_bvh.md](cout_traversee_bvh.md) §4).

## 0. Où en est le chantier

- [x] **Échantillonner par inversion et non par rejet.** Les deux boucles de rejet consommaient un
      nombre de tirages variable ; leurs applications vivent désormais à côté de leur unique
      appelant. Un seul écart d'apparence, documenté dans
      [`metal.rs`](../src/materials/metal.rs).
- [x] **`rand` 0.3 → 0.10.** Mécanique, et le lock perd le doublon 0.3/0.4.
- [x] **Le trait `Sampler` et `IndependentSampler`**, avec quatre tests à graine fixe, donc aucun ne
      peut échouer par malchance.
- [ ] **Le câblage** — §3 et §4 ci-dessous. C'est là que le déterminisme atterrit.
- [ ] **Les tests qui en dépendent** — §5.

## 1. Ce qui tire encore d'un générateur global

C'est la liste de travail du commit de câblage. Tout passe par `utils::random_double`, sauf les
deux `Sampler2`.

| Site | Tirages | Devient |
|---|---|---|
| [pdfs/cosine.rs:16-17](../src/pdfs/cosine.rs#L16) | 2 | un paramètre `u` de `generate` |
| [pdfs/hemisphere.rs:16-17](../src/pdfs/hemisphere.rs#L16) | 2 | idem |
| [pdfs/sphere.rs:16-17](../src/pdfs/sphere.rs#L16) | 2 | idem |
| [materials/dielectric.rs:88](../src/materials/dielectric.rs#L88) | 1 | `sampler.get_1d()` |
| [integrators/path.rs:38](../src/integrators/path.rs#L38) | 1 | `sampler.get_1d()` |
| [integrators/path.rs:96](../src/integrators/path.rs#L96) | 1 | idem, mais la ligne est commentée |
| [cameras/thin_lens.rs:107](../src/cameras/thin_lens.rs#L107) | 2 | le `p_lens` d'un `CameraSample` |
| `Sampler2`, [st.rs:74](../src/renderers/st.rs#L74) et [mt.rs:138](../src/renderers/mt.rs#L138) | 2 | le `p_film` d'un `CameraSample` |
| [examples/test_scene.rs](../examples/test_scene.rs#L97), [test_scene_cube.rs](../examples/test_scene_cube.rs#L80) | 11 chacun | un `IndependentSampler` local — §5 |

Quand la colonne de droite est faite, `utils.rs` n'a plus d'habitant et disparaît, avec sa ligne
de [lib.rs](../src/lib.rs).

**Ce qui ne tire rien**, et ne doit pas commencer : la construction et la traversée des deux BVH,
`bvh_stats`, `NormalIntegrator`. C'est un acquis du chantier BVH, obtenu en retirant précisément un
`random_double()` de `choose_comparator` ([docs/mesures_bvh.md](../docs/mesures_bvh.md) §3.2).

## 2. La conception retenue, et ce qu'elle a écarté

Le trait est **deux méthodes**, `get_1d` et `get_2d`, et ne dit rien des graines, des pixels ni des
index d'échantillon : ce sont les paramètres d'un sampler *particulier*, donc de son constructeur.
Un appelant tenant `&mut dyn Sampler` demande des nombres et n'apprend rien d'autre.

**Un sampler par (pixel, échantillon), construit sur la pile.** La portée d'un sampler est celle
d'un échantillon : il naît dans la boucle de `compute_pixel` et meurt avec le chemin. Concrètement
un `IndependentSampler` concret sur la pile, dont on passe `&mut` coercé en `&mut dyn Sampler` —
donc aucune allocation malgré les 2,4 millions de constructions d'une image 800×600 à 5
échantillons, et aucune indirection à la construction.

Trois choses ont été retirées de l'interface au passage, et voici pourquoi, pour que la question ne
se repose pas :

- **`start_pixel_sample`** — il n'existait que pour annoncer à un sampler stratifié qu'un nouvel
  échantillon commence. Avec une construction par échantillon, le constructeur reçoit
  `sample_index` et l'annonce est superflue.
- **`fork`** — il n'existait que parce qu'un sampler vivait aussi longtemps qu'un thread. Il n'en
  traverse plus aucun ; même `Send` est inutile.
- **`samples_per_pixel`** — il n'existait que pour l'arrondi au carré parfait du stratifié. Voir
  la note de fin.

**Et aucune fabrique, aucun `enum Type`.** Avec une seule implémentation il n'y a rien à
dispatcher, et la question ne se pose qu'avec la seconde. C'est aussi ce qui évite de rejouer les
seize `match config.integrator` : le jour venu, le choix se fait à l'unique ligne qui construit,
dans `compute_pixel`.

## 3. Faire sortir le sampler des feuilles plutôt que de l'y faire entrer

Cinq traits tirent aujourd'hui. Deux n'ont pas besoin d'un sampler, et le dire améliore la couture :

- **`Pdf::generate(&self, u: &Vector2f)`** — une pdf est une application du carré unité vers les
  directions, et les trois implémentations veulent exactement 2 nombres. `pdfs` ne dépend alors pas
  de `samplers`, et un test peut nourrir la pdf d'une grille fixe. C'est aussi ce qui rend vrai que
  le stratifié ne toucherait qu'une ligne : sans ça, il toucherait les trois pdfs.
- **`Camera::get_ray(&self, sample: &CameraSample)`** avec `CameraSample { p_film, p_lens }`, comme
  pbrt-v4. Absorbe la gigue que `st` et `mt` ajoutent à la main — donc déduplique — et laisse
  `bvh_stats` sans sampler : il passe le centre du pixel et un point de lentille fixe, ce qui
  préserve la propriété qu'aucun de ses rayons ne dépend d'un tirage.

Restent trois signatures à élargir avec `&mut dyn Sampler`, et c'est irréductible :
`Material::scatter`, `Light::sample_li`, `Integrator::li`.

Deux placements rejetés. Un `thread_local!` restaure le déterminisme mais cache la dépendance,
contre CLAUDE.md §2 — un `Material` qui tire doit le dire dans sa signature. Un sampler dans
`Scene` (partagée immuablement entre threads) ou dans `Interaction` (un relevé de géométrie et de
matériau) fait porter à ces types une responsabilité qui n'est pas la leur.

**Sites d'appel** : `get_ray` 4, `li` 3 vivants — `whitted.rs` est hors module —, `scatter` 2,
`sample_li` 1, `generate` 5. Une quinzaine. **Aucun des 15 `examples/` ne change** : ils appellent
`render_function`, et le sampler est construit dans le renderer.

## 4. Le commit de câblage

- `Config::seed: u64` et `--seed`. Défaut : une constante, **pas** l'horloge — le déterminisme est
  le comportement normal, pas une option.
- `compute_pixel`, dans les deux renderers : la boucle prend un index qui monte plutôt que le `ns`
  qui descend, et construit un `IndependentSampler` par tour.
- `CameraSample`, `Pdf::generate(u)`, les trois signatures.
- `utils.rs` et les deux `Sampler2` disparaissent.

## 5. Comment on vérifie

- Deux exécutions de la même commande → **PNG identiques octet pour octet**.
- `--threads 1` contre `--threads 8` → **PNG identiques**. C'est la garantie forte, et un test de
  justesse du renderer MT que rien ne fait aujourd'hui.
- `--samples_ppx 4` contre `--samples_ppx 8` : l'image de 4 est le préfixe exact de celle de 8
  ([docs/rendu_reproductible.md](../docs/rendu_reproductible.md) §4).
- Graine sur les deux tests de conservation d'énergie. Celui de
  [hemisphere.rs](../src/pdfs/hemisphere.rs#L41) est un vrai estimateur Monte-Carlo, à 5,5 σ de sa
  tolérance ; celui de [cosine.rs](../src/pdfs/cosine.rs#L42) est **de variance nulle** — sa
  contribution vaut 1 exactement pour chaque échantillon, donc il vérifie une identité algébrique
  et non une convergence. À dire dans son commentaire en y touchant.
- Les deux `examples/test_scene*` prennent un sampler à graine fixe, ce qui ferme le défaut de
  géométrie non reproductible de [docs/mesures_bvh.md](../docs/mesures_bvh.md) §2.3.

## 6. Note de fin : le sampler stratifié

Écarté de ce chantier, et non pas oublié. Ce qui le rendrait possible est en grande partie déjà là
— construction par (pixel, échantillon), tirages en nombre fixe, pdfs nourries d'un `u`. Restent :

- une permutation par dimension, engendrée à la volée depuis un hash du numéro de dimension, sinon
  les N échantillons s'alignent sur la diagonale de l'hypercube ;
- un compteur de dimension interne au sampler ;
- `samples_per_pixel()` sur le trait **si** l'implémentation arrondit au carré parfait que veut une
  grille √N × √N, ou deux paramètres à la construction comme pbrt, qui évite l'arrondi ;
- l'application concentrique du disque en remplacement de la polaire de
  [`thin_lens.rs`](../src/cameras/thin_lens.rs) — elle distord moins le carré unité, donc préserve
  mieux la structure d'un échantillon stratifié. Sans conséquence tant que les tirages sont
  indépendants.

Et sa limite, à énoncer le jour où il arrive : stratifier chaque dimension séparément donne un
hypercube latin, pas une stratification conjointe de l'espace des chemins. Le bénéfice est réel sur
la gigue, la lentille et le premier sommet, et décroît ensuite (PBR Book §8.6). Il se dégrade là où
les chemins d'un même pixel divergent — le branchement de `dielectric.rs`, et les profondeurs
inégales — ce qui affaiblit la garantie d'étalement sans jamais introduire de biais.
