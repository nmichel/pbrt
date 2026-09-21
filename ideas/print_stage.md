# `--print` — donner un appelant au `PrintVisitor`

Indexé depuis [IDEAS.md](../IDEAS.md). Non commencé. Charger une scène, l'imprimer, sortir sans
rendre : une option de ligne de commande de vingt lignes, posée sur un visiteur qu'il faut nettoyer
avant.

## 1. Ce que l'option imprime, et ce qu'elle n'imprime pas

**L'AST, pas la scène bâtie.** [`PrintVisitor`](../src/loader/visitors/print.rs) parcourt l'arbre du
parser et ne voit rien de ce que `SceneBuilderVisitor` construit. Depuis la CTM les deux vues
divergent visiblement : `--print` montre `object transformed … transform { … }` là où la scène bâtie
ne contient plus aucun `objects::Transformed`, le placement étant replié dans la géométrie.

Ce n'est pas un défaut — « ce que le parser a compris » est ce qu'on veut lire quand un fichier ne
donne pas l'image attendue — mais **le texte d'aide doit le dire**, sinon l'option promet un vidage
de scène qu'elle ne fait pas. Ce vidage-là, primitives et boîtes d'une scène commise, serait un
autre outil, et il n'existe pas.

## 2. Ce que ça répare

Le `PrintVisitor` n'a **aucun appelant**. `PrintVisitor::visit` ne prend qu'un `SceneNode`, donc
`visit_stage` est inatteignable, et ses deux tests d'origine impriment sans rien affirmer — seul
`test_a_placement_prints_after_the_object_it_places`, ajouté avec la CTM, tient une assertion. Un
visiteur sans appelant est du poids mort qui ressemble à du code vivant : soit il sert, soit il
part, et cette option est la façon de répondre « il sert ».

Et exercer révèle, comme le §3.3 le montre : un défaut y dort depuis toujours précisément parce que
rien n'appelle la méthode.

## 3. Trois défauts à régler avant, sinon l'option est inutilisable

### 3.1 `visit_shape_mesh` relit le `.ply` et imprime chaque événement

Il installe un `PlyEventObserver` qui affiche une ligne par propriété de sommet et une par face :
**249 186 lignes pour une scène portant `bun_zipper.ply`**, mesurées — soit ses 35 947 sommets à
cinq propriétés, plus ses 69 451 faces. Un `--print` sur n'importe quelle scène à maillage noie donc
le terminal, et le volume croît avec le nombre de propriétés que le fichier déclare. Au-delà du
volume, **un imprimeur n'a pas à parser un `.ply`** : il pousse `mesh <fichier>` et ne lit rien.

### 3.2 La sortie n'est pas du `.stage` rechargeable

C'est un vidage, et chaque écart est un point à reprendre :

| Ce qui sort | Ce que la grammaire veut |
|---|---|
| `color Spectrum { spectrum: [0.2, 0.8, 0.1] }` | `color 0.2 0.8 0.1` |
| `translate [0 0 2]` | `translate 0 0 2` |
| `rotate X 1.5708` | `rotate_x 1.5708` |
| `mesh ./f.ply` | `mesh file "./f.ply"` |

Les séparateurs sont à revoir dans le même geste : `translate [0 0 2]rotate X 1.5708` sort sans
espace entre deux étapes.

### 3.3 `visit_stage` a ses deux liaisons inversées

`StageNode::visit` visite la scène **puis** la caméra, donc le premier `pop` rend la caméra :

```rust
let scene = self.stack.pop().unwrap();   // c'est la caméra
let camera = self.stack.pop().unwrap();  // c'est la scène
println!("stage {} {}", scene, camera);
```

Le texte sort dans le bon ordre — caméra puis scène, comme la grammaire — **par compensation de deux
erreurs**. À corriger en nommant juste, ce qui ne change pas la sortie.

## 4. La forme de l'option, et le prérequis qu'elle partage

`--print` ne prend pas de valeur, comme `--help` — que [config.rs](../src/config.rs) traite **hors**
d'`OPTIONS`, par un `help_requested(&args)` lu avant même de construire la configuration. Or
`--no-progress` attend exactement la même chose ([IDEAS.md](../IDEAS.md), *Renderer &
infrastructure*). Une fonction ad hoc est une exception ; trois sont un motif. **Le coût réel est
donc le drapeau sans valeur dans `OPTIONS`, et il se paie une fois pour les deux options.**

Le reste est minuscule : dans `main.rs`, `Parser::parse(&text)`, puis
`stage.visit(&mut PrintVisitor::new())`, puis retour. `Parser::parse` rend bien un `StageNode`, donc
`visit_stage` devient atteignable sans élargir aucune signature.

Une dernière chose à corriger en passant : le visiteur imprime lui-même, par `println!` depuis
`visit_stage`. Maintenant que `PrintVisitor::rendered` existe, **l'impression appartient à
l'appelant** — le visiteur rend du texte, `main` l'écrit. C'est ce qui rend la sortie testable et
redirigeable.

## 5. Le gain qui dépasse l'option

Rendre la sortie rechargeable ouvre une **propriété**, et c'est là que le sujet devient rentable :

```text
t' = print(parse(t))        →        print(parse(t')) == t'
```

L'idempotence de `print ∘ parse`. Elle ne demande qu'une comparaison de chaînes — utile, parce que
l'AST est fait d'objets de trait (`Box<dyn ObjectNode>`) et n'a donc pas d'égalité structurelle à
offrir. Elle s'applique à chaque fichier de `test_files/`, couvre toute la chaîne du langage — lexer
compris — et **aurait attrapé seule le changement d'ordre de visite de la CTM**, là où il a fallu
écrire l'assertion à la main.

Elle a un prix : elle demande les §3.2 et §3.3, et elle touche l'orthographe publique du langage,
`substraction` compris, dont le renommage est déjà une entrée ouverte. À décider à ce moment-là, pas
avant.

## 6. Ordre d'attaque

- [ ] Drapeau sans valeur dans `OPTIONS`, partagé avec `--no-progress` : `--help` cesse d'être un
      cas particulier traité à côté.
- [ ] Nettoyage du `PrintVisitor` : le `.ply` qui dégouline (§3.1), les liaisons de `visit_stage`
      (§3.3), l'impression remontée dans `main`.
- [ ] `--print`, et son entrée dans `usage()` disant qu'il imprime la description **telle que lue**.
- [ ] Sortie rechargeable et test d'idempotence (§5) — à décider, et à ne pas mêler aux trois
      précédents.

## 7. Ce que ça débloque

Rien, et c'est assumé : le sujet est indépendant, rien n'en dépend, d'où sa place dans la liste.
C'est un outil pour qui écrit des scènes et du code de chargement, plus un morceau de code mort
rendu vivant ou retiré.
