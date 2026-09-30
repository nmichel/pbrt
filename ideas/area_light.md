# `AreaLight` — une surface émissive qui éclaire

Indexé depuis [IDEAS.md](../IDEAS.md). En cours, et en tête de la liste. **C'est le plus grand écart
au modèle physique du projet** : les pièces existent et sont testées, mais aucune scène ne construit
encore d'`AreaLight`, donc rien n'a changé dans l'image. Le §7 dit où l'on en est.

La radiométrie et les dérivations, pour les parties qui ont atterri, vivent désormais dans
[docs/sources_de_lumiere.md](../docs/sources_de_lumiere.md) — ce fichier ne garde que ce qui reste à
faire, et disparaîtra avec le sujet.

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

Ce qui manque est en amont.

**Une forme sait s'échantillonner, quand elle le peut.** `AreaSampleable` et `ShapeSample` vivent
dans [src/shapes.rs](../src/shapes.rs), à côté de `Shape` — c'est une face de la géométrie, pas de
l'objet —, et [`Rectangle`](../src/shapes/rectangle.rs) les implémente. Le trait est à part plutôt
qu'une méthode de `Shape` parce que `Plane` a une aire infinie et `csg::Union` une aire qui n'est
pas calculable : si `Shape` l'exigeait, ces formes devraient répondre à une question qu'elles ne
savent pas poser, et les seuls corps honnêtes seraient un panic ou un mensonge. Le raisonnement
complet et la dérivation du tirage sont dans les doc-comments.

**La route jusqu'à cette face est en place.** `Shape::area_sampler` a un corps par défaut à `None`,
`Rectangle` rend `Some(self)`, et [`shapes::Transformed`](../src/shapes/transformed.rs) relaie —
sans quoi, le chargeur enveloppant toute forme dans ce décorateur, **tout objet émissif aurait
paru non échantillonnable**. C'est la symétrique exacte du `Material::emitter` du §4, et pour la
même raison : Rust n'a pas d'upcast de trait objet, et le type concret est perdu au moment précis
où la question se pose.

Le relais place le point tiré comme il place déjà une touche, et laisse traverser inchangées les
(u, v) — un placement ne déplace pas un point dans son propre paramétrage — ainsi que la densité,
exprimée en mesure d'aire, qu'une isométrie ne change pas.

**Corollaire, et il n'est pas facultatif : un matériau émissif posé sur une forme qui ne sait pas
s'échantillonner est une erreur de chargement**, nommant la forme fautive. Rendre l'objet visible
mais non échantillonné reconduirait en silence le défaut même que ce chantier répare.

## 3. Le changement de mesure, qui est le cœur du sujet

**Atterri** ([lights/area_light.rs](../src/lights/area_light.rs)). La dérivation complète, depuis les
grandeurs radiométriques, est en [docs/sources_de_lumiere.md](../docs/sources_de_lumiere.md) §3 ; ce
qui suit est la forme sous laquelle la décision a été prise, et seul « l'écart assumé du départ »,
en fin de section, porte encore quelque chose qui n'est pas fait.

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
- **L'émission est unilatérale**, et c'est tranché : [`DiffuseLight`](../src/materials/diffuse_light.rs)
  n'émet que du côté de sa normale. `sample_li` doit donc rendre une radiance nulle quand
  `dot(nₗ, −wi) ≤ 0`, faute de quoi NEE éclairerait depuis une face que l'œil voit noire.

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

self.objects.push(Arc::new(Simple::new(Arc::clone(&shape), Arc::clone(&material))));   // visible

match (Arc::clone(&material).emitter(), Arc::clone(&shape).area_sampler()) {
    (Some(emitter), Some(sampler)) => self.scene.add_light(Arc::new(AreaLight::new(sampler, emitter))),
    (Some(_), None) => return Err(/* nommant la forme */),
    (None, _) => {}
}
```

**C'est ce que la CTM a rendu possible**, et ce prérequis est acquis : la forme sort du chargeur en
espace monde, [`shapes::Transformed`](../src/shapes/transformed.rs) ayant replié le placement dans la
géométrie. Le placement vivant *au-dessus* du point de partage, la lumière échantillonnerait la forme
en espace local pendant que l'objet est rendu ailleurs — erreur silencieuse dont l'ombre portée
serait le seul indice.

### L'émission appartient au matériau, et c'est un écart délibéré à pbrt

Une question se cache sous le `emitter` du bloc ci-dessus : **qui répond à « que cette surface
émet-elle ? »** Ici c'est le matériau, par [`Emitter::l`](../src/materials.rs), et `Material::emit`
comme, demain, `AreaLight::sample_li` y lisent la même implémentation. Dans pbrt c'est la lumière :
le matériau n'émet pas du tout, la primitive porte optionnellement une `AreaLight`, et l'interaction
la lui demande — le stub mort [`Intersection::le`](../src/geom/intersectable.rs) est cette
conception-là, recopiée puis commentée.

**L'émission reste au matériau**, pour deux raisons qui ne sont pas de goût. Le tableau du §2 de
CLAUDE.md énonce déjà `Material` comme « échantillonner (`scatter`) et évaluer (`f`) une BSDF,
**émettre (`emit`)** » : suivre pbrt demanderait de réécrire cette ligne. Et un `get_area_light` sur
la primitive percerait exactement la couture au nom de laquelle la route `Scene::commit` est écartée
en tête de section — dans l'autre sens, mais c'est la même.

**La forme est en place** : `Emitter` et `Material::emitter` vivent dans
[src/materials.rs](../src/materials.rs), `DiffuseLight` les implémente, et le doc-comment de chacun
porte son raisonnement. `AreaLight` tiendra un `Arc<dyn Emitter>` et non un `Arc<dyn Material>` :
elle ne recevra pas un objet capable de `scatter` dont elle ignorerait les trois quarts.

Ce qui reste à en faire est au §7. Le trait n'attend qu'un second consommateur — aujourd'hui seuls
`Material::emit` et les tests l'appellent —, et `Material::emitter` qu'un appelant au chargement,
lequel arrivera avec `Shape::area_sampler` dont il est la symétrique exacte (§2).

**Le prix, et il se paie au chantier suivant : MIS.** Quand un rayon échantillonné par la BSDF touche
une surface émissive, l'intégrateur doit pondérer par `power_heuristic(pdf_bsdf, pdf_li)`, donc
connaître *l'identité de la lumière touchée* — son aire, et la probabilité discrète `1/N` de l'avoir
tirée. Le `get_area_light` de pbrt n'est pas de la décoration, c'est ce chaînon ; ici l'intégrateur
tient le matériau, pas la lumière, et ne l'a donc pas. La sortie n'est pas forcément celle de pbrt :
la plus naturelle est qu'`Interaction` porte l'`Option<&dyn Light>` à côté du matériau — le même
chaînon, posé sur le type dont le rôle *est* de marier géométrie et matériau. Elle a un coût,
[`Simple::intersect`](../src/objects/simple.rs) clonant déjà un `Arc` par touche. **À trancher avec
MIS, pas ici.**

**Ce que `sample_li` donne à lire à la `Texture` est tranché**, et il n'y a plus rien à décider ici :
`Texture::shade` prend un [`SurfacePoint`](../src/geom/surface_point.rs) — `p`, `n`, `u`, `v` —, qui
se construit sans rayon. Un `ShapeSample` en porte un, donc NEE lit la radiance sur la texture même
que l'œil voit, au même point, sans qu'un point tiré ait à inventer une distance ni une direction
d'observation. L'écueil que ce choix écarte est qu'une source texturée éclaire d'une couleur et se
voie d'une autre.

**Les deux décisions qui devaient être prises avant d'écrire le sont** — celle-ci et
l'unilatéralité du §3. Ce qui suit dans le §7 est du code, plus un arbitrage.

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
  attrape un `pdf` faux d'un facteur constant, ce que l'œil ne voit pas. **Fait**, avec les deux
  moments qui éprouvent l'uniformité, que celui-ci seul ne voit pas.
- **Deux tests du changement de mesure, et il en faut deux.** Le premier est celui qui se calcule à
  la main : source plane vue de face à distance `d`, densité en angle solide `d²/aire`. Il ne suffit
  pas — son cosinus vaut 1, donc un cosinus omis y passe, ce que la mutation confirme. Le second est
  le pendant du précédent une mesure plus haut : `Σ 1/pdf_ω / N` converge vers **l'angle solide
  sous-tendu** par la source, `Ω = 4·atan(ab / (d·√(a² + b² + d²)))` pour un rectangle vu sur son
  axe. Il éprouve `d²` et le cosinus d'un seul nombre. **Faits.**
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
- **Le second témoin est [test_files/indirect_lighting.stage](../test_files/indirect_lighting.stage)**,
  et il vise autre chose que le premier. Un bloqueur opaque de 300 × 10 × 300 est posé entre le
  panneau et le sol, donc **presque tout ce qui éclaire le sol y arrive par rebond** : c'est la
  quantité que le défaut du §1 supprime, isolée par la géométrie plutôt que déduite d'un niveau
  moyen. Là où `cornell_box_exact.stage` dit si le niveau absolu est juste, celui-ci dit si le
  transport indirect existe.

  Il n'est pas encore un témoin utilisable : il déclare un `light point`, donc son sol n'est pas
  noir aujourd'hui et l'écart à mesurer est dilué. Le rendre éclairé par son seul panneau est à
  faire **avec** l'`AreaLight`, dans le même geste que la comparaison du témoin principal.

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
- [x] Comment `sample_li` lit la radiance d'une `Texture` (§4) : `Texture::shade` prend un
      [`SurfacePoint`](../src/geom/surface_point.rs), qui se construit sans rayon.
- [x] L'émission est unilatérale, du côté de la normale (§3), et huit scènes disent désormais de
      quel côté leur lampe éclaire — sept portaient un `rotate_y π`, qui laisse `(0, 1, 0)`
      inchangé et n'orientait donc rien.
- [x] `Emitter` + `Material::emitter` dans [src/materials.rs](../src/materials.rs), et
      `DiffuseLight::emit` qui délègue à `Emitter::l` : la règle d'unilatéralité et la lecture de la
      texture n'ont qu'une implémentation (§4). Le `Option` de `emit` ne dit plus qu'une chose —
      `None` signifie « pas un émetteur », une face sombre rendant `Some(BLACK)`.
- [x] `AreaSampleable` + `ShapeSample` dans [src/shapes.rs](../src/shapes.rs), implémentés sur
      [`Rectangle`](../src/shapes/rectangle.rs) — le panneau du Cornell box, dont l'échantillonnage
      uniforme est deux nombres.
- [x] Conservation de l'aire, **et** uniformité du tirage. La première seule ne suffit pas : la
      densité étant constante, `Σ 1/pdf / N` vaut l'aire quel que soit l'endroit où tombent les
      points. Il faut un second moment pour la forme du tirage, et un premier pour son centre — `x²`
      étant pair, il donne la même valeur sur `[0, a]` que sur `[-a, a]`, donc il ne verrait pas un
      tirage sur la mauvaise moitié.
- [x] `Shape::area_sampler` et sa valeur par défaut, `Rectangle` qui rend `Some(self)`, et le
      relais par `shapes::Transformed`, sans lequel toute forme placée aurait paru non
      échantillonnable (§2).
- [x] Les (u, v) d'un point *tiré* sont ceux d'un point *touché* au même endroit, sur la forme nue
      **comme sur la forme placée**. `Rectangle` rend `u = p.x, v = p.z`, non normalisés ; les
      normaliser au tirage, ou les placer au relais, ferait éclairer une source texturée d'une
      couleur et se voir d'une autre — ce que le §4 interdit, et ce que la mutation des deux
      confirme.
- [x] [`lights/area_light.rs`](../src/lights/area_light.rs) : `sample_li` par `sample_area`,
      changement de mesure aire → angle solide dérivé dans l'en-tête du module, et pas d'échantillon
      du tout du côté que `DiffuseLight` laisse noir (§3). `LightType` gagne `Area`, la seule des
      trois qu'un rayon puisse toucher.
- [x] Enregistrement par `SceneBuilderVisitor` : l'objet et la lumière partagent la même forme, déjà
      placée par la CTM (§4), **et le diagnostic de chargement avec** — un matériau émissif sur une
      forme non échantillonnable arrête le chargement en nommant la forme (§2). Les deux sont un
      seul geste : le diagnostic ne s'ajoute pas, il tombe de la conjonction des deux questions — le
      matériau émet-il, la forme s'échantillonne-t-elle. `ShapeNode` gagne un `name()` pour que le
      message cite le mot que l'auteur a écrit, un `Arc<dyn Shape>` n'ayant plus de nom à donner.
- [x] Ne **pas** toucher au garde `is_last_bounce_specular` — il tient : l'émission n'est comptée
      qu'en vue directe ou après un rebond spéculaire, NEE fait le reste, et rien n'est compté deux
      fois. Ce qui le fera tomber est MIS.
- [x] Un témoin dont tout l'éclairage est son panneau : `test_files/cornell_box_exact.stage` (§6).
- [x] Comparer `naive` et `path` sur ce témoin, et le dire dans le commit. **Ce que la comparaison a
      trouvé** : le panneau s'occultait lui-même. Le point tiré sur une source d'aire est un point
      d'une surface de la scène, et `intersect_p` accepte une touche à `t == far`, donc trois rayons
      d'ombre sur quatre revenaient bloqués par la lampe. Corrigé par un `SHADOW_RAY_END_EPSILON`
      relatif, pendant exact de celui du départ ; mesure et raisonnement en
      [docs/sources_de_lumiere.md](../docs/sources_de_lumiere.md) §6.6.
- [x] **`Sphere` échantillonnable par aire.** `area()` vaut `4πr²` et le tirage est φ uniforme avec
      **cos θ** uniforme — pas θ, qui entasserait les points aux pôles sans que rien ne manque à
      l'image. Les (u, v) du point tiré sont ceux du paramétrage de
      `compute_intersection_details`, sans quoi une lampe texturée éclairerait d'une couleur et se
      verrait d'une autre. Rien d'autre n'a été touché : l'enregistrement interroge
      `Material::emitter` et `Shape::area_sampler`, donc répondre à la seconde a suffi pour qu'une
      sphère émissive devienne une source.

      **Le prix, mesuré** : la part des tirages qui survit est la calotte que le point ombré voit,
      soit `(1 − r/d)/2` exactement — la moitié au mieux, et moins de près. Sur le témoin,
      32,7 % survivent juste sous la lampe et 43,8 % à 8,5 d'écart, contre 33,3 % et 44,1 % prédits.
      **Deux tirages sur trois sont jetés là où la lumière compte le plus**, et c'est la ligne de
      base que l'échantillonnage par cône doit battre.
- [ ] **`Triangle` puis le maillage.** Deux questions et non une : tirer *dans* un triangle, et
      choisir *lequel* proportionnellement à son aire — ce second point demandant une somme cumulée
      construite une fois. [lamp_triangle.ply](../test_files/lamp_triangle.ply) n'a qu'un triangle
      et ne peut rien dire du choix ; il faudra un second témoin à triangles d'aires nettement
      inégales, un maillage régulier ne distinguant pas un choix correct d'un choix uniforme.
- [ ] Échantillonnage en angle solide depuis le point de référence, en remplacement du tirage
      uniforme par aire, une fois la variance mesurée sur le témoin — elle l'est
      ([docs/sources_de_lumiere.md](../docs/sources_de_lumiere.md) §9.2).

      **Ce que cela coûte est plus petit qu'il n'y paraît**, et vaut d'être su avant de choisir
      l'ordre. `Light` ne bouge pas : `sample_li` reçoit déjà le point de référence et promet déjà
      une densité en angle solide, donc les trois sources sans géométrie ne sont pas concernées.
      Seul `AreaSampleable` change, il n'a que deux implémentations, et il change **par ajout** —
      la seconde méthode a un corps par défaut qui est exactement la conversion qu'`AreaLight`
      écrit aujourd'hui, comme les deux variantes de `Shape::Sample` chez pbrt. Aucune
      implémentation existante n'est donc forcée de changer.

      Les deux vraies difficultés sont ailleurs. **Le relais de `Transformed`** : tant que le corps
      par défaut s'applique il est juste, son `sample_area` rendant déjà des points en espace
      monde ; mais dès qu'une forme redéfinit le tirage, celui-ci a lieu en espace local et le
      relais doit y porter le point de référence puis ramener le résultat — densité comprise, qu'un
      déplacement laisse invariante mais qu'une mise à l'échelle ne laisserait pas. `Matrix4::scale`
      existe, la grammaire n'a pas d'étape `scale`, et ses quatre usages sont dans les caméras : le
      défaut est latent, pas vivant. **Et le `pdf` de `ShapeSample`** porterait alors deux mesures
      selon la méthode qui l'a produit, ce qui est l'ambiguïté qu'on a retirée du `Option` de
      `Material::emit` pour la même raison. Deux types de retour distincts.

Ensuite seulement, et dans leurs propres entrées d'`IDEAS.md` : `Light::pdf_li` puis MIS, qui fait
tomber le garde spéculaire, puis la roulette russe.
