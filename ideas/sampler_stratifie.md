# Sampler stratifié — moins de bruit à échantillonnage égal

Indexé depuis [IDEAS.md](../IDEAS.md). Non fait, et **débloqué** : le chantier du RNG graine a
livré tout ce dont celui-ci a besoin pour exister, et — ce qui compte autant — de quoi le
*mesurer*. Ce que ce chantier a produit est décrit par
[docs/rendu_reproductible.md](../docs/rendu_reproductible.md).

## 1. Le principe, et ce qu'il rapporte

Tirer N nombres indépendants sur [0,1) n'empêche pas qu'ils s'agglutinent : `0.11, 0.13, 0.15,
0.19, 0.92` est un tirage parfaitement légitime qui visite cinq fois un dixième du domaine.
L'estimateur reste sans biais, mais *cette* exécution-là est mauvaise — et « à quel point une
exécution isolée peut être mauvaise » est la définition de la variance.

Le stratifié renonce à l'indépendance pour interdire l'agglutination : N strates de largeur 1/N,
un échantillon dans chacune, tiré uniformément à l'intérieur — `xᵢ = (i + ξᵢ)/N`. Le ξᵢ compte : une
grille régulière serait déterministe, donc biaisée sur les fonctions périodiques.

L'estimateur reste sans biais, la densité valant N sur sa strate et le facteur s'annulant contre le
1/N de la moyenne. Et le gain se calcule exactement. En notant μᵢ et σᵢ² la moyenne et la variance
de f *sur la strate i* :

```text
Monte-Carlo simple :   Var = (1/N²) Σ σᵢ²  +  (1/N²) Σ (μᵢ − μ)²
Stratifié :            Var =  (1/N²) Σ σᵢ²
```

**Le second terme disparaît** — la variance « entre les strates » n'existe plus, puisque la strate
de chaque échantillon est imposée. Donc jamais pire, et strictement meilleur dès que les strates
n'ont pas la même moyenne. Sur une f lipschitzienne, σᵢ² = O(1/N²) et la variance tombe en O(1/N³) :
**le taux de convergence change**, l'erreur passe de N^(−1/2) à N^(−3/2).

## 2. Où ça casse, et le compromis universel

Stratifier un espace de dimension d avec k strates par axe demande k^d échantillons. Un chemin de
ce dépôt sous `--max_depth 3` consomme de l'ordre de 8 dimensions : il faudrait 2⁸ = 256
échantillons par pixel pour obtenir la stratification la plus grossière possible. Structurellement
inutilisable au-delà de deux ou trois dimensions.

Le compromis est de **stratifier chaque dimension séparément** — coût en échantillons : N, quelle
que soit la dimension. On garde que chaque projection 1D est parfaitement étalée ; on perd toute
garantie sur les projections 2D. C'est l'échantillonnage par hypercube latin.

Et il exige une **permutation par dimension**. Si l'échantillon *i* prenait la strate *i* partout,
les N échantillons s'aligneraient sur la diagonale de l'hypercube : chaque projection 1D parfaite,
le nuage catastrophique. La strate de l'échantillon *i* en dimension *d* est donc `permᵈ(i)`, avec
`permᵈ` tirée de `hash(d, graine, pixel)` et calculée à la volée, sans matérialiser le tableau.

Enfin, les quantités **intrinsèquement 2D** — position dans le pixel, point sur la lentille,
direction — veulent une grille √N × √N stratifiée conjointement, et non deux stratifications 1D
indépendantes. C'est ce que `get_2d` existe pour rendre possible.

## 3. Ce qui est déjà acquis

Le chantier du RNG graine a réglé les prérequis, la plupart sans les viser :

- **Les boucles de rejet sont parties.** Un compte de tirages variable décale les dimensions d'une
  quantité qui dépend du pixel, ce qui détruit l'alignement dont une permutation par dimension a
  besoin. C'était le vrai blocage.
- **`Pdf::generate(u)`** — les pdfs reçoivent leur échantillon, donc un sampler stratifié les atteint
  sans qu'aucune ne change.
- **Une caméra tire à travers un `Sampler`**, donc la lentille aussi.
- **La construction par (pixel, échantillon)** donne `sample_index` au constructeur, ce qui est
  exactement ce dont une implémentation stratifiée a besoin pour savoir quelle strate servir.
- **`--seed`** rend la mesure possible : démontrer « moins de bruit à échantillonnage égal » demande
  l'écart-type entre plusieurs réalisations, qu'une seule graine ne peut pas donner
  ([docs/rendu_reproductible.md](../docs/rendu_reproductible.md) §5).

## 4. Ce qui reste à écrire

- `samplers/stratified.rs`, avec un compteur de dimension interne remis à zéro à la construction.
- La permutation à la volée — `PermutationElement` de pbrt, O(1) en mémoire.
- `samples_per_pixel()` sur le trait **si** l'implémentation arrondit au carré parfait que veut une
  grille √N × √N ; ou deux paramètres à la construction comme pbrt, qui évite l'arrondi. C'est la
  seule des trois méthodes retirées du trait qui pourrait revenir.
- Le choix à l'exécution : `--sampler`, et le `match` qui va avec. Il vit à l'unique ligne de
  `compute_pixel` qui construit, et nulle part ailleurs — ne pas rejouer les seize
  `match config.integrator`.
- L'application concentrique du disque en remplacement de la polaire de
  [`thin_lens.rs`](../src/cameras/thin_lens.rs), qui distord moins le carré unité donc préserve
  mieux un échantillon stratifié. Sans conséquence tant que les tirages sont indépendants.

## 5. Comment on saura que ça marche

Dix rendus par sampler, `--seed 0` à `9`, et comparer l'écart-type par pixel. C'est la mesure que
`--seed` a rendue possible, et la seule qui démontre quoi que ce soit : un rendu de chaque ne prouve
rien, l'un pouvant être plus propre par chance.

Les deux tests de conservation d'énergie doivent passer inchangés — un sampler stratifié ne change
pas ce qu'un estimateur estime, seulement sa dispersion.

## 6. Sa limite, à énoncer le jour où il arrive

Stratifier chaque dimension séparément donne un hypercube latin, pas une stratification conjointe de
l'espace des chemins ; le bénéfice décroît avec la dimension (PBR Book §8.6). Il est réel sur la
gigue, la lentille et le premier sommet, et se dégrade là où les chemins d'un même pixel divergent —
le branchement de [`dielectric.rs`](../src/materials/dielectric.rs), et les profondeurs inégales.
Cela affaiblit la garantie d'étalement sans jamais introduire de biais.
