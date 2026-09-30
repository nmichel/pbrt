# Les sources de lumière, et la mesure de leurs échantillons

Ce que chaque implémentation de [`Light`](../src/lights.rs) rend à un intégrateur, dans quelle
**mesure**, et d'où vient la formule. Décrit le code tel qu'il est ; ce qui reste à faire est dans
[IDEAS.md](../IDEAS.md) et [ideas/](../ideas/).

Sujet voisin et distinct : [eclairage_declare.md](eclairage_declare.md) dit comment une scène
*déclare* ses sources, et pourquoi la frontière entre les deux syntaxes est la géométrie. Le présent
document ne parle pas du langage `.stage` mais de la physique et de l'estimateur.

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
Retenir cela évite à peu près toutes les fautes du §7.

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

Toute source doit honorer ce contrat *dans ses propres termes*. Les trois familles ci-dessous
l'honorent de trois façons qui n'ont rien en commun, et c'est tout l'intérêt de les lire ensemble.

## 3. Les deux mesures, et le jacobien qui les relie

L'intégrateur intègre sur les **directions**. Une surface, elle, s'échantillonne naturellement sur
son **aire** : tirer un point d'un rectangle est immédiat, tirer une direction pointant vers ce
rectangle ne l'est pas. Le pont entre les deux est un jacobien, et c'est la pièce centrale de tout
ce document.

### 3.1 De l'aire vers l'angle solide

Référence : [PBR Book, 4ᵉ éd., §4.2.3 — *Integrals over
Area*](https://pbr-book.org/4ed/Radiometry,_Spectra,_and_Color/Working_with_Radiometric_Integrals#IntegralsoverArea),
équation (4.9).

```text
[3]  L'angle solide sous lequel une surface est vue depuis p est, par définition, l'aire de sa
     projection centrale sur la sphère unité centrée en p.

[4]  Un élément d'aire dA en pₗ, de normale nₗ, est vu de biais : sa projection sur le plan
     perpendiculaire à la ligne de visée vaut

         dA_⊥ = dA · cos θₗ                                          [aire projetée]

     où θₗ est l'angle entre nₗ et w = (p − pₗ)/d, la direction de pₗ vers p.

[5]  Projeter centralement dA_⊥, situé à distance d, sur la sphère unité divise chaque longueur
     par d, donc les aires par d² :

         dω = dA_⊥ / d²                                              [homothétie de rapport 1/d]

[6]  dω = dA · cos θₗ / d²                                           [4] dans [5]
```

Les deux facteurs disent deux choses différentes. `cos θₗ` est un effet de **pose** : un panneau vu
par la tranche n'occupe presque rien du champ de vision. `1/d²` est un effet de **distance** : le
même panneau deux fois plus loin occupe quatre fois moins. Aucun des deux ne concerne ce que la
source émet.

### 3.2 La densité suit l'inverse

Une densité n'est pas une probabilité : c'est une probabilité *par unité de mesure*. Changer de
mesure la change donc. La contrainte est que le même événement garde la même probabilité :

```text
[7]  p_A(pₗ) · dA = p_ω(ω) · dω                                      [conservation de la probabilité]

[8]  p_ω(ω) = p_A(pₗ) · dA/dω = p_A(pₗ) · d² / cos θₗ                [6] dans [7]
```

C'est le cas particulier, pour ce changement de variables, de la règle générale
`p_T(y) = p(x)/|J_T(x)|` du [PBR Book, 4ᵉ éd.,
§2.4](https://pbr-book.org/4ed/Monte_Carlo_Integration/Transforming_between_Distributions).

**La densité explose quand `cos θₗ → 0`, et c'est correct.** Une source vue par la tranche
sous-tend un angle solide minuscule ; un point tiré sur elle représente donc une fraction
évanouissante des directions quittant `p`, et sa contribution doit être divisée par un grand nombre.
La variance qui en résulte est réelle — c'est elle que l'échantillonnage par angle solide supprimera.

### 3.3 La vérification qui ferme la boucle

[8] n'est pas à croire sur parole : la même intégrale s'écrit directement sur l'aire de la source, et
les deux estimateurs doivent coïncider terme à terme.

```text
[9]   L_direct(p, ω_o) = ∫_A f · L_e(pₗ, w) · V(p, pₗ) · |cos θ| · cos θₗ / d² dA     [1] avec [6]

[10]  ⟨·⟩_aire  = f · L_e · V · |cos θ| · cos θₗ / (d² · p_A)                          estimateur de [9]

[11]  ⟨·⟩_angle = f · L_e · V · |cos θ| / p_ω
                = f · L_e · V · |cos θ| · cos θₗ / (d² · p_A)                          [2] avec [8]
```

[10] et [11] sont la même expression. Le groupement `|cos θ| · cos θₗ / d²` de [9] est ce que la
littérature appelle le **terme géométrique** `G(p ↔ pₗ)`. Sous sa forme classique les deux cosinus
portent une valeur absolue, et `G` est alors rigoureusement symétrique en ses deux points — c'est la
raison de fond pour laquelle la lumière peut être transportée dans les deux sens. Ici le second est
signé, parce que l'émission est unilatérale (§6.3) : la dissymétrie n'est pas dans la géométrie mais
dans la source.

Le code ne calcule ni `G` ni [9] : il tient la mesure d'angle solide de bout en bout, parce que c'est
la mesure de l'intégrateur et qu'une seule conversion, faite à un seul endroit, est ce qui se
vérifie.

## 4. Une source de Dirac : [`PointLight`](../src/lights/point_light.rs)

Référence : [PBR Book, 4ᵉ éd., §12.2 — *Point Lights*](https://pbr-book.org/4ed/Light_Sources/Point_Lights).

### 4.1 Pourquoi elle n'a pas de radiance

Une source ponctuelle n'a pas d'aire. La radiance étant un flux par unité d'aire projetée, la sienne
serait un flux divisé par zéro : la grandeur n'existe pas. Ce qu'une telle source possède est une
**intensité** `I`, en W·sr⁻¹.

### 4.2 D'où vient le 1/d²

```text
[12]  L'éclairement en p est E = dΦ/dA_p, où dA_p est un élément d'aire autour du point ombré.

[13]  Le flux qui tombe sur dA_p est celui que la source émet dans l'angle solide que dA_p
      sous-tend *depuis la source* :

          dω = dA_p · cos θ / d²                                     [6], les rôles échangés

[14]  dΦ = I · dω = I · dA_p · cos θ / d²                            définition de l'intensité

[15]  E(p) = I · cos θ / d²                                          [14] dans [12]
```

[15] est la loi en carré inverse. **Elle vient du même jacobien [6] qu'au §3**, mais appliqué dans
l'autre sens : c'est ici le *récepteur* qui sous-tend un angle solide depuis la source, là c'était
l'*émetteur* qui en sous-tendait un depuis le point ombré.

### 4.3 L'intégrale s'effondre

La radiance incidente est une distribution, pas une fonction : toute la lumière arrive d'une seule
direction `ω_q`, celle de la source.

```text
[16]  L_i(p, ω) = (I/d²) · δ(ω − ω_q)                                δ sur la sphère, d'intégrale 1

[17]  ∫_{S²} f · L_i · |cos θ| dω = f(p, ω_o, ω_q) · (I/d²) · |cos θ_q|     propriété de la δ
```

Il n'y a rien à estimer : [17] est exact, et aucun nombre aléatoire n'est consommé — `sample_li`
ignore son `sampler`.

### 4.4 Ce que le code écrit, et l'abus qu'il commet

```rust
let spectrum = &self.i / wi.squared_length();   // I/d², par [15]
let sample = LightLiSample { spectrum, wi: wi.normalized(), pdf: 1.0 };
```

Reporté dans [2], cela rend `f · (I/d²) · |cos θ|`, qui est exactement [17]. **`pdf = 1.0` n'est pas
une densité** : c'est l'élément neutre qui rend inoffensive la division de la formule générique. Le
type dit `f64` et la valeur n'a pas d'unité.

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

## 5. Une source à l'infini : [`UniformInfiniteLight`](../src/lights/uniform_infinite_light.rs), [`BackgroundInfiniteLight`](../src/lights/background_infinite_light.rs)

Référence : [PBR Book, 4ᵉ éd., §12.5 — *Infinite Area
Lights*](https://pbr-book.org/4ed/Light_Sources/Infinite_Area_Lights).

Une source à l'infini n'a **pas de position**, seulement une distribution directionnelle
`L_∞(ω)` : la radiance qu'elle envoie est la même en tout point de la scène.

```text
[18]  L_i(p, ω) = L_∞(ω)                                             indépendante de p
```

Il n'y a donc aucune distance, aucun jacobien, et **aucun 1/d²**. La densité est celle d'un tirage
de direction, et rien d'autre : [`SpherePdf`](../src/pdfs/sphere.rs) tire uniformément sur la sphère
entière, d'où

```text
[19]  p_ω(ω) = 1/(4π)                                                aire de la sphère unité = 4π
```

et l'estimateur [2] devient `4π · f · L_∞(ω) · |cos θ|`.

Les deux implémentations ne diffèrent que par `L_∞` : une constante pour `UniformInfiniteLight`, un
dégradé vertical interpolé entre deux couleurs pour `BackgroundInfiniteLight`. Toutes deux
implémentent aussi `le(ray)`, que l'intégrateur consulte quand un rayon **s'échappe** sans rien
toucher — c'est le même `L_∞`, interrogé par l'autre bout du chemin, et c'est pourquoi les deux
fonctions doivent rester cohérentes.

L'absence de position est ce que portent les deux constructeurs de `VisibilityTester` : `between`
pour un segment, `towards_infinity` pour un rayon que rien ne borne. Forcer la forme à deux points
donnerait un segment de longueur nulle, dont la direction normalisée est `NaN`.

**Un écart assumé, et il coûte du bruit.** Tirer sur la sphère entière alors que seul l'hémisphère
autour de `n` peut contribuer gâche la moitié des échantillons : pour l'autre moitié,
[`Lambertian::f`](../src/materials/lambertian.rs) rend noir puisque `wo` et `wi` ne sont pas du même
côté. L'estimateur reste **non biaisé** — la densité couvre bien tout le domaine — mais il est plus
bruyant, et d'un facteur qui se calcule : en notant `μ` et `V` la moyenne et la variance qu'un
tirage sur l'hémisphère donnerait, un tirage sur la sphère a la même moyenne et une variance de
`2V + μ²`. Ce n'est pas seulement « la moitié des rayons perdus » : les échantillons qui restent
portent un poids double, et c'est le carré de ce poids qui entre dans la variance.

De même, le dégradé de `BackgroundInfiniteLight` n'est pas échantillonné par importance : les
directions les plus lumineuses ne sont pas tirées plus souvent que les autres.

## 6. Une source avec une géométrie échantillonnable : [`AreaLight`](../src/lights/area_light.rs)

Référence : [PBR Book, 4ᵉ éd., §12.4 — *Area
Lights*](https://pbr-book.org/4ed/Light_Sources/Area_Lights) et [§6.1.7 —
*Sampling*](https://pbr-book.org/4ed/Shapes/Basic_Shape_Interface#Sampling).

C'est le cas où le §3 sert, et c'est le seul type de source **qu'un rayon peut toucher** : une source
ponctuelle a une probabilité nulle d'être atteinte, une source à l'infini n'est jamais rencontrée que
par un rayon qui s'échappe.

### 6.1 Les trois pièces, et qui en répond

| Question | Répondant | Où |
|---|---|---|
| Où sont les points de la source, et avec quelle densité ? | `AreaSampleable` | [shapes.rs](../src/shapes.rs) |
| Que ce point émet-il vers `w` ? | `Emitter` | [materials.rs](../src/materials.rs) |
| Quelle direction cela fait-il, et quelle densité en angle solide ? | `AreaLight` | [lights/area_light.rs](../src/lights/area_light.rs) |

Le découpage n'est pas celui de pbrt, qui loge l'émission dans la lumière et fait porter un
accesseur de lumière à la primitive. Ici l'émission reste une face du **matériau**, conformément au
tableau du §2 de [CLAUDE.md](../CLAUDE.md) ; la raison, et le prix que MIS en paiera, sont au §4 de
[ideas/area_light.md](../ideas/area_light.md).

La forme échantillonnée est **le même `Arc<dyn Shape>` que l'objet visible rend**, déjà placé en
espace monde par la matrice de transformation courante. Il n'y a donc pas deux géométries à tenir
d'accord : il n'y en a qu'une.

### 6.2 Le tirage, et la conversion

[`Rectangle::sample_area`](../src/shapes/rectangle.rs) tire uniformément sur la surface, donc
`p_A = 1/aire`, puis `AreaLight::sample_li` applique [8] :

```rust
let cos_theta_l = vector3::dot(&sample.sp.n, &w);
let pdf = sample.pdf * squared_distance / cos_theta_l;   // [8]
```

**La radiance rendue n'est pas divisée par `d²`.** Elle est celle que la source émet, telle quelle.
Le carré inverse est *déjà* dans la densité par [8], et l'y mettre une seconde fois assombrirait
l'image d'un facteur `d²` — une erreur invisible sur la forme de l'image, qui ne change que sa
luminosité. C'est le §7 tout entier.

### 6.3 L'émission est unilatérale

[`DiffuseLight`](../src/materials/diffuse_light.rs) n'éclaire que la face vers laquelle sa normale
pointe :

```text
[20]  L(pₗ, w) = Lₑ(pₗ)  si nₗ ⋅ w > 0,  0 sinon
```

Le cas rasant `nₗ ⋅ w = 0` rend zéro, ce qui est la limite continue des deux côtés.

Cette règle a **une seule implémentation**, `Emitter::l`, dont `Material::emit` est une délégation.
Deux implémentations auraient été deux endroits où la règle peut dériver, et une divergence entre
elles ferait échouer la comparaison `naive` / `path` pour une raison étrangère au changement de
mesure — on chercherait le défaut dans le jacobien.

C'est aussi pourquoi `cos θₗ` **n'est pas sous une valeur absolue** en [8], là où le jacobien général
en mettrait une : la moitié que l'absolu servirait à récupérer est déjà répondue par « pas
d'échantillon ». pbrt offre un drapeau `twoSided` ; ce projet n'en a pas, et l'assume — une scène qui
veut éclairer des deux côtés pose deux surfaces dos à dos.

### 6.4 Trois cas sans échantillon

`sample_li` rend `None` plutôt qu'un échantillon nul quand le point tiré est confondu avec le point
ombré, quand `cos θₗ ≤ 0`, ou quand la densité n'est pas un nombre utilisable. La raison est
économique : un échantillon qui ne vaut rien coûte quand même un rayon d'ombre.

### 6.5 Qui la construit

[`SceneBuilderVisitor::visit_object_simple`](../src/loader/visitors/scene_builder.rs), à l'endroit
où il tient la forme *et* le matériau. Il pose l'objet visible, puis interroge les deux :
`Material::emitter` et `Shape::area_sampler`. Si les deux répondent, la forme placée part dans une
`AreaLight` sans qu'aucune géométrie soit recopiée — c'est le même `Arc`.

La quatrième combinaison, celle où le matériau émet et où la forme ne sait pas s'échantillonner,
**arrête le chargement en nommant la forme**. Ce n'est pas une vérification ajoutée : c'est le bras
qu'aucun autre ne couvre, et rendre la scène quand même reconduirait en silence, pour cet objet, le
défaut que la lumière d'aire existe pour supprimer.

### 6.6 Le rayon d'ombre s'arrête avant la source

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

## 7. Les deux 1/d², qui n'ont pas la même cause

C'est le piège du domaine, et il mérite son propre tableau.

| Source | Où vit `1/d²` | Pourquoi |
|---|---|---|
| `PointLight` | dans la **radiance** rendue | [15] : la source a une intensité, pas une radiance, et l'éclairement d'un récepteur décroît en `1/d²` |
| `AreaLight` | dans la **densité** rendue | [8] : la radiance ne décroît pas, c'est l'angle solide occupé par la source qui décroît |
| Sources à l'infini | nulle part | il n'y a pas de distance |

Les deux premières lignes sortent du même jacobien [6], lu dans deux directions opposées. Les deux
fautes qu'elles rendent possibles sont symétriques :

- **le mettre deux fois** pour une `AreaLight` — dans la radiance *et* dans la densité — assombrit
  l'image d'un facteur `d²` ;
- **l'oublier** pour une `PointLight` rend l'éclairage indépendant de la distance, ce qui se voit
  immédiatement et est donc la moins dangereuse des deux.

Aucune de ces fautes ne change la *forme* de l'image, seulement sa luminosité. Un œil ne les attrape
pas ; seul un second estimateur le peut.

## 8. Ce que les tests démontrent

Les tests font partie de la documentation ([CLAUDE.md](../CLAUDE.md) §4), et ceux-ci démontrent des
énoncés de ce document plutôt que des comportements.

| Test | Ce qu'il démontre |
|---|---|
| `test_the_density_integrates_to_the_area` ([rectangle.rs](../src/shapes/rectangle.rs)) | `Σ 1/p_A / N → aire` : la densité d'aire est normalisée |
| `test_the_draw_is_uniform_over_the_surface` | premier et second moments sur les deux axes : le tirage couvre la surface sans biais de position |
| `test_a_drawn_point_is_the_point_a_ray_finds_there` | la forme tirée et la forme vue sont la même |
| `test_the_density_in_solid_angle_is_the_area_density_times_the_jacobian` ([area_light.rs](../src/lights/area_light.rs)) | [8] sur une configuration calculable à la main, face à face |
| `test_the_estimator_measures_the_solid_angle_the_source_subtends` | `Σ 1/p_ω / N → Ω`, avec la forme close `Ω = 4·atan(ab/(d·√(a²+b²+d²)))` |
| `test_the_face_the_normal_points_to_emits` et ses deux voisins ([diffuse_light.rs](../src/materials/diffuse_light.rs)) | [20], les trois cas : face éclairée, face éteinte, direction rasante |
| `test_lambertian_energy_conservation_*` ([pdfs/](../src/pdfs/)) | qu'un estimateur Monte-Carlo de [2] est non biaisé |

**Il faut les deux tests d'`AreaLight`, et la mutation le prouve** — c'est le seul argument qui vaille
pour justifier un test qui a l'air redondant :

| Mutation de [8] | Test face à face | Test d'angle solide |
|---|---|---|
| sans `d²` | échoue | échoue |
| jacobien inversé (`cos θₗ/d²`) | échoue | échoue |
| **sans `cos θₗ`** | **passe** | échoue |
| radiance lue vers `ω_i` au lieu de `w` | échoue | passe |

Le test calculé à la main ne peut pas voir un cosinus manquant, puisque le sien vaut 1 ; et le test
statistique ne peut pas voir la mauvaise face de l'émetteur, puisqu'il ne lit que la densité.

**La preuve d'ensemble est ailleurs** : `naive` n'atteint les sources que par échantillonnage de
BSDF, `path` les atteint par NEE, et les deux doivent converger vers la même image. Un `d²` en trop
ou un cosinus manquant les sépare. La mesure est en [eclairage_declare.md](eclairage_declare.md) §4,
et c'est elle qui a rendu ce chantier vérifiable.

## 9. Les témoins, et ce que chacun répond

Trois scènes de `test_files/` servent de banc de mesure à l'éclairage. Chacune répond à **une**
question, et aucune ne répond à celle d'une autre — c'est le point de la section. Chaque fichier
porte en en-tête sa ligne de commande et la raison de son cadrage ; les chiffres ci-dessous sont
ceux du code tel qu'il est, lus par [`image_stats`](../src/bin/image_stats.rs), canal rouge, en
256 × 192 et `--seed 0`.

Aucune image de référence n'est versionnée, et il n'y a pas à en versionner : un rendu se rejoue
depuis sa graine, bit pour bit, quel que soit le nombre de fils
([rendu_reproductible.md](rendu_reproductible.md)). **Une référence est une ligne de commande, pas
un fichier.**

### 9.1 `direct_lighting.stage` — le biais

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
`cornell_box_exact.stage` lui en demande 16384 (§8). C'est la différence entre une boucle de mesure
qu'on lance en travaillant et une qu'on lance en partant déjeuner. Aucun pixel ne sature, ce qui est
une condition et non un hasard : le panneau est derrière l'objectif.

### 9.2 `grazing_source.stage` — la variance

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
du bruit et non un défaut. **C'est la ligne de base d'un échantillonnage par angle solide**, dont
tout l'objet est de faire baisser cette colonne à nombre de chemins constant.

**Ce témoin ne dit rien du biais**, et il faut le savoir avant de s'en servir : une bande mince
sous-tend trop peu d'angle solide pour que `naive` la trouve, qui donne 73,24 à 4096 chemins et
81,58 à 16384, toujours en train de monter vers 86,08. Ce qui est un défaut ici est précisément la
qualité recherchée en 9.1.

### 9.3 `indirect_lighting.stage` — le transport

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

### 9.4 `sphere_source.stage` — une source qui n'est pas plate

Le témoin de 9.1 avec son panneau remplacé par une sphère et sa caméra laissée en place, pour que
les deux jeux de chiffres se lisent l'un contre l'autre.

| chemins/pixel | `naive` | `path` |
|---|---|---|
| 64 | 100,56 | 106,17 |
| 256 | 105,85 | 106,71 |
| 1024 | 106,69 | 106,85 |
| 4096 | **106,84** | **106,87** |

Trois centièmes d'écart : le tirage uniforme sur l'aire d'une sphère est non biaisé.

**Il est en revanche coûteux, et d'une quantité qui se calcule.** L'émission étant unilatérale, un
point tiré sur la face opposée part en « pas d'échantillon » (§6.4), et la part qui survit est la
calotte que le point ombré voit :

```text
[21]  part utile = (1 − r/d) / 2
```

Une moitié au mieux, quand `d → ∞`, et d'autant moins qu'on est près. Sur ce témoin — lampe de
rayon 1 à trois unités du sol — 32,7 % des tirages survivent juste dessous et 43,8 % à 8,5 d'écart,
contre 33,3 % et 44,1 % que donne [21]. **Deux tirages sur trois sont jetés là où la lumière compte
le plus.** Un rectangle n'a pas d'équivalent de ce gaspillage, tous ses points regardant du même
côté ; c'est ce qui fait de cette scène celle où l'échantillonnage du cône sous-tendu se lira.

## 10. Les écarts au modèle physique, rassemblés

1. **`pdf = 1` pour une source δ** (§4.4) — correct pour l'estimateur, incomparable pour MIS.
2. **Tirage sur la sphère entière** pour les sources à l'infini (§5) — non biaisé, variance portée de
   `V` à `2V + μ²`.
3. **Pas d'échantillonnage par importance du dégradé** de `BackgroundInfiniteLight` (§5).
4. **Tirage uniforme sur l'aire** d'une source étendue, et non sur l'angle solide qu'elle sous-tend
   (§3.2) — non biaisé, bruyant quand la source est grande ou vue de biais.
5. **Pas d'émission bilatérale** (§6.3) — un choix, pas un manque.
6. **Choix uniforme de la source** parmi `N` ([path.rs](../src/integrators/path.rs)) — une petite
   source très lumineuse est tirée aussi souvent qu'un grand panneau faible.
7. **Le rayon d'ombre ne teste pas le dernier dix-millième du segment** (§6.6) — un occulteur collé
   à la source passe inaperçu. C'est le prix d'un défaut bien pire, et le seul de cette liste dont
   la mesure soit dans ce document.

## 11. Références

- [PBR Book, 4ᵉ éd., §4.2 — *Working with Radiometric
  Integrals*](https://pbr-book.org/4ed/Radiometry,_Spectra,_and_Color/Working_with_Radiometric_Integrals)
  — les grandeurs du §0 et l'équation (4.9) qui est le [6] de ce document.
- [PBR Book, 4ᵉ éd., §2.4 — *Transforming between
  Distributions*](https://pbr-book.org/4ed/Monte_Carlo_Integration/Transforming_between_Distributions)
  — la règle générale dont [8] est un cas.
- [PBR Book, 4ᵉ éd., chapitre 12 — *Light Sources*](https://pbr-book.org/4ed/Light_Sources) — les
  trois familles, et le `LightLiSample` sur lequel celui de ce projet est calqué.
- [PBR Book, 4ᵉ éd., §6.1.7 — *Sampling*](https://pbr-book.org/4ed/Shapes/Basic_Shape_Interface#Sampling)
  — les deux variantes de `Shape::Sample`, en aire et en angle solide.
- [eclairage_declare.md](eclairage_declare.md) — comment une scène déclare ses sources, et la
  comparaison `naive` / `path` qui sert de preuve.
- [ideas/area_light.md](../ideas/area_light.md) — ce qui reste à faire.
