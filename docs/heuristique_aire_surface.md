# L'heuristique d'aire de surface : construire un BVH

Ce document explique le critère de découpe utilisé par
[`BVHTree::find_best_split_plane`](../src/shapes/triangle_mesh/bvh.rs) — le SAH, *surface area
heuristic* — et les décisions de conception de son implémentation binnée : `BIN_COUNT`,
`bin_index`, `calculate_node_cost`, et le test de rentabilité de `subdivide`.

Il se lit en deux moitiés. Les §0 à §2 posent le critère : ce que « bon arbre » peut vouloir
dire, pourquoi c'est l'*aire* qui gouverne, et la fonction de coût qui en découle. Les §3 à §5
en tirent l'implémentation : pourquoi les aires sont celles des **unions**, comment le binning
ramène le coût de construction à O(N), et ce que devient le cas dégénéré.

La §1 dérive « l'aire, et pas le volume » depuis zéro, sans autre prérequis que la notion
d'ombre et une moyenne de cosinus ; un lecteur qui tient déjà la formule de Cauchy pour acquise
peut n'en lire que la première et la dernière sous-section.

**C'est ici que vit la dérivation détaillée**, avec ses contre-exemples chiffrés ; le
doc-comment de `find_best_split_plane` en donne une version resserrée, suffisante pour lire le
code sans quitter le fichier. En cas de doute sur le choix d'un plan, c'est la §3 qu'il faut
ouvrir ; sur l'indexation des bins, la §4.

La §7 donne l'**algorithme complet en pseudo-code**, chaque ligne annotée de l'équation et de
la section qui la justifient. C'est la carte : elle se lit après le terrain, ou avant, selon le
sens qu'on préfère.

Références :

- D. J. MacDonald et K. S. Booth, *Heuristics for ray tracing using space subdivision*, The
  Visual Computer 6(3), 1990 — l'origine du critère.
- I. Wald, *On fast Construction of SAH-based Bounding Volume Hierarchies*, IEEE Symposium on
  Interactive Ray Tracing, 2007 — la construction binnée implémentée ici.
- V. Havran, *Heuristic Ray Shooting Algorithms*, thèse, Czech Technical University, 2000,
  ch. 4 — la probabilité géométrique, traitée en détail.
- [PBR Book, *Bounding Volume Hierarchies*](https://www.pbr-book.org/3ed-2018/Primitives_and_Intersection_Acceleration/Bounding_Volume_Hierarchies)
  — la présentation de référence, dont les conventions de coût sont comparées en §2.

---

## 0. Qu'est-ce qu'un « bon » arbre ?

La question n'est pas rhétorique, parce qu'elle n'a **pas de réponse observable dans l'image**.
Un BVH est une structure d'accélération : quelle que soit sa forme, la traversée rend la même
intersection la plus proche, donc le même pixel. Un arbre mal construit ne produit pas une
image fausse, il produit la même image plus lentement. Toute notion de qualité doit donc être
définie comme un **coût espéré**, et un coût espéré exige une distribution de rayons.

C'est aussi ce qui rend ce chantier mesurable aujourd'hui alors que les chantiers d'éclairage
ne le sont pas : une traversée ne tire aucun nombre aléatoire, donc pour un jeu de rayons fixé
les compteurs de `TraversalStats` sont reproductibles à l'unité. Voir
[`src/bin/bvh_stats.rs`](../src/bin/bvh_stats.rs).

### Pourquoi les critères naïfs échouent

Deux critères viennent naturellement à l'esprit, et tous deux sont aveugles.

**La médiane** — couper de sorte que les deux enfants aient le même nombre de primitives.
L'arbre est parfaitement équilibré, sa profondeur est minimale, et il peut être détestable :
équilibrer les *effectifs* ne dit rien des *volumes*. Un maillage fait d'un amas dense et de
quelques triangles très éloignés voit sa médiane tomber dans l'amas, et le nœud gauche hérite
d'une boîte qui couvre tout l'espace jusqu'aux triangles lointains. Chaque rayon traverse alors
les deux enfants. C'est encore ce que fait le BVH de scène, à un axe tiré au hasard près
([src/bvh.rs](../src/bvh.rs)).

**Le milieu spatial** — couper la boîte du nœud en deux moitiés égales. Symétrique du
précédent : les étendues spatiales sont équilibrées, les effectifs pas du tout, et un côté peut
être vide.

Ce qu'aucun des deux ne modélise, c'est que le coût d'un enfant est le produit de deux
quantités antagonistes : **la probabilité qu'un rayon l'atteigne**, qui croît avec la taille de
sa boîte, et **le nombre de primitives à tester quand il l'atteint**. Minimiser l'un des deux
facteurs seul revient à ignorer l'autre. Le SAH est exactement le produit des deux.

Reste à dire de quelle « taille » il s'agit. Ce n'est ni le volume ni la plus grande dimension,
c'est l'**aire de surface** — un résultat que la §1 dérive plutôt que de le poser.

---

## 1. La probabilité géométrique : pourquoi l'aire

Le résultat à établir tient en une ligne. Pour deux **convexes** emboîtés B ⊆ A :

```text
P(un rayon qui rencontre A rencontre aussi B)  =  SA(B) / SA(A)          [1]
```

où SA est l'aire de la surface. Ni le volume, ni la plus grande dimension, ni le nombre de
faces : l'**aire**. C'est le fait central du document, et il n'a rien d'évident.

Cette section le dérive de bout en bout. Aucune théorie de la mesure n'est nécessaire : il
suffit de savoir ce qu'est une ombre, et de calculer une moyenne de cosinus.

### Ce que « un rayon au hasard » veut dire

Parler de probabilité exige une loi de tirage, et il n'y en a pas d'évidente : « une droite au
hasard dans ℝ³ » n'a pas plus de sens en soi que « un entier au hasard ». Il faut donc en
choisir une, et la choisir explicitement. Celle qu'on retient se donne comme une **recette**,
que l'on pourrait coder telle quelle :

```text
1.  tirer une direction ω uniformément sur la sphère unité
2.  tirer un point d'entrée uniformément sur un très grand disque perpendiculaire à ω
3.  le rayon est la droite issue de ce point, dans la direction ω
```

Ce choix n'est pas arbitraire : c'est le seul qui n'exprime aucune préférence. L'étape 1 ne
privilégie aucune direction — la loi est invariante par rotation. L'étape 2 ne privilégie aucun
endroit — la loi est invariante par translation. C'est ce que la littérature appelle la
**mesure cinématique**, ou la distribution des *droites uniformément distribuées* ; Havran
ch. 4 en donne le traitement formel. Nous n'aurons besoin que de la recette.

Noter d'emblée ce que l'étape 3 abandonne : le rayon n'a pas d'origine propre, il vient de
l'infini. Un rayon d'ombre, qui est un *segment* entre deux points précis, n'est pas un tirage
de cette loi — on y revient plus bas.

### Une direction à la fois : l'ombre

Gelons l'étape 1 : la direction ω est fixée, seul le point d'entrée reste aléatoire. Le
problème devient bidimensionnel, et sa réponse est une image d'enfance — la **lampe torche**.

Les rayons de direction ω qui rencontrent un corps K sont exactement ceux dont le point
d'entrée tombe dans l'**ombre** de K, c'est-à-dire dans sa projection orthogonale sur un plan
perpendiculaire à ω :

```text
        ω                                                   plan ⊥ ω
   ─────────►        ╭─────────╮                               │
   ─────────►       ╱           ╲                              │▓▓▓▓
   ─────────►      │      K      │        ══════►              │▓▓▓▓   ombre(K, ω)
   ─────────►       ╲           ╱                              │▓▓▓▓
   ─────────►        ╰─────────╯                               │
```

Le point d'entrée étant tiré uniformément sur le grand disque, la probabilité de tomber dans
une région donnée est proportionnelle à son aire. Donc, **à direction fixée** :

```text
P(rencontrer B | rencontrer A, direction ω)  =  ombre(B, ω) / ombre(A, ω)
```

Et B ⊆ A entraîne ombre(B, ω) ⊆ ombre(A, ω), donc le rapport reste bien dans [0, 1] :

```text
        ╭───────────────╮                              │▒▒▒▒▒▒▒▒▒
       ╱   A      ╭──╮   ╲                             │▒▒▓▓▓▒▒▒▒     B ⊆ A
      │           │B │    │          ══════►           │▒▒▓▓▓▒▒▒▒     ⟹ ombre(B) ⊆ ombre(A)
       ╲          ╰──╯   ╱                             │▒▒▒▒▒▒▒▒▒
        ╰───────────────╯                              │
```

Tout le reste de la section consiste à dégeler l'étape 1, c'est-à-dire à moyenner sur ω.

### L'ombre d'une boîte

Le seul corps dont le code ait besoin est l'AABB, et son ombre se calcule à vue.

Commençons par **une face plane** d'aire S et de normale n. Vue depuis ω, elle est raccourcie :
en se plaçant dans le plan engendré par n et ω, la projection conserve les longueurs
perpendiculaires à ce plan et multiplie les autres par le cosinus de l'angle entre n et ω. Une
seule dimension sur deux est donc contractée :

```text
ombre(face) = S · |n ⋅ ω|
```

Une boîte a six faces, en trois paires opposées. De chaque paire, ω n'en éclaire qu'**une** —
l'autre est cachée derrière, et son ombre coïncide exactement avec la première. Une boîte de
dimensions (dx, dy, dz) montre donc trois faces, chacune raccourcie par la composante de ω le
long de sa normale :

```text
paire de faces     aire d'une face     raccourci     contribution à l'ombre
     ⊥ x                dy·dz            |ωx|            dy·dz·|ωx|
     ⊥ y                dz·dx            |ωy|            dz·dx·|ωy|
     ⊥ z                dx·dy            |ωz|            dx·dy·|ωz|
```

```text
ombre(ω)  =  dy·dz·|ωx|  +  dz·dx·|ωy|  +  dx·dy·|ωz|
```

Contrôle sur le cube unité, où la formule se réduit à |ωx| + |ωy| + |ωz| :

```text
ω = (1, 0, 0)        face à face           ombre = 1
ω = (1, 1, 0)/√2     arête en avant        ombre = 2/√2 = √2 ≈ 1,414
ω = (1, 1, 1)/√3     coin en avant         ombre = 3/√3 = √3 ≈ 1,732
```

La dernière ligne est la silhouette maximale du cube, l'hexagone régulier d'aire √3 — un
résultat qu'on peut vérifier à la main, et qui rassure sur la formule.

### La moyenne sur les directions

Il ne reste qu'une quantité à calculer : ⟨|ωx|⟩, la moyenne de la valeur absolue d'une
composante de ω, pour ω uniforme sur la sphère. Deux dérivations, l'une calculatoire, l'autre
mémorable.

**Par l'intégrale.** En coordonnées sphériques d'axe x, la composante ωx vaut cos θ et
dω = sin θ dθ dφ :

```text
⟨|cos θ|⟩ = (1/4π) ∫₀^{2π} ∫₀^π |cos θ| · sin θ · dθ · dφ
          = (1/2) ∫₀^π |cos θ| · sin θ · dθ
          = (1/2) · 2 ∫₀^{π/2} cos θ · sin θ · dθ            [symétrie des deux hémisphères]
          = (1/2) · 2 · [sin²θ / 2]₀^{π/2}
          = 1/2
```

**Par Archimède.** Le théorème dit que la sphère et le cylindre qui la circonscrit ont la même
aire entre deux plans perpendiculaires à l'axe du cylindre. Conséquence directe : la projection
sur un axe d'un point uniforme de la sphère est **uniforme sur [−1, 1]**. Donc ωx suit une loi
uniforme sur [−1, 1], et ⟨|ωx|⟩ = 1/2 sans écrire une intégrale.

Par symétrie, ⟨|ωy|⟩ et ⟨|ωz|⟩ valent aussi 1/2. En moyennant la formule de l'ombre terme à
terme :

```text
⟨ombre⟩  =  ½·(dy·dz + dz·dx + dx·dy)  =  ½ · half_area            ← le pivot
```

**C'est le pivot de tout le document.** `half_area` n'est pas « l'aire de surface amputée d'un
facteur 2 » : c'est, au facteur 2 près, l'**ombre moyenne de la boîte** — autrement dit la
mesure des rayons qui la rencontrent. La grandeur que le code manipule est la bonne quantité
physique, et c'est SA qui porte une convention en trop.

Contrôle : pour le cube unité, ⟨ombre⟩ = ½ · 3 = 1,5, bien comprise entre le minimum 1 et le
maximum √3 ≈ 1,732 calculés plus haut.

### Le rapport `[1]`

Dégelons enfin l'étape 1. L'ensemble des rayons qui rencontrent K, sur *toutes* les directions,
a pour taille

```text
mesure(K)  =  ∫ ombre(K, ω) dω  =  4π · ⟨ombre(K)⟩
```

la constante 4π étant celle de la sphère, donc **la même pour tout corps K**. L'intégrale porte
sur la sphère entière et compte donc chaque droite deux fois, une par sens de parcours ; c'est
un facteur 2 de plus, et il est lui aussi le même pour tout K. La probabilité conditionnelle
est le rapport de ces deux mesures, et toutes ces constantes se simplifient :

```text
P(B | A)  =  mesure(B) / mesure(A)
          =  ⟨ombre(B)⟩ / ⟨ombre(A)⟩
          =  half_area(B) / half_area(A)          pour deux AABB
          =  SA(B) / SA(A)                        [1]
```

Le facteur ½ du pivot disparaît au passage, exactement comme le facteur 2 de SA. C'est
précisément ce qui justifie que le code ne calcule ni l'un ni l'autre.

**Un piège à ne pas manquer.** Ce qui vient d'être écrit est un *rapport de moyennes*, pas la
*moyenne du rapport*. Les deux quantités diffèrent en général, et c'est bien la première que la
traversée paie : un compteur additionne les touches sur tous les rayons, puis divise une fois à
la fin — il ne moyenne pas des fréquences calculées direction par direction. L'ordre des deux
opérations est ici imposé par ce que l'on mesure, pas par une commodité algébrique.

### Le cas général : la formule de Cauchy

La boîte est tout ce dont le code a besoin, mais elle ne dit pas d'où sort le « / 4 » de `[1]`,
ni pourquoi la convexité est requise. C'est l'objet de la **formule de Cauchy** (1832) : pour
un convexe K de ℝ³,

```text
⟨ombre(K)⟩  =  SA(K) / 4
```

La preuve, pour un polyèdre, est un simple comptage.

1. Chaque face F, d'aire S_F et de normale n_F, projette une ombre d'aire S_F · |n_F ⋅ ω| —
   c'est le résultat de la sous-section « L'ombre d'une boîte », qui ne supposait rien de plus
   qu'une face plane.
2. **Le double recouvrement.** Pour un convexe et une direction générique, une droite qui
   traverse l'intérieur de l'ombre coupe la surface exactement **deux fois** : une entrée, une
   sortie. Les droites tangentes forment un ensemble d'aire nulle et ne comptent pas. Donc les
   ombres des faces, mises bout à bout, recouvrent l'ombre du corps exactement deux fois :

   ```text
   Σ_faces  S_F · |n_F ⋅ ω|   =   2 · ombre(K, ω)
   ```

3. On moyenne sur ω. Chaque |n_F ⋅ ω| vaut 1/2 en moyenne — le calcul de la sous-section
   précédente ne dépend pas de la normale choisie, par invariance par rotation :

   ```text
   ½ · Σ_faces S_F  =  2 · ⟨ombre(K)⟩       ⟺       ⟨ombre(K)⟩ = SA(K) / 4
   ```

Les corps lisses s'obtiennent comme limites de polyèdres ; les AABB, elles, sont déjà des
polyèdres, et aucun passage à la limite n'est nécessaire pour ce qui nous occupe.

**Deux contrôles.**

```text
la boîte    Σ_faces S_F = 2·half_area, donc ⟨ombre⟩ = ½·half_area     ← le pivot, retrouvé
la sphère   l'ombre est le disque πR² quelle que soit ω, donc ⟨ombre⟩ = πR²
            et SA/4 = 4πR²/4 = πR²                                    ← exact, pas approché
```

La sphère est le cas où il n'y a rien à moyenner : l'ombre ne dépend pas de la direction. Que
Cauchy y tombe exactement juste, et non à un facteur près, est la meilleure confirmation qu'on
puisse demander.

**Où la convexité intervient.** À l'étape 2, et nulle part ailleurs. Un corps non convexe peut
être traversé plus de deux fois, et le comptage s'effondre. Un cube creux le montre en trois
lignes :

```text
cube 1 × 1 × 1, évidé d'une cavité 0,8 × 0,8 × 0,8

surface totale        6 (extérieur)  +  6 · 0,64 = 3,84 (intérieur)   =   9,84
Cauchy annoncerait    9,84 / 4                                        =   2,46
ombre réelle          celle du cube plein, ½ · 3                      =   1,50
```

Une surestimation de 64 % : la paroi intérieure est comptée alors qu'elle ne projette aucune
ombre propre. Une droite qui traverse l'objet y coupe la surface quatre fois, pas deux.

C'est précisément pourquoi les AABB sont le bon support : elles sont convexes par construction,
et `[1]` leur est **exact**, pas approché. Le maillage qu'elles englobent, lui, ne l'est pas —
mais ce n'est jamais lui qu'on teste, c'est sa boîte.

### Pourquoi l'aire, et pas le volume

L'attente spontanée est le volume : « une plus grosse boîte est plus souvent touchée ». Elle est
fausse, et il vaut la peine de voir pourquoi. Un rayon est une sonde **unidimensionnelle** dans
un espace à trois dimensions ; ce qu'il rencontre est une silhouette, un objet à deux
dimensions. Les physiciens appellent cela une *section efficace*, et le nom dit l'essentiel.

**Par mise à l'échelle.** Doubler toutes les dimensions d'une boîte multiplie son volume par 8
et son `half_area` par 4. La probabilité de la toucher suit le carré, pas le cube. Toute
grandeur qui croît comme une longueur³ est donc disqualifiée d'office.

**À volume égal.** Deux boîtes de volume 1 :

| boîte | dimensions | volume | half_area | ⟨ombre⟩ |
|---|---|---|---|---|
| le cube | 1 × 1 × 1 | 1 | 3 | 1,50 |
| l'aiguille | 100 × 0,1 × 0,1 | 1 | 20,01 | 10,005 |

Même volume, et l'aiguille est **6,7 fois plus probable**. Ce n'est pas une curiosité : c'est
exactement la forme de l'enfant que produit une découpe médiane sur le maillage « amas dense +
quelques triangles lointains » de la §0. Une boîte immense, presque entièrement vide, que
presque tous les rayons doivent ouvrir. Le SAH le voit ; un critère fondé sur le volume, ou sur
les effectifs, ne le voit pas.

### Vérifier soi-même

`[1]` s'estime en quelques dizaines de lignes, et la recette du début **est** l'estimateur —
c'est l'intérêt de l'avoir posée sous cette forme :

```rust
// A ⊇ B, deux AABB. On estime P(rencontrer B | rencontrer A).
let (mut hits_a, mut hits_b) = (0u64, 0u64);
let center = a.centroid();

// La demi-diagonale de A majore son rayon dans toute direction, donc le disque de ce
// rayon contient l'ombre de A quelle que soit ω.
let radius = 0.5 * (a.bmax - a.bmin).length();

for _ in 0..n_samples {
    let omega = utils::random_unit_vector();                      // étape 1

    // Une base orthonormée (e1, e2) du plan ⊥ ω, pour y tirer le point d'entrée.
    let helper = if omega.x.abs() < 0.9 { Vector3f::new(1.0, 0.0, 0.0) }
                 else { Vector3f::new(0.0, 1.0, 0.0) };
    let e1 = vector3::cross(&omega, &helper).normalized();
    let e2 = vector3::cross(&omega, &e1);

    let d = utils::random_in_unit_disk() * radius;                // étape 2
    let entry = center + e1 * d.x + e2 * d.y;
    let origin = entry - omega * (2.0 * radius);                  // étape 3, reculé en amont
    let ray = Ray::new(&origin, &omega);

    if a.hit(&ray, 0.0, f64::INFINITY).is_some() {
        hits_a += 1;
        if b.hit(&ray, 0.0, f64::INFINITY).is_some() {
            hits_b += 1;
        }
    }
}
// hits_b / hits_a  ──→  b.half_area() / a.half_area()
```

Le disque doit être assez grand pour contenir l'ombre de A quelle que soit ω ; le prendre plus
grand encore ne biaise rien, cela ne fait qu'ajouter des rayons rejetés. Trois cas à tester :

| A | B | half_area(A) | half_area(B) | P attendue |
|---|---|---|---|---|
| 4 × 2 × 2 | 1 × 1 × 1, centrée | 20 | 3 | 0,150 |
| 2 × 2 × 2 | la même boîte | 12 | 12 | 1,000 |
| 2 × 2 × 2 | plaque 2 × 2 × 0 | 12 | 4 | 0,333 |

La deuxième ligne est le contrôle de bon sens. La troisième relie cette section à la §5 : une
boîte **plate** a une ombre moyenne non nulle, donc une probabilité d'être touchée non nulle, et
`half_area` le dit correctement — plat n'est pas vide.

### Les hypothèses sont fausses, et il faut le savoir

La recette du début est un choix, et ce choix ne décrit pas le rendu. Les étapes 1 et 2 posent
des directions uniformes et des origines rejetées à l'infini ; s'y ajoutent deux hypothèses
tacites, l'absence d'occultation et un rayon qui ne s'arrête pas dans la boîte. Aucune des
quatre n'est vérifiée dans un *path tracer*. Elles ne sont pas là parce qu'on les croit vraies,
mais parce qu'elles donnent le seul critère analytique disponible et qu'il fonctionne bien en
pratique. Les conséquences sont recensées en §6.

### Le corollaire de code : pourquoi `half_area`

L'aire d'une boîte de dimensions (dx, dy, dz) vaut

```text
SA = 2·(dx·dy + dy·dz + dz·dx)
```

et [`AABoundingBox::half_area`](../src/geom/aabound.rs) rend délibérément la parenthèse sans
le facteur 2. Ce n'est pas une approximation à corriger un jour, ni même une amputation : la
dérivation ci-dessus a montré que la parenthèse est la grandeur **naturelle** — le double de
l'ombre moyenne, donc, à une constante universelle près, la mesure des rayons qui rencontrent
la boîte. Le facteur 2 de SA n'est que ce qui reste d'une convention d'aire de surface, et il
se simplifie dans **tous** les usages : dans `[1]` c'est un rapport d'aires, donc il disparaît ;
dans le test de rentabilité `[5]` il figure des deux côtés de l'inégalité. La demi-aire est la
forme juste, et la nommer ainsi évite de payer une multiplication par nœud pour une constante
qui s'annule.

---

## 2. La fonction de coût

Soit un nœud contenant N primitives, de boîte d'aire A. S'il est découpé en deux enfants L et
R, d'aires A_L et A_R et d'effectifs N_L et N_R, le coût espéré de la traversée du nœud est

```text
C(nœud)  =  t_trav  +  p_L · C(L)  +  p_R · C(R)                        [2]

avec, par [1],      p_L = A_L / A        p_R = A_R / A
```

`t_trav` est le coût d'un test de boîte, payé à coup sûr ; les coûts des enfants ne sont payés
qu'avec la probabilité de les atteindre.

`[2]` est une récurrence sur tout le sous-arbre, dont on ne connaît pas la forme au moment de
choisir le plan. Le SAH la tronque : **on suppose que les deux enfants seront des feuilles**,
c'est-à-dire que C(L) = t_isect · N_L et C(R) = t_isect · N_R. C'est l'approximation gloutonne,
et elle donne

```text
C(split)  ≈  t_trav  +  t_isect · (A_L·N_L + A_R·N_R) / A               [3]
```

Reste l'autre branche de l'alternative : ne pas découper du tout, et garder le nœud comme
feuille, ce qui coûte

```text
C(feuille)  =  t_isect · N                                              [4]
```

Choisir le meilleur plan, c'est minimiser `[3]` ; décider s'il faut découper, c'est comparer
`[3]` à `[4]`.

### L'algèbre qui donne la convention du code

Deux simplifications font passer de `[3]` à ce que le code calcule, et il vaut la peine de les
écrire, parce que les manquer conduit à « corriger » un membre sans l'autre.

**Un.** Pour le choix du plan, `A` est commun à tous les candidats d'un même nœud, et `t_trav`
est une constante additive. Ni l'un ni l'autre ne change l'`argmin` — le plan *qui* minimise,
par opposition à la valeur du minimum :

```text
argmin  [3]  =  argmin  (A_L·N_L + A_R·N_R)
```

C'est pourquoi `find_best_split_plane` ne normalise pas : la quantité qu'il minimise n'est pas
un coût, c'est un coût **à une transformation affine croissante près** — on lui a retiré une
constante et on l'a divisé par un facteur positif, deux opérations qui laissent le *classement*
des candidats intact. Or seul le classement nous intéresse ici.

**Deux.** Pour la décision feuille/nœud interne, il faut comparer des quantités de même
nature. Le code pose `t_trav = 0` et `t_isect = 1` (voir §6, c'est un écart au modèle), puis
multiplie les deux membres par `A` :

```text
   t_isect·(A_L·N_L + A_R·N_R) / A   <   t_isect · N          [3] < [4]
⟺  (A_L·N_L + A_R·N_R)               <   A · N                ×A, A > 0

découper vaut mieux  ⟺  A_L·N_L + A_R·N_R  <  A_node · N                [5]
```

Le membre de gauche est ce que rend `find_best_split_plane`, celui de droite est exactement
`calculate_node_cost`. **Les deux membres partagent la même convention non normalisée : ne
toucher qu'à l'un des deux redéfinit silencieusement le test.** C'est la seule raison pour
laquelle la comparaison de `subdivide` est correcte telle qu'elle est écrite.

À titre de comparaison, pbrt normalise l'autre côté : il divise par l'aire du nœud et compare
à `N` directement, avec `t_trav = 0,125` — la même inégalité, écrite dans l'autre sens. Les
deux conventions sont valides, aucune n'est mixable avec l'autre.

---

## 3. Pourquoi les *unions*, et pas les bins

C'est la section décisive, et celle que le code violait.

`A_L` de `[3]` est l'aire de la boîte du **futur enfant gauche**. Cet enfant contiendra toutes
les primitives situées à gauche du plan, donc sa boîte est l'**union** des boîtes de tous les
bins à gauche. L'aire d'un bin isolé n'est la probabilité de rien : aucun rayon ne « rencontre
le bin 3 » comme événement de la traversée — la traversée rencontre l'enfant gauche, ou pas.

Le balayage se fait donc en aires cumulées, préfixe et suffixe :

```text
A_L(i) = aire( bin₀ ∪ bin₁ ∪ … ∪ binᵢ )
A_R(i) = aire( binᵢ₊₁ ∪ … ∪ bin_{K−1} )
```

### Le contre-exemple qui tranche

Que la forme par bin soit « moins précise » serait bénin. Elle est bien pire : elle peut être
**dépourvue de toute information**. Construisons huit bins alignés sur x, chacun contenant un
triangle, chacun de boîte cubique 1 × 1 × 1, posés bout à bout :

```text
x →   0     1     2     3     4     5     6     7     8
      ├─────┼─────┼─────┼─────┼─────┼─────┼─────┼─────┤
bin      0     1     2     3     4     5     6     7
Nᵢ       1     1     1     1     1     1     1     1
```

L'union de k bins consécutifs est une boîte k × 1 × 1, d'aire demi `k·1 + 1·1 + 1·k = 2k + 1`.
Un bin seul, k = 1, a donc pour aire demi 3.

| plan i | N_L | N_R | par bin : A_L = A_R = 3 | par union : A_L = 2(i+1)+1, A_R = 2(7−i)+1 |
|---|---|---|---|---|
| 0 | 1 | 7 | 3·1 + 3·7 = **24** | 3·1 + 15·7 = **108** |
| 1 | 2 | 6 | 3·2 + 3·6 = **24** | 5·2 + 13·6 = **88** |
| 2 | 3 | 5 | 3·3 + 3·5 = **24** | 7·3 + 11·5 = **76** |
| 3 | 4 | 4 | 3·4 + 3·4 = **24** | 9·4 + 9·4 = **72** |
| 4 | 5 | 3 | **24** | 11·5 + 7·3 = **76** |
| 5 | 6 | 2 | **24** | 13·6 + 5·2 = **88** |
| 6 | 7 | 1 | **24** | 15·7 + 3·1 = **108** |

La colonne « par union » a un minimum net en i = 3, le plan médian — le bon. La colonne « par
bin » est **constante** : les sept plans sont à égalité, le critère ne distingue rien. Et comme
la boucle de sélection retient le premier candidat rencontré (comparaison stricte
`plane_cost < best_cost`), le plan élu est i = 0, le plus déséquilibré des sept.

C'est ainsi que les deux défauts se composaient : la forme par bin élisait le plan d'indice 0,
et le décalage d'un bin de la §4 plaçait ce plan-là sur `min_c` lui-même, côté gauche vide —
d'où le garde `left_count == 0` de `subdivide`, d'où l'arrêt de la subdivision. Les mesures
consignées dans [IDEAS.md](../IDEAS.md) sont la trace exacte de cette mécanique : des feuilles
contenant un tiers du maillage.

Le coût du nœud, pour compléter : l'union des huit bins a pour aire demi 2·8 + 1 = 17, donc
`[5]` donne 17 · 8 = 136. Le coût par union, 72, est bien inférieur : découper est rentable.

### L'invariant que la forme fautive ne possède pas

Une union ne peut que grandir quand on lui ajoute une boîte. Donc

```text
A_L(0) ≤ A_L(1) ≤ … ≤ A_L(K−2)          croissante
A_R(0) ≥ A_R(1) ≥ … ≥ A_R(K−2)          décroissante
```

Dans le tableau ci-dessus, la colonne des unions vérifie 3 ≤ 5 ≤ 7 ≤ 9 ≤ 11 ≤ 13 ≤ 15 ; les
aires par bin d'un maillage réel, elles, oscillent au gré de la taille des bins. Un
`debug_assert!` de monotonie dans la boucle d'accumulation aurait donc levé le défaut au
premier build de debug sur le lapin.

Portée honnête de ce garde : il attrape les suites non monotones, ce que les aires par bin sont
en général — mais pas le cas pathologique du contre-exemple ci-dessus, où elles sont constantes,
donc à la fois croissantes et décroissantes. C'est un garde utile, pas une preuve.

---

## 4. Le binning : de O(N²) à O(N)

Le SAH exhaustif considère comme candidat tout plan séparant effectivement deux primitives, soit
O(N) plans par axe ; évaluer chacun demande de classer les N primitives, soit O(N). Le coût est
donc **O(N²) par nœud**, ce qui est inutilisable sur un maillage de 871 414 triangles.

Le binning quantifie l'ensemble des candidats. On découpe l'étendue en K intervalles égaux, on
fait **une** passe O(N) qui accumule dans chaque bin une boîte et un effectif, puis un balayage
O(K) qui calcule les aires cumulées des deux côtés de chacune des K−1 frontières. Coût :
**O(N + K) par nœud**, avec K−1 = 7 plans candidats par axe au lieu de N.

`BIN_COUNT` vaut 8 ici ; pbrt utilise 12. C'est un levier mesurable : plus de bins rapproche du
SAH exhaustif et coûte plus cher à construire.

### Trois points de cadre, chacun source d'erreur

**Un — la classification porte sur les centroïdes, les aires sur les boîtes réelles.** Un
triangle va tout entier d'un côté du plan ; ce qui décide de quel côté, c'est son centroïde. En
revanche la boîte qui entre dans l'union est sa boîte englobante complète, qui **déborde** du
plan. Les deux enfants se recouvrent donc, et c'est correct : la boîte d'un enfant doit contenir
ses primitives en entier. Contre-intuitif, mais nécessaire.

**Deux — l'étendue binnée est celle des centroïdes, pas la boîte du nœud.** Un plan situé hors
de l'intervalle des centroïdes laisse forcément un côté vide, donc ne découpe rien. Binner sur
la boîte du nœud gaspillerait des bins sur des zones où aucun centroïde ne tombe.

**Trois — la frontière du bin i est à `min_c + (i+1)·w`.** Avec K bins il y a K−1 frontières
*internes*, et la frontière d'indice i est celle qui suit le bin i :

```text
centroïdes  min_c ├─────┬─────┬─────┬─────┬─────┬─────┬─────┤ max_c
bins                 0     1     2     3     4     5     6     7
frontières           └──0──┴──1──┴──2──┴──3──┴──4──┴──5──┴──6──┘
                     ↑                                         ↑
              min_c + 1·w                            min_c + 7·w
```

```text
frontière(i)  =  min_c + (i + 1)·w              i ∈ [0, K−2]            [6]
bin(p)        =  clamp( ⌊(p − min_c) / w⌋ , 0, K−1 )
```

Écrire `min_c + i·w` place la frontière 0 sur `min_c` lui-même : aucun centroïde n'est
strictement inférieur au minimum, le côté gauche est vide, et tous les autres plans sont
décalés d'un bin. C'est le second défaut corrigé par ce chantier.

### Partitionner par indice de bin, pas par position

Le coût d'un plan est calculé à partir des **effectifs par bin**, obtenus par `bin(p)`.
Si le partitionnement, lui, compare `centroïde < frontière(i)`, il exécute un *autre* calcul :
`⌊(p − min_c)/w⌋` et `p < min_c + (i+1)·w` sont deux expressions flottantes distinctes, dont
les erreurs d'arrondi diffèrent — voir [arithmetique_flottante.md](arithmetique_flottante.md)
§2. Pour un centroïde posé sur une frontière, elles peuvent se contredire.

La conséquence n'est pas une imprécision, c'est une **incohérence** : le plan retenu l'a été
pour un coût calculé sur une partition, et la partition réellement effectuée en est une autre,
de coût différent. Dans le cas limite où tout un côté basculerait, le garde
`left_count == 0` annulerait un découpage pourtant jugé rentable.

Le remède est structurel : le partitionnement appelle `bin_index`, **la même fonction** que le
binning, et compare des indices entiers. Effectifs prédits et partition effective coïncident
alors par construction, et non parce que les deux calculs sont d'accord la plupart du temps.

---

## 5. Le cas dégénéré : le vide

Trois situations, à traiter explicitement plutôt qu'à laisser à l'arithmétique.

**L'étendue des centroïdes est nulle** (`min_c == max_c`) : tous les centroïdes coïncident sur
cet axe, aucun plan ne sépare quoi que ce soit, et `w = 0` ferait une division par zéro. L'axe
est écarté. Si les trois axes le sont, le nœud reste feuille.

**Un côté du plan est vide** (`N_L == 0` ou `N_R == 0`) : ce n'est pas un découpage. Le rejet
doit être explicite. Avant ce chantier il se produisait par empoisonnement arithmétique : une
boîte vide rendait une aire `+inf`, et `+inf × N` puis `NaN < best_cost` faisaient tomber le
candidat par accident — voir §0 de [arithmetique_flottante.md](arithmetique_flottante.md) pour
la raison du débordement `f64::MIN − f64::MAX → −inf`. Depuis que
[`AABoundingBox::half_area`](../src/geom/aabound.rs) rend `0` sur une boîte vide, l'accident a
changé de sens : un côté vide fait paraître le plan **gratuit**. Le test explicite est donc la
seule forme correcte, sous l'une ou l'autre convention.

**Plat n'est pas vide.** Une boîte d'épaisseur nulle sur un axe — un triangle aligné sur un
plan de coordonnées — contient des points et a en général une aire non nulle : pour
d = (2, 0, 3), l'aire demi vaut 0 + 0 + 6 = 6. Elle doit donc contribuer son coût réel. C'est
ce que verrouillent `test_degenerate_box_area_is_faithful` et
`test_empty_and_flat_are_different_states`, et c'est aussi pourquoi `AABoundingBox::new` stocke
ses bornes fidèlement au lieu de gonfler les boîtes dégénérées, ce qu'elle a fait un temps.

---

## 6. Ce que le modèle ne couvre pas

Le SAH est une heuristique, et le savoir fait partie de son usage correct.

- **La traversée est comptée gratuite.** Le code pose `t_trav = 0` et `t_isect = 1` dans `[5]`,
  là où pbrt pondère la traversée à 1/8 du coût d'une intersection. Conséquence : le test de
  rentabilité surestime l'intérêt de découper, donc produit des feuilles plus petites que
  l'optimum. C'est un écart assumé, pas un oubli — introduire la constante en même temps que la
  correction du coût rendrait le avant/après inattribuable.
- **L'approximation est gloutonne et locale.** `[3]` suppose les deux enfants feuilles, ce qui
  est faux dès qu'ils sont découpés à leur tour, et le choix est fait nœud par nœud sans
  retour. L'arbre obtenu n'est pas l'optimum de `[2]`, et le SAH ne prétend pas l'être.
- **La distribution de rayons de `[1]` n'est pas celle du rendu.** Les rayons primaires
  partagent une origine et sont fortement cohérents ; les rayons d'ombre sont des *segments*
  bornés, pour lesquels la probabilité de rencontre n'est pas un rapport d'aires ; l'occultation
  fait qu'un rayon s'arrête. Et `bvh_stats` ne mesure aujourd'hui que les primaires, faute de
  générateur reproductible pour les autres.
- **Aucun découpage spatial.** Un triangle à cheval sur le plan est attribué entièrement à un
  côté, et sa boîte gonfle celle de l'enfant. Le SBVH de Stich et al. autorise à scinder la
  primitive elle-même, au prix de références dupliquées.
- **Le binning quantifie les candidats** : K−1 plans par axe au lieu de O(N). Le plan optimal
  peut se trouver entre deux frontières.
- **Le recouvrement des enfants n'est pas pénalisé.** Deux enfants dont les boîtes se chevauchent
  largement obligent souvent à visiter les deux, ce que `[3]` ne modélise pas : la somme
  `p_L + p_R` peut dépasser 1 sans que le coût en rende compte.

---

## 7. Ce que cela donne dans le code

Les §2 à §5 traitent chacune une pièce ; aucune ne montre l'assemblage. Voici la construction
entière, en deux procédures, chaque ligne annotée de l'équation et de la section qui la
justifient. La colonne de droite est le seul intérêt de ce pseudo-code : le vrai code Rust dit
*comment*, elle dit *au nom de quoi*.

```text
subdivide(nœud)                                                    ── la récursion
────────────────────────────────────────────────────────────────────────────────────────
  coût_feuille ← half_area(boîte(nœud)) · N(nœud)                  calculate_node_cost, [5]
  plan         ← meilleur_plan(nœud)

  si plan est absent           → laisser une feuille               aucun axe séparable, §5
  si plan.coût ≥ coût_feuille  → laisser une feuille               [5], découper ne paie pas

  ── partition en place, avec la MÊME fonction que le binning ──────────────────── §4
  fin_gauche ← début
  pour chaque triangle t du nœud, dans l'ordre :
      si bin_index(centroïde(t)[plan.axe], plan.min_c, plan.w) ≤ plan.frontière :
          échanger t et le triangle en fin_gauche                  passe unique, vers l'avant
          fin_gauche ← fin_gauche + 1

  si fin_gauche = début ou fin_gauche = fin  → abandonner          inatteignable, voir plus bas

  enfant gauche ← triangles [début, fin_gauche)                    boîte = union des boîtes
  enfant droit  ← triangles [fin_gauche, fin)                      *complètes*, §4
  subdivide(enfant gauche)  ;  subdivide(enfant droit)
```

```text
meilleur_plan(nœud) → (axe, frontière, coût)  ou  rien             ── le cœur du SAH
────────────────────────────────────────────────────────────────────────────────────────
  meilleur ← rien

  pour axe ∈ {x, y, z} :

      ── l'étendue binnée est celle des CENTROÏDES, pas la boîte du nœud ───────── §4
      (min_c, max_c) ← étendue des centroïdes du nœud sur cet axe
      si min_c = max_c :  axe suivant                              aucun plan ne sépare, §5
      w ← (max_c − min_c) / K                                      K = BIN_COUNT = 8

      ── une passe O(N) : chaque triangle tombe dans un bin ────────────────────── §4
      pour chaque triangle t du nœud :
          b ← bin_index(centroïde(t)[axe], min_c, w)               le centroïde classe…
          bin[b].boîte ← bin[b].boîte ∪ boîte_complète(t)          …la boîte entière s'accumule
          bin[b].n     ← bin[b].n + 1

      ── un balayage O(K) : les aires sont celles des UNIONS ───────────────────── §3
      pour i de 0 à K−2 :                                          K−1 = 7 frontières, [6]
          A_L[i] ← half_area( bin₀ ∪ … ∪ binᵢ )                    préfixe
          N_L[i] ← n₀ + … + nᵢ
          A_R[i] ← half_area( binᵢ₊₁ ∪ … ∪ bin_{K−1} )             suffixe
          N_R[i] ← nᵢ₊₁ + … + n_{K−1}

      invariant : A_L croissante, A_R décroissante                 debug_assert, §3

      ── K−1 candidats, tous du même nœud, donc comparables non normalisés ─────── §2
      pour frontière i de 0 à K−2 :
          si N_L[i] = 0 ou N_R[i] = 0 :  candidat suivant          un côté vide, §5
          coût ← A_L[i]·N_L[i] + A_R[i]·N_R[i]                     [3] privé de A et t_trav
          si coût < meilleur.coût :  meilleur ← (axe, i, coût)     strict : le premier gagne

  rendre meilleur
```

Le coût de construction se lit directement : trois axes, chacun une passe O(N) et un balayage
O(K), soit **O(N + K) par nœud** au lieu du O(N²) du SAH exhaustif — c'est le calcul de la §4.

### Trois choses que le pseudo-code rend visibles

Elles sont dispersées dans le texte, et c'est en les voyant côte à côte qu'on les retient.

- **`bin_index` apparaît deux fois**, une fois dans `meilleur_plan` et une fois dans
  `subdivide`. Ce n'est pas une répétition maladroite : c'est ce qui garantit que les effectifs
  qui ont produit le coût sont ceux de la partition réellement effectuée. Toute la §4 tient à
  ce que ces deux occurrences soient la même fonction, jamais deux expressions équivalentes.
  C'est aussi ce qui rend **inatteignable** le garde qui suit la partition : les deux passes
  appliquent la même fonction pure aux mêmes triangles, donc elles ne peuvent pas se
  contredire. Le code le conserve tout de même, en `debug_assert!`, parce que l'alternative
  serait un enfant vide et une récursion sans fin.
- **Les trois cas dégénérés de la §5 sont trois gardes distincts**, à trois endroits distincts :
  `min_c = max_c` écarte un axe, `N_L = 0 ou N_R = 0` écarte un candidat, `plan est absent`
  écarte le découpage entier. Aucun ne peut couvrir les deux autres.
- **Les aires ne sont lues qu'en un seul endroit**, sur les unions accumulées du balayage. Il
  n'existe nulle part dans l'algorithme d'expression qui lise l'aire d'un bin isolé — et c'est
  précisément ce que la §3 exige.

### Où vit chaque équation

| équation | où |
|---|---|
| `[1]` — l'aire comme probabilité | [`AABoundingBox::half_area`](../src/geom/aabound.rs) |
| `[3]` — le coût d'un plan candidat | `BVHTree::find_best_split_plane` |
| `[5]` — découper ou faire une feuille | `BVHTree::calculate_node_cost` et le test de `BVHTree::subdivide` |
| `[6]` — frontières et indices de bin | `BVHTree::bin_index` et le partitionnement de `subdivide` |

Le chemin exhaustif `BVHTree::exhaustive_split_cost` calcule `[3]` en balayant toutes les
primitives, sans binning. Il n'est pas utilisé par la construction : il sert d'oracle au test
d'équivalence, qui vérifie que le balayage préfixe/suffixe binné donne bien le même coût. C'est
le test qui aurait rendu visible la lecture par bin de la §3.

---

## 8. Pour aller plus loin

- MacDonald & Booth (1990), Wald (2007) et Havran (2000) — voir l'en-tête. Havran ch. 4 est la
  référence à ouvrir pour la mesure des droites et la formule de Cauchy.
- M. Stich, H. Friedrich, A. Dietrich, *Spatial Splits in Bounding Volume Hierarchies*, High
  Performance Graphics, 2009 — les découpages spatiaux, la limite §6.
- [PBR Book, *Bounding Volume Hierarchies*](https://www.pbr-book.org/3ed-2018/Primitives_and_Intersection_Acceleration/Bounding_Volume_Hierarchies)
  — l'implémentation binnée de référence, et la convention de coût normalisée de la §2.
- [arithmetique_flottante.md](arithmetique_flottante.md) — le modèle d'erreur invoqué en §4 et
  §5, et la raison du débordement d'une boîte vide.
