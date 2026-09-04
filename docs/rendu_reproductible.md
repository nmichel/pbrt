# Un rendu qui se répète : l'unité adressée et sa clé de flux

Ce document explique comment [`IndependentSampler`](../src/samplers/independent.rs) fabrique le
flux de nombres d'un échantillon — les constantes `PIXEL_BITS` et `SAMPLE_INDEX_BITS`, et
l'expression de la clé dans `IndependentSampler::new`.

Il se lit en deux moitiés. Les §0 à §2 posent le problème et sa solution : ce qu'un couple
(pixel, échantillon) désigne, les deux propriétés que la clé doit garantir, et le découpage en
champs de bits qui les obtient. Les §3 à §6 justifient les décisions : pourquoi l'aplatissement
arithmétique — celui qu'on écrirait spontanément — est écarté, ce que son remplacement achète,
comment la graine entre, et jusqu'où la promesse porte.

**C'est ici que vit l'explication détaillée**, avec ses tableaux et ses exemples chiffrés ; le
doc-comment de `IndependentSampler` en donne une version resserrée, suffisante pour lire le code
sans quitter le fichier. En cas de doute sur la forme de la clé, c'est la §2 qu'il faut ouvrir ;
sur le rôle de la graine, la §5.

Le branchement du sampler dans les renderers — qui construit celui d'un échantillon, et quand —
n'est pas décrit ici : il est dans [ideas/rng_graine.md](../ideas/rng_graine.md) tant qu'il n'est
pas fait.

Références :

- [PBR Book, 4ᵉ éd., *Sampling Interface*](https://pbr-book.org/4ed/Sampling_and_Reconstruction/Sampling_Interface)
  et [*Independent Sampler*](https://pbr-book.org/4ed/Sampling_and_Reconstruction/Independent_Sampler)
  — l'interface dont celle du projet est une réduction, et la même stratégie de flux par
  échantillon.
- M. E. O'Neill, *PCG: A Family of Simple Fast Space-Efficient Statistically Good Algorithms for
  Random Number Generation*, Harvey Mudd College, 2014 — le générateur retenu, §6.
- La documentation de `SeedableRng::seed_from_u64` de `rand_core`, dont la garantie est celle sur
  laquelle repose la §5.

---

## 0. L'unité adressée : un couple (pixel, échantillon)

Le renderer est fait de deux boucles imbriquées. En pseudo-code, ce que
[`compute_pixel`](../src/renderers/st.rs) exécute :

```text
pour chaque pixel (x, y)                    ← boucle externe, largeur × hauteur tours
    pour sample_index de 0 à N−1            ← boucle interne, N = samples_ppx tours
        tirer un point dans le pixel
        lancer un rayon, suivre un chemin   → une radiance Lᵢ
    couleur du pixel = (L₀ + L₁ + … + L_{N−1}) / N
```

`sample_index` est le compteur de la boucle **interne** : il dit lequel des N échantillons
Monte-Carlo de *ce* pixel est en cours de calcul, et il repart de zéro à chaque pixel.

D'où l'observation qui commande tout le reste : **un couple (pixel, `sample_index`) désigne
exactement un chemin** parmi tous ceux qu'un rendu trace. C'est l'unité qui doit recevoir son
propre flux de nombres, et c'est pourquoi la clé se construit sur ces coordonnées-là et sur aucune
autre — en particulier ni sur le thread qui exécute le tour de boucle, ni sur le rang du chemin
dans l'ordre où les threads l'ont pris.

## 1. Les deux propriétés que la clé doit garantir

La clé est le `u64` que `IndependentSampler::new` fabrique à partir de `(x, y, sample_index)` et
remet à `seed_from_u64` pour amorcer le générateur. Elle doit assurer deux choses, dont une seule
va de soi.

**[P1] Deux échantillons distincts ne partagent jamais une clé.** Sinon ils partagent le flux,
donc tirent les mêmes nombres, donc prennent les mêmes décisions. Deux pixels qui échantillonnent
identiquement ne produisent pas du bruit mais une **structure** — une régularité que l'œil lit
comme un défaut de géométrie ou de matériau, et qu'on cherchera longtemps ailleurs.

**[P2] La clé ne dépend pas de `samples_ppx`.** Celle-ci ne se devine pas, et c'est l'objet des
§3 et §4.

## 2. Le découpage en champs de bits

Un `u64` porte 64 bits. On les partage en trois tranches disjointes :

```text
 63           48 47           32 31                              0
┌───────────────┬───────────────┬────────────────────────────────┐
│       x       │       y       │          sample_index          │
│    16 bits    │    16 bits    │            32 bits             │
└───────────────┴───────────────┴────────────────────────────────┘
   PIXEL_BITS      PIXEL_BITS          SAMPLE_INDEX_BITS
```

soit

```text
clé = x ⋅ 2⁴⁸ + y ⋅ 2³² + sample_index                                    [1]
```

16 + 16 + 32 = 64, exactement. Les bornes que `new` vérifie en `debug_assert` sont celles des
champs : 65 536 pixels de côté, et 2³² échantillons par pixel.

**Pourquoi [1] satisfait [P1], en une phrase** : puisque les champs ne se chevauchent pas, on peut
*relire* `x`, `y` et `sample_index` depuis la clé par un décalage et un masque. Une application
qu'on sait inverser ne peut pas envoyer deux entrées différentes sur la même sortie.

En hexadécimal les trois champs se lisent à l'œil nu, un groupe chacun :

```text
x = 2, y = 3, sample_index = 1   →   0x0002_0003_00000001
                                       ─┬── ─┬── ────┬───
                                        x    y   sample_index
```

et les voisins immédiats se lisent de même :

| échantillon | clé |
|---|---|
| pixel (2,3), échantillon 1 | `0x0002_0003_00000001` |
| pixel (2,3), échantillon 2 | `0x0002_0003_00000002` |
| pixel (3,3), échantillon 1 | `0x0003_0003_00000001` |

## 3. L'aplatissement arithmétique, et pourquoi il est écarté

La façon classique de numéroter les tours d'une double boucle est arithmétique :

```text
pixel_index = y ⋅ largeur + x
clé         = pixel_index ⋅ samples_ppx + sample_index                    [2]
```

[2] satisfait [P1] : elle est injective tant que `sample_index < samples_ppx`. Elle échoue sur
[P2], et il faut voir *pourquoi* plutôt que le constater, parce que la raison est instructive.

Le nombre que [2] produit répond à la question : **combien de chemins ont été tracés avant
celui-ci, dans l'ordre des boucles ?** Or cette réponse dépend de combien d'échantillons chaque
pixel précédent a pris. Sur trois pixels :

| | N = 2 | N = 3 |
|---|---|---|
| pixel 0 | chemins **0, 1** | chemins **0, 1**, 2 |
| pixel 1 | chemins 2, 3 | chemins 3, 4, 5 |
| pixel 2 | chemins 4, 5 | chemins 6, 7, 8 |

Le premier échantillon du pixel 1 portait le numéro 2 ; il porte maintenant le numéro 3. Numéro
différent → clé différente → graine différente → flux différent → **autre chemin**. Seuls les
échantillons du pixel 0 gardent leur numéro, et c'est un accident de bord.

C'est un décalage au sens le plus banal : insérer un élément dans une liste renumérote tout ce qui
suit.

### Réserver, ou rationner

L'opposition entre [1] et [2] se dit comme une histoire de tableau, et c'est la formulation à
retenir.

**[2] tasse les lignes bout à bout.** Le pas d'une ligne *est* N, donc changer N ré-agence tout :

```text
N = 2 :  [p0.s0][p0.s1][p1.s0][p1.s1][p2.s0][p2.s1]
N = 3 :  [p0.s0][p0.s1][p0.s2][p1.s0][p1.s1][p1.s2]…
                              ↑ p1.s0 a bougé
```

**[1] donne à chaque pixel sa propre ligne, de longueur fixe** — 2³² emplacements, quel que soit
N. On n'en consomme que les N premiers, et les autres ne coûtent rien puisqu'ils ne sont jamais
adressés :

```text
pixel 0 │ s0  s1  s2  s3 …  (2³² emplacements)
pixel 1 │ s0  s1  s2  s3 …
pixel 2 │ s0  s1  s2  s3 …
          └── on n'en consomme que N, et l'adresse de p1.s0 ne dépend pas de N
```

L'adresse d'un échantillon est sa **position**, pas son **rang**. Réserver, c'est fixer le pas de
ligne une fois pour toutes ; rationner, c'est le recalculer à chaque rendu selon ce qu'on demande.
C'est ce recalcul qui fait entrer `samples_ppx` dans toutes les clés.

## 4. Ce que [P2] achète

Écrivons les deux moyennes d'un même pixel l'une sous l'autre :

```text
rendu à 4 échantillons :  (L₀ + L₁ + L₂ + L₃) / 4
rendu à 8 échantillons :  (L₀ + L₁ + L₂ + L₃ + L₄ + L₅ + L₆ + L₇) / 8
```

Sous [1], les clés d'un pixel sont `…_00000000` à `…_00000003` dans le premier cas et
`…_00000000` à `…_00000007` dans le second : **les quatre premières sont les mêmes**. Donc
`L₀ … L₃` sont les mêmes quatre nombres — mêmes clés, mêmes flux, mêmes chemins — et l'image à 8
échantillons *prolonge* celle à 4 en ajoutant quatre termes à une moyenne déjà calculée.

Sous [2], les huit `Lᵢ` de la seconde ligne sont huit chemins que la première n'a jamais tracés.
Les deux images n'ont en commun que la géométrie.

Deux conséquences pratiques :

- un aperçu à faible échantillonnage montre **la même réalisation du bruit** que l'image finale,
  en plus grossier ; le rendu final raffine l'aperçu au lieu de le remplacer ;
- comparer deux versions du code à 4 échantillons est bon marché **et** transportable : la
  comparaison chère commence par exactement les chemins de la comparaison bon marché.

## 5. La graine, et le partage des rôles

La clé finale n'est pas `[1]` mais `mix_seed(seed) ^ clé`, où `seed` est le `u64` que `--seed` fixe
pour tout le rendu. Le `mix_seed` n'est pas décoratif — la sous-section *Pourquoi la graine ne peut
pas entrer telle quelle* dit ce qui casse sans lui.

### À quoi sert la graine, puisque la clé suffit déjà

La question est légitime, et elle se pose exactement dans ces termes : `[1]` rend déjà le flux
d'un échantillon parfaitement déterminé, donc l'image se répète. Alors pourquoi un ingrédient de
plus ?

**Parce que sans lui, une scène et des options n'auraient qu'une seule image possible, pour
toujours.** La graine ne sert pas à rendre le rendu reproductible ; elle sert à en avoir
*plusieurs*, chacun reproductible.

Et statistiquement elle n'apporte rien : à `seed` fixée, elle ne fait que réétiqueter l'ensemble
des flux, sans en changer la qualité. C'est un **bouton**, pas un mécanisme. Sa justification est
pratique, et elle a deux usages.

### Premier usage : distinguer le bruit d'un bug

C'est l'usage qui compte, parce qu'il répare quelque chose que le déterminisme **casse**.

Supposons une tache claire isolée sur un mur du Cornell box. Deux explications possibles, et elles
n'appellent pas du tout le même travail :

- une *firefly* — un chemin a trouvé la lumière par une route improbable, sa contribution énorme
  n'a pas été noyée par les 4 autres échantillons du pixel. C'est du bruit, et il disparaîtra en
  augmentant `--samples_ppx` ;
- un défaut — une normale retournée, une forme qui fuit, un cosinus manquant. Aucun nombre
  d'échantillons ne le fera partir.

Aujourd'hui, on tranche gratuitement : on relance, le tirage change, et si la tache bouge c'est du
bruit. C'est le premier réflexe de diagnostic, et il ne coûte rien.

**Une fois ce chantier terminé, deux exécutions sont identiques au bit : la tache ne bougera plus
jamais**, qu'elle soit une firefly ou un bug. L'expérience disparaît, et rien ne signale qu'elle a
disparu. La graine la rend :

```text
pbrt … --seed 0   →  la tache est là
pbrt … --seed 0   →  la tache est là           (forcément — c'est la même image)
pbrt … --seed 1   →  la tache a disparu        → c'était du bruit
                  →  la tache est toujours là  → c'est un défaut du code ou de la scène
```

Et l'expérience est désormais meilleure qu'avant, parce qu'elle est elle-même reproductible : on
peut la refaire à l'identique, et citer `--seed 1` dans un commit ou un rapport de bug.

### Second usage : mesurer une variance

On ne peut pas estimer la dispersion d'un estimateur depuis **une** réalisation. Or c'est
exactement ce qu'il faut pour démontrer qu'une technique de réduction de variance marche.

Prenons la promesse du sampler stratifié — « moins de bruit à échantillonnage égal ». Un rendu de
chaque ne prouve rien : l'un peut être plus propre par chance. La mesure est celle-ci, sur un pixel
choisi, ou sur une image entière pixel par pixel :

```text
10 rendus indépendants, --seed 0 … 9, sampler indépendant   → 10 valeurs, écart-type 0.081
10 rendus indépendants, --seed 0 … 9, sampler stratifié     → 10 valeurs, écart-type 0.034
```

L'écart-type **entre les graines** est précisément l'erreur type de l'estimateur — la grandeur que
« réduire la variance » prétend faire baisser. Sans graine variable, on n'a qu'un nombre par
sampler et rien à comparer.

Le même argument vaut en petit pour les tests du sampler, et il y montre les deux faces d'un même
choix. Leur graine est **fixe**, ce qui les rend déterministes : ils ne peuvent pas échouer par
malchance un jour et passer le lendemain. Mais du même coup, chacun n'éprouve qu'**un seul flux**
parmi tous ceux que la clé sait produire.

Le test des moments démontre donc que *ce* flux-là a une moyenne de 1/2 à 5 ⋅ 10⁻³ près. Il ne
démontre pas que les flux en général l'ont. Pour cela, il faudrait rejouer le même test sur
beaucoup de graines et regarder combien s'écartent — ce qui est de nouveau une mesure de
dispersion, donc de nouveau une chose qu'une seule graine ne peut pas donner.

### Où la graine vit, et son défaut

Puisque la promesse est « mêmes options → même image », la graine **est une option**, au même titre
que `--samples_ppx`. D'où sa place dans `Config` plutôt qu'une constante de module qu'il faudrait
recompiler pour changer.

Et son défaut est une constante, **pas l'horloge** : le déterminisme est le comportement normal, la
variation est ce qu'on demande explicitement.

### Pourquoi la graine ne peut pas entrer telle quelle

L'idée naturelle est d'amorcer avec `seed ^ clé`. Elle échoue, et son échec est silencieux :
`--seed 1` rend alors **exactement la même image** que `--seed 0`, sans rien qui le signale.

La cause tient à la forme de [1]. Pour un pixel donné, les clés de ses échantillons ne diffèrent
que par le champ bas :

```text
clé de l'échantillon i   =   x ⋅ 2⁴⁸ + y ⋅ 2³² + i
clé(i) ^ clé(j)          =   i ^ j            ← un petit nombre
```

Donc XORer une petite graine `s` dans la clé de l'échantillon `i` produit… **la clé légitime de
l'échantillon `i ^ s` du même pixel**. La graine ne déplace pas le flux, elle **renumérote les
échantillons à l'intérieur du pixel**.

Et un renumérotage ne change rien à une moyenne. À 8 échantillons par pixel, `i ^ 1` pour
`i ∈ {0…7}` parcourt `{1,0,3,2,5,4,7,6}` — le même ensemble. Les huit chemins sont les mêmes, la
moyenne est la même, **l'image est la même**. Mesuré sur le code, avant correction :

| `samples_ppx` | graine | flux partagés avec la graine 0 |
|---|---|---|
| 8 | 1 | **8 / 8** — ensemble identique |
| 8 | 3 | **8 / 8** — ensemble identique |
| 8 | 7 | **8 / 8** — ensemble identique |
| 4 | 1 | **4 / 4** — ensemble identique |
| 5 | 1 | 4 / 5 |
| 5 | 7 | 2 / 5 |

C'est le premier usage de la §5 qui tombe : on cherche à savoir si une tache est du bruit, on tape
`--seed 1`, la tache ne bouge pas, et on conclut « c'est un bug » — à tort. Le bouton existe et ne
fait rien.

**Et aucun mélange en aval ne peut le rattraper.** `seed_from_u64` reçoit deux fois *la même
valeur* dans les deux exécutions ; une fonction ne peut pas distinguer des entrées égales. La
collision se produit dans l'arithmétique de la clé, en amont de tout mélange.

Le correctif est de faire sortir la graine de l'ensemble de ces différences **avant** le XOR, en
l'étalant sur les 64 bits — `mix_seed`, le finaliseur 64 bits de MurmurHash3. `mix_seed(1)` est un
grand nombre pseudo-aléatoire ; pour qu'il retombe sur une clé utilisée, il faudrait qu'il vaille
exactement la différence de deux clés utilisées, ce qui pour 2,4 millions de clés sur 2⁶⁴ n'arrive
essentiellement jamais. `mix_seed(0) = 0`, donc la graine par défaut laisse `[1]` intact.

Un test le garde : `test_a_small_seed_is_not_a_small_change` exige zéro flux partagé entre la
graine 0 et les graines 1 à 8.

### Pourquoi XOR, et le partage des rôles

Pour une graine fixée, l'application `k ↦ mix_seed(s) ^ k` est une **bijection** — elle est même sa
propre inverse. Des clés distinctes restent donc distinctes : [P1] survit intacte, et l'ensemble
des flux est permuté en bloc. N'importe quelle bijection ferait l'affaire, `wrapping_add` tout
autant ; XOR est sans retenue et conventionnel, il n'y a pas de raison plus profonde.

Une objection vient naturellement ici : *`seed = 0` et `seed = 1` donnent deux clés qui ne
diffèrent que d'un bit — les deux générateurs ne vont-ils pas se ressembler ?*

Non, et surtout : **ce n'est pas au XOR de s'en occuper.** Trois pièces distinctes portent trois
garanties distinctes, et les confondre est la source de toutes les erreurs sur ce mécanisme —
l'amorçage par `seed ^ clé` de la sous-section précédente en est une, qui attendait de `seed`
qu'il assure à lui seul la séparation.

| pièce | ce qu'elle garantit |
|---|---|
| la clé, [1] | la **distinction** — deux échantillons ne partagent jamais un flux |
| `mix_seed` | la **séparation** — deux graines donnent deux rendus réellement différents |
| `seed_from_u64` | l'**indépendance** — des clés voisines donnent des générateurs sans lien |

La garantie d'indépendance est celle que `rand_core` annonce explicitement : `seed_from_u64` est
conçue pour que des valeurs de faible poids de Hamming, comme 0 et 1, produisent quand même de
bonnes graines indépendantes. `rand` traite au surplus tout changement de son implémentation comme une
rupture de valeur, ce qui en fait un point d'appui stable et non un détail interne.

## 6. La portée de la promesse

Deux choses la bornent, et toutes deux se disent.

**Le générateur est nommé.** `Pcg64Mcg` plutôt que le `SmallRng` de `rand`, parce que ce dernier
est documenté comme libre de changer d'algorithme entre deux versions : un `cargo update`
réécrirait toutes les images en silence, sans une erreur de compilation. La reproductibilité est un
contrat, et un contrat demande une partie qui ne puisse pas s'en aller. Sa sortie sur 64 bits rend
au surplus un `f64` par tirage, là où un générateur 32 bits en demande deux.

**Ce qui est promis est une sortie identique au bit pour le même binaire sur la même machine.** Pas
entre machines, ni entre profils de compilation : `sqrt` est exact sous IEEE-754, mais `sin`, `cos`
et `powf` viennent de la libm de la plate-forme, et la contraction en FMA est un choix de backend.
C'est assez pour comparer deux commits, et c'est à cela que la promesse sert.
