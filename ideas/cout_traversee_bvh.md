# `t_trav` et la taille des feuilles — l'arbre de maillage est deux fois trop gros

Indexé depuis [IDEAS.md](../IDEAS.md). Non fait, et **bloqué** sur le
[sampler graine](rng_graine.md) — la raison est au
[§4](#4-pourquoi-le-balayage-attend-le-sampler-graine), et c'est la partie à ne pas sauter.

Les chiffres cités ici viennent de [docs/mesures_bvh.md](../docs/mesures_bvh.md), qui tient
l'instrument et la chronologie.

## 1. Le fait

Les feuilles du BVH de maillage tiennent **un triangle en moyenne**, donc l'arbre a ~2 nœuds par
triangle : 1 724 381 nœuds pour `dragon_vrip.ply`, 110 Mo à 64 octets le nœud. C'est le `t_trav = 0`
du coût `[5]` de [heuristique_aire_surface.md](../docs/heuristique_aire_surface.md) §6 : avec la
traversée comptée gratuite, découper gagne toujours.

## 2. Deux mauvaises raisons de vouloir le corriger, et la bonne

La formulation d'origine de cette entrée — « donner à `t_trav` le poids de pbrt, ~1/8 d'une
intersection, raccourcirait l'arbre » — est fausse deux fois, et la mesure l'a corrigée :

- **À 1/8, cela ne raccourcit presque rien.** Le terme `t_trav·A` ne mord que là où les deux enfants
  remplissent presque le parent. Pour deux triangles séparés, le coût de coupe s'effondre tandis que
  le coût de feuille vaut `A·N` : la marge est énorme et 1/8 ne la renverse pas.
- **~2 nœuds par triangle est la forme normale d'un build SAH descendant**, pbrt compris : son
  `maxPrimsInNode` est une borne supérieure qui *force* des coupes, et sous trois primitives il coupe
  au milieu sans consulter le modèle de coût du tout.

Ce qui argumente vraiment pour un arbre plus court est **la mémoire**, terme que `[3]` ne sait pas
exprimer. `dragon_vrip` est le seul maillage dont les nœuds dépassent tous les niveaux de cache de la
machine de mesure (110 Mo, contre 8,9 Mo pour `bunny` qui tient) et c'est le seul à casser de 40 %
une droite ns/test-de-boîte par ailleurs remarquablement plate sur quatre ordres de grandeur. Deux
cinquièmes du temps de traversée y sont donc dans un terme absent du modèle de coût, et raccourcir
l'arbre l'attaque indépendamment de l'échange boîtes/triangles.

## 3. Ce que la mesure dit de `t_trav` — plus que rien, moins qu'une calibration

Résoudre des paires de maillages ne donne rien : les comptes de boîtes et de triangles sont
anti-corrélés sur ces maillages — l'arbre s'approfondit à mesure que les feuilles s'amincissent —
donc les deux régresseurs sont colinéaires. Sur les dix paires, celles n'impliquant pas `cube` ont
des déterminants entre 0,5 et 3 et rendent des coûts du genre −90 ns par test de triangle.

La contrainte qui la fait parler est physique et non statistique : `c_box` doit être **le même** sur
les cinq maillages qui tiennent en cache. Balayer un couple partagé (`c_box`, `c_tri`) en minimisant
la dispersion du premier borne alors le second — 4,8 % de dispersion à `c_tri = 0`, 5,5 % à 3 ns,
6,5 % à 6 ns, 10,4 % à 10 ns. Donc `c_box ≈ 17,6–18,5 ns` et **`c_tri ≲ 6 ns`** : un test de
triangle coûte au plus le tiers d'un test de boîte. Un balayage à trois paramètres ajoutant un coût
fixe par rayon laisse ce coût à zéro, donc la génération de rayons est bien sous le bruit. Et le
désassemblage est d'accord indépendamment — `intersect_ray` est inliné, une division, trois sorties
anticipées, 15–20 cycles.

`[2]` facture `t_trav` **par nœud intérieur visité**, et cette traversée teste les deux enfants pour
les ordonner, d'où `box_tests = 2·(nœuds intérieurs) + 1`. Donc
`TRAVERSAL_COST = 2·c_box/c_tri`.

**Révisé par l'inlining du test de boîte** (`docs/mesures_bvh.md` §1.6) : un test de boîte tombe de
~18,4 à ~15,5 ns sur les maillages tenant en cache, et de 25,6 à 21,5 ns sur `dragon_vrip`, tandis que
le test de triangle est intact. `TRAVERSAL_COST` atterrit donc entre **5 et 10** plutôt qu'entre 6 et
12 — toujours quarante fois le 1/8 de pbrt, pour des raisons qui appartiennent à ce code et non à
l'algorithme : notre test de boîte tenait dans un appel non inliné faisant trois divisions, notre test
de triangle est inliné et sort tôt.

**Conséquence attendue sur le build**, depuis `Σ + t_trav·A < A·N` : dès que `N ≤ t_trav` le membre
droit `(N − t_trav)·A` est négatif alors que `Σ ≥ 0`, donc **aucun nœud sous ~7 triangles ne serait
jamais coupé**, et à `N = 16` avec des enfants à 0,55·A il ne coupe que de justesse. Équilibre autour
de 8 à 16 triangles par feuille, 8 à 16 fois moins de nœuds, et `dragon_vrip` de 110 Mo à 7–14 Mo —
dans le cache, donc la pénalité de 40 % du §2 se lève en même temps. Les deux effets poussent dans le
même sens.

Et une correction au plan qui traînait : la plage de balayage était centrée beaucoup trop bas.
`{0, 1, 2, 4, 8, 16}`, pas `{0, 1/8, 1/2, 1}`.

## 4. Pourquoi le balayage attend le sampler graine

Le jeu de rayons de `bvh_stats` est fait de **rayons primaires cohérents** : ils partent d'un même
point, voyagent ensemble et rentrent dans des nœuds encore chauds. C'est la population qui favorise
le *plus* un arbre profond, puisqu'elle amortit la mémoire d'un nœud sur des rayons voisins. Les
rayons secondaires partent de partout, paient le nœud plein tarif, et pousseraient l'optimum vers un
arbre encore plus plat.

Élire `t_trav` sur les seuls primaires figerait donc dans **tous** les builds que le renderer fera
un arbitrage mesuré sur un cinquième du problème. Le balayage attend le sampler graine.

C'est exactement ce qui n'a *pas* bloqué l'inlining du test de boîte, et c'est pourquoi celui-là est
passé d'abord : un test de boîte moins cher est un gain pour tous les rayons qui existent, sous
aucune hypothèse sur leur distribution.

## 5. Ce que serait le travail

- Nommer la constante et la documenter (pas de constante magique nue, CLAUDE.md §3) : elle est un
  rapport de coûts mesurés, donc son commentaire doit dire *sur quelle machine* et *contre quel test
  de triangle*, faute de quoi elle se périme en silence.
- La faire entrer dans la comparaison de `subdivide` sans casser la convention de coût : le
  `find_best_split_plane` actuel rend `A_L·N_L + A_R·N_R` et la comparaison se fait contre
  `A_node·N`, aucun des deux normalisé par `A_node`. Ajouter `t_trav·A_node` d'un seul côté est
  précisément ce qui redéfinirait le test en silence.
- Balayer `{0, 1, 2, 4, 8, 16}` sur les six maillages **et** sur des rayons secondaires, chronomètre
  à l'appui, et publier le tableau dans `docs/mesures_bvh.md`.
- Vérifier que les comptes de touches restent identiques à l'unité : la profondeur de l'arbre change,
  la géométrie non.
