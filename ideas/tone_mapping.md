# Le tone mapping

Aujourd'hui, la radiance d'un pixel devient un octet en deux étapes, dans les deux renderers
([st.rs](../src/renderers/st.rs), [mt.rs](../src/renderers/mt.rs)) : `gamma_correct`, une racine
carrée, puis `to_rgb`, qui écrête chaque canal à [0, 1]. Tout ce qui dépasse 1 est perdu.

## Ce que l'écrêtage coûte

L'écrêtage se fait **canal par canal**, et ne garde donc pas les rapports entre canaux. Une source
de radiance (6,4, 12, 0), un vert-jaune, s'affiche (1, 1, 0), un jaune pur. Toute source colorée
plus brillante que 1 dérive ainsi vers l'un des six coins saturés du cube RGB. Les témoins :

- `test_files/sphere_ring_strips.stage`, dont le panneau `6 30 0`, vert, s'affiche jaune, et dont
  les panneaux bleu-violet s'affichent magenta, alors que le sol qu'ils éclairent, moins lumineux,
  montre leur vraie teinte ;
- `test_files/dragon_dome.stage`, qui contourne le défaut : ses lampes ne dépassent pas 1 par
  canal, ce qui laisse voir leur teinte, et ce sont deux sources blanches qui éclairent la scène.
  Des lampes assez brillantes pour éclairer le dragon seules s'afficheraient aux coins saturés du
  cube, et la lumière blanche qui les remplace délave les couleurs qu'elles portent sur le sol et
  le dragon.

## Ce qui est envisagé

Un module `tone_mappers`, selon l'idiome du projet : un trait `ToneMapper`
(`fn map(&self, radiance: &Spectrum) -> Spectrum`), une implémentation par fichier, et une option
`--tone_map` pour choisir. Le *tone mapping* s'applique à la radiance linéaire moyennée du pixel,
avant l'encodage gamma. Les deux renderers devraient passer par une seule fonction pour convertir
un pixel, pour ne pas pouvoir diverger.

- **`clamp`, par défaut** : l'écrêtage actuel, nommé. Écrêter avant la racine carrée ou après donne
  les mêmes octets, puisque la racine est croissante et laisse 0 et 1 en place. Une valeur
  négative ou NaN finit à 0 dans les deux cas. Le défaut ne change donc aucun pixel, et aucune
  mesure de `docs/` n'est à refaire, puisque [`image_stats`](../src/bin/image_stats.rs) lit les
  images produites.
- **`reinhard`** : l'opérateur global de Reinhard et al., *Photographic Tone Reproduction for
  Digital Images*, SIGGRAPH 2002, éq. (4), L_d = L / (1 + L). Il est croissant, vaut l'identité au
  voisinage de 0 et ne dépasse jamais 1. Appliqué à chaque canal séparément, il ne garde pas les
  rapports : (6,4, 12, 0) donne (0,865, 0,923, 0), encore jaune. Reinhard l'applique à un scalaire
  `m`, puis divise la couleur par ce qu'il a fait à ce scalaire : c_d = c / (1 + m). Les rapports
  r : g : b sont alors gardés.

**Le choix de `m` est la seule vraie décision.** Reinhard prend la luminance
Y = 0,2126·r + 0,7152·g + 0,0722·b. Un bleu saturé (0, 0, 12) a alors Y = 0,87 et ressort à
(0, 0, 6,4) : au-dessus de 1, donc l'écrêtage final défait ce que la division a gardé. Avec le
canal le plus fort, m = max(r, g, b), le résultat vaut au plus m / (1 + m) < 1, et tous les rapports
survivent jusqu'à l'image. Le prix est que `m` n'est pas une grandeur perceptive : un bleu pur et un
blanc de même `m` sont comprimés de la même façon, bien que le bleu paraisse plus sombre. C'est un
écart à l'article, à documenter comme tel.

Les tests qui en font partie : `clamp` produit les mêmes octets que l'écrêtage actuel, y compris
pour une valeur négative, infinie ou NaN ; `reinhard` suit L/(1+L) sur le canal le plus fort
(0 ↦ 0, 1 ↦ 1/2, presque l'identité près de 0), garde les rapports entre canaux, et ne laisse aucun
canal atteindre 1.

## Ce qui reste hors du sujet

Le gamma : `gamma_correct` est un gamma 2,0, pas la courbe sRGB. C'est indépendant du choix de
l'opérateur, mais cela se règle au même endroit.
