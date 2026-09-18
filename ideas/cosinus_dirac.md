# Le cosinus qu'un bsdf spéculaire divise, et comment s'en passer

Indexé depuis [IDEAS.md](../IDEAS.md). Non commencé. **À faire avec MIS**, pour la raison donnée au
§5 — pas avant, pas séparément.

## 1. Le défaut, qui n'est pas une erreur

[`materials::cancel_integrator_cosine`](../src/materials.rs) divise l'atténuation d'un matériau
spéculaire par |cos θᵢ|, parce que l'intégrateur va multiplier par |cos θᵢ| trois lignes plus loin
([path.rs:84](../src/integrators/path.rs#L84), [naive.rs:44](../src/integrators/naive.rs#L44)). Le
calcul est juste : le bsdf d'un miroir est une distribution δ, son intégrale tient déjà compte de
l'angle solide projeté, donc le cosinus que l'estimateur applique à toute direction échantillonnée
est surnuméraire. C'est aussi la convention de pbrt-v3, où `SpecularReflection::Sample_f` rend
`R/AbsCosTheta(wi)` avec `pdf = 1`.

Le reproche n'est donc pas la justesse, c'est la **couture** : le matériau encode la formule de son
appelant. Le 1/|cos θᵢ| n'a de sens que parce qu'on sait ce que l'intégrateur fait ensuite, ce qui est
le genre de dépendance que CLAUDE.md §2 tient fermée entre intégrateur et renderer.

La preuve est dans le test : `test_specular_cosine_cancels_out` doit **réimplémenter l'arithmétique de
l'intégrateur** (`attenuation · |cos θᵢ| / pdf`) pour vérifier un matériau. Un test qui rejoue le
calcul du client pour valider le fournisseur dit que la responsabilité est du mauvais côté.

Deux conséquences concrètes, en plus de l'argument de structure :

- **Une singularité cachée.** `attenuation` part vers l'infini quand |cos θᵢ| → 0. Rien n'explose
  aujourd'hui uniquement parce que les deux matériaux gardent la géométrie en amont
  ([metal.rs](../src/materials/metal.rs) écarte une direction sous l'horizon, `Dielectric` ne passe
  qu'une réflexion ou une réfraction). C'est une précondition qui ne se voit pas dans la signature.
- **Le discriminant est au mauvais niveau.** `Material::is_specular()` est une propriété du
  *matériau* ; elle sera fausse dès qu'un matériau aura un lobe spéculaire *et* un lobe diffus tirés
  au hasard — diélectrique rugueux, couches. Le caractère singulier appartient à *l'échantillon*.

## 2. La forme retenue : le discriminant porté par l'échantillon

C'est celle de pbrt-v4, dont le `BSDFSample` porte un jeu de drapeaux avec `IsSpecular()`.
`ScatterInfo` dit si sa densité est une δ ou une densité par unité d'angle solide ; l'intégrateur
branche dessus :

- densité ordinaire → `beta *= attenuation · |cos θᵢ| / pdf`, inchangé ;
- δ → `beta *= attenuation`, sans cosinus et sans division.

Les matériaux spéculaires rendent alors ρ tel quel. `cancel_integrator_cosine` disparaît, la
singularité avec elle, et le pdf reste disponible séparément — ce dont MIS a besoin.

**Tension avec CLAUDE.md §2**, à assumer explicitement : §2 demande qu'une nouvelle variante d'un
concept arrive par une implémentation de trait, « pas par un `match` ou un `enum` dans le code
appelant ». Ce n'en est pas une : une mesure est absolument continue ou singulière, il n'y a pas de
troisième cas et il n'y en aura jamais. Ce n'est pas un point d'extension déguisé, c'est un type somme
sur une dichotomie fermée — et cette dichotomie est *déjà* dans le code sous la forme du booléen
`is_specular()`, simplement au mauvais niveau et sans que l'intégrateur s'en serve pour le cosinus.

## 3. Deux fausses pistes, dont une piège

**Renvoyer le poids combiné.** L'intégrateur n'utilise jamais `attenuation` et `pdf` séparément : ses
deux seuls usages calculent `attenuation · |cos θᵢ| / pdf`. Faire renvoyer ce produit par `scatter`
supprimerait d'un coup la division, le cosinus et la fonction — et c'est la solution la plus propre
pour un simple path tracer. **Mais MIS demande le pdf du bsdf pour lui-même**, pour pondérer par
`p_bsdf / (p_bsdf + p_light)`. Écarté pour cette raison, et pour elle seule.

**Déplacer la fiction dans le pdf** — `attenuation = ρ` et `pdf = |cos θᵢ|`. Le produit tombe juste et
la division sort des matériaux. **C'est un piège** : un `pdf` qui vaut `|cos θᵢ|` n'est pas une
densité, et le premier consommateur qui le prendra au mot est MIS. Une atténuation visiblement
compensée est un mensonge qu'on voit ; un pdf faux est un mensonge qui contamine le calcul suivant.

## 4. Ce que ça coûte à la vérification

`(ρ/|c|)·|c|` n'est pas `ρ` en flottant. **L'image changera**, d'un ULP sur les surfaces
spéculaires — vraisemblablement invisible après quantification sur 0-255, mais pas prouvable par une
empreinte. Le contrôle par image identique au bit près, qui a servi tout au long de
`chore/shading_frame`, n'est pas disponible ici : il faut une comparaison d'un autre genre, et
« vraisemblablement invisible » n'est pas « identique ».

Pour mémoire, l'autre coût : le projet est un projet d'apprentissage, et le 1/|cos θ| est ce qu'un
lecteur trouve dans pbrt-v3, livre ouvert à côté du code. S'en écarter demande de dire *lequel* des
deux pbrt on suit. La réponse est v4, et elle doit être écrite dans le code.

## 5. Pourquoi avec MIS, et pas avant

`is_specular()` n'a **qu'un seul** consommateur, [path.rs:86](../src/integrators/path.rs#L86), pour le
garde `is_last_bounce_specular` — et c'est MIS qui fait tomber ce garde
([area_light.md §5](area_light.md)). MIS réécrit par ailleurs exactement la pondération de `beta` qui
est en cause ici, et introduit le besoin d'un pdf propre. Faire ce travail avant MIS, c'est toucher
deux fois au même endroit ; le faire avec lui, c'est presque gratuit.

## 6. Ordre d'attaque

- [ ] Le discriminant dans `ScatterInfo`, et les deux intégrateurs qui branchent dessus.
- [ ] `Metal` et `Dielectric` rendent ρ ; `cancel_integrator_cosine` et son test s'en vont.
- [ ] `Material::is_specular()` devient dérivable de l'échantillon, une fois que MIS a retiré le garde
      `is_last_bounce_specular` qui est son unique usage.
- [ ] Une comparaison d'image qui ne suppose pas l'égalité au bit près, puisqu'elle n'est plus vraie.
