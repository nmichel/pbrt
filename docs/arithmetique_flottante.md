# Arithmétique flottante : u, δ et γ(n)

Ce document explique le modèle d'erreur qui sous-tend l'élargissement conservateur des slabs
dans [`AABoundingBox::hit`](../src/geom/aabound.rs) — les constantes `UNIT_ROUNDOFF`,
`GAMMA_3` et `SLAB_WIDENING`.

Il se lit en deux moitiés. Les §0 à §3 posent le cadre général : comment les `f64` représentent
les nombres, pourquoi toute borne d'erreur doit être *relative*, et ce que sont u, δᵢ et γ(n).
La §4 s'en sert pour dériver l'élargissement du slab, étape par étape.

**C'est ici que vit la dérivation détaillée**, avec ses intervalles calculés et ses
contre-exemples ; le doc-comment de `hit` en donne une version resserrée, suffisante pour lire
le code sans quitter le fichier. En cas de doute sur une étape, c'est la §4 qu'il faut ouvrir.

Référence : Nicholas J. Higham, *Accuracy and Stability of Numerical Algorithms*, 2ᵉ éd.,
chapitre 2 (représentation) et §3.1 (le modèle et γ(n)).

---

## 0. La source du problème : la représentation des flottants

### Un ensemble fini de valeurs

Un `f64` occupe 64 bits, répartis en trois champs (IEEE-754, format *binary64*) :

| champ | bits | rôle |
|---|---|---|
| signe s | 1 | ± |
| exposant e | 11 | l'ordre de grandeur, biaisé de 1023 |
| mantisse m | 52 | les chiffres significatifs |

Pour un nombre normalisé, la valeur représentée est

```text
(−1)ˢ × 1,m₁m₂…m₅₂ × 2ᵉ⁻¹⁰²³
```

Le `1,` initial est **implicite** : en base 2 le premier chiffre significatif d'un nombre non
nul vaut nécessairement 1, il est donc inutile de le stocker. On dispose ainsi de 53 bits
significatifs pour 52 bits stockés.

La conséquence de fond : 64 bits ne codent que 2⁶⁴ valeurs distinctes, alors que ℝ est
indénombrable. **Presque aucun réel n'est représentable.** Un `f64` n'est pas un nombre réel,
c'est le représentant d'un intervalle de réels, et toute opération doit choisir un
représentant pour son résultat — c'est l'arrondi.

### La mantisse est binaire, pas décimale

L'exemple canonique : 0,1 n'est pas représentable, parce que 1/10 n'a pas d'écriture finie en
base 2 (10 = 2 × 5, et le facteur 5 produit un développement périodique, exactement comme 1/3
en base 10). Le `f64` le plus proche de 0,1 vaut

```text
0,1000000000000000055511151231257827021181583404541015625
```

D'où le classique :

```text
0,1 + 0,2 = 0,30000000000000004
```

Ce n'est pas un bug de l'addition — les deux opérandes étaient déjà faux avant elle. Ici
l'erreur devient *visible* parce que la somme des deux approximations tombe à côté du
représentant de 0,3, mais elle est présente dans tout calcul.

Le critère de représentabilité n'est donc pas le *nombre* de décimales, mais d'être un rationnel
**dyadique**, k/2ⁿ :

```text
0,5   0,25   0,125   0,00390625   →  exacts : 2⁻¹, 2⁻², 2⁻³, 2⁻⁸
0,1   0,2    0,3                  →  jamais exacts, à aucune magnitude
```

0,1 n'a qu'une décimale et n'est représentable nulle part ; 0,00390625 en a huit et est exact
partout où l'exposant le permet. « Peu de chiffres après la virgule » ne veut rien dire ici —
seule compte la fraction.

### L'espacement n'est pas uniforme

C'est le point qui gouverne tout le reste, et celui qu'on oublie le plus facilement.
L'exposant multiplie la mantisse par une puissance de 2 : la grille des flottants n'est donc
pas régulière, elle est **logarithmique par paliers**. Entre deux puissances de 2 successives
(un *binade*) l'espacement est constant ; il double à chaque binade franchi.

L'écart entre deux flottants voisins s'appelle un **ULP** (*unit in the last place*) :

| autour de | ULP | ordre |
|---|---|---|
| 2⁻¹⁰⁰ | 2⁻¹⁵² | ≈ 1,75·10⁻⁴⁶ |
| 0,5 | 2⁻⁵³ | ≈ 1,11·10⁻¹⁶ |
| 1 | 2⁻⁵² | ≈ 2,22·10⁻¹⁶ |
| 100 | 2⁻⁴⁶ | ≈ 1,42·10⁻¹⁴ |
| 10⁶ | 2⁻³³ | ≈ 1,16·10⁻¹⁰ |
| 10¹⁵ | 2⁻³ | 0,125 |
| 10¹⁶ | 2¹ | 2 — au-delà de 2⁵³ les entiers consécutifs ne sont plus tous représentables |

Chaque binade contient le **même nombre** de flottants, 2⁵², quelle que soit sa largeur : il y
en a autant entre 1 et 2 qu'entre 1024 et 2048, et autant entre 0,5 et 1 — dans un intervalle
deux fois plus court, donc deux fois plus dense.

Les deux extrémités de la plage donnent la mesure de l'amplitude. L'ULP maximal est celui de la
dernière binade des normalisés, [2¹⁰²³, 2¹⁰²⁴) : il vaut 2⁹⁷¹ ≈ 2·10²⁹². Ainsi `f64::MAX` et son
voisin immédiat, qui ne diffèrent que par leur dernier chiffre affiché, sont séparés de 2·10²⁹².
Le minimum est le pas des dénormalisés, 2⁻¹⁰⁷⁴ ≈ 5·10⁻³²⁴. **L'ULP parcourt donc un facteur
2²⁰⁴⁵ ≈ 10⁶¹⁵** d'un bout du type à l'autre.

### Deux conclusions à ne pas tirer

Le tableau invite à deux contresens, qui sont les deux faces d'une même erreur.

**1 n'est pas le point de densité maximale.** La densité croît de façon monotone *vers* 0, sans
borne : les binades rétrécissent géométriquement en descendant, tout en gardant leurs 2⁵²
valeurs. Autour de 2⁻¹⁰⁰ la grille est ainsi 10³⁰ fois plus fine qu'autour de 1. Cela se
poursuit jusqu'au plancher des **dénormalisés**, à 2⁻¹⁰²² ≈ 2,2·10⁻³⁰⁸ : en dessous, l'exposant
est saturé, le `1,` implicite est abandonné — la mantisse perd alors réellement des bits
significatifs — et l'espacement se fige à 2⁻¹⁰⁷⁴ ≈ 5·10⁻³²⁴. C'est là, et là seulement, que la
densité est maximale. Les exposants des normalisés allant de −1022 à +1023, 1 se situe au
milieu de la plage, avec un peu plus de la moitié des flottants positifs en dessous de lui.

**La précision, elle, ne varie pas.** L'ULP de la première ligne du tableau n'est pas une
résolution « meilleure » que celle de la dernière : c'est la même résolution *relative*,
appliquée à un nombre plus petit. La mantisse porte exactement 53 bits significatifs dans
toutes les binades (jusqu'aux dénormalisés, précisément le cas où elle en perd). Ce que
l'exposant met à l'échelle est l'espacement **absolu**, pas la précision — rien n'est « dilué »
en montant, rien n'est gagné en descendant. Cette invariance d'échelle de la précision relative
est *tout le projet* du virgule flottante, par opposition au virgule fixe, où la résolution est
absolue et où les grands nombres sont bel et bien représentés plus grossièrement en relatif.

### Le budget de précision glisse

La bonne façon de se représenter ces 53 bits est un **budget constant que l'exposant déplace** :
plus la partie entière en consomme, moins il en reste pour la partie fractionnaire.

```text
123456789,123456789  →  123456789,12345679
                        └── 9 ──┘ └── 8 ─┘
                        entier     fraction
```

Les 18 chiffres significatifs fournis en entrée ne survivent pas ; ce sont ceux de droite qui
tombent, parce que la partie entière a déjà pris sa part du budget.

Attention à la lecture : le dernier chiffre passe de 8 à 9, ce qui ressemble à une coquille. Rien
n'est tronqué, c'est un **report d'arrondi** — la queue `…12345678|9` s'arrondit au plus proche
sur huit chiffres, et le 9 final reporte sur le 8 qui le précède. Le `f64` retenu vaut exactement

```text
123456789,12345679104328155517578125
```

et il faut bien 17 chiffres pour le désigner sans ambiguïté : `123456789,1234568`, la même valeur
à 16 chiffres, est un *autre* flottant. L'exemple relève donc du troisième régime du tableau
ci-dessous.

Ce qui est constant, c'est **53 bits** — pas un nombre de chiffres décimaux. La conversion vaut
53·log₁₀2 ≈ 15,95, et cette non-intégralité n'est pas un arrondi de commodité : elle est
structurelle, puisque 2ᵃ = 10ᵇ n'a pas d'autre solution que a = b = 0. C'est le même facteur 5
qui empêche 0,1 d'être représentable. La grille des `f64` est donc *strictement entre* celle des
décimaux à 15 chiffres et celle à 16 :

```text
10¹⁵  <  2⁵³ = 9 007 199 254 740 992  <  10¹⁶
```

D'où trois régimes, et une zone ambiguë entre deux garanties nettes :

| | aller-retour |
|---|---|
| **15 chiffres** | décimal → `f64` → décimal **toujours** fidèle (10¹⁵ < 2⁵³ : pas de collision possible) |
| **16 chiffres** | parfois fidèle, parfois non |
| **17 chiffres** | `f64` → décimal → `f64` **toujours** exact (2⁵³ < 10¹⁷) |

```text
16 chiffres, collision :
  "9007199254740992" → 9007199254740992
  "9007199254740993" → 9007199254740992      ← deux décimaux, un seul f64

17 chiffres, nécessaires :
  0,1 + 0,2 = 0,30000000000000004            ← 17 chiffres significatifs
  arrondi à 16 → 0,3, qui est un autre f64 : l'aller-retour est perdu
```

C'est l'origine de `f64::DIGITS == 15` — la garantie plancher — et du choix de 17 chiffres,
⌈53·log₁₀2⌉ + 1, par les afficheurs qui veulent la fidélité au bit près.

Noter enfin que 15,95 est un **pire cas**, pas une constante : le *wobble* de la §1 vaut log₁₀2 ≈
0,3 chiffre décimal. En bas d'une binade l'erreur relative vaut u, soit log₁₀(1/u) = 15,95
chiffres ; en haut, u/2, soit 16,26. C'est la valeur à citer pour une garantie, pas une propriété
invariante.

Poussé assez loin, le budget fractionnaire s'épuise complètement. Le nombre de bits disponibles
après la virgule vaut 52 − e, donc :

| binade | ULP | représentable |
|---|---|---|
| [2⁵¹, 2⁵²) ≈ 2,3·10¹⁵ | 0,5 | entiers et demis |
| **[2⁵², 2⁵³) ≈ 4,5·10¹⁵** | **1** | **les entiers, rien d'autre** |
| [2⁵³, 2⁵⁴) ≈ 9,0·10¹⁵ | 2 | un entier sur deux |
| [2⁵⁴, 2⁵⁵) | 4 | un entier sur quatre |

```text
2⁵² + 0,5 == 2⁵²   →  vrai    plus aucune partie fractionnaire
2⁵³ + 1   == 2⁵³   →  vrai    les entiers consécutifs y passent
2⁵³ + 2   == 2⁵³   →  faux
```

Au-delà de 2⁵³ ≈ 9,007·10¹⁵, ce ne sont plus les décimales qui manquent mais les **entiers**. Ce
n'est pas un défaut des grands nombres : ils n'ont pas « moins de précision », ils ont dépensé
leur budget à gauche de la virgule.

### Corollaire pratique

Une tolérance *absolue* n'a aucun sens sur toute la plage. Un `epsilon = 1e-9` est monstrueusement
grossier près de 0 — il enjambe 4,47·10¹⁸ flottants, soit près de la moitié de tous les flottants
positifs — et strictement nul au-delà de 2²⁴ ≈ 1,7·10⁷, où il devient plus petit qu'un demi-ULP et
où `x + 1e-9 == x`. Aucune constante ne peut être pertinente aux deux bouts d'une échelle d'ULP de
10⁶¹⁵ ordres de grandeur.

C'est la raison pour laquelle une comparaison `(a - b).abs() < EPS` est fragile, et la raison pour
laquelle [`aabound.rs`](../src/geom/aabound.rs) écrit `t0.abs() * SLAB_WIDENING` et non
`t0 - EPSILON` : la seule borne qui tienne sur toute la plage est **relative**, et une borne
relative *s'écrit comme un facteur multiplicatif* — un facteur suit l'échelle, un terme ne la suit
pas. Tout ce qui suit découle de là.

C'est aussi ce qui impose de garder les coordonnées d'une scène dans une plage raisonnable. À
10⁰–10³ l'ULP vaut 10⁻¹⁶ à 10⁻¹³ et tout va bien. Une scène modélisée à une échelle démesurée
verrait sa géométrie se quantifier grossièrement, et l'acné de surface réapparaîtrait quel que soit
le soin apporté au décalage anti-acné — parce que le décalage lui-même deviendrait plus petit qu'un
ULP, donc littéralement inopérant. Un décalage ou une tolérance ne signifie rien tant qu'on ne le
compare pas à l'ULP local.

---

## 1. u — l'*unit roundoff*

`u` majore l'erreur **relative** d'un arrondi. Pour `f64` :

```text
u = ε/2 = 2⁻⁵³ ≈ 1,11·10⁻¹⁶
```

où ε = 2⁻⁵² = `f64::EPSILON` est l'écart entre 1,0 et le flottant **suivant**.

Ce « suivant » n'est pas une précaution de style. 1 est une puissance de 2, donc une *frontière*
de binade : la grille change de pas exactement là, et l'espacement est asymétrique autour de 1.

```text
nextafter(1, +∞) = 1 + 2⁻⁵²
nextafter(1, 0)  = 1 − 2⁻⁵³        ← moitié moins loin
```

Ce qui singularise 1 n'est donc ni un record de densité (§0) ni une frontière : c'est d'être la
binade **dont le facteur d'échelle vaut 1**, e = 0 donc 2ᵉ = 1. L'espacement absolu et
l'espacement relatif y coïncident numériquement. ε est ainsi l'espacement relatif *mesuré à
l'échelle unité* — d'où le *unit* de « unit in the last place ». Son intérêt n'est pas d'être le
plus petit, c'est d'être une **ancre de normalisation** : pris là où le facteur d'échelle est
neutre, ε est un nombre pur, transportable à n'importe quelle magnitude par une simple
multiplication. C'est ce qui fait fonctionner `t0.abs() * SLAB_WIDENING` aussi bien à t = 10⁻³
qu'à t = 10⁶.

Le facteur 1/2 vient du mode d'arrondi par défaut, *round to nearest* : le résultat exact
tombe quelque part entre deux flottants voisins, et le plus proche des deux est à moins d'un
**demi**-espacement. D'où u = ε/2.

Attention à la nuance : ε est un écart *absolu*, u est une borne *relative* valable partout.
Pour x dans la binade [2ᵉ, 2ᵉ⁺¹), l'erreur relative d'un demi-ULP vaut 2ᵉ⁻⁵³/x, donc elle est
maximale quand x est le plus petit de sa binade :

```text
x → (2ᵉ)⁺    : erreur relative ≤ 2⁻⁵³ = u        ← le pire cas
x → (2ᵉ⁺¹)⁻  : erreur relative ≤ 2⁻⁵⁴ = u/2      ← le meilleur
```

Le pire cas se situe donc **juste au-dessus d'une puissance de 2** — de n'importe laquelle, pas
seulement de 1. Cette variation d'un facteur 2 à l'intérieur d'une binade est ce que Goldberg
appelle le *wobble* de la précision relative ; il vaut la base, donc 2 ici. `u` est le majorant
sur tout le wobble, et c'est ce qui en fait une borne valide à toute magnitude.

Dans le code : [`UNIT_ROUNDOFF`](../src/geom/aabound.rs).

---

## 2. δᵢ — l'erreur relative effectivement commise par l'opération i

IEEE-754 garantit que chaque opération élémentaire (`+`, `−`, `×`, `/`, `sqrt`) est
**correctement arrondie** : le matériel se comporte comme s'il calculait le résultat exact
puis l'arrondissait, une seule fois. C'est une garantie forte, et c'est elle qui rend
l'analyse possible. Elle s'écrit :

```text
fl(a ⊙ b) = (a ⊙ b)(1 + δ),   |δ| ≤ u
```

`δ` est donc simplement « ce qui a été perdu à cet arrondi », exprimé sous forme
multiplicative : le résultat calculé est le résultat exact, à un facteur très proche de 1
près. C'est une valeur **inconnue mais bornée** — on ne sait pas laquelle, seulement qu'elle
vit dans [−u, u]. C'est un objet d'analyse, pas une quantité qu'on pourrait lire à
l'exécution.

L'indice `i` numérote les opérations du calcul : chaque arrondi a **son propre** δ, sans
relation avec les autres (ni même signe, ni indépendance statistique — on ne suppose rien).
Trois arrondis, trois δ.

Pourquoi cette écriture multiplicative plutôt qu'additive ? Parce que les erreurs relatives se
**composent par produit**, ce qui rend la propagation triviale à travers une chaîne
d'opérations. Une écriture additive obligerait à traîner des termes dépendant des valeurs
intermédiaires. C'est tout l'intérêt du modèle.

### La cascade dans `hit`

Les trois opérations de [`hit`](../src/geom/aabound.rs) composent parce que chacune prend
en entrée le résultat **déjà arrondi** de la précédente :

```rust
let inv_dir = 1.0 / ray.direction[i];                    // δ₂
let t0 = (self.bmin[i] - ray.origin[i]) * inv_dir;       // δ₁ puis δ₃
```

```text
fl(bmin[i] − o[i]) = (bmin[i] − o[i])(1 + δ₁)                        [la soustraction]
fl(1 / d[i])       = (1 / d[i])(1 + δ₂)                              [la réciproque]

t̃ = fl( fl(bmin[i] − o[i]) × fl(1/d[i]) )
   = (bmin[i] − o[i])(1 + δ₁) · (1/d[i])(1 + δ₂) · (1 + δ₃)          [la multiplication]
   = t (1 + δ₁)(1 + δ₂)(1 + δ₃)
```

Noter que **n = 3 est une propriété du code, pas de la formule** : écrite en division directe
`(bmin[i] − o[i]) / d[i]`, l'expression n'arrondirait que deux fois. La réciproque est extraite
du quotient pour être partagée entre les deux plans du slab — ce qui ajoute un troisième
arrondi, dont l'erreur entre dans les deux t.

---

## 3. γ(n) — le majorant du produit de n facteurs

Le produit (1 + δ₁)…(1 + δₙ) reste proche de 1, mais il faut un majorant maniable de son écart
à 1. Développé :

```text
(1 + δ₁)…(1 + δₙ) = 1 + Σδᵢ + Σᵢ<ⱼ δᵢδⱼ + … 
```

soit 1 + (quelque chose de l'ordre de n·u), plus des termes en u², u³… négligeables
numériquement mais qu'une borne rigoureuse doit couvrir. Le lemme 3.1 de Higham le fait :

> Si |δᵢ| ≤ u pour i = 1…n, et si n·u < 1, alors
> ```text
> ∏ᵢ (1 + δᵢ)^ρᵢ = 1 + θₙ   avec |θₙ| ≤ γ(n),   γ(n) = n·u / (1 − n·u)
> ```
> où chaque ρᵢ vaut +1 ou −1.

Autrement dit : **γ(n) ≈ n·u, très légèrement gonflé.** Le dénominateur (1 − n·u) est la marge
qui absorbe tous les termes d'ordre supérieur en une forme close. γ(n) répond à la question
« n arrondis en cascade, ça dérive de combien au pire ? », et la réponse est « à peu près n
fois un arrondi » — intuitif, mais désormais démontré plutôt que supposé.

Deux propriétés justifient qu'on préfère cette forme au majorant naïf (1 + u)ⁿ − 1 :

- **Les exposants ρᵢ peuvent être négatifs**, c'est-à-dire que le lemme couvre aussi les
  divisions. C'est ce qui autorise le passage de t̃ = t(1 + e) à t = t̃/(1 + e) dans la
  dérivation de `hit`.
- **γ se compose** : γ(m) + γ(n) + γ(m)γ(n) ≤ γ(m + n). On peut donc chaîner des étapes déjà
  analysées séparément sans reprendre l'analyse à zéro.

Dans le code : [`GAMMA_3`](../src/geom/aabound.rs).

### Nommer le produit : le passage à t̃ = t(1 + e)

Le lemme énonce une borne sur `|∏(1 + δᵢ) − 1|`, et la suite de la dérivation travaille sur
`t̃ = t(1 + e)` avec `|e| ≤ γ(3)`. Le passage de l'un à l'autre se lit volontiers comme une
inférence — il n'en est pas une : **rien n'est déduit ici, on pose une définition.**

On baptise l'écart du produit à 1 :

```text
e := (1 + δ₁)(1 + δ₂)(1 + δ₃) − 1
```

C'est un simple changement de notation. Par transposition immédiate,
(1 + δ₁)(1 + δ₂)(1 + δ₃) = 1 + e, et en substituant dans l'expression de la §2 :

```text
t̃ = t (1 + δ₁)(1 + δ₂)(1 + δ₃) = t (1 + e)
```

Quant à |e| ≤ γ(3), ce n'est pas non plus une conséquence du lemme : **c'est le lemme**, réécrit
avec le nom qu'on vient de donner. La quantité que Higham majore est précisément e — c'est son
θₙ, et les deux formulations `|∏(1 + δᵢ) − 1| ≤ γ(n)` et `∏(1 + δᵢ) = 1 + θₙ, |θₙ| ≤ γ(n)` sont
la même phrase.

**À quoi ressemble e.** Développé pour n = 3 :

```text
e = δ₁ + δ₂ + δ₃  +  δ₁δ₂ + δ₁δ₃ + δ₂δ₃  +  δ₁δ₂δ₃
    └── ordre u ──┘    └──── ordre u² ───┘    └ u³ ┘
```

Le terme dominant est la **somme** des trois erreurs, d'où |e| ≲ 3u — l'intuition « trois
arrondis dérivent d'environ trois fois un arrondi ». Les termes croisés valent de l'ordre de
10⁻³² et sont numériquement invisibles, mais une borne rigoureuse doit les couvrir : c'est le
rôle du dénominateur de γ(3) = 3u/(1 − 3u), qui gonfle 3u juste assez pour les absorber.

**Pourquoi se donner cette peine.** Le baptême **effondre trois inconnues en une**. Avant : trois
quantités indépendantes δ₁, δ₂, δ₃, chacune dans [−u, u], soit un domaine de dimension 3. Après :
une seule quantité e dans [−γ(3), γ(3)], soit un segment. C'est ce qui rend l'inversion de la §4
faisable — il suffira d'étudier une fonction d'une seule variable au lieu de majorer un produit
de trois facteurs sur un cube.

---

## 4. De la borne à l'élargissement du slab

Nous disposons de t̃ = t(1 + e) avec |e| ≤ γ(3). C'est le mauvais sens : cette relation exprime
le t̃ **calculé** en fonction du t **exact**, alors que le seul des deux dont nous disposons à
l'exécution est t̃. Il faut donc l'inverser,

```text
t = t̃ / (1 + e)
```

et, e restant inconnu, produire un **intervalle qui contient certainement t**. La dérivation
procède en trois étapes, dont les deux dernières sont des relâchements volontaires.

### Étape A — les extremums sont aux bords

`1/(1 + e)` est strictement monotone en e (décroissante). Une fonction strictement monotone n'a
**pas d'extremum intérieur** : sur e ∈ [−γ, γ], ses valeurs extrêmes ne peuvent être atteintes
qu'aux deux bornes du segment. C'est ce qui permet de ne tester que deux valeurs de e au lieu
d'un continuum — sans la monotonie, parler des « valeurs extrêmes » de 1/(1 + e) sur le segment
n'aurait aucune raison de se réduire à l'examen de ses deux bouts.

Plaçons-nous d'abord dans le cas **t̃ > 0**, qui fixe quelle extrémité est le minimum :

```text
e = +γ  →  t = t̃/(1 + γ)      la plus petite valeur que t puisse prendre
e = −γ  →  t = t̃/(1 − γ)      la plus grande

t ∈ [ t̃/(1 + γ) ,  t̃/(1 − γ) ]              [A]  exact, mais avec des divisions
```

`[A]` est déjà un encadrement correct et il est le plus serré possible. Son seul défaut est de
contenir des divisions.

### Étape B — troquer les divisions contre des multiplications

C'est le rôle des deux inégalités `[2]` et `[3]` du doc-comment. Chacune remplace une division
par une multiplication, au prix d'une borne **moins serrée**. Le point qui gouverne tout : chaque
relâchement doit aller **vers l'extérieur**, faute de quoi l'encadrement cesse d'en être un. La
minoration ne peut que descendre, la majoration que monter. C'est ce qui fixe le sens de chaque
inégalité — et non un choix esthétique :

```text
t ≥ t̃/(1 + γ)   il faut quelque chose de PLUS PETIT  →  1/(1 + γ) ≥ 1 − γ     [2]
t ≤ t̃/(1 − γ)   il faut quelque chose de PLUS GRAND  →  1/(1 − γ) ≤ 1 + 2γ    [3]

t ∈ [ t̃(1 − γ) ,  t̃(1 + 2γ) ]               [B]  multiplications, asymétrique
```

Les deux preuves, élémentaires :

```text
[2]  (1 − γ)(1 + γ) = 1 − γ² ≤ 1, et (1 + γ) > 0, donc 1 − γ ≤ 1/(1 + γ)

[3]  (1 + 2γ)(1 − γ) = 1 + γ − 2γ² = 1 + γ(1 − 2γ) ≥ 1  ⟺  γ ≤ 1/2
     et (1 − γ) > 0, donc 1/(1 − γ) ≤ 1 + 2γ
```

La condition γ ≤ 1/2 de `[3]` est satisfaite de quinze ordres de grandeur avec γ(3) ≈ 3,3·10⁻¹⁶.

L'asymétrie de `[B]` — γ en dessous, 2γ au-dessus — n'est pas un artefact : elle reflète le fait
que 1/(1 + e) s'écarte plus vite de 1 vers le haut que vers le bas.

### Étape C — symétriser, puis passer en magnitude

Deux derniers relâchements, tous deux vers l'extérieur, tous deux pour l'uniformité et non pour
la justesse.

**C.1 — on prend le pire des deux côtés.** Comme 2γ > γ, remplacer `(1 − γ)` par `(1 − 2γ)`
fait **descendre** la borne inférieure :

```text
t̃(1 − 2γ)  <  t̃(1 − γ)                      (pour t̃ > 0)

t ∈ [ t̃(1 − 2γ) ,  t̃(1 + 2γ) ]              [C]  symétrique
```

On perd donc délibérément de la finesse. La validité est gratuite : si t ≥ t̃(1 − γ), alors
*a fortiori* t ≥ t̃(1 − 2γ). Affaiblir une minoration ne peut jamais la rendre fausse.

Avec γ = 0,1 (exagéré pour la lisibilité) et t̃ = 5, on voit chaque étape s'élargir sans jamais
lâcher la valeur exacte :

| | intervalle | contient le t exact ? |
|---|---|---|
| t réellement possible | [4,5455 ; 5,5556] | — |
| `[A]` exact | [4,5455 ; 5,5556] | oui, au plus serré |
| `[B]` après [2] et [3] | [4,5000 ; 6,0000] | oui |
| `[C]` symétrique | [4,0000 ; 6,0000] | oui |

**C.2 — ce que la symétrie achète : l'indifférence au signe.** C'est la vraie raison, et elle
n'apparaît qu'avec t̃ < 0. Toute l'étape A supposait t̃ > 0 ; c'est cette hypothèse qui a décidé
laquelle des deux extrémités était le minimum. Pour t̃ < 0 la monotonie échange les deux rôles,
et la formule asymétrique `[B]` **cesse purement et simplement d'être valide** :

```text
t̃ = −5,  γ = 0,1

t réellement possible                        [−5,5556 ; −4,5455]

[B] = [ t̃(1 − γ) , t̃(1 + 2γ) ] = [−4,5 ; −6,0]     ← incohérent, et faux :
                                                     −5,5556 n'est pas ≥ −4,5
[ t̃ − |t̃|·2γ , t̃ + |t̃|·2γ ]   = [−6,0 ; −4,0]     ← correct
```

Les facteurs `(1 − γ)` et `(1 + 2γ)` sont attachés à un *rôle* — minorant, majorant — qui dépend
du signe de t̃. Tant que la borne reste asymétrique, il faut donc savoir de quel côté on se
trouve, donc tester le signe. Une fois symétrique, la distinction s'évapore : il ne reste que
« écarter de 2γ|t̃| de part et d'autre », qui ne demande plus de connaître le signe.

**La symétrisation est donc ce qui rend possible l'écriture en magnitude.** Les deux moitiés de
l'étape C ne sont pas deux simplifications indépendantes : la première est la condition de la
seconde.

```text
t ∈ [ t̃ − |t̃|·2γ(3) ,  t̃ + |t̃|·2γ(3) ]      [4]  valable quel que soit le signe de t̃
```

C'est `SLAB_WIDENING = 2·γ(3)`.

### Le récapitulatif

```text
t = t̃/(1 + e),  |e| ≤ γ
   │
   │ monotonie de 1/(1+e)              → les extremums sont aux bords
   ▼
t ∈ [ t̃/(1 + γ) , t̃/(1 − γ) ]          [A]  exact, avec divisions
   │
   │ [2] et [3], orientées vers l'extérieur
   ▼
t ∈ [ t̃(1 − γ) , t̃(1 + 2γ) ]           [B]  multiplications, asymétrique
   │
   │ pire cas des deux côtés
   ▼
t ∈ [ t̃(1 − 2γ) , t̃(1 + 2γ) ]          [C]  symétrique
   │
   │ écriture en magnitude
   ▼
t ∈ [ t̃ − |t̃|·2γ , t̃ + |t̃|·2γ ]        [4]  insensible au signe
```

### Ce que cela donne dans le code

L'insensibilité au signe est exactement ce qui autorise les deux lignes de
[`hit`](../src/geom/aabound.rs) :

```rust
t0 = t0 - t0.abs() * SLAB_WIDENING;
t1 = t1 + t1.abs() * SLAB_WIDENING;
```

`t0` et `t1` sortent d'un `swap` conditionnel et peuvent chacun être de n'importe quel signe :
une boîte entièrement derrière le rayon donne deux t négatifs, une boîte contenant l'origine un
de chaque. Une constante unique, aucun test de signe, aucune branche.

### Le coût de ces relâchements

Nul, en pratique. γ(3) ≈ 3,3·10⁻¹⁶ contre 2γ(3) ≈ 6,7·10⁻¹⁶ : on double un élargissement déjà
invisible, qui reste entre trois et six ULP. On dépense un facteur 2 sur une quantité négligeable
pour acheter la disparition d'un cas particulier — l'arbitrage que la directive de projet
privilégie explicitement.

### Pourquoi élargir, et jamais rétrécir

Chaque intervalle de slab est élargi **vers l'extérieur** avant intersection : t₀ abaissé,
t₁ relevé. L'asymétrie est délibérée : élargir ne peut transformer qu'un raté en touche, jamais
l'inverse. Un faux négatif fait disparaître de la géométrie de l'image — un trou, un défaut
visible et difficile à diagnostiquer. Un faux positif coûte un test de primitive redondant. Le
choix est donc gratuit du point de vue de la justesse et payé en temps de calcul, ce qui est le
bon sens de l'échange pour un bounding volume, conservateur par nature.

---

## 5. Ordres de grandeur

| quantité | expression | valeur |
|---|---|---|
| ε (`f64::EPSILON`) | 2⁻⁵² | ≈ 2,22·10⁻¹⁶ |
| u (`UNIT_ROUNDOFF`) | ε/2 = 2⁻⁵³ | ≈ 1,11·10⁻¹⁶ |
| γ(3) (`GAMMA_3`) | 3u/(1 − 3u) | ≈ 3,33·10⁻¹⁶ |
| 2γ(3) (`SLAB_WIDENING`) | — | ≈ 6,66·10⁻¹⁶ |

Comme 2γ(3) ≈ 6u = 3ε, l'élargissement vaut **entre 3 et 6 ULP** selon la position de la
mantisse dans son binade — quelques flottants, jamais une distance géométrique. Pour un t de
l'ordre de 100, cela fait ≈ 6,7·10⁻¹⁴ unités de scène.

C'est exactement le point : la correction est la plus petite qui soit *démontrablement*
suffisante. À comparer avec ce qu'elle a remplacé, un gonflage de boîte à 0,01 par axe — treize
ordres de grandeur de trop, et surtout une valeur absolue, donc à la fois excessive près de
l'origine et insuffisante loin d'elle. Ce gonflage faussait de plus toute évaluation du SAH,
puisqu'une boîte épaissie déclare une aire qu'elle n'a pas ; c'est ce que verrouille
`test_degenerate_box_area_is_faithful`. Pourquoi cette aire gouverne le choix d'un plan de
découpe, et ce que le débordement d'une boîte vide y provoquait, sont traités dans
[heuristique_aire_surface.md](heuristique_aire_surface.md) §1 et §5.

`test_rounding_bound_magnitude` garde les constantes : si γ(3) sortait de [3u, 4u], ou si
`SLAB_WIDENING` dépassait 10⁻¹⁵, c'est que l'élargissement serait redevenu l'epsilon
macroscopique qu'il a chassé.

---

## 6. Ce que la borne ne couvre pas

Une borne d'erreur ne vaut que par la précision de son périmètre. Celle-ci couvre
**l'arithmétique du test de slab, et rien d'autre**.

- **L'erreur déjà présente dans les entrées n'est pas comptée.** `ray.direction` est le
  résultat d'une normalisation, `ray.origin` d'un décalage anti-acné et possiblement d'une
  transformation ; les sommets de la boîte viennent d'un `combine_with` sur des primitives
  elles-mêmes transformées. Chacune de ces erreurs se propage dans t et n'entre pas dans
  γ(3). Traiter cela demanderait de propager des bornes depuis la construction de la scène —
  ce que fait pbrt avec son type `EFloat`.
- **Les deux arrondis de l'élargissement lui-même ne sont pas comptés**, mais la marge les
  absorbe. `t0 - t0.abs() * SLAB_WIDENING` ajoute un arrondi sur la multiplication — d'ordre u²
  puisqu'il porte sur une quantité déjà en 6u, donc négligeable — et un sur la soustraction,
  d'ordre u sur t₀, donc de premier ordre. Ce dernier peut amputer l'élargissement d'un u :

  ```text
  2γ(3) appliqué par le code                              6 u
  réellement délivré, (1 + 2γ(1 − u))(1 − u) − 1          5 u
  exigence de l'étape A, 1/(1 − γ) − 1 et 1 − 1/(1 + γ)   3 u
  marge                                                   2 u   ✓
  ```

  Noter à quoi se compare le 5 u : à l'**exigence réelle de 3 u**, pas au 6 u appliqué. Le
  facteur 2 de `SLAB_WIDENING` était déjà un relâchement de confort (étape C.1), et c'est
  précisément lui qui paie ces arrondis non modélisés. La borne tient donc, avec 2 u de marge
  — moins de confort que le calcul brut ne le suggère, mais démontrablement assez.
- **Le cas d[i] == 0 sort du modèle** et n'en a pas besoin : il est traité en amont par un test
  de position de l'origine, sans division, donc sans arrondi. Voir la section « Rays parallel
  to a slab » du doc-comment de `hit`.

---

## 7. Pour aller plus loin

- Higham, *Accuracy and Stability of Numerical Algorithms*, 2ᵉ éd. — chapitre 2 pour la
  représentation, §3.1 pour le modèle standard et γ(n). La source du lemme.
- Goldberg, *What Every Computer Scientist Should Know About Floating-Point Arithmetic* (ACM
  Computing Surveys, 1991) — l'introduction de référence à IEEE-754.
- [PBR Book, *Managing Rounding Error*](https://www.pbr-book.org/3ed-2018/Shapes/Managing_Rounding_Error)
  — la même analyse appliquée au ray tracing, et le type `EFloat` qui propage les bornes
  depuis les entrées.
- [PBR Book, *Ray–Bounds Intersections*](https://www.pbr-book.org/3ed-2018/Shapes/Basic_Shape_Interface#RayBoundsIntersections)
  — la variante de pbrt, qui élargit t₁ seul par un facteur (1 + 2γ(3)). Moins cher, mais un
  facteur multiplicatif n'élargit que si t₁ > 0 ; `hit` élargit les deux bornes en magnitude,
  ce qui est conservateur quels que soient les signes.
