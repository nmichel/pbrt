# RNG graine et samplers stratifiés — un rendu qui se répète

Indexé depuis [IDEAS.md](../IDEAS.md). Non fait, et **premier de la liste de tête** : deux sujets
l'attendent nommément, la validation d'[`AreaLight`](area_light.md) et le balayage de `t_trav`
([cout_traversee_bvh.md](cout_traversee_bvh.md) §4).

L'objectif est étroit et se dit en une phrase : **une même scène, mêmes options de rendu, même
image**. Tout le reste de ce fichier est ce qu'il faut décider pour que cette phrase soit vraie sans
condition cachée.

## 1. L'état des lieux — tous les tirages du dépôt

Un seul générateur existe : le `thread_rng` global de `rand 0.3`, atteint par deux routes
indépendantes.

**Route 1 — [utils.rs](../src/utils.rs)**, `random_double()` = `rand::random::<f64>()`, sans graine
possible.

| Site | Tirages | Nature |
|---|---|---|
| [pdfs/cosine.rs:16-17](../src/pdfs/cosine.rs#L16) | 2 | analytique |
| [pdfs/hemisphere.rs:16-17](../src/pdfs/hemisphere.rs#L16) | 2 | analytique |
| [pdfs/sphere.rs:16-17](../src/pdfs/sphere.rs#L16) | 2 | analytique |
| [materials/dielectric.rs:88](../src/materials/dielectric.rs#L88) | 1 | choix réflexion / réfraction |
| [integrators/path.rs:38](../src/integrators/path.rs#L38) | 1 | choix de la lumière |
| [integrators/path.rs:96](../src/integrators/path.rs#L96) | 1 | roulette russe, commentée |
| `random_in_unit_disk` → [thin_lens.rs:67](../src/cameras/thin_lens.rs#L67) | **2 par essai, ~2,55 en moyenne, non borné** | rejet |
| `random_in_unit_sphere` → [metal.rs:40](../src/materials/metal.rs#L40) | **3 par essai, ~5,73 en moyenne, non borné** | rejet |
| `random_unit_vector` | — | **jamais appelé** |

Les deux espérances viennent des taux d'acceptation : π/4 pour le disque dans le carré, π/6 pour la
boule dans le cube.

**Route 2 — `Sampler2`**, dupliqué à l'identique dans [st.rs:69-87](../src/renderers/st.rs#L69) et
[mt.rs:133-151](../src/renderers/mt.rs#L133) : un `ThreadRng` et un `Range<f64>`, deux tirages pour
la gigue du pixel. Un `Sampler2::new()` par thread dans `mt`.

**Hors renderer.** [examples/test_scene.rs](../examples/test_scene.rs#L97) et
[test_scene_cube.rs](../examples/test_scene_cube.rs#L80) tirent leur *géométrie* de `random_double()`
— le défaut déjà nommé dans [docs/mesures_bvh.md](../docs/mesures_bvh.md) §4, et la raison pour
laquelle `many_spheres.stage` existe.

**Dans les tests.** Les deux tests de conservation d'énergie tirent 100 000 échantillons non graine,
et ils ne sont pas dans le même état :

- [hemisphere.rs](../src/pdfs/hemisphere.rs#L41) est un vrai estimateur Monte-Carlo. Sa contribution
  est `2 cos θ`, donc `Var = 1/3` et l'erreur type vaut 0,0018 pour 100 000 échantillons : la
  tolérance de 0,01 est à 5,5 σ. Sûr en pratique, mais c'est une probabilité, pas une garantie.
- [cosine.rs](../src/pdfs/cosine.rs#L42) est **de variance nulle**. La contribution vaut
  `(1/π)·cos θ / (|z|/π) = 1` exactement, pour chaque échantillon. Ce test ne démontre pas une
  convergence, il vérifie une identité algébrique — ce qui est utile, et n'est pas ce que son nom
  annonce. À dire dans son commentaire quand on y touchera.

**Ordre de grandeur.** Sous `PATH`, `max_depth 3`, caméra sténopé, matériaux lambertiens, un rayon
caméra qui ne s'échappe pas consomme 2 tirages de gigue puis deux itérations à 3 tirages — la
troisième touche sort de la boucle avant l'échantillonnage de lumière. Soit 8, majoré.

## 2. Ce qui ne tire rien, et n'est donc pas concerné

La construction et la traversée des deux BVH, `bvh_stats`, `NormalIntegrator`, et toutes les lumières
sauf par `SpherePdf`. C'est un acquis du chantier BVH, obtenu en retirant précisément un
`random_double()` de `choose_comparator` ([docs/mesures_bvh.md](../docs/mesures_bvh.md) §3.2), et il
ne faut pas le reperdre : le sampler ne doit pas entrer dans un accélérateur.

## 3. Graine par (pixel, échantillon), pas par thread

Un RNG graine par thread suffirait *aujourd'hui* : le tourniquet de
[mt.rs:73](../src/renderers/mt.rs#L73) est déterministe, chaque thread reçoit ses pixels dans un
ordre fixe, et chacun les traite en séquence. Mais l'image dépendrait alors de `--threads`, et le
chantier « Film + ordonnancement par tuiles » — qui veut de l'équilibrage de charge, donc un
ordonnancement décidé à l'exécution — la casserait le jour où il atterrit.

La forme juste est celle de pbrt : `start_pixel_sample(pixel, index)` **réamorce** le flux depuis
`hash(graine, pixel.x, pixel.y, index)`. Trois conséquences, dont la deuxième est un cadeau :

- l'ordonnancement futur est libre de tout réordonner, et la propriété survit au chantier des tuiles ;
- `--threads 1` et `--threads 8` rendent **la même image**, ce qui devient un test de justesse du
  renderer MT — quelque chose que rien ne vérifie aujourd'hui ;
- les deux `Sampler2` disparaissent, donc les ~50 lignes dupliquées entre `st` et `mt` diminuent sans
  qu'on les vise.

Le prix est d'un hash par échantillon, contre zéro pour un flux séquentiel par thread. Sur ~8 tirages
par échantillon, c'est du bruit.

## 4. Les boucles de rejet bloquent la stratification

Un sampler stratifié numérote ses dimensions : la première est la gigue, la deuxième la lentille, la
troisième le choix de lumière. Une boucle de rejet consomme un nombre de tirages **inconnu**, donc
décale toutes les dimensions suivantes d'une quantité qui dépend du pixel — la dimension « choix de
lumière » ne serait pas la même d'un pixel à l'autre, et la structure qu'on paie ne serait pas là.

Supprimer les deux rejets est donc un **prérequis**, pas un nettoyage de passage :

- `random_in_unit_disk` → l'application concentrique de pbrt sur un échantillon 2D. Exactement 2
  tirages, et moins de distorsion que la polaire naïve.
- `random_in_unit_sphere` → l'actuel rend un point *dans* la boule (`squared_length < 1`), pas *sur*
  la sphère, alors que `SpherePdf::generate` fait déjà le second. Garder la boule demande un rayon
  `r = u^(1/3)` — la racine cubique est ce qui redonne une densité uniforme en volume — donc 2 tirages
  de direction et 1 de rayon. C'est la version à retenir : elle laisse la distribution du lobe de
  `fuzz` inchangée, là où se contenter de la surface de la sphère la modifierait. Trancher autrement
  est défendable, mais alors c'est un écart et il se documente comme tel.
- `random_unit_vector` → supprimer, personne ne l'appelle.

Ces trois-là deviennent des fonctions **pures** d'un échantillon. Elles peuvent rester dans
[utils.rs](../src/utils.rs) — diff minimal, et elles y perdent leur couplage au RNG. Si elles se
multiplient, `src/sampling.rs` est le module qu'elles veulent.

## 5. Faire sortir le sampler des feuilles plutôt que de l'y faire entrer

Le réflexe est de passer `&mut dyn Sampler` aux cinq traits qui tirent. Deux d'entre eux n'en ont pas
besoin, et le dire améliore la couture :

- **`Pdf`** est une application du carré unité vers les directions. `generate(&self, u: &Vector2f)`
  dit ce qu'il est, et les trois implémentations veulent exactement 2 nombres — aucune ne veut un
  générateur. Bénéfice secondaire : les tests de conservation d'énergie choisissent alors leur source
  d'échantillons, fixe ou stratifiée.
- **`Camera`** : `get_ray(&self, sample: &CameraSample)` avec `CameraSample { p_film, p_lens }`, comme
  pbrt-v4. Cela absorbe la gigue que les deux renderers ajoutent aujourd'hui à la main, et surtout
  **`bvh_stats` reste sans tirage par construction** — il passe le centre du pixel et un point de
  lentille fixe, au lieu de devoir se fabriquer un sampler pour un appareil qu'il utilise à
  `lens_radius = 0`.

Il reste trois signatures à élargir, et c'est irréductible : `Material::scatter`, `Light::sample_li`,
`Integrator::li` prennent `&mut dyn Sampler`.

**Deux placements rejetés.** Un `thread_local!` restaure le déterminisme sous le §3 mais cache la
dépendance, contre CLAUDE.md §2 — un `Material` qui tire doit le dire dans sa signature. Et un sampler
dans `Scene` (partagée immuablement entre threads) ou dans `Interaction` (un relevé de géométrie et de
matériau, pas un porte-état mutable) fait porter à ces types une responsabilité qui n'est pas la leur.

**Sites d'appel réels** : `get_ray` 4, `li` 3 vivants — `whitted.rs` est hors module —, `scatter` 2,
`sample_li` 1, `generate` 5. Une quinzaine.

**Aucun des 15 `examples/` ne change.** Ils appellent `render_function`, et le sampler est construit
*dans* le renderer depuis `Config`. Ce qui évite au passage de répéter la faute des seize
`match config.integrator` : le `match` du sampler vit dans `samplers::Type::build`, un seul endroit.

## 6. Structure, et le choix du moteur

Conforme à CLAUDE.md §1 — un module = un concept, le fichier parent se lit comme une interface :

```
src/samplers.rs               → trait Sampler, enum Type, pub use
src/samplers/independent.rs   → IndependentSampler (le comportement actuel, mais graine)
src/samplers/stratified.rs    → StratifiedSampler (second temps)
```

```rust
pub trait Sampler: Send {
    fn start_pixel_sample(&mut self, pixel: &Vector2u, sample_index: usize);
    fn get_1d(&mut self) -> f64;
    fn get_2d(&mut self) -> Vector2f;
    fn fork(&self) -> Box<dyn Sampler>;
}
```

`Send` et non `Sync` : chaque thread possède le sien. `fork` plutôt que `clone_for_thread`, parce que
sous le §3 la graine ne dépend pas du thread — le fork ne porte aucune identité, et son nom ne doit
pas suggérer le contraire.

**Le moteur : `rand 0.10` et `rand_pcg 0.10`** (versions courantes vérifiées le 2026-08-20),
générateur `Pcg64Mcg` — sortie 64 bits, donc **un** tirage par `f64`, là où `Pcg32` en demande deux.

Le point à ne pas rater : `rand::rngs::SmallRng` est **explicitement non reproductible** entre
versions et plates-formes. Choisir un générateur nommé et gelé *est* le contrat, pas un détail
d'implémentation.

Le hash de (graine, pixel, index), lui, s'écrit dans le projet et se documente selon CLAUDE.md §4 :
c'est la pièce qui porte la reproductibilité, elle ne doit pas pouvoir changer sous nos pieds au
prochain `cargo update`.

**Écrire notre propre PCG a été pesé et écarté.** Une quarantaine de lignes, très dans l'esprit du §4,
et pourtant non : un générateur médiocre produit des artefacts de corrélation qui **ressemblent
exactement à un bug de renderer**. C'est le pire mode de défaillance possible pour un projet
d'apprentissage, et la seule chose qu'on y gagnerait est une dépendance de moins sur une caisse déjà
déclarée. Le hash oui, le moteur non.

## 7. Ordre de travail

Cinq commits, chacun vert seul.

1. **Tuer les boucles de rejet** — [utils.rs](../src/utils.rs) : applications pures d'un échantillon,
   `random_unit_vector` supprimé, la question boule/sphère de `Metal` tranchée et documentée. Aucun
   sampler encore. L'image change, puisque le compte de tirages change ; elle n'est de toute façon pas
   encore comparable à elle-même.
2. **Monter `rand` 0.3 → 0.10** — mécanique, et à vérifier contre la documentation plutôt que de
   mémoire : `Range`/`IndependentSample` sont devenus `Uniform`, `thread_rng` a changé de nom. Retire
   au passage l'import mort de [utils.rs:3](../src/utils.rs#L3) et le doublon `rand 0.4` du
   `Cargo.lock`. `Sampler2` reste sur le générateur global : aucun changement de comportement visé.
3. **Le chantier** — `src/samplers/`, `IndependentSampler`, `Config::seed` et `--seed` (défaut : une
   constante, **pas** l'horloge — le déterminisme est le comportement normal, pas une option),
   `CameraSample`, `Pdf::generate(u)`, les trois signatures élargies, réamorçage par
   (pixel, échantillon) dans les deux renderers. **Le déterminisme est acquis ici.**
4. **Les tests** — graine sur les deux tests de conservation d'énergie, plus le commentaire que le §1
   réclame sur celui de variance nulle ; un test de reproductibilité du sampler lui-même ; les deux
   `examples/test_scene*` prennent un sampler graine, ce qui ferme le défaut de géométrie non
   reproductible de [docs/mesures_bvh.md](../docs/mesures_bvh.md) §4.
5. **`StratifiedSampler`** et `--sampler`. Mêmes tests de conservation d'énergie, variance
   mesurablement plus basse à échantillons par pixel égaux.

## 8. Comment on vérifie

- Deux exécutions de la même commande → **PNG identiques octet pour octet**.
- `--threads 1` contre `--threads 8` → **PNG identiques**. C'est la garantie forte, et celle qui
  n'existerait pas avec un générateur par thread.
- `--samples_ppx 4` contre `--samples_ppx 8` sous `IndependentSampler` : les 4 premiers échantillons
  sont les mêmes, donc l'image de 4 est un préfixe exact de celle de 8. **Faux sous
  `StratifiedSampler`** — les strates dépendent du nombre total — et c'est correct, pas un défaut.
- Test unitaire : même (pixel, index) → même flux ; (pixel, index) différents → flux décorrélés.
- Les deux tests de conservation d'énergie deviennent des tests de régression à valeur exacte.

## 9. Ce que le chantier ne promet pas — et qui se dit dans le commit

- **L'identité au bit n'est pas garantie entre machines ni entre profils de compilation.** `sqrt` est
  exact IEEE, mais `sin`, `cos` et `powf` viennent de la libm et la contraction FMA dépend du backend.
  La promesse est : *même binaire, même machine, mêmes options*. C'est ce qu'il faut pour comparer deux
  commits, et c'est tout ce qui est revendiqué.
- **La stratification ne porte utilement que les premières dimensions.** Un chemin de profondeur
  inconnue consomme un nombre de dimensions non borné ; stratifier chacune indépendamment donne un
  hypercube latin, pas une stratification conjointe de l'espace des chemins, et le bénéfice décroît
  avec la dimension (PBR Book §8.6). La gigue, la lentille et le choix de lumière du premier sommet
  sont là où le gain est réel.
- **Le choix de la lumière reste uniforme.** Ce chantier ne touche pas à l'échantillonnage par
  puissance ; c'est MIS qui en parlera.

## 10. Deux corrections à l'entrée d'origine

- La cible n'est pas `rand 0.8` mais **`rand 0.10`**, courante au 2026-08-20.
- **`rand` n'achète ni les samplers stratifiés ni Sobol.** Les stratifiés s'écrivent ici — c'est le
  §5 de l'ordre de travail — et Sobol demanderait `sobol_burley` ou l'équivalent. Ce que `rand`
  achète vraiment : `SeedableRng`, `Uniform`, et `shuffle` pour les permutations du stratifié.
