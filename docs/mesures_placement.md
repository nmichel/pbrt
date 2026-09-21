# Placer une géométrie — les deux décorateurs, et ce que le choix coûte

Le projet a deux façons de mettre une géométrie quelque part, et elles ne font pas le même métier.
Ce document dit laquelle coûte quoi, et sur quelle mesure l'arbitrage repose.

## 0. Protocole

Chronomètre sur le **processus entier** — chargement, rendu, écriture — d'un binaire `--release`,
avec les mêmes options pour tous les points comparés :

```
--output_width 400 --output_height 300 --samples_ppx 16 --seed 7 --threads 4 --far 2000 --fov 60
```

**Minimum de 9 tirs** (5 pour le dragon, dont chaque tir coûte deux secondes). Le minimum plutôt que
la moyenne, parce que le bruit d'un système multitâche est à sens unique : il ajoute du temps, il
n'en retire pas.

**Plancher de bruit**, établi en comparant un même binaire à lui-même : le minimum est reproductible
à 1 % près (219 ms puis 219 ms sur `default`), la médiane à 3 % près. **Un écart de moins de 2 % sur
le minimum n'est donc pas un résultat.**

Machine : Apple M2 Pro, 12 cœurs, macOS (Darwin arm64). Les chiffres ne valent que les uns par
rapport aux autres.

## 1. Les deux décorateurs

| | [`shapes::Transformed`](../src/shapes/transformed.rs) | [`objects::Transformed`](../src/objects/transformed.rs) |
|---|---|---|
| Ce qu'il fait | **place** une forme : la géométrie est construite où elle appartient | **instancie** un objet bâti : une structure vue depuis plusieurs endroits |
| Où il se trouve | **sous** `Simple` — c'est une forme | **au-dessus** de `Simple` — c'est un objet |
| Ce qu'il rapatrie | la liste entière que rend `Intersectable::intersect` | la seule touche la plus proche, une `Interaction` |
| Qui l'emploie | le chargeur, à chaque feuille, sous la CTM | `examples/`, pour le placement programmatique ; le chargeur, pour l'instanciation, le jour où la grammaire saura nommer |

Les deux font le même aller-retour de rayon : monde → local à l'aller, local → monde au retour.

## 2. La mesure

Minimum sur 9 tirs, en millisecondes :

| scène | placement au-dessus de `Simple` | placement sous `Simple` | écart |
|---|---|---|---|
| `default` (2 rectangles) | 222 | **187** | −15,8 % |
| `cornell_box` (8 rectangles) | 822 | **770** | −6,3 % |
| `many_spheres` (445 sphères) | 630 | **622** | −1,3 % |
| `dragon_vrip.ply` (1 maillage, 5 tirs) | 1800 | **1763** | −2,1 % |

Descendre le placement d'un étage est donc **un gain**, et non le changement neutre qu'on attend
d'un déplacement de responsabilité. Le gain est d'autant plus net que la scène passe peu de temps
ailleurs : `many_spheres` est dominée par la traversée du BVH (22 nœuds et 38 tests de boîte par
rayon), le dragon par le parse de ses 33 Mo.

## 3. Deux effets opposés, et leur attribution

### Le gain : un clone d'`Arc` par touche

Un placement **au-dessus** de `Simple` reçoit une `Interaction` déjà construite, et ne peut pas la
modifier : il en rebâtit une avec l'intersection rapatriée, ce qui re-clone l'`Arc<dyn Material>`.
C'est un incrément atomique et son décrément, par touche, sur le chemin que tout rayon primaire
emprunte.

Un placement **sous** `Simple` n'a pas de matériau à porter : il transforme des `Intersection`, et
c'est `Simple` qui attache le matériau, une fois.

L'attribution est vérifiée et non supposée. En retirant ce seul clone du décorateur d'objet — lui
faire prendre possession de l'`Interaction` au lieu d'en reconstruire une —, le gain apparaît là
aussi, presque entier :

| scène | au-dessus | au-dessus, sans le second clone | sous `Simple` |
|---|---|---|---|
| `default` | 222 | 186 | 187 |
| `cornell_box` | 822 | 761 | 770 |
| `many_spheres` | 630 | 607 | 622 |

### Le coût : *n* transformations au lieu d'une

La troisième colonne du tableau ci-dessus dit l'autre moitié. Un décorateur de forme reçoit **toute**
la liste de touches et les rapatrie toutes, là où un décorateur d'objet n'en voit qu'une : une sphère
rend son entrée *et* sa sortie, donc deux transformations d'`Intersection` au lieu d'une.
`many_spheres` le montre à 622 contre 607, soit **+2,5 %** — et `default`, dont les rectangles ne
rendent qu'une touche, ne le montre pas du tout (187 contre 186, sous le plancher de bruit).

Le coût est donc borné par le nombre de touches qu'une forme rapporte, et il est payé sur les formes
fermées. Il reste très inférieur au gain sur les quatre scènes mesurées.

La transformation se fait **sur place**, dans le vecteur possédé que rend l'enfant. Collecter dans un
second `IntersectionResult` ajouterait une allocation par touche, ce qui est l'entrée ouverte
d'`IDEAS.md` sous *Accélérateurs* : le jour où le coût des *n* transformations pèsera, le remède ne
sera pas de remonter d'un étage mais de cesser de rendre une liste.

## 4. Ce que coûte une instance, et pourquoi ce n'est pas un placement

Replier un placement dans la géométrie ne coûte rien par rayon : la forme est simplement construite
où elle va. Instancier ne peut pas être gratuit, et les deux prix sont à connaître avant de choisir
ce mécanisme pour un objet qui n'apparaît qu'une fois :

- **un aller-retour de rayon par instance**, sur chaque rayon qui l'atteint ;
- **un sous-arbre que le BVH de scène ne fusionne pas** : l'instance est une primitive pour lui,
  quoi qu'elle contienne, donc l'arbre ne peut pas entrelacer son contenu avec la géométrie voisine.

Les deux sont le prix de ne construire une structure qu'une fois au lieu de *n*.
