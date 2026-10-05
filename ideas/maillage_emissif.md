# Un maillage qui éclaire

Indexé depuis [IDEAS.md](../IDEAS.md). Non commencé. Reporté à la clôture du chantier `AreaLight`.

## 1. Ce qui manque

[`TriangleMesh`](../src/shapes/triangle_mesh/triangle_mesh.rs) n'implémente pas `AreaSampleable`,
donc [`Shape::area_sampler`](../src/shapes.rs) rend `None` pour lui, donc une lampe triangulaire est
**refusée au chargement** par le diagnostic du visiteur — en nommant la forme :

```
an emissive material sits on a `mesh`, which cannot hand out points of itself: it would be seen and
would light nothing
```

[test_files/triangle_source.stage](../test_files/triangle_source.stage) est écrite et ne charge pas
pour cette raison. C'est le seul des écarts de
[docs/sources_de_lumiere.md](../docs/sources_de_lumiere.md) §14 qui soit un refus franc plutôt
qu'une approximation.

Rappel qui a surpris une fois : **le langage n'a pas de forme `triangle`**. Ses formes sont `sphere`,
`rectangle`, `plane`, `cylinder`, `aabox`, `mesh` et les opérations CSG ; un triangle n'existe qu'à
l'intérieur d'un maillage. Ajouter une production traverserait lexer → parseur → AST → les deux
visiteurs → le support VS Code, et c'est un sujet à soi seul.

## 2. Deux questions, et les confondre est la façon de se tromper

**Tirer *dans* un triangle.** Une carte du carré unité vers les coordonnées barycentriques, dont le
déterminant jacobien est constant — c'est ce que la racine carrée de `(1 − √u₁, u₂√u₁)` achète. La
dérivation complète, matrice et déterminant compris, est en
[docs/sources_de_lumiere.md](../docs/sources_de_lumiere.md) §10.2.

**Choisir *lequel*, proportionnellement à son aire.** C'est une inversion de densité discrète : une
somme cumulée des aires construite une fois, puis une dichotomie. Le facteur propre au triangle
s'annule exactement contre sa probabilité d'être choisi, et la densité résultante est `1/A_total` —
uniforme sur tout le maillage, ce qui est le but.

Un maillage uniforme ne distingue pas un choix correct d'un choix uniforme : **les deux questions ne
se testent pas sur la même géométrie**, et c'est le §4.

## 3. Ce que cela introduit, et qui n'existe pas encore ici

- **Un état construit une fois.** La somme cumulée est la première structure qu'une forme de ce
  projet aurait à bâtir pour être échantillonnée. Où elle vit — dans le maillage, dans un type à
  part rendu par `area_sampler` — est l'arbitrage à trancher.
- **Un coût de construction à mesurer.** Sur un maillage réel, pas sur un cube. La leçon de
  [docs/mesures_bvh.md](../docs/mesures_bvh.md) §2.3 vaut ici : un gain ou un coût de construction
  se rapporte à une exécution entière, jamais au chargement seul.
- **Une question de normales.** Un maillage porte des normales par sommet ; un point tiré doit
  rendre celle que `intersect` rapporterait au même endroit, faute de quoi l'émission unilatérale
  tranche différemment selon le chemin par lequel on arrive. C'est la même exigence que les (u, v)
  du rectangle, sur une grandeur qui décide d'un `if`.

## 4. Les témoins : un existe, l'autre est à écrire

[test_files/lamp_triangle.ply](../test_files/lamp_triangle.ply) tient **un seul** triangle, écrit à
la main, dont l'enroulement oriente la normale vers `−Y`. Il mesure le tirage *dans* un triangle, et
rien d'autre.

**Le second témoin manque** : il faut un maillage à triangles d'aires nettement inégales, sans quoi
rien ne distingue un choix proportionnel d'un choix uniforme. Deux triangles dans un rapport d'aires
de dix suffiraient, et la vérification est un comptage — chaque triangle doit recevoir une part des
tirages égale à sa part d'aire.

L'absence de biais se lira comme pour les autres formes : `naive` et `path` doivent converger vers
la même image, sur une scène calquée sur `direct_lighting.stage`.
