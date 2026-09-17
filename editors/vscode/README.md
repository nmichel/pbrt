# Support VS Code du langage `.stage`

Coloration syntaxique et aide contextuelle pour les fichiers de scène du projet. L'extension ne
*parse* rien : elle décrit. Le seul lecteur de `.stage` reste [le parser Rust](../../src/loader/parser.rs).

## Ce qu'elle apporte

**La coloration** suit le lexer au plus près, y compris dans ses refus. Trois choses apparaissent en
rouge, et ce sont précisément les trois façons dont le chargement échoue aujourd'hui sans un mot
utile :

- un mot inconnu — aucune production n'accepte un identifiant, donc `Sphere` ou `subtraction` est
  toujours une erreur ;
- `[`, `]`, `(`, `)` — lexés, mais fatals au parser ;
- tout autre caractère — une virgule, un `=`, un `1e-3`, un `.5`, une chaîne vide : le lexer s'arrête
  dessus **en silence** et le fichier est tronqué là.

**Le survol** donne la signature d'une production, ce qu'elle fait, ses paramètres et un exemple,
avec un lien vers le code qui en décide. Il répond aux pièges d'un langage entièrement positionnel :
`metal 0.3 color 1 1 1` met le flou avant la texture, `rectangle 555 555` mesure 555 et non 1110,
`aabox 165 330 165` donne des dimensions totales, `substraction` retire du **premier** `elem`.
Sur un nombre qui suit un `rotate_*`, le survol le convertit en degrés — les angles du langage sont
en radians.

**La complétion** propose ce que la grammaire attend à cet endroit : les sept formes après `simple`,
les quatre matériaux une fois la forme complète, les quatre étapes dans un bloc `transform`. Les
constructions à blocs s'insèrent comme squelettes.

## Installation

```sh
ln -s "$PWD/editors/vscode" ~/.vscode/extensions/pbrt-stage
```

depuis la racine du dépôt, puis `Cmd+Shift+P` → **Developer: Reload Window**.

Le lien symbolique suffit : il n'y a ni dépendance, ni `node_modules`, ni étape de compilation — du
JavaScript lu tel quel par l'hôte. Une modification de `src/*.js` prend effet au rechargement de la
fenêtre ; une modification de `syntaxes/stage.tmLanguage.json` **exige** ce rechargement, une
grammaire TextMate n'étant relue qu'au démarrage.

## Vérifier

1. Ouvrir [cornell_box_canonical.stage](../../test_files/cornell_box_canonical.stage), la scène qui
   couvre le plus de constructions, et [cube_mesh.stage](../../test_files/cube_mesh.stage) pour
   `mesh … reverse`, `checkerboard` et `dielectric`.
2. `Cmd+Shift+P` → **Developer: Inspect Editor Tokens and Scopes** : `rectangle` porte
   `support.type.shape.stage`, `-3.14159` porte `constant.numeric.stage`.
3. Taper `foo`, `,` puis `[` dans un brouillon : les trois doivent virer au rouge.
4. Survoler `rotate_y`, puis le nombre qui le suit.
5. `Ctrl+Espace` après `object `, puis à l'intérieur d'un `transform {`.

## Limites assumées

La complétion compte des jetons, elle ne construit pas d'arbre. Elle ne se trompe pas sur les cas
courants, mais elle perd le fil dès que le texte au-dessus du curseur est déjà invalide, et elle ne
sait pas toujours à quelle profondeur d'imbrication un `object transformed` se referme. Dans ce cas
elle propose **tous** les mots-clés plutôt que de deviner, chacun avec sa documentation. Quand elle
attend un nombre, elle ne propose rien : c'est le signal qu'aucun mot-clé n'est admis là.

Aucun diagnostic n'est publié. Souligner les vraies erreurs de syntaxe supposerait de rejouer le
parser, donc de convertir d'abord ses `panic!` en erreurs portant une ligne et une colonne — un
chantier Rust à part entière.

## Rester en phase avec le lexer

Les mots-clés sont écrits trois fois : dans [lexer.rs](../../src/loader/parser/lexer.rs), qui fait
foi, dans `syntaxes/stage.tmLanguage.json` et dans `src/grammar.js`.
[tests/vscode_grammar_sync.rs](../../tests/vscode_grammar_sync.rs) compare les trois listes et fait
échouer `cargo test` dès qu'elles divergent — c'est ce qui force un changement de grammaire à
traverser aussi l'éditeur. Le test lit les trois fichiers comme du texte, donc chacun doit garder la
forme qu'il y scanne : un bras `"mot" => Token::KW…` par mot-clé, des règles de coloration écrites
`\\b(?:a|b|c)\\b`, et une clé entre guillemets par entrée de la table.
