# IDEAS

Un sujet reste une ligne ici tant qu'une ligne suffit. Il prend son propre fichier dans
[ideas/](ideas/) dès qu'il porte une analyse — quelque chose qui vaut plus qu'une case à cocher — et
ce fichier-ci n'en garde alors que l'entrée d'index. `ideas/` n'est pas `docs/` : `docs/` décrit le
code tel qu'il est et doit rester digne de confiance, `ideas/` décrit ce qui n'est pas fait. Le
fichier d'un sujet disparaît quand le sujet atterrit — et ce qu'il a appris, s'il s'agit d'une mesure
ou d'un arbitrage sur le code tel qu'il est, passe dans `docs/`.

**Ce fichier est un index.** Une entrée cochée garde une ligne, pas son corps ; le raisonnement qui a
survécu au correctif est dans le fichier `docs/` indiqué.

**La liste qui suit est ordonnée, et cet ordre est celui du traitement envisagé.** Une dépendance s'y
dit en une incise ; quand elle contraint l'ordre, c'est l'ordre qui s'adapte. Les sections thématiques
plus bas portent le détail de chaque entrée — elles servent à retrouver un sujet, pas à savoir quoi
faire ensuite.

- [ ] **`AreaLight`** — surfaces émissives enregistrées comme sources échantillonnables. **Le plus
      grand écart au modèle physique du projet**, et son unique prérequis est acquis : la comparaison
      `naive` / `path` qui lui sert de preuve est désormais possible
      ([docs/eclairage_declare.md](docs/eclairage_declare.md) §4). Plan détaillé dans
      [ideas/area_light.md](ideas/area_light.md).
- [ ] **MIS** — dépend d'`AreaLight` : sans `pdf_li`, il n'y a rien à pondérer. Fait tomber le garde
      `is_last_bounce_specular` de l'intégrateur, et emporte avec lui
      [le cosinus qu'un bsdf spéculaire divise](ideas/cosinus_dirac.md), qui se règle dans le même
      geste et pas avant.
- [ ] **Roulette russe** — dépend de MIS, et corrige au passage la coupe prématurée de `path.rs:65`.
- [ ] **Éléments nommés dans la grammaire `.stage`** — déclarer une forme, un matériau, une texture
      ou un objet sous un nom, et le réutiliser plutôt que le recopier. **Indépendant du transport
      de lumière** : c'est un confort d'écriture des scènes, et c'est pourquoi il passe après MIS et
      sa roulette plutôt qu'avant. Sa seule dépendance est le chantier de la CTM, en tête de cette
      liste : sous une CTM, construire cuit le placement, donc la table des noms doit stocker des
      nœuds d'AST et non des objets bâtis — et c'est ce choix qui décide de tout le reste.
      [ideas/elements_nommes.md](ideas/elements_nommes.md)
- [ ] **Sampler stratifié** — indépendant, et débloqué : le chantier du RNG graine en a livré tous
      les prérequis, et de quoi le mesurer. **Rien n'en dépend**, d'où sa place ici plutôt qu'en
      tête. Principe, prérequis acquis et travail restant dans
      [ideas/sampler_stratifie.md](ideas/sampler_stratifie.md).
- [ ] **Abstraction `Film` + ordonnancement par tuiles** — indépendant, et déduplique les deux
      renderers. Détail sous *Renderer & infrastructure*.
- [ ] **`--print` — charger une scène, l'imprimer, sortir sans rendre.** Donne un appelant au
      [`PrintVisitor`](src/loader/visitors/print.rs), qui n'en a aucun : `visit_stage` est
      inatteignable et ses tests d'origine impriment sans rien affirmer. **Rien n'en dépend**, mais
      son prérequis est partagé — une option sans valeur, comme `--no-progress`, ce que `config.rs`
      ne sait pas encore porter dans `OPTIONS`. Trois défauts du visiteur à régler avant, dont un
      `.ply` relu et vidé ligne à ligne — 249 186 lignes pour une scène portant `bun_zipper.ply` —,
      et un gain qui dépasse l'option : l'idempotence de
      `print ∘ parse` comme test de toute la chaîne du langage.
      [ideas/print_stage.md](ideas/print_stage.md)
- [ ] **Balayage de `t_trav`, et feuilles de maillage plus grosses** — **débloqué** : les rayons
      secondaires sont désormais reproductibles, donc l'arbitrage peut être mesuré sur eux et pas
      sur le seul cinquième primaire ([ideas/cout_traversee_bvh.md](ideas/cout_traversee_bvh.md)).
- [ ] **Nested dielectrics** — un transmetteur dans un autre, et deux formes partageant une face avec
      des matériaux différents, sont tous deux rendus faux aujourd'hui. Dépend d'un prérequis interne :
      déplacer le décalage anti-acné de la position vers l'intervalle du rayon.
      [ideas/nested_dielectrics.md](ideas/nested_dielectrics.md)
- [ ] **Add cone volume** — indépendant, et petit.
- [ ] **`Rc` avec enveloppe `unsafe` au lieu d'`Arc`** pour passer aux threads
      ([article stackoverflow](https://stackoverflow.com/questions/63433718/how-to-freeze-an-rc-data-structure-and-send-it-across-threads)).
      Dépend d'une mesure : CLAUDE.md §3 demande que tout `unsafe` soit argumenté par un chiffre, donc
      le coût des compteurs atomiques doit être établi avant d'écrire la moindre ligne.
- [x] Add cylinder volume
- [x] Make BVH more generic
- [x] Add a scene from text file loader — `src/loader/`, et la grammaire `.stage` dit tout ce que le
      projet possède.
- [x] **Production `light` dans la grammaire `.stage`** — une scène est éclairée par ce qu'elle
      déclare, le chargeur n'ajoute plus rien, et la comparaison `naive` / `path` est possible.
      [docs/eclairage_declare.md](docs/eclairage_declare.md).
- [x] Add support for triangle based geometry — `src/shapes/triangle_mesh/`. Reste les normales de
      shading, suivies sous *Justesse / robustesse*.
- [x] **Chantier BVH** — SAH de maillage corrigé, `intersect_p` descendu dans les formes, arbre de
      scène à plat, traversée ordonnée avec resserrement de l'intervalle, test de boîte inliné.
      Mesures et arbitrages dans [docs/mesures_bvh.md](docs/mesures_bvh.md).
- [x] **Chantier du RNG graine** — un rendu répète son image, indépendamment du nombre de threads
      et du renderer choisi. `Sampler`, `IndependentSampler`, `--seed`, et `utils::random_double`
      supprimé. Contrat et clé de flux dans
      [docs/rendu_reproductible.md](docs/rendu_reproductible.md).
- [x] **Chantier de la CTM** — le chargeur tient la matrice de transformation courante et livre une
      géométrie **déjà placée** : `shapes::Transformed` place une forme, `objects::Transformed` est
      recentré sur l'instanciation, et `csg::Elem` a disparu. Image identique au bit près à chaque
      commit, et le placement descendu d'un étage se révèle plus rapide — les deux décorateurs, la
      mesure et son attribution sont dans
      [docs/mesures_placement.md](docs/mesures_placement.md).
- [x] **Bornes cachées à la construction — mesuré, chiffré, et écarté.** Le correctif marche et ne
      vaut pas son diff : un millième d'un aperçu. Le sujet sort de cette liste et garde son entrée
      sous *Accélérateurs* avec sa condition de réouverture ; le corpus de mesure y a gagné deux
      scènes. [docs/mesures_bvh.md](docs/mesures_bvh.md) §2.3.

---

# Défauts

Relevés lors d'une lecture complète de l'arbre au commit `1859a9e` (2026-07-28), sur la branche
`chore/revamp_bvh_for_trimesh`, plus ce que le chantier a trouvé en chemin. La case dit si l'entrée
tient toujours.

## Accélérateurs

- [ ] **`AABound::get_bounding_box` est un calcul, pas un accesseur** — et rien ne le cache.
      [`TriangleMesh`](src/shapes/triangle_mesh/triangle_mesh.rs) le recalcule en balayant **tous
      les sommets**, `Transformed` transforme huit coins, `Compound` replie sur ses enfants ;
      l'appel est récursif et arbitrairement coûteux. La construction du BVH de scène le rappelle
      O(n log² n) fois — **26 671 appels pour 445 primitives**, 60 par primitive.
      **Mesuré et écarté le 2026-08-20**, correctif écrit puis retiré : il gagne 1,2 ms sur une
      exécution de 1,02 s, soit un millième d'un aperçu et un cinquante-millième d'une image finie,
      contre un `subdivide` qui perd son `&mut self`. Le raisonnement complet, les tables et la
      condition de réouverture — plus de mille primitives dans une scène réelle — sont en
      [docs/mesures_bvh.md](docs/mesures_bvh.md) §2.3. **Ne pas rouvrir sans ce chiffre-là.**
- [ ] **Les feuilles de maillage tiennent un seul triangle**, ~2 nœuds par triangle, 110 Mo de nœuds
      pour `dragon_vrip.ply` : [ideas/cout_traversee_bvh.md](ideas/cout_traversee_bvh.md). Plus
      bloqué depuis que les rayons secondaires sont reproductibles.
- [ ] **Le SAH binné n'est pas porté sur le BVH de scène**, étudié et garé :
      [ideas/sah_bvh_scene.md](ideas/sah_bvh_scene.md).
- [ ] **Chaque `intersect` de forme rend un `Vec<Intersection>` frais** (`IntersectionResult`). Une
      touche coûte donc une allocation tas lue une fois puis jetée : `Simple` demande la liste à sa
      forme, y lit la plus proche et jette le reste. Les ratés sont gratuits — `Vec::new()` n'alloue
      pas avant le premier push. Les décorateurs de placement n'en ajoutent pas de second :
      [`shapes::Transformed`](src/shapes/transformed.rs) remplace chaque touche sur place dans le
      vecteur possédé de l'enfant, et le second vecteur d'
      [`objects::Transformed`](src/objects/transformed.rs) est sur une méthode que personne
      n'appelle (entrée sous *Renderer & infrastructure*). Restent les opérateurs CSG, qui
      reconstruisent une liste filtrée — inhérent à une opération ensembliste, non au placement. Pas
      dans la revue d'origine ; c'est le coût par test que les compteurs ne peuvent pas voir, et la
      raison pour laquelle `intersect_p` vaut plus que son effet sur `object_tests` ne le suggère.

Passés, corps dans [docs/mesures_bvh.md](docs/mesures_bvh.md) §3 :

- [x] Coût SAH calculé sur la mauvaise boîte — l'aire d'un bin au lieu de celle de l'union (§3.1).
- [x] Premier plan candidat dégénéré, subdivision arrêtée d'emblée (§3.1).
- [x] `evaluate_sah` code mort et bogué, devenu l'oracle de test `exhaustive_split_cost` (§3.1).
- [x] Partition comparant une position reconstruite là où le coût comptait des bins (§3.1).
- [x] Boîte de chaque nœud testée deux fois (§3.1).
- [x] `AABoundingBox::hit` recalculait trois réciproques par test de boîte (§3.1).
- [x] Un maillage vide faisait récurser `build_stats` dans un nœud inexistant (§3.1).
- [x] `query` clonait les primitives trouvées (§3.2).
- [x] L'arbre de scène était un arbre de pointeurs (§3.2).
- [x] Ni traversée ordonnée, ni resserrement de `far`, ni sortie anticipée (§3.2).
- [x] Axe de coupe tiré au hasard, donc build non reproductible et accélérateur non mesurable (§3.2).
- [x] `BVHNode::new` sur un vecteur vide récursait indéfiniment (§3.2).
- [x] Pas d'`intersect_p` au niveau de la scène (§3.2).
- [x] `intersect_p` n'atteignait pas l'intérieur des formes (§3.2).
- [x] `Plane` rapportait une boîte non bornée, `±f64::MAX` (§3.2).

## Justesse / robustesse

- [ ] **Les deux lumières infinies consomment en monde ce que `Pdf` promet en local.**
      [`Pdf`](src/pdfs.rs#L9) documente ses directions comme étant celles du repère de shading, et
      [uniform_infinite_light.rs:31](src/lights/uniform_infinite_light.rs#L31) comme
      [background_infinite_light.rs:34](src/lights/background_infinite_light.rs#L34) passent le
      résultat de `SpherePdf::generate` directement en `LightLiSample::wi`, que `path.rs` traite
      comme une direction monde. **Sans effet sur l'image** — la loi uniforme sur la sphère est
      invariante par rotation, donc les mêmes nombres sont valides dans les deux repères —, mais
      c'est le seul endroit du dépôt où la convention est violée à l'exécution, et il le restera
      tant que rien ne distingue les deux repères dans le type. Deux routes : convertir par
      `ShadingFrame`, ou reconnaître qu'une loi invariante par rotation n'appartient à aucun repère
      et le dire dans `SpherePdf`.
- [ ] **`SpherePdf` n'a pas de test de conservation d'énergie**, là où
      [`CosinePdf`](src/pdfs/cosine.rs#L37) et [`HemispherePdf`](src/pdfs/hemisphere.rs#L36) en ont
      un ; le §4 de CLAUDE.md en fait une obligation pour toute pdf. C'est aussi celle des trois dont
      la densité est la plus facile à contredire par mégarde, puisqu'elle est constante.
- [ ] **Les trois `Pdf::generate` partagent le même squelette (θ, φ) → direction** —
      [cosine.rs:16](src/pdfs/cosine.rs#L16), [hemisphere.rs:16](src/pdfs/hemisphere.rs#L16),
      [sphere.rs:16](src/pdfs/sphere.rs#L16) : `phi.cos() * r`, `phi.sin() * r`, `z = cos_theta`.
      Seule la loi en cos θ diffère, et c'est la seule chose qu'un lecteur cherche ; un
      `spherical_direction(sin_theta, cos_theta, phi)` ne laisserait que cela visible.
- [ ] **`unsafe` inutile** en [simple.rs:31-34](src/objects/simple.rs#L31) — un pointeur brut sert à
      lire `intersections[0]`, alors qu'`Intersection` est `Copy`. Suppose aussi que le premier
      élément est le plus proche ; mériterait d'assérer que tout `Intersectable` rend bien une liste
      triée par distance.
- [ ] **Les normales de maillage sont parsées et jamais utilisées** (avertissement de build) : pas de
      normales de shading interpolées, donc les maillages sont visiblement facettés. La normale
      géométrique est dérivée de `cross(dpdv, dpdu)` sur des UV par défaut, route détournée à
      l'orientation fragile.
- [ ] **Un maillage sans coordonnées de texture reçoit les *mêmes* trois coins pour tous ses
      triangles** — `DEFAULT_UV0..2` dans
      [triangle_mesh.rs](src/shapes/triangle_mesh/triangle_mesh.rs). Toute texture en (u, v) s'y
      répète donc à l'identique face par face et dessine la tessellation au lieu d'un motif :
      `checkerboard` sur le dragon en est la démonstration. Deux routes, non exclusives —
      paramétrer le maillage (dépliage, ou projection au chargement), ou n'y poser que des textures
      *solides*, fonction de p.
- [ ] **Un `.ply` à faces non triangulaires produit un maillage faux, en silence** —
      [`MeshBuilder::on_face_event`](src/loader/mesh_loader.rs#L60) empile les indices à plat sans
      trianguler, alors que `TriangleMesh` les relit par triplets : un quad devient un triangle et
      un morceau du suivant. Le même point n'accepte que `ListInt32`/`ListUInt32`, donc un
      `property list uchar ushort vertex_indices` rend un maillage vide plutôt qu'une erreur. Deux
      défauts du *consommateur* d'événements, pas du lecteur `.ply`, qui lui lit les deux
      correctement.
- [x] Les lumières à l'infini construisaient un testeur de visibilité dégénéré — le défaut le plus
      coûteux du chantier, trois ordres de grandeur ([docs/mesures_bvh.md](docs/mesures_bvh.md) §3.3
      et §2.2).
- [x] `AABoundingBox::new` gonflait chaque axe à 0,01 d'extension minimale, biaisant tout coût SAH ;
      le vrai défaut était dans `hit` ([docs/mesures_bvh.md](docs/mesures_bvh.md) §3.3, dérivation de
      la borne 2γ(3) dans [docs/arithmetique_flottante.md](docs/arithmetique_flottante.md) §4).

## Écarts au modèle physique

- [ ] **Pas de lumières d'aire — le plus grand écart.**
      [ideas/area_light.md](ideas/area_light.md) : `DiffuseLight` n'est qu'un matériau, aucun
      `AreaLight` n'est enregistré dans `Scene::lights`, donc **une surface émissive ne contribue à
      aucun éclairage indirect**. Ce fichier porte l'analyse et l'ordre d'attaque.
- [ ] **Pas de MIS.** `Light` n'a pas de `pdf_li`, donc NEE et échantillonnage de BSDF ne peuvent pas
      être pondérés l'un contre l'autre. Bloqué sur l'entrée ci-dessus, et c'est MIS qui fera tomber
      le garde `is_last_bounce_specular`.
- [ ] **Un matériau spéculaire encode la formule de son intégrateur** —
      [`cancel_integrator_cosine`](src/materials.rs) divise par |cos θᵢ| ce que l'intégrateur va
      multiplier par |cos θᵢ|. Le résultat est juste, et c'est la convention de pbrt-v3 ; la couture,
      elle, est percée, et la singularité en |cos θᵢ| → 0 n'est tenue que par la géométrie des deux
      appelants. Le correctif est un discriminant porté par l'échantillon et non par le matériau,
      comme le `BSDFSample` de pbrt-v4 ; à faire **avec MIS**, qui réécrit la même pondération et
      supprime l'unique usage d'`is_specular()`. Deux fausses pistes, dont un pdf falsifié qui
      contaminerait MIS, et le fait que l'image ne sera plus identique au bit près :
      [ideas/cosinus_dirac.md](ideas/cosinus_dirac.md).
- [ ] **La roulette russe est commentée** ([path.rs:93](src/integrators/path.rs#L93)) ; les chemins
      sont coupés net à `max_depth`, et la coupe de [path.rs:65](src/integrators/path.rs#L65) tombe
      *avant* l'échantillonnage de lumière du dernier sommet — perte d'énergie systématique.
- [ ] **Pas de tone mapping.** [`gamma_correct`](src/spectrum.rs#L21) est un `sqrt` (gamma 2,0, pas
      sRGB) et [`in_bound`](src/spectrum.rs#L134) écrête dur à 1,0 : toute la dynamique au-dessus de 1
      est jetée.
- [ ] **`Spectrum` est un triplet RGB sans espace de couleur déclaré** — ni primaires, ni point
      blanc. Le nom promet un rendu spectral qui n'existe pas.
- [x] Les lumières étaient câblées dans `Loader::load_scene` — un fichier `.stage` décrit désormais
      son éclairage, et le chargeur n'ajoute rien ([docs/eclairage_declare.md](docs/eclairage_declare.md)).

## Renderer & infrastructure

- [ ] **Dispatch en tourniquet par pixel** dans [mt.rs](src/renderers/mt.rs) : ~480 000 messages de
      canal pour une image 800×600, aucun équilibrage de charge (l'ordonnancement est fixé d'avance,
      donc un thread héritant d'une région coûteuse retient toute l'image), la boucle principale
      tourne à vide sur un `try_recv` non bloquant une fois l'itérateur de pixels épuisé, et les
      canaux sont non bornés. [`Bounds2`](src/geom/bounds2.rs) sait déjà faire le pavage qui corrige
      les quatre.
- [ ] **Lignes dupliquées** entre [st.rs](src/renderers/st.rs) et [mt.rs](src/renderers/mt.rs) :
      `compute_pixel` et `image_write` sont identiques. Les deux `Sampler2` en sont partis avec le
      chantier du RNG graine ; extraire `Film` (accumulation + écriture) réglerait le reste, et les
      deux renderers ne différeraient plus que par l'ordonnancement.
- [ ] **La barre de progression écrit dans un fichier ce qu'elle destine à un terminal** —
      [progress.rs](src/progress.rs) réécrit sa ligne avec un retour chariot et la colore en ANSI.
      Hors terminal, ni l'un ni l'autre n'est interprété : un rendu redirigé y laisse **une seule
      ligne** portant bout à bout les 121 états par lesquels la jauge est passée, séquences
      d'échappement comprises — 12 142 octets mesurés, et ce volume ne dépend pas de la taille de
      l'image, puisque la barre ne redessine que lorsque sa ligne change. Correctif retenu : une
      option `--no-progress`, qui dit ce que l'utilisateur veut, plutôt qu'une détection par
      `std::io::IsTerminal`, qui devine ce qu'il voudrait. Elle ne prend pas de valeur, donc elle
      pose la même question que `--help` : celle de l'option qui n'est pas une paire
      `--nom valeur`, aujourd'hui traitée hors de `OPTIONS` ([config.rs](src/config.rs)). Cette
      question est le prérequis commun de deux entrées — celle-ci et `--print`
      ([ideas/print_stage.md](ideas/print_stage.md) §4) —, donc elle se paie une fois pour les deux.
- [x] **Les rendus sont reproductibles.** Même scène, mêmes options, même image — et indépendamment
      du nombre de threads comme du renderer choisi. `utils::random_double` n'existe plus.
      [docs/rendu_reproductible.md](docs/rendu_reproductible.md).
- [ ] **`match config.integrator` est dupliqué 16 fois** — les 15 exemples plus
      [main.rs](src/main.rs) — et `match config.renderer` autant. Le §2 de CLAUDE.md demande qu'une
      nouvelle variante d'un concept arrive par une implémentation de trait, « pas par un `match` ou
      un `enum` dans le code appelant » ; c'est ce `match`, et c'est pourquoi ajouter `NAIVE` a cassé
      seize fichiers d'un coup. Le correctif est une fabrique à côté de chaque enum :
      `Type::build(max_depth)` dans [integrators.rs](src/integrators.rs), `Type::render_fn()` dans
      [renderers.rs](src/renderers.rs). Prendre `max_depth` plutôt que `&Config` — `config.rs` dépend
      déjà d'`integrators::Type`, et passer `&Config` fermerait le cycle.
- [ ] **Aucun exemple ne porte plus de surface émissive**, `cornell_box.rs` ayant été retiré. Le
      témoin visuel de l'`AreaLight` manquante est désormais `test_files/cornell_box.stage`, dont le
      panneau émissif n'éclaire rien : la scène est éclairée par les deux `light` qu'elle déclare, et
      le jour où elle ne déclarera plus qu'un panneau, elle sera noire tant qu'`AreaLight` n'existe
      pas. C'est ce que ce témoin doit montrer.
- [x] **La grammaire `.stage` sait décrire une lumière** — `light point`, `light uniform_infinite`,
      `light background_infinite`, frères des objets dans le bloc `scene`. Les lumières d'aire n'y
      passent pas et se disent toujours par le matériau `diffuse_light`. Les quatre décisions de
      conception, la mesure qui prouve la migration et ce que la production rend vérifiable sont dans
      [docs/eclairage_declare.md](docs/eclairage_declare.md).
- [ ] **Les éléments d'une CSG arrivent dans l'ordre inverse de leur déclaration.**
      `SceneBuilderVisitor::pop_csg_elems` les dépile, donc le dernier `elem` écrit devient
      `elements[0]`. Sans effet sur une union ou une intersection, qui sont commutatives ; décisif
      pour `csg substraction`, dont le premier élément est la base dont les autres sont retirés :
      `csg substraction { elem A … elem B … }` retire donc **A de B**. Les constructions
      programmatiques d'`examples/` passent leurs éléments dans l'ordre de lecture et retirent B de
      A : la grammaire et les exemples ne disent pas la même chose. L'entrée de survol de
      [editors/vscode/src/grammar.js](editors/vscode/src/grammar.js) annonce, elle, « le **premier**
      `elem` moins tous les suivants » : c'est le comportement voulu, pas celui qu'on obtient, et le
      correctif fera coïncider les trois. **Scène témoin** :
      [test_files/csg_sub_cube_sphere.stage](test_files/csg_sub_cube_sphere.stage), seule scène de
      `test_files/` à employer `substraction`, et qui écrit donc la sphère avant le cube pour obtenir
      *cube moins sphère* — l'ordre inverse de celui que la grammaire documente, et de celui de
      [examples/csg_substraction_cube_sphere.rs](examples/csg_substraction_cube_sphere.rs) qui rend la
      même forme. Correctif d'une ligne, mais c'est une rupture de comportement : à faire avec le
      renommage ci-dessous, qui casse déjà ces fichiers, et en retournant les deux `elem` de la scène
      témoin dans le même commit — sans quoi elle rendra *sphère moins cube*, une boule mordue par
      six faces plates au lieu d'un cube évidé.
- [ ] **Le mot-clé CSG `substraction` est orthographié à la française** — la forme anglaise est
      `subtraction`, et le reste de la grammaire est en anglais. C'est dans la surface publique du
      langage de scène, donc le renommer casse les `.stage` existants : accepter la rupture, ou
      accepter les deux graphies le temps d'une transition. Le renommage traverse aussi
      [editors/vscode/](editors/vscode/) — coloration et entrée de survol —,
      [test_files/csg_sub_cube_sphere.stage](test_files/csg_sub_cube_sphere.stage), qui est le seul
      `.stage` à écrire le mot, et
      [tests/vscode_grammar_sync.rs](tests/vscode_grammar_sync.rs) échoue tant que ce n'est pas fait.
      Le message du parseur, lui, réclame déjà la graphie anglaise qu'il refuse
      ([parser.rs:209](src/loader/parser.rs#L209)) : c'est par lui que la dette se fait sentir.
- [ ] **Deux des trois méthodes d'`Intersectable` sont inatteignables à l'étage objet.** Seul
      `intersect_p` y a une racine : [scene.rs:166](src/scene.rs#L166) et
      [scene.rs:182](src/scene.rs#L182) le posent aux primitives pour les rayons d'ombre. La liste
      de touches d'un objet n'en a aucune — `Compound` l'appelle sur ses enfants,
      `objects::Transformed` sur le sien, le `Wrapper` de [scene.rs](src/scene.rs#L21) la relaie, et
      aucun intégrateur ne descend là : le chemin chaud est `Scene::find_nearest` →
      `Object::intersect` → `Simple` → la forme. `contain_point` non plus, dont le doc-comment
      d'[`Intersectable`](src/geom/intersectable.rs#L51) dit que les opérateurs CSG sont les seuls
      usages — et ceux-là travaillent sur des formes. Seule la borne `Object: Intersectable`
      maintient les deux en vie, et fait écrire `intersect` trois fois. La question n'est donc pas le
      coût de ces implémentations mais la couture : un `Object` a besoin d'une AABB, d'une
      `Interaction` la plus proche et d'un test d'occultation, pas d'une liste de touches ni d'un
      prédicat d'intérieur. `BVH<T>` ne contraint rien ici, il n'exige que `T: AABound`.
- [ ] Poids mort : `src/_keep.rs` et `src/shapes/triangle.cpp` ne sont pas compilés ;
      `integrators/whitted.rs` ne compile plus et est commenté hors du module ; `crossbeam` est
      toujours déclaré dans `Cargo.toml` sans être utilisé (`thread::scope` l'a remplacé) ; le build
      émet 24 avertissements.
- [ ] `edition = "2018"` dans `Cargo.toml` contre `edition = "2021"` dans `rustfmt.toml`.
- [x] Le répertoire `examples/` ne compilait plus — 16 erreurs, toutes dues au `match
      config.integrator` antérieur à la variante `NAIVE`. Élagué puis corrigé ; `cornell_box.stage`
      avait dérivé en vitrine de matériaux, donc la boîte canonique a été portée dans
      `test_files/cornell_box_canonical.stage` (deux blocs lambertiens blancs, caméra en z = 800,
      émission 15,0 ; demande `--fov 60 --far 2000`).
- [x] Quinze exemples n'enregistraient aucune lumière et rendaient du noir pur — sous `PATH`, un
      `Scene::lights` vide offre trois chemins indépendants vers zéro. Chacun ajoute désormais le
      `BackgroundInfiniteLight` de « Ray Tracing in One Weekend » ; [csg_bowl.rs](examples/csg_bowl.rs)
      en porte la dérivation complète, y compris l'écart que cela coûte — `sample_li` tire dans un
      `SpherePdf` uniforme sur toute la sphère, donc la moitié des échantillons tombent sous
      l'horizon. Non biaisé, mais la variance se paie.
