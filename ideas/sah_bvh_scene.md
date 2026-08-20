# Porter le SAH binné sur le BVH de scène — étudié, et garé

Indexé depuis [IDEAS.md](../IDEAS.md). Étudié le 2026-08-18, **non fait et délibérément garé** :
ce fichier existe pour que la décision ne soit pas reprise à zéro, et pour dire ce qui la ferait
changer de camp.

La coupe du BVH de scène est encore une médiane (selon l'axe de plus grand étalement des centroïdes,
depuis que le tirage au hasard est parti). Le SAH binné du maillage pourrait la remplacer et être
partagé plutôt que copié.

## Ce que l'étude a trouvé

**Partageable** : `BIN_COUNT`/`SPLIT_COUNT`, `Bin`, `SplitCandidate`, `bin_index`,
`find_best_split_plane`, `centroid_extent`, `fill_bins`, et `exhaustive_split_cost` pour les tests —
environ 190 lignes, dont ~110 de modèle de coût vivant.

**Non partageable** : la partition (le maillage fait une passe de Lomuto en place sur `tri_idx`, la
scène doit couper en deux un `Vec<T>` qu'elle possède), la disposition des nœuds, la traversée, et
les deux `TraversalStats`. Donc « une seule conception dans le projet » serait exagéré : ~110 lignes
sur quelque 600.

Les deux côtés atteignent le centroïde et la boîte d'un élément différemment, donc un point d'entrée
partagé demande un accesseur. Monomorphisé, donc sans indirection :

```rust
// src/accel.rs — un nouveau module
pub fn find_best_split(
    count: usize,
    centroid_of: impl Fn(usize) -> Vector3f,
    bounds_of: impl Fn(usize) -> AABoundingBox,
) -> Option<SplitCandidate>
```

## Trois raisons pour lesquelles c'est garé

- **Aucun gain mesurable.** Moins de 1,3 test d'objet par rayon sur toutes les scènes
  ([docs/mesures_bvh.md](../docs/mesures_bvh.md) §2) : à 4–10 primitives, l'accélérateur n'est pas là
  où un rayon passe son temps, et à cette taille le SAH et la médiane élisent des plans à peine
  différents.
- **L'argument de la dérive est vide aujourd'hui.** C'est la raison habituelle de partager plutôt que
  copier — mais il n'existe qu'**une** copie du SAH. Partager empêcherait une dérive qui n'existe pas
  encore ; *ne pas* donner de SAH au BVH de scène l'empêche aussi bien, pour zéro ligne.
- **Cela coûte de la lisibilité du côté qui compte.** Dans le maillage, `find_best_split_plane`
  parcourt `node.left_first .. + tri_count` à vue. Derrière un accesseur, cette plage devient un
  décalage que les fermetures doivent porter, et le corps de l'algorithme ne montre plus sur quoi il
  itère. C'est une perte sur la seule pièce de ce chantier au gain mesuré (66 649 → 0,58 test de
  triangle par rayon) et gardée par trois tests.

## Ce qui le ferait sortir du garage

Une scène qui tient beaucoup de primitives — c'est la seule chose qui manque, et elle rend les trois
raisons ci-dessus caduques d'un coup.

**Et alors, dans cet ordre** : corriger d'abord le recalcul de `get_bounding_box` (entrée dans
`IDEAS.md`). Un portage du SAH multiplie ces appels — une passe pour l'étalement des centroïdes, une
pour le binning, par axe, par nœud — donc le poser sur l'accesseur non caché empilerait des appels en
O(V) les uns sur les autres.

## Ce qui aurait mérité l'unité de conception à la place

La traversée à plat et ordonnée — et c'est fait. Les deux accélérateurs y différaient d'une façon qui
*était* mesurée : l'arbre de scène faisait 7,00 tests de boîte par rayon sur un arbre de 7 nœuds,
soit chaque nœud, chaque rayon, sans ordre ni resserrement d'intervalle, quand le maillage avait les
deux. Voir `docs/mesures_bvh.md` §3.2.
