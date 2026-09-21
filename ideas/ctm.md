# La CTM — placer une forme plutôt qu'envelopper un objet

Indexé depuis [IDEAS.md](../IDEAS.md). Le premier geste du §7 est posé, et le sujet est **en tête de
la liste** : la production `light`, [`AreaLight`](area_light.md), MIS et les
[éléments nommés](elements_nommes.md) en dépendent tous, chacun pour une raison différente, et
aucune de ces raisons ne se voit avant d'avoir posé celle-ci.

## 1. Ce que « CTM » veut dire

**CTM** est l'abréviation de *current transformation matrix* — **matrice de transformation
courante**. Le terme vient de RenderMan et se retrouve tel quel dans pbrt.

C'est une notion de **chargeur**, pas de rendu. Pendant qu'il parcourt la description de scène, le
chargeur tient à tout instant la transformation composée qui mène du repère courant de la
description vers l'espace monde : entrer dans un bloc `transform` la multiplie à droite, en sortir
restaure la précédente. Toute géométrie déclarée à l'intérieur est donc **construite déjà placée**,
et il ne reste aucune transformation à appliquer après coup.

Le projet fait aujourd'hui l'inverse. La transformation n'est pas un contexte mais un **nœud**, et
elle s'applique *autour* d'un objet déjà bâti : [`objects::Transformed`](../src/objects/transformed.rs)
enveloppe un `Object`, fait voyager chaque rayon dans l'espace local de son enfant et rapatrie le
résultat. La géométrie, elle, ne sait jamais où elle est.

## 2. Le défaut : un type, deux métiers

`objects::Transformed` fait deux choses que rien, ni dans le code ni dans la grammaire, ne
distingue :

| | Ce que ça veut dire | Ce que ça coûte |
|---|---|---|
| **Placer** | cette forme apparaît une fois, à cet endroit | rien, si le placement est replié dans la forme |
| **Instancier** | cette structure apparaît *n* fois, et je ne veux la construire qu'une | un aller-retour de rayon par instance, et un sous-arbre non fusionné dans le BVH de scène |

Tant qu'elles sont confondues, trois choses restent hors d'atteinte :

- **Une forme ne peut pas être partagée en espace monde.** C'est le blocage d'`AreaLight` : la
  lumière et l'objet visible doivent regarder la même géométrie, or la lumière n'aurait accès qu'au
  rectangle local, à l'origine, pendant que l'objet est rendu ailleurs. L'erreur serait silencieuse,
  l'ombre portée en étant le seul indice.
- **MIS ne peut pas nommer la source qu'un rayon vient de toucher.** Le lien `Interaction::emitter`
  doit être estampillé là où la forme, le matériau et le placement se rencontrent ; aujourd'hui le
  placement arrive plus tard, autour d'un objet déjà clos.
- **Le nommage ne peut pas séparer l'abréviation de l'instanciation**
  ([elements_nommes.md](elements_nommes.md) §3).

Et un quatrième point, plus discret : `csg::Elem { shape, transform }`
([csg.rs](../src/shapes/csg.rs)) *est* déjà un décorateur de transformation au niveau de la forme,
écrit à la main dans un coin de `shapes/`. Le concept existe donc en double, sans que le second
porte son nom.

## 3. Le chantier

Quatre gestes, dont le troisième est le seul qui demande de réfléchir :

1. **`src/shapes/transformed.rs`** — le décorateur qui manque, jumeau de celui d'`objects/` mais un
   étage plus bas :

   ```rust
   /// A shape placed in the world.
   ///
   /// The loader folds the current transformation matrix into the geometry here, so that whoever
   /// holds the shape afterwards — the visible object, and the light sampling the same surface —
   /// reads the same world-space points without having to agree on a placement of their own.
   pub struct Transformed {
       shape: Arc<dyn Shape>,
       to_world: Box<Transform>,
   }

   impl Shape for Transformed {}
   ```

2. **`ObjectTransformedNode::visit` visite sa transformation *avant* son enfant.** Une ligne d'AST,
   et c'est elle qui rend la CTM possible : sans elle, le visiteur apprend le placement après avoir
   construit ce qu'il fallait placer.

3. **`SceneBuilderVisitor` tient la CTM sur une pile**, composée à l'entrée et dépilée à la sortie, et
   `visit_object_simple` construit une forme déjà placée :

   ```rust
   fn visit_object_simple(&mut self, _node: &ObjectSimpleNode) {
       let material = self.materials.pop().unwrap();
       let shape: Arc<dyn Shape> = Arc::new(shapes::Transformed::new(self.shapes.pop().unwrap(), self.ctm()));
       // `shape` is in world space here: the object and, later, the area light that samples it
       // can simply share this `Arc`.
       self.objects.push(Arc::new(Simple::new(shape, material)));
   }
   ```

4. **`csg::Elem` devient un `Arc<dyn Shape>`**, la paire forme + transformation étant désormais un
   type. Un concept de moins, et le même code de placement pour tout le monde. C'est le geste qui
   rapporte le plus de lignes, et celui qui demande le plus d'attention : la suite lui est consacrée.

### La CSG : rien ne change pour qui écrit une scène, et le code se simplifie

`csg union { elem <forme> transform { … } … }` reste mot pour mot ce qu'il est, et
`CSGShapeElemNode { shape, transform }` reste tel quel dans l'AST. Seul change ce que le visiteur en
construit.

Et la simplification est substantielle, parce que **`csg::Elem` *est* déjà un `shapes::Transformed`
écrit à la main**. [`Union::intersect`](../src/shapes/csg/union.rs) fait exactement ce que ferait le
décorateur : rayon vers le local, intersection de l'enfant, chaque touche rapatriée en monde. Idem
pour `contain_point` (point vers le local), `get_bounding_box` (boîte de l'enfant transformée) et
`is_inside`. Le mot `transform` apparaît 8 fois dans `union.rs`, 12 dans `substraction.rs` et 17 dans
`intersection.rs` — **un concept que les trois opérations ré-implémentent chacune pour son compte**.

Avec `elements: Vec<Arc<dyn Shape>>` à la place de `Vec<Box<Elem>>`, elles deviennent de la pure
logique ensembliste :

```rust
// avant — l'opération connaît les repères
let local_ray = e.transform.transform_ray_to_local(&ray);
let element_collisions = e.shape.intersect(&local_ray, near, far);
for collision in element_collisions.iter() {
    let collision_in_world_space = e.transform.transform_interaction_to_world(&collision);
    if !self.is_inside(&collision_in_world_space, e.as_ref()) { … }
}

// après — l'opération ne connaît que des formes
for collision in element.intersect(&ray, near, far).iter() {
    if !self.is_inside(collision, index) { … }
}
```

`csg::Elem` disparaît comme type, et avec lui la comparaison de pointeurs bruts `*const Elem` par
laquelle `is_inside` exclut l'élément qui a produit la touche : elle devient un saut d'indice, plus
clair et sans pointeur.

### Les deux transformations d'une CSG, et pourquoi elles ne se composent pas

C'est le seul point de vigilance du geste, et il doit être écrit dans le visiteur : les confondre
donnerait un double placement, silencieux.

| | Ce qu'elle positionne | Quand elle s'applique |
|---|---|---|
| la transformation d'un `elem` | une pièce **à l'intérieur** de l'assemblage, dans le repère propre de la CSG | à la construction de l'élément |
| la CTM | la CSG **entière** dans le monde | à `visit_object_simple`, une fois la forme complète |

Donc **un bloc `transform` d'`elem` n'empile pas la CTM**. Une CSG se construit dans son repère à
elle et se place une fois, comme n'importe quelle forme. On obtient des décorateurs imbriqués —
`Transformed(Union(Transformed(sphère), Transformed(sphère)), ctm)` — qui sont exactement les deux
niveaux d'aller-retour de rayon d'aujourd'hui, ni plus ni moins.

C'est aussi ce qui garde une CSG **nommable** : une forme déclarée dans son propre repère se replace
où l'on veut, ce qui est la règle « les `define` vivent hors du bloc `scene` » d'
[elements_nommes.md](elements_nommes.md) §8. Les deux chantiers tirent dans le même sens.

Dernier point, qui ferme la boucle avec [`AreaLight`](area_light.md) : une CSG ne sait pas
s'échantillonner par aire, donc son `area_sampler` rend `None`, donc `diffuse_light` posé sur une CSG
est une erreur de chargement nommant la forme — et non un objet lumineux qui n'éclaire rien.

## 4. Ce que devient `objects::Transformed` — il ne disparaît pas

C'est la nuance que le mot « CTM » fait facilement oublier, la CTM étant un mécanisme de *chargeur* :

- **`examples/` construit ses scènes à la main**, sans chargeur donc sans CTM, et appelle
  `Transformed::new` en une trentaine d'endroits. Rien n'y change : c'est le mécanisme de placement
  programmatique, et il reste juste.
- **Dans le chargeur, il devient le mécanisme d'instanciation** — sans usage tant que les éléments
  nommés ne sont pas là, ce qui est assumé : c'est un outil qui attend son appelant, pas du code
  mort.

Sa documentation doit dire lequel des deux métiers il sert dans quel contexte ; le nom, lui, reste
juste pour les deux.

## 5. Ce que le chantier ne rapporte pas, et le piège qui va avec

**Aucun gain par rayon.** Le décorateur de forme hérite exactement de l'aller-retour que celui
d'objet faisait. Ce chantier se justifie par la structure — une géométrie partagée en espace monde,
un seul concept de placement, le lien d'identité pour MIS à la construction de la feuille — et par
rien d'autre. Ne pas lui prêter d'argument de vitesse.

**Et il peut coûter, si on l'écrit sans y penser.** Le chemin chaud d'aujourd'hui transforme **une**
`Interaction` : `objects::Transformed` étant au-dessus de `Simple`, il reçoit le plus proche point
touché et le rapatrie seul. Un décorateur de forme est *en dessous* de `Simple`, donc il reçoit la
liste entière que rend `Intersectable::intersect` — et la rapatrier dans un second
`IntersectionResult` ajouterait une allocation par touche, qui est l'entrée ouverte d'`IDEAS.md`
sous *Accélérateurs*.

Le remède est d'une ligne : l'enfant rend un vecteur **possédé**, donc les copies en espace monde
remplacent les locales sur place.

La même ligne irait à `objects::Transformed`, qui construit encore un second vecteur — mais elle n'y
gagnerait rien : **personne n'appelle `Intersectable::intersect` à l'étage objet**, les rayons
d'ombre passant par `intersect_p` et le chemin chaud par `Object::intersect`. Ce n'est donc pas une
mesure, c'est une symétrie de lecture, et elle voyage avec l'item de documentation du §7. Ce que la
couture `Object: Intersectable` devient est une question à elle seule, ouverte dans `IDEAS.md` sous
*Renderer & infrastructure*.

```rust
fn intersect(&self, ray: &Ray, near: f64, far: f64) -> IntersectionResult {
    let local_ray = self.to_world.transform_ray_to_local(ray);
    let mut hits = self.shape.intersect(&local_ray, near, far);

    // The child hands back an owned vector, so each world-space copy replaces the local one where
    // it stands. Collecting into a second `IntersectionResult` would allocate once per hit.
    for hit in hits.iter_mut() {
        *hit = self.to_world.transform_interaction_to_world(hit);
    }

    hits
}
```

Avec cette forme, le chantier est neutre sur le chemin chaud. Sans elle, c'est une régression —
**c'est la seule mesure que ce chantier demande**, et elle se prend au chronomètre d'un rendu
complet, pas sur `bvh_stats` (§6).

## 6. La preuve

**C'est un refactor : l'image doit rester identique au bit près, à chaque commit.** Les rendus étant
reproductibles ([docs/rendu_reproductible.md](../docs/rendu_reproductible.md)), cette vérification
est disponible et sans ambiguïté — c'est la même que celle qui a tenu le chantier du repère de
shading.

Les compteurs de `bvh_stats` ne doivent pas bouger non plus, et c'est un contrôle indépendant du
précédent : les deux décorateurs calculent leur boîte de la même façon — les huit coins de la boîte
de l'enfant, transformés —, donc l'arbre construit est le même arbre. Un compteur qui se déplace
signale que quelque chose d'autre a changé en chemin.

## 7. Ordre d'attaque

- [x] `shapes::Transformed`, avec la transformation sur place du §5, et ses tests — dont un qui
      recoupe le placement d'un point échantillonné avec celui d'un point touché par un rayon.
- [x] `ObjectTransformedNode::visit` réordonné, CTM dans `SceneBuilderVisitor`, `visit_object_simple`
      qui construit placé. Image identique au bit près à ce commit : c'est le commit risqué.
- [x] `PrintVisitor` suit le nouvel ordre de visite sans changer sa sortie — l'aller-retour reste un
      aller-retour. Sa sortie de scène n'était imprimée par personne et affirmée par aucun test :
      `PrintVisitor::rendered` et une assertion la tiennent désormais.
- [ ] `csg::Elem` réduit à un `Arc<dyn Shape>` (§3), l'exclusion de `is_inside` passée à un indice, et
      un test qui vérifie qu'une CSG placée par la CTM n'applique pas deux fois le même déplacement —
      c'est le piège que le tableau des deux transformations décrit.
- [ ] Documentation d'`objects::Transformed` : placement programmatique pour `examples/`,
      instanciation pour le chargeur (§4). La transformation sur place du §5 y passe aussi, par
      symétrie avec le décorateur de forme et sans gain à en attendre — le dire dans le message.
- [ ] Chronomètre d'un rendu complet avant / après, pour clore la question du §5.

## 8. Ce que ça débloque, et qui est toute la raison de le faire

- **La production `light` et `AreaLight`** : la forme en espace monde se partage entre l'objet
  visible et la source échantillonnable, sans qu'aucun trait ne s'élargisse
  ([area_light.md](area_light.md) §4).
- **MIS** : `Interaction::emitter` s'estampille à la feuille, là où la forme, le matériau et le
  placement se rencontrent enfin au même endroit.
- **Les éléments nommés** : construire cuisant le placement, la table des noms devient une table de
  nœuds d'AST — et c'est ce choix qui rend le nommage compatible avec la CSG et avec les surfaces
  émissives ([elements_nommes.md](elements_nommes.md) §3).
