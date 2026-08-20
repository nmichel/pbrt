# `AreaLight` — une surface émissive qui éclaire

Indexé depuis [IDEAS.md](../IDEAS.md). Non commencé. **C'est le plus grand écart au modèle physique
du projet**, et le chantier désigné comme suivant.

## 1. Le défaut

`DiffuseLight` ([materials/diffuse_light.rs](../src/materials/diffuse_light.rs)) n'est qu'un
*matériau* : il répond à `emit`, et rien de plus. Aucun `AreaLight` n'est jamais enregistré dans
`Scene::lights`. Une surface émissive n'est donc **pas une source de lumière** pour le renderer, et
trois chemins indépendants mènent au noir :

- `PathIntegrator::sample_light` ([path.rs:31](../src/integrators/path.rs#L31)) tire une lumière
  uniformément dans `Scene::lights` ; le panneau n'y est pas, NEE ne peut pas l'échantillonner.
- L'émission n'est accumulée que si `is_last_bounce_specular`
  ([path.rs:58](../src/integrators/path.rs#L58)), donc uniquement en vue directe ou après un rebond
  spéculaire.
- `background_radiance` somme `le` sur les lumières `Infinite` ; une lumière d'aire n'en est pas.

**Effet net : une surface émissive ne contribue à aucun éclairage indirect.** Elle est visible, elle
n'éclaire rien. C'est pourquoi [loader.rs](../src/loader.rs) câble en dur une `PointLight` et une
lumière de fond pour que les scènes soient éclairées du tout — béquille que ce chantier retire.

Et cela bloque la suite : MIS demande un `Light::pdf_li` à pondérer contre l'échantillonnage de BSDF,
donc MIS attend ce fichier.

## 2. La couture qui manque

Le trait `Light` demande, en tout et pour tout
([lights.rs](../src/lights.rs)) :

```rust
fn sample_li(&self, intersection: &Intersection) -> Option<(LightLiSample, VisibilityTester)>;
```

soit une direction `wi`, la radiance reçue, une densité, et de quoi tester l'occultation. Tout est
là : `Intersection` porte le point `p` et la normale `n` du point ombré, et
`VisibilityTester::between` sait déjà borner la recherche à la distance de la lumière.

Ce qui manque est en amont : **une forme ne sait pas s'échantillonner**. `Shape` est
`Intersectable + AABound` — la géométrie répond « où le rayon te touche » et « quelle est ta boîte »,
jamais « donne-moi un point de ta surface, et avec quelle densité ». Il faut donc un troisième trait.

**Pourquoi un trait à part et non une méthode de `Shape` :** `Plane` a une aire infinie et
`Cylinder` un bord dont l'échantillonnage n'a rien d'évident. Si `Shape` l'exigeait, chaque forme
devrait fournir une implémentation, y compris celles qui n'en ont pas. Un trait séparé fait de
« être échantillonnable par aire » une propriété qu'une forme a ou n'a pas, ce qu'elle est.

Forme proposée, à placer dans `src/geom/` ou `src/shapes.rs` selon où le concept se lit le mieux :

```rust
pub struct ShapeSample {
    pub p: Vector3f,   // un point de la surface
    pub n: Vector3f,   // la normale en ce point
    pub pdf: f64,      // densité en mesure d'aire, soit 1/aire pour un tirage uniforme
}

pub trait AreaSampleable {
    fn area(&self) -> f64;
    fn sample_area(&self) -> ShapeSample;
}
```

## 3. Le changement de mesure, qui est le cœur du sujet

C'est le point où une erreur est invisible à l'œil et fausse l'image d'un facteur constant, donc
c'est là que la dérivation doit être écrite dans le code (CLAUDE.md §4). Référence : PBR Book 4e,
*Sampling Shapes* / *Area Lights*.

L'intégrateur travaille en **angle solide** : il divise par `sample_li.pdf` une contribution où
`wi` est une direction. La forme échantillonne en **aire**. Le pont est le jacobien entre les deux
mesures. Avec `p` le point ombré, `pₗ` le point tiré sur la source, `nₗ` sa normale,
`d = ‖pₗ − p‖` et `θₗ` l'angle entre `nₗ` et `−wi` :

```
[1]  dω = dA · |cos θₗ| / d²          élément d'angle solide sous-tendu par dA
[2]  p(ω) = p(A) · dA/dω = p(A) · d² / |cos θₗ|
```

Trois conséquences à ne pas manquer :

- **`|cos θₗ| → 0` fait exploser la densité.** Un point ombré presque dans le plan de la source y
  reçoit une contribution divisée par une densité énorme, donc quasi nulle : correct, mais la
  variance est là. Un `pdf` nul doit être traité comme « pas d'échantillon », pas divisé.
- **`d²` est la loi en carré inverse**, et elle sort du changement de mesure, pas d'un facteur
  ajouté à la main. Si on l'écrit deux fois, l'image est trop sombre d'un facteur `d²`.
- **L'émission est unilatérale** ou non, et c'est un choix à énoncer : si la source n'émet que du
  côté de `nₗ`, `sample_li` rend une radiance nulle quand `dot(nₗ, −wi) < 0`. `DiffuseLight` doit
  dire lequel des deux il est.

**L'écart assumé du départ.** Échantillonner uniformément l'aire est correct mais bruyant quand la
source sous-tend un petit angle solide vue du point ombré : la moitié des échantillons peut tomber
sur une face invisible, ou sous un angle rasant. Échantillonner directement l'angle solide (pbrt le
fait depuis un point de référence) est l'étape suivante, pas la première. À documenter comme
départure, avec sa conséquence : de la variance, pas un biais.

## 4. Qui construit l'`AreaLight`

Une `AreaLight` est une forme *plus* une radiance émise, et il faut qu'elle apparaisse à la fois dans
`Scene::lights` (pour NEE) et dans la scène comme objet visible (pour être vue). Deux routes :

- **`Scene::commit` parcourt les primitives** et enregistre une lumière pour chaque objet à matériau
  émissif. Mais `Object` marie forme et matériau et n'expose ni l'une ni l'autre ; il faudrait lui
  ajouter de quoi rendre sa forme, ce qui perce une couture que CLAUDE.md §2 tient fermée.
- **Le visiteur de chargement la construit**, ce qui est *recommandé* : `SceneBuilderVisitor` a la
  forme et le matériau en main au moment où il les assemble, donc il peut créer l'objet et la lumière
  qui partagent la même `Arc<dyn Shape>` sans qu'aucun trait ne s'élargisse. C'est aussi là
  qu'atterrit la production `light` de la grammaire `.stage`, donc les deux travaux se rencontrent.

## 5. Le double comptage, et pourquoi ne pas toucher au garde spéculaire

Dès que NEE peut échantillonner le panneau, un chemin qui l'atteint **par échantillonnage de BSDF**
et qui y ajouterait `material.emit` compterait la même contribution deux fois. Le garde
`is_last_bounce_specular` de [path.rs:58](../src/integrators/path.rs#L58) fait déjà exactement ce
qu'il faut : après un rebond diffus, NEE a servi, l'émission ne doit pas être ajoutée ; après un
rebond spéculaire, NEE n'a pas pu servir, elle doit l'être.

**Donc ce chantier ne touche pas à ce garde.** Il tombe avec MIS, et pas avant — c'est MIS qui permet
de prendre les *deux* estimateurs et de les pondérer au lieu d'en choisir un. L'estimateur
intermédiaire est correct et bruyant sur les grandes sources vues sous un angle rasant, ce qui est
précisément le cas que MIS répare.

## 6. Comment savoir que c'est juste

- **Un test de conservation d'énergie** sur `sample_area`, dans la lignée de
  [pdfs/cosine.rs](../src/pdfs/cosine.rs) et [pdfs/hemisphere.rs](../src/pdfs/hemisphere.rs) :
  l'estimateur Monte-Carlo de l'aire par `Σ 1/pdf / N` doit converger vers `area()`. C'est ce qui
  attrape un `pdf` faux d'un facteur constant, ce que l'œil ne voit pas.
- **Un test du changement de mesure** : pour une source plane vue de face à distance `d`, la densité
  en angle solide rendue doit valoir `d²/aire`, calculable à la main.
- **La comparaison `naive` / `path`**, qui est le meilleur test disponible et ne demande pas de
  sampler graine. Sur une scène éclairée par le seul panneau émissif, `NaiveIntegrator` accumule
  l'émission à chaque touche sans NEE, `PathIntegrator` passe par NEE : **les deux doivent converger
  vers la même image**. Un facteur `d²` en trop, un cosinus manquant ou une mesure non convertie
  changent la luminosité sans changer la forme de l'image — donc seule une comparaison à un autre
  estimateur les attrape.
- **Le témoin visuel** est `test_files/cornell_box.stage`, aujourd'hui éclairé par les lumières
  câblées dans le loader. Le rendre éclairé par son seul panneau *est* la démonstration du chantier.

## 7. Ordre d'attaque

Les deux premières lignes ne parlent pas d'`AreaLight` mais sont ce qui rend la suite mesurable.

- [ ] Cacher boîte + centroïde de chaque primitive à `Scene::commit`, construire le BVH sur ce cache
      — indépendant, ~20 lignes, et prérequis de tout ce qui relit des bornes.
- [ ] `rand 0.8`, RNG graine, graine dans `Config` — deux rendus redeviennent comparables ; débloque
      aussi le balayage de [cout_traversee_bvh.md](cout_traversee_bvh.md).
- [ ] Sampler stratifié en remplacement du `random_double` nu.
- [ ] `AreaSampleable` + `ShapeSample`, implémentés sur `Rectangle` d'abord — c'est le panneau du
      Cornell box, et son échantillonnage uniforme est deux nombres.
- [ ] Test de conservation d'aire sur cette implémentation, avant tout usage.
- [ ] `lights/area_light.rs` : `sample_li` par `sample_area`, conversion aire → angle solide dérivée
      dans le doc-comment, radiance nulle du mauvais côté si l'émission est unilatérale.
- [ ] Enregistrement par `SceneBuilderVisitor` : l'objet et la lumière partagent la même forme.
- [ ] Ne **pas** toucher au garde `is_last_bounce_specular`.
- [ ] Retirer les lumières câblées de `Loader::load_scene` ; témoin `cornell_box.stage`.
- [ ] Comparer `naive` et `path` sur ce témoin, et le dire dans le commit.
- [ ] Étendre à `Sphere` et `Triangle`, puis au maillage — tirage d'un triangle proportionnel à son
      aire, ce qui demande une somme cumulée des aires construite une fois.
- [ ] Échantillonnage en angle solide depuis le point de référence, en remplacement du tirage
      uniforme par aire, une fois la variance mesurée sur le témoin.

Ensuite seulement, et dans leurs propres entrées d'`IDEAS.md` : `Light::pdf_li` puis MIS, qui fait
tomber le garde spéculaire, puis la roulette russe.
