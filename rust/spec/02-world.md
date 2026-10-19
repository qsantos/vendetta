# 02 — World model and simulation loop

Scope: world data structures, world generation (random and from map files),
the tick loop and its ordering, the fief/building/resource economy, treasures,
decorations, scenarios and their scripts, victory/defeat, the per-player message
log, and a catalogue of `Rnd` usage.

Main sources: `ClsJeu`, `ClsJeuMonde`, `ClsJeuPartie`, `ModGenererMonde`,
`ClsPlaceObject`, `ClsJeuFief`, `ClsJeuBatiment`, `ClsJeuRessources`,
`ClsJeuDecors`/`ModDecors`, `ClsJeuObjet`, `ClsJeuTresors`/`ModTresors`,
`ClsJeuCartes`, `ClsJeuMessages`, `ClsJeuStastistiques`, `ModMain`,
`FrmMoteur2D` (logic part), `ModScenario`. The spec also cites the parts of
`ClsJeuPerso`, `ModIA`, `ModCombat`, `ClsJeuParam` and `ClsInterface` that the
world loop calls. Characters, combat, AI and parameter loading are covered in
detail by other spec files. This file only gives what is needed to order and
drive them.

Citations are `File:line` in the original VB6 sources (ISO-8859-1, CRLF,
1-based lines).

---

## 0. VB6 numeric semantics that matter here

A Rust port must reproduce these or the results drift:

| VB construct | Semantics |
|---|---|
| `LongVar = double_expr`, `IntVar = …`, `CLng(x)`, `CInt(x)` | **round half to even** (banker's rounding), e.g. 0.5→0, 1.5→2, 2.5→2, -0.5→0 |
| `Int(x)` | floor (towards −∞) |
| `a / b` | always floating-point division, even on integers |
| `a Mod b` | both operands are first **rounded to integers** (banker's), then the integer remainder is taken |
| `And` / `Or` | **not short-circuit**: both sides are always evaluated. This matters for `Rnd` calls inside conditions (each `Rnd` in the condition is always drawn) |
| `IIf(c, a, b)` | evaluates **both** `a` and `b` (it matters when one contains `Rnd`) |
| `For i = a To e` | `e` is evaluated once, at loop entry. When `e` is a Double and `i` is a Long/Integer, `e` is coerced to the counter type (banker's rounding); see the treasure generator (§5.2). *Uncertain; see §8.* |
| `Single` | IEEE float32. Accumulators declared `Single` (day clock, interface 1-s counter) must use f32 |
| `Collection` | 1-based. `Remove i` shifts the later items down |
| `ReDim A(n)` | creates indices `0..=n`, i.e. **n+1 elements** |

`Rnd` returns a float in `[0,1)` (VB6's 24-bit LCG:
`s = (s*1140671485 + 12820163) mod 2^24; return s/2^24`). The game calls
`Randomize` with no argument (time seed) once in `Main`
(`ModMain.bas:142`) and again at the start of every game in `Commencer`
(`ModMain.bas:302`). The commented-out `Randomize 12` at `ModMain.bas:299`
shows that a fixed seed was once considered for network play. Original runs
cannot be reproduced exactly, so the port may use any seeded PRNG, as long as it
keeps the *draw structure* described below (how many draws, under which
conditions, and the rounding that maps each draw to an integer).

---

## 1. World data structures

### 1.1 Global world state (`ModMain.bas:48-121`)

```
Partie: ClsJeuPartie           game mode/status
Monde: ClsJeuMonde             terrain grid
Cartes: ClsJeuCartes           map-file loader
Fiefs(): ClsJeuFief            0..UBound, one per fief (can grow at runtime, §4.7)
Chateaux(): ClsJeuBatiment     castles, same index as Fiefs
Persos(): ClsJeuPerso          all characters (villagers, monsters, invocations, projectiles)
Maisons(): ClsJeuBatiment      one house slot PER CHARACTER, Maisons(i) belongs to Persos(i)
Ressources(): ClsJeuRessources harvestable resource nodes on the map
Tresors: Collection<ClsJeuTresors>   (ModTresors.bas:20) ground treasures, 1-based
Decors:  Collection<ClsJeuDecors>    (ModDecors.bas:20) decorations, 1-based
Effets:  Collection<ClsJeuEffet>     visual effects (ModEffets.bas:21), cosmetic
Victoire(), Defaite(): Boolean per local player
NoPerso, Nopersos(): indices of the locally controlled characters
Jeu: ClsJeu                    rules, speed, pause, chrono, victory flags
Messages(): ClsJeuMessages     per local player message log
Stastistiques: ClsJeuStastistiques
```

Key invariant: **`Maisons(i)` is the house of `Persos(i)`**, so a character
owns at most one building. `Maisons(i).Numero = i` (set wherever the persos
are created, e.g. `ModMain.bas:446`). Castles keep `Numero = 0` (default),
except when loaded from a save.

### 1.2 `ClsJeu` (`ClsJeu.cls`)

Loaded from `Donnees/Jeu.ini` (`ClsJeu.cls:68-92`):

| Field | ini | default value | Meaning |
|---|---|---|---|
| `Vitesse` | `Vitesse` | 10 (min 5, max 30) | **ticks per real second** |
| `FPS` | `FPS` | 60 (5..60) | render rate |
| `AffichageRapide` | | 1 | render every loop iteration |
| `Regles(0)` RegleInvocationsMultiples | `Regle1` | 50 | |
| `Regles(1)` RegleTempsRechargementObjets | `Regle2` | 1 | |
| `Regles(2)` ReglePathfinding | `Regle3` | 1 (0 no, 1 yes, 2 advanced) | |
| `Regles(3)` ModeConfrontation | `Regle4` | 1 | only used in network confrontation |
| `EnPause` | | | when true, no ticks are run |
| `Campagne` | | | set by the UI (`FrmParam.frm:3194`) |
| `Chrono`, `IntituleChrono` | | | script-driven countdown display (§6.4) |
| `VictoireStandard`, `DefaiteStandard` | | | enable the built-in victory/defeat test (§6.3) |
| `Confrontation` | | | network confrontation mode |

`Init` (`ClsJeu.cls:94-100`): `Chrono=0, IntituleChrono="", EnPause=False,
VictoireStandard=DefaiteStandard=True`.

- `Duree = DureePartie / Vitesse` gives the elapsed game seconds (`ClsJeu.cls:254`).
- `Definir_Chrono` formats `Int(Chrono/Vitesse)` seconds as `m:ss"s"` or `ss"s"`
  (`ClsJeu.cls:265-276`).
- `Definir_Victoire`/`Definir_Defaite` let scripts set `Victoire(0)`/`Defaite(0)`.
  On a false→true transition they also play music 5 or 6 (`ClsJeu.cls:235-252`).

### 1.3 `ClsJeuPartie` (`ClsJeuPartie.cls`)

- `IndiceMode`: 0 Campaign, 1 Custom world ("NouveauMonde"), 2 Battlefield,
  3 Scenario, 4 Saved game, 5 Confrontation (names in `Partie.ini`).
- `RessourcesDeDepart`: index into `RessourcesDeparts`.
- `EnCours`: 0 waiting, 1 loading, 2 running (set to 2 just before the loop,
  `FrmMoteur2D.frm:420`, and to 0 at the end, `:1557`).

### 1.4 Terrain grid: `ClsJeuMonde` (`ClsJeuMonde.cls`)

Loaded from `Donnees/<lang>/Terrain.ini` (`:75-119`). English and French files
are identical apart from names.

```
LargeurCase = HauteurCase = 16        pixel size of a cell
Largeur, Hauteur: Long                map size in CELLS
TerrainCategorie(): Long              terrain type per cell (gameplay)
TerrainApparence(): Long              0..15 edge-shape tile index (rendering only)
TypeTerrain: Integer                  index of the world type (Mondes) used for roads
Diagonale = Sqr((Largeur*16)^2 + (Hauteur*16)^2)   (Long, rounded)
```

**Array size and indexing.** Both arrays are allocated with
`ReDim X((Largeur+1)*(Hauteur+1))` (`:129-130`), i.e. `(L+1)(H+1)+1` elements.
The **linear index uses stride `Largeur`** (not `Largeur+1`):

```
TerrainIndice(X, Y) = CLng(X / 16) + CLng(Y / 16) * Largeur      (:498-503)
```

X and Y are pixel coordinates.

- Banker's rounding gives a **pixel→cell mapping that alternates between 17- and
  15-pixel-wide cells**. Pixels 0..8 → cell 0, 9..23 → cell 1, 24..40 → cell 2,
  41..55 → cell 3, and so on. Even cells are 17 px wide and odd cells 15 px.
  The renderer, however, draws cell `c` at pixels `[16c, 16c+16)`
  (`ClsAffTerrain.cls:307`). This half-cell offset between logic and rendering
  is part of the original behaviour.
- A cell with x = Largeur aliases cell (0, y+1). Rows wrap: the left
  neighbour of x=0 is the last cell of the previous row.
- Out-of-range indices are handled per accessor (see below).

Terrain types (`Terrain.ini`, `NombreTypesTerrains=11`, so **12 types, 0..11**).
`Effet` is stored as `Val(Effet)-1`; a missing key gives -1.

| idx | Name (EN) | CoefVitesse | Inconstructible | Effet | Animation | ImgPersoDecalageY |
|---|---|---|---|---|---|---|
| 0 | Plain | 1 | | -1 | | |
| 1 | Herbs | 1 | | -1 | | |
| 2 | Ground | 1 | | -1 | | |
| 3 | Desert | 0.75 | | -1 | | |
| 4 | Mountain | 0.5 | 1 | -1 | | |
| 5 | Road | 1.25 | | -1 | | |
| 6 | Wall1 | 0.75 | 1 | -1 | | |
| 7 | Wall2 | 0.2 | 1 | -1 | | |
| 8 | Wall3 | 0.2 | 1 | -1 | | |
| 9 | Wall4 | 0.2 | 1 | -1 | | |
| 10 | Snow | 0.75 | | -1 | | |
| 11 | Water | 0.5 | 1 | 158 | 1 | 12 |

World types (`[Monde_n]`, `NombreTypesMondes=5`). `SurfaceTerrain(j)` is 0 for
every key not listed. `Relief` is the probability that a cell gets a seed.

| idx | Name | Relief | SurfaceTerrain (non-zero) | Route |
|---|---|---|---|---|
| 0 | Classic | 0.2 | 2:13 | 0 (no roads) |
| 1 | Plains | 0.1 | 1:13, 2:9, 4:6, 11:11 | 5 |
| 2 | Deserted | 0.7 | 3:9, 11:4 | 2 |
| 3 | Mountainous | 0.08 | 1:8, 2:8, 3:8, 4:18, 11:6 | 5 |
| 4 | Large lakes | 0.2 | 1:13, 2:9, 4:5, 11:13 | 5 |

Accessors:

- `Definir_Terrain(idx)` (private, `:506`): returns `TerrainCategorie(idx)` if
  `0 <= idx <= UBound`, else 0.
- `Constructible(X, Y, l=0, H=0)` (`:416-445`):
  - Point case: `t = TerrainIndice(X,Y)`. The result is
    `0 < t < UBound(TerrainCategorie) && !Terrains(cat(t)).Inconstructible`.
    **Cell index 0 and the last index are never constructible.**
  - Area case: true iff every sample point `(X+i, Y+j)` is constructible, for
    `i = 0, 16, 32, … <= l` and `j = 0, 16, … <= H`. Sampling stops at the first
    failure, so the far edge is only sampled when it falls on a multiple of 16.
- `CoefVitesse(Perso, X, Y)` (`:447-464`): if `0 < t < UBound`, it returns the
  type's `CoefVitesse`. **Side effect:** if the type has `Effet >= 0` and
  `Perso.Definir_Resistance(Objet_AttaqueResistance(Effet)) >
  Parametres.ResistanceMinimum`, it calls
  `Perso.EffetTemp_Ajouter(Effet, Objet_Duree(Effet), Objet_Duree(Effet))`.
  This is how water applies its effect. Outside that range it returns 1.
- `CoefVitesseSimple(X, Y)` is the same lookup without the side effect.
- `NombreCasesConstructibles` (`:549-558`) counts the cells over the whole
  array (including the aliasing tail) whose type is not Inconstructible.
- `Reformer_Terrain` (`:327-414`): see §2.3.

### 1.5 Fief: `ClsJeuFief` (`ClsJeuFief.cls`)

```
Numero (private), Nom, NumeroEquipe (team), TypePeuple (private, set via Definir_TypePeuple, also sets Nom)
Epoque: Integer (0-based age)
ChangerEpoque: Boolean            an age advance is in progress
TempsChangementEpoque: Long       ticks remaining
NombreCitoyens, NombreMorts: Long
Empereur: Long                    perso index of the ruler, or -1
Argent: Double                    castle repair fund
Ressources(0..NombreRessources-1): Double   castle stock (market)
Marches: Collection               house indices of built markets usable by this fief
```

Constants (`:33-34`): `QuantiteVieParArgent = 0.2` (HP per gold) and
`VitesseTransaction = 0.02/4/4 = 0.00125`.

`Init(i)` sets `Numero=i`, sizes `Ressources`, and sets `Empereur=-1`
(`:61-65`). The `Definir_ressources` setter clamps negative values to 0
(`:222-227`).

### 1.6 Building: `ClsJeuBatiment` (`ClsJeuBatiment.cls`)

The same class is used for castles (`Chateaux`) and houses (`Maisons`).

```
Numero (owner perso index for houses), PositionX/Y: Long (pixels, top-left), Largeur, Hauteur
EntreePositionX/Y, Construit, EnConstruction
Vie: Double, RecuperationVie: Double (pending heal), MaxVie: Long
Argent: Double (shop till), Stock (private Double), MaxStock: Double
StockObjets(0..NombreStockObjet): Double, ObjetsInventaire(0..PersosNombreObjetsInventaireMaison-1): Integer (-1 = empty)
Magasin, Fabrique: Boolean, Service: Integer, Marche: Integer
TypeBatiment: Integer (index into Parametres batiments), Tir: Boolean
NumeroAnimation (cosmetic)
```

Constants (`:32-34`): `CoefDepartVie = 0.09`, `VitesseRecuperationVie = 0.1`,
`CoefRecyclage = 0.5`.

- `Visible = Vie > 0` (`:343`). This uses raw `Vie`, without the pending
  `RecuperationVie`.
- `Definir_Vie` getter = `min(Vie + RecuperationVie, MaxVie)` (`:347-353`).
- The `Definir_Vie` setter (`:355-386`) holds the construction state machine:
  - `Vie >= MaxVie`: clamp to MaxVie and set `EnConstruction=False`. If it was
    not yet `Construit`, set `Construit=True`, then: if `Marche`, call
    `Fiefs(Persos(Numero).NumeroFief).Marches_Ajouter(Numero)`; if
    `Batiment_GainObjet(Type) >= 0`, give that object to `Persos(Numero)`.
  - `Vie <= 0`: set `EnConstruction=False` and `RecuperationVie=0`. If it was
    `Construit`, set it to False and, if `Marche`, call `Marches_Supprimer`.
    Then, unconditionally, if `Batiment_PerteObjet(Type) >= 0`, call
    `Persos(Numero).Objet_Supprimer` on that object.
  - Otherwise: `EnConstruction=True`.
  - Castles have TypeBatiment 0 and Numero 0, so the building-type-0
    parameters (Marche=0, GainObjet=-1, PerteObjet=-1 in the shipped data) and
    Persos(0) are used. With the shipped data this is harmless.
- `Etat` (`:388-399`) is a rendering stage. Once built, it is
  `NombreEtapeFabrication-1` when the building is a shop, factory or stocked
  service with `Stock < 1`, else `NombreEtapeFabrication`. While unbuilt it is
  `Int(Vie/MaxVie*NombreEtapeFabrication)`.
- `Definir_Stock` setter clamps the stock to `MaxStock` (no lower bound).
- `Stock_Objets(i) = v` also does `Stock += v - StockObjets(i)` (`:425-428`).
  `Stock` is therefore the total count of goods in the building (resources or
  objects).

### 1.7 Resource node: `ClsJeuRessources` (`ClsJeuRessources.cls`)

```
PositionX, PositionY: Long (pixels, top-left of a 32x32 sprite, AffRessources.Largeur/Hauteur = 32)
Nom, Apparence (= TerrainRessource index, sprite column), TypeRessource (= resource index harvested)
Quantite, MaxQuantite: Double
Visible = Quantite > 0
```

- `Definir_Type = v` (`:46-52`): `Nom = TerrainRessources_Nom(v)`,
  `Apparence = v`, `TypeRessource = TerrainRessources_TypeRessource(v)`,
  `MaxQuantite = Quantite = TerrainRessources_Quantite(v)`.
- `Definir_Type_Aleatoire` (`:54-61`): `i = Rnd * N` (Integer, so banker's
  rounding), and `i >= N` becomes 0. Type 0 therefore gets the `[0,0.5)` and
  `[N-0.5,N)` slices, `1/N` in total, so the draw is still uniform.
- **Nodes are infinite.** `Quantite` is never decremented by harvesting.
  Shipped `TerrainRessource_*` sections have no `Quantite` key, so
  `Quantite = 0` and `Visible` is always False, and `Visible` is never used by
  the gameplay code (it only matters for saves). Renderers draw every node
  (`FrmMoteur2D.frm:1816-1826`).

Shipped terrain resources (`Parametres_Ressources.ini`, `[TerrainRessource_n]`,
`TypeRessource` is 1-based in the file and stored as `-1`, `ClsJeuParam.cls:1456`):

| idx | Name | harvested resource index |
|---|---|---|
| 0 | Herbes médicinales | 0 |
| 1 | Pommier | 1 |
| 2 | Arbre | 2 |
| 3 | Gisement de pierre | 12 |
| 4 | Gisement de fer | 14 |
| 5 | Gisement de cristal | 17 |
| 6 | Gisement d'or | 23 |
| 7 | Gisement de soufre | 25 |

### 1.8 Treasure: `ClsJeuTresors` (`ClsJeuTresors.cls`)

```
X, Y: Long (pixel, top-left of a 26x26 box, AffTresor.Largeur/Hauteur = 26, ClsAffTresors.cls:34-35)
Fortune (private Double): running estimated value
IndiceImage: Integer = tier
Argent (private Double), Ressources(0..NR-1) (private Double), Objets: Collection<Integer> (object type indices)
```

- `Definir_Argent = v` → `Fortune += v - Argent`.
- `Definir_Ressources(i) = v` → `Fortune += (v - Ressources(i)) *
  Ressources_PrixVente(i)`.
- `Ajouter_Objet(k)` → add to `Objets` and `Fortune += Objet_PrixVente(k)`.
- Every `Fortune` assignment recomputes `IndiceImage` = the highest `i` with
  `Fortune >= Tresor_ValeurMin(i)` (`:69-79`). Tiers (`Tresors.ini`):
  0:0, 1:100, 2:300, 3:600, 4:1000, 5:3000, 6:6000. The tier only affects the
  sprite and the name.
- `Vide = Argent <= 1 && Objets.Count < 1 && all Ressources(i) < 1` (`:89-100`).
- `Rammasser_Tresor(Perso)` (`:55-67`) gives the perso the Argent, every
  resource (through `Perso.Definir_Ressources`, which may clamp — see the
  character spec) and each object (`Objet_Ajouter_Inventaire …, True`).

### 1.9 Decoration: `ClsJeuDecors` (`ClsJeuDecors.cls`)

```
Nom (tooltip; tombs get "&<name> (Niveau n)<epitaph><killer>"), Categorie, X, Y, Largeur, Hauteur
Temporaire = Decor_Duree(cat) > 0, Duree: Integer = Decor_Duree(cat) * Jeu.Vitesse (ticks)
EtapeAnimation, NombreEtapeAnimation
```

Decorations are **obstacles**: `Obstacle` returns code 16 for them. They block
building placement and resource placement. They do not block walking unless
the movement code uses `Obstacle` (character spec).

Shipped data: 1 Pont 71×88, 2 Drapeau 31×81, 3 Fontaine 89×86 (4 anim
frames), 4 Tombe 23×17 with Duree=60, 5 Tombe2 14×22 with Duree=60, …
(`Parametres_Decors.ini`).

### 1.10 Ground objects

There is no separate "item on the ground" entity. Dropped items, dropped
resources and loot all become **treasures** (§5).

### 1.11 `ClsJeuObjet` (`ClsJeuObjet.cls`)

`ClsJeuObjet` is an inventory slot that caches a copy of the object's
parameters (`Charger`, `:99-181`) and holds a stack count `Nombre`.

- `Actif = Numero > -1`.
- `Decharger` (`:183-254`) decrements `Nombre` if it is > 1, otherwise it
  clears the slot (`Numero=-1`, …).
- `PerteSiAttaque` (`:171-178`) =
  `PerteVie>0 || PerteEnergie>0 || PerteMagie>0 || PerteMoral>0 ||
  UtilisationUnique || PerteArgent>0 || (PerteRessourceIndice>=0 &&
  PerteRessourceQuantite>0)` (`And` binds tighter than `Or`).

Semantics are in the character/object spec.

---

## 2. World generation

### 2.1 Entry points

`FrmMoteur2D.MoteurJeu` (`FrmMoteur2D.frm:186`) calls `ModMain.Commencer`
(`:315`). `Commencer` (`ModMain.bas:290-770`) does the following:

1. `Randomize`. Then `Effets_Init`, `AffTemps.Init` (clock = 9:00),
   `AffDegat.Degats_Init`, `Tresors_Init` (empty), `Decors_Init` (empty),
   `Evenements_Initialiser` (no script), `Jeu.Init`, `Cartes.Chargee=False`.
2. Branch on the mode (network branches omitted; solo is the `Else` at `:393`):
   - Set `Monde.Largeur = LblLargeurCarte` and `Monde.Hauteur =
     LblHauteurCarte` (cells, UI sliders 128..1024; presets in `Partie.ini`
     `[Monde] TailleN*`).
   - `ReDim Ressources(LblNombreArbres)`, which gives **LblNombreArbres+1
     nodes**. The slider value is
     `CLng(TailleRessources * RessourcesMultiplicateur)`
     (`FrmParam.frm:3149`).
   - **"Campagnes"**: `Charger_Scenario(campaign\scenario, True)` (§6.1).
   - **"NouveauMonde"** (custom world), `:407-479`:
     1. For each fief i (count = `LblFiefs`): new Fief and new Chateau;
        `NumeroEquipe = ComboEquipe(i)+1`; `TypePeuple = ComboPeuple(i)`, or
        `Int(Rnd*NombrePeuples)` if "random" was chosen;
        `NombreCitoyens = LblVillageois(i)`.
     2. Persos 0..TotalVillageois-1 are assigned **consecutively** to fiefs:
        the first `LblVillageois(0)` go to fief 0, and so on. For each one:
        `New`, `NumeroFief=j`, `Init(i)` (draws Rnd), then
        `Tirer_Monstre_Aleatoire(i, peuple)` (draws Rnd, `ModGenererMonde.bas:65-115`);
        and `Maisons(i)` is created with `Numero=i`.
     3. For each fief: `Init` (it runs *after* the persos), set
        `Epoque = ComboEpoque`, and for each resource j with
        `Ressources_Epoque(j) <= Epoque` set the fief stock to
        `RessourcesDeparts(combo).ChateauxRessourcesDepart`.
     4. `Partie.RessourcesDeDepart = combo`. For each perso:
        `Perso_Charger_Ressources_Depart` (`ModGenererMonde.bas:198-211`).
        This only runs when `Partie.IndiceMode=1` and the race is not
        `DepartSansRessources`. It sets every resource with
        `Epoque <= fief epoch` **and** `NecessairePourConstruire` to
        `PersosRessourcesDepart`, and sets `Argent = PersosArgentDepart`. Then
        `NumeroEquipe = fief team`.
     5. Map: if "load map" (`OptionCarte(1)`) → `CmdChargerCarte_Click`
        (`FrmParam.frm:2266`: `Charger_Monde`, `Charger_Chateaux`,
        `Charger_Ressources`), then `Placer_Chateaux False` and
        `Cartes.Chargee=True`. Otherwise →
        `Monde.Generer_Monde(ComboTerrain)`, `Placer_Chateaux` (random),
        `Monde.Contruire_Routes2`, `Placer_Ressources`.
     6. `Placer_Joueurs`, then `VitesseEpoqueSelectionnee = ComboVitesseEpoque`.
   - **"NouvelleBataille"** (battlefield), `:481-607`: 2 fiefs with Epoque from
     the UI; castles get `MaxVie` from the people and **Vie=0** (no castles);
     soldiers are placed in 20-row grids at x=640 / x=1640, y=640. The map is
     loaded or generated, then `Placer_Ressources` runs. No roads. The details
     belong to the battle spec.
   - **"ChargerScenario"**: `Charger_Scenario(name)` (§6.1).
   - **"ChargerMonde"**: `Charger_Sauvegarde_Monde` (save spec).
3. Common post-processing (`:615-757`):
   1. For each perso: `IA = Not Projectile`. If `Joueur=0 && !ChoisirIA &&
      Race_IA_Selectionner(race)`: `If NiveauIA(=1) > Rnd` (always true, but
      one Rnd is drawn), then `ChoisirIA=True` and
      `FichierIA = FileIA.List(Int(Rnd*ListCount))`. Then `Actualiser_Bonus`.
   2. Load the human-controlled characters (campaign: into `NoPerso`;
      otherwise each selected save goes into the first free perso with
      `Joueur=0` whose team matches the save's team).
   3. Players' persos get `IA=False`.
   4. Difficulty (NouveauMonde/NouvelleBataille only):
      `Chateaux(i).MaxVie = MaxVie * CoefVieChateaux` (Long, so rounded) and,
      except in battle, `Vie = MaxVie`. AI persos get the XP bonus (character
      spec). For all persos, `ResurrectionNombre` comes from the UI and
      `VitesseResurrection *= UI factor`.
4. Back in `MoteurJeu` (`:324-418`):
   1. `Perso_Finalier` for all persos (appearance Rnd, AI file load).
   2. `Changer_Race_Personnage_Equipement` for AI persos.
   3. `Actualiser_Bonus` for all persos.
   4. `Fiefs(i).Marches_Actualiser` for every fief.
   5. Graphics and sounds are loaded.
   6. Per local player: load the interface and `Messages(i).Afficher_Conseil`
      (Rnd).
   7. `IA_Init` (Rnd).
   8. `Musique.Jouer_Musique 1` (Rnd).
   9. `Victoire/Defaite := False`, `Stastistiques.Init`,
      `Evenements_Lancer "Commencer"`, `Partie.EnCours = 2`. The loop starts.

### 2.2 Random terrain: `Generer_Monde(TypeMonde)` (`ClsJeuMonde.cls:121-159`)

```
N = NombreTypesTerrains (=11; types are 0..N)
erase and reallocate both arrays, size (L+1)*(H+1)+1, all 0
TypeTerrain = TypeMonde
R = 1 - Mondes(TypeMonde).Relief
k = 0
for i in 0..=L:                                    # outer = X
  for j in 0..=H:                                  # inner = Y; k is just sequential
    if Rnd > R:                                    # 1 draw per cell
      c = CLng((N-1)*Rnd + 1)                      # 1 more draw; banker's; range 1..N
      if c > N: c = 0                              # never happens
      elif SurfaceTerrain(c) == 0: c = 0
      Cat[k] = c
    k += 1
for t in 0..=N:                                    # INCLUDING type 0
  for pass in 0..=SurfaceTerrain(t):               # SurfaceTerrain(t)+1 passes, i.e. >= 1 even when 0
    Faire_Pousser(t)
Reformer_Terrain()
```

The seeding loop only assigns a cell when it is seeded. Seed type
distribution: with `x = 10*Rnd ∈ [0,10)`, `CLng(x+1)` gives 1 for x < 0.5,
k for x ∈ [k-1.5, k-0.5) (k = 2..10), and 11 for x ≥ 9.5. Types 1 and 11
therefore have probability 0.05 each and types 2..10 have 0.1 each. A seed of
type 0 is impossible here, but seeds of a type whose `SurfaceTerrain` is 0
become 0.

`Faire_Pousser(t)` (`:165-188`) is a sequential in-place cellular growth pass:

```
for j in 0..=UBound:
  if Apparence[j] == 0:                         # ALWAYS true during generation (apparence was erased)
    ch = 0
    for d in [-L, +L, -1, +1]:   if T(j+d) == t: ch += 15
    if ch > 0:
      for d in [-L-1, -L+1, +L-1, +L+1]: if T(j+d) == t: ch += 10
    if Rnd*100 < ch: Cat[j] = t                 # EXACTLY ONE Rnd per cell per pass
```

`T(idx)` returns 0 when the index is out of bounds. Neighbours use stride L
and wrap between rows.

Consequences to preserve:

- The type-0 passes run first and convert any seed whose orthogonal neighbours
  are all 0 into 0 with probability 1 (4×15 + 4×10 = 100). Isolated seeds are
  therefore erased before the other types grow.
- Later types can overwrite earlier ones.
- Number of Rnd draws per pass = `(L+1)(H+1)+1`.

The loading-screen progress call inside the loop is cosmetic.

### 2.3 `Reformer_Terrain` (`ClsJeuMonde.cls:327-414`)

Sequential and in place, for `i = 0..UBound`. Let U, D, Lf, Rt be
`T(i-L) > 0`, `T(i+L) > 0`, `T(i-1) > 0` and `T(i+1) > 0` (any non-zero type
counts, regardless of which). For cells with `T(i) > 0`:

| U | D | Lf | Rt | Apparence |
|---|---|---|---|---|
| 1 | 1 | 1 | 1 | 1 full |
| 0 | 1 | 0 | 0 | 2 |
| 1 | 0 | 0 | 0 | 3 |
| 0 | 0 | 0 | 1 | 4 |
| 0 | 0 | 1 | 0 | 5 |
| 1 | 1 | 0 | 0 | 6 |
| 0 | 0 | 1 | 1 | 7 |
| 0 | 1 | 1 | 1 | 8 |
| 1 | 0 | 1 | 1 | 9 |
| 1 | 1 | 0 | 1 | 10 |
| 1 | 1 | 1 | 0 | 11 |
| 0 | 1 | 0 | 1 | 12 |
| 0 | 1 | 1 | 0 | 13 |
| 1 | 0 | 0 | 1 | 14 |
| 1 | 0 | 1 | 0 | 15 |
| 0 | 0 | 0 | 0 | **category := 0** (isolated cells are deleted; apparence untouched) |

- **Gameplay effect:** single isolated non-plain cells become plain. The change
  happens in place, so it affects the later cells' tests: it can make the right
  and lower neighbours isolated in turn.
- Cells with `T=0` keep their previous `Apparence`. After a map load the arrays
  are `ReDim Preserve`d, so stale values may remain; this only matters for
  rendering.
- `Calculer_Diagonale` runs at the end.

### 2.4 Random castle placement: `Placer_Chateaux(True)` (`ModGenererMonde.bas:118-149`, `ClsPlaceObject.cls`)

```
P = new PlaceObject
P.SetBoardSize(lines = Hauteur-9, columns = Largeur-9)
if UBound(Fiefs) > 0: P.SetNumberOfObject(NF = UBound(Fiefs)+1)
     # MinSpace = Int((NbLine+NbColumn)/(Sqr(NF-2)+1)) - (Int(NF/Sqr(NF)) + 2)
P.SetPlacedObjectAsWall(True)           # no predefined objects
P.CalculateObjectsPlace()               # InitOjectPlace then MoveObjects
for k, pt in enumerate(P.objects):      # collection order
  Chateaux(k).PositionX = CLng(Int(8 - Rnd*16) + pt.ColumnPos*16)    # X drawn first
  Chateaux(k).PositionY = CLng(Int(8 - Rnd*16) + pt.LinePos*16)
```

The jitter `Int(8 - Rnd*16)` lies in `[-8, 7]`. Positions are 1-based cell
indices.

`GetBoxSize` (`ClsPlaceObject.cls:80-93`):

```
n = NF; box = (LineSize=NbLine, ColumnSize=NbColumn)
while n > 1:
  n = Int(n/2 + 0.5)
  if box.LineSize > box.ColumnSize: box.LineSize = Int(box.LineSize/2)
  else: box.ColumnSize = Int(box.ColumnSize/2)
```

`InitOjectPlace` (`:95-136`). The sweep is row-major, one object per box:

```
cur = (Top=1, Left=1); remaining = NF
while remaining > 0:
  occupied = any placed c with Top<=c.Line<=Top+LineSize and Left<=c.Col<=Left+ColumnSize
  if not occupied:
    remaining -= 1
    c.Line = Int(Rnd*LineSize + Top)       # line drawn first
    c.Col  = Int(Rnd*ColumnSize + Left)
    append c
  if not (Left + 2*ColumnSize - 1 > NbColumn): Left += ColumnSize
  else: Left = 1; Top += LineSize
```

`MoveObjects` (`:138-198`) is a repulsion relaxation, Gauss-Seidel style
(objects move in place, in order):

```
m = MinSpace; it = 0; moved = True
while it < 10000 and moved:
  moved = False
  for j in objects (in order):
    pos[k][l] = NbColumn*NbLine for k,l in -1..1
    for k in -1..1: for l in -1..1:
      for o in objects:
        if (o.Col, o.Line) != (j.Col, j.Line):              # excludes every object at j's exact spot
          d = |j.Line+k - o.Line| + |j.Col+l - o.Col|
          if d < m and d < pos[k][l]: pos[k][l] = d
    if pos[0][0] < m:
      moved = True
      v = pos[0][0]; tgt = (j.Line, j.Col)
      for k in -1..1: for l in -1..1:                        # k = line offset (outer), l = column offset
        if pos[k][l] >= v and pos[k][l] > 0 and 1 <= j.Line+k <= NbLine and 1 <= j.Col+l <= NbColumn:
          r = Rnd                                            # ALWAYS drawn when this point is reached
          if (pos[k][l] == v and r < 0.5) or pos[k][l] > v:
            v = pos[k][l]; tgt = (j.Line+k, j.Col+l)
      (j.Line, j.Col) = tgt
  it += 1
```

Quirks of random placement:

- There is **no constructibility or water test** for random castles.
- Castle 0 (normally the human player's fief) starts in the top-left box of the
  sweep. In random mode there is no shuffle.
- With a single fief no object is placed and castle 0 stays at (0,0).

Common castle setup (also for `Placer_Chateaux False`), `ModGenererMonde.bas:138-148`:

```
for i in 0..=UBound(Chateaux):
  p = Fiefs(i).TypePeuple
  MaxVie = Peuples_ChateauMaxVie(p) + sum_{j=0..Fiefs(i).Epoque} Epoque_BonusVieChateau(j)
  Largeur, Hauteur, EntreePositionX/Y = Peuples_Chateau*(p)
  Definir_Vie = MaxVie            # Construit=True; uses building-type-0 flags (see 1.6)
```

### 2.5 Roads: `Contruire_Routes2` (`ClsJeuMonde.cls:264-325`)

The function runs under `On Error Resume Next`, so out-of-range writes are
silently skipped. It only applies when `R = Mondes(TypeTerrain).Route > 0`.
`Contruire_Routes` (v1, `:190-262`) is dead code.

A virtual walker traces each road:

```
w = new ClsJeuPerso; w.Init(0)            # draws Rnd (NumeroPas, sex, maybe name)
w.Calculer_Longueur_Pas                   # step = min(Int(BonusDeplacement*Vitalite*CoefVitesse(at current pos)), Acceleration);
                                          # at least 1 if BonusDeplacement != 0; at most 16 (character spec)
for i in 0..=UBound(Fiefs):
  j = (i+1) mod NF                        # a RING: road from castle i to castle i+1, and the last back to 0
  w.Pos = (Chateaux(i).X + Chateaux(i).Largeur/2, Chateaux(i).Y + Chateaux(i).Hauteur/2)   # Long → rounded
  w.Aller_A(center of castle j)           # sets Direction, EnDeplacement via Orienter_Direction
  do:
    Cat[TI(x, y)] = R; Cat[TI(x+8, y+8)] = R; Cat[TI(x+16, y+16)] = R
    if Rnd > 0.8:
      if Rnd > 0.5:
        if Rnd > 0.5: x += 8, clamp to <= L*16   else: x -= 8, clamp to >= 0
      else:
        if Rnd > 0.5: y += 8, clamp to <= H*16   else: y -= 8, clamp to >= 0
  loop until not w.Marcher2               # body runs at least once
  # pave around castle i
  for kx in (X-16) .. (X+W+16) step 16:
    for ly in Y .. (Y+H) step 16:
      Cat[TI(kx, ly)] = R
Reformer_Terrain()
```

`Marcher2` (`ClsJeuPerso.cls:1467-1487`): if `EnDeplacement`, compute
`d = Direction - Position` and `l = |d|`. If `l >= step && l > 0`, then
`Position += step*d/l`, with each coordinate rounded to Long. Otherwise snap to
the target and set `EnDeplacement=False`. It returns `EnDeplacement`. The
direction is never recomputed, so the jitter only perturbs the path. Roads
**overwrite water/mountains**.

### 2.6 Random resource placement: `Placer_Ressources` (`ModGenererMonde.bas:151-171`)

```
for i in 0..=UBound(Ressources):
  if network: X = Rnd*(L*16 - 32); Y = Rnd*(H*16 - 32)            # Long, rounded
  else: repeat { X = Rnd*(L*16-32); Y = Rnd*(H*16-32) } until Obstacle(0, X, Y, 32, 32) == 0
  Ressources(i).PositionX/Y = X/Y
  Ressources(i).Definir_Type_Aleatoire                              # 1 Rnd
```

`Obstacle(perso=0, X, Y, w, h)` (`ModIA.bas:2407-2578`) returns the first
blocking category found, in this order:

1. Characters of a team different from Persos(0)'s that are `Attaquable` and
   overlap (code 9). Persos are not positioned yet at this point (they are at
   their initial coordinates), so this only matters near those coordinates.
2. Treasures (13).
3. Visible castles: 6 own castle, 8 allied, 11 enemy.
4. Visible houses (4, 5, 7, 10).
5. **Any resource node, including nodes not placed yet**: unplaced nodes still
   sit at (0,0), so the area near the origin is blocked (1).
6. Decorations (16).
7. `!Monde.Constructible(X, Y, w, h)` (999).

`Est_Superpose` (`ModIA.bas:2780-2816`) is the overlap test. It must be ported
verbatim, including its typos (`Y1+Largeur1`, `Y2+Largeur2`):

```
if w1 == 0 and h1 == 0:
  return X1>=X2 and X1<=X2+w2 and Y1>=Y2 and Y1<=Y2+h2
return ((X1<=X2 and X2<=X1+w1 or X1<=X2+w2 and X2+w2<=X1+w1) and
        (Y1<=Y2 and Y2<=Y1+h1 or Y1<=Y2+h2 and Y2+h2<=Y1+h1) or
        (X2<=X1 and X1<=X2+w2 or X2<=X1+w1 and X1+w1<=X2+w2) and
        (Y2<=Y1 and Y1<=Y2+h2 or Y2<=Y1+h1 and Y1+h1<=Y2+h2) or
        (X1>=X2 and X1+w1<=X2+w2 and Y1>=Y2 and Y1+w1<=Y2+w2) or
        (X2>=X1 and X2+w2<=X1+w1 and Y2>=Y1 and Y2+w2<=Y1+w1)) or
       (X1<X2 and X1+w1>X2+w2 and Y2<Y1 and Y1+h1<Y2+h2 or
        X2<X1 and X2+w2>X1+w1 and Y1<Y2 and Y2+h2<Y1+h1)
```

VB precedence applies: `And` binds tighter than `Or`.

### 2.7 Placing characters: `Placer_Joueurs` (`ModGenererMonde.bas:173-196`)

For each perso, with `C = Chateaux(NumeroFief)`:

```
X = C.X + C.Largeur/2 - Largeur/2 + C.EntreePositionX
Y = C.Y + C.Hauteur/2 - Hauteur/2 + C.EntreePositionY      (Long, rounded)
DansSonChateau = True
Direction = Position
```

Everyone starts inside their castle.

### 2.8 Map files: `ClsJeuCartes` (`ClsJeuCartes.cls`)

Config is `Donnees/Cartes.ini`. A map is a folder `Cartes\<name>\` containing
`Terrain.bmp` (required), `Chateaux.bmp`, `Ressources.bmp` (optional),
`Apercu.bmp`, `Carte.bmp` (cosmetic), `Description[FR|EN].txt` and
`Informations.ini` (cache). `Charger_Cartes` lists the subdirectories without
a "." in `Dir` order (`:129-143`).

BMP reading (`Charger_Monde`, `:228-316`; the same pattern is used for the
other two images):

- The width must be a multiple of 4 and the image uncompressed; otherwise a
  fatal message box ends the program.
- 24-bit: no palette; BGR triplets read from `bfOffBits`; rows bottom-up.
- Any other bit count is treated as **8-bit with a 256-entry BGRA palette read
  right after the 54-byte headers**, then one byte per pixel from `bfOffBits`.
- Width is a multiple of 4, so there is no row padding.
- `Monde.Largeur = width`, `Monde.Hauteur = height` (1 pixel = 1 cell).
  `NombreCases = (W+1)(H+1)`, a `ReDim Preserve`, so stale tail data can
  remain.

Terrain colour → type (`Cartes.ini [Terrain_1..12]` map to types 0..11,
`Type_Terrain`, `:696-707`). Unknown colours give 0 (plain):

| type | RGB |
|---|---|
| 0 Plain | 0,255,0 |
| 1 Herbs | 49,156,99 |
| 2 Ground | 132,132,0 |
| 3 Desert | 255,255,0 |
| 4 Mountain | 148,148,148 |
| 5 Road | 198,198,198 |
| 6..9 Walls | 255,206,156 / 255,206,0 / 255,156,0 / 255,99,0 |
| 10 Snow | 255,255,255 |
| 11 Water | 0,206,255 |

The pixel at image (x, row-from-top r) goes to cell index `r*W + x`. Then
`Reformer_Terrain` runs, so isolated pixels are erased (§2.3).

**Castles** (`Charger_Chateaux`, `:452-570`):

- If `Informations.ini` exists, read `[Chateaux] NombreChateaux` and
  `Chateau(n)_PositionX/Y`. If any entry is missing, fall back to the image.
- Image: every pure white pixel (255,255,255) is a castle with
  `X = x*16 - Peuples_ChateauLargeur(0)/2` and
  `Y = (r+1)*16 - Peuples_ChateauHauteur(0)/2`. Y is off by one row compared
  with the terrain (the counter starts at `Height` instead of `Height-1`).
  Peuple 0's size is always used.
- If `CreerFichierInformations`, write the positions to the cache.
- **`Chateaux()` is re-dimensioned to the number found, discarding the array
  created by `Commencer`.** The UI is expected to have set the fief count to
  this number.
- Then `Echanger_Emplacement_Chateaux` (`:572-591`) shuffles them:
  `for i in 0..n: j = Int(Rnd*remaining)+1`, take and remove the j-th
  remaining position (1-based).

**Resources** (`Charger_Ressources`, `:318-450`):

- If `Ressources.bmp` exists:
  - If `Informations.ini [Ressources] NombreRessources` is present,
    pre-allocate that many nodes. Otherwise grow the array per pixel found.
  - Every non-black pixel is a node with `X = x*16` and `Y = (r+1)*16 - 32`.
  - `Type = Type_Ressource(rgb)`: pure white gives a random type
    (`i = Rnd*8`, Integer-rounded, 8→0). Otherwise it matches `Cartes.ini
    [Ressource_1..8]`; with no match the type is 0.
  - Note the colour table bug: Ressource_1 (herbs) is 0,255,0. Ressource_8
    192,192,128 is distinct from Ressource_5 192,192,192.
  - Save `NombreRessources` to the cache if it was not fixed.
  - With a pre-allocated count and more pixels than the count, the original
    crashes (no error handler).
- Else: `ReDim Ressources(CLng(NombreCasesConstructibles/300))` (one more than
  the quotient) and `Placer_Ressources` (§2.6).

### 2.9 World lifetime

The world never changes size during a game. `Fiefs()`/`Chateaux()` can grow by
one per character that turns hostile (§4.7). `Tresors` and `Decors` are
dynamic collections. Terrain is static after generation; nothing modifies it at
runtime except the scenario commands at load time.

---

## 3. Time

### 3.1 The tick model (`FrmMoteur2D.frm:423-1523`)

There is a single real-time loop `Do While Not Quitter … DoEvents … Loop`. Each
iteration:

1. Music selection (cosmetic, but draws Rnd; see §7).
2. `TempsFin = Timer`, `DeltaTemps = TempsFin - TempsDeb` (float32 seconds
   since midnight).
3. Keyboard and mouse polling. UI actions **enqueue commands** (`Cmd_*`,
   `ModCommandes.bas:114-129`) into the `Commandes` collection. A few UI
   actions are immediate, but they are UI-only.
4. `IntTouches.Presser_Touche` (keyboard shortcuts, which also enqueue).
5. **Tick**, if `(DeltaTemps > 1/Vitesse or DeltaTemps < -1/Vitesse) and not
   Jeu.EnPause` (`:988-990`). The negative branch handles the midnight wrap of
   `Timer`. `TempsDeb = TempsFin`: time beyond one tick is **dropped**; there
   is no catch-up and at most one tick per loop iteration. The real rate is
   therefore at most `Vitesse` ticks/s, and slightly less in practice.
6. Rendering, if `DeltaTemps > 1/FPS` or `AffichageRapide` (cosmetic).

For the port, run a **fixed-step simulation of one tick per `1/Vitesse` s**.
`Vitesse` (default 10) is the game-speed setting: almost every rate in the
simulation is "per tick", so a higher `Vitesse` makes the game faster in
wall-clock time. The only quantities expressed in seconds are:

- decoration lifetime: `Decor_Duree * Vitesse` ticks, with the Vitesse sampled
  when the decoration is created (`ModDecors.bas:49`);
- `Jeu.Duree` and the chrono display (`/Vitesse`);
- the interface's 1-second "slow" update (§3.4).

`DureePartie` counts ticks (`:991`) and is reset to 0 in `MoteurJeu` (`:196`).

### 3.2 Order of one tick (solo / server branch)

```
DureePartie += 1; TempsDeb = TempsFin
[network message pump if connected]
Lire_Commandes()                              # ModCommandes.bas:134-243: execute queued commands in FIFO order, then clear.
                                              # On any runtime error: clear the whole queue (remaining commands lost).
camera scroll (cosmetic)
Comportement_IA()                             # ModIA.bas:152-174: NombreIAParTour+1 persos, round-robin (IndicePerso global cursor)
Comportement_IA_Fiefs()                       # ModIA.bas:289-297, for j in 0..=UBound(Fiefs):
    Fiefs(j).Transaction_Ressources           #   §4.2
    Fiefs(j).Restauration(Chateaux(j))        #   §4.3 (castle repair + age timer)
    Chateaux(j).Restauration                  #   §4.3
Deplacer_Persos()                             # FrmMoteur2D.frm:1570-1595, for i in 0..=UBound(Persos):
    if IA_Berserk > 0 and Vivant: IA_ComportementBerserk(i)
    RecuperationRapide                        #   attack/defense regen, reload countdown
    Deplacement1(...); Deplacement2(...)      #   movement (character spec)
    Travailler(...)                           #   §4.4; harvesting, building, picking treasures, ...
    if NoMessage >= 0: if Rnd > 0.99 or i == NoPerso: speech = Commentaires.Message(NoMessage, perso)   # Rnd always drawn here
TempVitesse += 1
if TempVitesse >= 10:                         # every 10th tick ("slow tick"); TempVitesse is form-level and NOT reset between games
    TempVitesse = 0
    Actions_Persos()                          # for each perso: RecuperationLente(Maisons(Numero), Fiefs(NumeroFief))  (§4.5)
    Actions_Tresors()                         # if Jeu.Confrontation: Tresor_Generer (§5.2)
Gestion_Utilisation_Speciale()                # ModCapacitesSpe.bas:20 (special abilities; other spec)
Evenements_Lancer("Jouer")                    # scenario script, every tick (§6.2)
AffTerrain.Animation_Suivante                 # cosmetic
Actualiser_Affichage_Batiments                # cosmetic (house animation frames)
Actualiser_Affichage_Decors                   # decoration animation + EXPIRY (§5.4) — gameplay-relevant (obstacles)
Actualiser_Effets_Visuel, Actualiser_Effets_Interfaces, AffDegat.Actualiser_Degats   # cosmetic
AffTemps.Actualiser                           # day clock (§3.3)
for each local player i:
    victory / defeat evaluation (§6.3)
    Interface(i).Actions_Automatiques(...)    # ClsInterface.cls:2438: auto-eat / auto-craft / auto-service, ENQUEUES commands (run next tick)
    Interface(i).ActualiserLent(...)          # every ~1 s: message log diff (§6.5), stats panel
```

Network clients run a reduced loop (`Actions_Persos_Client`, `_Lent`). That is
out of scope.

AI scheduling: `Nombre_IAs = UBound(Persos)+1` sets
`LPerso = UBound(Persos)` and `NombreIAParTour = CLng(LPerso*0.01)`, at least
1 (`ModIA.bas:2847-2866`). Each tick runs `j = 0..=NombreIAParTour`, i.e.
**NombreIAParTour+1** AI updates. Each one advances
`IndicePerso = IndicePerso+1`, wrapping to 0 after LPerso, and calls
`Comportement_IA_Persos`. `Arreter()` on an AI also triggers an immediate
`Comportement_IA_Persos(…, Simple=True)` (`ClsJeuPerso.cls:647-661`).

Commands enqueued during frame `f` therefore execute at the start of the next
tick, in enqueue order. Two commands span two queue entries and read
`Commandes(i+1)` while being executed:

- `ConstPersoCreerMaisonCoordonnees` + `...Type` (`ModCommandes.bas:146`);
- `AttaquerTerrainCoordonnees`.

### 3.3 Day clock (`ClsAffTemps.cls`, `Temps.ini`)

`Heures` starts at `JourneeDebut=9` and `Minutes` (Single) at 0. Every tick
(`:100-108`):

```
Minutes += 0.08                  # f32
if Minutes > 60:
    Actualiser_Temps()            # computed with the OLD hour (one-hour lag)
    Heures += Int(Minutes/60); Minutes = Minutes Mod 60 (→ 0, integer Mod); Heures = Heures Mod 24
```

With f32 accumulation an hour lasts **750 ticks**. `Actualiser_Temps` picks the
overlay phase from `[Temp_n] Debut` (0,3,4,5,6,7,17,18,19,20,21). The clock is
**display-only**: no gameplay code reads `Heures`. There is **no day counter
and no seasons** in this game.

### 3.4 Interface "slow" update period

`Compteur` (Single) `+= 1/Vitesse` each tick and fires when it reaches `>= 1`,
then resets to 0 (`ClsInterface.cls:2373-2376`). With f32 rounding the period
is `Vitesse` ticks, except **Vitesse+1 ticks for Vitesse ∈ {11, 12, 19, 23,
27, 28}**.

---

## 4. Economy

All rates are per tick unless stated otherwise. The character-level formulas
(Vitalite, skill levels, XP) are in the character spec. Only the parts that
move world state are given here.

Constants from `ClsJeuPerso.cls:32-80`, with `VitesseAction = 400`, so
`VitesseAction/400 = 1`:

```
VitesseConstruction = 3,  VitesseFabricationObjet = 0.1,  VitesseGenerationRessources = 1
IntervalesActions = 10
VitesseFatigueTravail = 10/1600/20*10*0.8 = 0.0025;  VitesseDemoralisationTravail = 0.0005
(same values for Construction and Fabrication)
VitesseResurrectionDepart = 2.5
```

### 4.1 Money and resource flows (overview)

- **Sources:**
  - harvesting nodes and producing in houses (§4.4);
  - item generation (`Generer_Ressource`, slow tick: `+GainArgent` and
    `+GainRessource(i)` when `GenerationRessources`, `ClsJeuPerso.cls:1947-1955`);
  - treasures;
  - selling to the castle (`Fief.Vendre`: perso resource → fief stock, perso
    gains `Quantite * PrixVente`, `ClsJeuFief.cls:158-171`);
  - selling objects to the castle (`Objet_Vendre_Chateau`: perso gains
    `Objet_PrixVente`; **the fief stock gains the object's `PrixRessources`**,
    `ClsJeuPerso.cls:2518-2545`);
  - the temple/bounty service (§4.6);
  - looting.
- **Sinks:**
  - buying from the castle at `PrixVente * CoefficientAchat(=2)`
    (`ClsJeuFief.cls:193-212`);
  - buying from houses (price passed through `Prix_Marchandee`, the haggling
    skill);
  - building costs;
  - object-making costs;
  - castle repair donations (to `Fief.Argent`);
  - becoming the ruler.
- **No taxes:** fiefs collect nothing automatically. `Fief.Argent` only grows
  through `Restaurer_Chateau` donations (`ClsJeuFief.cls:101-110`) and is only
  spent on castle repair.

Exact transaction rules (all quantities are floored with `Int` after clamping):

- `Fief.Vendre(r, q, perso, max)`: `q = perso.r` if `max` or `q > perso.r`;
  `q = Int(q)`.
- `Fief.Acheter(r, q, perso, max)`:
  1. `q = perso.Argent / (PrixVente*2)` if `max` or the perso cannot afford it;
  2. `q = min(q, fief stock)`;
  3. `q = min(q, perso.MaxRessources(r) - Int(perso.r))`;
  4. `q = Int(q)`.
- `Batiment.Acheter` (shop) works the same way against `Stock`.
  `Prix: Integer = q * Prix_Marchandee(PrixVente*2)` (banker's rounding to
  Integer); the money goes to `Maison.Argent`.
- `Fief.Vendre_ObjetsSelectionnees` (`:173-191`), called on Action 206/207 or
  by command:
  1. donate `ArgentSelectionne` to castle repair;
  2. sell the selected resources;
  3. sell the selected object unless it is inseparable;
  4. clear the selection.

### 4.2 Fief stock balancing: `Transaction_Ressources` (`ClsJeuFief.cls:80-99`)

Every tick, for every fief:

```
iMin = 0; for i: if Int(R[i]) < Int(R[iMin]) and Ressources_Epoque(i) <= Epoque: iMin = i      # strict <, first wins
iMax = 0; for i: if Int(R[i]) > Int(R[iMax]): iMax = i                                          # NO epoch filter
if |R[iMax] - R[iMin]| > 1:
    R[iMin] += 0.00125 * NombreCitoyens; R[iMax] -= 0.00125 * NombreCitoyens                   # direct array write, no clamp
```

`iMin` starts at 0 even if resource 0's epoch is above the fief's (it can only
be replaced by a lower one from an allowed epoch). The castle market slowly
"converts" its most abundant good into the scarcest, at
`0.00125 × citizens` units per tick.

### 4.3 Castle repair and the age (epoch) system

`Fief.Restauration(Chateau)` (`ClsJeuFief.cls:138-156`), every tick:

```
if Chateau.Definir_Vie < Chateau.MaxVie:              # getter includes pending RecuperationVie
    q = MaxVie - Definir_Vie + 1                       # +1 overshoot
    q = min(q, Argent * 0.2)
    Chateau.RecuperationVie += q
    Argent -= q / 0.2                                   # 5 gold per HP
if Epoque < NombreEpoques - 1:
    TempsChangementEpoque -= 1                          # decremented even when not changing (goes negative)
    if TempsChangementEpoque <= 0 and ChangerEpoque:
        Epoque += 1; ChangerEpoque = False
```

`Chateau.Restauration` (`ClsJeuBatiment.cls:155-164`), every tick:

```
if RecuperationVie >= 0.1: Definir_Vie = Vie + 0.1; RecuperationVie -= 0.1     # setter (clamps, state machine)
else: RecuperationVie = 0                                                          # remainder < 0.1 is discarded
```

The castle therefore heals at most **0.1 HP/tick**.

> **Quirk — destroyed castles rebuild themselves.** A castle at `Vie <= 0` is
> invisible, but if the fief still has `Argent`, `Fief.Restauration` adds
> `RecuperationVie`. On the same tick `Chateau.Restauration` sets `Vie = 0.1`,
> so the castle becomes visible again and keeps regenerating. The same happens
> when `Changer_Epoque` adds `RecuperationVie` (below).

Age advance, `Fief.Changer_Epoque(Chateau)` (`ClsJeuFief.cls:67-78`):

```
if not ChangerEpoque and Epoque < NombreEpoques-1:
    ChangerEpoque = True
    Chateau.MaxVie += Epoque_BonusVieChateau(Epoque+1)
    TempsChangementEpoque = Epoque_Duree(Epoque+1) * VitesseEpoque_Coef     # Long, rounded
    Chateau.RecuperationVie += Epoque_BonusVieChateau(Epoque+1) + 1
```

It is triggered from `Batir_Maison` (§4.4) when a house finishes and
`Batiment_NoEpoque(type) > Fief.Epoque`. **`NoEpoque` is 1-based** (it is not
decremented on load, `ClsJeuParam.cls:1520`) while `Fief.Epoque` is 0-based. So
**completing any building whose age is the fief's current age or later starts
the advance to the next age.**

The advance completes `Duree(next) × Coef` ticks later. The progress bar is
`(Duree(next)*Coef - TempsChangementEpoque) / (Duree(next)*Coef)`
(`FrmMoteur2D.frm:1804`).

Data (`Parametres_Fiefs.ini`, `NombreEpoques=5`; the `Epoque_6` section is
ignored):

| age | Name | BonusVieChateau | Duree (ticks to reach) | EmpereurPrix |
|---|---|---|---|---|
| 0 | Wood | 0 | 0 | 1000 |
| 1 | Iron | 500 | 5000 | 1500 |
| 2 | Ether | 1000 | 10000 | 2500 |
| 3 | Gold | 1500 | 15000 | 4000 |
| 4 | Imperial | 2500 | 25000 | 6000 |

`VitesseEpoque Coef` (UI choice; scenarios use `VitesseEpoqueDefaut=3`, i.e.
index 2): 2, 1.5, **1**, 0.5, 0.25.

Starting resources (`RessourcesDepart_n`):

| # | Name | Castle | Perso resource | Perso gold |
|---|---|---|---|---|
| 0 | Nothing | 0 | 0 | 0 |
| 1 | Few | 0 | 5 | 100 |
| 2 | Average | 100 | 10 | 300 |
| 3 | Lot | 300 | 15 | 1000 |
| 4 | Maximize | 500 | 20 | 1600 |

### 4.4 Buildings: placement, construction, production (`ClsJeuBatiment.Creer`, `ClsJeuPerso.Travailler`)

**Placement**:

- Player: `Cmd_Perso_Creer_Maison(perso, X, Y, type)` with the X/Y centred on
  the cursor (`FrmMoteur2D.frm:821`). The UI only allows it when the cursor is
  of type 2, i.e. a valid site.
- AI and scenario: `Trouver_Emplacement_Maison` (`ModIA.bas:2059-2117`):
  1. Reuse the previous house site if `Obstacle(...) == 0`.
  2. Otherwise make up to 11 attempts. Each attempt grows
     `DistanceObstacle(fief)` by 16 and samples
     `pos = anchor + Rnd*(2D + size) - D` per axis. The anchor is the chief's
     house or the chief (for invocations), else the castle with probability
     `1/(Marches.Count+1)` (or when there is no castle and no market), else a
     random market.
  3. Positions out of bounds are pulled back with `Rnd*16`.
  4. On success `DistanceObstacle = castle width`.

**`Batiment.Creer(X, Y, type, owner)`** (`ClsJeuBatiment.cls:81-131`):

- If `Batiment_CreerProprietaire(type) >= 0` and the owner is not an
  invocation: spawn an invocation of that race (`Creer_Invocation`) and have
  *its* house created recursively. The cost is still paid by the original
  owner, below.
- Otherwise:
  1. If this slot is already built, `Demolir(owner, MaisonPrincipale=True)`
     (refund, below). There is one building per character.
  2. Set `Position`, `Definir_TypeBatiment` (size, entry, flags, stock slots)
     and `Tir`; `Construit = False`.
  3. `MaxVie = ((CaracSpeciales0 - NivDepart)/NivDepart * 0.5 + 1) *
     Batiment_MaxVie(type)`, with `NivDepart = PersosNiveauSpecialesDepart = 20`
     and the skill read through `Definir_Carac_CompetenceSpeciales(0)`; Long,
     rounded.
  4. `Vie = MaxVie * 0.09`. The `Vie` field is written directly, without the
     setter, so the building is visible and unbuilt.
  5. `Argent = Stock = 0`;
     `MaxStock = Int(Batiment_MaxStock(type) * NivSpeciales0 / 20)`, using
     `Definir_Niveau_CompetenceSpeciales(0)`.
  6. Empty the inventory slots (-1); `owner.EtatFabricationObjet = 0`;
     `owner.Visible = True`; `owner.Rentrer_Maison(Me)`, which walks the owner
     to the site.
- Cost, if the owner is not an invocation: `Argent -= Batiment_PrixArgent`
  and each resource `-= Batiment_PrixRessource(type, i)`. The resource setters
  clamp at 0. **Affordability is not checked here**; the UI and AI check
  `PeutConstruire_Maison` beforehand (`ClsJeuPerso.cls:2247-2269`):
  `Argent >= PrixArgent`, every resource `>= PrixRessource`, and the
  emperor-only buildings require the ruler.

**Construction, Action 6** (`Travailler`, `ClsJeuPerso.cls:1296-1303`, then
`Batir_Maison`, `:2204-2245`). Each tick, while the character is not moving
and the house is visible:

```
MaisonCible.EnConstruction = True
v = 3 * Vitalite * (NivSpeciales0 + 20)/20 * Batiment_CoefVitesseConstruction(type)   # coef defaults to 1
Maison.Definir_Vie = Maison.Definir_Vie + v                                            # setter → Construit at MaxVie
(XP gain)
if Maison.Construit and Maison.Definir_Vie == MaxVie:
    own house → Action = 101 (go inside/work); someone else's → DansUneMaison = True; Arreter
    if Batiment_NoEpoque(type) > Fief.Epoque: Fief.Changer_Epoque(Castle)
else: Action = 6
```

Allies can help build any visible house (`Obstacle` code 4/7 → Action 6 via
UI/AI).

**Repair** uses the same Action 6 path on a damaged building.

**Demolition**, `Batiment.Demolir(owner, MaisonPrincipale)` (`:166-198`):

- If the owner is inside a house belonging to an invocation he commands and
  `MaisonPrincipale` is not set: demolish that one instead and kill its
  invocation.
- Otherwise, if `MaxVie > 0`:
  1. Refund `PrixArgent * 0.5 * Vie/MaxVie` gold, and
     `PrixRessource(i) * 0.5 * Vie/MaxVie` per resource with a price > 0.
  2. Eject the owner (`Action=0`).
  3. `Perso_Demolir_Batiment` moves the building's till, stock and objects to
     the owner if he overlaps the building, else into a new treasure
     (`ModCombat.bas:723-796`; §5.1).
  4. `Definir_Vie = 0`.
- In both cases `StockObjets` is zeroed.

**Production in a house** (Actions 101/102/600, `ClsJeuPerso.cls:1313-1331`):

- If the building has `RessourceFabrique >= 0` (stored as file value −1 by
  `Verification_Donnees`, `ClsJeuParam.cls:2219`): `Action = 102` and
  `Recolter_Ressources(RessourceFabrique, MaisonCible)`.
- Elif `Service`: `Preparer_Service`.
- Else `Arreter`.

**Harvesting**, `Recolter_Ressources(idx, Maison)` (`:2043-2120`). It is also
used outdoors for Action 2 on a node, with `idx = node.TypeRessource`:

```
mp = Ressources_TypeMatierePremiere(idx); qmp = Ressources_QuantiteMatierePremiere(idx)
VR = (1/400) * Vitalite * (NivRessources(idx) + 20)
gain = Ressources_VitesseExtraction(idx) * VR
if perso.R[idx] < MaxRessources(idx):
    if mp >= 0:
        if perso.R[mp] >= gain*qmp: perso.R[mp] -= gain*qmp; perso.R[idx] += gain
        else: Action = 100; comment 1 ("missing raw material")
    else: perso.R[idx] += gain
    if perso.R[idx] == MaxRessources(idx) and IA: Arreter
elif DansSaMaison and Maison.Stock < Maison.MaxStock:     # overflow into own house stock (for shops)
    same as above but the gain goes to Maison.Definir_Stock
else: Arreter; comment 0 ("full")
(XP when Action not in {0,100})
```

Example: wood (VitesseExtraction 1.5) with Vitalite 1 and skill 20 gives
0.15 per tick, i.e. 1.5/s at speed 10.

**Services**, `Preparer_Service` (`:2122-2173`), when the house's
`Stock < MaxStock`:

- `VC = (1/400) * Vitalite * NivServices(Service-1)`;
  `g = Service_VitesseCreation * VC`.
- Consume `g * Service_QuantiteMatierePremiere` units of the raw material if
  one is required (otherwise `Action=100`, comment 1).
- `Stock += g`; `Action = 600`.
- When the stock is full: `Arreter`, comment 0.

**Objects**, Action 500, `Fabriquer_Objet` (`:2331-2380`):

- `EtatFabricationObjet += 0.1 * Vitalite * NivObjets(Objet_Competence)`.
- When it reaches `Objet_DureeFabrication`: reset to 0,
  `Stock_Objets(slot) += 1`, `Action=100`, and pay `Objet_PrixRessources`
  (resources only, deducted at the end).
- Requires `PeutConstruire_Objet`: `Stock <= MaxStock-1` and enough resources
  (`:2382-2396`).

Selling from a house: shops sell `Stock` (`Batiment.Acheter`); factories sell
`Stock_Objets` (`Objet_Acheter_Maison`, price `Objet_PrixVente*2` haggled;
`ClsJeuPerso.cls:2402-2424`). The owner withdraws the till with
`Transfert_Argent_Prendre`.

### 4.5 Slow tick per character: `RecuperationLente` (`ClsJeuPerso.cls:1770-1927`)

This runs every 10 ticks. Fatigue and regeneration are in the character spec.
The world-relevant parts:

- `Generer_Ressource`;
- poison damage;
- **resurrection**: a dead perso inside his own visible castle with
  resurrections left decrements `DureeRessurection` by
  `VitesseResurrection / (NombreCitoyens/(NombreCitoyens-NombreMorts))`, or
  `/NombreCitoyens` if everyone is dead. So **resurrection is faster when more
  fief members are alive**. If the castle is gone, the perso is ejected;
- invocations resurrect in their own house;
- in Confrontation mode, instant resurrection at a random map position (2 Rnd,
  `:1920-1925`).

On death, `NombreMorts += 1` for non-invocations (`ClsJeuPerso.cls:4462`). On
resurrection, `NombreMorts -= 1` (`:4403`). On death the ruler loses his title
(`:4467`).

### 4.6 Ruler ("Empereur") and markets

- `Fief.Empereur_Changer(i, Payer)` (`ClsJeuFief.cls:112-131`): for
  `j = 0..LPerso` in the fief, perso `i` becomes ruler (it gets every
  `Objet_Empereur` object) and any other ruler loses the title (losing the
  emperor-only house and objects). If `Payer`, `i` pays
  `Epoque_EmpereurPrix(Epoque)`. Finally `Empereur = i`.
- Ways to become ruler: the player command (no affordability check in the
  command itself), the AI (`IA_Devenir_Empereur`, `ModIA.bas:2337`), or
  killing the ruler with `CoupDEtat` (`ModCombat.bas:408-412`, free).
- `PeutDevenir_Empereur` = no ruler, `Argent >= price`, and alive.
- Markets: `Marches_Ajouter(h)` (`ClsJeuFief.cls:250-257`) appends `h` to
  **this** fief's `Marches` once *per fief of the same team* (duplicates when
  allied fiefs exist). `Marches_Supprimer` removes one occurrence from every
  same-team fief. `Marches_Actualiser` runs once at start for each fief. The
  list biases AI house placement (§4.4) and allows selling at markets
  (Action 207 sells to `Fiefs(Persos(IndiceMaison).NumeroFief)`).

### 4.7 Fief ownership and dynamic fiefs

Fiefs are never captured: a destroyed castle stays with its fief (and can
self-repair, §4.3). Team membership is `NumeroEquipe` (fief level, copied to
persos).

`Perso_Hostile` (`ModCombat.bas:875-917`) creates a new fief with
`Fief_Ajouter_Nouveau` (`ModCapacitesSpe.bas:400-411`):

- team = max team + 1; it copies the people, age and age timer; the castle has
  `MaxVie=1` and `Vie=0`; 1 citizen.
- The perso and his invocations move to it.

`Attaquer_Allies(True)` sets `NumeroEquipe = -Numero` (a personal team)
without creating a fief.

---

## 5. Treasures, objects on the ground, decorations

### 5.1 Creation sources

`Tresor_Creer(X, Y)` (`ModTresors.bas:29-39`) appends a new empty treasure and
returns its 1-based index `= Count`. It is called by:

| Caller | Position | Content |
|---|---|---|
| `Perso_Tuer_Perso` (`ModCombat.bas:327-427`) if the killer does **not** overlap the victim (`Pille` = bounding boxes touch) | victim centre − 13 | 25% of gold, 25% of each resource; for each equipped then inventory object: if `Rnd < 0.25` and not indestructible: if `Rnd > 0.5`, the object goes to the treasure (or to the killer when `Pille`); in both cases it is removed from the victim. Then `Vide` → erase, else `Tresors_Verifier`. |
| `Perso_Piller_Batiment` (house destroyed by attack, `:663-721`) | house centre − 13 (Y uses **Largeur**) | 50% of the till; shop: 50% of the stock as its resource (only if `RessourceFabrique > 0`, **so resource index 0 is never looted**); factory: each stocked object with chance `0.5 > Rnd`. Then erase if `Vide`, else `Verifier`. |
| `Perso_Demolir_Batiment` (own demolition) if the owner does not overlap | same | 100% of the till, the stock, all stocked objects (loop `0..=NombreStockObjet`) and the house inventory. Erased if `Vide`, but **not merged**. |
| `Objet_Deposer_Sol` (`ClsJeuPerso.cls:2495-2517`) | perso X,Y | the object (unless inseparable) + the selected resources/gold (`Ressources_Deposer_Sol`, `:4814-4835`, then `Tresors_Verifier`). **Quirk:** if the object is inseparable, `IndexTresor` stays 0 and the original then accesses `Tresors(0)` → runtime error. The port should treat it as "drop only the selected resources in a new treasure" or as a no-op; this needs a decision. |
| `Ressources_Deposer_Sol()` | perso X,Y | selected gold and resources |
| `Tresor_Generer` (confrontation only) | random | §5.2 |
| Death tombs | — | a decoration, not a treasure (§5.4) |

`Tresors_Verifier(k)` (`ModTresors.bas:41-68`):

- If treasure k is `Vide`, erase it.
- Else find the first other treasure `i` (ascending) that overlaps it
  (26×26 boxes). Merge k into i (gold, resources, objects; the fortune is
  updated) and erase k. Only one merge is done.

**Index hazard:** `Tresor_Effacer` (erase on merge/empty) does **not** fix the
`IndiceTresor` of persos heading to a treasure. Only `Tresor_Detruire` (pickup)
does: persos with `Action=204` that target the same treasure are `Arreter`-ed
and those targeting a higher index are decremented (`:88-103`). A port with
stable IDs is fine as long as "heading to a treasure that no longer exists"
results in stopping.

### 5.2 `Tresor_Generer` (`ModTresors.bas:110-140`)

Called on the slow tick (every 10 ticks), **only in Confrontation mode**.

```
c = Rnd                                                   # always drawn
if (c < 1/60 or Count == 0) and Count <= 10:
    i = Tresor_Creer(CLng(Rnd*L*16 - 26), CLng(Rnd*H*16 - 26))   # X drawn first; may be NEGATIVE (−26..)
    if Rnd < 1/3: Argent = Rnd*1000
    for j in 0..=E1 where E1 = round(Rnd*6+1):              # see note
        if Rnd < 1/6:  Ressources[CInt(Rnd*(NR-1))] = Rnd*20 + 1   # assignment (replaces), not add
    for j in 0..=E2 where E2 = round(Rnd*6+1):
        k = CLng(Rnd*(NO-1))
        if not Objet_Secret(k): Ajouter_Objet(k)
    if Vide: Argent = Rnd*10000
```

Note on the loop bound: the `For … To Rnd*6+1` limit is evaluated once.
Assuming VB coerces it to the Long counter, there are 2..8 iterations. The
order between the index draw and the value draw in
`Ressources(Rnd…) = Rnd…` is uncertain (§8).

### 5.3 Picking up

A perso is sent with `Aller_Tresor` (`ClsJeuPerso.cls:750-761`): the target
is the treasure centre, using `LargeurTresor` for **both axes**, and
`Action=204`. When it stops moving with `Action=204`, `Travailler` calls
`Tresor_Ramasser(IndiceTresor)`: the perso gets everything, the treasure is
destroyed and the indices are fixed. Then `DansSaMaison=False` and `Arreter`
(`:1333-1336`). Treasures block house and resource placement (Obstacle 13).

### 5.4 Decorations lifecycle

- `Decor_Creer(cat, X, Y, Nom)` (`ModDecors.bas:29-52`).
- Tombs: `Race_Tombe(race) >= 0` creates one at the victim's position on every
  kill (`ModCombat.bas:424-426`). With Duree=60 it lasts `60*Vitesse` ticks.
- `Actualiser_Affichage_Decors` (`:54-72`), every tick:
  1. advance the animation of every decoration;
  2. then for `i = 1..Count`: if temporary, `Duree -= 1`; if `Duree <= 0`,
     remove it and **exit the loop**.

  At most one decoration expires per tick, and the decorations after the
  removed one are **not decremented that tick**.

---

## 6. Scenarios, victory, game end, messages

### 6.1 Scenario file (`.snr`): `Charger_Scenario` (`ModScenario.bas:28-383`)

Paths:

- scenarios: `Sauvegardes\Scenarios\<Lang>\<name>.snr`;
- campaigns: `Sauvegardes\Campagnes\<Lang>\<campaign>\<name>.snr`;
- the companion script has the same name with `.scr`;
- `.txt` files are descriptions.

`VitesseEpoqueSelectionnee = VitesseEpoqueDefaut` (`:43`).

Each line is **lower-cased** first (`:53`). Lines starting with `#` are
skipped. Parsing of lines that contain `=`:

- `X` = the text after `=` (up to `,` if present); `Y` = the text after `,`,
  else 0. They stay strings and VB coerces them to numbers in arithmetic.
- `(a)`: Min=Max=a-1. `(a-b)`: Min=a-1, Max=b-1 (a `-` before `)`).
  `(tous)`: Min=0, Max=UBound(Fiefs) if the line starts with `fief`, else
  UBound(Persos).
- Prop = the text between the first `_` and `=`.

Keys (with `L` = Monde.Largeur):

| Key | Effect |
|---|---|
| `nombre_fiefs=n` | n new fiefs (Init) and castles |
| `nombre_ressources=n` | n new nodes |
| `nombre_persos=n` | n persos: `Init` each (Rnd), house slots, effect slots |
| `monde_largeur`, `monde_hauteur` | cells |
| `type_terrain=n` | `Monde.TypeTerrain` (selects the road type only) |
| `epoque=n` | all fiefs: Epoque = n-1 |
| `rangees=n` | grid rows for multi-perso `position` |
| `choix_terrain=n` | **used both** as the world type for `debut` and as the terrain type painted by `terrain_carre*` |
| `terrain_carrevide(a-b)=c,d` | outline of the rectangle of cells x∈[a,c], y∈[b,d] (index `x + L*y`) set to ChoixTerrain; **then `Placer_Ressources` (re-draws all node positions and types)** |
| `terrain_carreplein(a-b)=c,d` | filled rectangle, then `Placer_Ressources` |
| `decor(n)=x,y` | `Decor_Creer(n-1, x, y)`, then `Placer_Ressources` |
| `fief(..)_peuple/equipe/epoque/maxvie/vie/ressources/position` | peuple = X-1; equipe; epoque = X-1; maxvie sets castle MaxVie and Vie; vie sets castle Vie (raw field); ressources = X for every resource with epoch ≤ fief epoch; position = X,Y. **`_nom` never matches** (the case label is `"Nom"` but the line was lower-cased) |
| `perso*(..)_…` | see below |
| `maison*(..)_type=n` | n=0: draw `Y = Int(Rnd*(NTypes-1))+1` until `NoEpoque(Y-1)-1 <= fief epoch`, not emperor-only and `CreerProprietaire <= 0`, then type = Y-1 (**the last building type is never drawn**). Else type = `Batiment_Numero(X)` (lookup by id). Then repeat `Trouver_Emplacement_Maison` until it succeeds, `Creer` (pays the cost!), `Arreter`, `Definir_Vie = MaxVie` (instantly built; market/GainObjet triggers) |
| `maison*_position=x,y` | move the house |
| `maison*_enfeu` | `Definir_Vie = CLng(Rnd*MaxVie*0.45)` |
| `maison*_stock=p` | if MaxStock>0: `Stock = Int(Rnd*MaxStock*p/100)`; factories redistribute the stock over the object slots: `for j in 0..Stock-1: s=Int(Rnd*(NStock+1)); Stock-=1; Stock_Objets(s)+=1` |
| `ressource*(..)_type=n` | TypeRessource / Nom / Apparence from TerrainRessource n-1 (Quantite not set) |
| `ressource*_position=x,y` | |
| line `debut` (no `=`) | `Generer_Monde(ChoixTerrain)`, `Placer_Chateaux` (random; **castle size and HP from each fief's current people, which is people 0 unless set before `debut`**), `Placer_Ressources`, `IA_Init`, then every perso at `CLng(Rnd*(L*16 - Chateaux(0).Largeur)), CLng(Rnd*(H*16 - Chateaux(0).Hauteur))` and `Arreter` |
| line `fin` | `Reformer_Terrain` |

Perso properties (`:191-293`). Many are skipped for `i = NoPerso`:

- `ia`, `ia_equiper`, `ia_manger`, `ia_pacifique`, `ia_temeraire`,
  `fichier_ia` (also sets ChoisirIA), `fichier_iascript`;
- `position=x,y`: then `y += 24`; if `(i-Min+1) mod Rangees == 0`:
  `y -= 24*Rangees`, `x += 24`. This fills columns of `Rangees` persos;
- `direction=dx,dy` (`Aller_A` relative); `allerchateau=n`; `allermaison=n`;
  `attaquerperso=n`; `attaquerpersos=n` (`n` increments for each perso);
  `attaquermaison`; `attaquerchateau`;
- `nom` (first letter capitalised); `chef=n` (stores the offset to the chief);
- `maxvie`/`vie`, `maxenergie`/`energie`, `maxmagie`/`magie`,
  `maxmoral`/`moral`, `maxattaque`/`attaque`, `maxdefense`/`defense`;
  `vitesse` (LongueurPas); `effetattaque` (X-1); `parole`;
- `grade=g`: `Tirer_Monstre_Aleatoire` with grade g-1; `race=r`; `sexe`;
- `fief=f`: sets the fief and team, re-rolls the monster (Rnd) for non-players
  and refills attack/defense;
- `dansunchateau=n`: places the perso at the entry of castle n
  (`DansSonChateau` or `DansUnChateau`);
- `argent=x[,p]`: `IIf(p=0, x, x - x*p/100 + Rnd*(x*p/100))`, **Rnd drawn
  even when p=0**;
- `ressource=0,y`: for every resource that is edible, or construction
  material, or `== Batiment_TypeMatierePremiere(Maisons(i).TypeBatiment)`
  (**this getter draws Rnd for factories, and it is always evaluated**), and
  whose epoch ≤ fief epoch: add `Rnd*y`. `ressource=n,y` sets resource n-1
  to y;
- `objet=n[,c]`: add max(c,1) copies of object n-1;
- `equipement=n,p`: n+1 tries, each with `Rnd < p/100`; on success draw
  `Int(Rnd*(NObj-1))` until the object is not secret (the last object is never
  drawn), and add it.

After the file:

1. `Actualiser_Infos_Fiefs` (`:385-399`) recounts, for each fief,
   `NombreCitoyens` = non-invocation members and `NombreMorts` = dead ones.
2. Invocations increase their chief's `NombreSoldats` and get
   `ChoisirIA=False`.
3. Stats are clamped to their max.
4. `Evenements_Charger` loads the `.scr` (if it is missing there is no script).

`Commencer`'s post-processing (§2.1 step 3) then applies. Difficulty scaling
is **not** applied to scenarios/campaigns.

### 6.2 Scenario scripts (`.scr`, VBScript): `Evenements_Lancer` (`ModScenario.bas:446-482`)

The script is called with `"Commencer"` once before the loop and with
`"Jouer"` **every tick** (§3.2). Each call does the following:

1. `ScriptControl.Reset`.
2. Re-add the objects `Persos`, `Maisons`, `Chateaux`, `Fiefs` (**new 1-based
   Collections**, so script `Persos(1)` = engine `Persos(0)`), `Tresors`,
   `Jeu`, `Moteur` (FrmMoteur2D), `Interface` (`Interface(0)`), `Parametres`,
   `Commentaires`, `Musique`, `Langue`.
3. `AddCode(full script text)`.
4. `Run(cmd)`.

Because of the Reset, **script-level global variables do not persist between
ticks**; all state must live in game objects. Scripts call public members of
the classes directly (e.g. `Jeu.VictoireStandard = False`,
`Jeu.Definir_Victoire = True`, `Persos(2).Aller_A …`, `Interface.Objectifs_Ajouter`,
`Persos(1).Definir_Ressources(8)`).

A faithful port needs either a small VBScript interpreter over a binding of
these objects, or a hand translation of the shipped scripts. The shipped
`.scr` files are listed under `Sauvegardes/Campagnes/*/*/*.scr`; none exist
for the standalone scenarios in this install. No script is run on network
clients.

### 6.3 Victory and defeat (`FrmMoteur2D.frm:1136-1172`)

Evaluated every tick, for each local player i (team T = team of `Nopersos(i)`):

```
if Jeu.VictoireStandard:
    Victoire(i) = for every fief j with NumeroEquipe != T:
                      not Chateaux(j).Visible and Fiefs(j).NombreMorts >= Fiefs(j).NombreCitoyens
if Victoire(i):  (i==0 → victory music)
elif Jeu.DefaiteStandard:
    f = fief of Nopersos(i)
    Defaite(i) = not Chateaux(f).Visible and Fiefs(f).NombreMorts >= Fiefs(f).NombreCitoyens
    (i==0 and Defaite → defeat music)
```

- Victory = every enemy fief has no castle **and** all its citizens are dead.
  Defeat = your own fief is in that state.
- Both are **recomputed every tick and can revert** (e.g. an enemy resurrects
  or a castle self-repairs, §4.3). With `VictoireStandard=False`, `Victoire(i)`
  keeps whatever the script set (`Jeu.Definir_Victoire`).
- Defeat is only tested when there is no victory. Defeat with
  `DefaiteStandard=False` is likewise script-controlled.
- **The game does not end on victory or defeat.** It only shows the banner
  sprite and plays music; the loop exits only when the player quits
  (`Quitter`). AI behaviour reads `Victoire(0)` (`ModIA2.bas:345, 359`).
- The hostile-perso fiefs (§4.7) count as enemy fiefs.
- Network confrontation disables both standard tests (`ModMain.bas:314-318`).

### 6.4 Chrono

`Jeu.Chrono` is never changed by the engine. Scripts set and decrement it, and
it is saved in world saves. When `IntituleChrono <> ""` and `Chrono > 0`, the
message log shows `IntituleChrono & Definir_Chrono` as its first line
(`ClsJeuMessages.cls:447-469`).

### 6.5 Game end (`FrmMoteur2D.frm:1530-1567`)

After the loop:

1. If `Jeu.Duree > 10` (seconds), save every perso owned by this player.
2. Save the interface layout.
3. Save the world on quit (if the option is set, there is no victory/defeat
   and it is not a campaign).
4. `Partie.EnCours = 0`.
5. `Stastistiques.Fin_Partie`: clears the temporary effects, inventory and
   equipment of the tracked player persos in memory (the saves were already
   written) and recomputes the bonuses.
6. `Document_Stastistiques` (report).

Statistics (`ClsJeuStastistiques.cls`): at start, a snapshot of each
`Joueur > 0` perso is reloaded from its save file. At the end, the report
shows deltas of level, treasure score, max stats, skill levels and bestiary,
plus the frags (`FragsPersos/Batiments/Chateaux`) and deaths.

### 6.6 Message log: `ClsJeuMessages` (`ClsJeuMessages.cls`)

There is one log per local player. `Actualiser_Messages(Perso, Fief)`
(`:183-434`) runs on the interface's slow update (§3.4) and diffs against the
values cached at the previous call. On the first call, or when the controlled
perso changes, it only caches. Messages, each enabled by a `Messages.ini`
flag (all enabled in the shipped file):

| Event | Text pattern (EN) |
|---|---|
| alive ↔ dead | `"- You bring back to life."` / `"- You're dead. Come back to castle or come upon temple."` |
| bestiary +n of race r | `"- You've defeated n <race lower, plural if n>1>."` |
| soldiers ±n | `"- You've recieved n soldier(s)."` / `"- You've lost n soldier(s)."` |
| gold change ≥ 1, excluding this period's passive generation `Definir_GenerationArgent` (alive only) | `"- You've recieved n coins."` / `"- You've lost n coins."` |
| resources gained (except `RessourceCourante`) | one line, `"- You've recieved a x, b y."` |
| resources lost (except the raw material of `RessourceCourante`) | `"- You've lost …"` |
| objects gained/lost (counting `ObjetSelectionne` as owned) | likewise, with plural names when ≥2 |
| max-stat increases | `"+ You've recieved n more point at the HEALTH level."` (and so on for each stat) |
| skill level ups | `"+ You've recieved n level at the skill <name>."` |
| ruler change in own fief | `"- <TITLE NAME IS DEAD>"` / `"- <NAME BECOME TITLE>"` (upper-cased) |

All amounts are `Int(...)` of the deltas. The display rules are cosmetic:

- each message expires after `Len(text)*0.16` real seconds;
- at most 12 messages are kept;
- multi-line messages add blank lines.

`Afficher_Conseil` adds one random tip from `Info_Conseils.txt` at start:
`Temp = Rnd*(n-1)` as a String, then `Val`, i.e. banker's rounding.

Speech bubbles (`ClsJeuCommentaires`) are picked with
`Int(Rnd*NombreMessages(type))`. The perso setter only accepts a new comment
when the current one is empty (`ClsJeuPerso.cls:4667-4674`). They are
cosmetic, but they are triggered from simulation code (they draw Rnd).

---

## 7. Random number usage

The original uses one global `Rnd` stream for everything, including frame-rate
dependent cosmetic code (music selection runs every frame). Its results are
therefore not reproducible even with a fixed seed. Recommendations:

- **Sim RNG**, seeded per game: every draw marked **S** below, in the stated
  order, respecting the non-short-circuit rule (draws inside `And`/`Or`/`IIf`
  are always made).
- **Cosmetic RNG**, separate stream: draws marked **C**. Moving these out of
  the sim stream loses no fidelity, since the original was never reproducible.

### 7.1 World generation and setup (one-off)

| Where | Draws | Kind |
|---|---|---|
| `ModMain.bas:416` (and `:498` battle) | random people per fief: `Int(Rnd*NombrePeuples)` | S |
| `ClsJeuPerso.Init` `:433` | `NumeroPas = Int(Rnd*3)` per perso init (also the road walker, save loads, invocations) | S (only affects the walking animation phase, but keep it in S to keep the Init order simple, or move it to C) |
| `Changer_Race_Personnage` `ClsJeuPerso.cls:4989` and `ClsJeuNoms.cls:74-76` | sex `Rnd > 0.5` (always drawn); name `NomsX(Rnd*L)` (Integer rounding) | S |
| `Tirer_Monstre_Aleatoire` `ModGenererMonde.bas:79,94,95` | one `Rnd < chance` per monster entry until a hit; or `Rnd < ChancesRaceAleatoire` then `Int(Rnd*NombreRacesJouables)` | S |
| `Generer_Monde` `ClsJeuMonde.cls:137-138` | 1 per cell (+1 if seeded) | S |
| `Faire_Pousser` `:181` | exactly 1 per cell per pass | S |
| `ClsPlaceObject` `:125-126, 180` | 2 per castle (line, column); 1 per candidate neighbour reached in `MoveObjects` | S |
| `Placer_Chateaux` `ModGenererMonde.bas:132-133` | 2 per castle (X jitter, Y jitter) | S |
| `Contruire_Routes2` `ClsJeuMonde.cls:293-304` | per walker step: 1 (`>0.8`); if true, +1 (axis) and +1 (sign) | S |
| `Placer_Ressources` `ModGenererMonde.bas:163-164`, `ClsJeuRessources.cls:57` | 2 per attempt (X, Y) until the site is free, then 1 for the type | S |
| `Echanger_Emplacement_Chateaux` `ClsJeuCartes.cls:585` | 1 per castle | S |
| `Type_Ressource` `ClsJeuCartes.cls:713` | 1 per white pixel in `Ressources.bmp` | S |
| `Commencer` AI selection `ModMain.bas:623,626` | 1 (`NiveauIA > Rnd`) + 1 (file index) per eligible perso | S |
| `ModScenario.bas:269,278,301,316,319,322,352-353,403,405` | `argent`, `ressource`, `maison_type`/`enfeu`/`stock`, `debut` positions, `equipement` | S |
| `Batiment_TypeMatierePremiere` getter `ClsJeuParam.cls:2644` | **draws `Rnd > 0.5` per matching (resource, object) pair for factory buildings; any caller of this getter consumes randomness** (scenario `ressource=0`, AI shopping `ModIA.bas:2128`, `ModIA2.bas:481`, etc.) | S |
| `IA_Init` `ModIA.bas:116,121` and `Nombre_IAs` `:2861` | IndiceObjet; one patrol angle per perso; +1 | S |
| `Perso_Finalier` → `AffPerso`/`AffApparence` `ClsAffPerso.cls:149`, `ClsAffApparence.cls:196` | appearance choice | C |
| `Afficher_Conseil` `ClsJeuMessages.cls:508`; `FrmMoteur2D.frm:2751` loading picture; `ClsPerMusique.cls:188,207,244` | tips, loading screen, music | C |

### 7.2 Per tick (sim)

| Where | Draws | Kind |
|---|---|---|
| `Deplacer_Persos` `FrmMoteur2D.frm:1589` | 1 per perso with a pending comment (`Rnd > 0.99`), plus 1 in `Commentaires.Message` when shown | the condition is C-able (it only decides whether a speech bubble is shown) |
| `Tresor_Generer` `ModTresors.bas:115-136` (confrontation, slow tick) | see §5.2 | S |
| `RecuperationLente` confrontation respawn `ClsJeuPerso.cls:1923-1924` | 2 | S |
| `Trouver_Emplacement_Maison` `ModIA.bas:2083-2108` | 2 per attempt + bounds fix-ups; anchor choice `Rnd <= 1/(n+1)` (always drawn when the first sub-condition is evaluated), `Int(Rnd*Count)` | S |
| AI (`ModIA.bas` 72 sites, `ModIA2.bas:727`) | see the AI spec | S |
| Combat (`ModCombat.bas:98,117,183,237,278,283,374-394,513-598,701`) | criticals, resistances, loot, retaliation and help calls; `:489, 552, 585` are damage-number/effect positions | S, except `:489/552/585` → C |
| `ModCapacitesSpe.bas:98,368,390-391,466`; `ModDialogue.bas` (help/orders replies) | see their specs | S |
| `ClsIntFenetre.cls:1407-1408`, `ClsInterface.cls:2268` | UI-triggered gameplay actions | S (they execute in frame context in the original; put them in the command path) |

### 7.3 Other notes on randomness

- Mapping a draw `r` to an integer:
  - `Int(r*n)` → uniform 0..n-1;
  - `Long = r*n` → banker's rounding, values 0..n where the endpoints have half
    weight;
  - `Int(a - r*b)` → floor.

  Reproduce the exact mapping at each site; the mapping alters distributions
  (e.g. object index `CLng(Rnd*(NO-1))` in `Tresor_Generer` gives the first and
  last objects half weight).
- The original also uses the wall clock (`Timer`) in gameplay-adjacent code:
  periodic perso save, comment duration, message expiry. None of it affects the
  simulation state except the periodic save.

---

## 8. Quirk list and open questions

The quirks above, to preserve:

1. The pixel→cell mapping is banker's-rounded (17/15-px alternating cells),
   while rendering uses floor (§1.4).
2. Cell index 0 and the last index are never constructible; speed lookups
   there return 1.
3. Rows wrap in all neighbour tests; the arrays have L+H+2 extra aliasing
   cells.
4. The type-0 growth pass erases isolated seeds; `Reformer_Terrain` deletes
   isolated cells in place, cascading to the right and downward.
5. Road layout is a ring of castles i→i+1; roads overwrite any terrain; there
   is no constructibility test for random castles.
6. Fief 0's castle starts in the top-left box of the sweep (random worlds are
   not shuffled).
7. Resource nodes are infinite; `Quantite` is unused.
8. Destroyed castles self-repair whenever the fief has money or an age advance
   starts (§4.3).
9. Any completed building of the current age starts the next age
   (`NoEpoque` 1-based vs `Epoque` 0-based).
10. `TempsChangementEpoque` counts down every tick even when idle.
11. `Fief.Restauration` overshoots each repair purchase by 1 HP; the castle
    heals 0.1 HP/tick; a leftover `RecuperationVie < 0.1` is discarded.
12. Fief balancing ignores the epoch when choosing the max; `iMin` starts at 0.
13. `Marches_Ajouter` duplicates entries per allied fief.
14. Victory and defeat do not end the game and are re-evaluated every tick.
15. Script globals reset every tick (`ScriptControl.Reset`).
16. Scenario `fief_nom` is ignored (lower-case bug); `maison_type=0` never
    picks the last building type; `perso_equipement` never picks the last
    object.
17. `ReDim X(n)` off-by-one: custom worlds get `LblNombreArbres+1` nodes and
    maps without a resource image get `CLng(cells/300)+1`.
18. Only one temporary decoration expires per tick, and the later ones skip
    their decrement on that tick.
19. House looting never loots resource index 0 (`RessourceFabrique > 0` test).
20. Treasure-index desync on merge/erase (§5.1); a perso dropping an
    inseparable object hits `Tresors(0)` (the original crashes).
21. `TempVitesse` (the 10-tick slow phase) persists across games.

Open questions (verify against the binary if exact parity is needed):

- **For-loop limit coercion:** for `For j = 0 To Rnd*6+1` with a Long `j`, is
  the Double limit compared as a Double (j ≤ 1.0..7.0, giving 2..7 iterations,
  or 8 only when the limit is exactly 7) or coerced to Long first (banker's,
  2..8 iterations)? The spec assumes coercion. The two readings differ only in
  the distribution of the iteration count.
- **Evaluation order of `obj.Prop(idxExpr) = valExpr`** when both sides
  contain `Rnd` (`ModTresors.bas:126`): which is drawn first?
- **BMP reading:** `Input(LOF(1), #1)` requests more bytes than remain after
  the seek. The original apparently tolerates this, and the pixel count is
  taken from the buffer length. A port should read exactly `W*H` pixels, and
  should handle 4-bit/1-bit BMPs as the original does: their palette and pixel
  bytes are misread, because every non-24-bit file is treated as 8-bit.
