# 04 — AI, command layer, dialogue

Scope: `ModIA.bas`, `ModIA2.bas`, `ClsJeuIA2.cls`, `ClsJeuIA3.cls`, `ClSurPerso.cls` +
`ClsSurJeu.cls` (the VBScript object model), `ModCommandes.bas`, `ClsCommande.cls`,
`ClsComPerso.cls`, `ClsComFief.cls`, `ClsComMessage.cls`, `ClsComMessages.cls`,
`ModDialogue.bas`, `ClsJeuCommentaires.cls` (comment lines said by characters),
`FrmEditeurIA.frm` (only for the `.ia` format), and the data in `IAs/`, `IAScripts/`.

Line references are `File:line` in the VB source (1-based, CRLF stripped). Lines that are
commented out in the source are ignored unless mentioned. All VB6 semantics of
`03-persos-combat.md` §0 apply here and matter a lot for the AI:

- `Double` -> `Long`/`Integer` assignment uses **banker's rounding**; `Int()` floors.
- `And`/`Or` **never short-circuit**: every `Rnd` in a condition is consumed, and every
  function call in a condition runs (with its side effects, e.g. search cursors advance).
  Precedence `Not` > `And` > `Or`.
- `IIf` evaluates both branches.
- `For i = a To b` evaluates `b` once; assigning the loop variable (`i = UBound(...)`) is the
  code's way of breaking out; the loop still increments after that assignment.
- Booleans are −1/0; a Boolean stored into a `Long`/`Integer` becomes −1/0, and vice versa
  (`CBool(x) = x <> 0`).

Character state, actions (`Action` codes 0…600), movement, combat and the ClsJeuPerso order
methods (`Aller_A`, `Rentrer_Maison`, `Attaquer_Perso`, …) are in `03-persos-combat.md`
(§3.1 action codes, §3.2 order methods). Buildings, fiefs, prices and trading
(`Acheter`/`Vendre`/`Transfert_*`/`Service_*`/`Creer`/`Demolir`) are in `02-world.md`;
`Parametres.*` fields are in `01-data.md`. This file references them by name.

Index notation: `Persos(i)` and `Maisons(i)` share the index (house `i` belongs to perso
`i`), `Chateaux(f)`/`Fiefs(f)` share the fief index. `LPerso = UBound(Persos())`.

---------------------------------------------------------------------------------------------

## 1. Where the AI runs (scheduling)

### 1.1 Frame loop (`FrmMoteur2D.frm:1087-1117`, server/solo branch)

Each game frame, in this order:

1. `Lire_Commandes` (`FrmMoteur2D.frm:1006` server, `:1042` solo) — executes the command
   queue (§9).
2. `Comportement_IA` (`:1102`) — the round-robin AI (§1.2).
3. `Comportement_IA_Fiefs` (`:1103`) — fief economy tick (§8).
4. `Deplacer_Persos` (`:1104`) — for **every** perso: if `IA_Berserk > 0 And Vivant` then
   `IA_ComportementBerserk i` (`FrmMoteur2D.frm:1575-1579`), regardless of `.IA` (even the
   human player if he has a berserk bonus); then movement/work (03 §3.3).
5. Every 10th frame `Actions_Persos`, `Actions_Tresors`; then `Gestion_Utilisation_Speciale`,
   `Evenements_Lancer "Jouer"` (scenario events).

Clients in network games never run the AI.

### 1.2 Round robin (`ModIA.bas:152-174`, `:2847-2876`)

Module state: `IndicePerso` (the round-robin cursor, starts at 0), `NombreIAParTour`,
`NombreRechercheParTour`, both computed in `Property Let Nombre_IAs(Valeur)`
(`ModIA.bas:2847`):

```
LPerso = Valeur - 1
NombreIAParTour        = CLng(LPerso * 0.01)  (VitesseIA, :24);         if 0 then 1
NombreRechercheParTour = CLng(LPerso * 0.6)   (VitesseRechercheIA, :25); if 0 then 1
```

(banker's rounding). `Nombre_IAs` is set by `IA_Init` (`:114`, `UBound(Persos)+1`) and by
`Perso_Ajouter_Nouveau` (`ModCapacitesSpe.bas:385`) whenever a perso slot is appended. It
`ReDim Preserve`s all per-AI arrays; new slots are 0 / False, and
`AnglePatrouille(LPerso) = Rnd * 6.28` (`:2861`, one Rnd per call).

`Comportement_IA` (`:152`):

```
for j in 0..=NombreIAParTour:          // NombreIAParTour+1 persos per frame
    IndicePerso += 1
    if IndicePerso > LPerso: IndicePerso = 0
    Comportement_IA_Persos(IndicePerso, Simple=false)
```

`Comportement_IA_Persos` is also called:
- from `ClsJeuPerso.Arreter` (`ClsJeuPerso.cls:658`) with `Simple = True` whenever an `IA`
  perso that is `Visible` stops (so a stopping AI immediately decides again; `Simple` only
  suppresses house placement, §3.1);
- from command 78 `ConstPersoDefinirIAAttitude` (`ModCommandes.bas:163`) with `Simple=False`.

### 1.3 Initialisation (`IA_Init`, `ModIA.bas:99-138`; `IA_InitPerso`, `:140-150`)

Called at game start (`FrmMoteur2D.frm:403`) and after loading a scenario
(`ModScenario.bas:350`). Order of operations (and Rnd consumption):

1. Create the MSScriptControl (`Language="VBScript"`, `AllowUI=False`), `SurPerso1`,
   `SurPerso2` (`ClSurPerso`), `SurJeu` (`ClsSurJeu`).
2. `ReDim DistanceObstacle(UBound(Fiefs))`.
3. `Nombre_IAs = UBound(Persos)+1` → **Rnd #1** for `AnglePatrouille(LPerso)`.
4. `IndiceObjet = CLng(Rnd * Persos(0).Nombre_Objets_Inventaire)` (**Rnd #2**); if it equals
   `Nombre_Objets_Inventaire` it becomes 0. `IndiceObjet` is a **single global cursor**
   shared by all persos (§6).
5. `for i in 0..LPerso-1` (note: **not** `LPerso`): `IA_InitPerso i`, then
   `AnglePatrouille(i) = Rnd * 6.28` (one Rnd each). The last perso therefore keeps
   `DistanceMaison = 0`, `BatimentAConstruire = 0`, `RessourceAFabriquer = 0`,
   `ObjetAFabriquer = 0` (array defaults), which is a quirk to reproduce.
6. Compute `BatimentNiveauMax = max(Batiment_Niveau(t))` and
   `NombreBatimentsDeNiveau(level)` (the latter is never used afterwards).

`IA_InitPerso i`: `DistanceMaison(i)=96` (`DistancePalierRechercheMaison`),
`BatimentAConstruire(i)=-1`, `RessourceAFabriquer(i)=-1`, `ObjetAFabriquer(i)=-1`,
`QuantiteAFabriquer(i)=0`; if `ChoisirIAScript`, load the script text from
`<App>\IAScripts\<FichierIAScript>` (`ClsJeuIA3.SetScriptFile`, `ClsJeuIA3.cls:38-49`: every
line + CRLF). It is also called for invocations created with an IA file
(`ModCapacitesSpe.bas:213`). Invocations created **without** an IA file never get it: their
slot keeps whatever values the previous occupant had (or 0 for a fresh slot), which is
harmless because they never run `Comportement_IA2`.

Per-AI arrays (`ModIA.bas:56-78`, all indexed by perso):
`DistanceRessource`, `DistanceTresor`, `DistanceMaison`, `DistanceAdversaire`,
`DistanceMaisonEnnemi`, `DistanceChateauEnnemi` (search radii, Long),
`IndiceMaison`, `IndiceAdversaire`, `IndiceMaisonEnnemi`, `IndiceChateauEnnemi`
(incremental search cursors, Long), `AnglePatrouille` (Single),
`BatimentAConstruire`, `RessourceAFabriquer`, `ObjetAFabriquer`, `QuantiteAFabriquer`
(Integer goals of the scripted AI), `QuitterBatiment` (Boolean). Per-fief:
`DistanceObstacle(f)` (Long). Note `DistanceAdversaire`, `DistanceMaisonEnnemi`,
`DistanceChateauEnnemi`, `DistanceTresor`, `DistanceRessource` start at 0.

These arrays are not saved in world saves; after loading, they are re-initialised by
`IA_Init`.

### 1.4 How a perso gets an AI

- `Persos(i).IA` (Boolean): the AI is active. Set `Not Projectile` for every perso at game
  start (`ModMain.bas:619`) and for invocations (`ModCapacitesSpe.bas:205`). Toggled by
  command 6 (§9). The human player's own perso has `IA = True` only when he switches
  to automatic mode (ModeAuto key) and has chosen an IA.
- `ChoisirIA` + `FichierIA` + `NumeroIA`: the scripted `.ia` AI (§3–4).
  - Race default: `Race_IA_Fichier` (`IA_Fichier=` in `Parametres_Races.ini`) applied by
    `Changer_Race_Personnage` (`ClsJeuPerso.cls:5103-5109`).
  - Invocation default: `Objet.InvocationIA_Fichier` passed to `Creer_Invocation`
    (`ModCapacitesSpe.bas:206-214`, which also sets `NumeroIA = Charger_IA(FichierIA)`).
  - Random assignment at game start (`ModMain.bas:617-631`): for each perso with
    `Joueur = 0`, `Not ChoisirIA` and `Race_IA_Selectionner(Race)`: `If NiveauIA > Rnd`
    (`NiveauIA = 1`, so always true but **one Rnd consumed**) then `ChoisirIA = True`,
    `FichierIA = FileIA.List(Int(Rnd * FileIA.ListCount))` (second Rnd). `FileIA` is the
    FileListBox of the root `IAs\` folder only (alphabetical, the 56 root files of §4.3).
  - Battlefield mode: every perso gets `FichierIA = "Bataille.ia"` (`ModMain.bas:550-551`).
  - Scenario keys `fichier_ia` / `fichier_iascript` (`ModScenario.bas:199-202`).
  - Player choice in FrmParam (`OptionIAScript`, `FrmParam.frm:3451-3457`):
    option 1 = `.ia` file, option 2 = script.
  - `NumeroIA` is resolved by `Charger_IA(FichierIA)` (`ModIA2.bas:133-153`,
    case-insensitive match on the bare file name in the list of all loaded IAs, −1 if not
    found). `Perso_Finalier` (`ModSavPerso.bas:456-461`) does it for every perso at load
    and **disables** `ChoisirIA` if the file is not found.
- `ChoisirIAScript` + `FichierIAScript`: VBScript AI (§5). Takes precedence over everything.
- Neither: the built-in "monster" AI (§2.3) or "soldier" AI (§2.4).

---------------------------------------------------------------------------------------------

## 2. Top-level decision tree (`Comportement_IA_Persos`, `ModIA.bas:176-287`)

```
Comportement_IA_Persos(i, Simple):
  if not Persos(i).IA: return
  if ChoisirIAScript:
      run the VBScript "Jouer" with Me = Persos(i), Cible = Persos(Persos(i).IndicePerso),
      Jeu = SurJeu                                             (§5; alive or dead!)
      return
  if Vivant:
      IA_Equiper(i)                                            (§6.1)
      if IA_Berserk > 0:        nothing (berserk is driven by Deplacer_Persos, §1.1)
      elif ChoisirIA:           Comportement_IA2(i, Simple)    (§3)
      elif Invocation:          (NumeroChef >= 0)
          IA_Recuperation_CaracSimple(i)                        (§2.6)
          if Inactif:
              if IA_Ordre > 0: obey the order                   (§2.5)
              else:            IA_Comportement_Soldats(i)       (§2.4)
      else:
          IA_Recuperer_Equipement_Monstre(i)                    (§2.3)
          IA_Comportement_Monstre(i)                            (§2.3)
  else (ghost):
      IA_Mort_Equiper(i)                                        (§6.2)
      if Invocation:
          if Maisons(i).Visible and Persos(NumeroChef).Vivant and not DansSaMaison:
              IA_Rentrer_Maison(i)        // invocations with a house resurrect there
      else:
          h = IndiceMaison
          if (DansUneMaison or DansSaMaison) and Maisons(h).Construit
             and Maisons(h).Service_Utilisable(p):
              Maisons(h).Service_Utiliser(p); QuitterBatiment(i) = True   // temple resurrection
          elif Action <> 1 and Action <> 201:
              found = Trouver_Maison_Proche(p, i, 0, 0, TypeService=1, X)  // always evaluated
              if found and not IA_Argent_Besoin(i):
                  if X >= 0: IA_Aller_Maison(i, X)
              elif Chateaux(NumeroFief).Visible and not DansSonChateau:
                  IA_Rentrer_Chateau(i, False)
```

`Service 1` is the temple/resurrection service. (Ghost movement and castle resurrection are
in 03 §8.2.)

### 2.1 Money thresholds (`ModIA.bas:47-48, 424-429`)

```
IA_Argent_Besoin(i) = Argent < 300 * (Fiefs(NumeroFief).Epoque + 1)   // "poor"
IA_Argent_Riche(i)  = Argent > 500 * (Fiefs(NumeroFief).Epoque + 1)   // "rich" (only used by the unused IA_Depenser_Argent)
```

`Epoque` is 0-based (age 1 → 0).

### 2.2 Shared helpers

| helper | lines | behaviour |
|---|---|---|
| `IA_Aller_Tresor(i, k)` | 361 | `Aller_Tresor(Tresors(k), k, …)`; `QuitterBatiment=True` |
| `IA_Rentrer_Maison(i)` | 365 | `Rentrer_Maison(Maisons(i))`; `QuitterBatiment=False` |
| `IA_Aller_Maison(i, k)` | 369 | if `k = i`: `IA_Rentrer_Maison`; else `Aller_Maison(Maisons(k), Reparer := Rnd < 0.1)` (**1 Rnd**), `QuitterBatiment=False` |
| `IA_Rentrer_Chateau(i, ChercherRessources)` | 377 | see below |
| `IA_Sortir_Batiment(i)` | 406 | `QuitterBatiment(i)=True` only (no movement) |
| `IA_Deplacement_Aleatoire(i)` | 431 | see below |
| `IA_Patrouiller_Cercle(i, P, R)` | 915 | see below |

`IA_Rentrer_Chateau(i, ChercherRessources)`:
```
if ChercherRessources:
    Distance = Monde.Diagonale; l = 0
    for k in Fiefs(nf).Marches (collection order):
        d = CLng(dist(p, Maisons(k)))          // top-left corners, Euclidean
        if d < Distance: Distance = d; l = k
    if Distance < dist(p, Chateaux(nf)) or not Chateaux(nf).Visible:
        p.Aller_Maison(Maisons(l), Rnd < 0.1)   // NOTE: l = 0 if there is no market
    else:
        p.Rentrer_Chateau(Chateaux(nf))
else:
    p.Rentrer_Chateau(Chateaux(nf))
QuitterBatiment(i) = False
```
The `Rnd < 0.1` is only consumed in the market branch.

`IA_Deplacement_Aleatoire(i)` (`:431-478`), `TL = AffTerrain.Largeur`, `TH = AffTerrain.Hauteur`
(tile size in pixels), `nx = Monde.Largeur*TL - p.Largeur`, `ny = Monde.Hauteur*TH - p.Hauteur`:
```
if p.Volant:
    X = CLng(PositionX + (Rnd*128 - 64) * TL); Y = CLng(PositionY + (Rnd*96 - 48) * TH)
    clamp X to [0, nx], Y to [0, ny]
else:
    H = p.Hauteur - TH; n = 0
    repeat:
        X, Y as above (2 Rnd), clamped; n += 1
    until p.Volant or Monde.CoefVitesseSimple(X, Y + H) >= 1 or n > 40
p.Aller_A(X, Y)
```
(up to 41 tries, 2 Rnd per try).

`IA_Patrouiller_Cercle(i, P, R)` (`:915-925`):
```
p.Aller_A(CLng(Cos(A)*R - p.Largeur/2 + P.PositionX + P.Largeur/2),
          CLng(Sin(A)*R - p.Hauteur/2 + P.PositionY + P.Hauteur/2))
A += 6.28 * R / p.BonusDeplacement * 2      // Single
if A > 6.28: A -= 6.28
```
with `A = AnglePatrouille(i)`. (R is always 128.)

### 2.3 Monsters (non-scripted, not invocations)

`IA_Recuperer_Equipement_Monstre(i)` (`:479-512`): if `DansSonChateau And Not
QuitterBatiment(i)`: `IA_Restaurer_Chateau(i)` (§7.6) and `QuitterBatiment(i) = True`.
(Everything else in that function is commented out.)

`IA_Comportement_Monstre(i)` (`:299-310`):
```
IA_Poursuivre_Fuyard(i)
if Inactif:
    IA_Recuperation_CaracSimple(i)
    if Inactif: IA_Agressif(i)
elif Rnd > 0.99:          // 1 Rnd whenever not idle
    p.Arreter()           // (which re-enters Comportement_IA_Persos with Simple=True)
```

`IA_Poursuivre_Fuyard(i)` (`:1250-1260`): if `(Action = 301 Or Action = 302) And
Persos(IndicePerso).Attaquable And NumeroEquipe <> Persos(IndicePerso).NumeroEquipe` then
`Attaquer_Perso(IndicePerso)` and say comment 16 ("Comme on se retrouve.") — i.e. a perso
attacking a building switches to the (stale) perso target stored in `IndicePerso` if that one
is an attackable enemy.

`IA_Agressif(i)` (`:514-547`), **one Rnd** `r` (Select Case evaluates it once):
```
if IA_Pacifique:
    r < 0.1:  if Invocation: Aller_A(chef.DirectionX + ChefDecalageX, chef.DirectionY + ChefDecalageY)
              else IA_Rentrer_Chateau(i, False)
    r < 0.2:  IA_Deplacement_Aleatoire(i)
    else:     nothing
else:
    r < 0.05: if IA_Attaquer_Chateau(i, X): Attaquer_Chateau(X)      (§2.7)
    r < 0.1:  (same as pacific r<0.1: follow chef / go home castle)
    r < 0.2:  IA_Deplacement_Aleatoire(i)
    r < 0.4:  if IA_Attaquer_Maison(i, X): Attaquer_Maison(X)
    else:     if IA_Attaquer_Perso(i, X): Attaquer_Perso(X)
```

### 2.4 Soldiers (invocations without `.ia` file, `IA_Ordre = 0`) — `IA_Comportement_Soldats` (`:549-618`)

Only reached when the soldier is `Inactif` (checked by the caller). By `Attitude`:

| Attitude | name (UI) | behaviour |
|---|---|---|
| 0 | cover / follow | if not EnCombat: `Aller_A(chef.DirectionX + ChefDecalageX, chef.DirectionY + ChefDecalageY)` |
| 1 | wait | if not EnCombat: `Aller_A(ChefPositionX, ChefPositionY)` |
| 2 | defend | `j = Trouver_Ennemi(i, ChefPositionX, ChefPositionY)`; if `j >= 0` and `Sqr((ChefPositionX - P_j.X)^2 + (ChefPositionY - P_j.Y)^2) <= 384`: `Attaquer_Perso(j)`, else `Aller_A(ChefPositionX, ChefPositionY)` |
| 3 | attack | if not EnCombat: `j = Trouver_Ennemi(i)`; if `j >= 0` and `Distance_Persos(p, P_j) <= 640`: `Attaquer_Perso(j)` |
| 4 | berserk | `IA_ComportementBerserk(i)` (with `IA_Berserk = 0` → case 0, seek & destroy) |

`ChefPositionX/Y` is the guard post, set by orders (§10).

### 2.5 Obeying an order (`IA_Ordre`, `ModIA.bas:204-237`)

Only for invocations without `.ia` file, when `Inactif`. `IA_Ordre`/`IA_Cible` are set by
`Ordonner_Chargez` and the `Ordonner_*` functions (§10).

| IA_Ordre | condition (only if not EnCombat) | action, else `IA_Ordre = 0` |
|---|---|---|
| 1 perso | `Persos(IA_Cible).Attaquable And Persos(IA_Cible).NumeroEquipe <> NumeroEquipe` | `Attaquer_Perso(IA_Cible)` |
| 2 house | `Maisons(IA_Cible).Visible And Persos(IA_Cible).NumeroEquipe <> NumeroEquipe` | `Attaquer_Maison(IA_Cible)` |
| 3 castle | `Chateaux(IA_Cible).Visible` (no team check) | `Attaquer_Chateau(IA_Cible)` |
| 4 guard | — | `IA_Garder_Perso(i)` |

`IA_Garder_Perso(i)` (`:620-673`), only if not EnCombat:
```
if Persos(IA_Cible).Vivant:
    if IA_Attaquer_Assaillant_Perso(i, IA_Cible, X): Attaquer_Perso(X)
    else: IA_Patrouiller_Cercle(i, Persos(IA_Cible), 128)
else: IA_Ordre = 0
```

### 2.6 Recovering bars

`IA_Recuperation_CaracSimple(i)` (`:1381-1418`), all inside `If IA_Manger`:
```
Recuperer_AutoVie/Energie/Magie/Moral (eat from own stock, 03 §4)
if Inactif:
    if Definir_Vie < Definir_MaxVie:            IA_Chercher_Ressource(i, 0 /*herbe*/); return
    if Definir_Energie < Definir_MaxEnergie*0.5: IA_Chercher_Ressource(i, 1 /*fruit*/); return
if PerteRessourceIndice >= 0 and ressources[PerteRessourceIndice] < PerteRessourceQuantite:
    IA_Chercher_Ressource(i, PerteRessourceIndice)   // ammunition
```

`IA_Recuperation_Carac(i)` (`:1262-1379`) — used by the scripted AI:
```
if IA_Manger:
    Recuperer_AutoVie/Energie/Magie/Moral
    if DansUneMaison:
        m = Maisons(IndiceMaison)
        if m.Magasin and Ressources_Mangeable(Batiment_RessourceFabrique(m.TypeBatiment)):
            m.Acheter(5, p); return               // buys 5 units of the shop's food
    else:
        if Inactif:
            if Vie < MaxVie:
                if IA_Argent_Besoin: IA_Chercher_Ressource(i, 0); return
                elif Trouver_Maison_Proche(p, i, 0, TypeBesoin=1, 0, X):
                    if X >= 0: IA_Aller_Maison(i, X) else IA_Chercher_Ressource(i, 0)   // (X>=0 always when found)
                    return
            if Energie < MaxEnergie*0.5:
                if IA_Argent_Besoin: IA_Chercher_Ressource(i, 1); return
                elif Trouver_Maison_Proche(p, i, 0, 2, 0, X): if X >= 0: IA_Aller_Maison(i,X); return
                else: IA_Chercher_Ressource(i, 1); return
            if Magie < MaxMagie*0.5 and not IA_Argent_Besoin:
                if Trouver_Maison_Proche(p, i, 0, 3, 0, X) and X >= 0: IA_Aller_Maison(i,X); return
            if Moral < MaxMoral*0.5 and not IA_Argent_Besoin:
                if Trouver_Maison_Proche(p, i, 0, 4, 0, X) and X >= 0: IA_Aller_Maison(i,X); return
        if PerteRessourceIndice >= 0 and res[PRI] < PerteRessourceQuantite:
            if IA_Argent_Besoin: IA_Chercher_Ressource(i, PRI); return
            elif Trouver_Maison_Proche(p, i, PRI, 0, 0, X): if X>=0: IA_Aller_Maison(i,X); return
            else: IA_Chercher_Ressource(i, PRI); return
if Inactif and IA_Equiper > 0:
    if Trouver_Tresor_Proche(p, i, X) and X > 0: IA_Aller_Tresor(i, X); return
```
Note that in the Vie branch, if the house search fails (and the perso is not poor), the code
falls through to the Energie test (no `Exit Sub`). Note `X > 0` for treasures (treasure
indices are 1-based; the test is correct).

### 2.7 Incremental searches (one "slice" per call)

These functions examine `NombreRechercheParTour + 1` candidates per call, continuing from a
per-AI cursor, with a radius that grows each time the cursor wraps. Distances are from
top-left corner to top-left corner.

`IA_Attaquer_Perso(n, ByRef k)` (`:940-964`): `k = -1`; repeat `N+1` times:
```
if IndiceAdversaire(n) > UBound(Persos):
    IndiceAdversaire(n) = 0
    DistanceAdversaire(n) += 48                  // DistancePalierRecherchePerso
    if DistanceAdversaire(n) > 640: DistanceAdversaire(n) = 64
c = IndiceAdversaire(n)
if DistanceAdversaire(n) > Sqr(dx²+dy²) and Persos(c).NumeroEquipe <> my team and Persos(c).Attaquable:
    return True, k = c                           // cursor NOT advanced
IndiceAdversaire(n) += 1
```
`IA_Attaquer_Maison(n, k)` (`:1018-1046`): same with `IndiceMaisonEnnemi`,
`DistanceMaisonEnnemi` (+96 per wrap, reset to 64 when > 640), condition
`dist < radius and Persos(c).NumeroEquipe <> team and Maisons(c).Visible` (no `Construit`
check). Cursor not advanced on success.
`IA_Attaquer_Chateau(n, k)` (`:1048-1074`): cursor `IndiceChateauEnnemi` over
`Chateaux()`, radius `+= Parametres.Peuples_ChateauLargeur(0) * 2` per wrap (reset to 64 when
> 640), condition `dist < radius and Fiefs(c).NumeroEquipe <> team and Chateaux(c).Visible`;
on success it sets `i = N` (ends the loop) and **does** advance the cursor.
`IA_Attaquer_Assaillant_Perso(n, cible, k)` (`:966-988`): shares `IndiceAdversaire(n)`
(wraps to 0 without changing the radius); success when the candidate `c` satisfies
`(c.Action = 300 And c.IndicePerso = cible) Or (c.Action = 301 And c.IndiceMaison = cible)
Or (c.Action = 302 And Persos(cible).DansSonChateau And c.IndiceChateau = Persos(cible).NumeroFief)`
(no team check); cursor not advanced on success.

`Trouver_Maison_Proche(P, n, TypeRessource, TypeBesoin, TypeService, ByRef k)`
(`:1948-2057`), returns Boolean; `k = -1`:
```
repeat N+1 times (for i in 0..=N):
    if IndiceMaison(n) > UBound(Maisons):
        DistanceMaison(n) += 96; IndiceMaison(n) = 0     // radius is never capped
    m = Maisons(c) with c = IndiceMaison(n)
    sameFief = Persos(c).NumeroFief = Persos(n).NumeroFief
    box = |P.X - m.X| < DistanceMaison(n) and |P.Y - m.Y| < DistanceMaison(n)
    if TypeService > 0:
        result = (m.Service = TypeService and m.Construit and m.Definir_Stock >= 1 and sameFief and box)
    elif TypeBesoin = 5:
        result = (m.Fabrique and m.Construit and m.Definir_Stock >= 1 and sameFief and box)
    elif m.Magasin or m.Fabrique:
        if TypeRessource = 0:
            pbm = Batiment_RessourceFabrique(m.TypeBatiment)
            if pbm >= 0 and TypeBesoin < 5 and m.Construit and m.Definir_Stock >= 1 and sameFief and box:
                TypeBesoin 1: result |= Ressources_GainVie(pbm) > 0
                           2: GainEnergie, 3: GainMagie, 4: GainMoral   (0: nothing)
        elif Batiment_RessourceFabrique(m.TypeBatiment) = TypeRessource and m.Construit
             and m.Definir_Stock >= 1 and sameFief and box
             and DistanceMaison(n) >= Sqr(dx²+dy²):
            result = True
    if result:
        k = c; DistanceMaison(n) = 96; i = UBound(Maisons)   // break (see note)
    IndiceMaison(n) += 1                                     // also after a hit
```
Notes: in the service and `TypeBesoin = 5` branches the result is **assigned**, in the other
branches only set to True; the "break" works because `UBound(Maisons) = LPerso >= N` in
practice. `TypeRessource = 0` means "any" (so resource index 0, herbe, can never be searched
specifically). `k` stays −1 only when nothing was found.

`Trouver_Tresor_Proche(P, n, k)` (`:1907-1926`):
```
DistanceTresor(n) = CLng(DistanceTresor(n) + AffTresor.Largeur * Rnd * 10)    // 1 Rnd
if DistanceTresor(n) > 724: DistanceTresor(n) = 0; return False
for t in 1..=Tresors.Count:
    if DistanceTresor(n) >= Sqr((P.X - T.X)² + (P.Y - T.Y)²):
        k = t; DistanceTresor(n) = 0; return True (break)
```

Full scans (no cursor):
- `Trouver_Ennemi(i, X=-1, Y=-1)` (`:1093-1118`): origin `(X,Y)` or the perso position;
  nearest `j` (strict `<`, ties → lowest index) with `NumeroEquipe <> team And Attaquable`,
  distance `CLng(Distance_Positions)`, starting bound `Monde.Diagonale`. −1 if none.
- `Trouver_MaisonEnnemi(i)` (`:1194-1211`): nearest visible house whose owner is in another
  team; bound `Monde.Largeur*TL + Monde.Hauteur*TH`; distances stored as `CLng`.
- `Trouver_ChateauEnnemi(i)` (`:1231-1248`): nearest visible castle with
  `Fiefs(j).NumeroEquipe <> team`, same bound.
- `Trouver_Allie(i, Vivant=True, Mort=False)` (`:1144-1174`): nearest same-team non-invocation
  `j` with `(Not Vivant Or P_j.Vivant) And Not (Mort And P_j.Vivant)` (includes `i` itself).
- `IA_Trouver_Ennemi/MaisonEnnemi/ChateauEnnemi(i, EnvoyerSoldats, Ctrl, Shift, Alt)`
  (`:1075, 1176, 1213`): find with the matching scan; if found: `EnvoyerSoldats` →
  `Ordonner_Chargez(i, 0|1|2, target, Ctrl, Shift, Alt)`, else `Persos(i).Attaquer_*(target)`.
- `IA_Trouver_Tresor(i)` (`:1928-1946`): nearest treasure (strict `<`, 1-based, default
  index 0 is never used because there is at least one treasure); `Aller_Tresor`.
- `IA_Trouver_Allie(i, Vivant, Mort)` (`:1121-1142`): `k = Trouver_Allie`; if `k >= 0`: if
  `|dx| < Largeur And |dy| < Hauteur` then `EnDeplacement = False`, else `Aller_A(P_k.X,
  P_k.Y)` and `EnDeplacement = True`; `IndicePerso = k`; returns True.
- `IA_Chercher_Ressource(i, r)` (`:1506-1512`) = `IA_Trouver_Ressource(i,
  Ressources_TypeTerrainRessource(r))`.
- `IA_Trouver_Ressource(i, T)` (`:1514-1537`): nearest resource node with `Apparence = T`
  (Euclidean, bound `Monde.Largeur*TL + Monde.Hauteur*TH`, stored as Long); then
  `Aller_Ressource(Ressources(k), k, …)` — **if none matches, k = 0**, so the perso walks to
  resource node 0.
- `IA_Lancer_projectile(n)` (`:927-938`): `k = Trouver_Ennemi(n)`; if `k > -1` and
  `Distance_Persos <= Definir_PorteeAttaque * 2`: `Attaquer_Terrain(centre of k, 0)`.

### 2.8 Berserk (`IA_ComportementBerserk`, `ModIA.bas:717-913`)

Run every frame by `Deplacer_Persos` for alive persos with `IA_Berserk > 0` (the value
comes from the race `IA_Berserk` or an object `Objet_Berserk` bonus,
`ClsJeuPerso.cls:3075, 3251`), and by soldiers in attitude 4 (`IA_Berserk = 0` → case 0).
The whole body only runs if `Not EnCombat Or Longueur_Pas = 0`.

| IA_Berserk | behaviour |
|---|---|
| 0, 1 seek & destroy | `IA_Trouver_Ennemi`; if not EnCombat: `IA_Trouver_ChateauEnnemi`; if still not: `IA_Trouver_MaisonEnnemi`; then if not EnCombat and `Kamikaze`: `Vivant = False`; if not `EnDeplacement`: `IA_Deplacement_Aleatoire` |
| 2 orbit chef | if not EnCombat: `j = Trouver_Ennemi(i)`; if `j >= 0 And Distance_Persos(chef, P_j) < 384`: `Attaquer_Perso(j)`, else `IA_Patrouiller_Cercle(i, chef, 128)` |
| 3 resurrector | if `IA_Trouver_Allie(i, Vivant=False, Mort=True)`: if not EnDeplacement: if `Race_AutoResurrection(Race)`: `Ressusciter_Perso(IndicePerso, Effet_Attaque)`; if Kamikaze: `Vivant = False`. Else `IA_Patrouiller_Cercle(i, chef, 128)` |
| 4 treasure picker | if `NumeroChef > -1` and chef alive and `Argent > 0`: `ArgentSelectionne = Argent`, `Ressources_Prendre_Tout`, `Objet_Prendre_Premier`, `Aller_Donner(NumeroChef)`. Then if not EnDeplacement: `IA_Trouver_Tresor`; if not EnDeplacement: `IA_Deplacement_Aleatoire` |
| 5 house guard | see below |
| 6 caravan | see below |

Case 5 (`:776-843`):
```
if Inactif and Vie < MaxVie and IA_Manger:
    Recuperer_AutoVie; if not EnCombat and Inactif: IA_Chercher_Ressource(i, 0)
if Inactif and res[PerteRessourceIndice] < PerteRessourceQuantite:      // res[-1] reads 0
    if IA_Argent_Besoin: IA_Chercher_Ressource(i, PRI)
    elif Trouver_Maison_Proche(p, i, PRI, 0, 0, X): if X >= 0: IA_Aller_Maison(i, X)
    else: IA_Chercher_Ressource(i, PRI)
if Inactif:
    if Maisons(i).Visible:
        if not DansSaMaison: Rentrer_Maison(Maisons(i))
        else:
            if Objet_Equipes_Actif(0): if Objet_BonusPorteeAttaque(Objet_Equipes_Type(0)) = 0: IA_Equiper(i)
            else: IA_Equiper(i)
            if Attaque = Definir_MaxAttaque and ProjectileEnCours = 0:
                IA_Lancer_projectile(i)
                if Inactif and Maisons(i).Definir_Vie < Maisons(i).MaxVie: Rentrer_Maison(Maisons(i))
    else: (same chain as case 0/1 but the final random move only if not EnCombat)
```

Case 6, caravan between own house and markets/castle (`:844-909`):
```
if Inactif and res[PRI] < PerteRessourceQuantite:
    IA_Chercher_Ressource(i, PRI); IA_Chercher_Ressource(i, PRI)      // twice (same target)
if Inactif:
    if Vie < MaxVie:
        if IA_Manger: Recuperer_AutoVie; if not EnCombat and Inactif: IA_Chercher_Ressource(i, 0)
    elif not Maisons(i).Visible: Vivant = False                         // dies if its house is gone
    elif Maisons(i).Definir_Vie < Maisons(i).MaxVie: if Rnd < 0.01: Rentrer_Maison(Maisons(i))
    elif DansSonChateau or DansUneMaison:                              // arrived at a market/castle
        j = Int(Distance_Positions(Maisons(Numero), p) / 12)
        Argent += j; show gain j (AffDegat type 2)
        if Persos(NumeroChef).Vivant: chef.Argent += j; show gain on chef
        Rentrer_Maison(Maisons(i))
    elif DansSaMaison:
        if Rnd < 0.005:
            c = Chateaux(nf).Visible and (Rnd <= 1 / (Marches.Count + 1))   // 2nd Rnd always consumed
            if c: Rentrer_Chateau(Chateaux(nf))
            else:
                j = Int(Rnd * Marches.Count)
                if Marches(j+1) <> i: Aller_Maison(Maisons(Marches(j+1)), Rnd < 0.1)
    else: Rentrer_Maison(Maisons(i))
```
(If `Marches.Count = 0` and the castle is invisible, `Marches(1)` raises a runtime error in
VB; the port should treat it as "do nothing".)

---------------------------------------------------------------------------------------------
## 3. The scripted (`.ia`) AI — `Comportement_IA2` (`ModIA2.bas:155-448`)

This is the AI of every villager, apprentice, emperor and most "monsters" with an
`IA_Fichier`. It is a goal machine: four persistent goals per perso
(`BatimentAConstruire`, `RessourceAFabriquer`, `ObjetAFabriquer`, `QuantiteAFabriquer`,
written `BAC`, `RAF`, `OAF`, `QAF` below), plus the ordered step list of the current age
(§4). Written `h = Maisons(i)`, `nf = NumeroFief`, `RF(t) = Batiment_RessourceFabrique(t)`
(resource produced by building type t, −1 if none), `MP(t) = Batiment_TypeMatierePremiere(t)`
(raw material, −1 if none), `res[r] = Definir_ressources(r)` (returns 0 for r < 0).

### 3.1 Main body

```
Comportement_IA2(i, Simple):
  IA_Poursuivre_Fuyard(i)                                          // §2.3
  if Action not in {3, 500}: IA_Recuperation_Carac(i)              // §2.6
  if not EnCombat and Action not in {2, 3, 102, 205, 500, 600}:    // not busy
    IA_Vente_Surplus_Objets(i)                                     // §7.7
    if h.Visible and h.Vie < h.MaxVie:
        IA_Rentrer_Maison(i)                                       // finish/repair the house
    elif BAC >= 0 and (h.TypeBatiment <> BAC or not h.Visible):    // (re)build a house
        if PeutConstruire_Maison(BAC):
            if IA_Changer_Maison(i):                               // §3.4, side effects
                if not Simple and Trouver_Emplacement_Maison(i, X, Y, BAC):   // §3.5
                    h.Creer(X, Y, BAC, i)
        else:
            for r in 0..NombreRessources-1:
                if res[r] < Batiment_PrixRessource(BAC, r):
                    IA2_ChercherMatieresPremiere(i, r, Batiment_PrixRessource(BAC, r)); break
    elif h.Visible and RF(h.type) >= 0 and res[RF] < Definir_MaxRessources(RF)
         and MP(h.type) <> -1 and Ressources_Recoltable(MP(h.type)):
        if res[MP] <= 1: IA_Chercher_Ressource(i, MP)               // harvest raw material
        else:            IA_Rentrer_Maison(i)                       // go home and transform it
    elif RAF >= 0:                                                  // §3.2
        ...
    elif OAF >= 0:                                                  // §3.3
        ...
    else:
        run the step list (§3.6)
```

### 3.2 Goal "make resource" (`RAF >= 0`, `:228-251`)

```
if res[RAF] < QAF:
    if Ressources_Recoltable(RAF): IA_Chercher_Ressource(i, RAF)
    elif h.Visible and RF(h.type) = RAF:
        if not DansSaMaison: IA_Rentrer_Maison(i)
        elif MP(h.type) >= 0 and res[MP] < 1:
            IA2_ChercherMatieresPremiere(i, MP(h.type), Parametres.PersosMaxRessources)
    elif Action <> 202 and Action <> 203:
        IA2_ChercherMatieresPremiere(i, RAF, QAF)
else:
    RAF = -1
```

### 3.3 Goal "get object" (`OAF >= 0`, `:252-286`)

```
if Action <> 500 and Objet_NombreType(OAF) < QAF:
    if h.Visible:
        if not DansSaMaison:
            IA2_ChercherObjet(i, OAF, QAF)
            if Inactif: IA_Rentrer_Maison(i)
        elif h.Fabrique:
            for j in fabricated objects of h.type:
                if Batiment_ObjetFabrique(h.type, j) = OAF:
                    if h.Stock_Objets(j) = 0:
                        if PeutConstruire_Objet(j, h): Creer_Objet(j, h)      // start crafting (Action 500)
                        else: IA2_ChercherObjet(i, OAF, QAF)
                    else:
                        Objet_Ramasser_Maison(j, h); OAF = -1               // take it from stock
                    break
        elif IA_Changer_Maison(i):
            h.Demolir(p); Comportement_IA2(i)   // recursive, Simple=False
            BAC = -1
    else:
        IA2_ChercherObjet(i, OAF, QAF)
```

### 3.4 Sub-procedures

`IA2_ChercherMatieresPremiere(i, r, q)` (`ModIA2.bas:450-521`) — obtain `q` units of resource `r`:
```
if Ressources_Recoltable(r):
    IA_Chercher_Ressource(i, r); RAF = r; QAF = CInt(q)
elif Argent > q * Ressources_PrixVente(r):
    if Action not in {201, 202} and not (DansUneMaison or DansSonChateau or DansUnChateau):
        if Trouver_Maison_Proche(p, i, r, 0, 0, X):            // a shop selling r
            if X >= 0: IA_Aller_Maison(i, X)
            RAF = r; QAF = q
        elif Fiefs(nf).res[MP(h.type)] > q and (Chateaux(nf).Visible or Marches.Count > 0):
            IA_Rentrer_Chateau(i, True); RAF = r; QAF = q        // NOTE: tests the fief stock of
                                                                 // the perso's *house* raw material, not r
        else: IA2_Fabriquer_Ressource(i, r, q)
    else:   // inside a building
        if DansUneMaison and Maisons(IndiceMaison).Magasin:
            if that shop's Definir_Stock >= 1:
                Maisons(IndiceMaison).Acheter(q - res[r], p); RAF = -1
            else:
                DansUneMaison = False; IA2_ChercherMatieresPremiere(i, r, q)   // recurse
        elif (DansUnChateau or DansSonChateau or (DansUneMaison and that house is a Marche))
             and Fiefs(nf).res[r] >= q:
            Fiefs(nf).Acheter(r, q - res[r], p); RAF = -1
        else: IA2_Fabriquer_Ressource(i, r, q)
else:
    IA2_Fabriquer_Ressource(i, r, q)
```

`IA2_Fabriquer_Ressource(i, r, q)` (`:523-544`) — walk down the transformation chain:
```
while Ressources_TypeMatierePremiere(r) >= 0 and res[Ressources_TypeMatierePremiere(r)] < q:
    r = Ressources_TypeMatierePremiere(r)
if Ressources_Recoltable(r): IA_Chercher_Ressource(i, r)
else: BAC = first building type t (lowest index) with RF(t) = r   (unchanged if none)
RAF = r; QAF = q
```

`IA2_ChercherObjet(i, o, q)` (`:546-634`), only if `Action <> 201`:
```
decided = False
if Argent >= Objet_PrixVente(o):
    for j in 0..UBound(Maisons):                         // first matching house wins
        if Maisons(j).Fabrique and Persos(j).NumeroEquipe = my team:
            for k in objects of Maisons(j).type:
                if Batiment_ObjetFabrique(type, k) = o and Maisons(j).Stock_Objets(k) >= q:
                    if i = j and DansSaMaison: Objet_Ramasser_Maison(k, Maisons(j)); OAF = -1
                    elif DansUneMaison and IndiceMaison = j: Objet_Acheter_Maison(k, Maisons(j)); OAF = -1
                    else: IA_Aller_Maison(i, j)
                    decided = True; break both loops
if not decided and h.Fabrique and h.Definir_Stock > 0:
    for j in 0..h.Definir_Nombre_Stock_Objet-1:
        if h.Stock_Objets(j) > 0 and Batiment_ObjetFabrique(h.type, j) = o:
            IA_Rentrer_Maison(i); decided = True      // (no break; may be called several times)
if not decided:
    OAF = o; QAF = q
    for r in resources: if res[r] < Objet_PrixRessources(o, r):
        IA2_ChercherMatieresPremiere(i, r, Objet_PrixRessources(o, r)); decided = True; break
if not decided:
    for t in building types with Batiment_Fabrique(t): if t fabricates o:
        BAC = t; OAF = o; QAF = q; break
```
Note the shop search covers every house of the same team (any fief), with no distance
limit and no `Visible`/`Construit` test, and requires the stock to hold `q` copies at once.

`IA2_Choisir_Maison(i, t)` (`:636-678`) — gather the material needed to *work* in building
type `t` before building it:
```
RAF = -1
if Batiment_Fabrique(t):
    for r in resources with Ressources_Epoque(r) <= Fiefs(nf).Epoque:
        for k in objects fabricated by t:
            if Objet_PrixRessources(Batiment_ObjetFabrique(t,k), r) > 0 and res[r] < PersosMaxRessources:
                IA2_Fabriquer_Ressource(i, r, PersosMaxRessources); break both
elif Batiment_Service(t) (non-zero):
    r = Service_MatierePremiere(Batiment_Service(t))
    if r >= 0 and res[r] < MaxRessources(r): IA2_Fabriquer_Ressource(i, r, MaxRessources(r))
elif RF(t) >= 0:
    r = Ressources_TypeMatierePremiere(RF(t))
    if r >= 0 and res[r] < MaxRessources(r): IA2_Fabriquer_Ressource(i, r, MaxRessources(r))
if RAF = -1: BAC = t
```
(`IA2_Fabriquer_Ressource` sets `RAF`, so `BAC = t` is only set once nothing is missing.
`IA2_Fabriquer_Ressource` may itself set `BAC` to the building producing a missing
transformed resource.)

`IA_Changer_Maison(i)` (`:680-708`) — may the perso abandon its current house?
```
if not h.Visible: return True
if h.Argent > 0 or (h.Definir_Stock > 0 and Not h.Service):   // NOTE: Not on an Integer is bitwise;
                                                              // the second term is just Stock > 0
    if DansSaMaison:
        h.Transfert_Argent_Prendre(0, p, Max=True)
        if h.Magasin: h.Transfert_Stock_Prendre(0, p, Max=True)
        elif h.Fabrique: for each object slot j: while h.Stock_Objets(j) >= 1 and not Inventaire_Plein:
                             Objet_Ramasser_Maison(j, h)
        return True
    else: IA_Rentrer_Maison(i); return False
return True
```

### 3.5 Choosing a building and a place

`Choisir_Batiment(n, f, Niveau = -1)` (`ModIA.bas:1816-1873`), returns a building type or −1.
`own = Maisons(n).TypeBatiment` (the current/last type, even if the house is not visible):
```
res = -1
if Niveau = -1:
    Niveau = 0
    for L = Int(Rnd * BatimentNiveauMax) + 1 down to Batiment_Niveau(own) + 1:   // 1 Rnd; may be empty
        if res = -1:
            res = Choisir_Batiment(n, f, L)
            if PeutConstruire_Maison(res): L = 0 (break) else res = -1
// "monopoly" rule — runs in every call, including the recursive ones:
if Nombre_Batiments(own, f) > 1:
    for t in 0..NombreTypeBatiments-1:
        if Nombre_Batiments(t, f) = 0 and Batiment_Niveau(t) < Batiment_Niveau(own):
            res = t; break
if res = -1:
    cnt = number of types with Batiment_Niveau = Niveau
    k = Int(Rnd * cnt)                                          // 1 Rnd
    res = the k-th (0-based) type with that level, in index order (−1 if cnt = 0)
elif Batiment_NoEpoque(res) - 1 > Fiefs(f).Epoque and PeutConstruire_Maison(res) and not Maisons(n).Visible:
    res = Choisir_Batiment(n, f, 0)
return res
```
`Nombre_Batiments(t, f)` (`:1801`) counts houses `j` with `TypeBatiment = t` and
`Persos(j).NumeroFief = f` — **including invisible (destroyed / never built) houses**,
because there is no `Visible` test.

`Trouver_Emplacement_Maison(n, ByRef X, ByRef Y, t)` (`:2059-2117`), `nf = Persos(n).NumeroFief`,
`W, H = Batiment_Largeur/Hauteur(t)`, `D = DistanceObstacle(nf)`:
```
X, Y = Maisons(n).PositionX/Y                              // try the old place first
if Obstacle(n, X, Y, W, H) = 0: D = Chateaux(nf).Largeur; return True
repeat 11 times (NombreRecherchesEmplacementMaison = 10, 0..=10):
    D += Monde.LargeurCase
    if Invocation:
        anchor = Maisons(chef) if Maisons(chef).Visible else Persos(chef)
        X = CLng(anchor.X + Rnd*(2D + anchor.Largeur) - D); Y = CLng(anchor.Y + Rnd*(2D + anchor.Hauteur) - D)
    elif (Chateaux(nf).Visible and Rnd <= 1/(Marches.Count+1)) or (not Chateaux(nf).Visible and Marches.Count = 0):
        // (the Rnd is consumed every time this test is evaluated)
        anchor = Chateaux(nf): same formulas
    else:
        j = Marches(Int(Rnd * Marches.Count) + 1); anchor = Maisons(j): same formulas
    if X < 0: X = CLng(Rnd * LargeurCase)
    elif X > Monde.Largeur*LargeurCase - W: X = CLng(Monde.Largeur*LargeurCase - W - Rnd*LargeurCase)
    if Y < 0: Y = CLng(Rnd * LargeurCase)          // NOTE: LargeurCase, not HauteurCase
    elif Y > Monde.Hauteur*HauteurCase - H: Y = CLng(Monde.Hauteur*HauteurCase - H - Rnd*HauteurCase)
    if Obstacle(n, X, Y, W, H) = 0: D = Chateaux(nf).Largeur; return True
return False                                              // D keeps growing for the next call
```
`DistanceObstacle` is per fief, starts at 0 and is reset to the castle width after each
success, so houses of a fief cluster around the castle and the markets in rings.

`Obstacle(n, X, Y, W, H, Memoriser)` (`ModIA.bas:2407-2578`) returns the first hit in this
order: 9 = attackable enemy perso (other than n); 13 = treasure; castle (visible): 6 = own
fief, 8 = allied fief, 11 = enemy; house (visible): own house → 5 if built and full life
else 4; allied → 7; enemy → 10; 1 = resource node; 16 = decor; 999 = not constructible
(`Monde.Constructible`); 0 = free. `Est_Superpose` (`:2780-2816`) is the rectangle overlap
test (with its odd `Largeur`-for-height terms in the containment clauses, to be copied
verbatim). These codes are also the UI cursor types (05-render-ui).

### 3.6 Running the step list (`ModIA2.bas:287-437`)

```
NumeroIA' = Charger_IA("Empereur.ia") if Definir_Empereur else NumeroIA
E = Fiefs(nf).Epoque
for j, step in steps(NumeroIA', E):       // stops at the first step that "acts"
    match step.Categorie:
    0 Ressource(v, q):
        if res[v] < q: IA2_ChercherMatieresPremiere(i, v, q); break
    1 Batiment(v):
        if h.TypeBatiment <> v and BAC <> v: IA2_Choisir_Maison(i, v); break
    2 Objet(v, q):
        if Objet_NombreType(v) < q: IA2_ChercherObjet(i, v, q); break
    3 Travailler:
        if h.Visible:
            if Chateaux(nf).Visible or Marches.Count > 0:
                if BatimentDefini(E):          // the age has a Batiment_ step
                    IA_ChercherMatieresPremiere(i); IA_Travail(i)
                else:                          // free career: try to upgrade
                    BAC = Choisir_Batiment(i, nf, Batiment_Niveau(h.type) + 1)
                    if BAC >= 0:
                        if Batiment_NoEpoque(BAC) - 1 > E:
                            BAC = h.type; IA_ChercherMatieresPremiere(i); IA_Travail(i)
                        // else: nothing now; next tick the BAC branch of §3.1 builds it
                    else: IA_ChercherMatieresPremiere(i); IA_Travail(i)
            else: IA_Agressif(i)
        else:
            BAC = Choisir_Batiment(i, nf)      // Niveau = -1
        break
    4 Patrouiller:
        IA_Agressif(i) ×3; break               // 3 independent Rnd draws
    5 Attaquer Ennemi:
        if Victoire(0): IA_Agressif(i)
        elif not EnCombat and Definir_Attaque = Definir_MaxAttaque:
            IA_Trouver_Ennemi(i, False…)       // nearest enemy, Attaquer_Perso
            if NombreSoldats > 0: Ordonner_Chargez(i)
            if not EnCombat: IA_Agressif(i)
        break
    6 Attaquer Donjon:
        if Victoire(0): IA_Agressif(i)
        elif not EnCombat and Attaque = MaxAttaque:
            IA_Trouver_ChateauEnnemi(i, False…); if NombreSoldats > 0: Ordonner_Chargez(i)
            if not EnCombat and Attaque = MaxAttaque:
                IA_Trouver_Ennemi(i, False…); if NombreSoldats > 0: Ordonner_Chargez(i)
                if not EnCombat: IA_Agressif(i)
        break
    7 ChangerMode(v):                          // does NOT break, except where noted
        0: IA_Pacifique = True        1: IA_Pacifique = False
        2: if IA_Donner(i, money, resources, objects): return
        3: if IA_Donner(i, money only): return
        4: if IA_Donner(i, resources only): return
        5: if IA_Donner(i, objects only): return
        6: if IA_Invoquer(i): return
        7: IA_Protecteur = True       8: IA_Protecteur = False
        9: IA_Egoiste = True          10: IA_Egoiste = False
        11: IA_Equiper = 0            12: IA_Equiper = 1      13: IA_Equiper = 2
    8 GagnerArgent(building v, q):
        if Argent < q:
            if h.Visible and h.TypeBatiment = v:
                if Chateaux(nf).Visible or Marches.Count > 0: IA_Travail(i) else IA_Agressif(i)
            else: IA2_Choisir_Maison(i, v)
            break
    9 GagnerArgentRessource(resource v, q):
        if Argent < q:
            if Chateaux(nf).Visible or Marches.Count > 0:
                if res[v] < MaxRessources(v): IA_Chercher_Ressource(i, v)
                elif DansUnChateau or DansSonChateau or (DansUneMaison and that house is a Marche):
                    Fiefs(nf).Vendre(v, 0, p, Max=True)
                else: IA_Rentrer_Chateau(i, True)
            else: IA_Agressif(i)
            break
```
If every step is satisfied (no `break`), nothing happens this tick. A `Batiment_` step is
satisfied as soon as `h.TypeBatiment = v` **even if the house is destroyed**
(`TypeBatiment` survives); rebuilding is then driven by the `BAC` branch of §3.1, because
`BAC` keeps its value after the house is built.

`IA_Donner(i, money, resources, objects)` (`ModIA2.bas:710-768`). **Quirk to reproduce:**
the target variable `IndiceCible` is declared `Boolean`:
```
IndiceCible = True                            // "-1"
if Invocation:
    if chef.Vivant and Distance_Persos(chef, p) <= 1024 and (chef.Visible or chef.DansSaMaison):
        IndiceCible = (NumeroChef <> 0)
else:
    repeat LPerso+1 times:
        k = Int(Rnd * LPerso)                 // 1 Rnd each; k in 0..LPerso-1
        if Perso_Allie(p, Persos(k)): IndiceCible = (k <> 0)     // same team (self included)
if IndiceCible > -1:                          // true only if IndiceCible = False, i.e. target 0
    target = 0
    gave = False
    if money and Argent >= 1: ArgentSelectionne = Argent; gave = True
    if resources: for r: if res[r] >= 1 and Persos(0).MaxRessources(r) - Persos(0).res[r] > 1:
                      Ressources_Prendre_Tout; gave = True; break
    if objects and Objet_Nombre_Total(True) >= 1 and not Persos(0).Inventaire_Plein:
        Objet_Prendre_Premier; gave = True
    if gave: Aller_Donner(0)
    return gave
return False
```
So the "give" modes only ever give to **perso index 0**: an invocation gives when its chef is
perso 0 (within 1024 px and visible or at home); a free perso gives when the last allied
index drawn is 0. Perso 0 is usually the human player (`NoPerso = 0` by default,
`FrmParam.frm:3011`; loaded scenarios/campaigns may put the player elsewhere, `ModMain.bas:664-675`),
so apprentices/domestics (whose `.ia` use `ChangerMode_3` "Donner tout") deliver to the
player only if he is their chef. Every evaluation of a non-invocation `ChangerMode_3..6`
consumes `LPerso+1` Rnd.

`IA_Invoquer(i)` (`:770-786`): for each active inventory slot j with
`Objet_Invocation(type) >= 0`: `Objet_Equiper(j)`; if `NumeroObjetUtilise >= 0`: return True.

### 3.7 Work (`IA_Travail`, `ModIA.bas:2119-2204`), only if `Inactif`

```
if DansUneMaison and not QuitterBatiment(i):               // in someone else's building
    m = Maisons(IndiceMaison)
    if m.Magasin:
        if RF(m.type) = MP(h.type): m.Acheter(0, p, Max=True)       // buy max raw material
        else: m.Acheter(Int(10 * Rnd), p)                           // 1 Rnd
    elif m.Fabrique:
        Z = Int(Rnd * Batiment_NombreObjetsFabriques(m.type))       // 1 Rnd
        if m.Stock_Objets(Z) > 0 and not Objet_Posseder(Batiment_ObjetFabrique(m.type, Z)):
            Objet_Acheter_Maison(Z, m)
    elif m.Service_Utilisable(p): m.Service_Utiliser(p)
    IA_Sortir_Batiment(i)
elif DansSaMaison and not QuitterBatiment(i):
    IA_Travailler_Maison(i)
elif ((DansUnChateau or DansSonChateau) and not QuitterBatiment(i))
     or (DansUneMaison and Maisons(IndiceMaison).Marche):
    IA_Transaction_Chateau(i); IA_Sortir_Batiment(i)
elif IA_Argent_Besoin(i) and h.Argent >= 1:  IA_Rentrer_Maison(i)    // fetch the till
elif h.Service = 2:                                                   // mercenary
    if Vie = Definir_Vie:
        IA_Agressif ×3
        if Inactif and h.Argent > 0: IA_Rentrer_Maison(i)
elif res[RF(h.type)] > 1 and h.Definir_Stock < h.MaxStock
     and (RF(h.type) >= 0 or h.Fabrique or h.Service > 0):
    IA_Rentrer_Maison(i)                                              // go home to work
elif (IA_Stock_Exedentaire(i)
      or ((IA_Argent_Besoin(i) or (res[MP(h.type)] <= 1 and MP(h.type) >= 0))
          and Fiefs(nf).res[MP(h.type)] >= 1))
     and (Chateaux(nf).Visible or Marches.Count > 0):
    if DansSonChateau: IA_Rentrer_Maison(i) else IA_Rentrer_Chateau(i, True)
else:
    IA_Agressif(i)
```
`MatierePremiereMin = 1`, `QuantiteMaxTransfert = 10`.

`IA_Stock_Exedentaire(i)` (`:411-423`) as actually written:
`res[ CInt((RF(h.type) > 1) And RF(h.type)) ] >= 0 Or (h.Definir_Stock > h.MaxStock - 1 And
h.Magasin)`. The index is the bitwise `And` of a Boolean (−1/0) and `RF`, i.e. `RF` if `RF > 1`
else 0, and `res[...] >= 0` is always true, so **`IA_Stock_Exedentaire` is always True**. The
selling branch therefore reduces to `Chateaux(nf).Visible Or Marches.Count > 0`, and the final
`IA_Agressif` is only reached when the fief has neither a castle nor a market.

`IA_Travailler_Maison(i)` (`:2206-2254`), in its own house:
```
h.Transfert_Argent_Prendre(0, p, Max=True)                // take the till
if h.Fabrique:
    repeat Z = Int(Rnd * nObj) until Objet_Epoque(Batiment_ObjetFabrique(h.type, Z)) - 1 <= Fiefs(nf).Epoque
    for X in 0..nObj-1:                                     // prefer an out-of-stock craftable item
        if h.Stock_Objets(X) = 0 and PeutConstruire_Objet(X, h) and epoch ok: Z = X; break
    if PeutConstruire_Objet(Z, h) and h.Definir_Stock < h.MaxStock: Creer_Objet(Z, h)
    else:
        if IA_Argent_Besoin(i) or h.Definir_Stock = h.MaxStock:
            take the first object slot X with Stock_Objets(X) >= 1 (Objet_Ramasser_Maison)
        IA_Rentrer_Chateau(i, True)
else:
    if h.Magasin and (h.Definir_Stock <= 1 or not IA_Argent_Besoin(i)):
        if h.Definir_Stock = h.MaxStock: IA_Rentrer_Chateau(i, True)
        else:
            h.Transfert_Stock_Deposer(0.5 * res[RF(h.type)], p)     // put half the product on sale
            IA_Rentrer_Chateau(i, True)
    else: IA_Rentrer_Chateau(i, True)
```
(The `repeat … until` loop consumes one Rnd per try and never ends if no object of the
building is allowed in the current age; the data always has one.)

`IA_ChercherMatieresPremiere(i)` (`ModIA.bas:1459-1492`), `mp = MP(h.type)`:
```
if (Inactif or Action = 202) and not DansUneMaison and h.Construit and res[mp] < 1 and mp >= 0:
    if Ressources_Recoltable(mp): IA_Chercher_Ressource(i, mp)
    elif Argent > 1:
        if Trouver_Maison_Proche(p, i, mp, 0, 0, X): if X >= 0: IA_Aller_Maison(i, X)
        elif (QuitterBatiment(i) and Action <> 201 and Fiefs(nf).res[mp] > 1 and Chateaux(nf).Visible)
             or Marches.Count > 0:
            IA_Rentrer_Chateau(i, True)
```

---------------------------------------------------------------------------------------------

## 4. The `.ia` file format and the shipped AIs

### 4.1 Loading (`ClsJeuIA2.Charger_IA`, `ClsJeuIA2.cls:69-162`; `ModIA2.bas:78-122`)

`Charger_Dossiers_IAs` loads every file of `IAs\` (flag `Joueur = True`, editable and
selectable by the player) then every file of each sub-folder (`Joueur = False`), in `Dir()`
order, into the global list `IA2s()`. Only the bare file name (`Nom`) is kept, so names must
be unique across folders. `Charger_IA(name)` looks a name up case-insensitively.

File: Latin-1 text, CRLF lines, read with `Line Input`. Grammar (prefix matching, in this
order, case-sensitive):

```
line 1  : "#" comment     -> Commentaire (optional; if absent, line 1 is parsed as line 2)
line 2  : "Equiper_" N    -> Equiper = N (optional; parsed into IA2.Equiper but never used by the game)
then, for each line L while not EOF:
  "Age_" N                -> current age index a = N - 1; step counter reset
  "#" text                -> Epoques(a).Commentaire = text  (the last one wins)
  "Ressource_" R "=" Q    -> step {Categorie 0, Valeur R-1, Quantite Q}
  "Batiment_" ID          -> step {1, Batiment_Numero(ID)}; Epoques(a).BatimentDefini = True
  "Objet_" O "=" Q        -> step {2, O-1, Q}
  "ChangerMode_" M        -> step {7, M-1}
  "GagnerArgent_" ID "=" Q-> step {8, Batiment_Numero(ID), Q}
  "GagnerArgentRessource_" R "=" Q -> step {9, R-1, Q}   (tested after "GagnerArgent_", which
                                                         does not match it: char 13 is "R", not "_")
  "Travailler"            -> step {3}
  "Patrouiller"           -> step {4}
  "Attaquer Ennemi"       -> step {5}
  "Attaquer Donjon"       -> step {6}
  anything else (blank)   -> ignored
```

- **The last line of the file is never processed** (the loop processes the previously read
  line and stops when `EOF` becomes true right after reading the last one). Every shipped file
  ends with `#Fin`, which is therefore skipped.
- `Batiment_Numero(ID)` matches the building `ID` string of `Parametres_Batiments.ini` (e.g.
  `11`, `CASERNE`); an unknown ID shows an error and **ends the program**
  (`ClsJeuParam.cls:2526-2544`).
- `Valeur`/`Quantite` are VB `Integer`s (conversion of the text by VB's implicit `CInt`).
- `Epoques()` has `Parametres.NombreEpoques` entries (5). An age missing from the file has no
  step array; `Definir_NombreEtapes` on it raises a runtime error. All shipped files define
  ages 1–5.
- `Sauvegarder_IA` (`:181-212`) writes the canonical form: `#comment`, `Equiper_N`, blank,
  then per age `Age_N`, `#comment`, the steps, a blank line, and finally `#Fin`.
- Creating a new IA from the editor (`Creer_IA_Vierge`, `:164-179`) writes `#`, blank, then
  for each age `Age_N`, `#`, `Travailler`, blank, then `#Fin`.

`ChangerMode_M` values (editor labels, `Donnees\Francais\Fenetre_EditeurIA.ini`
`ListeMode(M)`): 1 pacifist, 2 combative, 3 give everything, 4 give money, 5 give
resources, 6 give objects, 7 summon, 8 protector, 9 not protector, 10 selfish, 11
solidary, 12 do not equip, 13 equip by value, 14 equip by best skill (→ `IA_Equiper` 0/1/2).

Editor rules (`Info_EditeurIA.txt`): steps run in order and the AI passes to the next step
only when the previous one's condition holds; one terminal step (Travailler / Patrouiller /
Attaquer Ennemi / Attaquer Donjon) must end each age.

### 4.2 Which file a perso uses

| folder | used by |
|---|---|
| `IAs\` (56 files) | randomly assigned villagers (§1.4), and the player in automatic mode |
| `IAs\Apprentis\` | invocations created by objects with `InvocationIA_Fichier` (apprentices, `Domestique`) |
| `IAs\Empereur\Empereur.ia` | any `.ia`-driven perso while `Definir_Empereur` is True (replaces its own file) |
| `IAs\Empereur\Milicien imperial.ia` | invocation `InvocationIA_Fichier` |
| `IAs\Empereur\Mercenaire imperial.ia` | not referenced by any data file (identical to Milicien) |
| `IAs\Monstres\` | race `IA_Fichier` (Archimage, Capitaine/Chevalier/General/Soldat imperial, Savant, Vampire, Maman dragon), `Golem escavateur` (invocation), `Archer elfe` (unreferenced) |
| `IAs\Scenarios\Bataille.ia` | every perso in battlefield mode |
| `IAs\Scenarios\Berserk.ia` | campaign `Initiation 3 - Le combat.snr` (`Personnage(2)_Fichier_IA`) |

### 4.3 Shipped files

Appendix B lists all 74 files compactly (one line per age, steps in order; `desc` =
first-line comment; `header` = `Equiper_` line; per-age comments omitted). Numbers in `Ressource_`, `Objet_` are 1-based
indices into `Parametres_Ressources.ini` / `Parametres_Objets.ini`; `Batiment_`/`GagnerArgent_`
take building IDs.

Typical patterns:
- *Villager* (e.g. Alchimiste, Boulanger…): `ChangerMode_1` (pacifist), collect a few
  resources, earn money in a building (`GagnerArgent_`), buy/craft equipment, `Batiment_X`,
  `Travailler`.
- *Offensive* ("… offensif"): pacifist while equipping, then `ChangerMode_2` (combative),
  `Patrouiller` at age 1, then work.
- *Apprentice*: collect 20 of a resource, `ChangerMode_3` (give everything to perso 0 — see
  §3.6), repeat, `Batiment_X`, `Travailler`.
- *Empereur*: `ChangerMode_8` (protector), get summoning objects, `ChangerMode_7` (summon),
  `Patrouiller`.
- *Monsters*: equip/craft summons then `Patrouiller` / `Attaquer Ennemi` / `Attaquer Donjon`.

---------------------------------------------------------------------------------------------

## 5. VBScript AIs (`IAScripts\`)

### 5.1 Execution (`ModIA.bas:178-188`)

For a perso with `IA And ChoisirIAScript`, on each of its round-robin turns (alive or dead):
```
SurPerso1.Init(Persos(i)); SurPerso2.Init(Persos(Persos(i).IndicePerso))
ScriptControl.Reset
AddObject "Me",    SurPerso1, AddMembers=True     // members also callable without prefix
AddObject "Cible", SurPerso2, AddMembers=True
AddObject "Jeu",   SurJeu,    AddMembers=True
AddCode <full script text>                        // loaded once by IA_InitPerso (§1.3)
Run "Jouer"
```
No `IA_Equiper`, no recovery, no built-in behaviour runs for such persos: the script is the
whole AI. A script error is an unhandled runtime error in VB. The port should implement the
shipped behaviours natively (below) and may implement this object model as a Rust trait.

### 5.2 Object model

`ClSurPerso` (`ClSurPerso.cls`) wraps one perso index `Numero`. Everything else in that file
(≈4000 lines) is commented out. Index conventions: resource and object type numbers are
**1-based** in the script API (0 = none), `Epoque` is 1-based, `MaisonType` is 1-based.

Read-only properties (`Property Get`):

| name | value | line |
|---|---|---|
| `RegenerationVie/Energie/Magie/Moral` (Single) | `BonusVitesseRegeneration*` | 43-54 |
| `PositionX`, `PositionY` (Long) | position | 57-62 |
| `Vitesse` (Long) | `Longueur_Pas` | 63 |
| `DirectionX`, `DirectionY` (Long) | movement target | 66-71 |
| `Nom` | name | 73 |
| `Metier` | `Batiment_Metier(Maisons(n).TypeBatiment, Feminin)` | 76 |
| `Largeur`, `Hauteur` (Integer) | size | 80-85 |
| `NumeroFief`, `NumeroEquipe` (Long) | | 86-91 |
| `Epoque` (Integer) | `Fiefs(nf).Epoque + 1` | 92 |
| `Invocation` | `NumeroChef >= 0` | 95 |
| `NomChef` | `Persos(NumeroChef).Nom` | 98 |
| `Incommandable`, `ChefDecalageX/Y`, `Attitude`, `Formation`, `Empereur`, `NombreSoldats` | fields | 101-121 |
| `NomRace` | `Race_Nom(Race, Feminin)` | 122 |
| `Parole`, `Volant`, `ChoisirApparence`, `FichierApparence`, `Titre`, `Feminin`, `GainExperience`, `DureeRessurection`, `CommentairesSpecifiques` | fields | 125-151 |
| `Vie`, `MaxVie`, `ExpVie` (= `Progression_Vie*100`) | | 153-161 |
| `Energie`, `MaxEnergie`, `ExpEnergie`; `Magie`, `MaxMagie`, `ExpMagie`; `Moral`, `MaxMoral`, `ExpMoral` | same pattern | 163-191 |
| `Attaque`, `MaxAttaque`, `ExpAttaque`; `Defense`, `MaxDefense`, `ExpDefense` | | 193-211 |
| `PorteeAttaque` | `Definir_PorteeAttaque` | 213 |
| `EffetAttaque` (String) | `Effet_Nom(EffetAttaque)` | 216 |
| `SurfaceAttaque` | `BonusSurfaceAttaque` | 219 |
| `Kamikaze` | | 222 |
| `PerteVie/Energie/Magie/Moral/Argent`, `PerteRessourceIndice`, `PerteRessourceQuantite`, `Armure` | | 226-249 |
| `FragsPersos`, `FragsBatiments`, `FragsChateaux`, `Morts` | statistics | 362-373 |
| `MaisonPositionX/Y`, `MaisonLargeur/Hauteur`, `MaisonEntreePositionX/Y`, `MaisonConstruit`, `MaisonVie`, `MaisonMaxVie`, `MaisonArgent` | own house | 377-409 |
| `MaisonEnConstruction` | `Visible And Not (Construit And Definir_Vie = MaxVie)` | 398 |
| `MaisonType` | `TypeBatiment + 1` | 416 |
| `Action` (String) | `Persos(n).Action` | 1268 |
| `QuantiteRessources(k)` | `res[k-1]` if `0 < k < Nombre_CompetencesRessources`, else 0 (the last resource is unreachable) | 2135 |
| `Inactif`, `Visible`, `Vivant`, `EnCombat` | | 2140-2488 |
| `Niveau` (Double) | | 2499 |
| `Classe` | `Metier(Parametres)` | 2505 |
| `Objet_Equipe_Type(slot)` | 1-based slot; type+1 or 0 | 4218 |
| `Objet_Inventaire_Type(slot)` | 1-based slot; type+1 or 0 | 4229 |
| `Objet_TotalQuantite(t)` | `Objet_NombreType(t-1)` | 4241 |
| `Objet_EquipeQuantite(t)` | `Objet_EquipeNombreType(t-1)` | 4245 |
| `Objet_SacQuantite(t)` | count of inventory slots of type t-1 | 4249 |

Read/write: `PersoCible` (Long, ↔ `IndicePerso`, 281-286), `Dire_Commentaires` (String, ↔
`Definir_Commentaires`, i.e. the speech bubble, 2577-2582).

Methods (all return Boolean unless noted):

| method | effect | line |
|---|---|---|
| `MaisonBatir(X, Y, Type)` | if `Vivant And Obstacle(n, X, Y, W(Type), H(Type)) = 0 And PeutConstruire_Maison(Type)`: `Maisons(n).Creer(X, Y, Type, n)`. **Type is 0-based here** (not decremented); never returns True | 419 |
| `Aller_A(X, Y)` | if `0 <= X < Monde.Largeur*TL - Largeur` and same for Y: `Persos(n).Aller_A(X, Y)`, True | 667 |
| `Arreter()` | `Persos(n).Arreter`, True | 677 |
| `Rentrer_Maison()` | if own house visible: `Rentrer_Maison`, True | 682 |
| `Rentrer_Chateau()` | `Rentrer_Chateau(own castle)`; returns `Action = 202` | 695 |
| `Donner_Ressource(k, Q, Cible, Max)` | if target alive and visible: `Ressources_Prendre(k-1, Q, Max)`, `Aller_Donner(Cible)`; returns Empty | 732 |
| `Deplacement_Aleatoire()` | `IA_Deplacement_Aleatoire(n)`, True | 2779 |
| `Comportement_Agressif()` | if alive: `IA_Agressif(n)`, True | 2783 |
| `Comportement_Berserk()` | if alive: `IA_ComportementBerserk(n)`, True | 2833 |
| `Attaquer_Perso_Proche(Env, Ctrl, Shift, Alt)` | if `Trouver_Ennemi(n) >= 0` and alive: `IA_Trouver_Ennemi(n, …)`, True | 2954 |
| `Attaquer_Maison_Proche(…)` | same with houses | 3005 |
| `Attaquer_Chateau_Proche(…)` | same with castles | 3036 |
| `Aller_Ressource(T)` | if alive: `IA_Trouver_Ressource(n, T)` (T is a terrain *appearance* index, not decremented); True if now `Action = 3` and the node's `Apparence = T` | 3283 |
| `Ramasser_Tresor_Proche()` | if alive and treasures exist: `IA_Trouver_Tresor(n)`, True | 3553 |
| `Fabriquer_Ressource(k, Q)` | if alive and `Ressources_Epoque(k-1) <= Epoque`: `IA2_Fabriquer_Ressource(n, k-1, Q)`, True | 4196 |
| `Fabriquer_Objet(t, Q)` | if alive, `Objet_Epoque(t-1) <= Epoque` and not secret: `IA2_ChercherObjet(n, t-1, Q)`, True | 4206 |
| `Objet_SEquiper(t)` | equip the first inventory slot holding type t-1, True | 4260 |

`ClsSurJeu` (`ClsSurJeu.cls:32`), object `Jeu`: one property `Duree` =
`FrmMoteur2D.DureePartie / Jeu.Definir_Vitesse` (elapsed frames / game speed).

### 5.3 Shipped scripts

Only `IAScripts\Test.txt` is shipped:
```vbscript
Sub Jouer()
    If Me.Inactif Then
        Me.Deplacement_Aleatoire
        Me.Dire_Commentaires = "Je vais à X=" & Me.DirectionX & ", Y= " & Me.DirectionY
    End If
End Sub
```
Pseudocode: `if p.Inactif { IA_Deplacement_Aleatoire(p); p.say(format!("Je vais à X={}, Y= {}",
p.DirectionX, p.DirectionY)) }`. No data file references it; it is only reachable when the
player picks it in FrmParam (option "script") or a scenario uses `fichier_iascript`.

---------------------------------------------------------------------------------------------

## 6. Automatic equipment

### 6.1 `IA_Equiper(i)` (`ModIA.bas:1630-1768`), every AI turn of an alive non-script perso

`IndiceObjet` is one **global** inventory-slot cursor shared by all persos. Per call, it
examines `2` slots (`VitesseEquipement = 1`, loop `0..=1`) or `6` slots when `EnCombat`
(`VitesseEquipementCombat = 5`), advancing and wrapping at
`Persos(i).Nombre_Objets_Inventaire - 1`.

For each examined slot `s` that is active, with `t = Objet_Inventaire_Type(s)`:
```
if Objet_Utilisable(t):                       // consumables, spells, summons
    if Objet_UtilisationUnique(t): if EnCombat: Objet_Equiper(s)      // single-use only in combat
    else: Objet_Equiper(s)                                          // use immediately
else:
    c = Objet_Categorie(t)
    better(e) =  IA_Equiper=1: Objet_PrixVente(t) > Objet_PrixVente(Objet_Equipes_Type(e))
                 IA_Equiper=2: skill(t) > skill(e) or (skill(t) = skill(e) and price(t) > price(e))
                 where skill(x) = Definir_Niveau_CompetenceObjets(Objet_Competence(x))
    c = 0 (one-hand weapon): if not Actif(0) or not Actif(1): equip  else if better(slot 0): equip
    c = 1 (two-hand weapon):
        if not Actif(0): equip
        elif DansUneMaison or (DansSaMaison and Batiment_Tir(Maisons(IndiceMaison).type)
                               and Objet_BonusPorteeAttaque(t) > Objet_BonusPorteeAttaque(Equipes_Type(0))):
            equip          // (precedence: any two-hander is equipped whenever DansUneMaison)
        elif better(slot 0): equip
    other c: if not Actif(c): equip elif better(slot c): equip
```
`IA_Equiper = 0` or `3`: nothing. "equip" = `Objet_Equiper(s, …)` (03 §5.4).

### 6.2 `IA_Mort_Equiper(i)` (`:1770-1788`)

Ghosts with `IA_Equiper > 0`: examine 2 slots with the same global cursor; equip
(= use) any active item with `Objet_UtilisationMort(t)` (resurrection items).

---------------------------------------------------------------------------------------------

## 7. AI economy helpers

### 7.1 `IA_Transaction_Chateau(i)` (`ModIA.bas:2256-2319`) — in a castle or a market

```
if not h.Magasin or IA_Argent_Besoin(i) or h.Definir_Stock >= h.MaxStock - 1:
    if h.Fabrique:
        sold = 0
        for X in inventory slots: if active:
            for Y in objects of h.type: if Objet_Inventaire_Type(X) = Batiment_ObjetFabrique(h.type, Y):
                Objet_Vendre_Chateau(X, Fiefs(IndiceChateau)); sold += 1
        if sold = 0 and h.Argent = 0 and h.Definir_Stock = 0:
            h.Demolir(p)                                    // bankruptcy
    elif RF(h.type) >= 0:
        Fiefs(nf).Vendre(RF(h.type), 0, p, Max=True)        // sell all its product
IA_Devenir_Empereur(i)                                      // §7.3
IA_Acheter_Ressources_Pour_Manger(i)                        // §7.4
IA_Acheter_Ressources_Pour_Travailler(i)
if not IA_Argent_Besoin(i): IA_Acheter_Ressources_Pour_Construire(i)
IA_Restaurer_Chateau(i)                                     // §7.6
IA_Sortir_Batiment(i)
```
(Objects are sold to `Fiefs(IndiceChateau)`, the last castle entered, even in a market.)

### 7.2 `IA_Vente_Surplus_Objets(i)` (`:2388-2405`)

If in a castle or in a market: for inventory slots from last to first, sell
(`Objet_Vendre_Chateau(j, Fiefs(nf))`) any non-single-use object of which the perso owns
more than 1 (categories ≠ 0) or more than 2 (category 0, one-hand weapons).

### 7.3 `IA_Devenir_Empereur(i)` (`:2337-2386`)

If the own castle is visible and the fief has no emperor (`Fiefs(nf).Empereur < 0`),
`P = Fiefs(nf).Empereur_Prix = Epoque_EmpereurPrix(Epoque)`:
```
if Argent >= P: Empereur_Changer(i, Payer=True)
elif Argent + Ressources_TotalPrix + Objets_TotalPrix >= P:
    j = 0; do: Fiefs(nf).Vendre(j, 0, p, Max=True); j += 1
           loop until j = Nombre_CompetencesRessources or Argent >= P
    if Argent >= P: Empereur_Changer(i, True)
    else:
        j = 0; do: if inventory slot j active: Objet_Vendre_Chateau(j, Fiefs(nf), …, True); j += 1
               loop until j = Nombre_Objets_Inventaire or Argent >= P
        if Argent >= P: Empereur_Changer(i, True)
        else:
            j = 0; do: if equipped slot j active: Objet_Ranger(j); Objet_Vendre_Chateau(0, …, True); j += 1
                   loop until j = Nombre_Objets_Equipes or Argent >= P
            // (no final Empereur_Changer: becomes emperor on a later visit)
```
`Empereur_Changer(i, Payer)` (`ClsJeuFief.cls:112-131`): for all persos of the fief:
`i` gets `Definir_Empereur = True` (which grants every object with `Objet_Empereur`), any
previous emperor gets False; pays `Empereur_Prix` if `Payer`; `Fiefs.Empereur = i`.

### 7.4 Buying at the castle

`IA_Acheter_Ressources_Pour_Manger(i)` (`:1539-1586`), only if not poor; the **first**
matching purchase ends the procedure:
1. if `Moral < MaxMoral`: highest-index resource `j` with `Ressources_GainMoral(j) > 0` and
   not harvestable → `Fiefs(nf).Acheter(j, 0, p, Max=True)`;
2. else-if-not-returned, same for `Vie`/`GainVie`, then `Energie`/`GainEnergie`, then
   `Magie`/`GainMagie`.

`IA_Acheter_Ressources_Pour_Travailler(i)` (`:1588-1613`), if `h.Visible`:
- `Batiment_Fabrique(h.type)`: for each non-harvestable resource `j` used by at least one
  object of the building (`Objet_PrixRessources(obj, j) > 0`): `Fiefs(nf).Acheter(j, 0, p, Max=True)`;
- else if `MP(h.type) >= 0` and not harvestable: `Fiefs(nf).Acheter(MP, 0, p, Max=True)`.

`IA_Acheter_Ressources_Pour_Construire(i)` (`:1614-1628`), if `BAC >= 0`: for each resource
`j` with `Batiment_PrixRessource(BAC, j) > res[j]`, `j <> RF(h.type)` and not harvestable:
`Fiefs(nf).Acheter(j, 10 * Rnd, p)` (one Rnd per such resource; quantity is a Double).

### 7.5 Buying in shops: see `IA_Travail` (§3.7) and `IA_Recuperation_Carac` (§2.6).

### 7.6 `IA_Restaurer_Chateau(i)` (`:2321-2336`)

If the own castle is visible and damaged: donate
`Int(Argent - Argent * Chateau.Definir_Vie / Chateau.MaxVie) + 1` gold via
`Fiefs(nf).Restaurer_Chateau(q, p)` (`ClsJeuFief.cls:101`: clamped to the perso's money;
added to the fief treasury `Fiefs.Argent`). There is no `IA_Egoiste` test here.

---------------------------------------------------------------------------------------------

## 8. Fief tick (`Comportement_IA_Fiefs`, `ModIA.bas:289-297`)

Every frame, for each fief `j`: `Fiefs(j).Transaction_Ressources`, `Fiefs(j).Restauration
Chateaux(j)`, `Chateaux(j).Restauration` (formulas in 02-world; summary):

- `Transaction_Ressources` (`ClsJeuFief.cls:80-99`): `IndiceMin` = first resource with the
  smallest `Int(stock)` among resources available in the current age (start index 0
  regardless of its age); `IndiceMax` = first with the largest `Int(stock)` (any age); if
  `|stock(max) - stock(min)| > 1`: move `VitesseTransaction * NombreCitoyens`
  (`0.02/4/4 = 0.00125` per citizen) from max to min.
- `Fief.Restauration` (`:138-156`): if the castle is damaged, convert treasury into castle
  `RecuperationVie`: `q = min(MaxVie - Vie + 1, Argent * 0.2)`, `Argent -= q / 0.2`; then
  the age timer: if `Epoque < NombreEpoques - 1`: `TempsChangementEpoque -= 1`; if `<= 0`
  and `ChangerEpoque`: `Epoque += 1`, `ChangerEpoque = False`.
- `Chateau.Restauration` (`ClsJeuBatiment.cls:155`): applies `RecuperationVie` in steps of
  `VitesseRecuperationVie`.

---------------------------------------------------------------------------------------------

## 9. The command layer (`ModCommandes.bas`)

### 9.1 Queue

A command is a `ClsCommande {Categorie: Integer, Indice: Long, Var1: String, Var2: String}`
(`ClsCommande.cls`). All arguments are converted to **strings** when queued (Booleans become
`"True"`/`"False"`, numbers their decimal text) and converted back implicitly when executed.

- `Ajouter_Commande(cat, i, v1, v2)` (`:114-129`): on a network client, send it to the
  server (`Message_Envoyer(Categorie1Commandes=13, cat, i, , v1, v2)`), which re-queues it
  through `ClsComMessages.Analyser` (`ClsComMessages.cls:1509`); otherwise append to the
  local collection.
- `Lire_Commandes` (`:134-244`), once per frame before the AI: executes every queued command
  in order, then clears the queue. `On Error GoTo Erreur`: **any runtime error clears the
  whole remaining queue** silently.
- Some commands are emitted as **pairs**: the first one reads the second's fields via
  `Commandes(i + 1)`; the second has no handler (a no-op when its turn comes). If the partner
  is missing the read errors and the queue is dropped.

Commands are the only way the human player (keyboard/mouse, `ClsIntTouches`, `ClsInterface`,
`FrmMoteur2D`) and the `Ordonner_*` dialogue functions change the world. The AI itself calls
the perso methods directly (no queue, no validation besides its own logic).

### 9.2 Command table

`p = Persos(Indice)`, `h = Maisons(Indice)`, `F = Fiefs(p.NumeroFief)`. "Emitter" lists the
`Cmd_*` wrapper and its main callers; UI-side checks are listed under "validation" (the
executor itself never validates beyond what the called method does).

| # | constant | wrapper (args) | effect at execution | validation / emitters |
|---|---|---|---|---|
| 0 | PersoDeplacer | `Cmd_Perso_Deplacer(i, X, Y)` | `p.Aller_A(X, Y)` (Action 1; soldiers follow, chef says comment 23) | mouse on terrain (`FrmMoteur2D.frm:860`), `Ordonner_Deplacement`, `Ordonner_Garder_Perso` |
| 81 | PersoDeplacerSansCommentaire | `(i, X, Y)` | `p.Aller_A(X, Y, SansCommentaire=True)` | arrow keys (`ClsIntTouches.cls:302-345`): step = `BonusDeplacement+1` (ghost: `Definir_VitesseDeplacementFantome+1`), clamped to the map |
| 1 | PersoChercherRessource | `Cmd_Perso_Chercher_Ressource(i, k)` | `p.Aller_Ressource(Ressources(k), k, …)` (Action 3) | click on a resource node / Action key on a node; requires `Vivant` |
| 2 | PersoRentrerChateau | `(i)` | `p.Rentrer_Chateau(Chateaux(p.NumeroFief))` (Action 202; refused if `Hostile`) | castle button, key, click on own castle |
| 3 | PersoRentrerMaison | `(i, Rentrer)` | `p.Rentrer_Maison(h, Rentrer)` (101, or 7 = repair) | house button/key: `Rentrer = Ctrl And h.Construit`; key requires `Vivant Or Maison.Service = 1` |
| 4+5 | PersoCreerMaisonCoordonnees + Type | `Cmd_Perso_Creer_Maison(i, X, Y, Type)` | `h.Creer(X, Y, Type, i)` (Type read from the 2nd command) | build cursor click (`FrmMoteur2D.frm:821`), placement checks done by the UI cursor |
| 6 | PersoDefinirIA | `Cmd_Perso_Definir_IA(i, IA)` | `p.IA = IA` | **queued only if `p.ChoisirIA`** (so soldiers without an IA file never toggle); ModeAuto key, selection of soldiers (off) / deselection (on) |
| 7+8 | PersoAttaquerTerrain + Cible | `(i, X, Y, TypeCible)` | `p.Attaquer_Terrain(X, Y, TypeCible)` (Action 303 if it has a projectile) | middle click; `TypeCible` 1 if cursor on enemy house (10), 2 on enemy castle (11), else 0 |
| 9 | PersoVendreObjetsSelectionnees | `(i, Fief)` | `Fiefs(Fief).Vendre_ObjetsSelectionnees(p)` | click while an object is selected and inside a castle/market |
| 11 | PersoObjetDeposerSol | `(i, Objet, Inventaire)` | `p.Objet_Deposer_Sol(Objet, Inventaire)` | drop selected object outside (not inseparable) |
| 12 | PersoRessourcesDeposerSol | `(i)` | `p.Ressources_Deposer_Sol` | drop selected resources |
| 13 | PersoDefinirObjetSelectionne | `(i, Objet)` | `p.ObjetSelectionne = Objet` | after dropping (−1) |
| 14 | PersoDefinirChefPosition | `(i, X, Y)` | `p.ChefPositionX/Y = X/Y` | `Ordonner_Deplacement` |
| 15 | PersoDefinirIAOrdre | `(i, Ordre)` | `p.IA_Ordre = Ordre` | `Ordonner_*` |
| 78 | PersoDefinirIAAttitude | `(i, Attitude)` | `p.Attitude = A; p.IA_Ordre = 0; Comportement_IA_Persos(i)` | attitude buttons/keys 1–5 for every selected soldier (`ClsInterface.cls:4379`), which also clears the selection |
| 16 | PersoDefinirCommentaires | `(i, Text)` | `p.Definir_Commentaires = Text` | `Ordonner_*` |
| 17 | PersoAllerVendre | `(i, Fief/Maison, Marche)` | `p.Aller_Vendre(idx, Marche)` (sells at once if already there, else Action 206/207) | click castle/market with an object selected |
| 18 | PersoAllerMaison | `(i, Maison, Reparer)` | `p.Aller_Maison(Maisons(k), Reparer)` (201 or 7) | click on an allied house (`Reparer = True` on a house under construction, Ctrl with the Action key), temple for ghosts |
| 19 | PersoAllerChateau | `(i, Chateau)` | `p.Aller_Chateau(Chateaux(k), k)` (Action 203) | click on an allied castle |
| 20 | PersoDefinirIACible | `(i, Cible)` | `p.IA_Cible = Cible` | `Ordonner_*` |
| 21 | PersoAttaquerPerso | `(i, k)` | `p.Attaquer_Perso(k)` | click/Action key on an enemy; `Ordonner_Attaquer_Perso` |
| 22 | PersoAttaquerMaison | `(i, k)` | `p.Attaquer_Maison(k)` | |
| 23 | PersoAttaquerChateau | `(i, k)` | `p.Attaquer_Chateau(k)` | |
| 24 | PersoAllerTresor | `(i, k)` | `p.Aller_Tresor(Tresors(Int(k)), k, …)` | click/Action key on a treasure, requires `Vivant` |
| 25 | PersoAllerDonner | `(i, k)` | `p.Aller_Donner(k)` (Action 205) | click on an ally (cursor 14) |
| 26 | PersoObjetDeSelectionne | `(i)` | `p.Definir_ObjetSelectionne = False` | right click while holding an object (also `Cmd_Perso_Arreter` if Action 206/207) |
| 27+28 | PersoOrdonnerChargez + 2 | `Cmd_PersoOrdonnerChargez(i, Type, Cible, Ctrl, Shift, Alt)`; 2nd command = `(Indice=Ctrl, Shift, Alt)` | `Ordonner_Chargez(i, Type, Cible, Ctrl=CBool(2nd.Indice), Shift, Alt)` (§10.2) | right click on an enemy perso/house/castle (`Type = cursor − 9`) |
| 51 | PersoOrdonnerFormation | `(i, ChangerFormation)` | `Ordonner_Formation(i, ChangerFormation)` (§10.3) | formation key/button (`True`) |
| 29 | PersoRecupererCarac | `(i, Carac)` | 0 `Recuperer_Vie`, 1 `_Energie`, 2 `_Magie`, 3 `_Moral` | interface buttons |
| 30 | PersoObjetCreer | `(i, Objet)` | `p.Creer_Objet(Objet, h)` | craft button |
| 61 | PersoObjetArreterFabrication | `(i)` | `p.Arreter; p.EtatFabricationObjet = 0` | |
| 63 | PersoObjetMaisonPrendre | `(i, k, Quantite)` | `p.Objet_Ramasser_Maison(k, h, …, Quantite)` | |
| 69 | PersoObjetMaisonDeposer | `(i, k, Inventaire)` | `p.Objet_Deposer_Maison(k, h, Inventaire)` | |
| 64 | PersoObjetMaisonAcheter | `Cmd_Perso_ObjetMaisonAcheter(i, Magasin, k, Quantite)` emits **Quantite** commands `(i, Magasin, k)` | `p.Objet_Acheter_Maison(k, Maisons(Magasin))` | |
| 70 | PersoObjetChateauVendre | `(i, k, Inventaire)` | `p.Objet_Vendre_Chateau(k, F, Inventaire)` | |
| 31 | PersoUtiliserService | `(i, Maison)` | `Maisons(k).Service_Utiliser(p)` | Action key in a service building if `Service_Utilisable` |
| 32 | PersoDetruireMaison | `(i)` | `h.Demolir(p)` | demolish key/button |
| 33 | PersoRessourceManger | `(i, r)` | `p.Manger_Ressources(r)` | |
| 34 | PersoRessourcesDeposerMaison | `(i, Q, Max)` | `h.Transfert_Stock_Deposer(Q, p, Max)` | |
| 62 | PersoRessourcesAcheterMaison | `(i, Maison, Q)` | `Maisons(k).Acheter(Q, p)` | |
| 57 | PersoRessourcesAcheterChateau | `(i, r, Q)` | `F.Acheter(r, Q, p)` | |
| 35 | PersoRessourcesVendreChateau | `(i, r, Q)` | `F.Vendre(r, Q, p)` | |
| 36 | PersoTuerPerso | `(i, k)` | `Persos(k).Vivant = False` | Ctrl+"delete soldiers" key, for each selected soldier other than the chef |
| 37 | PersoTrouverRessource | `(i, T)` | `IA_Trouver_Ressource(i, T)` (T = terrain appearance 0–7) | keys TrouverRessource1–8, requires `Vivant` |
| 38+39 | PersoTrouverPersoEnnemi + 2 | `(i, Env, Ctrl)` + `(i, Shift, Alt)` | `IA_Trouver_Ennemi(i, Env, Ctrl, Shift, Alt)` | key, `Env = Ctrl Or Shift Or Alt`; with soldiers selected the UI uses `Ordonner_Attaquer_Perso(Trouver_Ennemi)` instead |
| 40+41 | PersoTrouverMaisonEnnemi + 2 | same | `IA_Trouver_MaisonEnnemi(…)` | |
| 42+43 | PersoTrouverChateauEnnemi + 2 | same | `IA_Trouver_ChateauEnnemi(…)` | |
| 44 | PersoTrouverTresor | `(i)` | `IA_Trouver_Tresor(i)` | key, requires `Vivant` |
| 45 | PersoRessourcePrendre | `(i, r, Q)` | `p.Ressources_Prendre(r, Q)` (select for giving) | |
| 46 | PersoRessourceLaisser | `(i, r, Q)` | `p.Ressources_Prendre(r, Q, , True)` (unselect) | |
| 77 | PersoArgentPrendre | `(i, Q)` | `p.Argent_Prendre(Q)` | |
| 47 | PersoArgentMaisonPrendre | `(i, Q)` | `h.Transfert_Argent_Prendre(Q, p)` | key "take money" in own house |
| 48 | PersoArgentMaisonLaisser | `(i, Q)` | `h.Transfert_Argent_Deposer(Q, p)` | key "deposit money" (all) in own house |
| 58 | PersoRessourceMaisonPrendre | `(i, Q)` | `h.Transfert_Stock_Prendre(Q, p)` | |
| 59 | PersoRessourceMaisonLaisser | `(i, Q)` | `h.Transfert_Stock_Deposer(Q, p)` | |
| 49 | PersoAppelerDelAide | `(i)` | `Appeler_De_lAide Persos(.Indice)` — passes an **object** to a `Long` parameter → runtime error → queue dropped. Never emitted (UI calls are commented out) | — |
| 50 | PersoArreter | `(i)` | `p.Arreter` | stop key; `Ordonner_Arret` |
| 52 | PersoObjetRanger | `(i, slot)` | `p.Objet_Ranger(slot)` | |
| 53 | PersoObjetEquiper | `(i, slot, Inventaire)` | `p.Objet_Equiper(slot, …, Inventaire)` | |
| 74 | PersoObjetEquiperADroite | `(i, slot, Inventaire)` | `p.Objet_Equiper(slot, …, Inventaire, True)` | |
| 65 | PersoObjetSelectionne | `(i, k)` | `p.ObjetSelectionne = k` | |
| 76 | PersoObjetEquipesRanger | `(i, slot)` | `p.Objet_Ranger(slot)` | |
| 68 | PersoObjetEquipesDetruire | `(i, slot)` | `p.Objet_Equipes_Detruire(slot)` | |
| 75 | PersoObjetInventaireAjouter | `(i, type)` | `p.Objet_Ajouter_Inventaire(type, …, True)` | |
| 72 | PersoObjetInventaireCharger | `(i, slot, type)` | `p.Objet_Inventaire_Charger(slot, type)` | |
| 66 | PersoObjetInventaireDetruire | `(i, slot)` | `p.Objet_Inventaire_Detruire(slot)` | |
| 71 | PersoObjetInventaireNombre | `(i, slot, n)` | `p.Objet_Inventaire_Nombre(slot) = n` | |
| 10 | PersoObjetMaisonAjouter | `(i, k)` | `p.Objet_DeposerInventaire_Maison(k, h)` | |
| 73 | PersoObjetMaisonCharger | `(i, slot, type)` | `h.Definir_ObjetsInventaire(slot) = type` | |
| 67 | PersoObjetMaisonDetruire | `(i, slot)` | `h.Definir_ObjetsInventaire(slot) = -1` | |
| 55 / 56 | PersoPrendreRessource / DeposerRessource | `Cmd_Perso_PrendreRessource(i, r, Q, Negatif)` | `Ressources_Prendre(r, Q)` / `Ressources_Prendre(r, Q, False, True)` | |
| 60 | PersoRestaurerChateau | `(i, Q)` | `F.Restaurer_Chateau(Q, p)` | castle window |
| 79 | PersoEffetTempEffacer | `(i, k)` | `p.EffetTemp_Effacer(k)` | |
| 80 | PersoEnvoyerCommentaire | `(i, Text)` | `Definir_Commentaires = ""` then `= Text` | chat line (§11.3) |
| 82 | PersoHostile | `(i)` | `Perso_Hostile(p)` (§11.4) | chat cheat `_regner sans partage` |
| 83 | PersoDevenirEmpereur | `(i)` | `F.Empereur_Changer(i, Payer=True)` | Action key in own castle if `PeutDevenir_Empereur` (`Not Fief.Empereur >= 0 And Argent >= Empereur_Prix And Vivant`) |
| 84 | PersoAttaquerAllies | `(i)` | `p.Attaquer_Allies(True)` (§11.4) | Help key → confirmation window → Valider |
| 54 | PersoVendreRessourceFief | — | no handler, never emitted | |

---------------------------------------------------------------------------------------------
## 10. Dialogue: calls for help and orders (`ModDialogue.bas`)

There is **no diplomacy, proposal or negotiation system** in Vendetta 3.0.0.17: no
character ever proposes, accepts or refuses anything. "Dialogue" consists of (a) calls for
help that make allies join a fight, (b) orders from a chef to his soldiers (invocations),
(c) speech bubbles (`Definir_Commentaires`) chosen from `Commentaires.ini`, and (d) the
player's free-text chat line. Team membership (`NumeroEquipe`) is the only relation;
it changes only through `Perso_Hostile` and `Attaquer_Allies` (§11.4).

Constants (`ModDialogue.bas:20-23`): `ChanceRepondreAide = 3`, `ChanceRepondreOrdre = 0.1`,
`IntervalSoldatsX = IntervalSoldatsY = 24`.

### 10.1 `Appeler_De_lAide(V)` (`:25-132`) — V is the perso calling for help

Called from combat (`ModCombat.bas:279`: a defender below half life, with less life than
its attacker, not an invocation, `Rnd > 0.2`; `:580` and `:599`: buildings under attack —
see 03 §6.3/§6.7). `pne = Persos(V).NumeroEquipe`.

```
for i in 0..UBound(Persos):                          // every potential helper, index order
  q = Persos(i)
  if q.NumeroEquipe <> pne or not q.IA: continue
  if q.IA_Protecteur and q.NombreSoldats > 4:        // a chef sends his troops
      match Persos(V).Action:
        300: if V.IA: V.say(6);  Ordonner_Chargez(i, 0, V.IndicePerso, Shift=True); q.say(10)
        301: if V.IA: V.say(7);  Ordonner_Chargez(i, 1, V.IndiceMaison, Shift=True); q.say(10)
        302: if V.IA: V.say(8);  Ordonner_Chargez(i, 2, V.IndiceChateau, Shift=True); q.say(10)
        else: for j in 0..UBound(Persos):
                  if (P_j.Action = 300 and P_j.IndicePerso = V)
                     or (P_j.Action = 301 and P_j.IndiceMaison = V and Rnd < 0.5):   // Rnd every j
                      if V.IA: V.say(9)
                      Ordonner_Chargez(i, 0, P_j.Numero, Shift=True); q.say(10); break
  elif not q.Invocation and not q.IA_Pacifique and not q.IA_Egoiste:
      if q.Action < 102 and q.Vitalite = 1:
          if Rnd < 3 / (Fiefs(V.NumeroFief).NombreCitoyens + 1):            // 1 Rnd
              same match as above, but q itself acts:
                300: q.Attaquer_Perso(V.IndicePerso)   301: q.Attaquer_Maison(V.IndiceMaison)
                302: q.Attaquer_Chateau(V.IndiceChateau)
                else: first attacker j as above (Rnd per j): q.Attaquer_Perso(j)
              with the same V.say(6|7|8|9) (if V.IA) and q.say(10)
```
`X.say(n)` means `X.Definir_Commentaires = ""` then `= Commentaires.Message(n, X)` for the
caller V, and only `= Commentaires.Message(n, q)` for the helper (each `Message` consumes one
Rnd, §11.1). `Shift=True` with `Ctrl=False` means "send at most 5 soldiers" (§10.2).
Note V itself can be its own helper (it is in its team and may be `IA`). The help
probability is `3 / (citizens + 1)` (≥ 1 for fiefs of ≤ 2 citizens).

### 10.2 `Ordonner_Chargez(C, TypeCible=-1, IndiceCible=-1, Ctrl, Shift, Alt, SansCommentaires)` (`:134-333`)

Order chef C's soldiers to attack (or to rejoin the formation). Called by every
ClsJeuPerso order method when `NombreSoldats > 0` (`Aller_A` passes its `SansCommentaire`;
`Rentrer_Maison`, `Rentrer_Chateau`, `Aller_Chateau`, `Aller_Ressource`, `Aller_Tresor`,
`Attaquer_Perso/Maison/Chateau`, resurrection), by the AI (§3.6), by calls for help and by
command 27.
```
if TypeCible = -1:
    match C.Action: 300 → (0, IndiceCible or C.IndicePerso)
                    301 → (1, IndiceCible or C.IndiceMaison)
                    302 → (2, IndiceCible or C.IndiceChateau)
NombreMax = Shift ? (Ctrl ? 10 : 5) : (Ctrl ? 1 : C.NombreSoldats)
sent = 0
for i in 0..UBound(Persos):
    s = Persos(i)
    if s.NumeroChef = C and s.Vivant and not s.EnCombat and not s.Incommandable
       and s.Attitude = 0 and s.IA_Ordre = 0 and not s.ChoisirIA and s.Action not in {2, 3}:
        match TypeCible:
          0: s.Attaquer_Perso(IndiceCible);  s.IA_Ordre = 1; s.IA_Cible = IndiceCible
          1: s.Attaquer_Maison(IndiceCible); s.IA_Ordre = 2; s.IA_Cible = IndiceCible
          2: s.Attaquer_Chateau(IndiceCible); s.IA_Ordre = 3; s.IA_Cible = IndiceCible
          else: s.Aller_A(C.DirectionX + s.ChefDecalageX, C.DirectionY + s.ChefDecalageY)
                s.ChefPositionX/Y = s.PositionX/Y          // the soldier's position *before* moving
        if Rnd < 0.1 and not SansCommentaires: s.Definir_Commentaires = Message(31, s)   // Rnd always consumed
        sent += 1
        if sent >= NombreMax: break
if sent > 0 and not SansCommentaires:
    C.Definir_Commentaires = ""
    C.Definir_Commentaires = Message(TypeCible 0: sent=1 ? 25 : 26
                                     1: 27 / 28;  2: 29 / 30;  else: 23, C)
```
Soldiers already holding an order (`IA_Ordre > 0`), in a non-zero attitude or with an
`.ia` file are never re-ordered. An attacking soldier keeps its `IA_Ordre` until the target
becomes invalid (§2.5).

### 10.3 `Ordonner_Formation(C, ChangerFormation, SansCommentaires)` (`:371-468`)

```
if ChangerFormation: C.Formation = (C.Formation + 1) mod 5         (Changer_Formation)
R = CLng(Int(Sqr(C.NombreSoldats)) * 1.5); if R = 0: R = 1
match C.Formation:
  0: R = CLng(R / 1.5); X = CLng(24*R/2 - 12); Y = CLng(24*R/2 + 12)
  1, 2: X = CLng(24*R/2 - 12); Y = 0
  3, 4: X = 0; Y = CLng(24*R/2 - 12)
j = 0
for i in 0..UBound(Persos):
  s = Persos(i)
  if s.NumeroChef = C and s.Vivant and s.IA_Ordre = 0 and s.Attitude = 0
     and not s.Incommandable and not s.ChoisirIA:
      match C.Formation:
        0, 1: if j mod R = 0: Y -= 24; X -= 24*(R-1) else X += 24
        2:    if j mod R = 0: Y += 24; X -= 24*(R-1) else X += 24
        3:    if j mod R = 0: X -= 24; Y -= 24*(R-1) else Y += 24
        4:    if j mod R = 0: X += 24; Y -= 24*(R-1) else Y += 24
      if X = 0 and Y = 0: i -= 1                     // slot (0,0) is the chef: redo this soldier
      else:
          s.ChefDecalageX/Y = X/Y
          if not C.EnCombat:
              s.Aller_A(C.DirectionX + X, C.DirectionY + Y); s.ChefPositionX/Y = s.DirectionX/Y
      j += 1
if not SansCommentaires: C.Definir_Commentaires = Message(C.EnDeplacement ? 23 : 22, C)
```
Formation 0 is a block in front of/around the chef, 1 rows going up, 2 rows going down,
3 columns going left, 4 columns going right.

### 10.4 Other orders (UI side, they emit commands)

- `Ordonner_Arret(C, ArmeeSelectionnee, Soldats)` (`:335-369`): for the selected soldiers
  (or, if none selected, all commandable alive soldiers of C without `.ia` file):
  `Cmd_Perso_Arreter`, `Cmd_Perso_DefinirIAOrdre(…, 0)`; then C says comment 21
  (`Cmd_Perso_DefinirCommentaires` twice: `""` then the message).
- `Ordonner_Activer_IA(C, IA=True)` (`:486-500`): for each alive soldier of C whose `IA <> IA`:
  `Cmd_Perso_Definir_IA` (only effective for soldiers with an `.ia` file).
- `Ordonner_Deplacement(C, Soldats, X, Y)` (`:502-525`): per selected soldier: Deplacer to
  `(X + ChefDecalageX, Y + ChefDecalageY)`, DefinirChefPosition (same point), IAOrdre 0,
  `Rnd < 0.1 And soldier <> C` → comment 31; then C comment 23.
- `Ordonner_Attaquer_Perso/Maison/Chateau(C, Soldats, Cible)` (`:527-609`): if `Cible >= 0`,
  per selected soldier: Attaquer*, IAOrdre 1/2/3, IACible; `Rnd < 0.1` → comment 31; then C
  comment 25/26, 27/28, 29/30 (1 soldier / several).
- `Ordonner_Garder_Perso(C, Soldats, Cible)` (`:611-633`): per soldier IAOrdre 4, IACible,
  Deplacer to the target's centre (`Cible.PositionY + Cible.Largeur/2 - s.Hauteur/2`,
  sic: width used for Y); C comment 38/39.

Each `Commentaires.Message(...)` in these functions is evaluated (Rnd consumed) when the
command is built, even if the command is later dropped.

---------------------------------------------------------------------------------------------

## 11. Speech, chat and team changes

### 11.1 Comment lines (`ClsJeuCommentaires.cls`)

`Commentaires.Message(n, P)` (`:84-94`): if `n >= 0`: if `P.CommentairesSpecifique` (race
option, e.g. animals) return `Messages(P.CommentairesSpecifiqueIndice, Int(Rnd *
NombreMessages(that)))`, else `Messages(n, Int(Rnd * NombreMessages(n)))`; `n < 0` → `""`.
**One Rnd per call with n >= 0.** Message type `n` is section `[Commentaires_<n+1>]` of
`Donnees\<langue>\Commentaires.ini`. Types used in this area:

| n | French example (Message(1)) | used by |
|---|---|---|
| 6 | "Venez m'aider !" | caller, perso attacked |
| 7 | "Détuisons cette barraque." | caller, house attacked |
| 8 | "Mettons fin à l'existence de ce quartier général." | caller, castle attacked |
| 9 | "A l'aide !" | caller, generic |
| 10 | "On arrive." | helper |
| 16 | "Comme on se retrouve." | `IA_Poursuivre_Fuyard` |
| 21 | "Stop !" | `Ordonner_Arret` |
| 22 | "Tous en formation !" | `Ordonner_Formation` (chef not moving) |
| 23 | "En avant !" | formation while moving, `Ordonner_Chargez` follow, `Ordonner_Deplacement` |
| 25/26 | "Occupe-toi de lui !" / "Occupez-vous de lui !" | attack perso, 1 / n soldiers |
| 27/28 | "Allez, rase-moi ça." / "Allez, rasez-moi ça." | attack house |
| 29/30 | "Détruis-moi ce quartier général." / "Détruisez-…" | attack castle |
| 31 | "Oui chef !" | soldier acknowledging (10%) |
| 38/39 | "Protège cet individu." / "Protègez cet individu." | guard order |

Display duration and the generic "show a comment with probability 1%" logic of
`Deplacer_Persos` are in 03/05.

### 11.2 Speech from scripts

`Me.Dire_Commentaires = text` sets the bubble directly (§5.2).

### 11.3 Player chat line (`ClsInterface.Dialogue_Valider`, `ClsInterface.cls:2250-2280`)

The Dialogue key opens a text line; on validation `Message = DialogueTexte1 & DialogueTexte2`:
- `"_heure=<n>"` (case-insensitive prefix): set the clock `AffTemps.Heures = Val(n)`.
- `"_regner sans partage"`: `Cmd_Perso_Hostile`.
- `"_test"`: cheat: +10000 gold, all resources to max, fill the inventory with random
  non-secret objects (`j = CLng(Rnd * (NombreObjets - 1))` per slot, executed locally, not via
  the queue).
- anything else: `Cmd_Perso_EnvoyerCommentaire(i, Message & String(Len(Message), " "))` (the
  text padded with as many spaces, which lengthens its display time).
The last 10 lines are kept for recall. Network chat (`ClsComMessages.MessageChat`) only
exists in the pre-game lobby list.

### 11.4 Team changes

- `Perso_Hostile(P)` (`ModCombat.bas:875-917`): create a new fief (`Fief_Ajouter_Nouveau`)
  with `NumeroEquipe = max(existing teams) + 1`, same people type, age and age timer as P's
  fief, `Nom = P.Nom`, `NombreCitoyens = 1`, `NombreMorts = 0|1`; decrement the old fief's
  counters; move P and all its soldiers (`NumeroChef = P`) to it (soldiers' `IA_Ordre 4` →
  0); `P.Arreter`; clear `DansUneMaison/DansSonChateau/DansUnChateau`.
- `Attaquer_Allies(True)` (`ClsJeuPerso.cls:864-882`): `NumeroEquipe = -Numero` (perso 0
  gets team 0!), `Arreter`; its soldiers get the same team and `Arreter`.
  `Attaquer_Allies(False)` restores `Fiefs(NumeroFief).NumeroEquipe`. `Hostile` is
  `NumeroEquipe <= 0` (`ClsJeuPerso.cls:4471`); hostile persos cannot enter their castle.
- `Perso_Allie(a, b)` = same `NumeroEquipe` (`ModCombat.bas:919`).

---------------------------------------------------------------------------------------------

## 12. Network message layer (`ClsComMessages`, `ClsComMessage`, `ClsComPerso`, `ClsComFief`)

Only relevant for DirectPlay multiplayer (candidate for omission in the browser port,
06-pregame §2.1.7). Summary for completeness:

- `ClsComMessage` is a 6-field record (`Temp1, Temp2: Integer; Temp3, Temp4: Long; Temp5,
  Temp6: String`) = `(Categorie1, Categorie2, Indice1, Indice2, Var1, Var2)`.
- `ClsComMessages.Analyser(c1, c2, i1, i2, v1, v2, ChatSeulement)` (`:1495`) dispatches on
  `Categorie1`: 0 chat (lobby list), 1 info requests (server answers), 2 game state (mode,
  running), 3 world (dimensions, map, terrain), 4 resources, 5 perso fields (≈60 sub-types:
  name, fief, race, positions, bars, XP, appearance, IA flags incl. `ChoisirIA`+`FichierIA`
  (28) and `ChoisirIAScript`+`FichierIAScript` (29), soldiers, inventory, effects,
  comments…), 6 house, 7 castle, 8 fief, 9 treasure, 10 decor, 11 damage numbers,
  12 visual effects, **13 commands** → `Ajouter_Commande(c2, i1, v1, v2)` on the server.
- `ClsComPerso` / `ClsComFief` are client-side mirrors (plain fields) of the replicated
  state; they hold no logic.
- The server is authoritative: clients only send commands; all AI runs on the server.

---------------------------------------------------------------------------------------------

## 13. Rnd usage (this area)

Every call consumes exactly one value of the global VB `Rnd` stream unless noted. Because
`And`/`Or`/`IIf` do not short-circuit, the listed draws happen whenever the line is
evaluated.

| site | expression | when |
|---|---|---|
| `ModIA.bas:2861` | `AnglePatrouille(LPerso) = Rnd*6.28` | each `Nombre_IAs` assignment (game start, each new perso slot) |
| `ModIA.bas:116` | `IndiceObjet = CLng(Rnd * N)` | `IA_Init` |
| `ModIA.bas:121` | `AnglePatrouille(i) = Rnd*6.28` | `IA_Init`, i = 0..LPerso−1 |
| `ModIA.bas:306` | `Rnd > 0.99` → `Arreter` | monster turn while not idle |
| `ModIA.bas:373, 396, 891` | `Reparer = Rnd < 0.1` | every `IA_Aller_Maison(i, k≠i)`, market branch of `IA_Rentrer_Chateau`, caravan |
| `ModIA.bas:442-443, 458-459` | `Rnd*128-64`, `Rnd*96-48` | 2 per try of `IA_Deplacement_Aleatoire` (≤ 41 tries) |
| `ModIA.bas:517, 529` | `Select Case Rnd` | each `IA_Agressif` (×3 for Patrouiller and mercenaries) |
| `ModIA.bas:870` | `Rnd < 0.01` | caravan with damaged house |
| `ModIA.bas:883-891` | `Rnd < 0.005`; then `Rnd <= 1/(n+1)` (always); then `Int(Rnd*n)` and `Rnd < 0.1` in the market branch | caravan at home |
| `ModIA.bas:1624` | `10 * Rnd` | per missing building resource in `IA_Acheter_Ressources_Pour_Construire` |
| `ModIA.bas:1827` | `Int(Rnd*BatimentNiveauMax)+1` | `Choisir_Batiment` with Niveau = −1 |
| `ModIA.bas:1856` | `Int(Rnd*k)` | each `Choisir_Batiment` call that reaches the level pick |
| `ModIA.bas:1883` | `AffRessources.Largeur*Rnd*10` | `Trouver_Ressource_Proche` (unused) |
| `ModIA.bas:1912` | `AffTresor.Largeur*Rnd*10` | each `Trouver_Tresor_Proche` (from `IA_Recuperation_Carac`) |
| `ModIA.bas:2083-2108` | 2 per try (X, Y), +1 for the castle/market choice (always evaluated in the non-invocation branch), +1 for the market index when chosen, +1 per clamped coordinate | `Trouver_Emplacement_Maison` |
| `ModIA.bas:2132` | `Int(10*Rnd)` | buying in a foreign shop (`IA_Travail`) |
| `ModIA.bas:2137` | `Int(Rnd*nObj)` | buying in a foreign workshop |
| `ModIA.bas:2213` | `Int(Rnd*nObj)` repeated until allowed | `IA_Travailler_Maison` in a workshop |
| `ModIA2.bas:727` | `Int(Rnd*LPerso)` ×(LPerso+1) | each `IA_Donner` of a non-invocation |
| `ModDialogue.bas:66, 113` | `Rnd < 0.5` | per examined j in the "find the attacker" loops |
| `ModDialogue.bas:80` | `Rnd < 3/(citizens+1)` | per eligible helper |
| `ModDialogue.bas:216, 227, 238, 256` | `Rnd < 0.1` | per soldier ordered by `Ordonner_Chargez` |
| `ModDialogue.bas:516, 539, 567, 595` | `Rnd < 0.1` | per selected soldier in `Ordonner_*` |
| `ClsJeuCommentaires.cls:87, 89` | `Int(Rnd * NombreMessages)` | every `Commentaires.Message(n >= 0)` |
| `ModMain.bas:623, 626` | `NiveauIA > Rnd`; `Int(Rnd*ListCount)` | random IA assignment at game start |
| `ClsInterface.cls:2268` | `CLng(Rnd*(NombreObjets-1))` | `_test` cheat |
| `ModCombat.bas:278, 578-599` | triggers of `Appeler_De_lAide` | see 03 §6 |

---------------------------------------------------------------------------------------------

## 14. Quirks checklist (reproduce for identical gameplay)

1. Berserk persos (`IA_Berserk > 0`) do nothing in the round robin (only `IA_Equiper`);
   their behaviour runs every frame from `Deplacer_Persos`, even for the human player.
2. `IA_Init` skips `IA_InitPerso` for the last perso (`BAC = 0` etc.).
3. `IndiceObjet` is one cursor shared by everybody.
4. Search cursors (`IA_Attaquer_Perso/Maison`, `IA_Attaquer_Assaillant_Perso`) do not advance
   on success; `DistanceAdversaire` & co. start at 0 so nothing is found before the first wrap;
   `DistanceMaison` is never capped.
5. `IA_Trouver_Ressource` falls back to node 0 when no node of that appearance exists.
6. `IA_Donner` only gives to perso 0 (Boolean target variable).
7. `IA_Stock_Exedentaire` is always True.
8. `IA2_ChercherMatieresPremiere` checks the fief stock of the *house raw material* instead of
   the requested resource before going to the castle.
9. `Nombre_Batiments` counts invisible houses.
10. `.ia` parser ignores the last line; `Equiper_` header unused by the game.
11. `Trouver_Emplacement_Maison` uses `LargeurCase` for the low-Y clamp.
12. Command 49 (`AppelerDelAide`) would error and drop the queue (never emitted).
13. `Cmd_Perso_Definir_IA` is silently dropped for persos without `ChoisirIA`.
14. Any runtime error inside `Lire_Commandes` drops the remaining commands of the frame.

---------------------------------------------------------------------------------------------

## Appendix A. Unused / dead code in this area

`IA_Depenser_Argent`, `IA_Chercher_Ressource_Pour_Construire`, `IA_Couper_Arbre`,
`IA_ChercherPierre`, `Trouver_Ressource_Proche`, `Nombre_Total_Batiments`,
`NombreBatimentsDeNiveau`, `IA_Argent_Riche` (only used by `IA_Depenser_Argent`),
`IA_Equiper = 3` ("intelligent", empty), `IA2.Equiper`, command constants 5, 8, 28, 39, 41,
43, 54 (no handlers), `IAs\Empereur\Mercenaire imperial.ia`, `IAs\Monstres\Archer elfe.ia`.
`ObstacleMort`, `ObstacleAmi`, `ObstacleEnnemi`, `ObstacleChateau` (`ModIA.bas:2580-2845`)
are used by the UI/movement code (05, 03), not by the AI.

## Appendix B. All shipped `.ia` files (compact)

Format: `Age_N:` followed by the steps in file order. `ChangerMode_M` meanings in §4.1.

```
#### Alchimiste.ia
desc: IA officielle. C'est le villageois de base qui se contente de fabriquer des élixirs.
header: Equiper_0
Age_1: ChangerMode_1 Ressource_1=20 Ressource_18=20 Ressource_3=5 GagnerArgent_7=110 Objet_101=1 Batiment_7 Travailler
Age_2: ChangerMode_1 Ressource_1=20 Ressource_18=20 GagnerArgent_7=220 Objet_3=1 Batiment_7 Travailler
Age_3: ChangerMode_1 Ressource_1=20 GagnerArgent_28=550 Objet_231=1 Ressource_3=16 Batiment_28 Travailler
Age_4: ChangerMode_1 GagnerArgent_41=800 Objet_97=1 Batiment_41 Travailler
Age_5: ChangerMode_1 GagnerArgent_41=800 Objet_97=1 Objet_66=1 Batiment_41 Travailler
#### Arbaletrier offensif.ia
desc: IA officielle. Un arbalètrier qui produit des armes à distance et s'équipe avec ses meilleurs arbalètes.
header: Equiper_0
Age_1: ChangerMode_1 Ressource_1=3 Objet_75=1 Objet_38=1 Ressource_13=8 Objet_10=1 ChangerMode_2 Patrouiller
Age_2: ChangerMode_1 Ressource_3=3 Objet_19=1 ChangerMode_2 Objet_75=1 Objet_99=1 Objet_38=1 Ressource_1=3 Ressource_2=3 Batiment_19 Travailler
Age_3: ChangerMode_1 Ressource_11=1 Ressource_3=3 Objet_50=1 ChangerMode_2 Objet_75=1 Objet_47=1 Objet_99=1 Objet_38=1 Ressource_7=3 Ressource_9=3 Batiment_19 Travailler
Age_4: ChangerMode_1 Ressource_11=1 Ressource_3=3 Objet_50=1 ChangerMode_2 Objet_75=1 Objet_47=1 Objet_99=1 Objet_38=1 Ressource_7=3 Ressource_9=3 Batiment_19 Travailler
Age_5: ChangerMode_1 Ressource_11=1 Ressource_3=3 Objet_209=1 ChangerMode_2 Objet_75=1 Objet_47=1 Objet_99=1 Objet_38=1 Ressource_7=3 Ressource_9=3 Batiment_19 Travailler
#### Archer.ia
desc: IA officielle. Un archer qui produit des armes à distance et s'équipe avec ses meilleurs arcs.
header: Equiper_0
Age_1: ChangerMode_1 Objet_10=1 GagnerArgent_11=480 Ressource_2=1 ChangerMode_2 Batiment_11 Ressource_13=8 Travailler
Age_2: ChangerMode_1 Ressource_2=1 Objet_18=1 ChangerMode_2 Batiment_22 Travailler
Age_3: ChangerMode_1 Ressource_2=1 GagnerArgent_22=220 Objet_45=1 Objet_49=1 ChangerMode_2 Batiment_22 Travailler
Age_4: ChangerMode_1 Ressource_2=1 GagnerArgent_22=480 Objet_45=1 Objet_75=1 Objet_49=1 ChangerMode_2 Batiment_22 Travailler
Age_5: ChangerMode_1 Ressource_2=1 GagnerArgent_22=480 Objet_45=1 Objet_75=1 Objet_208=1 ChangerMode_2 Batiment_22 Travailler
#### Armurier.ia
desc: IA officielle. Un armurier qui produit des armures et s'équipe avec ce qu'il fait de mieux.
header: Equiper_0
Age_1: ChangerMode_1 Ressource_15=20 Ressource_13=9 Batiment_11 Travailler
Age_2: ChangerMode_1 Objet_47=1 GagnerArgent_23=960 Objet_20=1 Batiment_23 Travailler
Age_3: ChangerMode_1 Ressource_11=1 GagnerArgent_23=300 Objet_14=1 ChangerMode_2 GagnerArgent_23=380 Objet_24=1 Batiment_23 Travailler
Age_4: ChangerMode_1 Ressource_11=1 GagnerArgent_23=600 Objet_104=1 ChangerMode_2 GagnerArgent_23=1920 Objet_108=1 Batiment_23 Travailler
Age_5: ChangerMode_1 Ressource_11=1 GagnerArgent_23=600 Objet_205=1 ChangerMode_2 GagnerArgent_23=1920 Objet_108=1 Batiment_23 Travailler
#### Artilleur.ia
desc: IA officielle. Un archer qui produit des armes à distance et s'équipe avec ses meilleurs armes à feu.
header: Equiper_0
Age_1: ChangerMode_1 Ressource_26=3 Objet_38=1 Batiment_11 Ressource_13=8 Travailler
Age_2: ChangerMode_1 Ressource_26=3 Objet_99=1 Batiment_22 Travailler
Age_3: ChangerMode_1 Ressource_26=3 Objet_76=1 Batiment_22 Travailler
Age_4: ChangerMode_1 Ressource_26=3 Objet_94=1 ChangerMode_2 Objet_216=1 Batiment_37 Travailler
Age_5: ChangerMode_1 Ressource_26=3 Objet_94=1 ChangerMode_2 Objet_216=1 Batiment_37 Travailler
#### Artisan.ia
desc: IA officielle. Un artisan qui produit différents outils.
header: Equiper_0
Age_1: ChangerMode_1 Batiment_10 Travailler
Age_2: ChangerMode_1 GagnerArgent_10=300 Objet_1=1 Batiment_10 Travailler
Age_3: ChangerMode_1 Ressource_11=1 GagnerArgent_10=300 Objet_1=1 Batiment_10 Travailler
Age_4: ChangerMode_1 Ressource_11=1 GagnerArgent_10=300 Objet_1=1 Batiment_10 Travailler
Age_5: ChangerMode_1 Ressource_11=1 GagnerArgent_10=300 Objet_1=1 Objet_228=1 Batiment_10 Travailler
#### Assassin offensif.ia
desc: IA officielle. Un assassin spécialisé dans les armes qui infliquent d'importants coups critiques.
header: Equiper_0
Age_1: ChangerMode_1 Objet_45=1 Objet_38=1 Ressource_12=5 Ressource_13=8 Ressource_15=4 Ressource_1=3 Batiment_6 Travailler
Age_2: ChangerMode_1 Ressource_11=1 Objet_45=1 Objet_13=1 ChangerMode_2 Objet_99=1 Ressource_1=3 Batiment_19 Travailler
Age_3: ChangerMode_1 Ressource_11=1 Objet_45=1 Objet_60=1 ChangerMode_2 Objet_99=1 Ressource_1=3 Batiment_19 Travailler
Age_4: ChangerMode_1 Ressource_11=1 Objet_45=1 Objet_60=1 ChangerMode_2 Objet_99=1 Ressource_1=3 Batiment_19 Travailler
Age_5: ChangerMode_1 Ressource_11=1 Objet_45=1 Objet_201=1 Objet_60=1 ChangerMode_2 Objet_99=1 Ressource_1=3 Batiment_19 Travailler
#### Barbare offensif.ia
desc: IA officielle. Vue et approuvée par Crom.
header: Equiper_0
Age_1: ChangerMode_2 Objet_8=1 Attaquer Ennemi
Age_2: ChangerMode_2 Objet_39=1 Ressource_7=1 Attaquer Ennemi
Age_3: ChangerMode_2 Ressource_11=1 Objet_16=1 Objet_24=1 Attaquer Ennemi
Age_4: ChangerMode_2 Ressource_11=1 Objet_106=1 Objet_24=1 Attaquer Ennemi
Age_5: ChangerMode_2 Ressource_11=1 Objet_211=1 Objet_24=1 Attaquer Ennemi
#### Berger.ia
desc: IA officielle. C'est le villageois de base qui se contente de fabriquer de la laine.
header: Equiper_0
Age_1: ChangerMode_1 GagnerArgent_4=76 Objet_51=1 Batiment_4 Travailler
Age_2: ChangerMode_1 GagnerArgent_4=330 Objet_109=1 Batiment_4 Travailler
Age_3: ChangerMode_1 GagnerArgent_4=330 Objet_109=1 Batiment_4 Travailler
Age_4: ChangerMode_1 GagnerArgent_4=330 Objet_109=1 Batiment_4 Travailler
Age_5: ChangerMode_1 GagnerArgent_4=330 Objet_109=1 Batiment_4 Travailler
#### Bijoutier.ia
desc: IA officielle. C'est le villageois de base qui se contente de fabriquer des bijoux.
header: Equiper_0
Age_1: ChangerMode_1 Ressource_24=20 Objet_2=1 Objet_3=1 Batiment_9 Travailler
Age_2: ChangerMode_1 Objet_7=1 Ressource_24=20 Batiment_20 Travailler
Age_3: ChangerMode_1 Objet_7=1 Ressource_24=20 Batiment_24 Travailler
Age_4: ChangerMode_1 Objet_7=1 Batiment_36 Travailler
Age_5: ChangerMode_1 Objet_7=1 Batiment_36 Travailler
#### Boucher.ia
desc: IA officielle. C'est le villageois de base qui se contente de fabriquer de la viande.
header: Equiper_0
Age_1: ChangerMode_1 Batiment_6 Travailler
Age_2: ChangerMode_1 GagnerArgent_6=168 Objet_65=2 Batiment_6 Travailler
Age_3: ChangerMode_1 GagnerArgent_6=252 Objet_65=2 Objet_51=1 Batiment_6 Travailler
Age_4: ChangerMode_1 GagnerArgent_6=336 Objet_65=2 GagnerArgent_6=300 Objet_1=1 Batiment_6 Travailler
Age_5: ChangerMode_1 GagnerArgent_6=336 Objet_65=2 GagnerArgent_6=300 Objet_1=1 Batiment_6 Travailler
#### Boulanger.ia
desc: IA officielle. C'est le villageois de base qui se contente de fabriquer du pain.
header: Equiper_0
Age_1: ChangerMode_1 Batiment_7 Travailler
Age_2: ChangerMode_1 GagnerArgent_7=220 Objet_45=1 Batiment_7 Travailler
Age_3: ChangerMode_1 GagnerArgent_7=220 Objet_45=1 Objet_51=1 Batiment_7 Travailler
Age_4: ChangerMode_1 GagnerArgent_7=880 Objet_45=1 Objet_1=1 Batiment_7 Travailler
Age_5: ChangerMode_1 GagnerArgent_7=880 Objet_45=1 Objet_1=1 Batiment_7 Travailler
#### Brise-mur offensif.ia
desc: IA officielle. Un guerrier spécialisé dans la destruction de bâtiments.
header: Equiper_0
Age_1: ChangerMode_1 Objet_8=2 ChangerMode_2 Ressource_1=3 Attaquer Donjon
Age_2: ChangerMode_1 Ressource_11=1 Objet_39=2 ChangerMode_2 Ressource_1=3 Attaquer Donjon
Age_3: ChangerMode_1 Ressource_11=1 Objet_30=1 ChangerMode_2 Ressource_20=1 Attaquer Donjon
Age_4: ChangerMode_1 Ressource_11=1 Objet_32=1 ChangerMode_2 Ressource_20=1 Objet_102=1 Attaquer Donjon
Age_5: ChangerMode_1 Ressource_11=1 Objet_210=2 ChangerMode_2 Ressource_20=1 Objet_102=1 Attaquer Donjon
#### Bucheron.ia
desc: IA officielle. C'est le villageois de base qui se contente de fabriquer des planches.
header: Equiper_0
Age_1: ChangerMode_1 GagnerArgent_3=120 Objet_4=1 Batiment_3 Travailler
Age_2: ChangerMode_1 GagnerArgent_3=120 Objet_4=1 Batiment_3 Travailler
Age_3: ChangerMode_1 GagnerArgent_3=440 Objet_40=1 ChangerMode_2 Batiment_3 Travailler
Age_4: ChangerMode_1 GagnerArgent_3=880 Objet_105=1 ChangerMode_2 Batiment_3 Travailler
Age_5: ChangerMode_1 GagnerArgent_3=2494 Objet_212=1 ChangerMode_2 Batiment_3 Travailler
#### Cavalier.ia
desc: IA officielle. C'est le villageois de base élève des chevaux.
header: Equiper_0
Age_1: ChangerMode_1 Ressource_8=20 Ressource_12=10 Ressource_15=4 Ressource_13=8 Ressource_15=20 Ressource_24=20 Batiment_12 Travailler
Age_2: ChangerMode_1 Batiment_17 Travailler
Age_3: ChangerMode_1 Batiment_17 Travailler
Age_4: ChangerMode_1 Batiment_17 Travailler
Age_5: ChangerMode_1 Batiment_17 Travailler
#### Chevalier de l'ordre.ia
desc: IA officielle. Un chevalier de l'ordre qui s'équipe avec ce qu'il fait de mieux.
header: Equiper_0
Age_1: ChangerMode_1 Ressource_15=20 Objet_74=1 Batiment_11 Travailler
Age_2: ChangerMode_1 Objet_47=1 Ressource_2=1 Batiment_23 Travailler
Age_3: ChangerMode_1 Objet_171=1 Objet_25=1 Batiment_23 Travailler
Age_4: ChangerMode_1 Objet_85=1 Ressource_20=1 ChangerMode_2 Objet_171=1 Objet_84=1 Batiment_40 Travailler
Age_5: ChangerMode_1 Objet_85=1 Ressource_20=1 ChangerMode_2 Objet_171=1 Objet_84=1 Batiment_40 Travailler
#### Chevalier du chaos.ia
desc: IA officielle. Un chevalier du chaos qui s'équipe avec ce qu'il fait de mieux.
header: Equiper_0
Age_1: ChangerMode_1 Ressource_15=20 Ressource_24=20 Batiment_11 Travailler
Age_2: ChangerMode_1 Objet_7=1 Ressource_24=20 Ressource_2=1 Batiment_23 Travailler
Age_3: ChangerMode_1 Ressource_24=20 GagnerArgent_24=640 Objet_171=1 Objet_25=1 Batiment_23 Travailler
Age_4: ChangerMode_1 Objet_106=1 ChangerMode_2 Objet_171=1 Objet_58=1 Batiment_39 Travailler
Age_5: ChangerMode_1 Objet_106=1 ChangerMode_2 Objet_171=1 Objet_58=1 Batiment_39 Travailler
#### Chevalier offensif.ia
desc: IA officielle. Le chevalier combat toujours au contact et porte toujours les armures les plus lourdes.
header: Equiper_0
Age_1: ChangerMode_1 Objet_11=1 Objet_8=2 ChangerMode_2 Ressource_1=3 Patrouiller
Age_2: ChangerMode_1 Objet_39=1 Objet_74=1 Objet_20=1 ChangerMode_2 Ressource_1=3 Ressource_7=3 Batiment_19 Travailler
Age_3: ChangerMode_1 Ressource_11=1 Objet_39=1 Objet_171=1 Objet_23=1 Objet_25=1 Objet_21=1 ChangerMode_2 Ressource_19=3 Batiment_19 Travailler
Age_4: ChangerMode_1 Ressource_11=1 Objet_103=1 Objet_171=1 Objet_23=1 Objet_25=1 Objet_21=1 ChangerMode_2 Ressource_19=3 Objet_69=2 Batiment_19 Travailler
Age_5: ChangerMode_1 Ressource_11=1 Objet_103=1 Objet_171=1 Objet_23=1 Objet_25=1 Objet_21=1 ChangerMode_2 Ressource_19=3 Objet_69=2 Batiment_19 Travailler
#### Cidrier.ia
desc: IA officielle. C'est le villageois de base qui se contente de fabriquer du cidre.
header: Equiper_0
Age_1: ChangerMode_1 GagnerArgent_8=100 Ressource_9=1 Batiment_8 Travailler
Age_2: ChangerMode_1 GagnerArgent_8=220 Objet_3=1 Ressource_3=6 Ressource_14=7 Batiment_8 Travailler
Age_3: ChangerMode_1 GagnerArgent_8=560 Objet_2=1 Objet_3=1 Ressource_3=6 Ressource_14=7 Batiment_8 Travailler
Age_4: ChangerMode_1 GagnerArgent_35=560 Objet_2=1 Objet_3=1 Objet_46=1 Batiment_35 Travailler
Age_5: ChangerMode_1 GagnerArgent_35=560 Objet_2=1 Objet_3=1 Objet_46=1 Batiment_35 Travailler
#### Clerc.ia
desc: IA officielle. C'est un villageois défensif qui réssuscite ses alliés.
header: Equiper_0
Age_1: ChangerMode_1 GagnerArgent_9=146 Objet_9=1 Ressource_3=8 Ressource_13=8 Batiment_9 Travailler
Age_2: ChangerMode_1 GagnerArgent_9=146 Objet_9=1 Ressource_3=11 Ressource_13=9 Batiment_9 Travailler
Age_3: ChangerMode_1 Ressource_11=1 Objet_42=1 Batiment_32 Travailler
Age_4: ChangerMode_1 Objet_27=1 Ressource_23=1 Objet_43=1 Batiment_32 Travailler
Age_5: ChangerMode_1 Objet_27=1 Ressource_23=1 Objet_43=1 Batiment_32 Travailler
#### Commandant offensif.ia
desc: IA officielle. Un commandant qui envoie des soldats à l'attaque.
header: Equiper_0
Age_1: ChangerMode_1 Objet_231=1 Ressource_24=20 Ressource_15=20 Objet_74=1 Ressource_1=3 Batiment_14 Travailler
Age_2: ChangerMode_1 Ressource_24=20 Objet_231=1 Objet_74=1 Ressource_1=3 Batiment_23 Travailler
Age_3: ChangerMode_1 Objet_16=1 Objet_171=1 Objet_21=1 Objet_25=1 Ressource_24=20 ChangerMode_2 Batiment_19 Travailler
Age_4: ChangerMode_1 Ressource_23=1 Objet_16=1 Objet_171=1 Objet_21=1 Objet_25=1 Objet_87=2 ChangerMode_2 Objet_83=4 Attaquer Donjon
Age_5: ChangerMode_1 Ressource_23=1 Objet_16=1 Objet_171=1 Objet_21=1 Objet_25=1 Objet_87=2 ChangerMode_2 Objet_83=4 Attaquer Donjon
#### Cordonnier.ia
desc: IA officielle. C'est le villageois de base qui se contente de fabriquer des chaussures.
header: Equiper_0
Age_1: ChangerMode_1 Objet_46=1 Batiment_13 Travailler
Age_2: ChangerMode_1 GagnerArgent_13=220 Objet_3=1 Objet_46=1 Batiment_13 Travailler
Age_3: ChangerMode_1 GagnerArgent_13=560 Objet_2=1 Objet_46=1 Batiment_13 Travailler
Age_4: ChangerMode_1 GagnerArgent_13=560 Objet_2=1 Objet_3=1 Objet_46=1 Batiment_13 Travailler
Age_5: ChangerMode_1 GagnerArgent_13=560 Objet_2=1 Objet_3=1 Objet_46=1 Batiment_13 Travailler
#### Defenseur.ia
desc: IA officielle. C'est le villageois qui construit des marchés.
header: Equiper_0
Age_1: Ressource_13=20 Batiment_11 Travailler
Age_2: Objet_18=1 ChangerMode_2 GagnerArgent_23=1200 Batiment_TOURDEFENSE Patrouiller
Age_3: Objet_18=1 ChangerMode_2 GagnerArgent_23=1200 Batiment_TOURDEFENSE Patrouiller
Age_4: Objet_18=1 ChangerMode_2 GagnerArgent_23=1200 Batiment_TOURDEFENSE Patrouiller
Age_5: Objet_18=1 ChangerMode_2 GagnerArgent_23=1200 Batiment_TOURDEFENSE Patrouiller
#### Elementaliste offensif.ia
desc: IA officielle. Un élémentaliste offensif qui attaque à coups d'élémentaux.
header: Equiper_0
Age_1: ChangerMode_1 Ressource_5=20 Ressource_18=20 Ressource_6=20 Objet_45=1 Batiment_3 Travailler
Age_2: ChangerMode_1 Ressource_5=20 Ressource_18=20 Ressource_6=20 Objet_231=1 Batiment_3 Travailler
Age_3: ChangerMode_1 Objet_31=1 Ressource_20=1 ChangerMode_2 Ressource_11=1 Objet_89=1 Batiment_19 Travailler
Age_4: ChangerMode_1 Objet_31=1 Ressource_20=1 ChangerMode_2 Ressource_23=1 Objet_89=1 Objet_215=1 Objet_147=1 Objet_33=3 ChangerMode_7 Batiment_19 Attaquer Donjon
Age_5: ChangerMode_1 Objet_31=1 Ressource_20=1 Ressource_23=1 Objet_89=1 Objet_215=1 ChangerMode_2 Objet_188=1 Objet_147=1 Objet_203=1 Batiment_19 Attaquer Donjon
#### Eleveur.ia
desc: IA officielle. C'est le villageois de base qui se contente de fabriquer du bétail.
header: Equiper_0
Age_1: ChangerMode_1 Objet_9=1 Batiment_5 Travailler
Age_2: ChangerMode_1 GagnerArgent_5=220 Objet_9=1 Objet_45=1 Batiment_5 Travailler
Age_3: ChangerMode_1 GagnerArgent_5=146 Objet_9=1 Objet_51=1 Batiment_5 Travailler
Age_4: ChangerMode_1 GagnerArgent_5=146 Objet_9=1 GagnerArgent_5=300 Objet_1=1 Batiment_5 Travailler
Age_5: ChangerMode_1 GagnerArgent_5=146 Objet_9=1 GagnerArgent_5=300 Objet_1=1 Batiment_5 Travailler
#### Etheraliste.ia
desc: IA officielle. C'est le villageois de base qui se contente de fabriquer de l'ether.
header: Equiper_0
Age_1: ChangerMode_1 Objet_231=1 Ressource_18=20 Ressource_3=10 Batiment_8 Travailler
Age_2: ChangerMode_1 Objet_46=1 Ressource_18=20 Batiment_18 Travailler
Age_3: ChangerMode_1 GagnerArgent_28=404 Objet_164=1 Batiment_28 Travailler
Age_4: ChangerMode_1 Objet_164=1 GagnerArgent_28=800 Objet_97=1 Batiment_28 Travailler
Age_5: ChangerMode_1 Objet_164=1 GagnerArgent_28=800 Objet_97=1 Batiment_28 Travailler
#### Fermier d'orge.ia
desc: IA officielle. C'est le villageois de base qui se contente de fabriquer de l'orge.
header: Equiper_0
Age_1: ChangerMode_1 Batiment_2 Ressource_13=6 Travailler
Age_2: ChangerMode_1 GagnerArgent_2=100 Objet_6=1 Batiment_2 Travailler
Age_3: ChangerMode_1 GagnerArgent_2=100 Objet_6=1 GagnerArgent_2=300 Objet_1=1 Batiment_2 Travailler
Age_4: ChangerMode_1 GagnerArgent_2=300 Objet_6=1 Objet_1=1 Objet_51=1 Batiment_2 Travailler
Age_5: ChangerMode_1 GagnerArgent_2=300 Objet_6=1 Objet_1=1 Objet_51=1 Batiment_2 Travailler
#### Fermier de ble.ia
desc: IA officielle. C'est le villageois de base qui se contente de fabriquer du blé.
header: Equiper_0
Age_1: ChangerMode_1 GagnerArgent_1=100 Objet_6=1 Batiment_1 Ressource_13=5 Travailler
Age_2: ChangerMode_1 GagnerArgent_1=100 Objet_6=1 Batiment_1 Travailler
Age_3: ChangerMode_1 GagnerArgent_1=100 Objet_6=1 Batiment_1 Travailler
Age_4: ChangerMode_1 GagnerArgent_1=100 Objet_6=1 Batiment_1 Travailler
Age_5: ChangerMode_1 GagnerArgent_1=100 Objet_6=1 Batiment_1 Travailler
#### Fondeur d'acier.ia
desc: IA officielle. C'est le villageois de base qui se contente de fabriquer d'acier.
header: Equiper_0
Age_1: ChangerMode_1 Ressource_15=20 Batiment_10 Ressource_13=6 Travailler
Age_2: ChangerMode_1 GagnerArgent_20=190 Objet_7=1 Batiment_20 Ressource_13=7 Travailler
Age_3: ChangerMode_1 GagnerArgent_24=190 Objet_7=1 GagnerArgent_24=550 Objet_231=1 Batiment_24 Travailler
Age_4: ChangerMode_1 GagnerArgent_24=190 Objet_7=1 GagnerArgent_24=550 Objet_66=1 Batiment_24 Travailler
Age_5: ChangerMode_1 GagnerArgent_24=190 Objet_7=1 GagnerArgent_24=550 Objet_66=1 Batiment_24 Travailler
#### Fondeur d'or.ia
desc: IA officielle. C'est le villageois de base qui se contente de fabriquer des lingots d'or.
header: Equiper_0
Age_1: ChangerMode_1 Ressource_24=20 Batiment_10 Ressource_13=6 Travailler
Age_2: ChangerMode_1 GagnerArgent_20=190 Objet_7=1 Ressource_24=20 Batiment_20 Ressource_13=7 Travailler
Age_3: ChangerMode_1 GagnerArgent_20=190 Objet_7=1 Ressource_24=20 Ressource_13=7 GagnerArgent_20=550 Objet_231=1 Batiment_20 Travailler
Age_4: ChangerMode_1 GagnerArgent_34=190 Objet_7=1 Batiment_34 Travailler
Age_5: ChangerMode_1 GagnerArgent_34=190 Objet_7=1 Batiment_34 Travailler
#### Fondeur de fer.ia
desc: IA officielle. C'est le villageois de base qui se contente de fabriquer de la fonte.
header: Equiper_0
Age_1: ChangerMode_1 Ressource_15=1 Batiment_10 Ressource_13=6 Travailler
Age_2: ChangerMode_1 GagnerArgent_20=190 Objet_7=1 Batiment_20 Travailler
Age_3: ChangerMode_1 GagnerArgent_20=190 Objet_7=1 Objet_1=1 Batiment_20 Travailler
Age_4: ChangerMode_1 GagnerArgent_20=190 Objet_7=1 GagnerArgent_20=220 Objet_45=1 Batiment_20 Travailler
Age_5: ChangerMode_1 GagnerArgent_20=190 Objet_7=1 GagnerArgent_20=220 Objet_45=1 Batiment_20 Travailler
#### Forgeron.ia
desc: IA officielle. Un forgeron qui produit des armes de contact et s'équipe avec ses meilleurs armes.
header: Equiper_0
Age_1: ChangerMode_1 Ressource_15=20 Ressource_1=1 Batiment_11 Ressource_13=7 Travailler
Age_2: ChangerMode_1 Batiment_21 Objet_14=2 Ressource_1=1 ChangerMode_2 Travailler
Age_3: ChangerMode_1 Batiment_21 Objet_14=2 Ressource_1=1 ChangerMode_2 Travailler
Age_4: ChangerMode_1 Batiment_21 Objet_14=2 Ressource_1=1 ChangerMode_2 Travailler
Age_5: ChangerMode_1 Batiment_21 Objet_14=2 Ressource_1=1 ChangerMode_2 Travailler
#### Gantier.ia
desc: IA officielle. C'est le villageois de base qui se contente de fabriquer des gants.
header: Equiper_0
Age_1: ChangerMode_1 Objet_73=1 Batiment_14 Travailler
Age_2: ChangerMode_1 Objet_73=1 GagnerArgent_14=110 Objet_3=1 Batiment_14 Travailler
Age_3: ChangerMode_1 Objet_73=1 GagnerArgent_14=280 Objet_2=1 Batiment_14 Travailler
Age_4: ChangerMode_1 Objet_73=1 GagnerArgent_14=110 Objet_3=1 GagnerArgent_14=320 Objet_46=1 Batiment_14 Travailler
Age_5: ChangerMode_1 Objet_73=1 GagnerArgent_14=110 Objet_3=1 GagnerArgent_14=320 Objet_46=1 Batiment_14 Travailler
#### Guerisseur.ia
desc: IA officielle. Un guerisseur spécialisé dans les ressources qui rendent la vie.
header: Equiper_0
Age_1: ChangerMode_1 Ressource_1=20 GagnerArgent_6=110 Objet_101=1 GagnerArgent_6=550 Objet_231=1 Batiment_6 Travailler
Age_2: ChangerMode_1 Ressource_1=20 GagnerArgent_6=110 Objet_101=1 GagnerArgent_6=550 Objet_231=1 Batiment_6 Travailler
Age_3: ChangerMode_1 GagnerArgent_27=110 Objet_101=1 GagnerArgent_27=900 Objet_42=1 Objet_27=1 Objet_29=1 Batiment_27 Travailler
Age_4: ChangerMode_1 GagnerArgent_27=110 Objet_101=1 GagnerArgent_27=900 Objet_157=1 Objet_42=1 Objet_27=1 Objet_215=1 Batiment_27 Travailler
Age_5: ChangerMode_1 GagnerArgent_27=110 Objet_101=1 GagnerArgent_27=900 Objet_157=1 Objet_42=1 Objet_27=1 Objet_215=1 Batiment_27 Travailler
#### Inquisiteur offensif.ia
desc: IA officielle. Un inquisiteur offensif.
header: Equiper_0
Age_1: ChangerMode_1 Objet_231=1 Ressource_13=15 Batiment_6 Travailler
Age_2: ChangerMode_1 Objet_76=1 Ressource_15=15 Batiment_18 Travailler
Age_3: ChangerMode_1 Ressource_11=1 Objet_16=1 Ressource_1=3 ChangerMode_2 Objet_171=1 Objet_76=1 Objet_48=1 Batiment_23 Travailler
Age_4: ChangerMode_1 Ressource_23=1 Ressource_19=2 Objet_85=1 ChangerMode_2 Ressource_20=2 Objet_215=1 Objet_84=1 Objet_171=1 Objet_83=2 ChangerMode_7 Batiment_40 Travailler
Age_5: ChangerMode_1 Ressource_23=1 Objet_86=1 Ressource_20=2 Ressource_19=2 Objet_84=1 Objet_215=1 Objet_171=1 ChangerMode_2 Objet_229=1 Objet_83=4 Batiment_19 Attaquer Donjon
#### Magicien offensif.ia
desc: IA officielle. Un mage offensif qui attaque à coups de golems.
header: Equiper_0
Age_1: ChangerMode_1 Ressource_13=20 Batiment_4 Travailler
Age_2: ChangerMode_1 Batiment_18 Travailler
Age_3: ChangerMode_1 Objet_31=1 ChangerMode_2 Ressource_20=1 Objet_44=1 ChangerMode_7 Attaquer Ennemi
Age_4: ChangerMode_1 Objet_31=1 ChangerMode_2 Objet_215=1 Ressource_20=1 Objet_193=1 ChangerMode_7 Batiment_38 Travailler
Age_5: ChangerMode_1 Objet_31=1 Objet_215=1 Ressource_20=1 Objet_217=1 ChangerMode_2 Objet_190=1 Objet_44=4 Attaquer Donjon
#### Magicien.ia
desc: IA officielle. Un mage qui travaille à sa guilde.
header: Equiper_0
Age_1: ChangerMode_1 GagnerArgent_4=220 Objet_45=1 Batiment_4 Travailler
Age_2: ChangerMode_1 GagnerArgent_4=220 Objet_45=1 Batiment_4 Travailler
Age_3: ChangerMode_1 GagnerArgent_29=720 Objet_31=1 Objet_44=1 ChangerMode_2 Batiment_29 Travailler
Age_4: ChangerMode_1 GagnerArgent_29=720 Objet_31=1 Objet_89=1 Objet_44=1 ChangerMode_2 Batiment_29 Travailler
Age_5: ChangerMode_1 GagnerArgent_29=720 Objet_31=1 Objet_89=1 Objet_44=1 ChangerMode_2 Batiment_29 Travailler
#### Mercenaire.ia
desc: IA officielle. Un soldat équipé moyennement qui protège le village.
header: Equiper_0
Age_1: ChangerMode_1 Objet_8=1 ChangerMode_2 Ressource_13=20 Objet_11=1 Ressource_1=3 Patrouiller
Age_2: ChangerMode_1 Objet_14=1 ChangerMode_2 Objet_20=1 Objet_74=1 Ressource_7=3 Batiment_19 Travailler
Age_3: ChangerMode_1 Objet_14=1 ChangerMode_2 Objet_20=1 Objet_47=1 Ressource_7=3 Batiment_19 Travailler
Age_4: ChangerMode_1 Objet_104=1 ChangerMode_2 Objet_47=1 Objet_108=1 Objet_24=1 Ressource_7=3 Batiment_19 Travailler
Age_5: ChangerMode_1 Objet_205=1 ChangerMode_2 Objet_47=1 Objet_108=1 Objet_24=1 Ressource_7=3 Batiment_19 Travailler
#### Milicien.ia
desc: IA officielle. Un soldat équipé moyennement qui protège le village.
header: Equiper_0
Age_1: ChangerMode_1 Objet_9=1 ChangerMode_2 Objet_38=1 Objet_11=1 Ressource_1=3 Ressource_13=4 Patrouiller
Age_2: ChangerMode_1 Objet_15=1 ChangerMode_2 Objet_74=1 Objet_20=1 Ressource_7=3 Batiment_19 Travailler
Age_3: ChangerMode_1 Objet_15=1 ChangerMode_2 Objet_47=1 Objet_20=1 Ressource_7=3 Batiment_19 Travailler
Age_4: ChangerMode_1 Objet_15=1 Objet_47=1 Objet_108=1 ChangerMode_2 Objet_69=1 Ressource_7=3 Batiment_19 Travailler
Age_5: ChangerMode_1 Objet_222=1 ChangerMode_2 Objet_47=1 Objet_108=1 Objet_100=1 Ressource_7=3 Batiment_19 Travailler
#### Mineur.ia
desc: IA officielle. C'est le villageois de base qui se contente de fabriquer des briques.
header: Equiper_0
Age_1: ChangerMode_1 Ressource_13=20 GagnerArgent_3=110 Objet_51=1 GagnerArgent_3=150 Objet_1=1 GagnerArgent_3=550 Objet_231=1 Batiment_3 Travailler
Age_2: ChangerMode_1 GagnerArgent_18=190 Objet_7=1 Objet_51=1 Objet_1=1 GagnerArgent_18=550 Objet_231=1 Batiment_18 Travailler
Age_3: ChangerMode_1 GagnerArgent_18=190 Objet_7=1 Objet_51=1 Objet_1=1 GagnerArgent_18=550 Objet_231=1 Batiment_18 Travailler
Age_4: ChangerMode_1 GagnerArgent_18=190 Objet_7=1 Objet_51=1 Objet_1=1 GagnerArgent_18=550 Objet_66=1 Batiment_18 Travailler
Age_5: ChangerMode_1 GagnerArgent_18=190 Objet_7=1 Objet_51=1 Objet_1=1 GagnerArgent_18=550 Objet_66=1 GagnerArgent_18=1350 Objet_70=1 Batiment_18 Travailler
#### Necromancien offensif.ia
desc: IA officielle. Un nécromancien offensif.
header: Equiper_0
Age_1: ChangerMode_1 Ressource_7=1 Batiment_3 Travailler
Age_2: ChangerMode_1 Ressource_7=1 Ressource_13=10 Batiment_3 Travailler
Age_3: ChangerMode_1 Ressource_11=1 Ressource_1=3 Objet_28=1 Objet_35=2 ChangerMode_7 Batiment_19 Attaquer Donjon
Age_4: ChangerMode_1 Ressource_23=1 Ressource_19=2 Objet_28=1 Objet_215=1 Objet_213=1 Ressource_20=2 ChangerMode_2 Objet_35=2 Objet_192=2 Batiment_19 Attaquer Donjon
Age_5: ChangerMode_1 Ressource_23=1 Ressource_19=2 Objet_28=1 Objet_215=1 Objet_213=1 Ressource_20=2 Objet_67=2 ChangerMode_2 Objet_189=1 Objet_191=2 Objet_192=2 Objet_204=1 Batiment_19 Attaquer Donjon
#### Necromancien.ia
desc: IA officielle. Un nécromancien défensif.
header: Equiper_0
Age_1: ChangerMode_1 Batiment_3 Travailler
Age_2: ChangerMode_1 GagnerArgent_3=220 Objet_45=1 Ressource_14=10 Batiment_3 Travailler
Age_3: ChangerMode_1 Ressource_11=1 Objet_28=1 Batiment_31 Travailler
Age_4: ChangerMode_1 Ressource_23=1 Ressource_1=3 Objet_45=1 Objet_28=1 Objet_215=1 ChangerMode_2 GagnerArgent_31=220 Batiment_31 Travailler
Age_5: ChangerMode_1 Ressource_23=1 Objet_34=1 Ressource_1=3 Objet_45=1 Objet_28=1 Objet_215=1 ChangerMode_2 GagnerArgent_31=220 Batiment_31 Travailler
#### Paladin offensif.ia
desc: IA officielle. Le chevalier combat toujours au contact et porte toujours les armures les plus lourdes.
header: Equiper_0
Age_1: ChangerMode_1 Objet_9=1 ChangerMode_2 Ressource_1=3 Objet_38=1 Objet_11=1 Patrouiller
Age_2: ChangerMode_1 Objet_14=1 ChangerMode_2 Ressource_1=3 Objet_74=1 Objet_20=1 Batiment_19 Travailler
Age_3: ChangerMode_1 Objet_14=1 Objet_23=1 ChangerMode_2 Ressource_11=1 Objet_171=1 Objet_25=1 Objet_21=1 Ressource_19=3 Batiment_19 Attaquer Donjon
Age_4: ChangerMode_1 Objet_14=1 Objet_23=1 ChangerMode_2 Objet_41=1 Ressource_11=1 Objet_171=1 Objet_25=1 Objet_84=1 Objet_100=1 Ressource_19=3 Batiment_19 Attaquer Donjon
Age_5: ChangerMode_1 Objet_205=1 Objet_23=1 Objet_41=1 ChangerMode_2 Ressource_11=1 Objet_171=1 Objet_25=1 Objet_84=1 Objet_100=1 Ressource_19=3 Batiment_19 Attaquer Donjon
#### Papetier.ia
desc: IA officielle. C'est le villageois de base qui se contente de fabriquer du papier.
header: Equiper_0
Age_1: ChangerMode_1 GagnerArgent_3=120 Objet_4=1 Batiment_3 Travailler
Age_2: ChangerMode_1 GagnerArgent_3=120 GagnerArgent_1=550 Objet_231=1 Ressource_14=8 Batiment_3 Travailler
Age_3: ChangerMode_1 GagnerArgent_3=120 Objet_4=1 Batiment_3 Travailler
Age_4: ChangerMode_1 GagnerArgent_33=120 Objet_4=1 GagnerArgent_33=550 Objet_66=1 Batiment_33 Travailler
Age_5: ChangerMode_1 GagnerArgent_33=120 Objet_4=1 GagnerArgent_33=550 Objet_66=1 Batiment_33 Travailler
#### Parcheminier.ia
desc: IA officielle. C'est le villageois de base qui se contente de fabriquer du parchemin.
header: Equiper_0
Age_1: ChangerMode_1 Batiment_5 Travailler
Age_2: ChangerMode_1 Objet_231=1 Batiment_17 Ressource_3=5 Ressource_14=9 Travailler
Age_3: ChangerMode_1 GagnerArgent_25=220 Objet_45=1 GagnerArgent_25=550 Objet_231=1 Batiment_25 Travailler
Age_4: ChangerMode_1 GagnerArgent_25=220 Objet_45=1 GagnerArgent_25=550 Objet_66=1 GagnerArgent_25=800 Objet_97=1 Batiment_25 Travailler
Age_5: ChangerMode_1 GagnerArgent_25=220 Objet_45=1 GagnerArgent_25=550 Objet_66=1 GagnerArgent_25=800 Objet_97=1 Batiment_25 Travailler
#### Porte-bouclier.ia
desc: IA officielle. Le porte bouclier s'équipe toujours avec ce qui offre le plus de défense et protège le village.
header: Equiper_0
Age_1: ChangerMode_1 Objet_12=2 ChangerMode_2 Ressource_1=3 Patrouiller
Age_2: ChangerMode_1 Objet_22=2 ChangerMode_2 Ressource_1=3 Batiment_19 Patrouiller
Age_3: ChangerMode_1 Objet_23=2 ChangerMode_2 Objet_20=1 Objet_24=1 Ressource_1=3 Batiment_19 Patrouiller
Age_4: ChangerMode_1 Objet_23=2 ChangerMode_2 Objet_108=1 Objet_24=1 Ressource_1=3 Batiment_19 Patrouiller
Age_5: ChangerMode_1 Objet_23=2 ChangerMode_2 Objet_223=1 Objet_24=1 Ressource_1=3 Batiment_19 Patrouiller
#### Pretre.ia
desc: IA officielle. C'est un villageois défensif qui réssuscite ses alliés.
header: Equiper_0
Age_1: ChangerMode_1 Objet_46=1 Ressource_13=8 Batiment_9 Travailler
Age_2: ChangerMode_1 Objet_9=1 Ressource_14=8 Batiment_9 Travailler
Age_3: ChangerMode_1 GagnerArgent_26=520 Objet_27=1 Batiment_26 Travailler
Age_4: ChangerMode_1 GagnerArgent_26=520 Objet_27=1 Batiment_26 Travailler
Age_5: ChangerMode_1 GagnerArgent_26=520 Objet_27=1 Batiment_26 Travailler
#### Pyromage.ia
desc: IA officielle. Un pyromage qui travaille à sa guilde.
header: Equiper_0
Age_1: ChangerMode_1 GagnerArgent_3=220 Objet_45=1 Batiment_3 Travailler
Age_2: ChangerMode_1 GagnerArgent_3=220 Objet_45=1 Ressource_5=12 Ressource_14=15 Batiment_3 Travailler
Age_3: ChangerMode_1 Objet_30=1 Objet_89=1 ChangerMode_2 Batiment_30 Travailler
Age_4: ChangerMode_1 Objet_30=1 Objet_89=1 Objet_215=1 ChangerMode_2 Batiment_30 Travailler
Age_5: ChangerMode_1 Objet_30=1 Objet_89=1 Objet_215=1 ChangerMode_2 Batiment_30 Travailler
#### Tailleur.ia
desc: IA officielle. C'est le villageois de base qui se contente de fabriquer des vêtements.
header: Equiper_0
Age_1: ChangerMode_1 Objet_2=1 Objet_3=1 GagnerArgent_9=320 Objet_46=1 Batiment_9 Travailler
Age_2: ChangerMode_1 Objet_2=1 Objet_3=1 GagnerArgent_9=320 Objet_46=1 Batiment_9 Travailler
Age_3: ChangerMode_1 Objet_2=1 Objet_3=1 GagnerArgent_9=320 Objet_46=1 Batiment_9 Travailler
Age_4: ChangerMode_1 Objet_90=1 Objet_3=1 GagnerArgent_9=320 Objet_46=1 Batiment_9 Travailler
Age_5: ChangerMode_1 Objet_90=1 Objet_3=1 GagnerArgent_9=320 Objet_46=1 Batiment_9 Travailler
#### Tanneur.ia
desc: IA officielle. C'est le villageois de base qui se contente de fabriquer du cuir.
header: Equiper_0
Age_1: ChangerMode_1 Objet_9=1 Batiment_12 Travailler
Age_2: ChangerMode_1 GagnerArgent_12=480 Objet_11=1 Batiment_12 Travailler
Age_3: ChangerMode_1 GagnerArgent_12=220 Objet_45=1 GagnerArgent_12=480 Objet_11=1 Batiment_12 Travailler
Age_4: ChangerMode_1 GagnerArgent_12=220 Objet_45=1 GagnerArgent_12=480 Objet_11=1 Batiment_12 Travailler
Age_5: ChangerMode_1 GagnerArgent_12=220 Objet_45=1 GagnerArgent_12=480 Objet_11=1 Batiment_12 Travailler
#### Tavernier.ia
desc: IA officielle. C'est le villageois de base qui se contente de fabriquer de la bière.
header: Equiper_0
Age_1: ChangerMode_1 Batiment_8 Travailler
Age_2: ChangerMode_1 GagnerArgent_8=220 Objet_3=1 Batiment_8 Travailler
Age_3: ChangerMode_1 GagnerArgent_8=320 Objet_46=1 Batiment_8 Travailler
Age_4: ChangerMode_1 GagnerArgent_8=560 Objet_2=1 Batiment_8 Travailler
Age_5: ChangerMode_1 GagnerArgent_8=560 Objet_2=1 Batiment_8 Travailler
#### Tecto-mage.ia
desc: IA officielle. Un tecto-mage qui travaille à son atelier.
header: Equiper_0
Age_1: ChangerMode_1 Objet_231=1 Ressource_13=20 Ressource_15=20 Ressource_18=20 Ressource_24=20 Batiment_10 Travailler
Age_2: ChangerMode_1 Objet_231=1 Objet_7=1 Ressource_13=20 Ressource_15=20 Ressource_18=20 Ressource_24=20 Batiment_10 Travailler
Age_3: ChangerMode_1 Objet_7=1 Ressource_15=20 Ressource_18=20 Ressource_24=20 Batiment_29 Travailler
Age_4: ChangerMode_1 GagnerArgent_38=190 Objet_7=1 Batiment_38 Travailler
Age_5: ChangerMode_1 GagnerArgent_38=190 Objet_7=1 Batiment_38 Travailler
#### Apprentis/Apprenti alchimiste.ia
desc: IA officielle utilisée pour l'apprenti alchimiste.
header: Equiper_0
Age_1: Ressource_1=20 ChangerMode_3 Ressource_18=20 ChangerMode_3 Ressource_7=20 ChangerMode_3 Batiment_6 Travailler
Age_2: Ressource_1=20 ChangerMode_3 Ressource_18=20 ChangerMode_3 Ressource_7=20 ChangerMode_3 Batiment_6 Travailler
Age_3: Ressource_19=20 ChangerMode_3 Ressource_20=20 ChangerMode_3 Batiment_27 Travailler
Age_4: Ressource_19=20 ChangerMode_3 Ressource_20=20 ChangerMode_3 Ressource_1=20 ChangerMode_3 Objet_110=1 ChangerMode_3 Batiment_41 Travailler
Age_5: Ressource_19=20 ChangerMode_3 Ressource_20=20 ChangerMode_3 Ressource_1=20 ChangerMode_3 Objet_110=1 ChangerMode_3 Batiment_41 Travailler
#### Apprentis/Apprenti artisan.ia
desc: IA officielle utilisée pour l'apprenti artisan.
header: Equiper_0
Age_1: Ressource_3=20 Ressource_12=20 Ressource_15=20 Ressource_16=20 ChangerMode_3 Batiment_10 Travailler
Age_2: Ressource_3=20 Ressource_12=20 Ressource_15=20 Ressource_16=20 ChangerMode_3 Batiment_10 Travailler
Age_3: Ressource_3=20 Ressource_12=20 Ressource_15=20 Ressource_16=20 ChangerMode_3 Batiment_10 Travailler
Age_4: Ressource_3=20 Ressource_12=20 Ressource_15=20 Ressource_16=20 ChangerMode_3 Batiment_10 Travailler
Age_5: Ressource_3=20 Ressource_12=20 Ressource_15=20 Ressource_16=20 ChangerMode_3 Batiment_10 Travailler
#### Apprentis/Apprenti forgeron.ia
desc: IA officielle utilisée pour l'apprenti forgeron.
header: Equiper_0
Age_1: Ressource_15=20 ChangerMode_3 Ressource_24=20 ChangerMode_3 Batiment_11 Travailler
Age_2: Ressource_15=20 ChangerMode_3 Ressource_16=20 ChangerMode_3 Batiment_20 Travailler
Age_3: Ressource_15=20 ChangerMode_3 Ressource_16=20 ChangerMode_3 Ressource_17=20 ChangerMode_3 Batiment_24 Travailler
Age_4: Ressource_15=20 ChangerMode_3 Ressource_16=20 ChangerMode_3 Ressource_17=20 ChangerMode_3 Ressource_25=20 ChangerMode_3 Batiment_24 Travailler
Age_5: Ressource_15=20 ChangerMode_3 Ressource_16=20 ChangerMode_3 Ressource_17=20 ChangerMode_3 Ressource_25=20 ChangerMode_3 Batiment_24 Travailler
#### Apprentis/Apprenti mage.ia
desc: IA officielle utilisée pour l'apprenti mage.
header: Equiper_0
Age_1: Ressource_18=20 ChangerMode_3 Batiment_4 Travailler
Age_2: Ressource_18=20 ChangerMode_3 Batiment_4 Travailler
Age_3: Ressource_20=20 ChangerMode_3 Ressource_21=20 ChangerMode_3 Batiment_25 Travailler
Age_4: Ressource_20=20 ChangerMode_3 Ressource_22=20 ChangerMode_3 Ressource_21=20 ChangerMode_3 Objet_152=1 ChangerMode_3 Batiment_33 Travailler
Age_5: Ressource_20=20 ChangerMode_3 Ressource_22=20 ChangerMode_3 Ressource_21=20 ChangerMode_3 Objet_152=1 ChangerMode_3 Batiment_33 Travailler
#### Apprentis/Apprenti necro.ia
desc: IA officielle utilisée pour l'apprenti nécromancien.
header: Equiper_0
Age_1: Batiment_5 Travailler
Age_2: Batiment_5 Travailler
Age_3: Objet_35=1 ChangerMode_3 Batiment_31 Travailler
Age_4: Objet_35=1 ChangerMode_3 Batiment_31 Travailler
Age_5: Objet_35=1 ChangerMode_3 Batiment_31 Travailler
#### Apprentis/Apprenti tecto.ia
desc: IA officielle utilisée pour l'apprenti tectomage.
header: Equiper_0
Age_1: Ressource_13=20 Batiment_3 Travailler
Age_2: Batiment_18 Travailler
Age_3: Objet_44=1 ChangerMode_3 Batiment_29 Travailler
Age_4: Objet_44=1 ChangerMode_3 Batiment_38 Travailler
Age_5: Objet_44=1 ChangerMode_3 Batiment_38 Travailler
#### Apprentis/Domestique.ia
desc: IA officielle utilisée pour le domestique.
Age_1: Ressource_2=20 Ressource_1=20 ChangerMode_3 Ressource_9=20 ChangerMode_3 Ressource_11=20 ChangerMode_3 Batiment_7 Travailler
Age_2: Ressource_2=20 Ressource_1=20 ChangerMode_3 Ressource_9=20 ChangerMode_3 Ressource_11=20 ChangerMode_3 Batiment_6 Travailler
Age_3: Ressource_2=20 Ressource_1=20 ChangerMode_3 Ressource_9=20 ChangerMode_3 Ressource_11=20 ChangerMode_3 Batiment_8 Travailler
Age_4: Ressource_2=20 Ressource_1=20 ChangerMode_3 Ressource_9=20 ChangerMode_3 Ressource_11=20 ChangerMode_3 Ressource_23=20 ChangerMode_3 Batiment_35 Travailler
Age_5: Ressource_2=20 Ressource_1=20 ChangerMode_3 Ressource_9=20 ChangerMode_3 Ressource_11=20 ChangerMode_3 Ressource_23=20 ChangerMode_3 Batiment_35 Travailler
#### Empereur/Empereur.ia
desc: IA officielle utilisée par l'empereur.
Age_1: ChangerMode_8 Objet_2=1 Objet_186=1 Objet_198=1 ChangerMode_7 Patrouiller
Age_2: ChangerMode_8 Objet_2=1 Objet_186=1 Objet_198=1 Objet_195=1 Objet_194=1 ChangerMode_7 Patrouiller
Age_3: ChangerMode_8 Objet_186=1 Objet_196=1 Objet_195=1 Objet_194=1 ChangerMode_7 Patrouiller
Age_4: ChangerMode_8 Objet_104=1 ChangerMode_2 Objet_87=2 Objet_186=1 Objet_220=1 Objet_197=1 Objet_196=1 Objet_195=1 Objet_194=1 ChangerMode_7 Patrouiller
Age_5: ChangerMode_8 Objet_205=1 Objet_223=1 Objet_81=1 Objet_199=1 ChangerMode_2 Objet_87=2 Objet_186=1 Objet_221=1 Objet_226=1 Objet_220=1 Objet_197=1 Objet_196=1 Objet_195=1 Objet_194=1 ChangerMode_7 Patrouiller
#### Empereur/Mercenaire imperial.ia
desc: IA officielle. Un soldat équipé moyennement qui protège le village.
Age_1: ChangerMode_1 Ressource_15=20 Ressource_13=20 Ressource_1=3 ChangerMode_2 ChangerMode_4 Patrouiller
Age_2: ChangerMode_1 Objet_14=1 Ressource_1=3 ChangerMode_2 Objet_20=1 Objet_24=1 Batiment_CASERNE ChangerMode_4 Travailler
Age_3: ChangerMode_1 Objet_14=1 Ressource_1=3 ChangerMode_2 Objet_20=1 Objet_24=1 Batiment_CASERNE ChangerMode_4 Travailler
Age_4: ChangerMode_1 Objet_14=1 Ressource_1=3 ChangerMode_2 Objet_20=1 Objet_24=1 Batiment_CASERNE ChangerMode_4 Travailler
Age_5: ChangerMode_1 Objet_14=1 Ressource_1=3 ChangerMode_2 Objet_20=1 Objet_24=1 Batiment_CASERNE ChangerMode_4 Travailler
#### Empereur/Milicien imperial.ia
desc: IA officielle. Un soldat équipé moyennement qui protège le village.
Age_1: ChangerMode_1 Ressource_15=20 Ressource_13=20 Ressource_1=3 ChangerMode_2 ChangerMode_4 Patrouiller
Age_2: ChangerMode_1 Objet_14=1 Ressource_1=3 ChangerMode_2 Objet_20=1 Objet_24=1 Batiment_CASERNE ChangerMode_4 Travailler
Age_3: ChangerMode_1 Objet_14=1 Ressource_1=3 ChangerMode_2 Objet_20=1 Objet_24=1 Batiment_CASERNE ChangerMode_4 Travailler
Age_4: ChangerMode_1 Objet_14=1 Ressource_1=3 ChangerMode_2 Objet_20=1 Objet_24=1 Batiment_CASERNE ChangerMode_4 Travailler
Age_5: ChangerMode_1 Objet_14=1 Ressource_1=3 ChangerMode_2 Objet_20=1 Objet_24=1 Batiment_CASERNE ChangerMode_4 Travailler
#### Monstres/Archer elfe.ia
desc: IA officielle. Un archer elfe qui s'équipe avec des arcs.
Age_1: ChangerMode_1 Ressource_2=1 Objet_18=1 ChangerMode_2 Patrouiller
Age_2: ChangerMode_1 Ressource_2=1 Objet_18=1 ChangerMode_2 Patrouiller
Age_3: ChangerMode_1 Ressource_2=1 Objet_18=1 ChangerMode_2 Patrouiller
Age_4: ChangerMode_1 Ressource_2=1 Objet_18=1 ChangerMode_2 Patrouiller
Age_5: ChangerMode_1 Ressource_2=1 Objet_18=1 ChangerMode_2 Patrouiller
#### Monstres/Archimage.ia
desc: IA officielle. Un mage offensif qui attaque à coups de golems.
Age_1: ChangerMode_1 Objet_44=4 ChangerMode_2 Patrouiller
Age_2: ChangerMode_1 Objet_44=4 ChangerMode_2 Attaquer Ennemi
Age_3: ChangerMode_1 Objet_44=4 ChangerMode_2 Attaquer Donjon
Age_4: ChangerMode_1 Objet_44=4 ChangerMode_2 Attaquer Donjon
Age_5: ChangerMode_1 Objet_44=4 ChangerMode_2 Attaquer Donjon
#### Monstres/Capitaine imperial.ia
desc: IA officielle. Une centurion impérial qui s'équipe pour le combat.
Age_1: Objet_57=1 Patrouiller
Age_2: Objet_57=1 Patrouiller
Age_3: Objet_57=1 Objet_21=1 Objet_25=1 Patrouiller
Age_4: Objet_57=1 Objet_21=1 Objet_25=1 Patrouiller
Age_5: Objet_57=1 Objet_21=1 Objet_25=1 Patrouiller
#### Monstres/Chevalier imperial.ia
desc: IA officielle. Une chevalier impérial qui s'équipe pour le combat.
Age_1: GagnerArgent_43=600 Patrouiller
Age_2: GagnerArgent_43=600 Objet_57=1 Patrouiller
Age_3: GagnerArgent_43=600 Objet_57=1 Objet_21=1 Patrouiller
Age_4: GagnerArgent_43=600 Objet_57=1 Objet_21=1 Patrouiller
Age_5: GagnerArgent_43=600 Objet_57=1 Objet_21=1 Patrouiller
#### Monstres/General imperial.ia
desc: IA officielle. Un général impérial qui s'équipe pour le combat.
Age_1: Objet_129=4 Attaquer Donjon
Age_2: Objet_129=4 Attaquer Donjon
Age_3: Objet_129=4 Attaquer Donjon
Age_4: Objet_129=4 Attaquer Donjon
Age_5: Objet_129=4 Attaquer Donjon
#### Monstres/Golem escavateur.ia
desc: IA officielle utilisée pour l'escavateur golem.
Age_1: Ressource_13=5 ChangerMode_3 Ressource_18=5 ChangerMode_3 Ressource_15=5 ChangerMode_3 Ressource_24=5 ChangerMode_3 Patrouiller
Age_2: Ressource_13=5 ChangerMode_3 Ressource_18=5 ChangerMode_3 Ressource_15=5 ChangerMode_3 Ressource_24=5 ChangerMode_3 Patrouiller
Age_3: Ressource_13=5 ChangerMode_3 Ressource_18=5 ChangerMode_3 Ressource_15=5 ChangerMode_3 Ressource_24=5 ChangerMode_3 Patrouiller
Age_4: Ressource_13=5 ChangerMode_3 Ressource_18=5 ChangerMode_3 Ressource_15=5 ChangerMode_3 Ressource_24=5 ChangerMode_3 Patrouiller
Age_5: Ressource_13=5 ChangerMode_3 Ressource_18=5 ChangerMode_3 Ressource_15=5 ChangerMode_3 Ressource_24=5 ChangerMode_3 Patrouiller
#### Monstres/Maman dragon.ia
desc: IA officielle. C'est une ia qui fabrique des bébés dragons.
Age_1: Objet_56=10 Patrouiller
Age_2: Objet_56=10 Patrouiller
Age_3: Objet_56=10 Patrouiller
Age_4: Objet_56=10 Patrouiller
Age_5: Objet_56=10 Patrouiller
#### Monstres/Savant.ia
desc: IA officielle. Un savant qui se fabrique des armes à feu.
header: Equiper_0
Age_1: ChangerMode_1 Objet_94=1 Ressource_26=1 Objet_148=1 ChangerMode_2 Patrouiller
Age_2: ChangerMode_1 Objet_94=1 Ressource_26=1 Objet_148=1 ChangerMode_2 Patrouiller
Age_3: ChangerMode_1 Objet_94=1 Ressource_26=1 Objet_148=1 ChangerMode_2 Patrouiller
Age_4: ChangerMode_1 Objet_94=1 Ressource_26=1 Objet_148=1 ChangerMode_2 Patrouiller
Age_5: ChangerMode_1 Objet_94=1 Ressource_26=1 Objet_148=1 ChangerMode_2 Patrouiller
#### Monstres/Soldat imperial.ia
desc: IA officielle. Un soldat impérial qui s'équipe pour le combat.
Age_1: GagnerArgent_42=600 Objet_57=1 Patrouiller
Age_2: GagnerArgent_42=600 Objet_57=1 Patrouiller
Age_3: GagnerArgent_42=600 Objet_57=1 Patrouiller
Age_4: GagnerArgent_42=600 Objet_57=1 Patrouiller
Age_5: GagnerArgent_42=600 Objet_57=1 Patrouiller
#### Monstres/Vampire.ia
desc: IA officielle. Le vampire.
Age_1: ChangerMode_1 Objet_35=3 ChangerMode_2 Patrouiller
Age_2: ChangerMode_1 Objet_35=3 ChangerMode_2 Attaquer Ennemi
Age_3: ChangerMode_1 Objet_35=3 ChangerMode_2 Attaquer Donjon
Age_4: ChangerMode_1 Objet_35=3 ChangerMode_2 Attaquer Donjon
Age_5: ChangerMode_1 Objet_35=3 ChangerMode_2 Attaquer Donjon
#### Scenarios/Bataille.ia
desc: IA officielle. Un guerrier qui combat toujours pendant les batailles.
Age_1: Patrouiller
Age_2: Patrouiller
Age_3: Patrouiller
Age_4: Patrouiller
Age_5: Patrouiller
#### Scenarios/Berserk.ia
desc: IA officielle. Un combattant fou furieux qui attaque tout ce qui bouge.
Age_1: Attaquer Ennemi
Age_2: Attaquer Ennemi
Age_3: Attaquer Ennemi
Age_4: Attaquer Ennemi
Age_5: Attaquer Ennemi
```
