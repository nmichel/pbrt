# Les sources de lumière, et la mesure de leurs échantillons

Ce que chaque implémentation de [`Light`](../src/lights.rs) rend à un intégrateur, dans quelle
**mesure**, et d'où vient la formule — intégralement dérivée. Décrit le code tel qu'il est ; ce qui
reste à faire est dans [IDEAS.md](../IDEAS.md) et [ideas/](../ideas/).

**Comment lire ce document.** Le §3 pose la seule mathématique dont tout le reste a besoin : ce
qu'est une densité de probabilité et ce qu'elle devient quand on change de variable. Il ne suppose
rien au-delà d'une dérivée et d'un déterminant 2 × 2, et les deux sont redémontrés. Le §4 l'applique
une fois, au passage de l'aire à l'angle solide. Tout le reste — §5 à §10 — est ce même calcul,
refait source par source, avec sa matrice jacobienne écrite en entier. Un lecteur pressé peut lire
le §3, le §4, puis la seule source qui l'intéresse.

Sujet voisin et distinct : [eclairage_declare.md](eclairage_declare.md) dit comment une scène
*déclare* ses sources. Le présent document ne parle pas du langage `.stage` mais de la physique et
de l'estimateur.

Référence d'ensemble : [PBR Book, 4ᵉ éd., chapitre 12 — *Light
Sources*](https://pbr-book.org/4ed/Light_Sources) et [§4.2 — *Working with Radiometric
Integrals*](https://pbr-book.org/4ed/Radiometry,_Spectra,_and_Color/Working_with_Radiometric_Integrals).

## 0. Le cadre, à poser avant toute formule

La plupart des erreurs d'éclairage viennent d'une convention implicite. Celles de ce projet :

| Symbole | Sens | Dans le code |
|---|---|---|
| `p` | le point ombré, celui qu'un rayon de l'œil a trouvé | `intersection.p` |
| `pₗ` | un point *sur la source* | `sample.sp.p` |
| `n` | la normale en `p` | `intersection.n` |
| `nₗ` | la normale en `pₗ` | `sample.sp.n` |
| `ω_o` | direction d'observation, **partant** de `p` | `-ray.direction` |
| `ω_i` | direction d'incidence, **partant** de `p` vers la source | `LightLiSample::wi` |
| `w` | direction d'émission, **partant** de `pₗ` | l'argument de `Emitter::l` |
| `d` | `‖pₗ − p‖`, la distance entre les deux | `squared_distance` vaut `d²` |
| `θ` | angle entre `n` et `ω_i` | |
| `θₗ` | angle entre `nₗ` et `w` | |
| `u` | le couple de nombres tirés dans [0, 1)² | `Vector2f`, de `Sampler::get_2d` |

**Les deux directions d'un même segment sont opposées** : `w = −ω_i`. C'est la confusion la plus
coûteuse du domaine, parce qu'elle ne produit pas une erreur bruyante mais une face éteinte —
[`materials::Emitter`](../src/materials.rs) porte l'avertissement au point d'usage, et
[`AreaLight::sample_li`](../src/lights/area_light.rs) prend explicitement l'opposé de `wi` avant
d'interroger l'émetteur.

**Toutes ces directions sont unitaires et en espace monde.** Le repère local où `z` est la normale
n'existe qu'à l'intérieur des matériaux ; `Light` ne le connaît pas
([`ShadingFrame`](../src/geom/shading_frame.rs) porte la convention et le changement de base).

**Les quatre grandeurs radiométriques** dont ce document a besoin, et qu'il ne faut pas confondre :

| Grandeur | Notation | Unité | Définition |
|---|---|---|---|
| Flux | Φ | W | l'énergie par unité de temps |
| Intensité | I | W·sr⁻¹ | `dΦ/dω` — un flux par unité d'angle solide |
| Éclairement | E | W·m⁻² | `dΦ/dA` — un flux par unité d'aire reçue |
| Radiance | L | W·m⁻²·sr⁻¹ | `dΦ/(dω dA cos θ)` — par unité d'angle solide *et* d'aire projetée |

La radiance est la grandeur que transporte un rayon, et **elle ne décroît pas avec la distance**.
Retenir cela évite à peu près toutes les fautes du §11.

## 1. L'intégrale à estimer

L'équation de rendu, restreinte à ce qui quitte une surface :

```text
[1]  L_o(p, ω_o) = L_e(p, ω_o) + ∫_{S²} f(p, ω_o, ω_i) · L_i(p, ω_i) · |cos θ| dω_i
```

`f` est la BSDF, `L_e` l'émission propre de la surface, et `|cos θ|` le facteur qui convertit une
aire en aire *projetée* perpendiculairement à `ω_i` — c'est la définition de la radiance qui le met
là, pas une pondération choisie.

Un intégrateur sépare l'intégrale en deux moitiés qu'il estime différemment :

- **l'éclairage direct** — la part de `L_i` qui vient directement d'une source. C'est ce que
  `Light::sample_li` sert, et le sujet de ce document ;
- **l'éclairage indirect** — la part qui a rebondi. C'est ce que `Material::scatter` sert.

[`PathIntegrator`](../src/integrators/path.rs) fait les deux et doit donc se garder de compter
l'émission deux fois : c'est le rôle de `is_last_bounce_specular`.
[`NaiveIntegrator`](../src/integrators/naive.rs) ne fait que la seconde — il ne consulte jamais
`Scene::lights` — et c'est précisément ce qui fait de lui un témoin indépendant
([eclairage_declare.md](eclairage_declare.md) §4).

## 2. L'estimateur, et ce que `pdf` doit donc signifier

Pour une densité `p(ω)` sur les directions et un tirage `ω_i ∼ p` :

```text
[2]  ⟨L_direct⟩ = f(p, ω_o, ω_i) · L_i(p, ω_i) · |cos θ| / p(ω_i)
```

C'est exactement ce qu'écrit [path.rs:73-75](../src/integrators/path.rs#L73), à un facteur près : la
scène ayant `N` sources et une seule étant tirée, avec la probabilité discrète `P = 1/N`, le même
argument s'applique une fois de plus et l'estimateur divise aussi par `P`.

**La couture est là.** L'intégrateur ne sait rien des sources ; il divise par `LightLiSample::pdf`.
Le contrat qui rend [2] juste tient en une phrase :

> `pdf` est une densité **par unité d'angle solide**, au point ombré, pour la direction `wi`
> rendue — et `spectrum` est la radiance qui arrive de cette direction-là.

Toute source doit honorer ce contrat *dans ses propres termes*. Les trois familles qui suivent
l'honorent de trois façons qui n'ont rien en commun, et c'est tout l'intérêt de les lire ensemble.

## 3. Changer de variable : la seule mathématique de ce document

Tout ce qui suit est une instance de ce paragraphe. Il vaut donc d'être lu en entier une fois.

### 3.1 Une densité n'est pas une probabilité

Une densité `p` ne donne pas la probabilité d'un point — pour une variable continue, celle-ci est
nulle. Elle donne une probabilité **par unité de mesure**, et il faut l'intégrer pour obtenir un
nombre :

```text
[3]  P(X ∈ [a, b]) = ∫[a,b] p(x) dx        et        ∫ p(x) dx = 1 sur tout le domaine
```

La conséquence pratique est que **`p` a une unité**, l'inverse de celle de `x`. Un point tiré
uniformément sur un segment de longueur `L` a pour densité `1/L`, qui est en m⁻¹ ; un point tiré
uniformément sur une surface d'aire `A` a pour densité `1/A`, en m⁻². Une densité « par unité
d'angle solide » est en sr⁻¹. Deux densités exprimées dans deux mesures différentes ne sont pas
comparables, et c'est pourquoi le §2 insiste autant sur celle qu'il attend.

### 3.2 Une variable : la règle, et sa démonstration

Soit `X` de densité `p_X`, et `Y = T(X)` où `T` est dérivable et strictement monotone — donc
inversible. Quelle est la densité de `Y` ?

On passe par la fonction de répartition, qui est une probabilité et ne change donc pas de valeur :

```text
[4]  Si T croît,  P(Y ≤ y) = P(T(X) ≤ y) = P(X ≤ T⁻¹(y))

     soit         F_Y(y) = F_X(T⁻¹(y))

     et en dérivant les deux membres par y, par la règle de dérivation composée,

                  p_Y(y) = p_X(T⁻¹(y)) · (T⁻¹)'(y) = p_X(x) / T'(x)        avec x = T⁻¹(y)

     Si T décroît, le raisonnement donne le même résultat au signe près, d'où en général

                  p_Y(y) = p_X(x) / |T'(x)|
```

**L'intuition tient en une phrase** : `T` étire localement le domaine d'un facteur `|T'|`. La même
probabilité, étalée sur un intervalle `|T'|` fois plus long, donne une densité `|T'|` fois plus
faible. **On divise par l'étirement.**

*Exemple, et c'est celui du rectangle au §8.* `X` uniforme sur [0, 1), densité 1, et
`T(x) = 2x − 1`. Alors `T' = 2` partout, donc `p_Y = 1/2` sur [−1, 1). Vérification : l'intégrale de
`1/2` sur un intervalle de longueur 2 vaut bien 1.

### 3.3 Deux variables : la matrice jacobienne et son déterminant

Tous les tirages de ce projet partent de **deux** nombres, donc la version à deux variables est
celle qui sert. Soit `T : (u₁, u₂) ↦ (y₁, y₂)`. On appelle **matrice jacobienne** de `T` la matrice
de ses quatre dérivées partielles :

```text
[5]  J_T(u) =  ⎡ ∂y₁/∂u₁   ∂y₁/∂u₂ ⎤
               ⎣ ∂y₂/∂u₁   ∂y₂/∂u₂ ⎦
```

Elle remplace le `T'` du cas à une variable, et le rôle de l'étirement est tenu par la **valeur
absolue de son déterminant** :

```text
[6]  p_Y(y) = p_U(u) / |det J_T(u)|
```

**Pourquoi le déterminant**, et c'est le seul point à ne pas prendre pour acquis. Un petit rectangle
du domaine, de côtés `du₁` et `du₂`, a pour image un petit parallélogramme dont les deux côtés sont
les vecteurs `(∂y/∂u₁)·du₁` et `(∂y/∂u₂)·du₂` — ce sont précisément les deux **colonnes** de `J_T`,
mises à l'échelle. Or l'aire d'un parallélogramme construit sur deux vecteurs est la valeur absolue
du déterminant de la matrice qui les porte en colonnes. Donc

```text
     aire d'arrivée = |det J_T| · aire de départ
```

et `|det J_T|` est bien le facteur d'étirement des **aires**, exactement comme `|T'|` était celui
des longueurs. On divise par lui pour la même raison qu'en [4].

### 3.4 Le motif qui revient partout : tirer dans le carré unité

Chaque échantillonneur de ce projet reçoit un `u` **uniforme sur le carré [0, 1)²**, dont l'aire
vaut 1, donc de densité `p_U = 1` partout. [6] se simplifie alors radicalement :

```text
[7]  p_Y(y) = 1 / |det J_T(u)|
```

> **La densité d'un point tiré est l'inverse de l'étirement d'aire que le tirage a fait subir au
> carré unité.**

Deux conséquences qui se lisent directement, et qui servent telles quelles au §8 et au §9 :

- si `T` est **affine**, `J_T` est constante, donc la densité est constante et vaut
  `1 / (aire de l'image)` ;
- si `T` n'est pas affine, la densité varie d'un point à l'autre, et c'est exactement ce qu'on veut
  quand on cherche à tirer plus souvent là où la lumière compte.

### 3.5 Quand on veut une densité imposée : la méthode d'inversion

[7] donne la densité qu'un tirage produit. La question inverse — *quel tirage produit la densité que
je veux ?* — se résout en intégrant. Pour une densité `q` voulue sur [a, b], on forme sa fonction de
répartition `F(y) = ∫[a,y] q`, qui croît de 0 à 1, et l'on pose

```text
[8]  T = F⁻¹      c'est-à-dire : tirer u uniforme, puis résoudre F(y) = u
```

C'est cohérent avec [4] : `F' = q`, donc `(F⁻¹)' = 1/q`, donc la densité de `y = F⁻¹(u)` est
`1/|(F⁻¹)'| = q`. Le §6 et le §9 s'en servent chacun une fois, et dans les deux cas `F` s'inverse à
la main.

## 4. Les deux mesures de l'éclairage : aire et angle solide

L'intégrateur intègre sur les **directions** ; une surface s'échantillonne naturellement sur son
**aire**. Le pont entre les deux est le changement de variable du §3, appliqué une fois pour toutes.

### 4.1 Ce qu'est un angle solide

L'angle solide sous lequel un objet est vu depuis un point `p` est, **par définition**, l'aire de sa
projection centrale sur la sphère de rayon 1 centrée en `p`. Son unité, le stéradian, est donc une
aire sans dimension. La sphère entière vaut `4π` sr, un hémisphère `2π`.

C'est la mesure naturelle de « combien de champ de vision cet objet occupe », et c'est pour cela
qu'un intégrateur travaille dedans : ce qui arrive en `p` ne dépend pas de la taille ni de la
distance d'une source prises séparément, mais de l'angle solide qu'elle occupe.

### 4.2 De l'aire vers l'angle solide

Référence : [PBR Book, 4ᵉ éd., §4.2.3 — *Integrals over
Area*](https://pbr-book.org/4ed/Radiometry,_Spectra,_and_Color/Working_with_Radiometric_Integrals#IntegralsoverArea),
équation (4.9).

```text
[9]   Un élément d'aire dA en pₗ, de normale nₗ, est vu de biais depuis p : sa projection sur le
      plan perpendiculaire à la ligne de visée vaut

          dA_⊥ = dA · |cos θₗ|

      où θₗ est l'angle entre nₗ et w = (p − pₗ)/d, la direction de pₗ vers p. C'est la définition
      même d'une projection orthogonale : une surface inclinée de θₗ par rapport au plan de visée
      n'en occupe que le cosinus.

[10]  Projeter dA_⊥, situé à distance d, sur la sphère **unité** est une homothétie de rapport 1/d.
      Une homothétie de rapport k multiplie les longueurs par k et donc les aires par k² :

          dω = dA_⊥ / d²

[11]  dω = dA · |cos θₗ| / d²                                        [9] dans [10]
```

Les deux facteurs disent deux choses différentes. `cos θₗ` est un effet de **pose** : un panneau vu
par la tranche n'occupe presque rien du champ de vision. `1/d²` est un effet de **distance** : le
même panneau deux fois plus loin occupe quatre fois moins. Aucun des deux ne concerne ce que la
source émet.

### 4.3 La densité suit l'inverse

[11] est un étirement d'aire, donc le §3.3 s'applique mot pour mot, `dA/dω` jouant le rôle de
`|det J|`. La contrainte est que le même événement garde la même probabilité :

```text
[12]  p_A(pₗ) · dA = p_ω(ω) · dω                                     conservation de la probabilité

[13]  p_ω(ω) = p_A(pₗ) · dA/dω = p_A(pₗ) · d² / |cos θₗ|             [11] dans [12]
```

C'est le cas particulier, pour ce changement de variable, de la règle générale [6] — et du [PBR
Book, 4ᵉ éd., §2.4](https://pbr-book.org/4ed/Monte_Carlo_Integration/Transforming_between_Distributions).

Le code l'écrit une seule fois, dans
[`shapes::solid_angle_from_area`](../src/shapes.rs), et c'est le corps par défaut dont le §7.3
explique le rôle.

**La densité explose quand `|cos θₗ| → 0`, et c'est correct.** Une source vue par la tranche
sous-tend un angle solide minuscule ; un point tiré sur elle représente donc une fraction
évanouissante des directions quittant `p`, et sa contribution doit être divisée par un grand nombre.
La variance qui en résulte est réelle — c'est elle que l'échantillonnage par angle solide supprime.

### 4.4 La vérification qui ferme la boucle

[13] n'est pas à croire sur parole : la même intégrale s'écrit directement sur l'aire de la source,
et les deux estimateurs doivent coïncider terme à terme.

```text
[14]  L_direct(p, ω_o) = ∫_A f · L_e(pₗ, w) · V(p, pₗ) · |cos θ| · |cos θₗ| / d² dA    [1] avec [11]

[15]  ⟨·⟩_aire  = f · L_e · V · |cos θ| · |cos θₗ| / (d² · p_A)                        estimateur de [14]

[16]  ⟨·⟩_angle = f · L_e · V · |cos θ| / p_ω
                = f · L_e · V · |cos θ| · |cos θₗ| / (d² · p_A)                        [2] avec [13]
```

[15] et [16] sont la même expression. Le groupement `|cos θ| · |cos θₗ| / d²` de [14] est ce que la
littérature appelle le **terme géométrique** `G(p ↔ pₗ)`, et il est rigoureusement symétrique en ses
deux points — c'est la raison de fond pour laquelle la lumière peut être transportée dans les deux
sens.

Le code ne calcule ni `G` ni [14] : il tient la mesure d'angle solide de bout en bout, parce que
c'est la mesure de l'intégrateur et qu'une seule conversion, faite à un seul endroit, est ce qui se
vérifie.

## 5. Une source de Dirac : [`PointLight`](../src/lights/point_light.rs)

Référence : [PBR Book, 4ᵉ éd., §12.2 — *Point Lights*](https://pbr-book.org/4ed/Light_Sources/Point_Lights).

### 5.1 Pourquoi elle n'a pas de radiance

Une source ponctuelle n'a pas d'aire. La radiance étant un flux par unité d'aire projetée, la sienne
serait un flux divisé par zéro : la grandeur n'existe pas. Ce qu'une telle source possède est une
**intensité** `I`, en W·sr⁻¹.

### 5.2 D'où vient le 1/d²

```text
[17]  L'éclairement en p est E = dΦ/dA_p, où dA_p est un élément d'aire autour du point ombré.

[18]  Le flux qui tombe sur dA_p est celui que la source émet dans l'angle solide que dA_p
      sous-tend *depuis la source* :

          dω = dA_p · cos θ / d²                                     [11], les rôles échangés

[19]  dΦ = I · dω = I · dA_p · cos θ / d²                            définition de l'intensité

[20]  E(p) = I · cos θ / d²                                          [19] dans [17]
```

[20] est la loi en carré inverse. **Elle vient du même jacobien [11] qu'au §4**, mais appliqué dans
l'autre sens : c'est ici le *récepteur* qui sous-tend un angle solide depuis la source, là c'était
l'*émetteur* qui en sous-tendait un depuis le point ombré.

### 5.3 L'intégrale s'effondre

La radiance incidente est une distribution, pas une fonction : toute la lumière arrive d'une seule
direction `ω_q`, celle de la source.

```text
[21]  L_i(p, ω) = (I/d²) · δ(ω − ω_q)                                δ sur la sphère, d'intégrale 1

[22]  ∫_{S²} f · L_i · |cos θ| dω = f(p, ω_o, ω_q) · (I/d²) · |cos θ_q|     propriété de la δ
```

Il n'y a rien à estimer : [22] est exact, et aucun nombre aléatoire n'est consommé — `sample_li`
ignore son `sampler`. **Aucun changement de variable n'intervient ici**, et c'est ce qui distingue
cette source de toutes les autres : il n'y a pas de tirage.

### 5.4 Ce que le code écrit, et l'abus qu'il commet

```rust
let spectrum = &self.i / wi.squared_length();   // I/d², par [20]
let sample = LightLiSample { spectrum, wi: wi.normalized(), pdf: 1.0 };
```

Reporté dans [2], cela rend `f · (I/d²) · |cos θ|`, qui est exactement [22]. **`pdf = 1.0` n'est pas
une densité** : c'est l'élément neutre qui rend inoffensive la division de la formule générique. Le
type dit `f64` et la valeur n'a pas d'unité — là où toute autre densité de ce document est en sr⁻¹.

C'est un abus assumé de l'interface, avec deux conséquences qu'il faut connaître :

- **Aucune variance** n'est introduite par ce terme, quelle que soit la graine.
- **MIS ne peut pas le pondérer.** Une heuristique de puissance compare deux densités dans une même
  mesure ; un `1` sans unité n'est comparable à rien. pbrt-v4 traite le cas en faisant rendre `0` à
  `PDF_Li` pour les sources δ, ce qui donne à la pondération un cas particulier explicite. Ce projet
  n'a pas encore de `Light::pdf_li` ; l'entrée est dans [IDEAS.md](../IDEAS.md), et elle est un
  prérequis de MIS.

À noter : [`LightType`](../src/lights.rs) existe et distingue `Point`, `Infinite` et `Area`, mais
**personne ne l'interroge pour cette question**. Son seul usage est
[`Integrator::background_radiance`](../src/integrators.rs), qui somme les `le` des sources à
l'infini. Le caractère singulier d'une source n'est aujourd'hui lisible nulle part dans l'interface.

## 6. Les sources à l'infini : [`UniformInfiniteLight`](../src/lights/uniform_infinite_light.rs), [`BackgroundInfiniteLight`](../src/lights/background_infinite_light.rs)

Référence : [PBR Book, 4ᵉ éd., §12.5 — *Infinite Area
Lights*](https://pbr-book.org/4ed/Light_Sources/Infinite_Area_Lights).

Une source à l'infini n'a **pas de position**, seulement une distribution directionnelle `L_∞(ω)` :

```text
[23]  L_i(p, ω) = L_∞(ω)                                             indépendante de p
```

Il n'y a donc aucune distance, aucun passage de l'aire à l'angle solide, et **aucun 1/d²**. Le seul
changement de variable est celui du tirage de direction lui-même, et il vaut d'être écrit en entier
parce que c'est le plus simple des quatre du document.

### 6.1 L'élément d'angle solide en coordonnées sphériques

Paramétrons une direction par sa colatitude `θ ∈ [0, π]` et son azimut `φ ∈ [0, 2π)` :

```text
[24]  ω(θ, φ) = (sin θ cos φ,  sin θ sin φ,  cos θ)
```

Sur la sphère unité, un petit rectangle curviligne délimité par `dθ` et `dφ` a pour côtés un arc de
méridien de longueur `dθ`, et un arc de parallèle de longueur `sin θ · dφ` — le facteur `sin θ`
étant le rayon de ce parallèle, qui s'annule aux pôles. D'où

```text
[25]  dω = sin θ · dθ · dφ
```

et la vérification, qui est aussi la valeur annoncée au §4.1 :

```text
[26]  ∫₀^{2π} ∫₀^{π} sin θ dθ dφ = 2π · [−cos θ]₀^π = 2π · 2 = 4π
```

### 6.2 Tirer uniformément, et la matrice jacobienne du tirage

« Uniformément sur la sphère » veut dire de densité constante **en angle solide**, donc `p_ω = 1/4π`.
[25] dit que ce n'est *pas* uniforme en `(θ, φ)` : à `dθ` égal, un pas près d'un pôle balaie moins de
sphère qu'à l'équateur. Le substitut qui redresse cela est le cosinus. En posant `c = cos θ`, dont
la différentielle est `dc = −sin θ dθ`, le facteur gênant disparaît :

```text
[27]  dω = dφ · |dc|          autrement dit : l'angle solide est uniforme en (φ, cos θ)
```

Le tirage est alors affine, et c'est ce que fait [`SpherePdf`](../src/pdfs/sphere.rs) :

```text
[28]  T : (u₁, u₂) ↦ (φ, c)  =  (2π·u₁,  1 − 2·u₂)
```

Sa matrice jacobienne [5] et son déterminant :

```text
[29]  J_T =  ⎡ ∂φ/∂u₁  ∂φ/∂u₂ ⎤  =  ⎡ 2π   0 ⎤        det J_T = −4π,   |det J_T| = 4π
             ⎣ ∂c/∂u₁  ∂c/∂u₂ ⎦     ⎣  0  −2 ⎦
```

Par [7], la densité du couple `(φ, c)` vaut `1/4π` ; et comme [27] dit que `dω` *est* `dφ dc`, cette
densité est déjà celle qu'on cherche :

```text
[30]  p_ω(ω) = 1/(4π)
```

L'estimateur [2] devient donc `4π · f · L_∞(ω) · |cos θ|`.

### 6.3 Les deux implémentations, et l'écart qu'elles partagent

Elles ne diffèrent que par `L_∞` : une constante pour `UniformInfiniteLight`, un dégradé vertical
interpolé entre deux couleurs pour `BackgroundInfiniteLight`. Toutes deux implémentent aussi
`le(ray)`, que l'intégrateur consulte quand un rayon **s'échappe** sans rien toucher — c'est le même
`L_∞`, interrogé par l'autre bout du chemin, et c'est pourquoi les deux fonctions doivent rester
cohérentes.

L'absence de position est ce que portent les deux constructeurs de `VisibilityTester` : `between`
pour un segment, `towards_infinity` pour un rayon que rien ne borne. Forcer la forme à deux points
donnerait un segment de longueur nulle, dont la direction normalisée est `NaN`.

**Un écart assumé, et il coûte du bruit.** Tirer sur la sphère entière alors que seul l'hémisphère
autour de `n` peut contribuer gâche la moitié des échantillons : pour l'autre moitié,
[`Lambertian::f`](../src/materials/lambertian.rs) rend noir puisque `wo` et `wi` ne sont pas du même
côté. L'estimateur reste **non biaisé** — la densité couvre bien tout le domaine — mais il est plus
bruyant, et d'un facteur qui se calcule. En notant `μ` et `V` la moyenne et la variance qu'un tirage
sur l'hémisphère donnerait, un tirage sur la sphère vaut `2·(contribution)` une fois sur deux et
zéro l'autre fois, donc

```text
      Soit Y la contribution qu'un tirage sur l'hémisphère donnerait, de moyenne μ et de variance V.
      Le tirage sur la sphère rend X = 2Y une fois sur deux, et 0 l'autre fois : 2Y parce que sa
      densité est deux fois plus faible, donc l'estimateur [2] divise par deux fois moins.

[31]  E[X]  = ½·E[2Y] = μ                       même moyenne : l'estimateur reste juste
[32]  E[X²] = ½·E[4Y²] = 2·E[Y²] = 2·(V + μ²)   car E[Y²] = Var(Y) + E[Y]²
[33]  Var(X) = E[X²] − E[X]² = 2V + μ²
```

Ce n'est donc pas seulement « la moitié des rayons perdus » : les échantillons qui restent portent
un poids double, et c'est le **carré** de ce poids qui entre dans la variance.

De même, le dégradé de `BackgroundInfiniteLight` n'est pas échantillonné par importance : les
directions les plus lumineuses ne sont pas tirées plus souvent que les autres.

## 7. Une source avec une géométrie : [`AreaLight`](../src/lights/area_light.rs)

Référence : [PBR Book, 4ᵉ éd., §12.4 — *Area
Lights*](https://pbr-book.org/4ed/Light_Sources/Area_Lights) et [§6.1.7 —
*Sampling*](https://pbr-book.org/4ed/Shapes/Basic_Shape_Interface#Sampling).

C'est le cas où le §4 sert, et c'est le seul type de source **qu'un rayon peut toucher** : une source
ponctuelle a une probabilité nulle d'être atteinte, une source à l'infini n'est jamais rencontrée que
par un rayon qui s'échappe. Cette section décrit la machinerie ; les deux suivantes font le calcul,
forme par forme.

### 7.1 Les trois pièces, et qui en répond

| Question | Répondant | Où |
|---|---|---|
| Où sont les points de la source, et avec quelle densité ? | `AreaSampleable` | [shapes.rs](../src/shapes.rs) |
| Que ce point émet-il vers `w` ? | `Emitter` | [materials.rs](../src/materials.rs) |
| Quelle direction cela fait-il, et que vaut-elle pour l'intégrateur ? | `AreaLight` | [lights/area_light.rs](../src/lights/area_light.rs) |

Le découpage n'est pas celui de pbrt, qui loge l'émission dans la lumière et fait porter un
accesseur de lumière à la primitive. Ici l'émission reste une face du **matériau**, conformément au
tableau du §2 de [CLAUDE.md](../CLAUDE.md).

La forme échantillonnée est **le même `Arc<dyn Shape>` que l'objet visible rend**, déjà placé en
espace monde par la matrice de transformation courante. Il n'y a donc pas deux géométries à tenir
d'accord : il n'y en a qu'une.

### 7.2 L'émission est unilatérale

[`DiffuseLight`](../src/materials/diffuse_light.rs) n'éclaire que la face vers laquelle sa normale
pointe :

```text
[34]  L(pₗ, w) = Lₑ(pₗ)  si nₗ ⋅ w > 0,  0 sinon
```

Le cas rasant `nₗ ⋅ w = 0` rend zéro, ce qui est la limite continue des deux côtés.

Cette règle a **une seule implémentation**, `Emitter::l`, dont `Material::emit` est une délégation.
Deux implémentations auraient été deux endroits où la règle peut dériver, et une divergence entre
elles ferait échouer la comparaison `naive` / `path` pour une raison étrangère au changement de
mesure — on chercherait le défaut dans le jacobien.

**C'est aussi ce qui explique les valeurs absolues du §4.** [11] et [13] en portent, parce qu'elles
décrivent une géométrie, vraie des deux côtés d'une surface. Le choix de ne retenir qu'une face est
une propriété de l'*émetteur*, pas de la forme, et il est appliqué par `AreaLight` après coup. pbrt
offre un drapeau `twoSided` ; ce projet n'en a pas, et l'assume — une scène qui veut éclairer des
deux côtés pose deux surfaces dos à dos.

### 7.3 Deux façons de traverser le pont, et qui choisit

Le §4 convertit une densité d'aire en densité d'angle solide *après* le tirage. Rien n'oblige à
passer par là : une forme qui sait quel angle solide elle sous-tend peut tirer **dedans**, et
dépenser chacun de ses points là où la lumière vient réellement.

[`AreaSampleable`](../src/shapes.rs) porte donc deux méthodes :

| Méthode | Mesure de sa densité | Qui la redéfinit |
|---|---|---|
| `sample_area(u)` | **aire**, m⁻² | `Rectangle` (§8), `Sphere` (§9.1) |
| `sample_solid_angle(reference, u)` | **angle solide**, sr⁻¹ | `Sphere` (§9.2) ; défaut = [13] pour les autres |

Le corps par défaut de la seconde est exactement la conversion [13]. Toute forme l'a donc
gratuitement, et celle qui sait mieux faire redéfinit. Aujourd'hui seule
[`Sphere`](../src/shapes/sphere.rs) redéfinit ; pour `Rectangle` la méthode existe mais n'est pas
écrite, et c'est le §10.1.

**Ce que ce partage déplace**, et c'est le point : `Light` ne change pas. `sample_li` recevait déjà
le point de référence et promettait déjà une densité en angle solide. Ce qui restait à séparer était
la géométrie de l'émission — une forme rend un point quelle que soit la face, avec une densité juste
des deux côtés, et c'est `AreaLight` qui sait que l'émetteur est unilatéral et jette ce qui regarde
ailleurs. Sans cette séparation, une forme ne pourrait pas tirer dans son propre angle solide sans
qu'on lui parle d'émission.

### 7.4 Deux cas sans échantillon

`sample_li` rend `None` plutôt qu'un échantillon nul quand la forme ne peut nommer aucune direction
— le point ombré est sur la surface, ou la densité a débordé — et quand le point tiré regarde du
mauvais côté. La raison est économique : un échantillon qui ne vaut rien coûte quand même un rayon
d'ombre.

### 7.5 Qui construit la lumière

[`SceneBuilderVisitor::visit_object_simple`](../src/loader/visitors/scene_builder.rs), à l'endroit
où il tient la forme *et* le matériau. Il pose l'objet visible, puis interroge les deux :
`Material::emitter` et `Shape::area_sampler`. Si les deux répondent, la forme placée part dans une
`AreaLight` sans qu'aucune géométrie soit recopiée — c'est le même `Arc`.

La quatrième combinaison, celle où le matériau émet et où la forme ne sait pas s'échantillonner,
**arrête le chargement en nommant la forme**. Ce n'est pas une vérification ajoutée : c'est le bras
qu'aucun autre ne couvre, et rendre la scène quand même reconduirait en silence, pour cet objet, le
défaut que la lumière d'aire existe pour supprimer.

### 7.6 Le rayon d'ombre s'arrête avant la source

Le segment testé s'arrête à `(1 − 10⁻⁴)` de la distance à la source, et cet écart-là n'est pas
cosmétique. Le point tiré sur une lumière d'aire **est un point d'une surface de la scène** : un
segment qui l'atteint exactement trouve cette surface en travers de son propre chemin, `intersect_p`
acceptant une touche à `t == far`. Mesuré sur le panneau de `cornell_box_exact.stage`, depuis un
point du sol qui le voit à la verticale : **1508 rayons d'ombre sur 2000 revenaient occultés par le
panneau lui-même**, 425 par le grand bloc — ceux-là légitimement — et 67 passaient.

C'est le pendant exact de `SHADOW_RAY_EPSILON`, qui écarte le départ de la surface ombrée ; aucun
des deux ne répare ce que l'autre traite. La différence est que celui-ci est **relatif** : ce qu'il
doit franchir est l'erreur d'arrondi sur la distance elle-même, laquelle croît avec cette distance.
Une source ponctuelle n'est jamais sur une surface, et c'est pourquoi le défaut n'apparaît qu'avec
les lumières d'aire.

### 7.7 Le double comptage, et le prix que MIS paiera

Dès que NEE peut échantillonner une source, un chemin qui l'atteint **par échantillonnage de BSDF**
et qui y ajouterait `material.emit` compterait la même contribution deux fois. Le garde
`is_last_bounce_specular` de [path.rs](../src/integrators/path.rs) fait exactement ce qu'il faut :
après un rebond diffus, NEE a servi et l'émission ne doit pas être ajoutée ; après un rebond
spéculaire, NEE n'a pas pu servir et elle doit l'être.

C'est un choix **entre** deux estimateurs, et c'est ce que MIS remplacera : prendre les deux et les
pondérer, au lieu d'en élire un. L'estimateur intermédiaire est correct, et bruyant sur les grandes
sources vues sous un angle rasant — précisément le cas que MIS répare.

**Le prix du découpage du §7.1 se paiera là.** Pour pondérer, l'intégrateur doit évaluer
`power_heuristic(pdf_bsdf, pdf_li)` au moment où un rayon de BSDF touche une surface émissive, donc
connaître **l'identité de la lumière touchée** — son aire, et la probabilité discrète `1/N` de
l'avoir tirée. Le `get_area_light` que pbrt fait porter à ses primitives n'est pas de la décoration :
c'est ce chaînon. Ici l'intégrateur tient le matériau, pas la lumière, et ne l'a donc pas. La sortie
n'est pas forcément celle de pbrt — la plus naturelle serait qu'`Interaction` porte un
`Option<&dyn Light>` à côté du matériau, sur le type dont le rôle *est* de marier géométrie et
matériau. Elle a un coût, [`Simple::intersect`](../src/objects/simple.rs) clonant déjà un `Arc` par
touche. **À trancher avec MIS**, et pas avant.

## 8. Le rectangle : tirage par aire

[`Rectangle`](../src/shapes/rectangle.rs) est défini dans le plan `y = 0`, centré sur l'origine, de
demi-côtés `a` et `b` — il s'étend donc sur `[−a, a] × [−b, b]` et son aire vaut `4ab`.

### 8.1 Le tirage, et sa matrice jacobienne

Le tirage est **affine et séparable**, une coordonnée par composante de `u` :

```text
[35]  T : (u₁, u₂) ↦ (x, z) = ((2u₁ − 1)·a,  (2u₂ − 1)·b)
```

Chaque ligne est le `2x − 1` de l'exemple du §3.2, mise à l'échelle. La matrice jacobienne [5] est
donc diagonale et constante :

```text
[36]  J_T =  ⎡ ∂x/∂u₁  ∂x/∂u₂ ⎤  =  ⎡ 2a   0 ⎤        det J_T = 4ab = aire
             ⎣ ∂z/∂u₁  ∂z/∂u₂ ⎦     ⎣  0  2b ⎦
```

Et [7] donne immédiatement la densité, constante parce que le déterminant l'est :

```text
[37]  p_A(x, z) = 1 / |det J_T| = 1 / aire
```

C'est le cas le plus simple du document, et il vaut d'être lu en premier : la densité **est**
l'inverse de l'aire parce que l'étirement du carré unité **est** l'aire. Les deux sections suivantes
font le même calcul avec des cartes qui ne sont plus affines.

### 8.2 Les coordonnées de texture ne sont pas normalisées

`sample_area` rend `u = x` et `v = z`, non ramenés à [0, 1), parce que **c'est ce que `intersect`
rapporte**. Un point tiré et un point touché doivent nommer le même endroit de la même surface,
faute de quoi une source texturée éclaire d'une couleur et se voit d'une autre. Normaliser ici
paraîtrait plus propre et casserait exactement cela.

### 8.3 Ce qui lui manque

Le rectangle ne redéfinit pas `sample_solid_angle` : il passe par [13]. C'est non biaisé et bruyant
quand la source est grande, proche, ou vue de biais, et c'est le §10.1.

## 9. La sphère : deux tirages pour la même surface

[`Sphere`](../src/shapes/sphere.rs) est centrée sur l'origine de son espace, de rayon `r`, et
paramétrée par les coordonnées sphériques [24] avec `z` pour axe polaire : `u = φ/2π` et `v = θ/π`.

### 9.1 Tirage par aire

#### L'élément d'aire

Sur une sphère de rayon `r`, le même raisonnement qu'au §6.1 donne des côtés `r·dθ` et
`r·sin θ·dφ`, donc

```text
[38]  dA = r² · sin θ · dθ · dφ            et        ∫ dA = 4πr²      par [26]
```

#### Le tirage, en deux étages

La carte se lit en deux temps, et c'est plus clair ainsi qu'en une seule matrice 2 × 2 : d'abord du
carré unité vers `(φ, c)` avec `c = cos θ`, puis de `(φ, c)` vers la surface.

**Premier étage**, identique à [28] et [29] — c'est le même tirage que pour une direction :

```text
[39]  T₁ : (u₁, u₂) ↦ (φ, c) = (2π·u₁,  1 − 2·u₂)        |det J_{T₁}| = 4π
```

**Second étage.** Par [38] et la substitution `dc = −sin θ dθ` du §6.2, `dA = r²·|dφ dc|` : le
passage de `(φ, c)` à la surface étire les aires d'un facteur **constant** `r²`. C'est ce facteur
constant qui fait tout l'intérêt de la variable `c` ; en `(φ, θ)` il aurait valu `r² sin θ` et aurait
varié d'un point à l'autre.

**Les deux ensemble.** Les étirements se composent en se multipliant, donc

```text
[40]  étirement total = 4π · r²        et par [7] :        p_A = 1 / (4πr²)
```

qui est bien l'inverse de l'aire [38]. Le point tiré est alors

```text
[41]  sin θ = √(1 − c²) ≥ 0            la racine positive, θ étant dans [0, π]
[42]  pₗ = r · (sin θ cos φ,  sin θ sin φ,  c)
```

#### La faute que cela évite

**Tirer `θ` uniformément au lieu de son cosinus** est l'erreur à ne pas commettre, et elle est
silencieuse : les points couvrent toujours toute la surface, donc rien ne manque à l'image ; ils
s'entassent simplement autour des pôles. Une lampe éclairerait alors comme si elle était plus
brillante en haut et en bas qu'à sa ceinture. Le test `test_the_draw_is_uniform_over_the_surface`
l'attrape en mesurant `∫z² dA`, qui vaut `r²·aire/3` pour le bon tirage et `r²·aire/2` pour l'autre.

#### Ce qu'il coûte

L'émission étant unilatérale [34], un point tiré sur la face opposée part en « pas d'échantillon ».
La part qui survit est la calotte que le point ombré voit, et elle se calcule : l'horizon vu de `p`
est le cercle de tangence, et la calotte visible a pour hauteur `r − r²/d`, d'où

```text
[43]  part utile = (r − r²/d) / (2r) = (1 − r/d) / 2
```

Une moitié au mieux, quand `d → ∞`, et d'autant moins qu'on est près. C'est ce que le §9.2 supprime.

### 9.2 Tirage dans le cône sous-tendu

Référence : [PBR Book, 4ᵉ éd., §6.2 — *Spheres*, « Sampling »](https://pbr-book.org/4ed/Shapes/Spheres#Sampling).

Au lieu de tirer un point et d'en déduire une direction, on tire une **direction** dans le cône que
la sphère occupe, puis on en déduit le point. Toutes les directions de ce cône rencontrent la
sphère : aucun tirage n'est perdu.

#### L'ouverture du cône

Depuis un point à distance `d > r` du centre, la direction extrême est celle qui rase la surface. Au
point de tangence, le rayon de la sphère est perpendiculaire à la ligne de visée, ce qui donne un
triangle rectangle d'hypoténuse `d` et de côté opposé `r` :

```text
[44]  sin θmax = r / d
```

#### Son angle solide

Par [25], en intégrant l'élément d'angle solide sur le cône :

```text
[45]  Ω = ∫₀^{2π} ∫₀^{θmax} sin θ dθ dφ = 2π · [−cos θ]₀^{θmax} = 2π · (1 − cos θmax)
```

#### Le tirage, et sa matrice jacobienne

Uniforme dans ce cône veut dire de densité constante `1/Ω`. Par [27], l'angle solide est uniforme en
`(φ, cos θ)` ; il suffit donc de tirer `cos θ` uniformément sur `[cos θmax, 1]` — un intervalle de
longueur `1 − cos θmax` — et `φ` sur `[0, 2π)` :

```text
[46]  T : (u₁, u₂) ↦ (c, φ) = (1 − u₁·(1 − cos θmax),  2π·u₂)

[47]  J_T =  ⎡ −(1 − cos θmax)   0  ⎤      |det J_T| = 2π·(1 − cos θmax) = Ω
             ⎣        0         2π  ⎦

[48]  p_ω = 1 / |det J_T| = 1/Ω                                      par [7] et [27]
```

C'est la méthode d'inversion [8] sous sa forme la plus simple : la densité voulue étant constante,
sa fonction de répartition est affine et s'inverse de tête.

#### De la direction au point

Une direction ne suffit pas : l'émetteur se lit **en un point**. Plaçons le point de référence à
l'origine d'un repère dont l'axe `z` pointe vers le centre, qui est alors en `C = (0, 0, d)`. La
direction tirée fait l'angle `θ` avec cet axe. Le rayon `x = t·ω` rencontre la sphère là où
`‖t·ω − C‖ = r` :

```text
[49]  t² − 2·t·d·cos θ + d² − r² = 0                                 en développant le carré

[50]  t = d·cos θ − √(r² − d²·sin²θ)                                 la racine la plus proche
```

Appelons `α` l'angle **au centre** entre la direction qui pointe vers le référent et celle qui
pointe vers le point tiré. En projetant sur l'axe — la composante selon `z` du vecteur `P − C` vaut
`t·cos θ − d` :

```text
[51]  cos α = (d − t·cos θ) / r

            = [d·sin²θ + cos θ·√(r² − d²·sin²θ)] / r                 [50] reporté et développé

            = sin²θ/sin θmax + cos θ·√(1 − sin²θ/sin²θmax)           avec d = r/sin θmax, par [44]
```

La normale au point tiré est cette direction-là, et le point s'en déduit :

```text
[52]  nₗ = (sin α cos φ,  sin α sin φ,  −cos α)                      dans le repère ci-dessus
[53]  pₗ = C + r·nₗ
```

**Seule la composante axiale est niée**, et le signe de la composante transverse n'est pas un détail
à prendre sur parole : `α` se mesure au centre depuis la direction qui pointe *vers* le référent,
c'est-à-dire `−z` ici, donc la normale penche du **même** côté que la direction tirée. Nier le
vecteur entier placerait le point à l'azimut `φ+π` quand la direction est à `φ` — les deux
nommeraient alors des côtés opposés de la sphère, et un rayon d'ombre viserait un endroit dont la
densité ne parle pas. En `α = 0` les deux formes coïncident, ce qui fait qu'un cas vu de face ne les
distingue pas.

#### La direction vient du tirage, jamais du point

```text
[54]  ω_i = (sin θ cos φ,  sin θ sin φ,  cos θ)                      dans le même repère
```

`θ` et `φ` *sont* ce qui a été tiré ; le point est ce qu'on en déduit. Reprendre la direction depuis
le point — normaliser `pₗ − p` — est un aller-retour par une soustraction, et cette soustraction
s'annule dans un cas qui n'a rien de rare : **un point ombré posé sur cette surface même**, ce qu'un
chemin produit chaque fois qu'il atterrit sur la lampe avant d'y faire du NEE. Porté dans l'espace
de la forme, un tel point tombe à `r` à quelques ulps près ; un cheveu *dehors*, et [44] donne
`sin θmax` un ulp sous 1, [51] donne `cos α = 1` pour tout tirage, le point s'effondre sur le
référent et la soustraction est un `0/0`. Mesuré avant que [54] ne remplace la soustraction : un
échantillon sur deux cents revenait `NaN`, et un pixel qui en croisait un restait noir — la moitié
de l'image à 512 chemins par pixel.

#### Deux annulations évitées

`1 − cos θmax` de [45] est une soustraction de nombres presque égaux dès que la sphère est petite ou
lointaine, et `sin²θ = 1 − cos²θ` l'est tout autant. pbrt protège les deux par un développement de
Taylor en deçà de `sin²θmax < sin²(1,5°)`. Une identité exacte le fait sans seuil :

```text
[55]  1 − cos θmax = sin²θmax / (1 + cos θmax)            car (1−c)(1+c) = 1 − c² = s²
[56]  sin²θ = (1 − cos θ)·(1 + cos θ)                     avec 1 − cos θ = u₁·(1 − cos θmax), exact
```

Les deux membres de droite ne sont bâtis que de sommes de quantités positives, donc rien ne s'y
annule pour aucune configuration — voir [arithmetique_flottante.md](arithmetique_flottante.md). Le
départ de la référence va donc vers plus de justesse, pas moins.

#### Le point de référence à l'intérieur

Il n'y a plus de cône : toute direction rencontre la surface, et [44] passerait 1. Le tirage retombe
alors sur celui par aire, suivi de la conversion [13]. Une sphère émissive n'éclaire de toute façon
rien en son intérieur — la face interne regarde ailleurs — mais la retombée garde une réponse
définie plutôt qu'un cas particulier à retenir.

### 9.3 Ce que le cône rapporte

Écart quadratique moyen contre une même référence convergée, la seule différence entre les deux
colonnes étant la méthode de tirage de la sphère, sur
[sphere_source.stage](../test_files/sphere_source.stage) :

| chemins/pixel | tirage par aire | tirage dans le cône |
|---|---|---|
| 16 | 26,75 | **1,41** |
| 64 | 12,80 | **0,79** |
| 256 | 6,40 | **0,51** |

Le bruit est divisé par **seize**, et la moyenne ne bouge pas — 106,89 contre 106,87 : le gain est
de la variance en moins, pas une image différente. Seize fois moins de bruit à nombre de chemins
constant vaut, à bruit constant, environ **250 fois moins de chemins**, l'écart d'un estimateur de
Monte-Carlo décroissant en `1/√N`.

## 10. Ce qui n'est pas fait, et la forme que cela prendrait

Les deux manques ont leur entrée dans [ideas/](../ideas/) ; ce qui suit est la mathématique qu'ils
demandent, posée ici parce qu'elle appartient au même sujet que tout ce qui précède.

### 10.1 Le rectangle sphérique

[`Rectangle`](../src/shapes/rectangle.rs) tire encore par aire et convertit par [13]. La méthode qui
lui correspond est celle de la **sphère** au §9.2, transposée à un quadrilatère : construire une
paramétrisation du carré unité vers le *rectangle sphérique* — la projection du rectangle sur la
sphère unité centrée au point ombré — qui préserve les aires, et dont la densité est donc l'inverse
constant de l'angle solide sous-tendu.

**L'angle solide d'un polygone sphérique** se calcule sans intégrale, par son **excès sphérique**.
Pour un triangle sphérique d'angles intérieurs `A`, `B`, `C`, le théorème de Girard donne une aire
de `A + B + C − π` ; pour un polygone à `n` sommets, la généralisation par découpage en triangles
donne

```text
[57]  Ω = (somme des n angles intérieurs) − (n − 2)·π

     soit, pour un quadrilatère :  Ω = α + β + γ + δ − 2π
```

Les quatre angles sont ceux entre les grands cercles portant les côtés, et se lisent sur des
produits vectoriels des quatre directions sommets.

Le tirage lui-même inverse une densité en deux temps : une première coordonnée partage `Ω`, une
seconde situe le point dans la tranche obtenue. C'est le §3.5 appliqué deux fois, mais aucune des
deux inversions ne se fait de tête comme en [46].

La carte qui préserve les aires est donnée sous forme analytique par C. Ureña, M. Fajardo et
A. King, *An Area-Preserving Parametrization for Spherical Rectangles*, Computer Graphics Forum
32(4), EGSR 2013, p. 59-66 ([DOI 10.1111/cgf.12151](https://doi.org/10.1111/cgf.12151), article sur
[la page de l'auteur](https://www.ugr.es/~curena/publ/2013-egsr/)). pbrt-v4 l'implémente sous le nom
`SampleSphericalRectangle` et l'appelle depuis `BilinearPatch::Sample` ([PBR Book, 4ᵉ éd.,
§6.6](https://pbr-book.org/4ed/Shapes/Bilinear_Patches)).

Le témoin et sa ligne de base existent : `grazing_source.stage` et l'échelle 7,42 / 3,72 / 1,79 du
§13.2.

### 10.2 Les maillages de triangles

Aucun maillage ne sait aujourd'hui s'échantillonner, et le diagnostic du §7.5 refuse donc une
lampe triangulaire. Il y a là **deux questions**, et les confondre est la façon de se tromper.

#### Tirer *dans* un triangle

Un point d'un triangle de sommets `P₀, P₁, P₂` s'écrit en coordonnées barycentriques
`P = b₀P₀ + b₁P₁ + (1 − b₀ − b₁)P₂`, le domaine admissible étant le triangle
`{b₀ ≥ 0, b₁ ≥ 0, b₀ + b₁ ≤ 1}`, d'aire `1/2` dans le plan `(b₀, b₁)`.

Tirer `(b₀, b₁)` uniformément dans ce domaine ne se fait pas en tirant deux nombres indépendants —
cela remplirait le carré, pas le triangle. La carte usuelle est

```text
[58]  T : (u₁, u₂) ↦ (b₀, b₁) = (1 − √u₁,  u₂·√u₁)
```

dont la matrice jacobienne a cette fois un terme croisé :

```text
[59]  J_T =  ⎡ ∂b₀/∂u₁  ∂b₀/∂u₂ ⎤  =  ⎡ −1/(2√u₁)    0   ⎤
             ⎣ ∂b₁/∂u₁  ∂b₁/∂u₂ ⎦     ⎣  u₂/(2√u₁)  √u₁  ⎦

[60]  det J_T = (−1/(2√u₁))·√u₁ − 0·(u₂/(2√u₁)) = −1/2        |det J_T| = 1/2
```

Le déterminant est **constant**, ce qui est tout l'intérêt de la racine carrée en [58] : la carte
étire uniformément, donc elle est uniforme sur son image. Par [7], `p_(b₀,b₁) = 2`, qui s'intègre
bien à 1 sur un domaine d'aire `1/2`.

Reste le passage des barycentriques à la surface, qui est linéaire :
`P − P₂ = b₀·(P₀ − P₂) + b₁·(P₁ − P₂)`. Son étirement d'aire est l'aire du parallélogramme bâti sur
les deux arêtes, c'est-à-dire `‖(P₀ − P₂) × (P₁ − P₂)‖ = 2·aire`. En composant :

```text
[61]  p_A = p_(b₀,b₁) / (2·aire) = 2 / (2·aire) = 1 / aire
```

#### Choisir **lequel**, proportionnellement à son aire

Un maillage est une somme de triangles, et tirer uniformément sur sa surface veut dire choisir le
triangle `k` avec la probabilité `A_k / A_total`, puis tirer dedans. C'est la méthode d'inversion
[8] dans le cas discret : on construit une fois la suite des sommes cumulées
`c_k = (A₁ + … + A_k)/A_total`, puis on cherche par dichotomie le premier `k` tel que `u ≤ c_k`.

La densité combinée se simplifie, et c'est ce qui rend le procédé correct :

```text
[62]  p_A = (A_k / A_total) · (1 / A_k) = 1 / A_total
```

Le facteur propre au triangle disparaît exactement. Le coût est une somme cumulée construite une
fois — donc un état à placer, et un coût de construction à mesurer sur un maillage réel.

Un détail qui évite de gâcher un nombre : une fois `k` trouvé, le reste
`(u − c_{k−1}) / (c_k − c_{k−1})` est de nouveau uniforme sur [0, 1) et peut servir de première
coordonnée au tirage [58].

**Le témoin manque pour la seconde question.**
[lamp_triangle.ply](../test_files/lamp_triangle.ply) n'a qu'un triangle et ne peut rien dire du
choix : il faudra un maillage à triangles d'aires nettement inégales, un maillage régulier ne
distinguant pas un choix correct d'un choix uniforme.

## 11. Les deux 1/d², qui n'ont pas la même cause

C'est le piège du domaine, et il mérite son propre tableau.

| Source | Où vit `1/d²` | Pourquoi |
|---|---|---|
| `PointLight` | dans la **radiance** rendue | [20] : la source a une intensité, pas une radiance, et l'éclairement d'un récepteur décroît en `1/d²` |
| `AreaLight` par aire | dans la **densité** rendue | [13] : la radiance ne décroît pas, c'est l'angle solide occupé par la source qui décroît |
| `AreaLight` par angle solide | **nulle part** | [48] : la densité est l'inverse d'un angle solide, et `d` n'intervient plus qu'à travers [44] |
| Sources à l'infini | nulle part | il n'y a pas de distance |

Les deux premières lignes sortent du même jacobien [11], lu dans deux directions opposées. Les deux
fautes qu'elles rendent possibles sont symétriques :

- **le mettre deux fois** pour une `AreaLight` — dans la radiance *et* dans la densité — assombrit
  l'image d'un facteur `d²` ;
- **l'oublier** pour une `PointLight` rend l'éclairage indépendant de la distance, ce qui se voit
  immédiatement et est donc la moins dangereuse des deux.

Aucune de ces fautes ne change la *forme* de l'image, seulement sa luminosité. Un œil ne les attrape
pas ; seul un second estimateur le peut.

## 12. Ce que les tests démontrent

Les tests font partie de la documentation ([CLAUDE.md](../CLAUDE.md) §4), et ceux-ci démontrent des
énoncés de ce document plutôt que des comportements.

| Test | Ce qu'il démontre |
|---|---|
| `test_the_density_integrates_to_the_area` ([rectangle.rs](../src/shapes/rectangle.rs), [sphere.rs](../src/shapes/sphere.rs)) | `Σ 1/p_A / N → aire` : [37] et [40] sont normalisées |
| `test_the_draw_is_uniform_over_the_surface` | premiers et seconds moments : le tirage couvre la surface sans biais, et `cos θ` est bien la variable uniforme [39] |
| `test_a_drawn_point_is_the_point_a_ray_finds_there` | la forme tirée et la forme vue sont la même, (u, v) comprises (§8.2) |
| `test_the_density_in_solid_angle_is_the_area_density_times_the_jacobian` ([area_light.rs](../src/lights/area_light.rs)) | [13] sur une configuration calculable à la main, face à face |
| `test_the_estimator_measures_the_solid_angle_the_source_subtends` | `Σ 1/p_ω / N → Ω`, avec la forme close `Ω = 4·atan(ab/(d·√(a²+b²+d²)))` |
| `test_the_density_integrates_to_the_solid_angle` ([sphere.rs](../src/shapes/sphere.rs)) | `Σ 1/p_ω / N → Ω` de [45] |
| `test_the_draw_is_uniform_over_the_cone` | `∫cos θ dω = π·sin²θmax` : le cône est tiré en `cos θ` et non en `θ` |
| `test_every_drawn_point_faces_the_reference` | aucun tirage perdu, ce que [43] quantifiait pour l'autre méthode |
| `test_the_two_draws_measure_the_same_solid_angle` | les deux tirages du §9 mesurent la même chose — le lien entre la méthode neuve et la conversion qui préexistait |
| `test_the_face_the_normal_points_to_emits` et ses deux voisins ([diffuse_light.rs](../src/materials/diffuse_light.rs)) | [34], les trois cas : face éclairée, face éteinte, direction rasante |
| `test_lambertian_energy_conservation_*` ([pdfs/](../src/pdfs/)) | qu'un estimateur Monte-Carlo de [2] est non biaisé |

**Il faut les deux tests d'`AreaLight`, et la mutation le prouve** — c'est le seul argument qui
vaille pour justifier un test qui a l'air redondant :

| Mutation de [13] | Test face à face | Test d'angle solide |
|---|---|---|
| sans `d²` | échoue | échoue |
| jacobien inversé (`cos θₗ/d²`) | échoue | échoue |
| **sans `cos θₗ`** | **passe** | échoue |
| radiance lue vers `ω_i` au lieu de `w` | échoue | passe |

Le test calculé à la main ne peut pas voir un cosinus manquant, puisque le sien vaut 1 ; et le test
statistique ne peut pas voir la mauvaise face de l'émetteur, puisqu'il ne lit que la densité.

Le même argument vaut pour la sphère, et une mutation le montre aussi : `θ` tiré au lieu de `cos θ`
n'est vu que par les moments, une aire amputée de son `4` que par la normalisation, et un `v`
renversé que par le test point-tiré-contre-point-touché.

**Un cas où aucune image n'aurait suffi.** La faute d'azimut décrite en [52] — le vecteur entier nié
au lieu de sa seule composante axiale — laissait l'image d'une sphère uniforme **rigoureusement
inchangée**, la scène étant symétrique. Seul le test qui tire un rayon vers le point tiré et compare
la touche l'a vue. Une lampe texturée, elle, aurait éclairé la scène depuis le mauvais côté.

**La preuve d'ensemble est ailleurs** : `naive` n'atteint les sources que par échantillonnage de
BSDF, `path` les atteint par NEE, et les deux doivent converger vers la même image. Un `d²` en trop
ou un cosinus manquant les sépare. La mesure est en [eclairage_declare.md](eclairage_declare.md) §4.

## 13. Les témoins, et ce que chacun répond

Quatre scènes de `test_files/` servent de banc de mesure à l'éclairage. Chacune répond à **une**
question, et aucune ne répond à celle d'une autre — c'est le point de la section. Chaque fichier
porte en en-tête sa ligne de commande et la raison de son cadrage ; les chiffres ci-dessous sont
ceux du code tel qu'il est, lus par [`image_stats`](../src/bin/image_stats.rs), canal rouge, en
256 × 192 et `--seed 0`.

Aucune image de référence n'est versionnée, et il n'y a pas à en versionner : un rendu se rejoue
depuis sa graine, bit pour bit, quel que soit le nombre de fils
([rendu_reproductible.md](rendu_reproductible.md)). **Une référence est une ligne de commande, pas
un fichier.**

### 13.1 `direct_lighting.stage` — le biais

Un sol et un panneau 4 × 4 à trois unités au-dessus, rien d'autre. Le panneau sous-tend beaucoup :
vu du centre du sol son facteur de forme vaut 0,36, donc mieux qu'un rayon cosinusoïdal sur trois
l'atteint. C'est ce qui rend `naive` — qui n'a aucun moyen de viser une source — utilisable comme
second estimateur pour un coût raisonnable.

| chemins/pixel | `naive` | `path` |
|---|---|---|
| 16 | 97,80 | 103,27 |
| 64 | 102,75 | 103,51 |
| 256 | 103,43 | 103,57 |
| 1024 | 103,57 | 103,59 |
| 4096 | **103,59** | — |

Les deux estimateurs se rejoignent au centième, et `naive` y arrive en **1024 chemins** là où
`cornell_box_exact.stage` lui en demande 16384. C'est la différence entre une boucle de mesure
qu'on lance en travaillant et une qu'on lance en partant déjeuner. Aucun pixel ne sature, ce qui est
une condition et non un hasard : le panneau est derrière l'objectif.

### 13.2 `grazing_source.stage` — la variance

Une bande de 20 sur 0,6 qui rase le sol à 0,35 de haut, la caméra devant elle. Le poids d'un point
tiré, `cos θ · cos θₗ / d²`, y varie autant qu'il est possible : `d²` d'un facteur deux cents entre
les deux bouts de la bande, et `cos θₗ` de 0,3 à 0,02. Le tirage uniforme par aire dépense donc
l'essentiel de ses points là où ils ne valent rien.

Ce qui se lit ici n'est pas la moyenne mais **l'écart quadratique moyen** contre un rendu convergé
de la même scène, `path` à 8192 chemins, de moyenne 86,08 :

| chemins/pixel | moyenne | écart quadratique |
|---|---|---|
| 64 | 85,84 | 7,42 |
| 256 | 86,02 | 3,72 |
| 1024 | 86,08 | 1,79 |

L'écart est divisé par deux quand les chemins sont multipliés par quatre : c'est le `1/√N` d'un
estimateur Monte-Carlo, et le fait qu'il tienne sur trois points dit que ces chiffres mesurent bien
du bruit et non un défaut. **C'est la ligne de base du rectangle sphérique** (§10.1), dont tout
l'objet est de faire baisser cette colonne à nombre de chemins constant.

**Ce témoin ne dit rien du biais**, et il faut le savoir avant de s'en servir : une bande mince
sous-tend trop peu d'angle solide pour que `naive` la trouve, qui donne 73,24 à 4096 chemins et
81,58 à 16384, toujours en train de monter vers 86,08. Ce qui est un défaut ici est précisément la
qualité recherchée en 13.1.

### 13.3 `indirect_lighting.stage` — le transport

Une pièce approchant la Cornell box, avec une dalle de 300 × 10 × 300 entre le panneau et le sol :
presque rien n'atteint le sol directement, donc ce qu'on y lit a rebondi. C'est la quantité qu'une
surface émissive non enregistrée comme lumière supprime entièrement.

| chemins/pixel | `naive` | `path` |
|---|---|---|
| 256 | 66,21 | 70,26 |
| 1024 | 69,70 | 70,41 |
| 4096 | **70,28** | — |

Treize centièmes d'écart, sur une image dont l'éclairage est presque tout indirect. La scène
déclarait naguère une `light point` à l'intérieur de la pièce, qui éclairait le sol directement et
noyait dans le terme direct la mesure du terme indirect.

### 13.4 `sphere_source.stage` — une source qui n'est pas plate

Le témoin de 13.1 avec son panneau remplacé par une sphère et sa caméra laissée en place, pour que
les deux jeux de chiffres se lisent l'un contre l'autre.

| chemins/pixel | `naive` | `path`, tirage par aire |
|---|---|---|
| 64 | 100,56 | 106,17 |
| 256 | 105,85 | 106,71 |
| 1024 | 106,69 | 106,85 |
| 4096 | **106,84** | **106,87** |

Trois centièmes d'écart : le tirage uniforme sur l'aire d'une sphère est non biaisé. C'est aussi sur
cette scène que [43] a été vérifié — 32,7 % des tirages survivaient juste sous la lampe et 43,8 % à
8,5 d'écart, contre 33,3 % et 44,1 % prédits — et que le gain du cône est chiffré au §9.3.

## 14. Les écarts au modèle physique, rassemblés

1. **`pdf = 1` pour une source δ** (§5.4) — correct pour l'estimateur, incomparable pour MIS.
2. **Tirage sur la sphère entière** pour les sources à l'infini (§6.3) — non biaisé, variance portée
   de `V` à `2V + μ²`.
3. **Pas d'échantillonnage par importance du dégradé** de `BackgroundInfiniteLight` (§6.3).
4. **Tirage uniforme sur l'aire** pour le `Rectangle`, et non sur l'angle solide qu'il sous-tend
   (§10.1) — non biaisé, bruyant quand la source est grande ou vue de biais. La `Sphere` ne l'a plus.
5. **Aucun maillage ne s'échantillonne** (§10.2) — une lampe triangulaire est refusée au chargement.
6. **Pas d'émission bilatérale** (§7.2) — un choix, pas un manque.
7. **Choix uniforme de la source** parmi `N` ([path.rs](../src/integrators/path.rs)) — une petite
   source très lumineuse est tirée aussi souvent qu'un grand panneau faible.
8. **Le rayon d'ombre ne teste pas le dernier dix-millième du segment** (§7.6) — un occulteur collé
   à la source passe inaperçu. C'est le prix d'un défaut bien pire, et le seul de cette liste dont
   la mesure soit dans ce document.
9. **Les échantillons de densité non finie sont jetés** ([13] déborde quand `cos θₗ → 0`), ce qui
   retire une portion du domaine et introduit donc un biais — infime, et que le tirage en angle
   solide supprime puisque sa densité est constante.

## 15. Références

- [PBR Book, 4ᵉ éd., §2.4 — *Transforming between
  Distributions*](https://pbr-book.org/4ed/Monte_Carlo_Integration/Transforming_between_Distributions)
  — la règle [6], dont tout le reste de ce document est une instance.
- [PBR Book, 4ᵉ éd., §4.2 — *Working with Radiometric
  Integrals*](https://pbr-book.org/4ed/Radiometry,_Spectra,_and_Color/Working_with_Radiometric_Integrals)
  — les grandeurs du §0 et l'équation (4.9), qui est le [11] de ce document.
- [PBR Book, 4ᵉ éd., chapitre 12 — *Light Sources*](https://pbr-book.org/4ed/Light_Sources) — les
  trois familles, et le `LightLiSample` sur lequel celui de ce projet est calqué.
- [PBR Book, 4ᵉ éd., §6.1.7 — *Sampling*](https://pbr-book.org/4ed/Shapes/Basic_Shape_Interface#Sampling)
  et [§6.2 — *Spheres*](https://pbr-book.org/4ed/Shapes/Spheres#Sampling) — les deux variantes de
  `Shape::Sample`, et le cône du §9.2.
- [PBR Book, 4ᵉ éd., §A.5 — *Sampling Multidimensional
  Functions*](https://pbr-book.org/4ed/Sampling_Algorithms/Sampling_Multidimensional_Functions) —
  les cartes du carré unité vers le disque, la sphère et l'hémisphère.
- C. Ureña, M. Fajardo, A. King, *An Area-Preserving Parametrization for Spherical Rectangles*,
  Computer Graphics Forum 32(4), EGSR 2013, p. 59-66 —
  [DOI 10.1111/cgf.12151](https://doi.org/10.1111/cgf.12151), article et transparents sur
  [www.ugr.es/~curena/publ/2013-egsr](https://www.ugr.es/~curena/publ/2013-egsr/). La méthode du
  §10.1, et [PBR Book, 4ᵉ éd., §6.6](https://pbr-book.org/4ed/Shapes/Bilinear_Patches) pour une
  implémentation à lire à côté.
- [eclairage_declare.md](eclairage_declare.md) — comment une scène déclare ses sources, et la
  comparaison `naive` / `path` qui sert de preuve.
- [arithmetique_flottante.md](arithmetique_flottante.md) — pourquoi [55] et [56] sont écrits ainsi.
