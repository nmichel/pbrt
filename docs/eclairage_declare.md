# L'éclairage est déclaré par la scène

Ce que le langage `.stage` dit d'une source de lumière, et ce que cela rend vérifiable. Décrit le
code tel qu'il est ; ce qui reste à faire est dans [IDEAS.md](../IDEAS.md) et
[ideas/](../ideas/).

## 1. Deux façons de déclarer une source, et elles ne se recouvrent pas

| | **`light`** | **matériau `diffuse_light`** |
|---|---|---|
| S'écrit | directement sous `scene` | sur un `object`, à la place du matériau |
| A une géométrie | non | oui — celle de la forme qui le porte |
| Un rayon peut la toucher | jamais | oui, et il la voit |
| Entre dans `Scene::lights` | à la construction, par `SceneBuilderVisitor` | pas encore ([ideas/area_light.md](../ideas/area_light.md)) |

La frontière est celle de la géométrie, et c'est la seule qui découpe proprement. Une source
ponctuelle et un ciel n'ont pas de surface : rien dans la scène ne peut être touché et reconnu comme
étant l'un d'eux, donc ils ne peuvent exister que déclarés. Une surface émissive, elle, *est* déjà un
objet ; lui donner en plus une syntaxe `light` reviendrait à écrire deux fois la même chose et à
laisser les deux écritures diverger.

Les trois productions `light` couvrent donc exactement les trois implémentations de
[`Light`](../src/lights.rs) sans géométrie :

```
light point                    light uniform_infinite         light background_infinite
  color 15.0 15.0 15.0           color 0.5 0.5 0.5              color 1.0 1.0 1.0
  transform {                                                   color 0.5 0.7 1.0
    translate 0.0 2.0 1.0
  }
```

Le bloc `transform` de `point` est obligatoire, celui des deux autres n'existe pas : une source à
l'infini offre une direction et n'a pas de position. C'est la même distinction que portent les deux
constructeurs de `VisibilityTester`, `between` et `towards_infinity`.

## 2. L'ordre des lumières est une coordonnée

[`SceneBuilderVisitor::visit_scene`](../src/loader/visitors/scene_builder.rs) verse les lumières dans
la scène **dans l'ordre de déclaration**, et les objets dans l'ordre inverse. L'asymétrie est
voulue : [`PathIntegrator::sample_light`](../src/integrators/path.rs) choisit une source en tirant un
nombre et en le mettant à l'échelle du compte, donc permuter la liste donne au même tirage une source
différente — même scène, même graine, autre image. L'index d'un objet, lui, n'est lu par personne :
l'accélérateur trie les primitives lui-même.

C'est ce qui rend une migration de scène vérifiable au bit près plutôt qu'à l'œil.

## 3. La radiance de fond appartient à la scène

[`Integrator::background_radiance`](../src/integrators.rs) somme les `le` des lumières
`LightType::Infinite` de la scène, et c'est l'implémentation **par défaut** du trait : ni `path` ni
`naive` ne la redéfinissent. Sur un ensemble vide la somme est noire, ce qui est la réponse honnête —
une scène qui ne déclare pas de ciel n'en a pas.

Seul [`NormalIntegrator`](../src/integrators/normal.rs) la redéfinit, à noir : il dessine des
normales et non de la lumière, et le bleu d'un `background_infinite` s'y lirait comme une normale +z.

## 4. Ce que cela rend vérifiable

`NaiveIntegrator` ne consulte jamais `Scene::lights` : toute sa lumière vient de `material.emit`
rencontré par échantillonnage de BSDF. `PathIntegrator` passe par NEE. Les deux estimateurs doivent
donc converger vers la même image sur une scène dont l'éclairage est atteignable par un rayon — et
ils ne le peuvent que si aucune source invisible n'est ajoutée dans leur dos.

Mesuré sur une sphère lambertienne posée sur un plan, éclairées par un seul `background_infinite`,
100 × 75, 1024 chemins par pixel, `--max_depth 8`, `--seed 1`, moyenne des canaux sur l'image
finie (0–255) :

| | R | V | B |
|---|---|---|---|
| `--integrator naive` | 192,29 | 186,16 | 208,40 |
| `--integrator path` | 192,31 | 186,19 | 208,45 |

Cinq centièmes d'un niveau d'écart au pire, sur une image où l'éclairage indirect compte. **C'est
cette comparaison qui est la preuve d'`AreaLight`** ([ideas/area_light.md](../ideas/area_light.md)
§6) : un `d²` en trop, un cosinus manquant ou une mesure non convertie changent la luminosité sans
changer la forme de l'image, et seul un second estimateur les attrape. Elle était impossible tant
qu'une `PointLight` était ajoutée à toute scène, `path` la voyant par NEE et `naive` n'ayant aucune
chance de la toucher.

## 5. Une scène sans lumière

[`Loader::load_scene`](../src/loader.rs) écrit un avertissement sur stderr plutôt que de refuser le
fichier : une description a le droit de ne rien éclairer, et une surface émissive que la caméra vise
directement reste visible. Le reste est noir, et la cause est une ligne absente d'un fichier, pas le
renderer.
