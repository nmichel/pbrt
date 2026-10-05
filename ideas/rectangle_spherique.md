# Échantillonner un rectangle par l'angle solide qu'il sous-tend

Indexé depuis [IDEAS.md](../IDEAS.md). Non commencé. Reporté à la clôture du chantier `AreaLight`,
qui a laissé la couture en place et la mesure faite.

## 1. Ce qui manque, exactement

[`Rectangle`](../src/shapes/rectangle.rs) ne redéfinit pas `sample_solid_angle` : il tire un point
uniformément sur son aire, puis le corps par défaut du trait convertit la densité par le jacobien
`p(ω) = p(A)·d²/|cos θₗ|`. [`Sphere`](../src/shapes/sphere.rs) ne passe plus par là — elle tire dans
le cône qu'elle occupe — et le rectangle est désormais la seule forme restée sur l'ancienne voie.

**Il n'y a qu'une méthode à écrire.** Le trait porte déjà les deux variantes, `AreaLight` appelle
déjà la bonne, et `Transformed` porte déjà le point de référence vers l'espace local et ramène le
résultat. Rien d'autre n'est à toucher.

## 2. Ce n'est pas une question d'exactitude

**L'estimateur actuel est non biaisé** : le changement de mesure est exact, et tout tirage dont la
densité couvre le domaine converge vers la bonne valeur. Ce chantier ne corrigerait aucune image
convergée ; il ferait arriver plus vite au même endroit.

Une seule nuance, mince : quand `cos θₗ` s'effondre, `d²/|cos θₗ|` déborde et
[`solid_angle_from_area`](../src/shapes.rs) jette ces échantillons. Jeter une portion du domaine est
un biais — négligeable en pratique, et qu'un tirage en angle solide supprime, sa densité étant
constante et ne débordant jamais.

## 3. Ce qui le motive quand même

- **Le rectangle éclaire presque tout le dépôt.** C'est la forme émissive de huit des neuf scènes,
  Cornell box comprise. La sphère qu'on vient d'accélérer n'éclaire qu'un témoin écrit pour elle ;
  la forme restée sur le tirage lent est celle qui porte l'éclairage réel du projet.
- **Son mauvais régime est celui des intérieurs.** Source large, proche, vue de biais — c'est un mur
  près du plafond d'une Cornell box, et c'est ce que `grazing_source.stage` chiffre au §4.
- **C'est une mathématique que le projet n'a nulle part ailleurs** : excès sphérique, aire d'un
  quadrilatère sur la sphère, inversion d'une densité en deux temps. Pour un renderer
  d'apprentissage, c'est une raison suffisante en soi, plus que le gain de bruit.

**Le contrepoids, et il est honnête : MIS est un plus gros levier.** Il fait tomber le garde
`is_last_bounce_specular`, emporte le cosinus de [cosinus_dirac.md](cosinus_dirac.md), et améliore
*toutes* les configurations au lieu d'une forme. Si l'ordre devait se décider sur le rendement seul,
MIS passe devant.

## 4. Le témoin existe, et sa ligne de base est mesurée

[test_files/grazing_source.stage](../test_files/grazing_source.stage) : une bande de 20 sur 0,6
rasant le sol à 0,35 de haut, où le poids d'un point tiré varie autant qu'il est possible — `d²`
d'un facteur deux cents entre les deux bouts, `cos θₗ` de 0,3 à 0,02.

Écart quadratique moyen contre un rendu convergé de la même scène, qui est la colonne à faire
baisser :

| chemins/pixel | écart quadratique |
|---|---|
| 64 | 7,42 |
| 256 | 3,72 |
| 1024 | 1,79 |

Protocole et lecture dans [docs/sources_de_lumiere.md](../docs/sources_de_lumiere.md) §13.2 ;
`image_stats` donne le chiffre.

**Un second témoin tient le régime singulier** : dans
[test_files/floor_strip.stage](../test_files/floor_strip.stage), un panneau debout pose son arête
basse à 0,001 du sol, et un point du sol à son pied voit `d → 0` dans `cos θ · cos θₗ / d²`. Les
pixels isolés très lumineux qu'il produit n'apparaissent qu'à partir de `--max_depth 3` — un
rebond qui retombe au pied du panneau — et `path` et `naive` y restent d'accord à 0,4 % près
(15,0 en 128 × 128, 8192 et 16384 chemins par pixel) : ce n'est pas un biais, c'est ce bruit-là
que le tirage en angle solide borne.

**`grazing_source.stage` ne dit rien du biais** : une bande mince sous-tend trop peu d'angle
solide pour que `naive` la trouve. Pour l'absence de biais, c'est `direct_lighting.stage` qu'il faut, où les deux
estimateurs se rejoignent au centième.

## 5. La mathématique

Entièrement posée dans [docs/sources_de_lumiere.md](../docs/sources_de_lumiere.md) §10.1 : l'angle
solide d'un quadrilatère sphérique par son excès, `Ω = α + β + γ + δ − 2π`, et le tirage comme une
inversion de densité en deux temps — dont aucune des deux ne se fait de tête, à la différence du
cône de la sphère.

**La référence à suivre**, et c'est elle qu'il faudra ouvrir plutôt que de reconstruire la méthode :

> C. Ureña, M. Fajardo, A. King, *An Area-Preserving Parametrization for Spherical Rectangles*,
> Computer Graphics Forum 32(4), actes du Eurographics Symposium on Rendering 2013, p. 59-66.
> DOI [10.1111/cgf.12151](https://doi.org/10.1111/cgf.12151) — article et transparents sur [la page
> de Carlos Ureña](https://www.ugr.es/~curena/publ/2013-egsr/).

L'article donne une fonction analytique du carré unité vers le rectangle sphérique qui **préserve
les aires** — c'est exactement la propriété recherchée, puisqu'elle rend la densité constante et
égale à `1/Ω`.

**Une implémentation de référence à lire à côté** : pbrt-v4 en fait sa fonction
`SampleSphericalRectangle`, appelée par `BilinearPatch::Sample` avec le point de référence lorsque
le quadrilatère est effectivement un rectangle ([PBR Book, 4ᵉ éd., §6.6 — *Bilinear
Patches*](https://pbr-book.org/4ed/Shapes/Bilinear_Patches)). Ce projet n'a pas de *bilinear patch*
et la méthode atterrira donc directement sur `Rectangle`, qui est plus simple : ses quatre sommets
sont coplanaires par construction, là où pbrt doit d'abord vérifier qu'ils le sont.

## 6. Les deux pièges que la sphère a déjà révélés

Ils ne sont pas propres à la sphère, et ils attendront celui qui écrira cette méthode-ci.

- **La direction se prend dans le tirage, jamais dans le point.** Reprendre `ω` en normalisant
  `pₗ − p` est un `0/0` dès qu'un chemin fait du NEE depuis un sommet posé sur la lampe. Un
  échantillon sur deux cents revenait `NaN`, et la moitié de l'image était noire à 512 chemins.
- **Un signe d'orientation ne se vérifie pas sur une image.** Le point tiré à l'azimut `φ+π` de sa
  propre direction laissait l'image d'une source uniforme rigoureusement inchangée. Seul le test
  qui tire un rayon vers le point tiré et compare la touche l'a vu — et une source *texturée*
  aurait éclairé depuis le mauvais côté.
