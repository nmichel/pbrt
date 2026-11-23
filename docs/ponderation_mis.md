# La pondération de NEE et du tirage de BSDF

[`PathIntegrator`](../src/integrators/path.rs) atteint chaque source par deux routes : le tirage de
la source (*next event estimation*, NEE) et un rayon de BSDF qui la rencontre. Il garde les deux et
les pondère par l'heuristique de puissance de Veach, de sorte que les deux poids d'une même
direction somment à 1. La dérivation — l'estimateur à deux stratégies, la condition de non-biais,
l'heuristique et le traitement des δ comme sa limite — est dans l'en-tête de `path.rs`, et n'est pas
répétée ici. Ce document dit ce que cette pondération change à l'image, et le défaut qu'elle a levé.

Les références : [PBR Book, 4ᵉ éd., §2.2.3 — *Multiple Importance
Sampling*](https://pbr-book.org/4ed/Monte_Carlo_Integration/Improving_Efficiency#MultipleImportanceSampling)
et [§13.4 — *A Better Path
Tracer*](https://pbr-book.org/4ed/Light_Transport_I_Surface_Reflection/A_Better_Path_Tracer) ;
E. Veach, *Robust Monte Carlo Methods for Light Transport Simulation*, thèse, Stanford, 1997,
chapitre 9.

## 1. Ce que chaque participant fournit

| Grandeur de l'en-tête | Qui la donne |
|---|---|
| `p_f` d'un rayon tiré | `ScatterInfo::density` |
| `p_f` d'une direction tirée par une source | [`Material::pdf`](../src/materials.rs) |
| `p_l` d'une direction tirée par la source | `LightLiSample::density` |
| `p_l` d'une direction tirée par la BSDF | [`Light::pdf_li`](../src/lights.rs) |
| la source touchée par un rayon de BSDF | [`Interaction::area_light`](../src/interaction.rs) |
| `P(l)`, la chance de choisir une source | `PathIntegrator::light_selection_probability`, `1/N` |

Les quatre densités sont **par unité d'angle solide au sommet d'où part la direction**. Le contrat
de `pdf_li` et de `Material::pdf` — rendre la densité du tirage pour toute direction qu'il produit,
0 pour les autres — est vérifié séparément par les tests de chaque implémentation ; un désaccord ne
bruite pas, il biaise, les deux poids cessant de sommer à 1.

## 2. Le défaut que la pondération a levé : d'où l'on interroge la source

**Le rayon de BSDF ne part pas du point ombré.** Un matériau décale l'origine de son rayon le long
de la normale avant de l'envoyer — `0.001` pour [`Lambertian`](../src/materials/lambertian.rs) —, ce
qui évite que le rayon retrouve la surface dont il part. Or `pdf_li` retrouve le point qu'une
direction atteint en **intersectant de nouveau la source** : interrogé depuis le point ombré, il
retrace un rayon qui n'est pas celui qu'on a suivi.

Pour une direction rasante, la différence n'est pas négligeable. Le point visé se déplace de
`décalage / tan(élévation)` sur la source ; à 6° d'élévation, cela fait environ un centième. Une
direction qui a touché la source près de son bord lointain la *manque* alors à la seconde
intersection. `pdf_li` répond 0, l'heuristique donne tout le poids à la BSDF, et l'émission compte
en entier — alors que NEE la comptait déjà.

Sur `test_files/grazing_source.stage`, dont la bande rayonne 600, un chemin sur cent mille
apportait ainsi ρ·L = 360 à un point du sol dont la radiance vaut 0,48. Mesuré en
linéaire, 200 000 chemins depuis `(0, 0, 2)` :

| | variance | plus grand chemin |
|---|---|---|
| NEE seul | 0,43 | 3,06 |
| MIS, source interrogée depuis le point ombré | 1,73 | 362,1 |
| MIS, source interrogée depuis l'origine du rayon | 0,43 | 3,06 |

La correction est dans `BsdfDraw::from`, qui retient l'origine du rayon tiré. Un test
(`test_a_bsdf_ray_reaching_a_thin_source_takes_almost_none_of_it`) monte la géométrie où le défaut
est le plus probable — une bande de 2 mm vue à 45° — et échoue sans la correction.

**Ce qu'aucune image n'aurait montré clairement** : sur ce témoin, le défaut se lisait comme « MIS
est plus bruité que NEE seul » (écart quadratique 6,39 contre 3,80 à 1024 chemins, contre une référence que le défaut
atteignait elle aussi), ce qu'un
lecteur pressé aurait pris pour une propriété de l'heuristique. Il fallait descendre au point et
comparer la variance des deux estimateurs : celle de MIS ne doit dépasser celle d'aucune stratégie
seule que d'un terme borné, et un chemin qui porte ρ·L entier là où NEE domine est impossible.

## 3. Ce que l'image y gagne

Protocole du [§13 de sources_de_lumiere.md](sources_de_lumiere.md) : 256 × 192, `--seed 0`, la
ligne de commande de chaque témoin, canal rouge lu par [`image_stats`](../src/bin/image_stats.rs).
L'écart quadratique est mesuré contre un rendu de 8192 chemins de l'intégrateur pondéré ;
« avant » est le `PathIntegrator` qui élisait une seule route par le type du dernier rebond.

| Témoin | chemins/pixel | moyenne avant | moyenne après | écart avant | écart après |
|---|---|---|---|---|---|
| `direct_lighting` | 64 | 103,51 | 103,54 | 4,02 | 3,62 |
| | 1024 | 103,59 | 103,60 | 1,04 | 0,94 |
| `floor_strip` | 64 | 11,07 | 11,29 | 6,90 | 5,20 |
| | 256 | 11,61 | 11,83 | 4,89 | 2,86 |
| | 1024 | 11,85 | 12,05 | 3,71 | 1,52 |
| `grazing_source` | 1024 | 86,08 | 86,08 | 1,79 | 1,79 |
| `sphere_source` | 1024 | 129,34 | 129,34 | 0,34 | 0,35 |
| `indirect_lighting` | 1024 | 70,41 | 70,41 | 2,86 | 2,85 |

**Aucune moyenne convergée ne bouge**, et c'est la condition : la pondération change la variance,
jamais l'espérance. Sur `cornell_box_exact.stage`, 256 × 256, `--max_depth 8`, 1024 chemins, l'image
reste à 74,83 / 61,64 / 35,56, au centième près ce qu'elle était, à côté des 74,79 / 61,60 / 35,52
de `naive` à 16384 chemins ([eclairage_declare.md](eclairage_declare.md) §4). Les moyennes de
`floor_strip` montent à nombre de chemins égal parce que les pixels saturés y disparaissent : un
pixel écrêté a perdu son énergie, et MIS n'en écrête plus aucun là où l'ancien estimateur en
écrêtait dix.

**Le gain est où la source sous-tend un grand angle solide**, et seulement là. C'est le régime où
le tirage par aire paie `d²/|cos θₗ|` et où un rayon cosinusoïdal trouve la source sans passer par
ce rapport :

- au pied du panneau de `floor_strip`, où `d → 0` : le bruit est divisé par 2,4 et les pixels
  saturés disparaissent ;
- sous le panneau de `direct_lighting`, grand et proche : −10 % ;
- **nulle part ailleurs**. Le cadrage de `grazing_source` ne montre que le sol loin de la bande,
  qu'elle sous-tend à peine : NEE y garde tout le poids, et l'image est identique au centième. Au
  pied de cette même bande, hors champ, la variance d'un point est divisée par 8 à 40 — mais aucun
  pixel ne le voit.
- `sphere_source` est le seul témoin où l'image est très légèrement plus bruitée : le tirage dans
  le cône y est déjà presque la distribution idéale, et le poids cédé à la BSDF est un poids cédé à
  une stratégie moins bonne. C'est le terme borné que Veach garantit, pas un défaut.

## 4. Ce que cela laisse au rectangle sphérique

[ideas/rectangle_spherique.md](../ideas/rectangle_spherique.md) attendait de MIS qu'il déplace la
ligne de base de `grazing_source`. **Elle n'a pas bougé** — 7,42 / 3,72 / 1,79 à 64, 256 et 1024
chemins, avant comme après —, pour la raison du §3 : ce que le témoin montre est le sol loin de la
source, où le tirage de BSDF n'a rien à offrir. Le rectangle sphérique, qui supprime le rapport
`d²/|cos θₗ|` du tirage de la source elle-même, garde donc toute sa ligne de base.
