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

- [ ] **RNG graine + samplers stratifiés** — indépendant. Deux choses en dépendent : la validation
      d'`AreaLight` et le balayage de `t_trav`, aucune des deux n'étant démontrable sans un rendu
      reproductible. Inventaire des tirages, décisions et ordre d'attaque dans
      [ideas/rng_graine.md](ideas/rng_graine.md).
- [ ] **`AreaLight`** — surfaces émissives enregistrées comme sources échantillonnables. Le plus grand
      écart au modèle physique du projet ; plan détaillé dans
      [ideas/area_light.md](ideas/area_light.md).
- [ ] **Production `light` dans la grammaire `.stage`** — dépend d'`AreaLight`, qui décide de ce que la
      production doit *ne pas* couvrir, et atterrit dans le même visiteur. Retire les lumières câblées
      du loader. Détail sous *Renderer & infrastructure*.
- [ ] **MIS** — dépend d'`AreaLight` : sans `pdf_li`, il n'y a rien à pondérer. Fait tomber le garde
      `is_last_bounce_specular` de l'intégrateur.
- [ ] **Roulette russe** — dépend de MIS, et corrige au passage la coupe prématurée de `path.rs:65`.
- [ ] **Abstraction `Film` + ordonnancement par tuiles** — indépendant, et déduplique les deux
      renderers. Détail sous *Renderer & infrastructure*.
- [ ] **Balayage de `t_trav`, et feuilles de maillage plus grosses** — dépend du sampler graine, pour
      la raison qui compte : élire la constante sur les seuls rayons primaires figerait un arbitrage
      mesuré sur un cinquième du problème ([ideas/cout_traversee_bvh.md](ideas/cout_traversee_bvh.md)).
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
      projet possède *sauf* les lumières, entrée ouverte ci-dessus.
- [x] Add support for triangle based geometry — `src/shapes/triangle_mesh/`. Reste les normales de
      shading, suivies sous *Justesse / robustesse*.
- [x] **Chantier BVH** — SAH de maillage corrigé, `intersect_p` descendu dans les formes, arbre de
      scène à plat, traversée ordonnée avec resserrement de l'intervalle, test de boîte inliné.
      Mesures et arbitrages dans [docs/mesures_bvh.md](docs/mesures_bvh.md).
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
      pour `dragon_vrip.ply` : [ideas/cout_traversee_bvh.md](ideas/cout_traversee_bvh.md). Bloqué sur
      le sampler graine, et la raison compte.
- [ ] **Le SAH binné n'est pas porté sur le BVH de scène**, étudié et garé :
      [ideas/sah_bvh_scene.md](ideas/sah_bvh_scene.md).
- [ ] **Chaque `intersect` de forme rend un `Vec<Intersection>` frais** (`IntersectionResult`), et
      `Transformed::intersect` en construit un second pour tenir les copies transformées. Une touche
      coûte donc une ou deux allocations tas lues une fois puis jetées. Les ratés sont gratuits —
      `Vec::new()` n'alloue pas avant le premier push. Pas dans la revue d'origine ; remarqué en
      prenant la référence de scène. C'est le coût par test que les compteurs ne peuvent pas voir, et
      la raison pour laquelle `intersect_p` vaut plus que son effet sur `object_tests` ne le suggère.

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

- [ ] **`unsafe` inutile** en [simple.rs:31-34](src/objects/simple.rs#L31) — un pointeur brut sert à
      lire `intersections[0]`, alors qu'`Intersection` est `Copy`. Suppose aussi que le premier
      élément est le plus proche ; mériterait d'assérer que tout `Intersectable` rend bien une liste
      triée par distance.
- [ ] **Les normales de maillage sont parsées et jamais utilisées** (avertissement de build) : pas de
      normales de shading interpolées, donc les maillages sont visiblement facettés. La normale
      géométrique est dérivée de `cross(dpdv, dpdu)` sur des UV par défaut, route détournée à
      l'orientation fragile.
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
- [ ] **La roulette russe est commentée** ([path.rs:93](src/integrators/path.rs#L93)) ; les chemins
      sont coupés net à `max_depth`, et la coupe de [path.rs:65](src/integrators/path.rs#L65) tombe
      *avant* l'échantillonnage de lumière du dernier sommet — perte d'énergie systématique.
- [ ] **Pas de tone mapping.** [`gamma_correct`](src/spectrum.rs#L21) est un `sqrt` (gamma 2,0, pas
      sRGB) et [`in_bound`](src/spectrum.rs#L134) écrête dur à 1,0 : toute la dynamique au-dessus de 1
      est jetée.
- [ ] **`Spectrum` est un triplet RGB sans espace de couleur déclaré** — ni primaires, ni point
      blanc. Le nom promet un rendu spectral qui n'existe pas.
- [ ] **Les lumières sont câblées dans `Loader::load_scene`** : une `PointLight` en (0, 2, 1) et un
      `BackgroundInfiniteLight`, ajoutés à toute scène quoi qu'elle dise. Un fichier `.stage` ne
      décrit donc pas son éclairage — il hérite de celui-là. Le travail de grammaire qui répare cela
      est sous *Renderer & infrastructure*.

## Renderer & infrastructure

- [ ] **Dispatch en tourniquet par pixel** dans [mt.rs](src/renderers/mt.rs) : ~480 000 messages de
      canal pour une image 800×600, aucun équilibrage de charge (l'ordonnancement est fixé d'avance,
      donc un thread héritant d'une région coûteuse retient toute l'image), la boucle principale
      tourne à vide sur un `try_recv` non bloquant une fois l'itérateur de pixels épuisé, et les
      canaux sont non bornés. [`Bounds2`](src/geom/bounds2.rs) sait déjà faire le pavage qui corrige
      les quatre.
- [ ] **~50 lignes dupliquées** entre [st.rs](src/renderers/st.rs) et [mt.rs](src/renderers/mt.rs) :
      `compute_pixel`, `Sampler2`, `image_write` sont identiques. Extraire `Film` (accumulation +
      écriture) et `Sampler` ; les deux renderers ne devraient alors différer que par
      l'ordonnancement.
- [ ] **Les rendus ne sont pas reproductibles.** `rand 0.3` par `thread_rng`, non graine ici, donc
      deux exécutions ne sont pas comparables — ce qui rend invérifiable tout changement de
      l'intégrateur. Analyse, structure et ordre d'attaque dans
      [ideas/rng_graine.md](ideas/rng_graine.md), qui corrige au passage deux affirmations de cette
      entrée : la cible est `rand 0.10`, et `rand` n'achète ni les samplers stratifiés ni Sobol.
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
      rendu passe par les lumières que [loader.rs](src/loader.rs) câble plutôt que par quoi que ce
      soit que le fichier de scène déclare.
- [ ] **La grammaire `.stage` ne sait pas décrire une lumière.** Elle dit tout ce que le projet
      possède — caméras, formes, matériaux, textures, transformations — sauf le seul concept sans
      lequel une scène ne s'affiche pas. Le travail est la chaîne habituelle, mot-clé du
      [lexer](src/loader/parser/lexer.rs) → [parser](src/loader/parser.rs) → nœud d'
      [AST](src/loader/ast.rs) → méthode de [`Visitor`](src/loader/visitors.rs) → les deux visiteurs
      (`PrintVisitor` doit refaire l'aller-retour, `SceneBuilderVisitor` doit appeler
      `Scene::add_light`). Quatre décisions à prendre avant d'écrire, dont trois ne sont pas
      évidentes :
      **(1) Où vit le nœud.** `SceneNode` porte `objects: Vec<Box<dyn ObjectNode>>` ; une lumière est
      membre de la scène et non d'un objet, donc un `lights: Vec<Box<dyn LightNode>>` frère est la
      place juste — pas un `object light`, qui la ferait passer par le chemin forme + matériau.
      **(2) Les lumières d'aire ne passent pas par cette production.** `diffuse_light` existe déjà
      comme *matériau*, et c'est la bonne route : une surface émissive se déclare en posant ce
      matériau sur un objet, et c'est le visiteur qui en tire l'`AreaLight`
      ([ideas/area_light.md](ideas/area_light.md) §4). La production `light` ne couvre donc que les
      lumières **sans géométrie** — `point`, `uniform_infinite`, `background_infinite` et son
      dégradé à deux couleurs. Deux syntaxes pour un même concept serait le piège à éviter.
      **(3) Une position se dit comme celle d'un objet.** `PointLight` prend un `Transform` ; la
      grammaire a déjà `transform { translate … }`, donc réutiliser ce bloc plutôt qu'inventer un
      `pos x y z` garde une seule façon de placer une chose dans la scène.
      **(4) La migration est une rupture, et il faut la vouloir.** Le jour où le loader n'ajoute plus
      rien, tout `.stage` sans bloc `light` rend du noir — ce qui est honnête, et demande de reprendre
      les fichiers de `test_files/` un par un. C'est aussi ce qui rend le témoin d'`AreaLight`
      démontrable : une scène éclairée par ce qu'elle déclare, et rien d'autre.
- [ ] **Le mot-clé CSG `substraction` est orthographié à la française** — la forme anglaise est
      `subtraction`, et le reste de la grammaire est en anglais. C'est dans la surface publique du
      langage de scène, donc le renommer casse les `.stage` existants : accepter la rupture, ou
      accepter les deux graphies le temps d'une transition.
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
