# Mesures des accélérateurs — l'instrument, les chiffres, les arbitrages

Indexé depuis [IDEAS.md](../IDEAS.md). Ce fichier tient ce que le chantier BVH a mesuré : l'outil,
la chronologie datée des mesures, et le raisonnement des correctifs qui ont produit la forme
actuelle du code.

> **Comment le lire.** [§4](#4-les-chiffres-de-référence-daujourdhui) donne l'état courant, et
> c'est la seule section à laquelle comparer une mesure fraîche. §1 et §2 sont une **chronologie
> datée** : chaque tableau vaut pour le commit qui le porte et pas au-delà. §3 archive les
> correctifs déjà passés — il dit pourquoi le code a cette forme, il ne décrit aucun travail à
> faire. Ce qui reste à faire est dans `ideas/`.

---

## 0. L'instrument

`TraversalStats` et `BuildStats` dans [bvh.rs](../src/shapes/triangle_mesh/bvh.rs), exposés par
`TriangleMesh::intersect_instrumented` / `build_stats`, pilotés par
[src/bin/bvh_stats.rs](../src/bin/bvh_stats.rs).

```
cargo run --release --bin bvh_stats -- test_files/<mesh>.ply
cargo run --release --bin bvh_stats -- test_files/<scene>.stage
```

L'extension distingue les deux modes. Un `.ply` n'a pas de caméra et l'outil doit inventer un jeu de
rayons : les rayons primaires de 6 caméras pinhole sur une orbite déterministe, 200×200 chacune —
240 000 rayons, sans RNG, reproductibles à l'unité. Un `.stage` porte la sienne, et le jeu de rayons
est celui du rendu : `Loader::load_scene` rend cette caméra, donc un rayon par centre de pixel d'un
rendu 200×150 aux réglages par défaut. Deux jeux y sont comptés **séparément** : les rayons
primaires, et un rayon d'ombre depuis chaque point touché vers la `PointLight` que
`Loader::load_scene` câble en (0, 2, 1) — le segment que NEE lance réellement.

`bvh_stats` chronomètre aussi ce qu'il compte, et la raison est écrite dans son en-tête : les
compteurs ne classent deux arbres que si l'un gagne sur les deux, et un arbre plus profond échange
des tests de boîte contre des tests de triangle. Lire cet échange demande `t_trav`, et pondérer les
compteurs par `t_trav` pour élire `t_trav` est circulaire — l'arbitre est donc l'horloge murale sur
le même jeu de rayons fixe. Trois passes, la plus rapide étant le chiffre à comparer.

Le **chargement** est chronométré de la même façon depuis le 2026-08-20, trois passes et la plus
rapide, parce qu'un correctif de *construction* ne se lit nulle part ailleurs. Ce chiffre vaut
`parse + build`, et il faut savoir ce que cela cache : parser un `.ply` coûte environ **1,9 µs par
sommet** là où balayer les sommets pour une boîte en coûte **1,6 ns** — mille fois moins. Sur une
scène de maillages le parse écrase donc la construction, et seule la **différence** entre deux
mesures s'impute au build. C'est la raison pour laquelle la scène qui démontre un changement de
construction est une scène de sphères, où le parse n'est pas le plancher.

Et une fois cette différence obtenue, **la rapporter à l'exécution entière et non au chargement** :
la construction a lieu une fois, le rendu des milliers de fois, donc un « −74 % de chargement »
peut valoir un millième du temps total. Le [§2.3](#23-le-coût-des-bornes-à-la-construction--mesuré-et-laissé--2026-08-20)
est cette histoire-là, et c'est ce dénominateur qui y a écarté un correctif par ailleurs correct.

**Le corpus de scènes** tient dans `test_files/`. Les quatre premières — `default`, `cornell_box`,
`bunny_mesh`, `dragon_mesh` — portent 2 à 8 primitives, ce qui suffit à mesurer une traversée et pas
du tout à mesurer une construction. Deux s'y sont ajoutées le 2026-08-20 pour cela :
[`many_spheres.stage`](../test_files/many_spheres.stage), 445 primitives, et
[`many_meshes.stage`](../test_files/many_meshes.stage), 100 maillages — voir
[§2.3](#23-le-coût-des-bornes-à-la-construction--mesuré-et-laissé--2026-08-20) pour d'où elles viennent.

**Deux limites à énoncer, toutes deux vérifiées sur le terrain.**

- **Un compteur ne mesure que les rayons qu'on a pensé à lancer.** Le défaut le plus coûteux du
  chantier — les rayons d'ombre `NaN` des lumières à l'infini, trois ordres de grandeur — était
  invisible ici : `bvh_stats` lance des rayons d'ombre bien formés vers la lumière ponctuelle, les
  dégénérés n'existaient pas dans son jeu. Voir [§3.2](#32-bvh--scène).
- **Ce sont les rayons cohérents.** Ils partent d'un même point, voyagent ensemble et rentrent dans
  des nœuds encore chauds : c'est la population qui favorise le *plus* un arbre profond. Les rayons
  secondaires partent de partout, paient le nœud plein tarif, et pousseraient l'optimum vers un
  arbre plus plat. Les mesurer demande le sampler graine — voir
  [ideas/cout_traversee_bvh.md](../ideas/cout_traversee_bvh.md).

---

## 1. Maillage — chronologie

### 1.1 Référence — 2026-07-30, avant correction du SAH

| mesh | tris | nodes (leaves) | depth | leaf tris mean / max | nodes/ray | box tests/ray | **tri tests/ray** |
|---|---|---|---|---|---|---|---|
| `cube.ply` | 12 | 1 (1) | 1 | 12.0 / 12 | 1.00 | 1.00 | 5.80 |
| `bun_zipper_res4.ply` | 948 | 3 (2) | 2 | 474.0 / 716 | 1.51 | 2.45 | 254.41 |
| `bunny.ply` | 69 451 | 121 (61) | 14 | 1138.5 / 37 469 | 2.63 | 5.80 | 7464.08 |
| `dragon_vrip_res4.ply` | 11 102 | 351 (176) | 19 | 63.1 / 3143 | 3.20 | 7.36 | 517.28 |
| `dragon_vrip_res3.ply` | 47 794 | 1021 (511) | 22 | 93.5 / 15 949 | 2.98 | 6.68 | 3622.88 |
| `dragon_vrip.ply` | 871 414 | 2301 (1151) | 27 | 757.1 / 316 949 | 2.89 | 6.45 | 66 649.36 |

Taux de touche 18–20 % sur les maillages organiques, 48 % sur les cubes : les moyennes portent bien
sur des jeux de rayons qui atteignent la géométrie.

**Ce que les chiffres disaient.** L'arbre ne filtrait presque rien : sur le dragon complet un rayon
était facturé 66 649 tests de triangle, soit **7,6 % du maillage entier**, et une seule feuille
tenait 316 949 des 871 414 triangles — 36 % du modèle. `cube.ply` n'était pas subdivisé du tout
(1 nœud). La subdivision s'arrêtait presque immédiatement, signature du premier plan candidat
dégénéré : `best_pos = bound_min` laisse le côté gauche vide, le garde `left_count == 0` retourne, et
le nœud reste une feuille. Les deux défauts se composaient — le coût minimisé n'était pas le SAH, et
le plan gagnant était inutilisable.

Pour l'échelle : 240 000 rayons primaires contre `dragon_vrip.ply` prenaient **2 min 17 s**.

### 1.2 Effet d'une aire nulle sur la boîte vide — 2026-07-31

`AABoundingBox::half_area()` retourne `0` sur une boîte vide au lieu de `+inf`. Voulu comme un
nettoyage de justesse, pas comme une amélioration de l'arbre — mais les chiffres bougent, et pas dans
le sens que l'intuition suggère.

| mesh | nodes (leaves) | depth | max leaf | tri tests/ray |
|---|---|---|---|---|
| `cube.ply` | 1 (1) → **23 (12)** | 1 → 5 | 12 → **1** | 5.80 → **1.31** |
| `bun_zipper_res4.ply` | 3 (2) → 3 (2) | 2 → 2 | 716 → 716 | 254.41 → 254.41 |
| `bunny.ply` | 121 (61) → 217 (109) | 14 → 16 | 37 469 → 37 469 | 7464.08 → 7464.08 |
| `dragon_vrip_res4.ply` | 351 (176) → 935 (468) | 19 → 22 | 3143 → 3143 | 517.28 → 517.13 |
| `dragon_vrip_res3.ply` | 1021 (511) → 2423 (1212) | 22 → 26 | 15 949 → 15 949 | 3622.88 → 3623.01 |
| `dragon_vrip.ply` | 2301 (1151) → 5755 (2878) | 27 → 32 | 316 949 → 316 949 | 66 649.36 → 66 658.07 |

Comptes de touches inchangés sur les six, donc la partition est intacte.

**Lecture.** Le `+inf` n'était pas un accident inoffensif : il *rejetait d'office* tout plan candidat
comportant un bin vide, ce qui explique l'arrêt si précoce de la subdivision. Le retirer double plus
que le nombre de nœuds sur le dragon. Mais cela n'achète rien par rayon, parce que les aires sont
toujours lues par bin : un bin vide fait maintenant paraître un plan **gratuit** — faux dans l'autre
sens. Les nœuds supplémentaires découpent donc des miettes tandis que la feuille dominante, celle
qu'un vrai SAH attaquerait, reste intacte sur tous les maillages organiques. `cube.ply` est
l'exception qui démontre le mécanisme : 12 triangles sur 8 bins laissent la plupart des bins vides,
donc l'empoisonnement y bloquait tous les plans.

Conclusion pour l'étape suivante : la boîte vide devait être corrigée, mais le gain est entièrement
dans la fonction de coût elle-même.

### 1.3 Après la correction du SAH — 2026-07-31

Le coût lit désormais les aires des **unions** accumulées de part et d'autre du plan, les plans
candidats tombent sur les bonnes frontières de bin, les côtés vides sont rejetés explicitement, et la
partition classe avec le même `bin_index` que le modèle de coût.

| mesh | nodes (leaves) | depth | max leaf | **tri tests/ray** | box tests/ray |
|---|---|---|---|---|---|
| `cube.ply` | 11 (6) | 6 | 2 | 5.80 → **1.18** | 1.00 → 8.62 |
| `bun_zipper_res4.ply` | 1811 (906) | 14 | 4 | 254.41 → **0.77** | 2.45 → 15.27 |
| `bunny.ply` | 138 881 (69 441) | 21 | 2 | 7464.08 → **0.58** | 5.80 → 21.48 |
| `dragon_vrip_res4.ply` | 21 137 (10 569) | 19 | 5 | 517.28 → **0.63** | 7.36 → 18.22 |
| `dragon_vrip_res3.ply` | 92 929 (46 465) | 21 | 5 | 3622.88 → **0.60** | 6.68 → 20.87 |
| `dragon_vrip.ply` | 1 724 381 (862 191) | 29 | 6 | 66 649.36 → **0.58** | 6.45 → 24.76 |

Quatre à cinq ordres de grandeur sur les grands maillages. La feuille dominante de `dragon_vrip.ply`
passe de 316 949 triangles — 36 % du modèle — à 6. Mesurer les six maillages prenait 2 min 17 s pour
le seul dragon ; l'ensemble tourne maintenant en quelques secondes.

**La partition est intacte, vérifiée deux fois.** Les comptes de touches bruts sont identiques *à
l'unité* sur les six maillages (`cube` 116 004, `bun_zipper_res4` 47 693, `bunny` 46 904,
`dragon_res4` 42 797, `dragon_res3` 43 451, `dragon_vrip` 43 269) — `bvh_stats` imprime désormais le
compte brut, et pas seulement le pourcentage arrondi, précisément pour rendre cette comparaison
possible. Et un test unitaire compare `BVHTree::query` à l'intersection en force brute de tous les
triangles, en exigeant l'égalité exacte de la distance *et* de l'indice du triangle ; le compte de
touches seul ne remarquerait pas un rayon ayant trouvé un triangle plus lointain.

**Deux coûts, tous deux attendus.** Les tests de boîte par rayon montent d'un facteur 3 à 4 : un
arbre plus profond veut dire plus de nœuds à rejeter, c'est l'échange que fait le SAH et qu'il gagne
largement. Et le nombre de nœuds vaut maintenant ~2× le nombre de triangles, donc les feuilles
tiennent un triangle en moyenne — c'est l'écart `t_trav = 0` qui se voit
([heuristique_aire_surface.md](heuristique_aire_surface.md) §6).

**Un rendu est inchangé**, vérifié en rendant `cube_mesh.stage` depuis le commit précédent et depuis
celui-ci. Une comparaison pixel-exacte est impossible tant que le sampler n'est pas graine, donc le
test en force brute ci-dessus est la vraie garantie.

### 1.4 Après suppression du double test de boîte — 2026-07-31

| mesh | box tests/ray | tri tests/ray | nodes visited/ray |
|---|---|---|---|
| `cube.ply` | 8.62 → **5.83** (−32 %) | 1.18 → 1.19 | 3.79 → 3.01 |
| `bun_zipper_res4.ply` | 15.27 → **10.04** (−34 %) | 0.77 → 0.77 | 6.23 → 5.25 |
| `bunny.ply` | 21.48 → **14.19** (−34 %) | 0.58 → 0.58 | 8.29 → 7.17 |
| `dragon_vrip_res4.ply` | 18.22 → **12.01** (−34 %) | 0.63 → 0.63 | 7.22 → 6.11 |
| `dragon_vrip_res3.ply` | 20.87 → **13.75** (−34 %) | 0.60 → 0.60 | 8.12 → 6.96 |
| `dragon_vrip.ply` | 24.76 → **16.31** (−34 %) | 0.58 → 0.58 | 9.45 → 8.23 |

Un −34 % plat sur tous les maillages organiques, ce qui est attendu : le test retiré était l'un des
trois par nœud environ. Les tests de triangle ne bougent pas, comme ils le doivent — cela change
*comment* un nœud est atteint, jamais quels triangles une feuille contient. Comptes de touches
identiques à l'unité de nouveau.

**Deux réserves pour lire ce tableau.**

`nodes_visited` **a changé de définition** dans le même commit, donc sa colonne mélange deux effets
et ne mesure aucun gain. Il comptait chaque pop ; il compte maintenant les nœuds dont le contenu a
réellement été examiné, en excluant ceux qu'un `min_t` resserré écarte au pop et la racine d'un rayon
qui manque le maillage. Seuls `box_tests` et `triangle_tests` sont comparables à travers ce commit —
et `box_tests` est celui dont le commit parle. L'appariement est désormais exact et vérifiable :
`box_tests = 2 · (nœuds intérieurs examinés) + 1`.

`cube.ply` passe de 1,18 à 1,19 test de triangle parce que **l'ordre de départage a basculé**. Quand
deux enfants sont entrés à exactement la même distance, l'ancien code visitait le droit d'abord, le
nouveau le gauche. Aucun des deux n'est mieux fondé, tous deux trouvent la même touche la plus proche
— le test en force brute le garantit — et seule une géométrie alignée sur les axes produit assez
d'égalités exactes pour que cela se voie.

À titre indicatif, `bunny_mesh.stage` en 120×90×4 passe de 53 s à 40 s. Pas une mesure contrôlée,
puisqu'un sampler non graine signifie que les deux exécutions ont tracé des chemins différents ; le
compte de tests de boîte est le chiffre contrôlé.

### 1.5 Référence chronométrée — 2026-08-20

Apple M2 Pro, 16 Go, macOS 26.5.2, `--release`. Compteurs inchangés par rapport aux tableaux
ci-dessus, à l'unité, sur les six maillages et les deux scènes.

| mesh | load (parse + build) | traversal, plus rapide sur 3 | passes |
|---|---|---|---|
| `cube.ply` | 258 µs | **25.59 ms** | 30.27, 27.57, 25.59 |
| `bun_zipper_res4.ply` | 995 µs | **42.41 ms** | 42.79, 42.81, 42.41 |
| `bunny.ply` | 76.3 ms | **62.64 ms** | 67.85, 65.04, 62.64 |
| `dragon_vrip_res4.ply` | 9.90 ms | **50.96 ms** | 51.48, 50.96, 51.50 |
| `dragon_vrip_res3.ply` | 45.1 ms | **60.95 ms** | 63.12, 62.52, 60.95 |
| `dragon_vrip.ply` | 929 ms | **100.11 ms** | 100.11, 103.13, 101.97 |

| scène | load | les deux jeux de rayons, plus rapide sur 3 |
|---|---|---|
| `cornell_box_canonical.stage` | 99 µs | **3.09 ms** |
| `bunny_mesh.stage` | 71.3 ms | **11.24 ms** |

**La durée *est* le compte de tests de boîte.** En divisant l'une par l'autre, sur 240 000 rayons par
maillage :

| mesh | nodes | node MB | box/ray | ns/ray | **ns par test de boîte** |
|---|---|---|---|---|---|
| `cube.ply` | 11 | 0.0 | 5.83 | 106.6 | **18.3** |
| `bun_zipper_res4.ply` | 1 811 | 0.1 | 10.04 | 176.7 | **17.6** |
| `dragon_vrip_res4.ply` | 21 137 | 1.4 | 12.01 | 212.3 | **17.7** |
| `dragon_vrip_res3.ply` | 92 929 | 5.9 | 13.75 | 254.0 | **18.5** |
| `bunny.ply` | 138 881 | 8.9 | 14.19 | 261.0 | **18.4** |
| `dragon_vrip.ply` | 1 724 381 | 110.4 | 16.31 | 417.1 | **25.6** |

Cinq maillages dans une bande de 17,6–18,5 ns, sur quatre ordres de grandeur de nombre de triangles,
avec une ordonnée à l'origine quasi nulle — donc la génération de rayons, dont l'en-tête de l'outil
prévient qu'elle dilue le chiffre, est en fait sous le bruit. Compteurs et horloge s'accordent, ce
qui valide l'instrument.

**Le dragon casse la droite de 40 %, et c'est la mémoire.** C'est le seul maillage dont les nœuds —
110 Mo à 64 octets chacun — dépassent tous les niveaux de cache de cette machine ; `bunny` à 8,9 Mo
non. `[3]` compte des tests, pas des défauts de cache : sur un grand maillage, deux cinquièmes du
temps de traversée tiennent donc dans un terme auquel le modèle de coût n'a pas de place. Cet
argument-là appartient à [ideas/cout_traversee_bvh.md](../ideas/cout_traversee_bvh.md), qui porte
aussi la calibration de `t_trav` que cette table permet de borner.

### 1.6 Après inlining du test de boîte — 2026-08-20

`#[inline(always)]` sur `AABoundingBox::hit` et sur `BVHTree::hit_box`, rien d'autre. Même machine et
même protocole qu'en §1.5, plus rapide sur trois passes.

| mesh | traversal | | box tests/ray | tri tests/ray |
|---|---|---|---|---|
| `cube.ply` | 24.76 → **23.04 ms** | −7 % | 5.83 | 1.19 |
| `bun_zipper_res4.ply` | 41.18 → **34.79 ms** | −16 % | 10.04 | 0.77 |
| `dragon_vrip_res4.ply` | 51.00 → **44.86 ms** | −12 % | 12.01 | 0.63 |
| `dragon_vrip_res3.ply` | 60.29 → **53.13 ms** | −12 % | 13.75 | 0.60 |
| `bunny.ply` | 62.63 → **55.22 ms** | −12 % | 14.19 | 0.58 |
| `dragon_vrip.ply` | 100.88 → **84.18 ms** | −17 % | 16.31 | 0.58 |
| `cornell_box_canonical.stage` | 3.09 → **2.64 ms** | −15 % | | |
| `bunny_mesh.stage` | 11.24 → **9.73 ms** | −13 % | | |

Pas un compteur ne bouge, ni un compte de touches — 116 004, 47 693, 42 797, 43 451, 46 904,
43 269, les mêmes chiffres que tous les tableaux ci-dessus. C'est tout l'intérêt d'un changement
bit-identique : les compteurs en sont un test de régression complet, et un seul point de mouvement
aurait signifié un bug plutôt qu'un gain.

Les chiffres de scène gagnent autant que ceux de maillage bien que l'enveloppe de scène ait été
laissée telle quelle, `hit` étant partagée : une traversée de scène teste les boîtes de ses propres
nœuds par la même fonction inlinée.

---

## 2. Scène — chronologie

### 2.1 Référence — 2026-07-31

| scène | prims | jeu de rayons | nodes/ray | box tests/ray | object tests/ray | hit |
|---|---|---|---|---|---|---|
| `default.stage` | 2 | primaires | 1.27 | 2.90 | 0.32 | 31.8 % |
| | | ombre | 1.84 | 3.00 | 0.84 | 83.8 % |
| `cornell_box.stage` | 8 | primaires | 5.18 | 9.63 | 0.87 | 76.0 % |
| | | ombre | 9.16 | 15.00 | **2.16** | 95.9 % |
| `bunny_mesh.stage` | 4 | primaires | 3.85 | 7.00 | 0.85 | 54.5 % |
| | | ombre | 3.28 | 7.00 | 0.28 | 9.0 % |
| `dragon_mesh.stage` | 4 | primaires | 4.23 | 7.00 | 1.23 | 54.5 % |
| | | ombre | 2.99 | 6.39 | 0.30 | 11.0 % |

**Ce que les chiffres ont corrigé.** Le plan de ces étapes supposait l'accumulateur coûteux parce
qu'il distribue beaucoup de candidats. Il ne le fait pas : **les tests d'objet par rayon restent sous
1,3** sur toutes les scènes, rayons primaires compris. À quatre ou huit primitives, l'accélérateur
n'est simplement pas là où un rayon primaire passe son temps.

Deux choses que la table dit tout de même :

- **L'intérieur de l'arbre n'élague rien.** Un arbre de 4 primitives a 7 nœuds, et les scènes de
  maillage testent 7,00 boîtes par rayon — chaque nœud, chaque rayon. Les rectangles 3×3 du sol et
  des murs recouvrent tout le champ, donc la racine et ses deux enfants sont touchés par presque tous
  les rayons, et seules les feuilles rejettent quelque chose. `cornell_box` fait mieux, 9,63 sur
  15 nœuds. C'est ce que l'ordonnancement et le resserrement d'intervalle peuvent attaquer.
- **Les rayons d'ombre sont le pire cas, et `cornell_box` dit pourquoi.** 95,9 % d'entre eux sont
  occultés, et `unoccluded` cherchait encore la touche la *plus proche* au lieu de s'arrêter à la
  première : 2,16 tests d'objet là où un suffirait.

**Et une limite à énoncer franchement.** Le gain d'`intersect_p` est surtout *à l'intérieur* de
chaque test d'objet — le `Vec<Intersection>` alloué, la normale de shading, les coordonnées de
texture, les ∂p/∂u et ∂p/∂v calculés puis jetés — et aucun compteur ici ne peut le voir.

**Un défaut de l'instrument, trouvé ici.** La mesure de scène n'était **pas** reproductible :
`BVHNode::choose_comparator` tirait son axe de coupe d'un `random_double()` non graine, donc l'arbre
— et avec lui tout compte de nœud et de boîte — différait d'une exécution à l'autre. Mesuré sur
`cornell_box.stage`, trois exécutions consécutives : 9,63, 8,95 et 9,31 tests de boîte par rayon
primaire. `object_tests` était bien plus stable, variant à la deuxième décimale, parce qu'il dépend
des primitives situées le long du rayon plutôt que de la façon dont elles ont été groupées. Corrigé
depuis — voir [§3.2](#32-bvh--scène).

### 2.2 Après `intersect_p` et le visibility tester — 2026-08-15

Tests d'objet par rayon d'ombre, le chiffre visé :

| scène | avant | après |
|---|---|---|
| `default.stage` | 0.84 | **0.00** |
| `cornell_box.stage` | 2.16 | **0.91** |
| `bunny_mesh.stage` | 0.28 | **0.20** |
| `dragon_mesh.stage` | 0.30 | **0.21** |

La prédiction ci-dessus était juste, et à côté de l'essentiel. `cornell_box` est divisé par deux
comme prévu ; `default.stage` tombe à zéro, parce qu'avec `far` portant désormais la distance de la
lumière, les boîtes de feuille au-delà de la lumière sont rejetées et aucun candidat ne survit. Et
ensuite :

**`bunny_mesh.stage`, 120×90×4 : 38,9 s → 0,13 s.** Trois cents fois, ce qu'aucun compteur de la
table ne prédit — il fallait donc l'attribuer plutôt que l'annoncer. Deux expériences ont échoué à
reproduire la lenteur : restaurer le testeur dégénéré seul ne suffit pas, ni avec `far = 0` ni avec
`far = f64::MAX`, parce qu'`intersect_p` court-circuite au premier candidat qui rapporte une touche
et qu'un rayon `NaN` fait rapporter une touche au premier candidat. La décisive a été de retirer
`BackgroundInfiniteLight` de l'*ancien* build : 38,9 s → 0,16 s.

Tout tenait donc à cette seule lumière. Son testeur construisait un rayon entre un point et lui-même,
dont la direction normalisée est `NaN`, et **`NaN` défait tous les tests de rejet des deux
accélérateurs** — `f64::max(NaN, tmin)` retourne `tmin`, `f64::min(NaN, tmax)` retourne `tmax`, donc
aucune slab ne rejette et toute boîte rapporte une touche. Envoyé dans `intersect(.., f64::MAX)`,
chacun de ces rayons parcourait l'arbre de scène entier *et les 138 881 nœuds du maillage du lapin,
en testant les 69 451 triangles* — une fois par échantillon NEE de cette lumière, par rebond, par
échantillon de pixel.

Deux leçons à garder :

- **L'instrumentation ne pouvait pas voir cela**, et ne l'a dit qu'après coup. Voir la limite énoncée
  en [§0](#0-linstrument).
- **Une quantité qu'on ne peut pas rejeter est pire qu'une grande.** Le même mécanisme
  `NaN`-défait-`max` était déjà documenté dans `AABoundingBox::hit`, où le cas du rayon parallèle est
  traité explicitement plutôt que laissé à la propagation. Il a remordu un niveau plus haut, dans le
  rayon lui-même.

`cornell_box.stage` change aussi, et spectaculairement : une pièce fermée était éclairée par le ciel,
parce que la lumière de fond n'était jamais occultée par ses propres murs. Elle ressemble maintenant
à une boîte de Cornell — coins sombres, blocs projetant des ombres — au lieu d'un intérieur blanc
délavé. C'est un correctif de justesse, pas un réglage de contraste.

### 2.3 Le coût des bornes à la construction — mesuré, et laissé — 2026-08-20

**Le défaut est réel, le correctif tient en quinze lignes, et il ne vaut pas la peine d'être écrit.**
Cette section dit pourquoi, parce que le chiffre qui tranche n'est pas celui qu'on va chercher
spontanément.

`AABound::get_bounding_box` se lit comme un accesseur et est un calcul : `TriangleMesh` balaie tous
ses sommets, `Transformed` transforme les huit coins de ce que rend l'objet intérieur, `Compound`
replie sur tous ses enfants. Rien ne le cache, et la construction est l'appelant qui le rappelle le
plus — `widest_centroid_axis` balaie une plage entière à chaque niveau, et `compare_centroid`
demande **deux fois par comparaison**, dans un `sort_by`, à chaque niveau.

**Le compte**, relevé par un compteur atomique posé sur les trois sites d'appel de `bvh.rs`, retiré
depuis :

| scène | primitives | appels par build | par primitive |
|---|---|---|---|
| `cornell_box.stage` | 8 | 90 | 11 |
| `many_meshes.stage` | 100 | 5 440 | 54 |
| `many_spheres.stage` | 445 | 26 671 | 60 |

Le facteur par primitive croît en `log² n` : O(n log² n) appels pour O(n) réponses distinctes. Sur
`many_meshes`, chacun de ces 5 440 appels balaie 453 sommets.

**Le correctif a été écrit et mesuré** avant d'être retiré. Une enveloppe privée à
[bvh.rs](../src/bvh.rs) tenant boîte et centroïde, calculés une fois par primitive à l'entrée de
`BVH::new`, la construction ne lisant plus que ces deux champs. Chargement le plus rapide sur trois
passes, Apple M2 Pro / 16 Go / macOS 26.5.2 / `--release` :

| scène | primitives | avant | avec le correctif | |
|---|---|---|---|---|
| `many_spheres.stage` | 445 | 1.62 ms | 0.42 ms | −74 % |
| `many_meshes.stage` | 100 | 71.5 ms | 67.7 ms | −4 ms |
| `cornell_box.stage` | 8 | 15.6 µs | 10.9 µs | |
| `bunny_mesh.stage` | 4 | 68.1 ms | 65.3 ms | |
| `dragon_mesh.stage` | 4 | 931 ms | 882 ms | |

Et en montant en taille, sur des scènes générées pour l'occasion et non versionnées — grilles de
sphères, et grilles du même lapin `bun_zipper_res4` :

| scène | avant | avec le correctif | rapport |
|---|---|---|---|
| 442 sphères | 1.89 ms | 0.46 ms | 4.1× |
| 1 765 sphères | 9.86 ms | 2.38 ms | 4.1× |
| 7 057 sphères | 55.25 ms | 8.43 ms | 6.6× |
| 28 225 sphères | 224.75 ms | 32.29 ms | 7.0× |
| 100 maillages | 71.5 ms | 67.7 ms | −4 ms |
| 400 maillages | 291.3 ms | 273.2 ms | −18 ms |
| 900 maillages | 657.7 ms | 614.5 ms | −43 ms |

−74 % se lit très bien. **C'est le mauvais dénominateur.** La construction a lieu une fois par
exécution, et le bon dénominateur est l'exécution entière :

| scène | build épargné | exécution complète, réglages par défaut | part |
|---|---|---|---|
| `many_spheres.stage` (445) | 1.2 ms | 1.02 s | **0.1 %** |
| `many_meshes.stage` (100 maillages) | 4 ms | 0.77 s | 0.5 % |
| `dragon_mesh.stage` (4) | ~0 | 1.47 s | — |
| 28 225 sphères | 192 ms | 0.86 s | 26 % → 4 % |

Et « réglages par défaut » veut dire 800×600 à **5 échantillons par pixel et profondeur 3** — un
aperçu. `images/test_scene_250_spp.png` dit ce qu'est une vraie image : cinquante fois plus long,
ce qui divise par cinquante chaque part de la colonne de droite. Le gain vaut donc un millième d'un
aperçu et un cinquante-millième d'une image finie.

**La seule ligne qui pèse est la dernière, et elle décrit une scène qui n'existe pas** — 28 225
primitives, ni écrivable à la main ni produite par quoi que ce soit dans le dépôt. C'est elle qui
fixe la condition de réouverture : au-delà du millier de primitives *dans une scène réelle*, la
construction redevient un terme du temps total et le correctif se justifie tout seul.

**Ce que coûtait le correctif**, et pourquoi le compte tombe du mauvais côté. Il n'y a pas de coût
d'interface — l'enveloppe est privée au module, `BVH::new` garde sa signature, `Scene` n'est pas
touchée. Le coût est ailleurs : `subdivide` perd son `&mut self` pour une fonction associée à trois
paramètres, parce que la récursion emprunte les nœuds et les primitives ensemble, et l'enveloppe se
pose puis se retire en fin de `new`. Quinze lignes de gain de lisibilité au comparateur
(`a.centroid[axis]` plutôt que `a.get_bounding_box().centroid()[axis]`, ce qui est réellement mieux)
contre une dégradation de la fonction qui structure la construction. À 0,1 % de l'exécution, c'est
l'arbitrage que `CLAUDE.md` §0 tranche contre la performance.

**Ce que l'expérience a appris sur l'instrument**, et qui reste acquis quoi qu'il arrive du
correctif :

- **Le parse PLY est trois ordres de grandeur plus cher par sommet que le balayage de boîte** —
  1,9 µs contre 1,6 ns. Aucune scène de maillages ne peut donc isoler un changement de construction,
  et c'est pourquoi le corpus a eu besoin d'une scène de sphères.
- **Les compteurs de traversée sont indifférents à tout cela.** Ils étaient identiques à l'unité,
  comptes de touches compris, sur les sept scènes et les six maillages, avec et sans le correctif —
  ce qui était attendu, l'arbre étant le même arbre, et ce qui a confirmé que la mesure portait bien
  sur la construction seule.
- Le chargement est désormais chronométré sur trois passes comme le reste ([§0](#0-linstrument)) :
  une passe unique ne tenait pas la milliseconde qu'il fallait lire.

**D'où vient `many_spheres.stage`, et pourquoi pas `examples/test_scene.rs`.** La scène finale de
« Ray Tracing in One Weekend » est le seul objet du dépôt qui tienne des centaines de primitives, et
elle existait déjà — comme *exemple Rust*. Elle ne pouvait pas servir de référence, pour deux
raisons dont la seconde est la vraie :

- `bvh_stats` ne charge que `.ply` et `.stage`, et un exemple est un binaire séparé qui construit sa
  scène en Rust. Obstacle mécanique.
- Ses 441 petites sphères sont **tirées d'un `random_double()` non graine**. La géométrie diffère
  donc à chaque exécution, donc l'arbre aussi, donc aucun compteur n'est comparable à lui-même —
  exactement le défaut que l'axe de coupe aléatoire avait, corrigé au [§3.2](#32-bvh--scène), et
  pour lequel le même argument vaut : un accélérateur dont le coût ne se mesure pas ne s'améliore
  pas exprès.

`many_spheres.stage` est cette scène **gelée** : un tirage, transcrit une fois dans le langage
`.stage`, positions et matériaux écrits dans le fichier. La géométrie devient une propriété du
fichier au lieu d'une propriété de l'exécution, et c'est tout ce qui manquait.
`examples/test_scene.rs` reste ce qu'il est — un exemple, et une image.

`many_meshes.stage` répond à un besoin distinct, déjà écrit au [§3.1](#31-bvh--maillage) : le
hissage des réciproques y est noté « à réouvrir si une scène tient un jour beaucoup de maillages »,
parce que les réciproques sont hissées *par traversée* et qu'un rayon les recalcule une fois par
maillage candidat. À 1 à 4 objets, c'était du bruit ; cette scène en tient cent.

---

## 3. Les correctifs passés — le raisonnement qui survit au correctif

### 3.1 BVH — maillage

**Le coût SAH était calculé sur la mauvaise boîte.** `left_bin.bounds.half_area()` — l'aire d'un
*seul* bin — là où le SAH veut l'aire de l'union de tous les bins de ce côté du plan.
`left_box`/`right_box` étaient accumulés correctement puis jamais relus. Le balayage
préfixe/suffixe lit désormais les unions accumulées. La dérivation est dans
[heuristique_aire_surface.md](heuristique_aire_surface.md), dont le §3 porte le contre-exemple qui
tranche : pour huit bins équipeuplés d'étalement croissant, le coût par bin n'est pas seulement
imprécis, il est **constant** — il ne distingue rien, égalise sur les sept plans, et la comparaison
stricte élit alors le premier, c'est-à-dire le dégénéré. Un `debug_assert!` garde l'invariant qui
manquait à la forme par bin : une union ne peut que croître, donc les aires préfixes ne peuvent pas
décroître et les aires suffixes pas croître.

**Le premier plan candidat était dégénéré.** `bound_min + i * inv_scale` avec `i` partant de 0
plaçait le premier plan exactement sur `bound_min`, côté gauche vide, le garde `left_count == 0`
retournant et la subdivision s'arrêtant. La frontière `i` est maintenant en
`centroid_min + (i + 1) · bin_width`, et `inv_scale` — le doublon mal nommé de `scale` qui faisait
lire la mauvaise formule comme plausible — a disparu.

**`evaluate_sah` était du code mort, et bogué** : `f64::MAX` pour un coût nul, sous un commentaire
parlant d'une division qu'il ne fait pas. Corrigé, renommé `exhaustive_split_cost`, marqué
`#[cfg(test)]`, et utilisé comme oracle de `test_binned_cost_matches_exhaustive_scan` — qui échoue
sur le défaut par bin, vérifié en le réintroduisant.

**La partition comparait une position flottante reconstruite** alors que le coût était dérivé de
comptes de bins, si bien que les deux pouvaient diverger pour un centroïde sur une frontière et que
le plan gagnant pouvait être noté sur une partition qui n'a jamais eu lieu. `SplitCandidate` porte
maintenant l'indice de frontière et les paramètres de binning plutôt qu'une position, et les deux
chemins appellent le même `bin_index`. Pas dans la revue d'origine — trouvé en planifiant le
correctif.

**La boîte de chaque nœud était testée deux fois**, une fois avant que son parent l'empile et une
fois de plus après le pop. La pile porte désormais `StackEntry { node_idx, entry_distance }` : la
distance est mesurée quand le parent ordonne ses enfants — ce qu'il doit faire de toute façon — et
voyage avec l'indice au lieu d'être recalculée. Le pop réexamine toujours cette distance contre
`min_t`, ce qui est la moitié utile de l'ancien re-test et ne coûte aucun test de boîte. La racine
est testée une fois hors de la boucle, donc un rayon qui manque le maillage coûte exactement un test
de boîte. Chiffres en [§1.4](#14-après-suppression-du-double-test-de-boîte--2026-07-31).

**`AABoundingBox::hit` recalculait trois réciproques par test de boîte.** `1.0 / ray.direction[i]`
par axe, par appel ([aabound.rs](../src/geom/aabound.rs)) — 16,31 tests de boîte par rayon sur
`dragon_vrip`, soit **49 divisions f64 par rayon, recalculant toutes les mêmes trois valeurs**. Elles
sont invariantes de boucle, mais la frontière d'appel les cachait à la LICM : `objdump` montrait
`hit` comme une fonction de 110 instructions atteinte par `bl`, jamais inlinée, avec trois `fdiv`
dedans.

*Corrigé, et par aucune des deux routes envisagées.* `#[inline(always)]` sur `hit` **et** sur
`BVHTree::hit_box` est tout le correctif : les divisions n'étaient jamais le coût, la frontière
d'appel l'était. Une fois retirée, la LICM hisse les trois réciproques dans le prologue de la
traversée — visible dans le désassemblage, trois `fdiv` en +0xb4 avec leurs résultats spillés puis
rechargés par test de boîte. Les deux attributs sont nécessaires : marquer `hit` seul ne fait que
déplacer la frontière vers l'enveloppe à compteurs, ce qui ne mesure rien du tout. Chiffres en
[§1.6](#16-après-inlining-du-test-de-boîte--2026-08-20) ; compteurs et comptes de touches identiques
à l'unité sur les six maillages et les deux scènes, ce que « bit-identique » achète comme test.

*Deux choses mesurées et rejetées, pour qu'on ne les retente pas.* Une réciproque cachée sur `Ray`
coûte 3 à 7 % sur `dragon_vrip` — neuf passes alternées par variante, sans recouvrement d'un tour à
l'autre — et est neutre sur les maillages tenant en cache. Les divisions étant déjà hissées, tout ce
qu'elle ajoute est trois `f64` à lire dans un `Ray` 50 % plus gros à chaque test de boîte, sur le
seul maillage où le trafic mémoire est déjà le goulet. Et le même attribut sur le `hit_box` de
*scène* est neutre : ces arbres tiennent 4 à 8 primitives sur 3 à 4 niveaux, donc une boucle de
traversée aussi courte n'a rien à hisser.

*La dépendance à l'optimiseur est assumée, et voici en quoi elle consiste.* Passer les réciproques en
argument à `hit` — la forme de pbrt, et la façon structurelle d'écrire le même hissage — a été pesée
et écartée : la signature `hit(ray, tmin, tmax)` pose une question de géométrie, et un quatrième
paramètre portant un cache serait la seule chose dedans qui ne parle pas du domaine, dans une
fonction dont le doc-comment est une dérivation de bornes d'erreur flottante. C'est l'arbitrage que
`CLAUDE.md` tranche contre la performance. Ce sur quoi le gain repose, précisément : `alwaysinline`,
que l'inliner doit honorer plutôt que peser ; `noalias` sur `&mut TraversalStats`, pour que LLVM
sache que le compteur ne peut pas aliaser le rayon ; et la LICM sur une division invariante de
boucle. Le chemin nominal, pas un recoin de l'optimiseur. Et le mode de défaillance est une perte de
15 % de traversée de maillage, jamais une image fausse — détectable en relançant `bvh_stats` contre
les chiffres du §1.6, ce à quoi l'instrument sert.

*À réouvrir si une scène tient un jour beaucoup de maillages* : les réciproques sont hissées par
*traversée*, donc un rayon les recalcule une fois par maillage candidat. Aux 1 à 4 objets des scènes
d'aujourd'hui c'est du bruit ; à cinquante, l'argument change de camp.

**Un maillage vide faisait récurser `build_stats` dans un nœud inexistant.** `build` laissait une
racine à `tri_count == 0`, qu'`is_leaf` rapporte comme un nœud intérieur, donc un parcours suivait
`left_first` dans un `nodes` vide. Pire que décrit : `left_first == 0` sur cette racine, donc
`collect_build_stats` récursait dans la racine elle-même et débordait la pile avant d'atteindre le
frère manquant ; la traversée, elle, violait la précondition de `hit`-sur-boîte-vide. Clos en
interdisant l'état plutôt qu'en le représentant — `TriangleMesh::new` exige une liste d'indices non
vide, ce qui est la seule porte y menant, et `build_stats` redit l'invariant par un `debug_assert` là
où il s'y appuie. C'est le traitement que l'arbre de scène avait déjà reçu (voir `BVHNode::new` en
[§3.2](#32-bvh--scène)), appliqué au second des deux accélérateurs. Ce qui reste est l'encodage :
`is_leaf` lit toujours `tri_count == 0` comme « intérieur », donc l'état est inatteignable plutôt
qu'indicible.

### 3.2 BVH — scène

**`query` clonait les primitives trouvées** dans un `Vec` accumulateur — une allocation plus un
incrément de compteur atomique par candidat. Mesuré : **moins d'un candidat par rayon primaire**,
donc le coût était réel mais petit. Le « N incréments atomiques par rayon » de la revue se lisait
comme si N était grand ; il ne l'est pas. Corrigé en deux temps : l'arbre à plat a fait de chaque
feuille une **plage** de primitives qu'elle possède, ce qui a retiré le vecteur par feuille, et la
fermeture a retiré l'accumulateur avec le dernier des incréments. À noter que la lecture « réel mais
petit » est celle que l'horloge a contredite — voir l'entrée sur la traversée ordonnée ci-dessous.

**L'arbre était un arbre de pointeurs** — `Option<Box<BVHNode<T>>>` avec un `Vec<T>` dans chaque
feuille, donc une allocation tas par nœud plus une par feuille, dispersées là où l'allocateur les
mettait, et une traversée forcément récursive. `BVH<T>` est maintenant un `Vec<Node>` adressé par
indice, les primitives dans un second vecteur permuté de sorte que chaque feuille possède une plage
contiguë — la même disposition que `BVHTree` dans `shapes::triangle_mesh`. Deux allocations, des
nœuds contigus, une pile explicite, et `T: Clone` n'est plus requis puisqu'aucune primitive n'est
jamais copiée. Délibérément **à comportement préservé** : même coupe, même ordre, donc chaque
compteur identique au chiffre sur les quatre scènes. C'était l'intérêt de le faire seul.

**Pas de traversée ordonnée, pas de resserrement de `far`, pas de sortie anticipée.**
`Scene::intersect` collectait tous les candidats, puis les testait tous. Le BVH filtrait sans
ordonner. `Accumulator` est remplacé par une fermeture `FnMut(&T, f64, f64) -> Option<f64>` :
l'arbre élague, la fermeture intersecte et rapporte la distance qu'elle a adoptée, et la traversée
resserre son intervalle dessus — bornant ses tests de boîte autant que ses tests de primitive.
L'enfant le plus proche est ouvert d'abord, pour que le resserrement ait la meilleure chance de mordre
tôt. `intersect_p` a sa propre traversée non ordonnée, `BVH::query_p`, dont le seul avantage est de
pouvoir **s'arrêter**.

Deux leçons à garder :

- **Le plafond prédit en planifiant avait raison sur les compteurs et tort sur l'horloge.** Prédit
  7,00 → environ 6 tests de boîte par rayon primaire ; mesuré 7,00 → 7,00 sur les scènes de maillage
  et 9,12 → 9,07 sur `cornell_box`. Les arbres tiennent 4 à 8 primitives sur 3 à 4 niveaux, et le sol
  et les murs recouvrent tout le champ, donc il n'y a presque rien à élaguer — chaque nœud est
  atteint quel que soit l'ordre. Les tests d'objet baissent, eux, d'environ 8 % : 0,85 → 0,77,
  1,23 → 1,15, 0,87 → 0,80. Les rayons d'ombre gagnent par la seule sortie anticipée, `cornell_box`
  15,00 → 13,22 tests de boîte par rayon.
- **L'horloge a bougé bien plus que cela : `cornell_box.stage` à 32 spp est passé de
  19,27/17,24/17,19 s à 15,61/15,03/14,71 s, environ −14 %, trois exécutions chacune et sans
  recouvrement.** Les compteurs primaires et d'ombre ne peuvent pas en rendre compte, donc le reste
  vient de ce que `bvh_stats` ne mesure pas — les rayons secondaires, incohérents, où le resserrement
  paie beaucoup mieux, et le `Vec` par rayon que l'accumulateur allouait à chaque requête, maintenant
  disparu. Ce partage n'est *pas* mesuré ; c'est l'hypothèse restante.

**L'axe de coupe était tiré au hasard**, ce qui rendait le build non reproductible et donc
l'accélérateur non mesurable : trois exécutions consécutives de `cornell_box.stage` donnaient 9,63,
8,95 et 9,31 tests de boîte par rayon primaire. La coupe se fait maintenant selon l'axe de plus grand
étalement des **centroïdes** — déterministe, et meilleur pari par ailleurs, étant l'axe le long
duquel un plan sépare le plus les primitives. `cornell_box` donne 9,12 à chaque exécution.

**`BVHNode::new` sur un vecteur vide** tombait dans le bras `_` et récursait indéfiniment ;
`Scene::commit` sur une scène vide y arrivait. Mode de défaillance mesuré, en retirant le garde :
`fatal runtime error: stack overflow`, SIGABRT — pas un blocage. `Scene::build_bvh` retourne
maintenant `None` pour une liste de primitives vide, ce qui est là où la vacuité appartient :
l'`Option` la porte déjà, donc `BVHNode` n'a jamais à le faire, et `intersect` lit `None` comme
« rien à toucher » sans un seul test de boîte. `BVHNode::new` énonce la précondition et l'assère,
même raisonnement qu'`AABoundingBox::hit`. Trois tests, dont un qui construit sur sept primitives et
vérifie que la boîte racine les englobe toutes.

**Pas d'`intersect_p` au niveau de la scène.** Les rayons d'ombre passaient par la recherche complète
plus-proche-touche-plus-matériau là où un booléen avec sortie anticipée suffit. Fait pour `Scene` :
`intersect_p` s'arrête au premier occulteur, reste non ordonné — l'ordonnancement existe pour
atteindre la touche la *plus proche* plus tôt, et ici n'importe quelle touche en vaut une autre — et
teste les candidats par `Intersectable::intersect` plutôt que `Object::intersect`, donc aucun
matériau n'est cloné pour être jeté.

**`intersect_p` n'atteignait pas l'intérieur des formes.** `Scene::intersect_p` s'arrêtait au premier
*objet*, mais tester cet objet lançait encore `Intersectable::intersect` — pour un maillage, la
recherche complète de la plus proche touche parmi ses triangles *et* les dérivées de shading, pour un
booléen. `Intersectable::intersect_p` a désormais une implémentation par défaut qui délègue à
`intersect`, donc rien ne casse et le gaspillage est retiré là où il vaut de l'être :
`BVHTree::intersect_p` (traversée any-hit — pas d'ordre, pas de resserrement, pas de plus-proche à
garder, retour au premier triangle), `TriangleMesh`, et les surcharges de transfert dans `Simple`,
`Transformed`, `Compound` et le `Wrapper` de `Scene`, sans lesquelles le défaut les masquerait.

**Mesuré : ~3 %** sur les scènes de maillage en 200×150×64 — `bunny_mesh` 1,36 s → 1,32 s,
`dragon_mesh` 2,65 s → 2,57 s — et rien sur `cornell_box`, qui n'a pas de maillage et dont les formes
retombent donc sur le défaut. Les compteurs de tests d'objet ne bougent pas du tout, comme prévu :
cela change le prix d'un test, pas leur nombre. Le retour est petit parce qu'aucune scène de test ne
combine les deux choses qu'il lui faut — un maillage *et* une forte proportion de rayons d'ombre
occultés. `cornell_box` occulte 19,9 % des siens mais ne tient que des rectangles ; `bunny_mesh`
tient un maillage de 69 451 triangles mais n'occulte que 2,7 %. Une scène fermée contenant un
maillage montrerait bien plus, et c'est le cas pour lequel cela existe.

**`Plane` rapportait une boîte non bornée**, `±f64::MAX` en x et z, donc un `half_area` à `inf` — ce
qui empoisonnerait tout coût SAH dès qu'un `Plane` siège dans le BVH de scène. Pas un bug de bornes
à proprement parler, mais une question de conception : une primitive non bornée n'a rien à faire dans
une structure d'accélération (pbrt les en garde dehors). `Plane` retourne maintenant une boîte
honnêtement infinie au lieu de `±f64::MAX` — un nombre *fini* tenant lieu d'infini, dont la
différence débordait vers `+inf` de toute façon, mais par accident et d'une manière qu'aucun prédicat
ne pouvait distinguer d'une boîte simplement énorme. `AABoundingBox::is_bounded` peut le dire
désormais, `Scene::commit` trie les primitives non bornées sur une liste testée pour chaque rayon, et
`BVHNode::new` assère que ce qu'il tient est borné.

### 3.3 Robustesse

**Les lumières à l'infini construisaient un testeur de visibilité dégénéré** — toutes deux passaient
`(0,0,0)` pour les deux extrémités et ignoraient leur argument `_intersection`, donc le rayon d'ombre
avait une direction nulle. Le défaut le plus coûteux trouvé sur cette branche, par trois ordres de
grandeur : voir [§2.2](#22-après-intersect_p-et-le-visibility-tester--2026-08-15). Une lumière à
l'infini n'a pas de position à viser, seulement une direction, ce qui est précisément pourquoi la
forme à deux points ne pouvait pas l'exprimer ; `VisibilityTester::towards_infinity` le fait.

**`AABoundingBox::new` gonflait chaque axe à une extension minimale de 0,01**, ce qui biaisait tout
`half_area` et donc tout coût SAH. L'enquête a montré que le clamp n'était pas le garde qu'il
paraissait : `Plane` et `Rectangle` se rembourraient eux-mêmes à la main, donc les triangles — le
chemin critique du SAH — étaient ses seuls clients. Le vrai défaut était dans
[`hit`](../src/geom/aabound.rs), dont le `tmax <= tmin` rejetait toute slab d'épaisseur nulle ; le
clamp ne faisait que le masquer. Changements : `hit` rejette maintenant sur `tmax < tmin` pour qu'une
touche tangentielle compte (requis : une boîte englobante est une borne *conservatrice*) ; les rayons
parallèles à une slab sont traités explicitement au lieu de compter sur `f64::max` pour laisser
tomber un NaN silencieusement ; l'intervalle de slab est élargi de la borne d'arrondi 2γ(3) ; `new`
stocke la borne fidèlement avec un `debug_assert` ; `combine` et `Compound::get_bounding_box`
utilisent `empty()` au lieu d'une boîte inversée rattrapée par accident ; `Rectangle` a lâché son
rembourrage ±1,0 pour une borne plate exacte. Quatre tests ajoutés pour les cas dégénérés et
rasants.

La borne d'arrondi a son propre écrit dans [arithmetique_flottante.md](arithmetique_flottante.md) :
§0–§3 couvrent la représentation flottante, le modèle d'erreur standard et γ(n) ; §4 dérive 2γ(3) pas
à pas, avec intervalles calculés et le contre-exemple qui force la forme en magnitude. Le
doc-comment de `hit` porte une version condensée de la même dérivation.

Au passage, `new_invalid()` est renommé `empty()` — l'élément neutre de l'union, pas un état invalide
— `is_empty()` est la façon de poser la question, `half_area()` rapporte `0`, l'aire de l'ensemble
vide, et `hit()` assère que la boîte est non vide au lieu de compter sur le débordement pour tomber
du bon côté. Effet mesuré en [§1.2](#12-effet-dune-aire-nulle-sur-la-boîte-vide--2026-07-31) : cela
débloque la subdivision mais n'achète rien par rayon.

---

## 4. Les chiffres de référence d'aujourd'hui

C'est la table à laquelle comparer une mesure fraîche. Elle vaut pour l'état courant du dépôt, sur
Apple M2 Pro / 16 Go / macOS 26.5.2 / `--release`, sur le jeu de rayons décrit au
[§0](#0-linstrument). Le dernier chantier daté, le [§2.3](#23-le-coût-des-bornes-à-la-construction--mesuré-et-laissé--2026-08-20),
n'a rien changé au code : il a ajouté deux scènes à cette table et écarté un correctif.

| mesh | tris | nodes (leaves) | box/ray | tri/ray | traversée | touches |
|---|---|---|---|---|---|---|
| `cube.ply` | 12 | 11 (6) | 5.83 | 1.19 | 23.04 ms | 116 004 |
| `bun_zipper_res4.ply` | 948 | 1 811 (906) | 10.04 | 0.77 | 34.79 ms | 47 693 |
| `dragon_vrip_res4.ply` | 11 102 | 21 137 (10 569) | 12.01 | 0.63 | 44.86 ms | 42 797 |
| `dragon_vrip_res3.ply` | 47 794 | 92 929 (46 465) | 13.75 | 0.60 | 53.13 ms | 43 451 |
| `bunny.ply` | 69 451 | 138 881 (69 441) | 14.19 | 0.58 | 55.22 ms | 46 904 |
| `dragon_vrip.ply` | 871 414 | 1 724 381 (862 191) | 16.31 | 0.58 | 84.18 ms | 43 269 |

| scène | primitives | chargement (parse + build) | traversée (les deux jeux) |
|---|---|---|---|
| `cornell_box_canonical.stage` | 8 | 23 µs | 2.73 ms |
| `bunny_mesh.stage` | 4 | 68.0 ms | 10.13 ms |
| `many_spheres.stage` | 445 | 1.67 ms | 14.73 ms |
| `many_meshes.stage` | 100 | 71.6 ms | 15.69 ms |

Le chargement de `many_spheres` est **le chiffre à surveiller pour tout changement de
construction** : c'est le seul de la table où le build est le terme dominant, et le
[§2.3](#23-le-coût-des-bornes-à-la-construction--mesuré-et-laissé--2026-08-20) donne 0.42 ms comme
plancher connu — ce que ces 1.67 ms deviendraient si les bornes étaient cachées.

Et leurs compteurs, qui sont ce qu'un changement de traversée doit reproduire :

| scène | jeu | nodes/ray | box/ray | object/ray | touches |
|---|---|---|---|---|---|
| `cornell_box_canonical.stage` | primaires | 1.39 | 3.42 | 0.18 | 5 027 |
| | ombre | 7.90 | 13.00 | 1.62 | 1 043 |
| `bunny_mesh.stage` | primaires | 3.77 | 7.00 | 0.77 | 16 336 |
| | ombre | 3.20 | 6.97 | 0.20 | 444 |
| `many_spheres.stage` | primaires | 7.54 | 14.59 | 0.75 | 16 296 |
| | ombre | 22.40 | 38.67 | 2.29 | 9 991 |
| `many_meshes.stage` | primaires | 6.62 | 13.18 | 0.53 | 6 831 |
| | ombre | 9.16 | 16.31 | 1.01 | 2 070 |

`many_spheres` est la scène où l'arbre travaille : 445 primitives réparties dans le volume, donc
14,6 tests de boîte par rayon primaire et 38,7 par rayon d'ombre, contre 3 à 7 sur les scènes à
quatre objets. C'est la première du corpus dont les rayons d'ombre traversent vraiment l'arbre —
61 % d'entre eux sont occultés, par des sphères et non par un mur.

**Les comptes de touches sont un test de régression, pas une statistique.** Tout changement censé
préserver la géométrie doit les reproduire *à l'unité* ; un seul point d'écart est un bug. C'est ce
qui a validé l'inlining du test de boîte, où rien d'autre n'aurait pu le faire.

**Trois pièges de comparaison.** Les scènes de cette section ne sont pas celles des tables de
compteurs du [§2](#2-scène--chronologie) : `cornell_box.stage` a dérivé en vitrine de matériaux et
la boîte canonique vit dans `cornell_box_canonical.stage` (il lui faut `--fov 60 --far 2000`).
`nodes_visited` a changé de définition en cours de route — voir la réserve du
[§1.4](#14-après-suppression-du-double-test-de-boîte--2026-07-31) ; seuls `box_tests` et
`triangle_tests` traversent tout ce fichier avec le même sens. Et le chargement des scènes de
maillage — `bunny_mesh`, `many_meshes` — est un chiffre de parse, pas un chiffre de construction :
le [§0](#0-linstrument) dit dans quel rapport, et [§2.3](#23-le-coût-des-bornes-à-la-construction--mesuré-et-laissé--2026-08-20)
ce qu'il faut mesurer à la place.
