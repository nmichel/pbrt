# `AreaLight` — une surface émissive qui éclaire

Indexé depuis [IDEAS.md](../IDEAS.md). Non commencé, et en tête de la liste. **C'est le plus grand
écart au modèle physique du projet**, et tous ses prérequis sont acquis (§7).

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
n'éclaire rien. C'est pourquoi toute scène de `test_files/` sauf une déclare un `light` en plus de son
panneau émissif : sans lui elle serait noire, sous `path`, partout où la caméra ne vise pas
directement le panneau. L'exception est `cornell_box_exact.stage`, qui ne déclare que son panneau et
ne se rend donc qu'avec `naive` — c'est le témoin du §6.

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

Ce qui manque est en amont, et à deux endroits.

**Une forme ne sait pas s'échantillonner.** `Shape` est `Intersectable + AABound` — la géométrie
répond « où le rayon te touche » et « quelle est ta boîte », jamais « donne-moi un point de ta
surface, et avec quelle densité ». Il faut donc un troisième trait.

**Pourquoi un trait à part et non une méthode de `Shape` :** `Plane` a une aire infinie et
`csg::Union` une aire qui n'est pas calculable. Si `Shape` l'exigeait, chaque forme devrait fournir
une implémentation, y compris celles qui n'en ont pas. Un trait séparé fait de « être
échantillonnable par aire » une propriété qu'une forme a ou n'a pas, ce qu'elle est.

À placer dans `src/shapes.rs`, à côté de `Shape` : c'est une face de la géométrie, pas de l'objet.

```rust
/// A point drawn on a surface, with the density it was drawn with.
pub struct ShapeSample {
    pub p: Vector3f,
    pub n: Vector3f,

    /// Surface parameters at `p`, so the emitted radiance is read from the very texture the
    /// visible surface shows. Without them a textured emitter would light the scene with one
    /// colour and be seen with another.
    pub u: f64,
    pub v: f64,

    /// Density **in area measure** — 1/area for a uniform draw. The conversion to solid angle
    /// belongs to the light: it needs the shaded point, which a shape knows nothing about.
    pub pdf: f64,
}

pub trait AreaSampleable: Send + Sync {
    fn area(&self) -> f64;

    /// Draws a point, consuming `u` rather than drawing from a sampler — the contract
    /// `Pdf::generate` already holds, and the one that keeps a render reproducible.
    fn sample_area(&self, u: &Vector2f) -> ShapeSample;
}
```

**Et le visiteur ne peut pas savoir qu'une forme le sait.** Sa pile tient des `Arc<dyn Shape>` : le
type concret est perdu au moment précis où la question se pose. Rust n'a pas d'upcast de trait
objet, et `Any` suivi d'un downcast serait le bricolage à éviter. La sortie est une méthode par
défaut sur `Shape` qui rend l'autre face de la forme, avec un receveur `Arc<Self>` pour qu'elle
rende un handle possédé :

```rust
pub trait Shape: Intersectable + AABound {
    /// The area-sampling face of this shape, when it has one.
    ///
    /// Rust has no trait upcast, and the scene builder holds shapes as `Arc<dyn Shape>` by the
    /// time it needs the answer. Asking the shape keeps "can I be sampled by area" a property the
    /// shape states, rather than one the caller infers from a type it no longer has.
    fn area_sampler(self: Arc<Self>) -> Option<Arc<dyn AreaSampleable>> {
        None
    }
}

impl Shape for Rectangle {
    fn area_sampler(self: Arc<Self>) -> Option<Arc<dyn AreaSampleable>> {
        Some(self)
    }
}
```

`self: Arc<Self>` est un receveur compatible avec les traits objets, le corps par défaut passe, et
`Plane`, `csg::*` et `TriangleMesh` répondent `None` sans une ligne. C'est le prix, en Rust, d'une
propriété qu'un objet a ou n'a pas ; les deux autres routes cassent l'une l'invariant « les deux
méthodes vont ensemble » — poser `area()` et `sample_area()` sur `Shape` en `Option` —, l'autre la
lisibilité.

**Corollaire, et il n'est pas facultatif : un matériau émissif posé sur une forme qui ne sait pas
s'échantillonner est une erreur de chargement**, nommant la forme fautive. Rendre l'objet visible
mais non échantillonné reconduirait en silence le défaut même que ce chantier répare.

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

## 4. Qui construit l'`AreaLight`, et comment elle et l'objet regardent la même surface

Une `AreaLight` est une forme *plus* une radiance émise, et il faut qu'elle apparaisse à la fois dans
`Scene::lights` (pour NEE) et dans la scène comme objet visible (pour être vue). Deux routes ont été
pesées ; la seconde est retenue.

- **`Scene::commit` parcourt les primitives** et enregistre une lumière pour chaque objet à matériau
  émissif. Écartée : `Object` marie forme et matériau et n'expose ni l'une ni l'autre ; il faudrait
  lui ajouter de quoi rendre sa forme, ce qui perce une couture que CLAUDE.md §2 tient fermée.
- **Le visiteur de chargement la construit**, et c'est la route retenue : `SceneBuilderVisitor` a la
  forme et le matériau en main au moment où il les assemble, donc il crée l'objet et la lumière qui
  partagent le même `Arc` sans qu'aucun trait ne s'élargisse. C'est aussi là qu'atterrit la
  production `light` de la grammaire `.stage`, donc les deux travaux se rencontrent.

**Ce qui est partagé est la forme, et elle doit être en espace monde.** C'est le point qui décide de
tout le reste : aucun type ne porte les deux rôles, ce sont deux participants distincts qui regardent
la même géométrie.

```rust
// dans `visit_object_simple`, une fois la CTM repliée dans la forme
let shape: Arc<dyn Shape> = Arc::new(shapes::Transformed::new(local_shape, self.ctm()));

self.objects.push(Arc::new(Simple::new(Arc::clone(&shape), material)));          // visible

if let Some(sampler) = Arc::clone(&shape).area_sampler() {                       // échantillonnable
    self.scene.add_light(Arc::new(AreaLight::new(sampler, emitted, two_sided)));
}
```

**C'est ce que la CTM a rendu possible**, et ce prérequis est acquis : la forme sort du chargeur en
espace monde, [`shapes::Transformed`](../src/shapes/transformed.rs) ayant replié le placement dans la
géométrie. Le placement vivant *au-dessus* du point de partage, la lumière échantillonnerait la forme
en espace local pendant que l'objet est rendu ailleurs — erreur silencieuse dont l'ombre portée
serait le seul indice.

**Reste une décision à prendre ici, et elle n'est pas cosmétique** : `DiffuseLight` tient sa radiance
sous forme de `Texture`, et `Texture::shade` demande une `Intersection` entière quand un
`ShapeSample` n'en est pas une — `d` et `wo` n'ont aucun sens pour un point tiré. Deux issues, et il
faut choisir avant d'écrire `sample_li` : porter (u, v) dans `ShapeSample` et donner à `Texture` un
point d'entrée qui s'en contente, ou restreindre la première version à une émission uniforme et le
dire. Ce qu'il ne faut pas, c'est que NEE lise une radiance différente de celle que l'œil voit.

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
- **Ce qui n'a pas besoin d'être testé**, et c'est l'intérêt du dessin du §4 : que la lumière et
  l'objet visible soient au même endroit. Ils tiennent le même `Arc`, déjà placé ; il n'y a pas deux
  placements qui pourraient diverger, donc pas de propriété à vérifier.
- **La comparaison `naive` / `path`**, qui est le meilleur test disponible. Sur une scène éclairée
  par le seul panneau émissif, `NaiveIntegrator` accumule
  l'émission à chaque touche sans NEE, `PathIntegrator` passe par NEE : **les deux doivent converger
  vers la même image**. Un facteur `d²` en trop, un cosinus manquant ou une mesure non convertie
  changent la luminosité sans changer la forme de l'image — donc seule une comparaison à un autre
  estimateur les attrape.

  **Elle est disponible**, et vérifiée sur un cas où elle doit déjà tenir : une scène éclairée par un
  seul `background_infinite`, où les deux estimateurs s'accordent à cinq centièmes d'un niveau sur
  255 ([docs/eclairage_declare.md](../docs/eclairage_declare.md) §4). Elle ne l'était pas tant que le
  chargeur ajoutait une `PointLight` à toute scène : `NaiveIntegrator` ne consulte jamais
  `Scene::lights`, et une source ponctuelle a une probabilité nulle d'être touchée par un rayon.

  **Ce qu'elle exige de la scène de test**, en revanche, est que son éclairage soit entièrement
  atteignable par un rayon — donc un panneau émissif et rien d'autre.
- **Le témoin visuel existe** : `test_files/cornell_box_exact.stage`, la Cornell box de la donnée
  mesurée — géométrie, caméra et panneau publiés. Elle ne déclare aucun `light`, donc son éclairage
  est entièrement atteignable par un rayon, ce que le point précédent exige ; son en-tête porte la
  ligne de commande, qui n'est pas celle par défaut.

  Sous `naive` elle donne l'image, et son niveau absolu est recoupé avec l'éclairement direct calculé
  à la main sur quatre surfaces, l'écart restant allant dans le sens de l'interréflexion. Sous `path`
  elle donne une pièce noire où seul le panneau se voit : **c'est le défaut du §1, à l'œil nu.** Les
  deux rendus qui se rejoignent sont la recette d'acceptation de ce chantier.

  `cornell_box.stage` garde son `light point` et son ciel : c'est la scène du même sujet qui se rend
  aujourd'hui sous les deux intégrateurs, et lui retirer ses `light` n'a plus à servir de
  démonstration.

## 7. Ordre d'attaque

**Tous les prérequis sont acquis.** La CTM fait descendre le placement sur la forme, donc la
géométrie que l'objet visible et la source échantillonnable doivent tenir tous les deux existe en
espace monde (§4) — sans quoi il faudrait donner à l'`AreaLight` une transformation à elle, donc
énoncer deux fois le même placement et vivre avec leur dérive possible. Et la scène déclare son
éclairage, le chargeur n'ajoutant plus rien : la comparaison `naive` / `path` du §6 est possible, et
mesurée à cinq centièmes d'un niveau sur une scène où l'éclairage indirect compte
([docs/eclairage_declare.md](../docs/eclairage_declare.md) §4).

- [x] RNG à graine fixe, graine dans `Config` — deux rendus sont comparables
      ([docs/rendu_reproductible.md](../docs/rendu_reproductible.md)).
- [x] Production `light` de la grammaire `.stage`, et lumières câblées retirées du chargeur
      ([docs/eclairage_declare.md](../docs/eclairage_declare.md)).
- [ ] Trancher les deux décisions ouvertes **avant** d'écrire : l'émission est-elle unilatérale, et
      comment `sample_li` lit la radiance d'une `Texture` (§4). Toutes deux se voient sur l'image, et
      la première vaut un facteur deux.
- [ ] `AreaSampleable` + `ShapeSample`, implémentés sur `Rectangle` d'abord — c'est le panneau du
      Cornell box, et son échantillonnage uniforme est deux nombres.
- [ ] Test de conservation d'aire sur cette implémentation, avant tout usage.
- [ ] `Shape::area_sampler`, sa valeur par défaut, et le diagnostic de chargement qui va avec : un
      matériau émissif sur une forme non échantillonnable est une erreur nommant la forme (§2).
- [ ] `lights/area_light.rs` : `sample_li` par `sample_area`, conversion aire → angle solide dérivée
      dans le doc-comment, radiance nulle du mauvais côté si l'émission est unilatérale.
- [ ] Enregistrement par `SceneBuilderVisitor` : l'objet et la lumière partagent la même forme, déjà
      placée par la CTM (§4).
- [ ] Ne **pas** toucher au garde `is_last_bounce_specular`.
- [x] Un témoin dont tout l'éclairage est son panneau : `test_files/cornell_box_exact.stage` (§6).
- [ ] Comparer `naive` et `path` sur ce témoin, et le dire dans le commit.
- [ ] Étendre à `Sphere` et `Triangle`, puis au maillage — tirage d'un triangle proportionnel à son
      aire, ce qui demande une somme cumulée des aires construite une fois.
- [ ] Échantillonnage en angle solide depuis le point de référence, en remplacement du tirage
      uniforme par aire, une fois la variance mesurée sur le témoin.

Ensuite seulement, et dans leurs propres entrées d'`IDEAS.md` : `Light::pdf_li` puis MIS, qui fait
tomber le garde spéculaire, puis la roulette russe.
