# Éléments nommés dans la grammaire `.stage`

Indexé depuis [IDEAS.md](../IDEAS.md). Non commencé, et délibérément placé **après MIS** : c'est un
confort d'écriture des scènes, rien du transport de lumière n'en dépend. Sa seule dépendance est
acquise : la CTM fait descendre le placement sur la forme et recentre `objects::Transformed` sur
l'instanciation ([docs/mesures_placement.md](../docs/mesures_placement.md)).

## 1. Le besoin

La grammaire ne sait rien nommer. Toute réutilisation est une copie textuelle : un matériau partagé
par cinq objets est déclaré cinq fois, et le changer demande cinq corrections dont une peut
manquer ; une lampe posée dix fois est écrite dix fois. C'est un défaut d'ergonomie, pas de justesse
— d'où sa place dans la liste.

## 2. Deux sémantiques, et elles ne servent pas le même besoin

Nommer, c'est associer un nom à quelque chose. Tout dépend de *quoi*.

| | **(A) Nom d'un nœud d'AST** | **(B) Nom d'un objet construit** |
|---|---|---|
| Instancier, c'est | ré-élaborer le sous-arbre sous la transformation courante | envelopper l'objet partagé dans `objects::Transformed` |
| Se compose avec CSG | oui, pour une forme nommée | non (§5) |
| Surfaces émissives | chaque copie fabrique sa propre `AreaLight`, à sa place | ambigu, à interdire (§8) |
| Coût à l'exécution | nul : l'arbre final est plat, aucune indirection | un aller-retour de rayon par instance, sous-arbre non fusionné dans le BVH de scène |
| Ce qui est partagé | la géométrie lourde, par `Arc` — un maillage d'un million de triangles n'est pas copié | la structure entière : *n* instances font *n* primitives, pas *n* × feuilles |

**(A) répond au besoin du §1**, qui est un besoin d'écriture. (B) répond à un besoin de mémoire —
une forêt de dix mille arbres — que le projet n'a pas encore rencontré. **Faire (A) d'abord**, et
n'ajouter (B) que le jour où une scène réelle le réclame, sous un mot-clé distinct : les deux ne
sont pas interchangeables, et laisser croire qu'elles le sont produirait des écarts de performance
inexplicables pour qui écrit la scène.

## 3. Ce que la CTM impose à ce chantier, et ce qu'elle lui laisse

**Elle ne rend pas (A) possible** — ré-élaborer un sous-arbre d'AST marche avec ou sans elle.

**Elle rend (A) obligatoire pour qui veut replacer.** Sous une CTM, construire cuit le placement :
l'objet bâti est en espace monde, et aucune transformation ne le ramène ailleurs sans défaire ce qui
a déjà été replié dans ses feuilles. Donc la table des noms stocke des **nœuds d'AST**, pas des
`Arc<dyn Object>`. C'est le point de dessin dont tout le reste découle.

**Elle n'autorise (B) que par `objects::Transformed`**, dont c'est précisément le métier : placer est
l'affaire de la CTM, instancier reste la sienne. Les deux mécanismes sont complémentaires, comme chez
pbrt, où la CTM et `TransformedPrimitive` coexistent depuis toujours.

## 4. La syntaxe

Le lexer rend `Token::Identifier` pour tout mot qui n'est pas un mot-clé, et les mots-clés sont
réservés. Donc **une référence ne demande aucune syntaxe nouvelle** : là où une production est
attendue, un identifiant est une référence. Un seul mot-clé à ajouter, au point de déclaration.

```
define shape    lamp_glass  csg substraction { elem sphere 1.0 transform { }
                                               elem sphere 0.9 transform { } }
define material brass       metal 0.1 color 0.8 0.6 0.2
define object   lamp        compound {
  object simple lamp_glass brass
  object transformed
    object simple rectangle 0.4 0.4 diffuse_light color 12 12 12
    transform { translate 0 0.3 0 }
}

scene
  object transformed object lamp transform { translate -2 0 0 }
  object transformed object lamp transform { translate  2 0 0 }

  # une forme nommée se recompose dans une CSG, avec un matériau libre à chaque usage
  object simple
    csg union {
      elem lamp_glass transform { rotate_y 30 }
      elem sphere 1
    }
    brass
```

`define` plutôt que `let` : le second suggère une variable, donc une valeur qui pourrait changer,
quand il s'agit d'une abréviation.

## 5. Ce qui se compose avec quoi, et pourquoi la limite est juste

Un `csg::Elem` prend une **`Shape`**, et une CSG raisonne par appartenance d'un point à des solides.
Un `compound` nommé est un ensemble d'**`Object`s**, donc de matériaux — et l'intersection d'une
sphère rouge et d'un cube bleu n'a pas de couleur. La règle à écrire dans la documentation de la
grammaire :

- une **forme** nommée se réutilise partout où une forme est attendue, CSG comprise, avec un
  matériau différent à chaque usage ;
- un **objet** nommé se réutilise partout où un objet est attendu, jamais dans une CSG.

C'est la couture `Shape` / `Object` de CLAUDE.md §2 qui parle, pas une insuffisance du dessin. Elle
dicte la bonne pratique : nommer au niveau `shape` chaque fois que c'est possible, puisque c'est ce
qui se réutilise le plus largement.

## 6. Là où les deux visiteurs cessent de se ressembler

Jusqu'ici `PrintVisitor` et `SceneBuilderVisitor` font le même parcours et n'en tirent pas la même
chose. Une référence les sépare pour la première fois : le constructeur doit **développer** la
déclaration, l'imprimeur doit rendre le **nom** — sans quoi l'aller-retour perd le nommage et grossit
le fichier au lieu de le reproduire.

Conséquence de dessin, et elle n'est pas facultative : **`Node::visit` d'un nœud de référence ne
récurse pas.** Il appelle la méthode du visiteur, rien d'autre. La récursion appartient à
`SceneBuilderVisitor::visit_object_named`, qui va chercher la déclaration et la visite lui-même. Si
la récursion était dans `visit`, `PrintVisitor` recevrait le développement sur sa pile et n'aurait
plus aucun moyen d'imprimer le nom.

## 7. La friction Rust, connue d'avance

`Visitor::visit_*` reçoit `&Node` de durée de vie anonyme : le visiteur ne peut pas *emprunter* un
nœud pour le revisiter plus tard. Les nœuds déclarables doivent donc être des `Rc<dyn …>` et non des
`Box<dyn …>` — un changement limité à eux, pas une durée de vie virale sur le trait `Visitor` :

```rust
/// Named declarations, elaborated on each reference rather than stored built: under a CTM,
/// building bakes the placement, so a built object can no longer be re-placed.
declarations: HashMap<String, Rc<dyn Node>>,
```

```rust
let declared = Rc::clone(self.declarations.get(&node.name).expect("undeclared name"));
declared.visit(self); // owned handle: `self` is free to be borrowed mutably
```

`Node::visit` prend déjà `&self`, donc il est ré-entrant tel quel.

## 8. Les pièges, à trancher avant d'écrire

- **La capture de CTM.** Une déclaration élaborée sous une transformation courante non triviale
  capturerait la position du point de déclaration, et `lamp` voudrait dire une chose différente
  selon l'endroit du fichier où elle est écrite. Remède retenu : **les `define` vivent hors du bloc
  `scene`**, donc hors de tout contexte de transformation. C'est une règle de grammaire, que le
  parser vérifie, plutôt qu'une invariante à tenir dans le visiteur.
- **Les cycles.** `define object a compound { object a }` fait récurser le constructeur jusqu'au
  débordement de pile, sans message. Une déclaration ne peut référencer que des noms déjà déclarés :
  la règle interdit le cycle par construction et s'applique en une comparaison au moment de
  l'insertion dans la table.
- **Nom inconnu, nom redéclaré.** Les deux sont des erreurs de chargement, pas des silences. Le
  second en particulier : une redéclaration qui écrase ferait dépendre le rendu de l'ordre des
  lignes.
- **Une surface émissive dans une instance partagée**, et c'est le seul piège de justesse physique.
  Sous (A) il n'existe pas : chaque ré-élaboration fabrique sa propre `AreaLight` à sa propre place.
  Sous (B) la géométrie n'existe qu'une fois en espace local alors qu'il faut *n* sources, une par
  instance, et le lien « ce point touché appartient à quelle source » — celui dont MIS a besoin —
  redevient ambigu dès qu'une instance contient plus d'un émetteur. **pbrt refuse purement et
  simplement la géométrie émissive dans une instance**, et c'est la décision à reprendre : erreur de
  chargement, avec un message qui dit quoi faire à la place — déclarer les copies.

  Si (B) arrive un jour, la forme qui généralise proprement n'est pas un champ de transformation sur
  la lumière mais le décorateur : l'`AreaLight` tient toujours un échantillonneur **en espace
  monde**, et une instance lui donne `shapes::Transformed(échantillonneur_partagé, transformation de
  l'instance)` — une enveloppe minuscule par instance, pas une copie.

## 9. Ordre d'attaque

La chaîne habituelle, mot-clé `define` du [lexer](../src/loader/parser/lexer.rs) →
[parser](../src/loader/parser.rs) → nœuds d'[AST](../src/loader/ast.rs) → méthodes de
[`Visitor`](../src/loader/visitors.rs) → les deux visiteurs → le support éditeur d'
[editors/vscode/](../editors/vscode/), que [tests/vscode_grammar_sync.rs](../tests/vscode_grammar_sync.rs)
rend obligatoire.

- [ ] `Box` → `Rc` sur les nœuds déclarables, seul prérequis structurel (§7).
- [ ] `define` et la table de déclarations, avec les trois diagnostics du §8 — nom inconnu, nom
      redéclaré, référence en avant — et un test par diagnostic.
- [ ] Références de **forme** d'abord : c'est ce qui se compose le plus largement, CSG comprise, et
      ce qui vérifie le dessin au moindre coût.
- [ ] Références de **matériau** et de **texture**, qui n'ont aucune interaction avec le placement.
- [ ] Références d'**objet**, où la transformation courante entre en jeu — et où le test qui compte
      est qu'une déclaration instanciée deux fois à deux endroits produise deux objets distincts,
      correctement placés.
- [ ] Une scène de `test_files/` réécrite avec des noms, rendue avant et après : **l'image doit être
      identique au bit près**. C'est la seule preuve que le développement ne change rien.
- [ ] `PrintVisitor` rend le nom et non le développement (§6), vérifié par un aller-retour sur cette
      même scène.

**Non traité ici, et volontairement** : l'instanciation vraie (B). Elle mérite sa propre entrée le
jour où une scène la réclame, avec son mot-clé, son interdit sur les émetteurs et sa mesure.
