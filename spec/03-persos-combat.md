# 03 — Characters (persos) and combat

Scope: `ClsJeuPerso.cls` (the character object), `ClSurPerso.cls` (the script facade),
`ModCombat.bas`, `ModCapacitesSpe.bas` (used objects, invocations, projectiles),
`ModDialogue.bas` (orders to soldiers, calls for help), `ClsJeuNoms.cls`,
`ClsJeuEffet`/`ClsJeuEffetTemp`/`ClsJeuDegat`/`ClsJeuCompetences`, the perso part of
`ModSavPerso.bas`, the pre-game character generation in `FrmParam.frm`, and the data files
`Combat.ini`, `Parametres_Persos.ini`, `Parametres_Races.ini`, `Parametres_Resistances.ini`,
`Parametres_CompetencesObjets.ini`.

Line references are `File:line` in the VB source (1-based, CRLF stripped). Unless stated
otherwise every formula is executed with VB6 `Double` arithmetic and assigned to the declared
type of the target (see §0).

AI decision making (`ModIA*.bas`, `ClsJeuIA*.cls`), buildings, objects, fiefs, trading prices,
treasures and world generation belong to other spec files. They are only described here where
they are part of a character's state machine.

---------------------------------------------------------------------------------------------

## 0. VB6 semantics that change gameplay (read first)

These rules apply to every formula below. A Rust port must reproduce them.

1. **Assigning a Double to a `Long`/`Integer` rounds with banker's rounding**: round half to
   even, `CLng(2.5)=2`, `CLng(3.5)=4`. This applies to positions (`PositionX As Long`), to
   `Longueur_Pas`, to `BonusMax*` (Long), to `Prix_Marchandee` (Integer), to array indices
   computed from a Single/Double (`NomsF(Rnd * L)`), and so on. `Int(x)` is a floor (toward −∞:
   `Int(-1.5) = -2`).
2. **`And`/`Or` never short-circuit.** Every operand is evaluated, so a `Rnd` call that is an
   operand of `And`/`Or` is **always consumed**, even when the other operand is already false.
   Precedence: `Not` > `And` > `Or`. `IIf(c, a, b)` evaluates both `a` and `b`.
3. `Not` on an Integer is a bitwise NOT. `A And B` with Integer operands is a bitwise AND
   (see the quirk in §6.5).
4. `For i = a To b` evaluates `b` once. Persos appended to `Persos()` during a loop are not
   visited by that loop.
5. `Rnd` is the standard VB6 generator: a 24-bit LCG,
   `s = (s * 0x43FD43FD + 0xC39EC3) mod 2^24`, returning `s / 2^24` as a Single. It is seeded by
   `Randomize` (Timer) at `ModMain.bas:142,302`. All game randomness goes through the one global
   stream, so **the order of the Rnd calls is part of the game state**. §11 lists every Rnd
   call in this area.
6. Booleans are −1/0. `-CDbl(True) = 1` is the value written to save files.

---------------------------------------------------------------------------------------------

## 1. Global constants

### 1.1 `Combat.ini` (`ModCombat.bas:46-69`)

| key | value | meaning |
|---|---|---|
| CoefDegatPerso | 0.5 | damage per point of (Attack − Defense) on characters |
| CoefDegatBatiment | 1 | damage multiplier on buildings |
| DureeImmobilisation | 0 | ticks of stun applied to a defender that is hit |
| ChanceCoupCritique | 0.05 | P(critical hit) = P(Rnd > 0.05 fails) |
| CoefDegatCoupCritique | 1 | added to the crit-skill ratio |
| ChanceEchecCritique | 0.05 | P(critical failure) |
| PerteEchecCritique | 20 | attack points lost on a critical failure |
| MaxDegatBatiment | 20 | maximum attack points spent per hit on a building |
| CoefPillageArgentPersoMort | 0.25 | share of the victim's money that is looted |
| CoefPillageRessourcesPersoMort | 0.25 | share of the victim's resources that is looted |
| CoefPillageObjetsPersoMort | 0.25 | per-item chance that an item is taken from the victim |
| CoefDestructionObjets | 0.5 | chance that a taken item is destroyed instead of transferred |
| CoefPillageArgentMaison | 0.5 | building looting |
| CoefPillageRessourcesMaison | 0.5 | building looting |
| CoefPillageObjetsMaison | 0.5 | building looting |

`Const ImpactRalentissement = True` (`ModCombat.bas:26`).

### 1.2 `Parametres_Persos.ini` (`ClsJeuParam.cls:903-923`)

`CoefficientAchat=2`, `PersosLargeurDefaut=24`, `PersosHauteurDefaut=32`,
`PersosMaxVie=PersosMaxEnergie=PersosMaxMagie=PersosMaxMoral=20`,
`PersosNiveauSpecialesDepart=20`, `PersosMaxRessources=20`, `PersosNiveauServicesDepart=20`,
`PersosNiveauObjetsDepart=20`, `PersosAttaque=20`, `PersosPorteeAttaque=0`,
`PersosDefense=20`, `PersosVitesse=4`, `PersosAcceleration=1`,
`PersosNombreObjetsEquipes=11`, `PersosNombreObjetsInventaire=32`,
`PersosNombreObjetsInventaireMaison=32`.

### 1.3 Constants in `ClsJeuPerso.cls:32-117`

`IntervalesActions=10` and `VitesseAction=400` (so `VitesseAction/400 = 1`). The values are
already evaluated here. "Per slow tick" means once per `RecuperationLente` call, i.e. every 10
game ticks (§4).

| Constant | Value | Unit |
|---|---|---|
| VitesseConstruction | 3 | building HP per tick (×modifiers) |
| VitesseFabricationObjet | 0.1 | crafting progress per tick (×modifiers) |
| VitesseFatigueDeplacement | 0.01 | Energy per slow tick while moving |
| VitesseDemoralisationDeplacement | 0 | |
| VitesseFatigueTravail / Construction / Fabrication | 0.0025 | Energy per slow tick × skill |
| VitesseDemoralisationTravail / Construction / Fabrication | 0.0005 | Morale per slow tick × skill |
| VitesseFatigueCombat | 0.125 | Energy per slow tick × Vitalite, per non-full combat bar |
| VitesseDemoralisationCombat | 0.025 | Morale, same |
| VitesseGenerationRessources | 1 | |
| VitesseRecuperationVie | 0.0375 | fraction of max per slow tick drained from the food pool |
| VitesseRecuperationEnergie | 0.05 | |
| VitesseRecuperationMagie | 0.05 | |
| VitesseRecuperationMoral | 0.025 | |
| VitesseRecuperationAttaque | 1.25 | attack points per tick |
| VitesseRecuperationDefense | 0.75 | defense points per tick |
| VitesseRegenerationVieEnAttente | 0.0005 | fraction of max per slow tick, idle outside (Action 0) |
| VitesseRegenerationEnergieEnAttente | 0.0005 | |
| VitesseRegenerationMagieEnAttente, MoralEnAttente | 0 | |
| VitesseRegenerationVieEnRepos | 0.001 | resting at home (Action 100) |
| VitesseRegenerationEnergieEnRepos | 0.001 | |
| VitesseRegenerationMagieEnRepos | 0 | |
| VitesseRegenerationMoralEnRepos | 0.00025 | |
| VitesseDeplacementFantome | 6 | px/tick for dead characters (ghosts) |
| VitesseResurrectionDepart | 2.5 | |
| CoefExpVie/Energie/Magie/Moral | 20 | |
| CoefExpAttaque | 400 | |
| CoefExpDefense | 250 | |
| CoefExpCompSpeciales | 20 | |
| CoefExpCompConstruction | 0.00075 | |
| CoefExpCompConstructionObjet | 0.05 | |
| CoefExpCompDestruction | 0.0025 | |
| CoefExpCompDestructionObjet | 1 | |
| CoefExpCompCoupCritique | 0.02 | (= 0.00125·4·4) |
| CoefExpCompCoupCritiqueObjet | 1 | |
| CoefExpCompMarchandage | 0.001 | |
| CoefExpCompMarchandageObjet | 1 | |
| CoefExpCompRessources | 15 | |
| CoefExpCompObjets | 20 | |
| CoefExpCompObjetsFabrication | 0.0025 | |
| CoefExpCompObjetsVie | 0.2 | |
| CoefExpCompObjetsEnergie | 2 | |
| CoefExpCompObjetsMagie | 0.2 | |
| CoefExpCompObjetsMoral | 10 | |
| CoefExpCompObjetsAttaque | 0.1 | |
| CoefExpCompObjetsDefense | 0.16 | |
| CoefExpCompObjetsResistance | 0.2 | |
| CoefExpCompObjetsBonus | 1 | |
| CoefExpCompPorteeBonus | 0.25 | |
| CoefExpCompSurfaceBonus | 0.25 | |
| CoefExpCompArmureBonus | 0 | |
| CoefExpCompServices | 20 | |
| NombreAttitudes, NombreFormations | 5, 5 | |
| LongueurPasMax | 16 | set in `Init_Carac` (`:579`) |

`ModDialogue.bas:20-23`: `ChanceRepondreAide = 3`, `ChanceRepondreOrdre = 0.1`,
`IntervalSoldatsX = IntervalSoldatsY = 24`.

Terrain (`Donnees/Francais/Terrain.ini`): `LargeurCase = HauteurCase = 16` px. Movement
coefficient `CoefVitesse` per terrain type: 0 Plaine 1, 1 Herbe 1, 2 Terre 1, 3 Désert 0.75,
4 Montagne 0.5, 5 Route 1.25, 6 Muraille1 0.75, 7–9 Muraille2–4 0.2, 10 Neige 0.75,
11 Eau 0.5 (with `Effet=159`, i.e. it applies object 158 as a temporary effect, §5.6).

`Jeu.ini`: `Vitesse=10` (game ticks per second), `Regle1=50` (Invocations_Multiples, in %),
`Regle2=1` (object recharge times enabled), `Regle3=1` (pathfinding: 0 none, 1 normal,
2 advanced), `Regle4=1`, `AfficherProjectiles=1` (if 0, objects never get `LanceProjectile`,
`ClsJeuParam.cls:1619`).

---------------------------------------------------------------------------------------------

## 2. Character state (`ClsJeuPerso.cls:119-413`)

There is one `ClsJeuPerso` per slot of the global array `Persos()`. Slot `i` owns the house
`Maisons(i)` and the effect slot `EffetsPersos(i)`; the arrays always grow together
(`ModCapacitesSpe.bas:371-398`). `Numero` is the slot index.

### 2.1 Identity and affiliation
- `Numero`, `Nom`, `Feminin`, `Race`, `Titre`, `Joueur` (network PlayerID; 0 = NPC; the local
  player has PlayerID 1), `ControleJoueur`, `Protege` (the save-file CRC is valid).
- `NumeroFief`: fief index. `NumeroEquipe`: team. Two persos are allies iff their `NumeroEquipe`
  are equal (`Perso_Allie`, `ModCombat.bas:919`). `Hostile = NumeroEquipe <= 0`
  (`:4471`).
- `NumeroChef`: −1 normally; ≥0 means this perso is an invocation (soldier, summon or projectile)
  of `Persos(NumeroChef)`. `Invocation = NumeroChef >= 0` (`:4486`).
- `NombreSoldats`: number of live non-projectile invocations whose chef is this perso.
  `ProjectileEnCours`: number of live projectiles.
- `Empereur` (accessed through `Definir_Empereur`).
- `Incommandable`, `Attitude` (0–4), `Formation` (0–4), `ChefDecalageX/Y` (offset in the chef's
  formation), `ChefPositionX/Y`.
- AI flags: `IA`, `ChoisirIA`, `FichierIA`, `ChoisirIAScript`, `FichierIAScript`, `NumeroIA`,
  `IA_Equiper` (Integer), `IA_Manger`, `IA_Pacifique`, `IA_Temeraire`, `IA_Protecteur`,
  `IA_Egoiste`, `Berserk` (private, read through `IA_Berserk`, which returns `BonusBerserk`),
  `IA_Ordre` (0 none, 1 attack perso, 2 attack house, 3 attack castle, 4 guard perso), `IA_Cible`.
- Race flags copied from the race: `Parole` (may speak), `Volant`, `GainExperience`,
  `Kamikaze`, `Pietinement` (trample strength), `Inattaquable`, `Projectile`,
  `ProjectileCible` (0 perso, 1 house, 2 castle), `RestaurationComplete`, `Immobile`
  (race speed is 0).
- Statistics: `FragsPersos`, `FragsBatiments`, `FragsChateaux`, `Morts`; `Bestiaire(race)`
  holds kill counts per race.

### 2.2 Primary bars (Vie, Energie, Magie, Moral)
For each X in {Vie, Energie, Magie, Moral}:
- `X As Double`: the current value.
- `MaxX As Long`: the **base** maximum (race base + one point per level).
- `BonusMaxX As Long`: the effective maximum = (MaxX + item bonuses) × race coefficient
  (§5.2). `Definir_MaxX` returns `BonusMaxX`.
- `RecuperationX As Double`: a pool of pending healing (from food, potions or resurrection),
  drained over time (§4.3).
- XP: `ExpX`, `NivX` (level), `ExpNivX` (XP threshold of the current level), `ExpNivPlusX`
  (threshold of the next level).
- `Definir_X` (Get) = `min(X + RecuperationX, BonusMaxX)`. It is the value used by the UI, by
  eating, and by the Vie check of the AI.

`Vivant = (Vie > 0)` (`:4386`). Many checks read the raw `Vie`.

### 2.3 Combat bars
- `Attaque As Double`, `MaxAttaque` (base), `BonusMaxAttaque` (effective); same for `Defense`.
  Attack works as an "energy bar" that is spent by hitting. Defense works as a shield bar that
  incoming attacks deplete. Both regenerate every tick (§4.2).
- XP: `ExpAttaque`, `NivAttaque`, … and `ExpDefense`, ….
- `PorteeAttaque` (base range, from the race) → `BonusPorteeAttaque` (`Definir_PorteeAttaque`).
- `Armure` (race) → `BonusArmure` (`Definir_Armure`): the floor of `Defense`.
- `EffetAttaque` (visual), `AttaqueResistance` (race, the index of the resistance that opposes
  this perso's attack) → `BonusAttaqueResistance`.
- `SurfaceAttaque` → `BonusSurfaceAttaque` (area radius; if > 0 every attack is an area attack).
- `AttaqueCauseEffet` (race) and `BonusAttaqueCauseEffet` (Collection of object indices applied
  as on-hit temporary effects).
- `AttaqueAutomatique` (from items), `ForcerAttaque` (the next hit ignores the "bar full" rule).
- Attack costs, summed over the equipped items: `PerteVie`, `PerteEnergie`, `PerteMagie`,
  `PerteMoral`, `PerteArgent`, `PerteRessourceIndice`, `PerteRessourceQuantite`.
- `Poison` (per slow tick), `PoisonLanceur`.
- `DureeRechargeAttaque` (cooldown counter, in ticks), `DureeRechargeAttaqueMax`,
  `DureeRechargeAttaqueObjet` (the sum of `DureeRechargeAttaque` over items and effects).
- `LanceProjectile` (race index of the projectile spawned by the weapon, −1 if none),
  `NombreProjectiles` (1 + bonus), `NumeroProjectileLance`, `NumeroObjetUtilise`
  (an object waiting to be processed by `Gestion_Utilisation_Speciale`, −1 if none).

### 2.4 Skills ("compétences")
Four families. Each one has a `Carac` (the base value that increases by one per level), `Exp`,
`Niv`, `BonusNiv` (the effective value), `ExpNiv` and `ExpNivPlus`.
- **Special skills** (`Competences_Speciales.ini`, 4 of them): 0 Construire bâtiment,
  1 Détruire bâtiment, 2 Coup critique, 3 Marchandage. `CaracSpeciales` starts at
  `PersosNiveauSpecialesDepart = 20`.
- **Resource skills**: one per resource. The "Carac" is `MaxRessources(i)`, the **carrying
  capacity**, which starts at `PersosMaxRessources = 20`. The level `NivRessources(i)` starts at
  0. The effective harvest skill `BonusNivRessources(i)` is a level (starting at 0, shown as
  level + 20).
- **Service skills**: `NombreServices − 1` of them (service 0 is "none"), starting at
  `PersosNiveauServicesDepart = 20`.
- **Object skills** (`Parametres_CompetencesObjets.ini`): index 0 "Sans", then clothes, shoes,
  gloves, jewels, tools, light weapons, edged, blunt, spear, heavy, axes, bows, marksman, ….
  They start at `CaracObjetsDepart = PersosNiveauObjetsDepart = 20`. The object skill scales
  every bonus of the objects that use it (§5.3).

`Definir_Niveau_Competence*` returns `BonusNiv*` as an **Integer**.

### 2.5 Resistances
`Resistances(k)` (Single, from the race, clamped to [0.2, 3]) and `BonusResistances(k)`
(the effective value). 8 resistances: 0 Physique, 1 Projectiles, 2 Feu, 3 Froid, 4 Lumière,
5 Mort, 6 Eau, 7 Electricité. **A resistance is a damage multiplier and a probability**: a lower
value means a better resistance. `Definir_Resistance(-1) = 1` (`:5254`).

### 2.6 Economy and inventory
- `Argent` (private Double, clamped to [0, 10 000 000] by `Definir_Argent`, `:4316`).
- `Ressources(i)` (Double, clamped to [0, `Definir_MaxRessources(i)` = `MaxRessources(i)` +
  `BonusMaxRessourcesObjet`] by `Definir_ressources`, `:3917`).
- `ObjetsEquipes(0..10)` (slots: 0 main hand, 1 second hand, then the slot equals the object
  category: 2 armour, 3 helmet, 4 shoes, 5 back, 6/7 rings, 8 arms, 9 necklace, 10 mount),
  `ObjetsInventaires(0..31)` (stacks with a `Nombre`, up to `Objet_Superposable`).
- Selection state used for giving, dropping and selling: `ObjetSelectionne`,
  `RessourceSelectionne`, `RessourcesSelectionnes()`, `ArgentSelectionne`.
- Crafting: `IndiceObjet`, `EtatFabricationObjet`. Generation: `GenerationRessources`,
  `GainArgent`, `GainRessource()`.
- `EffetsTemp` (Collection of `ClsJeuEffetTemp {Numero, Duree, DureeMax, Proprietaire}`) and
  `EffetsRecharge` (same class; `Numero` = object index, `Duree` = remaining cooldown).

### 2.7 Position and action
- `PositionX/Y` (Long, the top-left corner), `Largeur/Hauteur` (from the race), `DirectionX/Y`
  (destination), `CibleX/Y` (ground target), `Direction` (0 none, 1 up, 2 right, 3 down,
  4 left), `EnDeplacement`, `NumeroPas` (animation frame 0–3), `LongueurPas` (base speed),
  `BonusDeplacement`, `BonusVitesse`, `Acceleration`, `Longueur_Pas` (the current step),
  `Longueur_PasTemp` (the accelerated step), `DureeImmobilisation`, path state
  `CheminX/CheminY`, `CheminDirectionX/Y`, `CheminComplet`.
- Location flags: `DansSaMaison`, `DansUneMaison`, `DansSonChateau`, `DansUnChateau`.
  `Visible = Not (any of them)`; `Visible = v` sets all four flags to `Not v` (`:4371-4380`).
  `Attaquable = Vivant And Visible And Not Inattaquable`.
- Targets: `IndicePerso` (target perso), `IndiceMaison`, `IndiceChateau`, `IndiceRessource`,
  `IndiceTresor`.
- `Action` (Integer), see §3.
- `RessourceCourante` (−1 = none), `ServiceCourant`.
- `Vitalite` and `BonusVitalite` (§4.1).
- `DureeRessurection`, `ResurrectionNombre` (−1 = unlimited), `VitesseResurrection`.

---------------------------------------------------------------------------------------------

## 3. Actions: the state machine

### 3.1 Action codes (`ClsJeuPerso.cls:1987-2036`, comments)

| code | meaning | becomes on arrival (Deplacement3) |
|---|---|---|
| 0 | idle outside | — |
| 1 | walking (Aller_A) | 0 |
| 2 | harvesting outside | — |
| 3 | walking to a resource | 2 |
| 6 | building / repairing a house | — |
| 7 | walking to build | 6 |
| 100 | resting at home | (if it moves → 101) |
| 101 | going home to work | stays 101 → work |
| 102 | working at home (resource) | (if it moves → 103, a dead state that falls into `Case Else`) |
| 201 | entering a shop or house | handled by Travailler |
| 202 | returning to own castle | |
| 203 | going to an allied castle | |
| 204 | going to a treasure | |
| 205 | going to give to a perso | (handled in Deplacement1/2) |
| 206 | going to sell at a castle | |
| 207 | going to sell at a market | |
| 300 | attacking a perso | |
| 301 | attacking a house | |
| 302 | attacking a castle | |
| 303 | shooting at the ground (CibleX/Y) while walking to DirectionX/Y | |
| 400 | dead, resurrecting in the castle | |
| 500 | crafting an object at home | |
| 600 | preparing a service at home | |

`EnCombat = (300 <= Action < 400)`. `Inactif = (Action = 0 Or Action = 100)` (`:3933`).

Transition rule on arrival (`Deplacement3`, `:1241-1244`): if `Action` is odd and < 100, then
`Action -= 1`. While moving (`:1214-1223`): if `Action > 99` and `Action` is even and < 200,
then `Action += 1` (100→101, 102→103). If `Action = 7` and the target house is no longer
visible, the perso calls `Arreter`. Every moving tick also sets `Maisons(IndiceMaison).EnConstruction = False`
(`:1224`).

### 3.2 Orders (the public methods that set an action)

All orders except `Aller_Donner` and `Attaquer_Terrain` end with `If NombreSoldats > 0 Then
Ordonner_Chargez Numero` (soldiers follow the order, §9.3).

- `Aller_A(x, y)` (`:636`): `Direction = (x, y)`, `Orienter_Direction`, `Action = 1`,
  `Visible = True`. If the perso has soldiers and is alive, `Ordonner_Chargez` with
  `SansCommentaire`.
- `Arreter` (`:647`): `Direction = Position`, `Longueur_PasTemp = 0`,
  `Action = DansSaMaison ? 100 : 0`, `RessourceCourante = -1`. **If `IA And Visible`, it
  immediately calls `Comportement_IA_Persos(Numero, True)`** (the AI picks a new task re-entrantly).
  Note that `EnDeplacement` is not cleared.
- `Rentrer_Maison(m, Rentrer)` (`:670`): the destination is the house entrance
  `m.Pos + m.Size/2 − Size/2 + m.EntreePosition`. `Action = 101` if (the house is built and at
  full HP) or dead or `Rentrer`, else `7`. `IndiceMaison = m.Numero`, `Visible = True`.
  Nothing happens if the house is not visible.
- `Aller_Maison(m, Reparer)` (`:689`): same destination. `Action = 201` if (built and full) or
  dead or not `Reparer`, else 7.
- `Rentrer_Chateau(c)` (`:709`): **refused if `Hostile`**. Destination = castle entrance,
  `Action = 202`.
- `Aller_Chateau(c, i)` (`:724`): `Action = 203`, `IndiceChateau = i`.
- `Aller_Ressource(r, i, w, h)` (`:737`): destination
  `(r.X + w/2 − Largeur/2, r.Y + w/2 − Hauteur/2)` (note: `w` is used for Y too),
  `Action = 3`.
- `Aller_Tresor` (`:750`): the same pattern, `Action = 204`.
- `Aller_Donner(i)` (`:763`): `IndicePerso = i`, `Action = 205`, `Visible = True`.
- `Aller_Vendre(i, Marche)` (`:770`): sells immediately if already inside the right
  building, otherwise walks to it with `Action = 207` (market) or `206` (castle).
- `Attaquer_Perso(i)` (`:804`): `Action = 300`, `IndicePerso = i`. The perso stays hidden only
  if it is `DansUneMaison` in a house with `Tir` (it can shoot from the house), has range > 0
  and the target is within range; otherwise `Visible = True`. It sets
  `DureeRechargeAttaque = DureeRechargeAttaqueMax = DureeRechargeAttaqueObjet` and
  `ForcerAttaque = True`.
- `Attaquer_Maison(i)` / `Attaquer_Chateau(i)` (`:825`, `:835`): `Action` 301/302, the same
  cooldown and `ForcerAttaque`.
- `Attaquer_Terrain(x, y, type)` (`:845`): only if `LanceProjectile >= 0`. It sets
  `ProjectileCible = type`, `Action = 303`, `Visible = True` unless the perso is in a `Tir`
  house, the cooldown, `ForcerAttaque`, `Cible = (x, y)`, and orients toward the target. The
  walking destination is unchanged.
- `Attaquer_Allies(v)` (`:864`): if v, `NumeroEquipe = -Numero` (hostile to everyone) and
  `Arreter`; else `NumeroEquipe = Fiefs(NumeroFief).NumeroEquipe`. Every invocation of this perso
  copies the new team and calls `Arreter`.
- `Creer_Objet(n, house)` (`:2317`): if `n <> IndiceObjet`, reset the progress. `Action = 500`.
- Player commands (`ModCommandes.bas:146-242`) map onto these. They are queued and executed at
  the **start of the next tick** (`Lire_Commandes`).

### 3.3 Per-tick processing of one perso (`FrmMoteur2D.frm:1570-1595`)

For `i = 0 .. UBound(Persos)` (the bound is fixed at loop start):
1. If `IA_Berserk > 0 And Vivant`: `IA_ComportementBerserk(i)` (AI spec).
2. `RecuperationRapide` (§4.2).
3. `Deplacement1` (§3.4).
4. `Deplacement2` (§3.5), which may call `Deplacement3` (§3.6).
5. `Travailler` (§3.7).
6. If `NoMessage >= 0` (set by work functions): `If Rnd > 0.99 Or i = NoPerso Then` say
   `Commentaires.Message(NoMessage)`. **One Rnd is consumed per perso that has a pending
   message, plus one more if a comment is generated** (`Commentaires.Message` always draws
   `Int(Rnd * n)`, `ClsJeuCommentaires.cls:85-93`).

The full tick order (`FrmMoteur2D.frm:988-1177`) is: `DureePartie += 1` → `Lire_Commandes`
→ `Comportement_IA` → `Comportement_IA_Fiefs` → `Deplacer_Persos` (above) →
`TempVitesse += 1; if TempVitesse >= 10 { TempVitesse = 0; Actions_Persos (RecuperationLente
for every perso, index order); Actions_Tresors }` → `Gestion_Utilisation_Speciale` (§6.5) →
the "Jouer" events → visual updates → victory checks → for each local player
`Interface.Actions_Automatiques` (auto-eat when below a threshold, auto crafting queue,
auto-use of the current building's service; these only *queue* commands).

### 3.4 Deplacement1 — target tracking (`:973-1039`)

It always calls `Calculer_Vitalite` first (§4.1). Then, if `EnCombat Or Action = 205`:
- 205: if the target is (Visible or DansSaMaison) and alive, the destination is the target's
  centre aligned on our top-left: `T.Pos + T.Size/2 − Size/2`. Otherwise `Arreter`. Then
  `Orienter_Direction`.
- 300: if the target is visible and alive, the destination is the same centre formula.
  Else if the target is alive (it is hidden in a building): an IA switches to
  `Attaquer_Maison(T.Numero)` if `T.DansSaMaison`, `Attaquer_Maison(T.IndiceMaison)` if
  `T.DansUneMaison`, `Attaquer_Chateau(T.NumeroFief)` if `T.DansSonChateau`, or
  `Attaquer_Chateau(T.IndiceChateau)` if `T.DansUnChateau`. A non-IA calls `Arreter`. Then, if
  `Not Maisons(T.Numero).Tir`, the perso says message 15 (**Rnd consumed**). Else (the target is
  dead): `Arreter`, and a projectile dies.
- 301 / 302: track the building's centre; if it is no longer visible, `Arreter` (a projectile
  dies).

### 3.5 Deplacement2 — act if in range, else move (`:1040-1197`)

"In range" is a box test with the attack range `R = Definir_PorteeAttaque`:
`T.X + T.W >= X − R And T.Y + T.H >= Y − R And T.X <= X + W + R And T.Y <= Y + H + R`.
For 205 the range is R = 0 against `Persos(IndicePerso)`. For 300 it is tested against
`Persos(IndicePerso)`, for 301 against `Maisons(IndiceMaison)`, for 302 against
`Chateaux(IndiceChateau)`. **For 303 the test is always true.**

**Giving (205)** in range: if `ObjetSelectionne >= 0` and it is not inseparable, it is added to
the target's inventory (with auto-equip) and the selection is cleared. Then `Ressources_Donner`
transfers `ArgentSelectionne` and the selected resources (capped by the target's free capacity,
`:4792`), the target says message 46 (if `Parole`), and the giver calls `Arreter`.

**Attacking** (300–303) in range, and only if `DureeRechargeAttaque = 0`. Let
`PIF = (ProjectileEnCours > 0 And LanceProjectile >= 0)` (a projectile is already in flight) and
`T = Persos(IndicePerso)`. The perso attacks if (`:1091-1097`):

```
( Attaque >= 1 And not PIF And Attaque = Definir_MaxAttaque )        ' attack bar full
Or ForcerAttaque                                                     ' first hit after an order
Or ( not PIF And Action = 300 And
     ( if slot0 or slot1 has PerteSiAttaque:  Attaque >= T.Vie / CoefDegatPerso + T.Defense
       else:                                  Attaque >  T.Vie + T.Defense + 1 ) )   ' can finish
Or AttaqueAutomatique
```

(The literal expression is `Attaque >= 1 And (not PIF And Attaque >= (T.Vie+1)/0.5/
T.Resistance + T.MaxDefense Or Attaque = Max) And IIf(PIF, False, Attaque = Max) Or …`. The
middle factor is implied by the third, which gives the simplification above. `PerteSiAttaque` is
true for an object with any Vie/Energie/Magie/Moral/Argent cost, single use, or a resource cost,
`ClsJeuObjet.cls:211`.)

When the condition holds, the perso checks its cost reserves in this order:
`Vie > PerteVie`, `Energie > PerteEnergie`, `Magie >= PerteMagie`, `Moral > PerteMoral`,
`Definir_Argent >= PerteArgent`, `Ressources(PerteRessourceIndice) >= PerteRessourceQuantite`.
On the first failure the perso says the matching message (17 Vie, 18 Energie, 19 Magie,
20 Moral, 45 Argent, 34 resource) and calls `Objet_Ranger_Armes` (Arreter, then slots 0 and 1
go back to the inventory if there is room). On success:
1. If `LanceProjectile >= 0`: `NumeroProjectileLance = LanceProjectile` (the projectile is
   spawned later in the same tick, §6.5). If `Action = 303`, `Deplacement3` also runs (the
   perso keeps walking).
   Else if `BonusSurfaceAttaque > 0`: `Perso_Attaquer_Surface` (§6.4) centred on the target
   position (the centre argument is ignored, see §6.4).
   Else: `Perso_Attaquer_Perso(Me, T)` (300), `Perso_Attaquer_Batiment(Me, Maisons(IndiceMaison),
   Persos(IndiceMaison))` (301), `Perso_Attaquer_Batiment(Me, Chateaux(IndiceChateau),
   Persos(IndicePerso), IndiceChateau)` (302). For 303 without a projectile weapon, nothing.
2. **Only if slot 0 is active**: Vie, Energie, Magie and Moral −= (slot0 + slot1) per-object
   costs (through the `Definir_*` setters, so XP is gained). Single-use objects in slots 0 and 1
   are unloaded (`Decharger` decrements the stack).
3. `ForcerAttaque = False`; `Argent −= PerteArgent`;
   `Ressources(PerteRessourceIndice) −= PerteRessourceQuantite` (the aggregated values);
   `DureeRechargeAttaque = DureeRechargeAttaqueMax = DureeRechargeAttaqueObjet`.
4. If `Kamikaze`: `Vivant = False`.

After the in-range branch, a flying perso advances `NumeroPas` (mod 4).

**Otherwise** (not in range, or another action): `Deplacement3`, then if `EnDeplacement Or
Volant`, `NumeroPas = (NumeroPas + 1) mod 4`.

### 3.6 Deplacement3 — movement step (`:1199-1260`)

```
if DureeImmobilisation <= 0:
    Calculer_Longueur_Pas()
    if Marcher3():                     # still moving
        if Projectile: Collision_Projectile()
        apply the "while moving" action transitions (§3.1)
        Maisons(IndiceMaison).EnConstruction = False
    else:                              # arrived / not moving
        if Projectile and Vivant: Vivant = False
        Direction = Volant ? 3 : 0
        DirectionX/Y = PositionX/Y; Longueur_PasTemp = 0
        if Action odd and < 100: Action -= 1
        if Action = 100 and EtatFabricationObjet > 0 and PeutConstruire_Objet(...):
            Creer_Objet(IndiceObjet)   # resumes crafting (Action 500)
else:
    DureeImmobilisation -= 1
    if Projectile: Vivant = False      # not guarded by Vivant (double death possible, §8.1)
```

#### Step length (`Calculer_Longueur_Pas`, `:4337`)
Alive:
```
L = Int(BonusDeplacement * Vitalite)
if not Volant: L = Int(L * Monde.CoefVitesse(Me, PositionX, PositionY + Hauteur - 16))
if L < 1 and BonusDeplacement <> 0: L = 1
elif L > 16: L = 16
Longueur_PasTemp = min(Longueur_PasTemp + Acceleration, L)
Longueur_Pas = Longueur_PasTemp
```
Dead: `Longueur_Pas = 6` (no acceleration).

`Monde.CoefVitesse(perso, x, y)` (`ClsJeuMonde.cls:447`) reads the cell
`idx = CLng(x/16) + CLng(y/16) * Monde.Largeur` (**banker's rounding, not floor**; an x beyond
the map width spills into the next row). If `0 < idx < UBound(TerrainCategorie)` it returns
`Terrains(TerrainCategorie(idx)).CoefVitesse`, else 1. **As a side effect**, if the terrain has
an `Effet` (water) and the perso's resistance to that object's `AttaqueResistance` is above the
minimum (0.2), it calls `EffetTemp_Ajouter(effet, Duree, DureeMax = Duree)` (no Rnd). Because
this is called every tick for every live, non-flying perso that is not stunned (moving or not),
standing in water keeps the effect topped up at its full duration.

An example for a human (speed 4, acceleration 1) on grass: steps of 1, 2, 3, 4, 4, … px/tick.
A hit sets the defender's `Longueur_Pas = 0` (`ModCombat.bas:111`), but `Longueur_PasTemp` is
untouched and recomputed next tick, so the hit has no lasting slowdown effect. It does affect
the flee test in §6.1, which reads `Longueur_Pas` right after the hit.

#### Orientation (`Orienter_Direction`, `:1405`)
With `h = Longueur_Pas / 2`: if `ty < y − h` (up) → `Direction` 1, unless the target is also
beyond `x ± h` and `|dx| >= |dy|`, in which case 4 (left) or 2 (right). If `ty > y + h` (down)
→ 3, with the same left/right override. Else if `tx < x − h` → 4, `tx > x + h` → 2. If any case
matched, `EnDeplacement = True`; if none did, `EnDeplacement = False`.

#### Walking along the path (`Marcher3`, `:1489`)
If `EnDeplacement`: when `(DirectionX, DirectionY) <> (CheminDirectionX, CheminDirectionY)` or
`Not CheminComplet`, recompute the path (`Calculer_Chemin`) and store the destination. Let
`(wx, wy) = Chemin(1)`, `d = (wx − x, wy − y)` and `l = |d|`:
- if `l >= Longueur_Pas And l > 0`: `x = CLng(Longueur_Pas*dx/l + x)` (likewise y);
  `Visible = True`.
- else if the path has more than one waypoint: remove waypoint 1 and orient toward the new
  waypoint 1. If the distance to it is 0, snap to the final `Direction` and stop
  (`EnDeplacement = False`); otherwise take a full `Longueur_Pas` step toward it **from the
  current position** (the remainder of the previous step is lost).
- else: snap to `(DirectionX, DirectionY)` and set `EnDeplacement = False`.

It returns `EnDeplacement`. **No collision test is made while walking**: characters pass through
each other, buildings, resources and decor.

#### Pathfinding (`Calculer_Chemin`, `:1535`)
This is a **greedy local search, not A\***. The path is a list of waypoints.

```
clear Chemin; I = 16 (LargeurCase)
if not Volant and Vivant and Regle_Pathfinding > 0:
    TempPi = -1
    if Action = 300 and Persos(IndicePerso).EnDeplacement:
        NbEtapesMax = 2;   CheminComplet = False   # recomputed every tick while chasing
    else:
        NbEtapesMax = 1024; CheminComplet = True
    TempX = TempY = 0          # NOTE: initial values are 0, not the position
    while (|TempX − DirX| > 16 or |TempY − DirY| > 16) and count < NbEtapesMax and not Quitter:
        P = (count = 0) ? Position : Chemin(last)
        l1 = |P − Dir|
        for i in 0..7:   # order: UL, U, UR, L, R, DL, D, DR
            N = P + offset_i   # straight: ±16; diagonal: ±16*0.7 = ±11.2 → stored in Long (11)
            cost = 16 / CoefVitesseSimple(N.x, N.y + Hauteur − 16) + (|N − Dir| − l1)
            if i = 0 or cost < best: best = cost; TempRi = i; (TempX,TempY) = N
        if count = 0: append (TempX,TempY)               # TempPi stays −1
        elif TempPi = TempRi: replace the last waypoint with (TempX,TempY)
        elif count >= 2:
            if (TempX,TempY) = Chemin(count−1): Quitter = True   # oscillation
            else: append; TempPi = TempRi
        elif (TempX,TempY) = Chemin(count): Quitter = True
        else: append; TempPi = TempRi
else:
    CheminComplet = True
if count = 0: append Direction
else: replace the last waypoint with Direction
      if Regle_Pathfinding > 1: repeat Chemin_Supprimer_EtapesSuperflues while it removes something and count >= 3
```

`CoefVitesseSimple` is the same lookup as `CoefVitesse` without the effect side effect. The cost
of a neighbour is its traversal time (16 / speed coefficient: plain 16, road 12.8, desert or
snow 21.3, mountain or water 32, walls 80) plus the change in straight-line distance to the
destination. Ties keep the lowest index. Because `TempX/TempY` start at (0,0), when the
destination is within 16 px of the map origin on both axes no waypoint is generated (the path is
then the straight line to the destination).

`Chemin_Supprimer_EtapesSuperflues` (`:1680`, advanced mode only): for each inner waypoint i,
walk from waypoint i−1 toward waypoint i+1 in steps of `LongueurPas` (the base speed), sampling
`CoefVitesseSimple`. If every sample has the same coefficient as the start, remove waypoint i.
Stop when the step is within `LongueurPas` of waypoint i+1, or after a change of coefficient.

Flying persos, ghosts and the no-pathfinding rule go in a straight line.

### 3.7 Travailler — work at the current location (`:1270-1403`)

It only runs when `Not EnDeplacement`. Alive:
- 0: `RessourceCourante = -1`. If own castle not visible: `DansSonChateau = False`. If
  `Chateaux(IndiceChateau)` not visible: `DansUnChateau = False`. If `Maisons(IndiceMaison)` is
  not (visible and built): `DansUneMaison = False`.
- 2: `RessourceCourante = Ressources(IndiceRessource).TypeRessource`;
  `Recolter_Ressources(RessourceCourante, Maisons(Numero))`; `DansUneMaison = False`.
- 6: if the target house is visible: `EnConstruction = True` and `Batir_Maison`; else
  `Arreter`.
- 100: `DansSaMaison = MaisonCible.Visible`; if visible, `RessourceCourante = -1`.
- 101, 102, 600: if the target house is visible: `DansSaMaison = True`,
  `RessourceCourante = Batiment_RessourceFabrique(type)`. If ≥ 0: `Action = 102` and
  `Recolter_Ressources` (into the house). Else if the house has a `Service`: `Preparer_Service`.
  Else `Arreter`. If the house is not visible: `DansSaMaison = False`, `Arreter`.
- 204: `Tresor_Ramasser(IndiceTresor)`, `DansSaMaison = False`, `Arreter`.
- 500: if the house is visible, `Fabriquer_Objet`; else `DansSaMaison = False`, `Arreter`.
- 201: `DansUneMaison = visible And built`; `Arreter`.
- 202: `DansSonChateau = own castle visible`; `Action = 0`.
- 203: `DansUnChateau = target castle visible`; `Action = 0`.
- 206: set the castle flag, `Fiefs(IndiceChateau).Vendre_ObjetsSelectionnees`, `Action = 0`.
- 207: set the house flag, sell to `Fiefs(Persos(IndiceMaison).NumeroFief)`, `Action = 0`.
- else: `RessourceCourante = -1`.

Dead (ghost): 0 → the same flag clean-up; 100, 101, 102 → `DansSaMaison = visible`, `Arreter`;
201 → `DansUneMaison = …`, `Arreter`; 202 → `DansSonChateau = castle visible`, `Action = 0`
(this is how a ghost enters the castle to resurrect, §8.2).

### 3.8 Work formulas

**Harvesting / producing a resource** (`Recolter_Ressources`, `:2043`), per tick:
```
V = 1/400 * Vitalite * (BonusNivRessources(i) + 20)
q = Ressources_VitesseExtraction(i) * V
mp = Ressources_TypeMatierePremiere(i); qmp = Ressources_QuantiteMatierePremiere(i)
if Ressources(i) < Definir_MaxRessources(i):
    if mp >= 0: if Ressources(mp) >= q*qmp { Ressources(mp) -= q*qmp; Ressources(i) += q }
                else { Action = 100; NoCommentaire = 1 }
    else: Ressources(i) += q
    if Ressources(i) = Max and IA: Arreter
elif DansSaMaison and Maison.Stock < Maison.MaxStock: the same, but into Maison.Stock
else: Arreter; NoCommentaire = 0
if Action not in {0, 100}:
    if Bonus_CompetenceRessources(i):        # an equipped item boosts this skill
        V *= (NivRessources(i)+20) / (BonusNivRessources(i)+20)
        for each equipped object o with BonusCompetencesRessources(i) > 0:
            Gestion_Niveau(object skill o.Competence, V, type 8)
    Gestion_Niveau(MaxRessources(i), V, type 6)   # +1 carrying capacity per level
```
The resource node in the world is not depleted by this function.

**Service** (`Preparer_Service`, `:2122`): `S = Maison.Service`,
`V = 1/400 * Vitalite * BonusNivServices(S−1)`, `q = Service_VitesseCreation(S) * V`. If the
house stock is below max: consume raw material `q * Service_QuantiteMatierePremiere(S)` if
available, otherwise set `Action = 100, NoCommentaire = 1`, **but `Action = 600` is then set
unconditionally** (the raw material shortage does not stop the work; no stock is added that
tick). If the stock is full: `Arreter`. XP:
`Gestion_Niveau(CaracServices(S−1), q * Service_CoefExp(S), type 7)` and the boosting objects
(when `Action` ∉ {0, 100}).

**Building** (`Batir_Maison`, `:2204`), per tick:
`VB = 3 * Vitalite * (BonusNivSpeciales(0) + 20) / 20 * Batiment_CoefVitesseConstruction(type)`;
`Maison.Vie += VB`; XP `Gestion_Niveau(CaracSpeciales(0), VB * 0.00075 * 20, type 9)` and the
same amount to boosting objects. When `Construit And Vie = MaxVie`: own house → `Action = 101`;
someone else's → `DansUneMaison = True`, `Arreter`. If `Batiment_NoEpoque(type) > Fief.Epoque`,
`Fief.Changer_Epoque`. Otherwise `Action = 6`.

**Crafting** (`Fabriquer_Objet`, `:2331`), per tick, while `PeutConstruire_Objet` (house
stock ≤ MaxStock − 1 and the perso holds all resource costs):
```
obj = Batiment_ObjetFabrique(type, IndiceObjet); c = Objet_Competence(obj)
VF = 0.1 * Vitalite * BonusNivObjets(c)
EtatFabricationObjet += VF
xp = VF * 20 * 0.0025 * Objet_CoefXPFabrication(obj) * CaracObjets(c) / BonusNivObjets(c)
boosting objects and skill c gain xp (type 8)
if Etat >= Objet_DureeFabrication(obj):
    Etat = 0; Maison.Stock_Objets(IndiceObjet) += 1; NoCommentaire = 0; Action = 100
    pay all Objet_PrixRessources(obj, k)           # paid at completion
else: Action = 500
```
If crafting is impossible: `Arreter`.

**Eating** (`Manger_Ressources`, `:2175`): if `Ressources(i) >= 1`: `Ressources(i) −= 1` and
`RecuperationX += Ressources_GainX(i) * (BonusMaxX / MaxX)` for the 4 bars. `Recuperer_X`
(`:893`…) eats from the **first** edible resource with `GainX > 0` and at least 1 unit,
repeating until `Definir_X >= Definir_MaxX − GainX*coef` or the stock is below 1.
`Recuperer_AutoVie` triggers below 100% Vie; Energie, Magie and Moral trigger below 50%.

**Bargaining** (`Prix_Marchandee`, `:3800`):
`price = CInt(P / 2 * (1 + 20 / BonusNivSpeciales(3)))` (banker's rounding). At skill 20 it is
P; it tends to P/2. Buying an object from a house (`Objet_Acheter_Maison`, `:2402`) costs
`Prix_Marchandee(Objet_PrixVente * CoefficientAchat)` per unit, the money goes to the house, and
the buyer gains marchandage XP = price. Selling an object at the castle
(`Objet_Vendre_Chateau`, `:2518`) pays `Objet_PrixVente` (not bargained) and adds the object's
resource cost to the fief stock.

**Emperor**: `PeutDevenir_Empereur = Fief.Empereur < 0 And Argent >= Epoque_EmpereurPrix(Fief.Epoque)
And Vivant` (`:2271`). The command calls `Fief.Empereur_Changer(i, Payer=True)`
(`ClsJeuFief.cls:112`). Becoming emperor adds every `Objet_Empereur` object to the inventory
(no auto-equip). Losing the title (death, or another emperor) calls `Fief.Empereur_Changer(-1)`,
demolishes the perso's house if it is an emperor building, and destroys the emperor objects in
the equipment and the inventory.

---------------------------------------------------------------------------------------------

## 4. Recovery and regeneration

### 4.1 Vitalite (`Calculer_Vitalite`, `:3944`), every tick in Deplacement1
```
if not Vivant: Vitalite = 0
else:
  v = BonusVitalite           # 1 + Σ Objet_BonusVitalite (equipment + temporary effects)
  for r in (Vie/BonusMaxVie, Energie/BonusMaxEnergie, Moral/BonusMaxMoral):   # raw values
      if r < 0.1: v -= 0.45 elif r < 0.25: v -= 0.25 elif r < 0.5: v -= 0.125
  Vitalite = max(v, 0.1)
```
Vitalite multiplies speed, harvesting, building, crafting and service speed, combat bar
regeneration and combat fatigue. Since `RecuperationRapide` runs before `Deplacement1`, it uses
the previous tick's Vitalite.

### 4.2 RecuperationRapide — every tick (`:1929`)
If alive:
- if `Attaque < Max`: `Definir_Attaque += 1.25 * Vitalite * BonusMaxAttaque / MaxAttaque`
  (clamped to the max; an increase gives no XP).
- if `Defense < Max`: `Definir_Defense += 0.75 * Vitalite * BonusMaxDefense / MaxDefense`
  (clamped to [BonusArmure, max]).
- if `DureeRechargeAttaque > 0`: decrement it (floor 0).

### 4.3 RecuperationLente — every 10th tick (`:1770`)
Alive:
1. If `EnDeplacement`: `Energie −= 0.01`, `Moral −= 0`; if `Pietinement > 0`:
   `Perso_Pietinement` (§6.6).
   Else, by `Action`:
   - 0: `Vie += 0.0005*MaxVie`, `Energie += 0.0005*MaxEnergie` (Magie and Moral +0).
   - 2: `Energie −= 0.0025*(NivRessources(RC)+20)`, `Moral −= 0.0005*(NivRessources(RC)+20)`
     (the raw level, not the boosted one).
   - 6: `Energie −= 0.0025*CaracSpeciales(0)`, `Moral −= 0.0005*CaracSpeciales(0)`.
   - 100: `Vie += 0.001*Max`, `Energie += 0.001*Max`, `Moral += 0.00025*Max`.
   - 102: if `RC >= 0`, as in case 2; else ×`CaracServices(ServiceCourant−1)`.
   - 500: ×`CaracObjets(competence of the object being crafted)`.
   (The `Max` values are the effective `BonusMax*`; every write goes through the `Definir_*`
   setters, so losses give XP, §7.)
2. Drain the food pools: for each bar X, with `s = VitesseRecuperationX * BonusMaxX` (0.0375,
   0.05, 0.05, 0.025): if `RecuperationX >= s` then `X += s; RecuperationX −= s`, else
   `X += RecuperationX; RecuperationX = 0`.
3. Combat fatigue: if `Attaque < Max`: `Energie −= 0.125*Vitalite`, `Moral −= 0.025*Vitalite`.
   The same again if `Defense < Max`.
4. Proportional regeneration: `X += BonusVitesseRegenerationX * BonusMaxX` for the 4 bars, where
   `BonusVitesseRegenerationX` = race `VitesseRegenerationX` + Σ object/effect bonuses. Races
   default to 0 except Magie (0.0025 for most races); `Infatigable` races get 0.01 for Energie
   and Moral (`ClsJeuParam.cls:2342`).
5. `Generer_Ressource`: if `GenerationRessources`, `Argent += GainArgent` and
   `Ressources(k) += GainRessource(k)`.
6. `Gestion_Effets_Temporaires` (§5.6), then `Gestion_Effets_Recharge`.
7. Poison: if `Poison >= 1`: `Definir_Vie = Vie − Poison` (display the damage); if the perso dies
   and `PoisonLanceur >= 0`, `Perso_Tuer_Perso(Persos(PoisonLanceur), Me)` (loot and kill
   credit go to the poisoner).

Dead: see §8.2.

---------------------------------------------------------------------------------------------

## 5. Bonuses, equipment and effects

### 5.1 Ajuster_Bonus (`:3351`): reset to base values
`BonusVitesseRegenerationX = race value`, `BonusDeplacement = LongueurPas`, `BonusVitesse = 0`,
`BonusMaxX = MaxX` (the 6 bars), `BonusPorteeAttaque = PorteeAttaque`, `LanceProjectile = -1`,
`BonusNivSpeciales = CaracSpeciales`, `BonusNivRessources = NivRessources`,
`BonusNivServices = CaracServices`, `BonusNivObjets = CaracObjets`,
`BonusResistances = Resistances`, `BonusMaxRessourcesObjet = BonusMaxRessourcesRace`,
`BonusAttaqueResistance = AttaqueResistance`, `BonusSurfaceAttaque = SurfaceAttaque`,
`BonusArmure = Armure`, `Monture = 0`, `BonusVolant = Volant`.

### 5.2 Actualiser_Bonus (`:3061`): recompute everything
It is called on every equip/unequip, every level-up, and every add/remove of a temporary effect.
1. Clear the object appearances; `BonusVitalite = 1`; `BonusBerserk = Berserk`;
   `GenerationRessources = False`; `AttaqueAutomatique = False`;
   `BonusAttaqueCauseEffet = [AttaqueCauseEffet]` if ≥ 0, else empty; `GainArgent = 0`;
   `GainRessource = 0`.
2. Save the old maxima `T_X = BonusMaxX` (and `T_Deplacement`).
3. `Perte* = 0`, `PerteRessourceIndice = 0`, `Poison = 0`, `PoisonLanceur = -1`,
   `DureeRechargeAttaqueObjet = 0`, `NombreProjectiles = 1`.
4. `Ajuster_Bonus`.
5. Object-skill bonuses: for each active equipped object (slots 0..10), for each object skill j:
   `BonusNivObjets(j) += BonusSup(BonusNivObjets(o.Competence), o.BonusCompetencesObjets(j))`.
   Then the same for each temporary effect (with the perso's own skill). This is order-dependent:
   it reads the partially updated array.
6. `BonusNivObjets(j) = CLng(BonusNivObjets(j) * CoefRaceCompObjets(j))`.
7. `Ajouter_Bonus_Objet` for equipped slots **2..10**, then slots **0, 1** (so the weapons win
   the "last value" fields), then each temporary effect `(Numero, EffetTemp=True,
   Proprietaire)`.
8. Race scaling: `BonusMaxX = CLng(BonusMaxX * CoefRaceX)` for the 6 bars;
   `BonusNivSpeciales(j) *= CoefRaceCompSpeciales(j)`;
   `BonusNivRessources(j) += 20*CoefRaceCompRessources(j) − 20`;
   `BonusNivServices(j) *= CoefRaceCompServices(j)` (all assigned to Long).
9. Floors: `BonusDeplacement <= 1` → `Immobile ? 0 : 1`; every other `BonusMax* <= 0` → 1.
10. `BonusDeplacement += BonusVitesse`.
11. Rescale: `X = X * BonusMaxX / T_X` and `RecuperationX = RecuperationX * BonusMaxX / T_X`
    (for Vie, Energie, Magie, Moral, and also Attaque and Defense without a pool). Then
    `Attaque = min(Attaque, BonusMaxAttaque)`; **if `Armure > BonusMaxDefense` then
    `BonusMaxDefense = Armure`** (the race armour, not BonusArmure);
    `Defense = min(Defense, BonusMaxDefense)`.
12. Clamp every `Ressources(i)` to `Definir_MaxRessources(i)`.

### 5.3 Ajouter_Bonus_Objet (`:3208`)
`C` = the object skill level of `Objet_Competence(n)` of the perso, or of `Persos(Proprietaire)`
for an effect cast by someone else. Scaling functions (`:3392-3440`), with `D = CaracObjetsDepart = 20`:
- `BonusSup(C, V) = Int(V*C/D)` (an Integer; since `CoefExpCompObjetsBonus = 1`).
- `GainSup(C, V) = V*C/D` (a Single, no rounding).
- `PorteeSup(C, V) = SurfaceSup(C, V) = Int((V*C/D − V)*0.25 + V)`.
- `ArmureSup(C, V) = Int(V)` (coefficient 0).
- `InvocationSup(C, V) = max(1, (V*C/D − V)*(Regle1/100) + V)`, with Regle1 = 50.

Contributions:
- Movement: for a non-flyer, if `BonusSup(C, BonusDeplacement) >= 1`, then
  `BonusDeplacement = LongueurPas + BonusSup(...)`. This **replaces** the value, so the last
  object processed wins: slots go 2..10, then 0 and 1, so a mount (slot 10) overrides shoes
  (slot 4) unless a weapon also has a movement bonus. For everyone, if
  `BonusSup(...) < 0`, then `BonusDeplacement = LongueurPas + raw BonusDeplacement` (raw, not
  scaled).
- `BonusVitesse += BonusSup(C, BonusVitesse)`; `BonusVitalite += BonusVitalite(raw)`.
- `BonusMaxX += BonusSup(C, BonusX)` for Vie, Energie, Magie, Moral, Attaque, Defense.
- If `Objet_AttaqueResistance > 0` (strictly; index 0 "Physique" cannot be set this way) and it
  is not an effect: `BonusAttaqueResistance = it` (last wins).
- `BonusPorteeAttaque += PorteeSup(C, BonusPorteeAttaque)`.
- If `Objet_LanceProjectile > 0`: `LanceProjectile = it`.
- `BonusSurfaceAttaque += SurfaceSup(C, SurfaceAttaque)`.
- If `Objet_AttaqueCauseEffet >= 0`: add it to `BonusAttaqueCauseEffet`.
- `AttaqueAutomatique |= ...`; if `Objet_Berserk > 0`: `BonusBerserk = it`.
- If not an effect: `Perte* += raw costs`; `PerteRessourceIndice = idx if idx > 0`;
  `PerteRessourceQuantite += q`.
- Poison: if `Objet_Poison > 0`: `Poison += max(1, BonusSup(C, Poison))`; if
  `Proprietaire >= 0`, `PoisonLanceur = Proprietaire`.
- `DureeRechargeAttaqueObjet += raw DureeRechargeAttaque`.
- `NombreProjectiles += BonusSup(C, ProjectileSupplementaire)`.
- Special, resource and service skills: `+= BonusSup(C, ...)`.
- If `GainArgent > 0`: `GainArgent += GainSup(C, ...)` and `GenerationRessources = True`
  (likewise per resource).
- `BonusArmure += ArmureSup`; `BonusMaxRessourcesObjet += BonusSup(C, BonusMaxRessources)`;
  `BonusVitesseRegenerationX += raw`.
- Resistances: for each k with `Objet_BonusResistance(n, k) <> 0`:
  `BonusResistances(k) = clamp(BonusResistances(k) − bonus, 0.2, 3)`.
- Appearance / mount bookkeeping (display; `Monture` = the appearance index).
- If `Objet_Volant`: `BonusVolant = True`.

### 5.4 Equipping (`Objet_Equiper`, `:2648`)
A mount object is refused unless `Race_UtilisationMonture`. For a **non-usable** object:
`Equipement_Libre` (`:2895`) tells whether its slot is free (category 0: not both hands used and
no two-hander; category 1: both hands free; 6/7: not both ring slots used; else the slot equals
the category). If the slot is not free, the displaced item(s) are unequipped (category 0
replaces slot 1 when `CoteDroit`, otherwise slot 0; a two-hander frees 0 and 1; rings behave
like category 0 on slots 6/7; other categories free their own slot). The new item is loaded with
the hand-shuffling rules of `:2715-2788`. The source inventory stack is decremented (or
`ObjetSelectionne = -1`), the displaced items go back to the inventory, any slot-1 item is dropped
when slot 0 is a two-hander, and `Actualiser_Bonus` runs. The function returns True.

For a **usable** object (potions, scrolls, spells), when `EffetRecharge_Pret(n)` holds and
`(Vivant Or UtilisationMort) And NumeroObjetUtilise = -1 And Not (Vivant And SelfResurrection)`:
the cost checks are `Vie > PerteVie Or Not Vivant`, `Energie > PerteEnergie`,
`Magie >= PerteMagie`, `Moral > PerteMoral`, `Argent >= PerteArgent`,
`Ressources(idx) >= q` (messages 17/18/19/20/45/47 on failure). On success:
`NumeroObjetUtilise = n`; object-skill XP `+= Objet_Experience(n)`; pay Vie/Energie/Magie/Moral
(with XP), the stack is unloaded if single use, pay money and resources; if
`Objet_DureeRecharge > 0`, `EffetRecharge_Ajouter(n, DureeRecharge)`. The actual effect is
applied later in the tick by `Gestion_Utilisation_Speciale` (§6.5).

`EffetRecharge_Pret(n)` (`:5444`): true if `DureeRecharge(n) = 0` or there is no recharge entry
for n. Otherwise, if `RechargesMultiples(n) = 0`, false; else ready iff
`Int(BonusSup(C, RechargesMultiples) − remaining / DureeRecharge) > 0`. `EffetRecharge_Ajouter`
adds the duration to an existing entry (charges stack).

New items (`Objet_Ajouter_Inventaire`, `:2574`): placed in the first free slot, or stacked on
the first stack of the same object that is below `Objet_Superposable`. If
`NouveauObjet And IA_Equiper <> 0 And Equipement_Libre And Not Utilisable`, it is equipped
immediately.

`Objet_Ranger(slot)` (`:3000`): if the inventory is not full, the item goes back to the
inventory, `Actualiser_Bonus`, and Attaque/Defense are clamped to the new max.

### 5.5 Resistances in practice
- Race resistances: they start at `Defaut = 1`. The listed `Statut`s override the non-default
  entries in list order; explicit `Resistance(k)=` keys apply before the statuses. Then everything
  is clamped to `[Minimum 0.2, Maximum 3]` (`ClsJeuParam.cls:2366-2380`). **So an "immunity" of
  0 becomes 0.2**: damage is ×0.2, and effects never apply because the effect test requires
  `resistance > Minimum`.
- Statuses (`Parametres_Resistances.ini`): 1 Petit (PH 1.25, PR 0.75), 2 Grand (PH 0.75,
  PR 1.25), 3 Inflammable (FE 2), 4 Mouillé (FE 0.65, FR 1.25, EL 1.1), 5 Animal (FE 1.25,
  FR 0.75), 6 Mort vivant (PR 0.75, FR 0.5, LU 1.5, MO 0), 7 Esprit (LU 0.5, MO 2), 8 Esprit
  d'air (PR 0.5), 9 Esprit de feu (FE 0, FR 1.5), 10 Esprit d'eau (EA 0), 11 Esprit de terre
  (PH 0.75, PR 0.75, FE 2, EA 0.75), 12 Esprit de mort (LU 1.5, MO 0.5), 13 Dragon (FE 0.5,
  FR 1.5), 14 Artificiel (PH 0.5, PR 0.5, MO 0), 15 Machine (PH 0.75, PR 0.75, EA 2, EL 3,
  MO 0). The `Statut=` key in a race takes the 1-based status number; `Statut=0` clears the list.
- The attack resistance index of a race defaults to **0 (Physique)** when not given (an unset
  Integer field, `ClsJeuParam.cls:1947`). The object default is −1 (`:1301`).

### 5.6 Temporary effects (`EffetsTemp`)
- `EffetTemp_Ajouter(No, Duree, DureeMax=-1, Proprietaire=-1→Numero)` (`:5318`): if an entry with
  the same `No` **and** the same owner exists, `Duree += Duree` and, if `DureeMax > -1` and the
  sum exceeds it, clamp both `Duree` and `DureeMax` to `DureeMax`. Otherwise a new entry is
  created with `DureeMax = Duree` and `Actualiser_Bonus` runs. The duration setter caps at
  1 000 000 and raises `DureeMax`.
- Ticking (`Gestion_Effets_Temporaires`, `:1957`), once per slow tick and only while alive: for
  i = 1..count, if `Duree <> -1` (−1 = permanent), decrement it; if it reaches ≤ 0, erase it,
  `Actualiser_Bonus`, and **`Exit For`** (the effects after it skip their decrement this slow
  tick). Recharges (`:1976`) work the same way, with no −1 case and the same early exit. Units
  are slow ticks: one unit = 10 game ticks = 1 s at the default speed.
- `EffetTemp_Effacer(i)`: if `Objet_EffetAnnuleRestaureObjet`, the object is given back
  (`Objet_Ajouter_Inventaire_Fast`); then remove it and `Actualiser_Bonus`.
- At death, `EffetTemp_Effacer_Mort` removes every effect with `Objet_EffetAnnulationMort`.
- An effect is an object index. Its bonuses apply through `Ajouter_Bonus_Objet` with
  `EffetTemp = True` (no costs, no attack-resistance override), scaled by the **caster's** object
  skill (`Proprietaire`).

`ClsJeuEffet` and `ClsJeuDegat` are purely visual (sprite animation and floating numbers). They
have no gameplay effect. `ClsJeuCompetences` is a UI-only record.

---------------------------------------------------------------------------------------------

## 6. Combat

### 6.1 Perso_Attaquer_Perso(A, D, Experience=True) (`ModCombat.bas:83-325`)
```
Att = A.Attaque; if A.Projectile: Att = min(Att, Persos(A.NumeroChef).Attaque)
Def = D.Defense
r1 = Rnd                                                    # always drawn
if r1 > 0.05 or (A.Kamikaze and not A.Projectile):           # success
    CoupDEtat = D.Empereur and A.NumeroFief = D.NumeroFief
    visual effect A.Effet_Attaque on D
    D.DureeImmobilisation = 0 (Combat.ini); D.Longueur_Pas = 0
    Temp = D.Vie / 0.5 + D.Defense − 1                       # attack points "needed"
    res = D.Definir_Resistance(A.BonusAttaqueResistance)
    if Att > Def:
        r2 = Rnd
        if r2 > 0.05:   dmg = (Att − Def) * 0.5 * res
        else:           # critical hit
            k = (A.Projectile ? chef : A).BonusNivSpeciales(2) / 20
            dmg = (Att * (k + 1) − Def) * 0.5 * res
            the shooter (chef for a projectile) says message 2 (Rnd)
            if Experience: shooter.Gestion_Niveau_Comp_Speciale(2, dmg)
        dmg = min(dmg, D.Vie); D.Definir_Vie = D.Vie − dmg     # display dmg
        D.Gestion_Niveau_Resistances(A.BonusAttaqueResistance, dmg)
    else:
        D.Definir_Vie = D.Vie − 1                             # a hit that does not pierce does 1
    D.Definir_Defense = Def − Att                             # clamped to [D.BonusArmure, D.max]
    if not D.Vivant: kill handling (§6.2)
    else: on-hit effects and reactions (§6.3)
else:                                                         # critical failure
    the shooter says message 3 (Rnd); Temp = 20
# pay the attack
if A.Projectile:
    chef.Definir_Attaque −= Temp  (with XP)  if Experience, else  chef.Attaque = chef.Definir_Attaque − Temp (no clamp)
else: A.Definir_Attaque −= Temp (with XP, clamped at 0)  /  A.Attaque = A.Definir_Attaque − Temp
```
Notes:
- The attacker pays `min(Attaque, D.Vie/0.5 + D.Defense − 1)`: it only spends what the kill
  needed, computed **before** the damage and from the defender's current Vie and Defense. The
  setter clamps at 0. On a critical failure it pays 20.
- The crit multiplier at the starting skill (20) is ×2 on the attack (`k = 1`, plus
  `CoefDegatCoupCritique = 1`).
- The damage is fractional (Vie is Double).
- A blow that kills gives no Vie XP to the victim (the setter kills first, §7.2).
- `Inattaquable` is not checked here; it only filters target searches and area attacks.

### 6.2 Kill handling (`ModCombat.bas:170-231`)
If D is an invocation, `Not A.IA_Pacifique`, `A.Vitalite = 1` (exact equality) and D's chef is
alive: an IA attacker switches to `Attaquer_Perso(D.NumeroChef)` and says message 44. If the
chef has soldiers and is an IA, `Ordonner_Chargez(chef, 0, A.Numero, Rnd > 0.5, Rnd > 0.5,
Rnd > 0.8)` (three Rnd, left to right, as Ctrl/Shift/Alt; this only happens when the chef is an
IA with soldiers).

Credit:
- If A is an invocation: if `A.Kamikaze Or A.Projectile`, the chef's bestiary +1, and if the
  chef's house is a built `Service = 2` (mercenary), `Maisons(chef).Service_Utiliser(chef, Score,
  Experience=True)` with
  `Score = (D.MaxAtt + D.MaxDef + D.MaxVie) * Service_Coefficient(2) * (chef.BonusNivServices(1) + 20) / 20`.
  Otherwise A's own bestiary +1. Then `Perso_Tuer_Perso(chef if alive else A, D, CoupDEtat)` and
  the chef's `FragsPersos += 1`.
- Else: `Perso_Tuer_Perso(A, D, CoupDEtat)`, bestiary +1, `FragsPersos += 1`; mercenary bonus if
  A's own house is a built service-2 house, with
  `Score = (…) * Service_Coefficient(2) * ((A.BonusNivServices(1) − 20 + 1)/20/10 + 1)` (a
  different formula from the invocation case). `Service_Utiliser` case 2 adds Score to the
  house's money and gives `Gestion_Niveau_Prime(Score)`, i.e. service skill 1 XP
  `Score * Service_CoefExp(2)`.

`Perso_Tuer_Perso(A, D, CoupDEtat)` (`:327-427`) — loot:
1. If `Persos(D.IndicePerso)` is in A's team, is not A, and D was attacking it (`D.Action = 300`),
   it thanks A (message 14, Rnd).
2. `Pille` = the boxes of A and D overlap. If not, a treasure is created at D's centre.
3. Money: `0.25 * D.Argent` goes to A (or to the treasure); `D.Argent *= 0.75`.
4. Resources: each one, 0.25 to A/the treasure, D keeps 0.75 (A's gain is capped by A's
   capacity; the excess is lost).
5. Equipped slots 0..10, then inventory 0..31: for each active, non-indestructible item,
   `r = Rnd; if r < 0.25`: `r' = Rnd; if r' > 0.5` transfer it (to A's inventory with
   auto-equip, or to the treasure); then **destroy it at D in every case** (the slot or one unit
   of the stack: `Decharger` decrements a stack). Note `Rnd < 0.25 And Not Indestructible`
   always draws its Rnd for every active item, even an indestructible one.
6. Coup d'état: if `CoupDEtat`, `Fiefs(A.NumeroFief).Empereur_Changer(A.Numero)` (free) and
   `A.Attaquer_Allies(False)`.
7. An empty treasure is erased; otherwise `Tresors_Verifier` (merging with overlapping
   treasures).
8. If `Race_Tombe(D.Race) >= 0`, a tomb decor is created with the label
   `"&" & D.Nom & " (" & EtiquetteNiveau & " " & Int(D.Niveau) & ")" & (fem/masc label) & A.Nom`.

The death itself (`Vivant = False`) has already happened inside `D.Definir_Vie` (§8.1).

### 6.3 Defender not killed (`ModCombat.bas:232-304`)
1. On-hit effects: for each `e` in `A.BonusAttaqueCauseEffet` (in order):
   `res = D.Resistance(Objet_AttaqueResistance(e))`; if `res > 0.2` (no Rnd otherwise) and
   `Rnd < res`: `D.EffetTemp_Ajouter(e, dur, dur, owner)` with
   `dur = (Objet_Duree(e) = -1) ? -1 : A.GainSup(A.BonusNivObjets(Objet_Competence(e)), Objet_Duree(e))`,
   `owner = A.Projectile ? A.NumeroChef : A.Numero`. So a resistance above 1 means a certain
   effect, and 0.5 a 50% chance.
2. If `D.IA`:
   - If `D.Action = 300`: retarget only if the attacker (the chef for a kamikaze invocation or a
     projectile) is closer in Manhattan distance than D's current target.
   - Else: `D.Attaquer_Perso(attacker or chef)`.
   - Then: `if D.Vie < D.MaxVie/2 And D.Vie < A.Vie And Not D.Invocation And Rnd > 0.2`
     (Rnd always drawn): `Appeler_De_lAide(D)` (§9.2); then
     `if D.Longueur_Pas >= A.Longueur_Pas And D.Vie < A.Vie And Rnd > 0.9 And Not D.IA_Temeraire`
     (Rnd always drawn): flee to its built house, else to its visible castle, else
     `IA_Deplacement_Aleatoire`, and A says message 13. Because `D.Longueur_Pas` was just set
     to 0, the flee only happens when `A.Longueur_Pas = 0`.
3. Else (a human-controlled or non-IA defender): if `D.Inactif Or D.Action = 2`, it counter-attacks
   automatically (the chef for a kamikaze invocation).

### 6.4 Perso_Attaquer_Surface(A, X, Y, Rayon, …) (`:606-661`)
X and Y are **ignored**; distances are measured from A's position. With `init = A.Attaque` and
`TempAttaqueMin = 0`:
- Persos: every i with `NumeroEquipe <> A's` and `Attaquable` and
  (`dist(A.pos, P_i.pos) < Rayon` or `i = A.IndicePerso`): `Perso_Attaquer_Perso(A, P_i, False)`,
  then `A.Attaque = init`.
- Houses: every i with `Persos(i).NumeroEquipe <> A's`, the house visible, and
  `dist((h.X + h.W/2, h.Y + h.W/2), A.pos) < Rayon` (sic: W used for Y) or `i = A.IndiceMaison`:
  `Perso_Attaquer_Batiment(A, Maisons(i), Persos(i))`.
- Castles: `Fiefs(i).NumeroEquipe <> A's`, visible, distance from the castle's top-left corner or
  `i = A.IndiceChateau`: `Perso_Attaquer_Batiment(A, Chateaux(i), Persos(A.IndicePerso),
  NoChateau)`. **`NoChateau` is the parameter (−1 unless the order was 302), not `i`**, so a
  castle destroyed this way is processed as if it were a house (looting etc.).
- The "minimum" update `If TempAttaqueMin > A.Attaque` can never fire (it starts at 0), so at the
  end `A.Definir_Attaque = 0`: **an area attack always empties the attack bar** (with attack XP
  for the whole bar).

### 6.5 Projectiles, used objects and invocations
`Gestion_Utilisation_Speciale` (`ModCapacitesSpe.bas:20`), once per tick, for every perso
(index order, bound fixed at the start):
1. `If .NumeroProjectileLance >= 0 And Not .ProjectileEnCours`: `Not` on the Integer
   `ProjectileEnCours` is bitwise, so the condition is true unless `ProjectileEnCours = -1`.
   `Creer_Invocation(NumeroProjectileLance, NombreProjectiles, Numero, Effet_Attaque,
   Projectile=True)`, then `NumeroProjectileLance = -1`.
2. If `NumeroObjetUtilise >= 0` (`Objet = n`, `C = BonusNivObjets(Objet_Competence(n))`):
   - If `Objet_Invocation(n) >= 0`:
     `Creer_Invocation(inv, InvocationSup(C, Quantite), i, EffetAttaque, False,
     Int(InvocationSup(C, QuantiteMax)), InvocationIA_Fichier)`.
   - If `PorteeUtilisation > 0`: `Effets_Proches(...)` (below), with the gains
     `BonusSup(C, GainX) * Manger_Ressources_Coef(X)` and the range `PorteeSup(C, Portee)`.
     Else: if `SelfResurrection`, `Ressusciter_Perso(i)`; `RecuperationX += BonusSup(C, GainX) *
     coef`; a visual effect if it is not an invocation.
   - If `Objet_Impot`: `Prelever_Impots` (every live perso of the same fief within range pays
     `Argent * Quantite`, and houses with `Argent >= 1/Quantite` pay too; the caster receives the
     total).
   - Self effect: if `Resistance(Objet_AttaqueResistance(n)) > 0.2` and `Rnd < it`:
     `Effet_Sur_Perso(self, n, C, 0, i)`, which applies `Objet_CreerObjet(n)` as a temporary
     effect (when `Objet_Duree(n) <> 0`, duration `GainSup(C, Duree)` or −1) or gives the
     object `Quantite` times.
   - `NumeroObjetUtilise = -1`.

`Effets_Proches(L, ami, ennemi, zone, gains…, Resurrection, Quantite, …)` (`:427`): it repeats
`max(1, Quantite)` times. Each time it scans all persos `i <> L` that qualify:
`(ami >= 0 And alive And same team) Or (ennemi >= 0 And alive And other team) Or (same team And
((Vie < Max And alive And Resurrection = 0) Or (dead And Resurrection > 0 And Not Invocation)))`,
and that are `Attaquable` (so they must be alive, which makes resurrection by area use
impossible) and closer than the best so far. For each such candidate it draws
`Rnd < Resistance(res)` (only when the resistance is > 0.2). A candidate that passes and has not
been affected before becomes the best. If the best is within `zone`: resurrect (unreachable),
add the gains to its Recuperation pools, `Effet_Sur_Perso(target, ami or ennemi, C, 1|2, L)`
(`CreerObjetAllie`/`CreerObjetEnnemi`), and record it. If no target is found, the loop stops.

`Creer_Invocation(inv, Quantite, chef, effet, Projectile, QuantiteMax, FichierIA)` (`:148`):
1. `n = Int(Quantite) + (Rnd < frac(Quantite) ? 1 : 0)` (always 1 Rnd).
2. For each of the n:
   a. If `QuantiteMax > 1`: scan the persos with `NumeroChef = chef` and `Race = inv` (**dead or
      alive**); on the QuantiteMax-th one found, `Vivant = False` (even if it is already dead,
      which double-decrements `NombreSoldats`).
   b. Reuse the first slot j holding a dead invocation whose house is not visible (`Init(j)`),
      else `Perso_Ajouter_Nouveau` (which also calls Init).
   c. Copy the chef's fief and team; set `Projectile`; `Changer_Race_Personnage(inv)`;
      `Changer_Race_Personnage_Equipement` (the race talent objects with PrixTalents ≤ 0, then
      auto-use). `IA = Not Projectile`; set the AI file if given; `IA_Ordre = 0`;
      `NumeroChef = chef`.
   d. Projectile: copy the chef's `MaxAttaque` (→ its `Definir_MaxAttaque`), its demolition skill
      level as the projectile's `CaracSpeciales(1)`, and for each of the chef's on-hit effects the
      chef's object skill level for that effect's skill.
   e. `Actualiser_Bonus`; `Vivant = True` (full Vie, Attaque and Defense; `NombreSoldats++` for a
      non-projectile); Energie, Magie and Moral full; appearance (display Rnd, §11).
   f. Projectile: `chef.ProjectileEnCours++`; the position is the chef's centre. For the first
      projectile, the aim point is the centre of the chef's target (by chef.Action 300/301/302,
      with `ProjectileCible` 0/1/2), else `(chef.CibleX, chef.CibleY)` with `chef.Action = 1`
      (the chef keeps walking) and `ProjectileCible = chef.ProjectileCible`. With `l = max(1,
      CLng(|aim − pos|))` (l is a Long), `Aller_A(pos + 2*chef.Range/l*(aim − pos))`: the
      projectile flies to twice the chef's range along the aim line. Extra projectiles reuse
      that aim and l (computed from their own position, which is the same), offset by
      `±i*32` on both axes (`+` for even i, `−` for odd i).
      Non-projectile: offset in an 8-branch star (`i mod 8`: right, left, down, up, UL, UR, DL,
      DR; the distance is `(Int(i/8)+1)` times the chef's width or height); the position is
      `chef.pos + offset`; copy the chef's current order (300/301/302) or
      `Aller_A(chef.Direction + offset)`; a visual effect; it says message 33 (Rnd).
3. If not a projectile: `Ordonner_Formation(chef, , True)`.

Projectile flight: a projectile is a perso with `IA = False` and `Action = 1`, usually flying.
Each moving tick, `Collision_Projectile` (`ClsJeuPerso.cls:1724`) calls
`Obstacle(Numero, pos, size)`, which returns the **first** of: an enemy attackable perso (9), a
treasure (13), a castle (6/8/11), a house (4/5/7/10), a resource (1), a decor (16), terrain
(999). Only 9–11 count, and only when they match `ProjectileCible` (0↔9, 1↔10, 2↔11), so a
treasure lying under an enemy house hides it. Persos are tested first. On a perso hit:
`Perso_Attaquer_Perso` (or the surface version); the projectile dies only if it is `Kamikaze` or
`chef.Attaque = 0`, otherwise **it pierces** and can hit again on later ticks. On a building hit
the projectile always dies. On arrival at its destination it dies. A projectile's damage uses
`min(projectile.Attaque, chef.Attaque)` and drains the **chef's** attack bar. Its crits use the
chef's crit skill.

### 6.6 Trampling (`Perso_Pietinement`, `:798`)
On every slow tick while moving: for each perso i, first `if A.Attaque < Pietinement then
A.Definir_Attaque = Pietinement` (re-arm). If i is an enemy, attackable, overlaps A
(`Est_Superpose`) and `A.Pietinement > i.Pietinement`: `Perso_Attaquer_Perso(A, P_i, False)`.
Then enemy-owned visible houses that overlap get `Perso_Attaquer_Batiment(A, M_i, P_i)` (with
the same re-arm), and enemy visible castles that overlap get `Perso_Attaquer_Batiment(A, C_i,
Persos(A.IndicePerso), i)`.

### 6.7 Attacking buildings (`Perso_Attaquer_Batiment(A, B, owner, NoChateau=-1)`, `:451`)
```
Att = A.Projectile ? chef.Attaque : A.Attaque
Temp = B.Vie
if Temp > 20 and (not A.Kamikaze or A.Projectile): Temp = 20
if Temp > Att: Temp = Att  elif Temp < 1: Temp = 1
dmg = Temp * 1 * A.BonusNivSpeciales(1) / 20          # the projectile's own copy of the chef's skill
B.Definir_Vie = B.Vie − dmg
(the shooter = chef or A).Attaque −= Temp              # raw, no clamp, no combat XP
the shooter gains Gestion_Niveau_Comp_Speciale(1, Temp)  # demolition XP = Temp*0.05
```
(`Defense = A.Defense` is read and unused.) Because `Temp >= 1` is forced, a `ForcerAttaque` hit
with `Att < 1` makes Attaque negative.

If the building is destroyed (`Not B.Visible`): `A.Arreter`, `B.EnConstruction = False`. For a
castle (`NoChateau >= 0`): a visual effect; each live member of that fief says message 12 with
`Rnd > 0.6` (one Rnd per perso in the array, because of the `And`); `FragsChateaux++` (the chef's
for an invocation). For a house: `Perso_Piller_Batiment` (the chef for an invocation; a treasure
with `0.5 * B.Argent`, `0.5 * stock` of the produced resource for shops with
`RessourceFabrique > 0`, and each stocked object with probability 0.5, `CoefPillageObjetsMaison > Rnd`),
`FragsBatiments++`, the owner says message 5, a visual effect; then, if A is an IA and the owner
was `DansSaMaison`, with `Rnd < 0.9` A attacks the owner.

If not destroyed: for a castle, a visual effect at a random offset (2 Rnd); for every live,
non-invocation, non-egoist member of that fief, `If .IA And Rnd > 0.92` they attack the attacker
(or its chef for a kamikaze invocation), then `If Rnd > 0.9` they say message 11; then
`If Rnd > 0.9`: `Appeler_De_lAide(A)`. For a house: a visual effect (2 Rnd); if the owner is a
live IA and not an invocation, `If owner.Action <> 300 And Rnd > 0.75` the owner attacks A; then
`If B.Vie < B.MaxVie/2 And Rnd > 0.65`: `Appeler_De_lAide(owner)`; the owner says message 4.
The damage number display also draws 1 Rnd (`:489`).

---------------------------------------------------------------------------------------------

## 7. Experience and levels

### 7.1 Gestion_Niveau(Carac, Gain, Exp, Niv, ExpNiv, ExpNivPlus, type) (`ClsJeuPerso.cls:3840`)
```
if GainExperience:
    Exp += Gain
    while Exp >= ExpNivPlus:
        Carac += 1; Niv += 1; ExpNiv = ExpNivPlus
        ExpNivPlus += (Niv + 1) * K[type]
        Actualiser_Bonus()
```
`K` = 20 (Vie, Energie, Magie, Moral), 400 (Attaque), 250 (Defense), 15 (resources),
20 (services), 20 (objects), 20 (special skills). The initial `ExpNivPlus = K`. So **level n is
reached at total XP `K * n(n+1)/2`** (Vie: 20, 60, 120, 200, …; Attaque: 400, 1200, 2400, …).
Each level adds +1 to the base value (`MaxVie`, `MaxAttaque`, `CaracSpeciales(i)`,
`MaxRessources(i)` (carrying capacity), …), before the item bonuses and the race coefficient.
Races without `Experience = 1` have `GainExperience = False`: they never gain XP and never level
up, even from a loaded save.

### 7.2 XP sources
- **Vie** (`Definir_Vie` setter, `:3493`): if the new value is ≤ 0 and the perso is alive, it
  dies first (`Vivant = False` sets Vie = 0) and gains nothing. Otherwise, for a loss,
  `XP = (Vie − new) * MaxVie / BonusMaxVie`, and objects with `BonusVie > 0` gain
  `XP * 0.2` (only if `Definir_BonusVie = Int(BonusMaxVie − MaxVie*CoefRaceVie) > 0`). Then
  `Vie = min(new, BonusMaxVie)` (no lower clamp).
- **Energie**: `XP = loss` (not normalised); objects ×2. Clamped to [0, max].
- **Magie**: `XP = loss * MaxMagie / BonusMaxMagie`; objects ×0.2. Clamped.
- **Moral**: `XP = loss`; objects ×10. Clamped.
- **Attaque** (`Definir_Attaque`, `:3607`): first clamp to [0, max]; for a decrease,
  `XP = loss * MaxAttaque / BonusMaxAttaque`; objects ×0.1.
- **Defense**: clamp to [BonusArmure, max]; for a decrease, `XP = loss * MaxDefense /
  BonusMaxDefense`; objects ×0.16.
- **Special skills** (`Gestion_Niveau_Comp_Speciale(idx, Score)`, `:3763`):
  `XP = Score * coef * 20`, with coef 0.00075 (build), 0.0025 (demolish), 0.02 (crit, with
  `Score` capped at the current `Attaque`), 0.001 (bargain). Boosting objects get
  `XP * {0.05, 1, 1, 1}`. Building calls `Gestion_Niveau` directly (§3.8).
- **Resistances** (`Gestion_Niveau_Resistances(k, Score)`, `:3822`): only if
  `BonusResistances(k) < Resistances(k)` (an item improves it). Each equipped item with
  `Objet_BonusResistance(n, k) > 0` gains `Score * MaxVie / BonusMaxVie * 0.2 * bonus`.
- Resources, services and crafting: §3.8. Using an item: `Objet_Experience`. Mercenary bounty:
  `Gestion_Niveau_Prime`.

So almost every action trains something: walking trains Energie, being hit trains Vie and
Defense, hitting trains Attaque, working trains Energie, Moral and the job skill.

### 7.3 Derived level and talents
`Niveau = 1 + 0.1*(NivVie+NivEnergie+NivMagie+NivMoral+NivAttaque+NivDefense) + 0.01*Σ(levels of
every special, resource, service and object skill)` (`:4497`). It is shown on tombs and used by
the AI.
`Defnir_Score_Tresor = Int(Σ all skill levels / 12)` (`:4532`): the "fortune" budget for
choosing talents (race objects with `PrixTalents`) in the pre-game screen
(`FrmParam.frm:4392`).
`Metier` (`:4550`) is the label of the playable building whose skill (the average over its
crafted objects' skills, its service skill or its resource skill, including the fractional
progress) is the highest. It is display only.

### 7.4 Gestion_Tous_Niveaux (`:3691`)
It calls `Gestion_Niveau(…, Gain=0, …)` on every bar and skill, which replays the level-ups for
the stored XP. It is used after loading a save and after the difficulty XP bonus (§10.3).

---------------------------------------------------------------------------------------------

## 8. Death, ghosts and resurrection

### 8.1 Vivant = False (`:4434-4468`)
`Morts += 1`; clear the comment; `Arreter`; `Vie = 0`; the race's death visual effect; all
`Recuperation* = 0`; `DureeImmobilisation = 0`; `DureeRessurection = 100`;
`EffetTemp_Effacer_Mort`; `Attaquer_Allies(False)` (back to the fief team). For an invocation:
the chef's `ProjectileEnCours−−` (projectile) or `NombreSoldats−−`, and if its house is visible
but not built, the house is demolished. Otherwise `Fiefs(NumeroFief).NombreMorts += 1`. An
emperor loses the title. **The setter does not check the previous state**: setting it on an
already dead perso repeats all of this (Morts, NombreMorts, NombreSoldats decrements).

Equipment is kept, apart from what looting took (§6.2).

### 8.2 Ghost phase (`RecuperationLente`, dead branch, `:1887-1926`)
A dead perso still moves (6 px/tick, straight line, through everything) and can be ordered to
`Rentrer_Chateau` (unless hostile). Every slow tick:
- If `DansSonChateau`: if the castle is visible: if `ResurrectionNombre > 0 Or = -1`:
  with `NC = Fief.NombreCitoyens` and `NM = Fief.NombreMorts`,
  `DureeRessurection −= (NC = NM) ? VitesseResurrection / NC : VitesseResurrection / (NC / (NC − NM))`.
  If it is ≤ 0: `Vivant = True` and decrement `ResurrectionNombre` (unless −1). Else
  `Action = 400`. If no resurrections are left: `Action = 0`. If the castle is destroyed:
  `Action = 0`, `DansSonChateau = False`.
- Else if it is an invocation in its own visible house: `DureeRessurection −= VitesseResurrection`;
  at ≤ 0, `Vivant = True`.
- Else in confrontation mode: `Vivant = True` and a random position
  `(Rnd * (W*16 − Largeur), Rnd * (H*16 − Hauteur))`.

`VitesseResurrection = 2.5 * Resurrection speed option` (0.25, 0.5, 1, 2, 4, 100,
`Partie.ini [Resurrections_Vitesse]`). `ResurrectionNombre` comes from the option (0, 1, 3, 10,
25, 100, −1) (`ModMain.bas:752`). Example: 10 citizens, 1 dead, normal speed:
`2.5 * 9/10 = 2.25` per slow tick → 45 slow ticks ≈ 45 s.

### 8.3 Vivant = True (`:4392-4433`)
`Arreter`; `Vie = BonusMaxVie`; `Definir_Attaque = max`; `Definir_Defense = max`; for an
invocation (not a projectile) the chef's `NombreSoldats++`, else `Fief.NombreMorts−−`. If
`RestaurationComplete`: resources are raised to at least the race's starting resources, missing
race equipment is given back, and Energie, Magie and Moral are set to full. Else in confrontation
mode: E, M and M full. Else: Energie = max only if it was 0, and Moral = max only if it was 0.

Other resurrection paths: the temple service (`Service = 1`, `ClsJeuBatiment.cls:305`:
`Vivant = True`, then `Vie = Service_Coefficient(1) * max`, one stock unit spent, paid at the
bargained price unless it is the owner); `Ressusciter_Perso` (self-resurrection objects and
berserk mode 3): `Vivant = True`, `Vie = 5%`, `RecuperationVie = 95%` of max.

---------------------------------------------------------------------------------------------

## 9. Relations between characters

There is no loyalty or diplomacy system. The relations are:

### 9.1 Teams, fiefs, emperor, hostility
- Allies = equal `NumeroEquipe`. A fief has one team; several fiefs may share a team (it is
  chosen in the pre-game setup). Every target search, area attack, trample and help call uses
  team equality. Direct attacks do not check teams.
- `Attaquer_Allies(True)` makes the perso hostile to everyone (`NumeroEquipe = -Numero`) until
  it dies or revokes it. A hostile perso cannot enter its castle.
- `Perso_Hostile` (`ModCombat.bas:875`, the "become independent" command) creates a new fief with
  team `max(existing teams) + 1`, the same people type and epoch, `NombreCitoyens = 1`; it is
  removed from the old fief's counts; its soldiers follow (`IA_Ordre 4` → 0).
- The emperor (one per fief): bought (§3.8) or taken by killing the emperor while in the same
  fief (a coup d'état, §6.2).

### 9.2 Calls for help (`Appeler_De_lAide(p)`, `ModDialogue.bas:25`)
For every IA perso i of p's team (array order):
- If `IA_Protecteur And NombreSoldats > 4`: send its soldiers (`Ordonner_Chargez(i, type,
  target)`) at p's current target (300 → the perso, 301 → the house, 302 → the castle). For
  other actions, look for the first j with `(Action = 300 And IndicePerso = p) Or (Action = 301
  And IndiceMaison = p And Rnd < 0.5)` (**one Rnd per j scanned**, until found) and charge it.
  Comments: p says 6/7/8/9 if p is an IA, i says 10.
- Else, if not an invocation, not pacifist and not egoist: if `Action < 102 And Vitalite = 1`
  and `Rnd < 3 / (Fief(p).NombreCitoyens + 1)`: i itself attacks p's target, with the same case
  analysis.

### 9.3 Chef and soldiers (invocations)
Soldiers are invocations created by objects (spells, golems, conscription, pacts, …). The
`NumeroChef` link is permanent (save files store it). `NombreSoldats` counts the live ones.
- `Ordonner_Chargez(chef, type=-1, target=-1, Ctrl, Shift, Alt, SansCommentaires)` (`:134`): the
  default type and target come from the chef's action. `NombreMax`: Shift+Ctrl 10, Shift 5,
  Ctrl 1, else all. Every soldier with `NumeroChef = chef`, alive, not in combat, not
  incommandable, `Attitude = 0`, `IA_Ordre = 0`, without its own AI file, and not harvesting
  (Action ≠ 2, 3): type 0/1/2 → `Attaquer_*` and `IA_Ordre = 1/2/3`, `IA_Cible = target`;
  otherwise `Aller_A(chef.Direction + ChefDecalage)`. Each soldier draws
  `Rnd < 0.1 And Not SansCommentaires` (always drawn) for message 31. The chef says 25–30 or 23.
- `Ordonner_Formation(chef, change, …)` (`:371`): `Rangee = CLng(Int(Sqr(NombreSoldats)) * 1.5)`
  (min 1). Formation 0: `Rangee /= 1.5`, start `X = 24*R/2 − 12`, `Y = 24*R/2 + 12`; 1 and 2:
  `X = 24*R/2 − 12, Y = 0`; 3 and 4: `X = 0, Y = 24*R/2 − 12`. For each eligible soldier (alive,
  `IA_Ordre = 0`, `Attitude = 0`, commandable, no AI file), with the running count j: formations
  0/1: on `j mod R = 0` → `Y −= 24, X −= 24*(R−1)`, else `X += 24`; formation 2 is the same with
  `Y += 24`; 3: on wrap `X −= 24, Y −= 24*(R−1)`, else `Y += 24`; 4: the same with `X += 24`. A
  computed offset of (0,0) is skipped (`i = i − 1`, so the same soldier is reprocessed with the
  next offset). It sets `ChefDecalage` and, if the chef is not fighting, `Aller_A(chef.Direction
  + offset)`.
- `Ordonner_Arret`, `Ordonner_Deplacement`, `Ordonner_Attaquer_*`, `Ordonner_Garder_Perso`
  (`IA_Ordre = 4`), `Ordonner_Activer_IA`: they act on explicit selections and queue commands.
- When a soldier is killed, its killer may switch to the chef, and an IA chef counter-charges
  (§6.2).
- Demolishing the house of one of your soldiers kills it (`ClsJeuBatiment.cls:170`).

### 9.4 Recruitment
There is no separate recruitment mechanic: soldiers come from using invocation objects
(`Objet_Invocation`, quantity scaled by object skill and Regle1, capped by `QuantiteMax`), bought
or crafted like other objects. Pre-game, the player chooses the fief composition. NPCs are
generated per fief (§10.2).

---------------------------------------------------------------------------------------------

## 10. Character creation

### 10.1 Init(i) (`ClsJeuPerso.cls:427`)
`Numero = i`; `NumeroPas = Int(Rnd*3)` (**Rnd #1**); `Argent = 0`; `Attitude = 0`;
`NumeroChef = -1`; the selections reset; `SurfaceAttaque = 0`; frags and deaths reset;
`ResurrectionNombre = -1`; `VitesseResurrection = 2.5`; `Kamikaze`, `Projectile`,
`Pietinement`, `Inattaquable` reset; `DureeRechargeAttaque = 0`; `NombreProjectiles = 1`;
`IndiceRessource = 0`, `IndiceChateau = NumeroFief`, `IndicePerso = Numero`; `Init_Carac`;
bars full; `Ressources`, `Resistances` and the object slots are allocated; effects and recharges
are cleared; `Changer_Race_Personnage(0)` (human; **Rnd for the sex, plus a name Rnd if
`Joueur <= 0`**); `Ajuster_Bonus`; `Calculer_Vitalite`; the bestiary is allocated; `Arreter` (may
call the AI if the reused object still has `IA = True`); a new `ClsJeuIA3`; `IA`, `ChoisirIA`
and `ChoisirIAScript` set to False.

`Init_Carac` (`:518`): it keeps the current bar ratios when all maxima are > 0 (else 1); resets
`Parole = True`, `Volant = False`, `GainExperience = True` and the AI flags; sets the maxima to
the `Persos*` defaults and restores the bars by ratio; zeroes all XP and levels, with
`ExpNivPlus = K`; `MaxAttaque = 20`, `PorteeAttaque = 0`, `MaxDefense = 20`, `LongueurPas = 4`,
`LongueurPasMax = 16`, `Acceleration = 1`; `Init_Competences` (Carac to the defaults, the
`ExpNivPlus*` to their K).

### 10.2 Changer_Race_Personnage(race, SansEquipement) (`:4965`)
1. Remember the bar ratios (0 when the max is 0).
2. `Race = race`. **Sex**: `If Rnd > 0.5 And Not Race_RestrictionSexe(race, True)` →
   `Feminin = True`, else False. (`RestrictionFeminin` forbids female; the male restriction is
   not consulted here. The INI key read is `RestrictionMaculin`; the 11 entries spelled
   `RestrictionMasculin` and the 2 spelled `RestrictionMasulin` in `Parametres_Races.ini` are
   ignored.)
3. If `Joueur <= 0`: `Nom = Noms.Tirer_Nom_Aleatoire(Feminin)` if `Race_NomAleatoire`, else the
   race name (by sex). The name draw is `NomsX(Rnd * L)`, with the index CLng-rounded, where L =
   the last index of `NomsM.ini`/`NomsF.ini` (one name per line; the first and last names have
   half probability).
4. Copy from the race: size; appearance settings; `MaxVie..MaxDefense` (the race base, default 20
   via `Verification_Donnees`); the 6 `CoefRace*`; the regeneration speeds;
   `RestaurationComplete`; the skill coefficients (default 1); `Resistances`; `GainExperience`;
   `Volant`; `Parole`; specific comments; `BonusMaxRessourcesRace`; `PorteeAttaque`;
   `EffetAttaque`; `AttaqueCauseEffet`; `AttaqueResistance`; `Armure`; `LongueurPas`
   (`Race.Vitesse`; −1/unset → 4); `Immobile = (LongueurPas = 0)`; `Acceleration` (≤ 0 → 1);
   `Kamikaze`; `Incommandable`. Note that the levels and skill Caracs are **not** reset (a race
   change keeps the XP-derived +levels, but `MaxVie` etc. are overwritten by the race base
   **without** the level bonus until `Gestion_Tous_Niveaux` runs).
5. Unless `SansEquipement`: `Ressources(i) = max(current, Race_Ressources(i))`; each race
   `Equipement` object is added to the inventory (auto-equip if `IA_Equiper > 0`), then
   `Actualiser_Bonus`.
6. Mounts the race cannot use are unequipped. `Ajuster_Bonus`. Then `Pietinement`,
   `Inattaquable`, `IA_Equiper` (`Race.Equiper`), the AI flags, `IA_Berserk`, and the AI file
   (`ChoisirIA` if one is given).
7. Restore the bars by ratio (only where the ratio is > 0).

`Changer_Race_Personnage_Equipement` (`:5121`): adds the race's "talent" objects (the race
`Objet(n)` list plus the 14 common objects of `[Races]`) that have `PrixTalents <= 0`; an object
that changes the appearance resets it. Then `Objets_Utilisation_Automatique`: every inventory
object with `UtilisationAutomatique` is used (`Objet_Equiper`) and **`Gestion_Utilisation_Speciale`
runs immediately for all persos**.

### 10.3 NPC generation for a new world (`ModMain.bas:408-464, 615-630, 716-756`)
Per perso (fief by fief): `Init(i)`, then `Tirer_Monstre_Aleatoire(i, people type)`
(`ModGenererMonde.bas:65`):
- If the people type has monsters: for `k = 0 ..`, `if Rnd < MonstreChance(k)` pick
  `MonstreIndice(k)` (and its AI file) and stop. If none was picked (or the picked race index is
  0), take the last monster.
- Else (humans): `if Rnd < ChancesRaceAleatoire (0.3)`: `t = Int(Rnd * NombreRacesJouables)`;
  scan the races `1..NombreRaces−1`, decrementing t on each playable race, and pick when
  `t <= 0`. (t = 0 and t = 1 both give the first non-human playable race, which therefore has
  double weight.) Otherwise human (race 0).
- Then `Changer_Race_Personnage(race)` and the people-specific AI file.
- `Perso_Charger_Ressources_Depart` (resource mode 1 only): unless `DepartSansRessources`, every
  resource of the fief's epoch that is needed for building gets the configured starting amount,
  and the money is set.
- `IA = Not Projectile`. For an NPC without an AI file and with `Race_IA_Selectionner`:
  `If NiveauIA (=1) > Rnd` (always true, but it draws a Rnd) then
  `FichierIA = FileIA.List(Int(Rnd * ListCount))`. Then `Actualiser_Bonus`.
- The difficulty XP bonus (NPCs only, `Joueur = 0`): `ExpVie = ExpEnergie = ExpMagie =
  ExpMoral = B`, `ExpAttaque = ExpDefense = 20*B`, every special, resource and service skill and
  every object skill from index 1 = B; `GainExperience` is forced on for `Gestion_Tous_Niveaux`
  and then restored. B = 0, 1100, 4200, 9300, …, 101000, 10010000 (`Partie.ini`; B = 1100 gives
  level 10 everywhere, since `20*10*11/2 = 1100` and `400*55 = 22000`).
- Later (`FrmMoteur2D.frm:324-338`): `Perso_Finalier` (appearance, AI loading) and, for NPCs,
  `Changer_Race_Personnage_Equipement`; then `Actualiser_Bonus` for all.

### 10.4 Player characters and the save file (`ModSavPerso.bas`)
A saved character (`<CheminSavPerso>/<Nom><ext>`, section `SectionPerso`) stores **only**:
`Sexe` ("Féminin"/"Masculin"), `Race`, `Equipe`, `Apparence`, `FichierApparence`,
`FichierApparenceBase<n>`, `IA`, `FichierIA`, `ChoisirIAScript`, `FichierIAScript`,
`Attitude`, `Formation`, the XP values `Vie`, `Energie`, `Magie`, `Moral`, `Attaque`, `Défense`,
`CompétenceSpéciale<n>`, `CompétenceRessource<n>`, `CompétenceService<n>`, the object skills
under their `SAVPerso` key (e.g. `CompétenceObjet25` for "Sans"), `BestiaireMonstre<IndiceBestiaire>`,
and a CRC under `XPBestiaire`. The `SectionTalents` section holds `Tresor<n>` = 1 for each
chosen race talent object. **There is no equipment, inventory, money or resources: a character
starts every game with its race kit and talents, and only its XP persists.**

Loading (`Charger_Sauvegarde_Perso`, `:50`): remember the bar ratios → `Init_Carac` →
`Changer_Race_Personnage(Race)` (the sex Rnd is overwritten next) → `Nom`, `Feminin`, `NumeroFief
= Equipe` → appearance and AI → the XP values → `Gestion_Tous_Niveaux` → restore the ratios →
`Actualiser_Bonus` → `Protege = (CRC matches)`. Then `Charger_Sauvegarde_Tresor` adds each
chosen talent object not already owned and auto-uses it.

The CRC (`PersoToCRC`, `:465`): a string `Nom & Hex(Val(ChoisirIA)) & Hex(Val(Feminin)) &
Hex(Val(ExpVie)) & … & Hex(Val(ExpDefense))`, then `Hex(Val(x))` for each skill XP ≥ 1 (object
skills from mod files excluded) and each bestiary count ≥ 1 (mod races excluded). `Hex()` of a
Double rounds to a Long. `CRC16` sums `Asc` of each character; `CRC = (Not (S mod 65536)) + 1`;
the result is `Right(Hex(CRC), 2) & Mid(Hex(CRC), 5, 2)` read as hex. Saving: the CRC is written
only for a new character or a protected one. The character is saved at the end of a game if
`Jeu.Duree > 10`, and periodically if `SauvegardePeriodique`.

### 10.5 "Générer 1 personnage" (`FrmParam.frm:2325`)
The buttons generate 1, 10 or 50 characters. For each one:
1. `LstRaces.ListIndex = Int(Rnd * LstRaces.ListCount)` (the list holds the playable races, or
   all races if `JouerToutesLesRaces`). Setting it fires `LstRaces_Click`: for a sex-restricted
   race the sex buttons are disabled and forced (female-restricted → male, otherwise female).
2. If the sex buttons are enabled: `If Rnd < 0.5` → male, else female.
3. `TxtNomPerso = Noms.Tirer_Nom_Aleatoire(female)` (1 Rnd). `TxtNomPerso_Change` disables
   creation if the name is empty, contains `\ / : * ? " < > | .`, or matches an existing
   character (case-insensitive).
4. If creation is enabled, `CmdCreerPerso_Click` (`:3218`):
   `Persos(NoPerso).Init` (Rnd: NumeroPas, sex; `Joueur = 1`, so no name Rnd) →
   `Nom = UCase(first letter) & rest` → `Changer_Race_Personnage(race)` (sex Rnd) →
   `Feminin = the option` → `ChoisirIA = False`, `FichierIA = FichierIADefaut` →
   `AffApparence.Tirer_Apparence_Au_Hasard` (for each of the race's appearance categories,
   `Int(Rnd * number of images in its folder)`) → save with a CRC → write the default interface
   settings (`AutoBatimentService = 1`, `CompetencesTri = 2`) → select the new character in the
   list (which reloads it) → preselect every free talent (`PrixTalents <= 0`), which saves the
   `Tresor` flags. Then `FileApparence.ListIndex = Rnd * (ListCount − 1)` and
   `FileIA.ListIndex = Rnd * (ListCount − 1)` (CLng-rounded; each fires a click that stores the
   file and saves).
   If creation is disabled (e.g. a duplicate name), the iteration is retried (`i = i − 1`), with
   at most 1000 total attempts.
The result is a level-1 character (all XP 0) of a random race, sex, name, appearance and AI file.

### 10.6 Race table (`Parametres_Races.ini`, 106 races; blank = default)
Defaults: Vie, Energie, Magie, Moral, Attaque and Defense 20; coefficients 1; speed 4;
acceleration 1; armour 0; range 0; magic regeneration 0.0025 (when set). The playable races:

| # | race | CoefVie | CoefEnergie | CoefMagie | CoefMoral | CoefAtt | CoefDef | speed | other |
|---|---|---|---|---|---|---|---|---|---|
|0|Humain| | | | | | |4| |
|1|Nain|1.25|1.25|0.75| | |1|3|craft and mining skill coefs|
|2|Elfe|0.75|0.75|1.5| |1|1| | |
|3|Demi-Elfe| |0.75|1.25| |1|1| | |
|4|Colosse|1.5|1.25|0.25| |1.5|0.5| | |
|5|Kobold|0.75|1.25| | |0.75|0.75| | |
|6|Félin|0.75|1.25| |1.25|0.75|1|5|Vie regen 0.0005|
|7|Fée|0.25| |2| |0.25|1.5|6|flying, status Petit|
|8|Drake|1.25|0.75|1|0.75|1.25| | |range 3, status Dragon|
|9|Garou| | | | | | | |Vie and Magie regen 0.0015|
|10|Ombre|0.5| |1.5| |0.5|1.5| |status Mort vivant|
|11|Celeste|0.75| |1.25|0.75| | | | |
|80|Centaure|1.25|1.25|0.75|1|1.25|0.75|6| |

The other races are monsters, summons and projectiles (e.g. 83 Boule de feu, 84 Flêche,
85 Boulet de canon, 86 Pierre, 87 Balle, 89 Orbe de givre: flying kamikaze projectiles with speed
8–16 and acceleration 10–16). A port must load the INI rather than hard-code these.
The table uses 0-based race indices (= INI section number − 1). Note `CoefVie = 3,375` in
`[Race_105]`: `Val("3,375") = 3` (Val stops at the comma).

### 10.7 ClSurPerso (script API, `ClSurPerso.cls`)
A thin wrapper used by `.ia` scripts (`ClsJeuIA3`). It has read-only properties mirroring §2
(XP values exposed as the progression percentage `(Exp − ExpNiv)/(ExpNivPlus − ExpNiv)*100`,
maxima as the `Definir_Max*` values, 1-based object and resource indices) and the actions
`Aller_A` (refused outside `[0, W*16 − Largeur) × [0, H*16 − Hauteur)`), `Arreter`,
`Rentrer_Maison`, `Rentrer_Chateau`, `MaisonBatir` (alive, no obstacle, affordable →
`Maisons(i).Creer`), `Donner_Ressource`, `Deplacement_Aleatoire`, `Comportement_Agressif`,
`Comportement_Berserk`, `Attaquer_*_Proche`, `Aller_Ressource`, `Ramasser_Tresor_Proche`,
`Fabriquer_Ressource`, `Fabriquer_Objet` and `Objet_SEquiper`. They delegate to the functions
above and to the AI module. The large commented-out blocks in the file are dead code.

---------------------------------------------------------------------------------------------

## 11. Rnd usage in this area (in execution order within each function)

All the draws below happen on the shared VB6 stream (§0.5). "Always" means the draw is an operand
of a non-short-circuit `And`/`Or` and is consumed regardless of the other operand.

| Location | Draw | Notes |
|---|---|---|
| `ClsJeuPerso:433` Init | `Int(Rnd*3)` → NumeroPas | every Init (new perso, reused invocation slot, CmdCreerPerso) |
| `ClsJeuPerso:4989` Changer_Race | `Rnd > 0.5` → female | always (`And`) |
| `ClsJeuNoms:74/76` | `Names(CLng(Rnd*L))` | when `Joueur <= 0` and `NomAleatoire`, and when generating a name |
| `ClsJeuCommentaires:87/89` | `Int(Rnd*n)` | **every** `Commentaires.Message(k >= 0)` call, even if the comment is then refused (the perso already has a comment, or cannot speak) |
| `FrmMoteur2D:1589` | `Rnd > 0.99` | per perso with a pending work message |
| `ClsJeuPerso:1923-1924` | 2× position | confrontation respawn |
| `ModCombat:98` | critical failure | every Perso_Attaquer_Perso |
| `ModCombat:117` | critical hit | only if Att > Def |
| `ModCombat:146/151` | comment (crit) | |
| `ModCombat:178` | comment 44 | |
| `ModCombat:183` | 3× (Ctrl, Shift, Alt) | IA chef with soldiers |
| `ModCombat:237` | effect application | per on-hit effect whose resistance is > 0.2 |
| `ModCombat:278` | `Rnd > 0.2` | always, for a living IA defender |
| `ModCombat:283` | `Rnd > 0.9` flee | always, inside the 278 block |
| `ModCombat:293` | comment 13 | |
| `ModCombat:337` | comment 14 | |
| `ModCombat:374/391` | `Rnd < 0.25` | per active item of the victim (always) |
| `ModCombat:377/394` | `Rnd > 0.5` | per taken item |
| `ModCombat:489` | damage display x-jitter | every building hit |
| `ModCombat:513` | `Rnd > 0.6` | per perso in the array when a castle is destroyed (always) |
| `ModCombat:516,539,569,601` | comments | |
| `ModCombat:543` | `Rnd < 0.9` | always, when a house is destroyed |
| `ModCombat:552` / `585` | 2× effect position | castle / house hit not destroyed |
| `ModCombat:561` | `Rnd > 0.92` | per eligible defender of the castle (always within the nested block) |
| `ModCombat:568` | `Rnd > 0.9` | per eligible defender (comment 11) |
| `ModCombat:579` | `Rnd > 0.9` | help call by the castle attacker |
| `ModCombat:590` | `Rnd > 0.75` | always, for a living IA non-invocation owner |
| `ModCombat:598` | `Rnd > 0.65` | always, for a house hit |
| `ModCombat:701` | `0.5 > Rnd` | per stocked object unit when looting a house |
| `ModCapacitesSpe:98` | self-effect | when the resistance is > 0.2 |
| `ModCapacitesSpe:368` | invocation count rounding | once per Creer_Invocation |
| `ModCapacitesSpe:390-391` | 2× position | confrontation join |
| `ModCapacitesSpe:466` | per candidate | Effets_Proches |
| `ModCapacitesSpe:191/202` | Init + Changer_Race draws | per created invocation (sex, name, NumeroPas, appearance) |
| `ModCapacitesSpe:346` | comment 33 | per non-projectile invocation |
| `ModDialogue:66/113` | `Rnd < 0.5` | per j scanned (always) |
| `ModDialogue:80` | `Rnd < 3/(NC+1)` | per eligible helper |
| `ModDialogue:216,227,238,256` | `Rnd < 0.1` | per soldier ordered (always) |
| `ModDialogue:516,539,567,595` | `Rnd < 0.1` | per selected soldier (always) |
| `ModGenererMonde:79` | monster choice | until success |
| `ModGenererMonde:94-95` | random playable race | |
| `ModMain:623,626` | AI file | per eligible NPC |
| `FrmParam:2340-2354` | race, sex, name, appearance, AI file | character generation |
| `ClsAffApparence:196` | `Int(Rnd*n)` per appearance category | appearance draws (also for invocations) |
| `ModIA:*` | AI | see the AI spec (berserk, random walks) |

---------------------------------------------------------------------------------------------

## 12. Known quirks to preserve (summary)

1. An area attack always resets the attacker's attack to 0 (§6.4). Its centre is the attacker,
   and castles are hit with `NoChateau = -1`.
2. A hit on a building costs `max(1, min(B.Vie, 20, Att))` attack points without clamping, so the
   bar can go negative (§6.7).
3. The attacker pays `D.Vie/0.5 + D.Defense − 1` attack points (pre-hit), capped by its bar (§6.1).
4. "Immune" resistances are 0.2 (§5.5).
5. Walking ignores every obstacle. The pathfinding is greedy, with the (0,0) start bug and
   banker's-rounded cell lookups (§3.6).
6. Effect ticking stops after the first expiry in a slow tick (§5.6).
7. `Vivant = False` is not idempotent (§8.1). `Creer_Invocation` may kill dead summons again
   (§6.5).
8. `Preparer_Service` ignores a raw material shortage for the action state (§3.8).
9. Non-kamikaze projectiles pierce until the chef's attack bar is 0 (§6.5).
10. `Not ProjectileEnCours` is bitwise (§6.5).
11. Resurrection through area-use objects cannot target ghosts (`Attaquable` requires alive, §6.5).
12. A defender that was hit has `Longueur_Pas = 0`, so the IA flee check almost never succeeds (§6.3).
13. The race attack resistance defaults to Physique (index 0); objects can only override it with
    an index > 0 (§5.3).
14. `RestrictionMasculin`/`RestrictionMasulin` keys are ignored, and the random sex draw ignores
    the male restriction (§10.2).
15. Attack costs (Vie, Energie, Magie, Moral) are only deducted when slot 0 holds an item, but the
    checks use the sum over all equipped items (§3.5).
