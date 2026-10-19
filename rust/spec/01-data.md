# 01 — Data & configuration formats, parameter model

Scope: everything under `Donnees/`, the map format under `Cartes/`, the parameter
data model built by `ClsJeuParam`, mods, and the conventions used to find images
under `Images/`. UI layout files (`Interface*.ini`, `Fenetres.ini`, `Boutons.ini`)
are covered only briefly. Network play and the AI script language are out of
scope.

Source references are `File:line` in the VB6 tree
(`Vendetta-3.0.0.17/`). Line numbers count from the top of the file, including
the `VERSION`/`Attribute` header.

---

## 0. Conventions that apply everywhere

### 0.1 Encoding, paths, case

* All text files are **ISO-8859-1 (Windows-1252), CRLF**. Keys contain
  non-ASCII characters (e.g. `MinQuantitéRessources` in `Partie.ini`,
  `SAVPerso = CompétenceObjet25`), so keys must be compared in that
  encoding, or after decoding to Unicode.
* `App.Path` is the install root (`C:\Vendetta`). Most paths are built by plain
  string concatenation, which often gives a doubled backslash (`App\\Donnees\…`).
  Treat `\\` as `\`.
* Windows file lookup is **case-insensitive**, and the data relies on that. For
  example, `Fiesta feline` and `Conv Arc Ange` (object `Image` keys) do not
  match the case of the files on disk. The port must use case-insensitive path
  resolution.
* **Directory enumeration order matters.** VB `Dir()` returns entries in
  filesystem order: alphabetical on NTFS, arbitrary under Wine on ext4. Several
  loaders draw random numbers that depend on that order: random sprite or
  appearance choice, and the order in which mods are applied. To match Windows,
  enumerate in case-insensitive ascending order (NTFS order).

### 0.2 VB6 value semantics the loaders rely on

| Operation | Behaviour to replicate |
|---|---|
| `Val(s)` | Skips blanks. Parses an optional sign, digits, an optional `.` fraction and an optional exponent, and stops at the first other character. Locale-independent: `.` is the decimal separator. `Val("1,75") = 1`. `Val("")=0`. `Val("&H00FF&")` parses hex (`&H…`, optional trailing `&`), so colour values such as `&H000000FF&` are 255. |
| Implicit `String → Integer/Long` (e.g. `PersosMaxVie = FicIni.Parametre(...)`, no `Val`) | `CInt`/`CLng` of the string. Locale-dependent, rounds to nearest even, and **raises error 13 on `""`**. Every key read this way is effectively mandatory: a missing key crashes the game. |
| `Double/Single → Integer/Long` (assignment, array index) | **Round half to even** (banker's rounding), not truncation. `Int()` floors. |
| `Integer field = Val(x)` | The same rounding (`Val("2.5")` stored in an Integer gives 2). Overflow over 32767 raises an error. |
| `Boolean field = Val(x)` | Nonzero → `True`. VB `True` is `-1`; when written back to INI, code uses `-CDbl(b)` or explicit 1/0. |
| `CBool(s)` on a string | `"0"`/`"1"`/numbers work. `""` raises a type mismatch. |
| `"3" + 5` (string + number) | Numeric addition. Used by the relative-reference syntax (§2.3). |
| `Select Case` on strings | **Case-sensitive** (no `Option Compare Text`). `quantite` ≠ `Quantite`. |
| `Rnd` | VB's LCG. Covered elsewhere, but note that loaders call `Rnd` (§10, §11). |

### 0.3 Index bases

The files mix 1-based and 0-based references. Internally the model is
**0-based**, and "none" is `-1`. §13 has the full cheat sheet. Most fields are
stored as `Val(v) - 1`, so `0` in a file means "none".

---

## 1. Installation layout (data-relevant parts)

```
Vendetta\
  Donnees.ini                 bootstrap: data paths (§3.1)
  Donnees\                    all configuration and parameter files
    *.ini                     language-neutral files
    Francais\, English\       language folders (Langues.ini → Dossier)
    ModsOfficiels\            official mod overlays, *.ini (none shipped at root)
      English\                English names overlay (Parametres_*.ini)
      Francais\               empty (Mods.txt only)
  Mods\                       player mods (*.ini, plus one level of sub-folders)
    Vrac\…                    shipped example mods, at depth 2, so NOT loaded (§12)
  Cartes\<map name>\          predefined maps (§10)
  Images\                     sprites (§11)
  Sons\                       *.wav effect sounds (referenced by Effet.Son)
  Musiques\<category>\        music
  IAs\ (+1 level sub-dirs)    *.ia AI files (format out of scope)
  IAScripts\                  AI scripts (out of scope)
  Sauvegardes\{Persos,Mondes,Scenarios,Campagnes}\   saves
```

---

## 2. The two INI readers

### 2.1 `ClsFicINI`: Win32 `GetPrivateProfileString`

`ClsFicINI.cls:76-86`. The caller sets `FicIni.Fichier` (a path relative to
`App.Path`, without extension) and `FicIni.Section`, then reads
`FicIni.Parametre(key)`:

```
GetPrivateProfileString(Section, key, "", buf, 255, App.Path & "\" & Fichier & ".ini")
```

The port must reproduce the Windows semantics:

* Section and key names are matched **case-insensitively**, with surrounding
  whitespace trimmed (so `VitesseEpoqueChoisie =3` works).
* The first matching section and the first matching key win.
* Leading and trailing whitespace of the value is trimmed. **One pair of matching
  surrounding quotes** (`"…"` or `'…'`) is removed (`Chemin = "\Cartes\"` gives
  `\Cartes\`).
* No inline comments: `Foo = 1 ;bar` returns `1 ;bar`. Lines starting with `;`
  are comments. Lines starting with `#` are not comments, but they are just
  keys nobody asks for (`#Taille1Nom=…` in `Partie.ini`).
* **The result is truncated to 254 characters** (`STRING_SIZE = 255`,
  `ClsFicINI.cls:47`). This affects long strings read through this path, such
  as `Commentaires.ini` messages and `Info*` labels. It does not affect
  `Parametres_*.ini`, which use the custom parser.
* A missing key returns `""`.
* `Parametre(key) = value` (`:88`) calls `WritePrivateProfileString`. The game
  **writes user preferences back into the data files**: `Jeu.ini`, `Partie.ini`,
  `Langues.ini`, `Terrain.ini` (TerrainOptimise/TerrainAnime), `Cartes.ini`,
  `Commentaires.ini`, `Temps.ini`, `Ecran.ini`, `Musiques.ini`, `Sons.ini`,
  `Messages.ini`, `Boutons.ini`, `Interface*.ini`, `Documents.ini`,
  `SavMonde.ini`, and the per-map `Informations.ini` (§10.5).
* `ClsFicINI.Class_Initialize` (`:60`) reads `Donnees.ini` [Donnees] to get
  `Chemin`, `CheminMods`, `CheminModsOfficiels`, `JournalErreurActif` and
  `JournalErreurFichier`. If logging is active, every read is appended to the
  log file (debug only).

A second instance `FicPer` (`ModMain.bas:44`) uses the extension `.per` for
character saves (§3.13).

### 2.2 Custom parser (`Parametres_*.ini` and mods)

`ClsJeuParam.cls:1228-2107`. It is used for `Parametres_Ressources`,
`_Batiments`, `_CompetencesObjets`, `_Resistances`, `_Objets`, `_Decors`,
`_Peuples`, `_Effets`, `_Races` and every mod file. **It is not an INI parser.**
It is a line-oriented state machine whose state (`Section`, `i`) is reset only at
the start of each file (`Parametre_Lire_Fichier`, `:1228`).

For each line read with `Line Input`:

1. **Filter** (`Parametre_Filtre`, `:2166`): VB `Trim` (spaces only), strip
   leading TAB characters, then `Trim` again. Inner and trailing tabs remain.
2. Skip the line if it is empty or starts with `#` or `;`.
3. **A line with no `=` is a header candidate** (`:1252`). Matching is by
   case-sensitive prefix. The index text is `Mid(line, len(prefix)+1,
   len(line)-len(prefix)-1)`, i.e. everything up to the last character, which is
   assumed to be `]`.

   | Prefix | Index → entity | Sets `Section`? | Creation rule when the index is new |
   |---|---|---|---|
   | `[Ressource_N]` | `i = Val(N)-1` | **no** (relies on an earlier `[Ressources]`) | if `i ≥ Count`: `Count += 1` (append ONE) |
   | `[TerrainRessource_N]` | `i = Val(N)-1` | **no** (needs `[Terrain_Ressources]`) | append one |
   | `[Batiment_ID]` | `i = Gestion_Cle_Batiments(ID)` (string key, §2.4) | Batiments | create if the ID is unknown |
   | `[CompetenceObjet_ID]` | `i = Gestion_Cle_CompetencesObjets(ID)` | CompetencesObjets | create if unknown |
   | `[Objet_N]` / `[Objet_+]` | `i = Val(N)-1`, or `Count` for `+` | Objets | if `i ≥ Count`: `Count = i+1` (gaps possible) |
   | `[Peuple_N]` / `[Peuple_+]` | `Val(N)-1` / `Count` | Peuples | append one |
   | `[Effet_N]` / `[Effet_+]` | same | Effets | append one |
   | `[Race_N]` / `[Race_+]` | same | Races | `Count = i+1` |
   | `[Decor_N]` / `[Decor_+]` | same | **no** (needs `[Decors]`) | append one |
   | `[Resistance_N]` / `+` | same | Resistances | `Count = i+1` |
   | `[Statut_N]` / `+` | same | Statuts | `Count = i+1` |
   | `[Ressources]`, `[Terrain_Ressources]`, `[Batiments]`, `[CompetencesObjets]`, `[Objets]`, `[Peuples]`, `[Effets]`, `[Decors]`, `[Resistances]`, `[Statuts]` | — | yes | — |
   | `[Races]` | sets `i = -1` (the "all races" pseudo-entity, used for common items) | Races | — |
   | anything else (including stray text) | **ignored: Section and i unchanged** | | |

   * "Append one" (`Count += 1`) while `i` stays at `Val(N)-1` means a
     non-contiguous index (e.g. `[Ressource_30]` when 26 exist) writes out of
     bounds. That is a crash; a valid file never does it.
   * With `Count = i+1`, only the **last** element gets the creation defaults
     and array allocations. Gap entries stay zeroed with unallocated arrays,
     which crashes if they are used.
   * Re-opening an existing index (e.g. a mod with `[Race_78]`) creates nothing
     and **modifies fields in place**. Unspecified fields keep their values.
4. **A line with `=` is a key line** (`:1427-1430`):
   * `Valeur` = text after the **first** `=`, space-trimmed.
   * `Temp` = text before it, space-trimmed.
   * `j` = `Parametre_Indice`: if the key contains `(` and `)`, the text between
     them, trimmed and implicitly converted to `Long`. `Foo()` raises a type
     mismatch and crashes. Without parentheses `j = 0`.
   * The key name is the text before `(`, trimmed.
   * Dispatch on `Section`, then `Select Case key` (**case-sensitive**). Unknown
     keys are silently ignored.
5. Error handlers are commented out, so any runtime error crashes the game.

Localised keys: wherever the table below says `Nom[L]`, the parser accepts both
`Nom` and `Nom` + `Langues.Suffixe` (`FR` or `EN`, §3.2). Both write the same
field, so **the last one in file order wins**. A key with the other language's
suffix is ignored. Example: in English, `NomFR = Marché` is ignored and
`NomEN = Market` is used.

### 2.3 Relative-reference syntax (mostly for mods)

Some keys accept `+`, `-`, `+N` or `-N` instead of an absolute 1-based index.
The base files never use it (except header `[X_+]`), but mods do. The VB
arithmetic is reproduced exactly below, **bugs included**. `N` is a decimal
integer. Counts are the values *at the moment the line is parsed*. `i` is the
0-based index of the current entity.

| Key (section) | `+` or `-` alone | `+N` | `-N` | absolute `v` |
|---|---|---|---|---|
| Batiment `CreerProprietaire` | `NbRaces-1` | `N+NbRaces-1` | `N-NbRaces-1` (bug) | `v-1` |
| Objet `EquipementRace`, `TresorRace` | `NbRaces-1` | `N+NbRaces-1` | `N-NbRaces-1` (bug) | `v-1` |
| Objet `LanceProjectile` | `+`: `NbRaces-1` (`-` falls to `Val("-")-1 = -1`) | — | — | `v-1` |
| Objet `EffetAttaque`; Race `EffetAttaque`, `EffetMort` | `+`: `NbEffets-1` | — | — | `v-1` |
| Objet `AttaqueCauseEffet` | `NbRaces-1` (bug: uses races) | `N+NbObjets-1` | `N-NbObjets-1` (bug) | `v-1` |
| Objet `Invocation` | `NbRaces-1` | `NbRaces-1+N` | `NbRaces-1-N` | `v-1` |
| Objet `CreerObjet`, `CreerObjetAllie`, `CreerObjetEnnemi` | `NbObjets-1` | `N+i` | `N-i` (bug) | `v-1` |
| Peuple `MonstreIndice(j)` | `NbRaces-1` | `NbRaces+N-1` | `NbRaces-N-1` | `v-1` |
| Race `Objet(j)`, `Equipement(j)` | `NbObjets-1` | `N+NbObjets-1` | `-N+NbObjets-1` | `v-1` |
| Race `Tombe` | `NbDecors-1` | `N+NbDecors-1` | `-N+NbDecors-1` | `v-1` |
| Race `IndiceBestiaire` | `i+1` | `N+i+1` | `N-i+1` (bug) | `v` (raw) |

`CreerObjet = +0` therefore means "this object itself" (SylPack's
`Objet_Capitalisme.ini`).

### 2.4 String-keyed entities

`Gestion_Cle_Batiments` (`:2109`) and `Gestion_Cle_CompetencesObjets` (`:2135`)
search the existing entries for an `ID` string match (case-sensitive). If none
matches, they **append** a new entry with that ID. An empty ID returns index 0.

* Buildings: the ID is the header text (`[Batiment_17]` → ID `"17"`,
  `[Batiment_PALAIS]` → `"PALAIS"`) or an explicit `ID = x` key, which
  re-targets `i`. The building **index** is creation order. IDs 15 and 16 do not
  exist, so ID `"17"` is index 14. Never compute `ID-1`.
  * On creation: `PrixRessources` is sized to the resource count,
    `CreerProprietaire = GainObjet = PerteObjet = -1`, and `FichierMod` is set.
* Item skills (CompetencesObjets): the ID is the header text (`"1"`…`"28"`,
  `"MAGIESHAMANIQUE"` in a mod).
  * On creation: `NoListe = i`. Every existing object's `BonusCompetencesObjets`
    and every race's `CoefCompObjets` array is grown.
  * The Objet key `Competence = X` calls this function too. **Referencing an
    unknown skill ID creates a new skill.**

### 2.5 Mod loading (`Parametres.Init 8`, `ClsJeuParam.cls:1154-1165`)

`Parametre_Lire_Mods(Chemin, SousDossiers, FichierMod)` (`:1196`):

* It enumerates `Dir(Chemin)` files whose name ends in `.ini`
  (case-sensitive `Right(…)=".ini"`) and parses each one, in `Dir` order.
* If `SousDossiers`, it then enumerates sub-entries whose name contains no `.`
  and recurses into each one **with `SousDossiers = False`**. Only **one level**
  of sub-folders is ever read.

Order:

1. `App.Path & "\Donnees\ModsOfficiels\"`: files only, `FichierMod=False`.
2. `…\ModsOfficiels\` + language folder (`English\` or `Francais\`): files only,
   `FichierMod=False`. This is how the English names are applied.
3. `App.Path & "\Mods\"`: files plus one sub-folder level, `FichierMod=True`.
   The shipped `Mods\Vrac\<pack>\*.ini` are at depth 2, so they are **not
   loaded**. Their content assumes `Mods\<pack>\` (e.g. paths
   `Mods\SylPack\…`).
4. `Verification_Donnees` (§8).

`FichierMod` is stored on Ressource, Batiment, CompetenceObjet, Objet, Peuple and
Race. It is only used by save and UI code.

---

## 3. Language-neutral files in `Donnees\`

Loader order at start-up (`ModMain.bas:121-262`):

| # | What | Class / sub | Files |
|---|---|---|---|
| 1 | INI helper | `ClsFicINI` init | `Donnees.ini` |
| 2 | Languages | `ClsLangues` init (`:46`) | `Langues.ini` |
| 3 | Global labels | `ClsJeuParam.Class_Initialize` (`:687`) | `<L>\Parametres_Globales.ini`, `<L>\Chargement.ini` |
| 4 | Loading captions | `ModMain.bas:135` | `<L>\Chargement.ini` [Chargement_1] |
| 5 | Sound, music | `ClsPerSon`, `ClsPerMusique` | `Sons.ini`, `Musiques.ini` |
| 6 | Game mode names | `ClsJeuPartie` (`:43`) | `<L>\Partie.ini` |
| 7 | World / terrain | `ClsJeuMonde` (`:75`) | `<L>\Terrain.ini` |
| 8 | Maps | `ClsJeuCartes` (`:90`) | `Cartes.ini`, lists `Cartes\` |
| 9 | Game settings | `ClsJeu` (`:68`) | `Jeu.ini` |
| 10 | Comments, messages | `ClsJeuCommentaires`, `ClsJeuMessages` | `<L>\Commentaires.ini`, `<L>\Messages.ini` |
| 11 | Sprite classes | `ClsAffApparence.Init1` (`:55`), `ClsAffPerso` init (`:39`), `ClsAffSouris`, `ClsAffDegats`, `ClsAffTemps` | `Apparences.ini`, `Souris.ini`, `Combat.ini` [Degats], `Temps.ini`; scans `Images\Apparences`, `Images\Persos` |
| 12 | Screen | `ClsPerEcran` (`:43`) | `Ecran.ini` |
| 13 | Interface | `ClsInterface.Init1` | `Fenetres.ini`, `Interface<W>[_Jn].ini`, `<L>\Interface.ini`, `Boutons.ini` |
| 14 | Docs | `Init_Documents` | `Documents.ini` |
| 15 | Params 0 | `Parametres.Init 0` | `Parametres_Persos.ini`, `<L>\Competences_Speciales.ini` |
| 16 | Names | `ClsJeuNoms` (`:40`) | `NomsM.ini`, `NomsF.ini` |
| 17 | Params 1 | `Init 1` | `<L>\Parametres_Fiefs.ini` |
| 18 | Params 2 | `Init 2` | `Parametres_Ressources.ini` |
| 19 | Params 3 | `Init 3` | `Parametres_Batiments.ini` |
| 20 | Params 4 | `Init 4` | `<L>\Tresors.ini`, `<L>\Services.ini`, `Parametres_CompetencesObjets.ini`, `Parametres_Resistances.ini`, `Parametres_Objets.ini`, `Parametres_Decors.ini` (in this order) |
| 21 | Params 5 | `Init 5` | `Parametres_Peuples.ini`, then `CheminImage` defaulting |
| 22 | Params 6 | `Init 6` | `Parametres_Effets.ini`, then `DureeEtape 0→1` |
| 23 | Params 7 | `Init 7` | `Parametres_Races.ini` |
| 24 | Params 8 | `Init 8` | mods (§2.5), then `Verification_Donnees` (§8) |
| 25 | Combat | `Init_Combat` (`ModCombat.bas:46`) | `Combat.ini` [Combat] |
| 26 | Saves | `Init_Sauvegarde_Perso`, `Init_Sauvegarde_Monde` | `SavPerso.ini`, `<L>\SavMonde.ini` |
| 27 | AIs | `Charger_Dossiers_IAs` (`ModIA2.bas:78`) | `IAs\` and one sub-level |
| 28 | Appearances | `AffApparence.Init2` (`:105`) | enumerates appearance folders (including ones added by races) |

Consequences of this order that the port must keep:

* **Objects (Init 4) are parsed before Peuples, Effets and Races.**
  * In the base files, `+` references to races or effects from objects would
    resolve to −1.
  * `TresorRace` and `EquipementRace` only work from mods, because `Races()`
    does not exist yet during Init 4.
* `LanceProjectile` and `DureeRecharge` depend on `Jeu.ini` (#9), which is loaded
  earlier (§3.4).
* The fixups `Peuple.CheminImage := CheminImageDefaut` (Init 5) and
  `Effet.DureeEtape 0 → 1` (Init 6) run **before** mods. Peoples and effects
  added by mods do not get them.

`<L>` below means `Langues.Dossier`, e.g. `Francais\`.

### 3.1 `Donnees.ini` (install root) — `[Donnees]`

| Key | Value | Meaning |
|---|---|---|
| `JournalErreurActif` | 0 | debug: log every INI read |
| `JournalErreurFichier` | `Doc_Journal de chargement.txt` | log file |
| `Chemin` | `\Donnees\` | `FicIni.Chemin`, the prefix of all data paths |
| `CheminMods` | `\Mods\` | player mods |
| `CheminModsOfficiels` | `\Donnees\ModsOfficiels\` | official mods |

### 3.2 `Langues.ini` — `ClsLangues.cls:46`

`[Langues] NombreLangues=2, LangueChoisie=1` (1-based, written back when
changed). Each `[Langue_k]` has `Nom`, `Dossier` (`Francais\`, `English\`) and
`Suffixe` (`FR`, `EN`).

* `Dossier` selects the language folder.
* `Suffixe` selects localised keys (§2.2) and the map description file
  (`DescriptionFR.txt`).

### 3.3 `NomsM.ini`, `NomsF.ini` — `ClsJeuNoms.cls:40`

These are not INI files: plain lists, one name per line, no header (166 male,
128 female names). They are read with `Line Input` until EOF, so an empty last
line would become an entry.

`Tirer_Nom_Aleatoire(feminine)` returns `Noms[CLng(Rnd * UBound)]`. The index is
**rounded half to even**, so the first and last names have half the probability
of the others. It is used for races with `NomAleatoire=1` (`ClsJeuPerso.cls:4996`)
and for player name suggestions.

### 3.4 `Jeu.ini` — `[Jeu]`, `ClsJeu.cls:68`

All keys are read with `Val`. All are written back by the options UI.

| Key | Default file value | Meaning |
|---|---|---|
| `AffichageRapide` | 1 | fast rendering flag |
| `FPS`, `FPSMin`, `FPSMax` | 60, 5, 60 | target frame rate |
| `Vitesse`, `VitesseMin`, `VitesseMax` | 10, 5, 30 | game speed (ticks per game second; `Jeu.Duree = DureePartie / Vitesse`) |
| `AfficherFantomes`, `AfficherFantomesPNJ` | 0, 0 | show dead (ghost) players / NPCs |
| `AfficherProjectiles` | 1 | **if 0, the Objet key `LanceProjectile` is never loaded**, so no projectile entities exist (ClsJeuParam `:1619`) |
| `Mode2Joueurs`, `Mode2JoueursEcranSepares` | 0, 1 | split-screen two-player mode |
| `Regle1` | 50 | `Regle_Invocations_Multiples` (Integer) — summon rule ("Invoque ou non les invocations par plusieurs"); its use is covered elsewhere |
| `Regle2` | 1 | `Regle_TempsRechargement_Objets`. **If 0, the Objet key `DureeRecharge` is ignored at load** (`:1760`) |
| `Regle3` | 1 | pathfinding: 0 none, 1 normal, 2 advanced |
| `Regle4` | 1 | "confrontation only" in network play (out of scope) |

### 3.5 `Partie.ini` — `FrmParam.frm:2775-3060` (read), `:4266-…` (written on unload)

This file holds the new-game dialog state and the option lists. Every key is
read with implicit conversion unless noted. A companion
`<L>\Partie.ini` provides the localised display names `[Monde] TailleNNom`,
`RessourcesNNom`, `DifficulteNNom`, `[Resurrections_*] ResurrectionNNom`,
plus `[Partie] Mode1..6`, `EnCours1..3`.

**[Personnages]** (UI memory):

* `FichiersSelection`, `CompetencesSelection`: tab indices.
* `CompetencesAfficherToutes`, `CompetencesTri` (0..2), `CompetencesAffichage`
  (0..2).
* `NombrePersonnages`, `Personnage<i>` (0/1, multiselect state of the saved
  character list), `DernierPersonnage`.
* `PersonnageJoueur1`, `PersonnageJoueur2`.

**[Campagnes]** `DerniereCampagne`, `DernierScenario` (list indices).

**[Monde]**

| Key | Meaning |
|---|---|
| `TypePartie` | tab index (0 Campagne, 1 Monde personnalisé, 2 Bataille, 3 Scénario, 4 Sauvegarde, 5 Mode 2J, …) |
| `TypeCarte` | 1 = generate the map, 2 = load a predefined map |
| `DerniereCarte` | 1-based index into the map list |
| `DernierMonde`, `DernierScenario`, `DernierePartie` | list memories |
| `NombreTailles`, `TailleChoisie` | size presets; `TailleNDimensions = W*H` in tiles (split on `*`), `TailleNRessources` (resource node count), `TailleNFiefs`. Presets: 128², 192², 256², 384², 512², 640², 768², 1024² → 50/100/200/400/800/1200/2000/3200 nodes, 2/3/4/6/8/10/12/16 fiefs |
| `TerrainChoisi` | 1-based world type (`Terrain.ini` [Monde_k]) |
| `NombreRessources`, `RessourceChoisie`, `RessourcesNMultiplicateur` | 0.5 / 1 / 2 |
| `EpoqueChoisie`, `RessourcesDepartsChoisie`, `VitesseEpoqueChoisie` | 1-based indices into the `Parametres_Fiefs` lists |
| `MinLargeur`/`ValLargeur`/`MaxLargeur`, same for `Hauteur` | 128 / 256 / 1024 (map size sliders, in tiles) |
| `MinQuantitéRessources`/`ValQuantitéRessources`/`MaxQuantitéRessources` | 20 / 200 / 6400 |
| `NombreDifficultes`, `DifficulteChoisie`, `DifficulteNBonusXP`, `DifficulteNCoefVieChateaux` | 12 levels; BonusXP 0, 1100, 4200, 9300, 16400, 25500, 36600, 49700, 64800, 81900, 101000, 10010000; castle HP coefficient 1, 1.5, … 6, 51 |

**[Resurrections_Nombre]** `NombreResurrections=7`, `ResurrectionChoisie`,
`ResurrectionNValeur` = 0, 1, 3, 10, 25, 100, −1 (−1 = unlimited).

**[Resurrections_Vitesse]** `NombreResurrections=6`, values 0.25, 0.5, 1, 2, 4,
100.

**[Fiefs]**

* `MinFiefs=2`, `ValFiefs=4`, `MaxFiefs=16`, `EquilibreFiefs`.
* `MinVillageoisFief=1`, `MaxVillageoisFief=250`.
* `ValVillageoisFiefN` (citizens per fief).
* `PeupleFiefN`: 1-based people. A value equal to `NombrePeuples+1` (14) is
  "random"; above the list it is clamped to random.
* `EquipeFiefN`: team.

**[Bataille]**, **[Armée_1]**, **[Armée_2]**:

* `MinSoldats`/`ValSoldats`/`MaxSoldats`, `Peuple` (1-based, 14 = random).
* `Equipement` (0/1).
* `Objet<k>` (0/1): selection state of the k-th entry of the battle equipment
  list. That list contains only objects with `Unique=0` and `PrixVente>0`, in
  object order, so `k` is **not** an object index.

### 3.6 `Combat.ini`

`[Combat]` is read by `ModCombat.bas:46`, all keys with `Val`:

| Key | Value | Meaning |
|---|---|---|
| `CoefDegatPerso` | 0.5 | damage coefficient vs characters |
| `CoefDegatBatiment` | 1 | vs buildings |
| `DureeImmobilisation` | 0 | stun duration after a hit |
| `ChanceCoupCritique`, `CoefDegatCoupCritique` | 0.05, 1 | critical hit chance and bonus |
| `ChanceEchecCritique`, `PerteEchecCritique` | 0.05, 20 | fumble chance and attack points lost |
| `MaxDegatBatiment` | 20 | cap on damage per hit vs buildings |
| `CoefPillageArgentPersoMort`, `…RessourcesPersoMort`, `…ObjetsPersoMort` | 0.25 each | loot share from killed characters |
| `CoefDestructionObjets` | 0.5 | chance that looted objects are destroyed instead |
| `CoefPillageArgentMaison`, `…RessourcesMaison`, `…ObjetsMaison` | 0.5 each | loot share from buildings |

`[Degats]` is read by `ClsAffDegats.cls:54` (floating damage numbers, visual
only):

* `AfficherDegats`, `TailleNormale=8`, `TailleCritique=10`.
* `CouleurDegat`/`CouleurGueri`/`CouleurVente`: VB `&H00BBGGRR&` colours, red,
  blue and yellow.
* `DureeMax=15`, `DecalageX/Y=0/-6`, `DeplacementX/Y=0/-1`.
* `AfficherInfoBulle` is present but not read here.

### 3.7 `Cartes.ini` — `ClsJeuCartes.cls:90`

`[Cartes]` (all via GetPrivateProfileString):

* `Chemin="\Cartes\"`, `ExtentionImage=".bmp"`, `ExtentionTexte=".txt"`.
* `NomImageApercu=Apercu`, `NomImageTerrain=Terrain`,
  `NomImageRessources=Ressources`, `NomImageChateaux=Chateaux`,
  `NomImageCarte=Carte`.
* `NomFichierDescription=Description`, `NomFichierInformations=Informations`.
* `CreerFichierInformations=1`: write the `Informations.ini` cache; written back
  by the options UI.
* `NombreTypesTerrains=12`, `NombreTypesRessources=8`. The global keys sit in
  `[Cartes]` and are read before the `[Terrain_k]` sections.

`[Terrain_k]` (k=1..12) and `[Ressource_k]` (k=1..8) hold `R`, `V`, `B`: the
**exact** RGB colours used to decode map bitmaps (§10). Map colour k decodes to
terrain category or terrain-resource type **k−1**.

| k | Terrain (category k−1) | R,V,B | | k | Map resource (TerrainRessource k−1) | R,V,B |
|---|---|---|---|---|---|---|
| 1 | Plaine | 0,255,0 | | 1 | Herbes médicinales | 0,255,0 |
| 2 | Herbe | 49,156,99 | | 2 | Pommier | 255,0,0 |
| 3 | Terre | 132,132,0 | | 3 | Arbre | 64,192,64 |
| 4 | Désert | 255,255,0 | | 4 | Gisement de pierre | 128,128,128 |
| 5 | Montagne | 148,148,148 | | 5 | Gisement de fer | 192,192,192 |
| 6 | Route | 198,198,198 | | 6 | Gisement de cristal | 0,0,255 |
| 7 | Muraille1 | 255,206,156 | | 7 | Gisement d'or | 255,255,0 |
| 8 | Muraille2 | 255,206,0 | | 8 | Gisement de soufre | 192,192,128 |
| 9 | Muraille3 | 255,156,0 | | | (white 255,255,255 = random type) | |
| 10 | Muraille4 | 255,99,0 | | | | |
| 11 | Neige | 255,255,255 | | | | |
| 12 | Eau | 0,206,255 | | | | |

### 3.8 `Parametres_Persos.ini` — `Parametres.Init 0` (`ClsJeuParam.cls:891-932`)

This file is read through GetPrivateProfileString, mostly with **implicit
conversion, so every key is mandatory**.

`[Transactions] CoefficientAchat = 2`: buying price = selling price × 2.

`[Personnages]` holds the race defaults, used whenever a race field is 0 or
missing (§8.4):

| Key | Value | Use |
|---|---|---|
| `PersosLargeurDefaut`, `PersosHauteurDefaut` (Val) | 24, 32 | default sprite frame size |
| `PersosMaxVie`, `…MaxEnergie`, `…MaxMagie`, `…MaxMoral` | 20 each | base characteristics |
| `PersosNiveauSpecialesDepart` | 20 | starting level of special skills |
| `PersosMaxRessources` | 20 | carrying capacity |
| `PersosNiveauServicesDepart`, `PersosNiveauObjetsDepart` | 20, 20 | starting skill levels |
| `PersosAttaque`, `PersosDefense` | 20, 20 | base attack and defence |
| `PersosPorteeAttaque` | 0 | bare-hand range |
| `PersosVitesse`, `PersosAcceleration` | 4, 1 | default speed |
| `PersosNombreObjetsEquipes` | 11 | equipment slots |
| `PersosNombreObjetsInventaire`, `PersosNombreObjetsInventaireMaison` | 32, 32 | inventory sizes (backpack / house) |

### 3.9 `Apparences.ini`

`[Apparences] Chemin=Apparences\` (no trailing newline in the file). It is read
by `ClsAffApparence.Init1` (`:55`); the appearance root is
`Images\Apparences\`. See §11.4.

### 3.10 `Ecran.ini` — `ClsPerEcran.cls:43`

* `[Ecran]`: `ModeFenetre` (0 = fullscreen), `NombreResolution=5`,
  `Resolution=3` (1-based).
* `[Resolution_k]`: `Largeur`, `Hauteur` — 640×480, 800×600, 1024×768,
  1280×1024, 1600×1200.

The width selects `Images\Fond<W>.bmp` and `Interface<W>[_J1|_J2].ini`. Full
screen uses a **16 bpp** display mode (`ClsPerEcran.cls:190`), which matters for
colour keying (§11.1).

### 3.11 `Temps.ini` (day/night overlay) — `ClsAffTemps.cls:58`

* `[Temps]`:
  * `Chemin="Temps\"` (under `Images\`), `Afficher=0` (overlay disabled by
    default).
  * `Largeur`/`Hauteur=512`: overlay tile size.
  * `NombreTemps=11`.
  * `JourneeDebut=9`: starting hour.
  * `JourneeVitesse=0.08`: minutes added per tick.
* `[Temp_k]`: `Debut` (hour), `Afficher` (CBool), `Image`.

The period is the first k whose interval `Debut(k) ≤ h ≤ Debut(k+1)` contains
the hour, otherwise the last one. The `#…` lines are inert keys.

### 3.12 `Sons.ini`, `Musiques.ini`, `Souris.ini`, `Documents.ini`, `Touches*.ini`

These files have no effect on gameplay; they are listed here so that every file
is accounted for.

* `Sons.ini [Sons]`: `Jouer=1`, `Chemin=\Sons\`. `Effet.Son` file names are
  relative to `Sons\`.
* `Musiques.ini [Musiques]`:
  * `Jouer`, `UtiliserDirectMusic` (0 required under Wine).
  * `ExtensionMusiques1..3` = `.mp3`, `.mid`, `.wav`.
  * `Chemin=\Musiques\` and `CheminMenu|Debut|Ville|Combat|Mort|Victoire|Defaite|Boss|Empereur`
    (sub-folders).
  * `IntervalMusiques=256` seconds.
* `Souris.ini [Souris]`: `Chemin="Curseurs\"`, `NombreCurseurs=17`,
  `LargeurImage=HauteurImage=24`. The sheet is
  `Images\Curseurs\ArrowCursor.bmp` (432×24 = 18 frames of 24).
* `Documents.ini [Documents]`: HTML documentation and statistics export (file
  names, HTML snippets, browser paths). The key `NavigateurChoisi` appears
  twice; the first occurrence wins.
* `Touches.ini [Touches]`: DirectInput scan codes (`ToucheQuitter=1` Esc,
  `ToucheAction=28` Enter, `ToucheFenetreCarte=15` Tab, …), read by
  `ClsIntTouches.cls:134`.
* `TouchesDialogue.ini [TouchesDialogue]`: editing keys of the chat/command
  line. Its header comment documents cheat commands `_HEURE=XX` and
  `_REGNER SANS PARTAGE`.
* `Reseau.ini`, `<L>\Reseau.ini`: network, out of scope.

### 3.13 `SavPerso.ini`, `<L>\SavMonde.ini` (save locations)

`SavPerso.ini [SavPerso]` (`ModSavPerso.bas:35`):

* `CheminSavPerso=\Sauvegardes\Persos\`, `ExtensionFichiersPerso=.per`.
* Section names inside a `.per` file: `SectionPerso=Personnage`,
  `SectionInterface=Interface`, `SectionTalents=Talents`.
* `SauvegardePeriodique=1`, `PeriodeSauvegarde=60`.

A `.per` file is itself an INI file read through `FicPer`. Keys include `Race`,
`Sexe` (`"Féminin"`), `Vie`, `Attaque`, `CompétenceSpéciale…` and the
`CompetenceObjet.SAVPerso` names.

`SavMonde.ini [SavMonde]`:

* `CheminSavMonde=\Sauvegardes\Mondes\`, `ExtensionFichiersMonde=.mnd`,
  `FichierVierge=\Sauvegardes\Monde vierge`.
* Auto-save flags, and info labels for the save preview text.

The save formats themselves are specified elsewhere.

### 3.14 UI layout files (brief)

* `Fenetres.ini`: `[Fenetres] NombreFenetres=8`; `[Fenetre_k]` has `Largeur`,
  `Hauteur` (320×368), `NombreImages`, `Image1..n` (bitmaps under
  `Images\Fenetres\`). Read by `ClsInterface.cls:355` and `ClsIntFenetre`.
  Windows: 1 building, 2 resources, 3 characteristics, 4 skills, 5 inventory
  (4 images: male, female, male at home, female at home), 6 bestiary, 7 map,
  8 objectives.
* `Interface<W>.ini`, `Interface<W>_J1.ini`, `Interface<W>_J2.ini`:
  `[Interface]` pixel positions and colours of the HUD for each resolution and
  split-screen player. Two-player mode is possible only if `_J1` exists
  (`ClsInterface.cls:340`).
* `Boutons.ini [Boutons]`:
  * Tooltip settings; button sheet names and frame sizes (`NomBoutonX`,
    `LargeurBoutonX`, `HauteurBoutonX`).
  * `CheminBoutonConstruire="Construire\"`, `NomBoutonDemolir="_Demolir"`,
    construct buttons 28×28.
* `<L>\Interface.ini`, `<L>\Fenetres.ini`, `<L>\Fenetre_*.ini`,
  `<L>\Messages.ini`: captions and labels. A `#` inside a displayed string is a
  **line break** (`ClsIntMessage.cls:32`).

---

## 4. Language folder files (`Donnees\<L>\`)

Gameplay values in the FR and EN folders are identical (checked by diff). Only
display strings differ.

| File | Read by | Content |
|---|---|---|
| `Parametres_Globales.ini` | `ClsJeuParam.Class_Initialize` (`:690-869`) | `[Parametres_Globales]` ≈170 UI labels `EtiquetteXxx`, plus error message texts. Labels only. |
| `Chargement.ini` | `ModMain.bas:135`, `ClsJeuParam:871-882` | `[Chargement_1]` 21 window-title captions; `[Chargement_2]` 14 loading-screen captions (`Etiquette_Chargement(i)`); `[Erreur_Chargement]` |
| `Competences_Speciales.ini` | Init 0 (`:925-932`) | §4.1 |
| `Parametres_Fiefs.ini` | Init 1 (`:934-976`) | §4.2 |
| `Tresors.ini` | Init 4 (`:1025-1035`) | §4.3 |
| `Services.ini` | Init 4 (`:1037-1055`) | §4.4 |
| `Terrain.ini` | `ClsJeuMonde.cls:75` | §4.5 |
| `Partie.ini` | `ClsJeuPartie.cls:43`, `FrmParam` | mode names, list display names (§3.5) |
| `Commentaires.ini` | `ClsJeuCommentaires.cls:54` | §4.6 |
| `Messages.ini` | `ClsJeuMessages.cls:110` | HUD message log: `Afficher*` flags, `NombreMaxMessages=12`, `Duree=0.16`, positions, message templates, tips file `FichierConseil="Info_Conseils.txt"` |
| `Interface.ini`, `Fenetres.ini`, `Fenetre_*.ini` | UI | labels |
| `SavMonde.ini` | `ModSAVMonde.bas:44` | §3.13 |
| `Reseau.ini` | network | out of scope |
| `Info_*.txt` | help viewer | plain text (Lisez-moi, FAQ, Conseils = in-game tips, EditeurIA) |

### 4.1 `Competences_Speciales.ini`

`[Competences_Speciales]`:

* `NombreCompetencesSpeciales=4` (implicit conversion).
* `CompetencesSpeciales1..4`: names (quoted). FR: "Construire bâtiment",
  "Détruire bâtiment", "Coup critique", "Marchandage".

Special skill index s (0-based): 0 build, 1 demolish, 2 critical hit,
3 bargaining.

### 4.2 `Parametres_Fiefs.ini` — epochs, starting resources, epoch speeds

`[Epoques] NombreEpoques = 5`. Only `Epoque_1..5` are loaded; **`[Epoque_6]`
("Age secret") is present but ignored**, because the count is 5. That is why
buildings with `NoEpoque=6` are not playable (§8.1).

`[Epoque_k]` (k=1..NombreEpoques), loaded fields (implicit conversion):

| Key | Type | Meaning |
|---|---|---|
| `Nom` | string | epoch name |
| `Duree` | Long | duration to reach this epoch (0, 5000, 10000, 15000, 25000) |
| `BonusVieChateau` | Long | castle HP bonus. A castle sums the bonuses of epochs 0..current (`ModGenererMonde.bas:144`): 0/500/1000/1500/2500 |
| `EmpereurNomM`, `EmpereurNomF` | string | title of the ruler (Chef, Seigneur, Prince/Princesse, Roi/Reine, Empereur/Impératrice) |
| `EmpereurPrix` | Long | 1000/1500/2500/4000/6000 |

`ChateauxRessourcesDepart`, `PersosRessourcesDepart` and `PersosArgentDepart`
also appear in `[Epoque_k]` but are **not read**. Starting resources come from
the next list.

`[RessourcesDeparts] NombreRessourcesDeparts = 5`. Each `[RessourcesDepart_k]`
has `Nom`, `ChateauxRessourcesDepart`, `PersosRessourcesDepart` and
`PersosArgentDepart` (all read with Val):

| k | Nom | Castle (per resource of an available epoch) | Character resources | Character money |
|---|---|---|---|---|
| 1 | Rien | 0 | 0 | 0 |
| 2 | Peu | 0 | 5 | 100 |
| 3 | Moyen | 100 | 10 | 300 |
| 4 | Beaucoup | 300 | 15 | 1000 |
| 5 | Maximum | 500 | 20 | 1600 |

For each fief, every resource with `Ressource.Epoque ≤ fief epoch` gets
`ChateauxRessourcesDepart` (`ModMain.bas:451-457`).

`[VitesseEpoques] NombreVitesseEpoques = 5`. `VitesseEpoqueDefaut = 3` is
present but **not read**. `[VitesseEpoque_k]` has `Nom` and `Coef` (Val):
2, 1.5, 1, 0.5, 0.25 (Très lent → Très rapide).
`Parametres.VitesseEpoque_Coef()` returns the coefficient of
`VitesseEpoqueSelectionnee` (set from the combo box index at game start,
`ModMain.bas:476`).

### 4.3 `Tresors.ini`

`[Tresors] NombreTresors = 7`. Each `[Tresor_k]` has `Nom` and `ValeurMin`
(implicit Integer): 0, 100, 300, 600, 1000, 3000, 6000 (Pauvre → fabuleux
trésor). These are treasure value tiers; their use is covered by the treasure
spec.

### 4.4 `Services.ini`

`[Services] NombreServices = 3`. `[Service_k]`, k=1..3, gives **service index
s = k−1**. Index 0 is the "no service" sentinel. A commented-out `;[Service_4]`
header is followed by live keys, which are inert because `NombreServices=3`.

| Key | Read as | s=0 Aucun | s=1 Résurrection | s=2 Primes de combat |
|---|---|---|---|---|
| `Nom`, `Description` | string | Aucun | Résurrection | Primes de combat |
| `VitesseCreation` | Val | 0 | 0.25 | 0 |
| `TypeMatierePremiere` | Val (raw, **not** −1) | −1 | −1 | −1 |
| `QuantiteMatierePremiere` | Val | 1 | 1 | 1 |
| `Prix` | Val (stored in a String field!) | 0 | 200 | 0 |
| `Coefficient` | Val | 0 | 1 (share of HP restored) | 0.3 (share of bounties earned) |
| `NomCompetence` | string | Aucun | Prier | Percevoir des primes |
| `Effet` | Val (raw, **0-based effect index**) | −1 | 4 (= Effet_5 "Résurrection") | 9 (= Effet_10 "Gain d'argent") |
| `CoefExp` | Val | 0 | 4 | 0.15 |

`Batiment.Service` holds a service index s (0 = none). **Service skills** are
indexed `s−1`: skill 0 is Résurrection/Prier and skill 1 is Primes. There are
`NombreServices−1` of them; see §6.7 and §6.8 for the inconsistent file
indexing.

### 4.5 `Terrain.ini` — terrain types and world presets

`ClsJeuMonde.cls:75-119`. All keys are read with Val.

`[Terrains]`:

* `TerrainOptimise=0`: pre-render optimisation. Must be 0 under Wine.
  Written back.
* `LargeurCase=16`, `HauteurCase=16`: **tile size in world pixels**.
* `TerrainAnime=0`, `NombreEtapesAnimation=3`.
* `NombreTypesTerrains=11`: the **highest category index**. Categories are
  0..11 (12 types).

`[Terrain_c]` for c = 0..11. Note the 0-based section names, unlike Cartes.ini.

| c | Nom | CoefVitesse | Inconstructible | Other |
|---|---|---|---|---|
| 0 | Plaine | 1 | | |
| 1 | Herbe | 1 | | |
| 2 | Terre | 1 | | |
| 3 | Désert | 0.75 | | |
| 4 | Montagne | 0.5 | 1 | |
| 5 | Route | 1.25 | | |
| 6 | Muraille1 | 0.75 | 1 | |
| 7 | Muraille2 | 0.2 | 1 | |
| 8 | Muraille3 | 0.2 | 1 | |
| 9 | Muraille4 | 0.2 | 1 | |
| 10 | Neige | 0.75 | | |
| 11 | Eau | 0.5 | 1 | `Effet=159` → **object** index 158 ("Eau", a 20 s "Mouillé" temporary effect); `Animation=1`; `ImgPersoDecalageX=0`, `ImgPersoDecalageY=12` (sprite offset when standing in water) |

Fields: `Nom`, `Effet` (stored `Val−1`: an object index applied as a temporary
effect when walking on the tile; −1 none), `CoefVitesse` (movement
multiplier), `Animation`, `Inconstructible`, `ImgPersoDecalageX/Y`.

`CoefVitesse(perso, x, y)` (`:447`) applies the terrain effect only if the
character's resistance to `Objet(Effet).AttaqueResistance` (here Eau, resistance
index 6) is greater than `ResistanceMinimum`.

`[Mondes] NombreTypesMondes=5`. `[Monde_k]` (k=1..5) gives world type k−1:

* `Nom`.
* `Relief` (Single).
* `SurfaceTerrain(c)` for c=0..11 (missing = 0): the number of growth passes
  for category c.
* `Route`: the category used for roads; 0 = no roads.

| k | Nom | Relief | SurfaceTerrain non-zero | Route |
|---|---|---|---|---|
| 1 | Classique | 0.2 | (2)=13 | 0 |
| 2 | Plaines | 0.1 | (1)=13, (2)=9, (4)=6, (11)=11 | 5 |
| 3 | Désert | 0.7 | (3)=9, (11)=4 | 2 |
| 4 | Montagneux | 0.08 | (1)=8, (2)=8, (3)=8, (4)=18, (11)=6 | 5 |
| 5 | Grands Lacs | 0.2 | (1)=13, (2)=9, (4)=5, (11)=13 | 5 |

### 4.6 `Commentaires.ini` (speech bubbles)

* `[Commentaires]`:
  * `Afficher`.
  * `Duree=16` (stored /100).
  * `DureeMin`, `DureeMax`, `DureeIntervale`, `Couleur`, `DecalageX/Y`.
  * `NombreTypeMessages=53`, `NombreMaxMessagesParType=18`.
* `[Commentaires_t]`: `NombreMessages`, `Message(1..n)`.

`Message(t, perso)` (`:84`) returns `Messages[t][Int(Rnd*n)]`. It uses type
`perso.CommentairesSpecifiqueIndice` instead when the race has
`CommentairesSpecifique` (§6.9). **It consumes one `Rnd` per message.**

---

## 5. Parameter model overview

`ClsJeuParam` (one global instance, `Parametres`) holds the static game data.
Entity sets, counts in the shipped data, and their files:

| Entity (VB type) | Count | File | Key |
|---|---|---|---|
| Special skills (strings) | 4 | `<L>\Competences_Speciales.ini` | index |
| Epochs `Epoque` | 5 | `<L>\Parametres_Fiefs.ini` | index |
| Starting-resource presets `RessourcesDepart` | 5 | idem | index |
| Epoch speeds `VitesseEpoque` | 5 | idem | index |
| Resources `ParamRessources` | 26 | `Parametres_Ressources.ini` | index |
| Map resource nodes `ParamTerrainRessources` | 8 | idem | index |
| Buildings `ParamBatiments` | 47 | `Parametres_Batiments.ini` | **string ID** |
| Treasures `ParamTresors` | 7 | `<L>\Tresors.ini` | index |
| Services `ParamServices` | 3 | `<L>\Services.ini` | index |
| Item skills `ParamCompetencesObjets` | 28 | `Parametres_CompetencesObjets.ini` | **string ID** |
| Resistances `ParamResistances` | 8 | `Parametres_Resistances.ini` | index |
| Statuses `ParamStatuts` | 15 | idem | index |
| Objects `ParamObjets` | 232 | `Parametres_Objets.ini` | index |
| Decors `ParamDecors` | 5 | `Parametres_Decors.ini` | index |
| Peoples `ParamPeuples` | 13 | `Parametres_Peuples.ini` | index |
| Visual effects `ParamEffets` | 47 | `Parametres_Effets.ini` | index |
| Races `ParamRaces` | 106 | `Parametres_Races.ini` | index |
| Terrain types, world types | 12, 5 | `<L>\Terrain.ini` (in `ClsJeuMonde`) | index |

Relationships (→ means "references by index"):

```
Ressource.TypeMatierePremiere → Ressource (raw material; -1 none)
TerrainRessource.TypeRessource → Ressource      (derives Ressource.Recoltable / .TypeTerrainRessource)
Batiment.RessourceFabrique → Ressource (shop producing a resource)
Batiment.ObjetsFabriques[] → Objet (factory)         Objet.FabricationBatiment (mod) → Batiment ID
Batiment.Service → Service                            Batiment.PrixRessources[r] (per Ressource)
Batiment.CreerProprietaire → Race (owner unit spawned on construction)
Batiment.GainObjet / PerteObjet → Objet
Objet.Competence → CompetenceObjet (ID)               Objet.PrixRessources[r], GainRessource[r], BonusCompetencesRessources[r]
Objet.EffetAttaque → Effet                            Objet.AttaqueResistance → Resistance
Objet.BonusResistances[k] (per Resistance)           Objet.AttaqueCauseEffet / CreerObjet* → Objet (temp effect objects)
Objet.Invocation / LanceProjectile → Race             Objet.PerteRessourceIndice → Ressource
Objet.BonusCompetencesSpeciales[s] / Services[s-1] / Objets[c]
Peuple.MonstreIndice[] → Race (+ chance, AI file)
Race.Objet[] / Equipement[] → Objet ; common items (Races header) appended to every race
Race.Statut{} → Statut ; Statut.Resistances[k] override Race.Resistances[k]
Race.EffetAttaque / EffetMort → Effet ; Race.Tombe → Decor ; Race.AttaqueCauseEffet → Objet
Race.CoefCompSpeciales[s] / Ressources[r] / Services[s-1] / Objets[c] ; Race.Ressources[r]
Terrain.Effet → Objet ; Monde.Route → terrain category
Service.Effet → Effet (0-based!)
```

---

## 6. Parameter files (custom parser), key by key

Notation for each key:

* *stored as*: `v` (raw string), `Val`, `Val−1` (1-based → 0-based), or
  `bool` (`Val≠0`).
* *idx*: the meaning of `(j)`.
* *default*: the value when the entity is created and the key is absent.

The defaulting applied later is described in §8.

### 6.1 `Parametres_Ressources.ini` (Init 2)

Structure: `[Ressources]`, `[Ressource_1..26]`, `[Terrain_Ressources]`,
`[TerrainRessource_1..8]`. The ordering matters (§2.2): the plain headers set
the Section.

`[Ressource_r]` → resource r−1:

| Key | Stored | Meaning / default |
|---|---|---|
| `Nom`, `NomCompetence` | v | name, and the gathering skill name ("Couper du bois") |
| `Image` | v | file under `Images\Ressources\` (default: `Nom`, §8) |
| `CheminImage` | v | if non-empty, replaces the folder (`App.Path\<CheminImage><Image>.bmp`) |
| `VitesseExtraction` | Val | production/gathering speed |
| `TypeMatierePremiere` | Val−1 | raw material resource (−1 = harvested from nature / none) |
| `QuantiteMatierePremiere` | Val | raw material units per unit produced |
| `PrixVente` | Val | selling price (buying price = ×CoefficientAchat) |
| `Mangeable` | bool | edible |
| `GainVie`, `GainEnergie`, `GainMagie`, `GainMoral` | Val | effect when eaten |
| `NecessairePourConstruire` | bool | used as a building material |
| `Epoque` | Val−1 | first epoch (0-based) at which it exists |
| (derived) `Recoltable`, `TypeTerrainRessource` | | set from TerrainRessource (§8.2) |
| (derived) `NecessairePourAttaquer` | | set from objects' ammo (§8.3) |

Shipped resources (index: name, raw material ← quantity, price, epoch0):

0 Herbes medic. (—, 3, 0, edible +0.5 HP) · 1 Pommes (—, 2, 0, +1 En) ·
2 Bois (—, 4, 0, build) · 3 Planches (←2 ×1, 13, 0, build) · 4 Laine (—, 7) ·
5 Bétail (—, 8) · 6 Viande (←5 ×1, 23, +5 HP) · 7 Blé (—, 6) ·
8 Pain (←7, 16, +10 En) · 9 Orge (—, 7) · 10 Bière (←9, 21, +2 Mo) ·
11 Cuir (←5 ×0.75, 20) · 12 Pierre (—, 5, build) · 13 Briques (←12, 14, build,
ep 1) · 14 Fer (—, 8) · 15 Fonte (←14, 18, ep 1) · 16 Acier (←14 ×2.25, 30, ep 2)
· 17 Cristaux (—, 6) · 18 Fioles de soin (←0 ×3, 35, +10 HP, ep 2) ·
19 Ether (←17 ×3, 50, +10 Ma, ep 2) · 20 Parchemin (←5, 30, ep 2) ·
21 Papier (←2 ×3, 30, ep 3) · 22 Cidre (←1 ×3, 28, +3 Mo, ep 3) · 23 Or (—, 12) ·
24 Lingot d'or (←23 ×2, 34, ep 3) · 25 Soufre (—, 7).
(The file's `TypeMatierePremiere` values are 1-based, e.g. Planches has 3 = Bois.)

`[TerrainRessource_t]` → map node type t−1:

* `Nom`.
* `TypeRessource` (Val−1).
* `Quantite` (Val). It is **absent in the data, so it is 0**. The node quantity
  is therefore 0 and `ClsJeuRessources.Visible` is false, but that field is
  never used: nodes are effectively infinite.

| t | Nom | resource |
|---|---|---|
| 0 | Herbes médicinales | 0 |
| 1 | Pommier | 1 |
| 2 | Arbre | 2 |
| 3 | Gisement de pierre | 12 |
| 4 | Gisement de fer | 14 |
| 5 | Gisement de cristal | 17 |
| 6 | Gisement d'or | 23 |
| 7 | Gisement de soufre | 25 |

### 6.2 `Parametres_Resistances.ini` (Init 4)

`[Resistances]` holds the global keys and the section switch:

* `Defaut=1`: initial value of every race/status resistance.
* `Minimum=0.2`, `Maximum=3`: clamp applied to race resistances (§8.4).

`[Resistance_k]`:

* `Nom`, `NomComplet`, `Attaque` (attack-type label), `Abreviation`.
* Index k−1: 0 Physique (PH), 1 Projectiles (PR), 2 Feu (FE), 3 Froid (FR),
  4 Lumière (Lu), 5 Mort (MO), 6 Eau (EA), 7 Electricité (EL).

`[Statuts]` switches the section. `[Statut_k]` has `Nom` and
`Resistance(n)` / `Resistances(n)` = Val (n 1-based resistance). On creation,
all values are `Defaut`.

| k | Statut | overrides (resistance index n, 1-based → value) |
|---|---|---|
| 1 | Petit | 1→1.25, 2→0.75 |
| 2 | Grand | 1→0.75, 2→1.25 |
| 3 | Inflammable | 3→2 |
| 4 | Mouillé | 3→0.65, 4→1.25, 8→1.1 |
| 5 | Animal | 3→1.25, 4→0.75 |
| 6 | Mort vivant | 2→0.75, 4→0.5, 5→1.5, 6→0 |
| 7 | Esprit | 5→0.5, 6→2 |
| 8 | Esprit d'air | 2→0.5 |
| 9 | Esprit de feu | 3→0, 4→1.5 |
| 10 | Esprit d'eau | 7→0 |
| 11 | Esprit de terre | 1→0.75, 2→0.75, 3→2, 7→0.75 |
| 12 | Esprit de mort | 5→1.5, 6→0.5 |
| 13 | Dragon | 3→0.5, 4→1.5 |
| 14 | Artificiel | 1→0.5, 2→0.5, 6→0 |
| 15 | Machine | 1→0.75, 2→0.75, 7→2, 8→3, 6→0 |

Resistance values are damage multipliers (lower = more resistant). The "0 =
immunity" values are **clamped to `Minimum` = 0.2** at load (§8.4). Gameplay
code tests "resistance > ResistanceMinimum" to treat the minimum as immune (e.g.
the water effect, §4.5).

Bug to keep in mind for mods: a `[Resistance_N]` header that increases the
count resizes only the **last** race's and the last object's arrays (`:1376-1381`
index with `NombreRaces-1` / `NombreObjets-1` instead of `j`).

### 6.3 `Parametres_CompetencesObjets.ini` (Init 4)

`[CompetencesObjets]`, then `[CompetenceObjet_ID]` (string ID, §2.4):

| Key | Stored | Meaning |
|---|---|---|
| `ID` | re-targets `i` | |
| `SAVPerso` | v | key name used in `.per` saves (`CompétenceObjet25`, …). Keeps old saves compatible after re-ordering |
| `NoListe` | Val−1 | display order (default = creation index) |
| `Nom[L]` | v | name |

Shipped (index : ID : NoListe : FR name):

| idx | Nom | NoListe | | idx | Nom | NoListe |
|---|---|---|---|---|---|---|
| 0 | Sans | 0 | | 14 | Armes à feu | 14 |
| 1 | Vêtements | 1 | | 15 | Armures légères | 15 |
| 2 | Chaussures | 2 | | 16 | Armures moyennes | 16 |
| 3 | Gants | 3 | | 17 | Armures lourdes | 17 |
| 4 | Bijoux | 4 | | 18 | Boucliers | 18 |
| 5 | Outils | 5 | | 19 | Vêtements magiques | 26 |
| 6 | Armes légères | 6 | | 20 | Magie pyrotechnique | 19 |
| 7 | Armes tranchantes | 7 | | 21 | Magie nécromantique | 20 |
| 8 | Armes contondantes | 8 | | 22 | Magie rédemptrice | 21 |
| 9 | Armes d'hast | 9 | | 23 | Magie tectonique | 22 |
| 10 | Armes lourdes | 10 | | 24 | Recrutement | 27 |
| 11 | Haches | 11 | | 25 | Elixirs magiques | 23 |
| 12 | Arcs | 12 | | 26 | Equitation | 24 |
| 13 | Armes de tir | 13 | | 27 | Magie répressive | 25 |

(ID = idx+1 in the base file.)

### 6.4 `Parametres_Objets.ini` (Init 4)

`[Objets]`, then `[Objet_n]` / `[Objet_+]` → object n−1. The file comment lists
the categories.

**Defaults at creation** (`:1278-1303`):

* Allocated arrays: `PrixRessources[NbRess]`, `BonusCompetencesSpeciales[NbSpec]`,
  `BonusCompetencesRessources[NbRess]`,
  `BonusCompetencesServices[NbServices−1]` (indices 0..NbServices−2),
  `BonusCompetencesObjets[NbCompObj]`, `BonusResistances[NbRes]`,
  `GainRessource[NbRess]`, and `ApparenceDecalageX/Y[5]`,
  `ApparenceAvantPlan[5]` (indices 1..4 used).
* `EffetAttaque=-1`, `CoefXPFabrication=1`, `Invocation=-1`,
  `CreerObjet=CreerObjetAllie=CreerObjetEnnemi=-1`, `AttaqueCauseEffet=-1`,
  `AttaqueResistance=-1`.
* All other fields are 0/""/False. In particular `Competence=0` (the "Sans"
  skill), `LanceProjectile=0`, `PerteRessourceIndice=0`.

**Categorie** (`Categorie`, implicit Integer):

| Value | Slot | Value | Slot |
|---|---|---|---|
| 0 | one-handed weapon | 6 | one-hand ring |
| 1 | two-handed weapon (also used for spells, tools and summons) | 7 | two-hand ring |
| 2 | armour | 8 | arms (gloves) |
| 3 | helmet | 9 | necklace |
| 4 | shoes | 10 | mount |
| 5 | back | | |

Objects with no `Categorie` (pure temporary-effect objects such as 158 Eau,
160 Glace, 161 Poison) get 0.

| Key | Stored | Meaning |
|---|---|---|
| `Nom[L]`, `NomPluriel[L]`, `Description[L]` | v | names (`NomPluriel` defaults to `Nom`, §8) |
| `NomEffet[L]` (sets M and F), `NomEffetM[L]`, `NomEffetF[L]` | v | name of the temporary effect, per sex |
| `Image` | v | `Images\Objets\<Image>.bmp` (default `Nom`) |
| `CheminImage` | v | replaces the folder (relative to `App.Path\`) |
| `Secret` | bool | not craftable, not listed; forces `Epoque=-1` (§8.3) |
| `Unique` | bool | not saved with the world (talents, racial powers) |
| `Indestructible` | bool | cannot be destroyed or taken |
| `Inseparable` | bool | cannot be given, dropped or sold |
| `Superposable` | Val | stack size |
| `Empereur` | bool | emperor-only object |
| `Categorie` | implicit Int | slot, see above |
| `Competence` | **skill ID** → index (§2.4) | item skill governing the object |
| `PrixRessources(r)` | Val | craft cost in resource r−1 |
| `Epoque` | Val (1-based, kept raw) | minimum epoch; recomputed (§8.3) |
| `DureeFabrication` | Val (Integer) | craft time |
| `CoefXPFabrication` | Val | default 1 |
| `FabricationBatiment` | building ID | **appends** this object to that building's `ObjetsFabriques` (mods) |
| `EquipementRace` | race ref (§2.3) | appends to `Race.Equipement` (mods only) |
| `TresorRace` | race ref; absolute `0` → −1 = **all races** (common items list) | appends to `Race.Objet` or the common list (mods only) |
| `PrixVente` | Val (Integer) | price |
| `BonusVie/Energie/Magie/Moral/Attaque/Defense` | Val (Integer) | stat bonuses while equipped |
| `BonusPorteeAttaque` | Val | range bonus (pixels) |
| `LanceProjectile` | race ref (`+`, or `v−1`) — **only if `Jeu.AfficherProjectiles`** | race spawned as the projectile (races 82-89 are projectiles). 0 means "no projectile" (race 0 can never be one) |
| `ProjectileSupplementaire` | Val | extra projectiles |
| `EffetAttaque` | effect ref (`+`/`v−1`) | hit visual |
| `AttaqueResistance` | Val−1 | resistance the attack is checked against |
| `BonusResistance(k)` | Val | additive change to resistance multiplier k−1 while equipped/active (may be negative; how it combines is in the combat spec) |
| `SurfaceAttaque` | Val | area-of-effect radius |
| `AttaqueAutomatique` | bool | attacks continuously |
| `AttaqueCauseEffet` | object ref (§2.3) | temporary-effect object applied on hit |
| `BonusDeplacement` | Val | speed bonus for non-flyers |
| `BonusVitesse` | Val | speed bonus for everyone |
| `BonusVitalite` | Val | vitality % bonus |
| `BonusCompetencesSpeciales(s)` | Val → `[s−1]` | special-skill bonus |
| `BonusCompetencesRessources(r)` | Val → `[r−1]` | |
| `BonusCompetencesServices(n)` | Val → `[n−2]` (n = Service_N number, so `(2)` = Résurrection) | |
| `BonusCompetencesObjets(c)` | Val → `[c−1]` (item-skill **index**+1) | |
| `BonusArmure`, `BonusMaxRessources` | Val | |
| `BonusVitesseRegenerationVie/Energie/Magie/Moral` | Val | |
| `PerteVie/Energie/Magie/Moral` | Val | cost per use/attack (e.g. mana) |
| `PerteArgent` | Val (Long) | money cost |
| `PerteRessourceIndice` | Val−1 | ammunition resource |
| `PerteRessourceQuantite` | Val | ammo per shot; >0 marks the resource `NecessairePourAttaquer` |
| `Poison` | Val | |
| `ApparencePerso` | Val | (legacy) |
| `CheminApparence` | v | adds an appearance folder (`AffApparence.Ajouter_Dossier`); not stored on the object |
| `FichierApparence` (M and F), `FichierApparenceM`, `FichierApparenceF` | v | overlay sprite **file name with extension** (e.g. `hut.bmp`), searched across all appearance folders (§11.4). Non-empty M ⇒ `Objet_ChangerApparence` |
| `NumeroApparenceM/F` | Val | overwritten at runtime by the sprite index lookup |
| `ApparenceLargeur`, `ApparenceHauteur` | Val | overlay frame size |
| `ApparenceDecalageX{H,D,B,G}`, `ApparenceDecalageY{H,D,B,G}` | Val → `[1..4]` | per-direction offset (H=1 up, D=2 right, B=3 down, G=4 left) |
| `ApparenceAvantPlan{H,D,B,G}` | **`Not CBool(Val)`** → `[1..4]` | inverted: file `1` stores False |
| `ChangerApparencePerso` | bool | metamorphosis |
| `Utilisable` | bool | used (cast) rather than equipped |
| `UtilisationUnique` | bool | consumed on use |
| `UtilisationMort` | bool | usable while dead |
| `PorteeUtilisation` | Val | radius of effects |
| `Experience` | Val | XP per use (`1,75` → **1**, §9) |
| `Invocation` | race ref (§2.3) | summoned race |
| `InvocationIA_Fichier` | v | AI file of the summon (name, found under `IAs\`) |
| `Quantite`, `QuantiteMax` | Val | count / threshold (summons, tax rate, …). **Lowercase `quantite` is ignored** |
| `GainVie/Energie/Magie/Moral` | Val | instant gain on use |
| `GainArgent` | Val | |
| `Impot` | bool | tax spell (`Quantite` = share collected) |
| `GainRessource(r)` | Val → `[r−1]` | generated resources |
| `Resurrection` | Val (Long) | number of people resurrected |
| `Berserk` | Val | |
| `SelfResurrection` | bool | |
| `CreerObjet`, `CreerObjetAllie`, `CreerObjetEnnemi` | object ref (§2.3) | object or temporary effect created on self / nearby allies / nearby enemies |
| `Duree` | Val (Long) | temporary effect duration; 0 = permanent object; −1 is used by orbs |
| `DureeRecharge` | Val — **only if `Jeu.Regle2≠0`** | cooldown |
| `DureeRechargeAttaque` | Val | attack cooldown |
| `RechargesMultiples` | Val | charges |
| `EffetAnnuleRestaureObjet`, `EffetAnnulationImpossible`, `EffetAnnulationMort` | bool | effect cancellation rules |
| `UtilisationAutomatique` | bool | auto-used at game start |
| `Monture` | bool | mount |
| `Volant` | bool | flying |
| `PrixTalent` / `PrixTalents` | Val | talent-point cost (treasure/talent system) |

Object index landmarks used by other data:

* 110-182 are mostly `Secret`/`Unique` talent items (orbs, racial powers)
  priced in `PrixTalents`. 112-117, 127, 132, 135, 137, 140, 142 and 143 are
  orbs.
* 158 "Eau" (water effect), 160 "Glace", 161 "Poison affaiblissant".
* 169-171 horses (mounts); 172 "Ailes".
* 183 "Couronne".
* 184-197 recruits and apprentices (`Invocation=1` = race 0 Humain, with
  apprentice AI files).
* 199 "Impôt".
* 231 "Dragon bleu" (mount).

### 6.5 `Parametres_Decors.ini` (Init 4)

`[Decors]` switches the section; it is required, because `[Decor_N]` does not
set it. `[Decor_d]` → decor d−1:

* `Nom`, `Image` (file under `Images\Decors\`, or `.`-prefixed absolute path).
* `Largeur`, `Hauteur` (frame size).
* `NombreEtapeAnimation` (frames, horizontally), `Duree`.

| d | Nom | Image | Size | Anim | Duree |
|---|---|---|---|---|---|
| 0 | Pont | Pont | 71×88 | | |
| 1 | Drapeau | Drapeau | 31×81 | | |
| 2 | Fontaine | Fontaine | 89×86 | 4 | |
| 3 | Tombe | Pierre tombale3 | 23×17 | | 60 |
| 4 | Tombe2 | Croix | 14×22 | | 60 |

### 6.6 `Parametres_Peuples.ini` (Init 5)

`[Peuples]` has `CheminImageDefaut = Images\Batiments\Chateaux\`. That key can
appear in any Peuple section; it sets a global. `[Peuple_p]` → people p−1:

| Key | Stored | Meaning |
|---|---|---|
| `Nom[L]` | v | |
| `Image` | v | castle sprite name |
| `CheminImage` | v | folder; empty ⇒ `CheminImageDefaut` (applied right after Init 5, so **not for mod peoples**) |
| `NombreMonstres` | Val | **ReDim Preserve** of the three monster arrays to n |
| `MonstreIndice(j)` | race ref (§2.3) | race of the j-th monster type |
| `MonstreChance(j)` | Val | draw probability |
| `MonstreFichierIA(j)` | v | AI file name |
| `NomChateau[L]` | v | castle name |
| `ChateauMaxVie`, `ChateauLargeur`, `ChateauHauteur`, `ChateauEntreePositionX/Y` | Val | castle stats: all 2000 HP and 128×128 with entrance (0,0), except Sylvestres, which is 128×141 with entrance (1,49) |

Monster selection (`Tirer_Monstre_Aleatoire`, `ModGenererMonde.bas:65`) walks the
list in order: `If Rnd < chance(j) Then race = indice(j)`. See the game-setup
spec for the details.

| p | Nom | Image | monsters (race index 0-based : chance) |
|---|---|---|---|
| 0 | Humains | Humains | none (citizens) |
| 1 | Animaux | Animaux | 6:0.1 (AI Eleveur.ia), 12:.75, 13:.5, 14:.5, 15:.5, 16:.5 |
| 2 | Bêtes sauvages | BetesSauvages | 17:.75, 18-21:.5 |
| 3 | Morts-vivants | MortsVivants | 10:0.1 (Necromancien offensif.ia), 22:.75, 23-26:.5 |
| 4 | Sorciers | Sorciers | 27:.75, 28-31:.5 |
| 5 | Hommes-bêtes | HommesBetes | 9:0.1, 32:.75, 33-36:.5 |
| 6 | Technologistes | Technologistes | 37:.75, 38-41:.5 |
| 7 | Sylvestres | Sylvestres | 76:0.3, … |
| 8 | Pillards | Pillards | … |
| 9 | Impériaux | Imperiaux | … |
| 10 | Démons | Demons | … |
| 11 | Elémentaux | Elementaux | … |
| 12 | Dragons | Dragons | … |

(The file's `MonstreIndice` values are 1-based.) In the UI the people list is
followed by "Aléatoire", so `PeupleFief=14` means random.

### 6.7 `Parametres_Effets.ini` (Init 6)

`[Effets]` holds the special-effect globals (all `Val−1`):

* `EffetClique=38` (click marker = effect 37).
* `EffetMaisonEnFeu=4` (effect 3, Mur de feu).
* `EffetMaisonDetruite=24`, `EffetChateauDetruit=24` (Destruction).
* `EffetNouvelObjectif=39` (Courrier).
* `EffetDureeRestante=44`, `EffetDureeRestante2=47` (clocks).

`[Effet_e]` → effect e−1:

* `Nom[L]` (only `Nom`; the localised suffix is **not** supported for effects).
* `Fichier`: sheet under `Images\Effets\`, or `.`-prefixed path. Empty = no
  sprite, sound only (Clique, Miaulement).
* `Largeur`, `Hauteur`: frame size.
* `NombreEtapes`: frames in one row.
* `DureeEtape` (Double; 0 → 1 after Init 6).
* `Son`: file in `Sons\`.

47 effects: 0 Coup simple, 1 Lame, 2 Flamerole, 3 Mur de feu, 4 Résurrection,
5 Jet de flammes, 6 Coup de griffes, 7 Explosion, 8 Pointe, 9 Gain d'argent,
10 Brasier, 11 Choc, 12 Sphère noire, 13 Tornade, 14 Gaz empoisonné,
15 Coup de hache, 16 Guérison, 17 Pierres, 18 Explosion 2,
19 Grands coups de griffes, 20 Crâne, 21 Colonne de feu, 22 Etincelles vertes,
23 Destruction, 24 Cercle bleu, 25 Dévorer, 26 Explosion 3, 27 Dépense,
28 Eclair, 29 Tir, 30 Flamme, 31 Effet célérité, 32 Bonus, 33 Outil,
34 Sphère, 35 Sphère 2, 36 Explosion 4, 37 Clique, 38 Courrier, 39 Glace,
40 Lame pure, 41 Génération, 42 Musique, 43 Horloge rouge, 44 Miaulement,
45 Ténèbres, 46 Horloge blanche.

### 6.8 `Parametres_Races.ini` (Init 7)

The `[Races]` header (pseudo index −1) holds:

* `ChancesRaceAleatoire = 0.3` and `JouerToutesLesRaces = 0` (globals).
* `NombreObjets=14` and `Objet(1..14)`: the **common items** (object indices
  112, 113, 114, 115, 117, 127, 116, 153, 154, 179, 140, 122, 141, 143, after
  −1). They are appended to every race in §8.4.

**Defaults at creation** (`:1334-1355`):

* `DossiersCategoriesF/M` sized 1.
* `Vitesse=-1` (sentinel).
* Coefficient arrays: `CoefCompSpeciales[NbSpec]`, `CoefCompRessources[NbRess]`,
  `CoefCompServices[NbServices]` (indices 0..NbServices−1),
  `CoefCompObjets[NbCompObj]`.
* `Resistances[k] = ResistanceDefaut`, `Ressources[NbRess]`.
* Empty `Statut` and `ObjetRace` collections.
* `Tombe=-1`, `AttaqueCauseEffet=-1`, `EffetMort=-1`.
* **`EffetAttaque` and `AttaqueResistance` default to 0**, unlike objects: effect
  0 "Coup simple", resistance 0 "Physique".

| Key | Stored | Meaning |
|---|---|---|
| `Nom[L]` (M and F), `NomM[L]`, `NomF[L]` | v | singular names |
| `NomPluriel[L]` (M and F), `NomMPluriel[L]`, `NomFPluriel[L]` | v | plural (bestiary sort key = `NomMPluriel`) |
| `NomAleatoire` | bool | NPCs get random first names (§3.3) instead of the race name |
| `Jouable` | bool | selectable by players (races 0-11) |
| `Description[L]` | v | |
| `RestrictionMaculin` (sic) | bool | **male forbidden**. The spellings `RestrictionMasculin` / `RestrictionMasulin` are **ignored** (§9) |
| `RestrictionFeminin` | bool | female forbidden |
| `Largeur`, `Hauteur` | Val | sprite frame size (default 24×32) |
| `Portrait` | v | image path relative to `App.Path\` (e.g. `Images\Portraits\Humain.gif`) |
| `NombreCategoriesApparences` | Val | number of layered appearance categories; ReDims the folder arrays |
| `DossiersCategoriesF(j)`, `DossiersCategoriesM(j)`, `DossiersCategories(j)` (both) | v | folder of category j (relative to `Images\Apparences\`, or `.\…` absolute: then also registered as an appearance folder) |
| `DefinirApparence` | bool | true ⇒ the sprite is a whole sheet chosen at random from `CheminApparence`; false ⇒ layered appearance from categories |
| `CheminApparenceM`, `CheminApparenceF`, `CheminApparence` (both) | v | sheet folder under `Images\Persos\`; a leading `.` ⇒ the rest is relative to `App.Path`, `CheminAbsolu=True`, folder registered |
| `ApparenceVerouillee` | bool | appearance never changes (equipment overlays not drawn) |
| `Vie`, `Energie`, `Magie`, `Moral`, `Attaque`, `Defense` | Val (Long) | base characteristics (0 ⇒ `Persos*` default) |
| `CoefVie/Energie/Magie/Moral/Attaque/Defense` | Val | growth coefficients (0 ⇒ 1) |
| `VitesseRegenerationVie/Energie/Magie/Moral` | Val | regeneration per tick |
| `Parole` | bool | can talk (comments) |
| `Experience` | bool | gains experience |
| `Equiper` | Val (Integer, used as bool) | can equip objects |
| `CoefCompSpeciale(s)` | Val → `[s−1]` | |
| `CoefCompRessource(r)` | Val → `[r−1]` | |
| `CoefCompService(n)` | Val → `[n−1]`. **n=1 is the Résurrection service skill**: a different convention from objects' `BonusCompetencesServices` | |
| `CoefCompObjet(c)` | Val → `[c−1]` | |
| `Volant` | bool | flying |
| `CommentairesSpecifique` | Val−1 | uses one comment type for all speech (−1 = normal) |
| `BonusMaxRessources` | Val | carrying bonus (may be negative) |
| `PorteeAttaque` | Val | natural range |
| `EffetAttaque` | `+` / Val−1 | natural attack visual |
| `AttaqueCauseEffet` | Val−1 | object applied on hit |
| `AttaqueResistance` | Val−1 | resistance type of the natural attack |
| `Armure` | Val | |
| `Vitesse`, `Acceleration` | Val | movement (−1 / ≤0 ⇒ defaults) |
| `Pietinement` | Val | trample |
| `Inattaquable` | bool | cannot be targeted (projectiles) |
| `AutoResurrection` | bool | |
| `IA_Pacifique`, `IA_Manger`, `IA_Temeraire`, `IA_Protecteur`, `IA_Egoiste`, `IA_Incommandable`, `IA_Selectionner` | bool | AI flags |
| `IA_Berserk` | Val | AI mode (e.g. 5 = defends its building, 6 = market caravan) |
| `IA_Fichier` | v | default AI file |
| `Infatigable` | bool | forces energy and moral regeneration to 0.01 (§8.4) |
| `Kamikaze` | bool | dies on hit (projectiles, mines) |
| `RestaurationComplete` | bool | fully restored when entering the castle |
| `Argent` | Val | starting money |
| `Resistance(k)`, `Resistances(k)` | Val → `[k−1]` | then statuses override and the clamp applies (§8.4) |
| `Ressource(r)`, `Ressources(r)` | Val → `[r−1]` | starting resources |
| `DepartSansRessource`, `DepartSansRessources` | bool | ignore the game's starting resources |
| `NombreObjets` | Val | **ReDim (no Preserve)** `Objet[]`; at the `[Races]` level it sizes the common list |
| `Objet(j)` | object ref (§2.3) | racial treasure item j. Also added to the `ObjetRace` collection (racial-only list) |
| `NombreEquipement` | Val | ReDim `Equipement[]` |
| `Equipement(j)` | object ref | starting equipment |
| `Statut` | Val; `0` clears the collection, else adds `v−1` | statuses (repeatable key) |
| `IndiceBestiaire` | raw / §2.3 | bestiary number saved in `.per` files (1-based) |
| `UtilisationMonture` | bool | can ride mounts |
| `Tombe` | decor ref | grave decor left on death |
| `EffetMort` | `+` / Val−1 | death visual |

Races (0-based) in the shipped data:

* 0-11 playable: Humain, Nain, Elfe, Demi-Elfe, Colosse, Kobold, Félin, Fée,
  Drake, Garou, Ombre, Celeste.
* 12-16 animals (Poule, Vache, Bélier, Chien, Cheval).
* 17-21 wild (Rat, Loup, Tigre, Lion, Tigre Blanc).
* 22-26 undead (Squelette, Zombie, Fantôme, Spectre, Vampire).
* 27-31 sorcerers (Mannequin, Familier, Acolyte, Golem de pierre, Archimage).
* 32-36 beastmen (Crapautaure, Porcotaure, Homme-singe, Minotaure, Horreur).
* 37-41 technologists (Automate, Robot, Savant, Char d'assaut, Aéronef).
* 42-46 raiders (Pirate, Viking, Amazone, Assassin, Chevalier noir).
* 47-51 imperial (Soldat, Chevalier, Cavalier, Capitaine, Général).
* 52-56 demons (Succube, Démon, Démon majeur, Arch-démon, Démon-roi).
* 57-61 elementals (air, eau, terre, feu, mort).
* 62-67 dragons (Dragon volant, Bébé dragon, Dragon vert, Dragon bleu,
  Dragon ancien, Squelette dragon).
* 68-75: Ange, Chasseur d'âmes, Golem de cristal, Golem de fer, Golem d'acier,
  Rebelle, Paladin, Chauve souris.
* 76-80: Ours, Dryade, Licorne, Golem d'or, Centaure.
* 81-89 projectiles and summons: Flammerole, Mine, Boule de feu, Flêche,
  Boulet de canon, Pierre, Balle, Esprit de vie, Orbe de givre.
* 90-95: Chat, Renard, and the major elementals (air, eau, terre, feu).
* 96 Garde (owner of the Tour de défense), 97 Caravane (owner of the Marché).
* 98-105: Squelette archer, Squelette guerrier, Golem Excavateur, Fusilier,
  Hache, Cuirassier, Arc ange, and "Arc ange" again (index 105, actually the
  storm summoned by object 228 "Tempête").

### 6.9 `Parametres_Batiments.ini` (Init 3)

`[Batiments]` has `CheminImageDefaut = Images\Batiments\` (a global, settable in
any building section). `[Batiment_ID]` (string ID, §2.4):

| Key | Stored | Meaning / default |
|---|---|---|
| `ID` | re-targets | |
| `Nom[L]` | v | |
| `FemininFR`/`FemininEN` (only the suffixed form) | bool | grammatical gender of the name |
| `Image` | v | default `Nom` |
| `CheminImage` | v | default `CheminImageDefaut`; a leading `.` ⇒ the first **2** characters are stripped |
| `ImageBouton` | v | construct button, default `Image` (`Images\Boutons\Construire\<ImageBouton>.bmp`) |
| `CheminImageBouton` | v **minus its first character** | custom button folder |
| `Metier[L]` (M and F), `MetierM[L]`, `MetierF[L]` | v | owner's job title |
| `Description[L]` | v | |
| `Empereur` | bool | only the emperor can build it (`Prix += 100000` when auto-priced) |
| `Limite` | Val | max per fief (0 = unlimited) |
| `CreerProprietaire` | race ref (§2.3) | on construction, a unit of this race owns it (−1 = the builder) |
| `Roi`, `RestrictionRoi` | bool | building it makes you king / king-only (`RestrictionRoi` is commented out in the data) |
| `GainObjet`, `PerteObjet` | Val−1 | object granted to the owner / removed from the owner (−1 none) |
| `Tir` | bool | can shoot from inside |
| `Marche` | bool | resource market |
| `Prix` | Val | total value (sort key); if 0, computed (§8.1) |
| `PrixArgent` | Val | money cost |
| `PrixRessources(r)` | Val → `[r−1]` | resource cost |
| `MaxVie` | Val | HP |
| `CoefVitesseConstruction` | Val | 0 ⇒ 1 |
| `RessourceFabrique` | Val (1-based in the file), then **−1 in §8.1** | resource produced (shops) |
| `NombreObjetsfabriques` (lowercase f) | Val; ReDim Preserve | |
| `ObjetsFabriques(j)` | Val−1 | crafted objects (factories) |
| `Magasin` | bool | resource shop |
| `Fabrique` | bool | object factory |
| `Service` | Val | service index (§4.4), 0 none |
| `MaxStock` | Val | stock capacity |
| `Niveau` | Val | importance for the AI |
| `NoEpoque` | Val (1-based) | epoch needed |
| `Largeur`, `Hauteur` | Val | size (= sprite frame) |
| `EntreePositionX/Y` | Val | entrance offset |
| `NombreEtapeFabrication` | Val | construction stages = sprite rows − 1 (§11.3) |
| `NombreEtapeAnimation` | Val | animation frames (columns) |
| `ApparencePerso` | Val | unused |

Building index : ID : name (NoEpoque, role):

* 0:1 Ferme de blé (1, makes 7 Blé) · 1:2 Ferme d'orge (9) · 2:3 Scierie (3) ·
  3:4 Bergerie (4) · 4:5 Elevage (5) · 5:6 Boucherie (6) · 6:7 Boulangerie (8) ·
  7:8 Taverne (10).
* 8:9 Magasin de vêtements (factory) · 9:10 Atelier (factory) ·
  10:11 Boutique (factory) · 11:12 Tannerie (makes 11) · 12:13 Cordonnerie ·
  13:14 Ganterie.
* 14:17 Ecurie (2) · 15:18 Carrière de pierre (2, makes 13) ·
  16:19 Baraquement (2, service 2) · 17:20 Fonderie (15) · 18:21 Forge ·
  19:22 Archerie · 20:23 Armurerie.
* 21:24 Aciérie (3, 16) · 22:25 Parcheminerie (20) · 23:26 Eglise (3, service 1)
  · 24:27 Laboratoire (18) · 25:28 Laboratoire d'éther (19) ·
  26:29 Guilde de magie · 27:30 Ecole de pyromagie · 28:31 Nécropole ·
  29:32 Ecole aquavitae.
* 30:33 Papeterie (4, 21) · 31:34 Orfèvrerie (24) · 32:35 Cidrerie (22) ·
  33:36 Bijouterie · 34:37 Arsenal · 35:38 Atelier tectonique ·
  36:39 Guilde du chaos · 37:40 Guilde de l'ordre ·
  38:41 Laboratoire d'élixirs.
* NoEpoque 6, so **not playable**: 39:42 Forge impériale ·
  40:43 Armurerie impériale · 41:44 Quartier général · 42:45 Nid de dragons.
* Secondary buildings:
  * 43:TOURDEFENSE (2; owner race 96; Tir; PrixArgent 1200; CoefVitesse 0.25).
  * 44:MARCHE (2; owner race 97; Marche; CoefVitesse 0.2).
  * 45:PALAIS (1; Empereur; Limite 1; `GainObjet = PerteObjet = 200`, i.e.
    object 199 "Impôt"; a factory of 11 imperial recruits and mounts).
  * 46:CASERNE (6, service 2, not playable).

(In the list above the role number is the 0-based resource index; the file
values are 1-based.)

---

## 7. Runtime parameters outside `ClsJeuParam`

Terrain types and world presets are held by `ClsJeuMonde` (§4.5, §10). Map
colour tables are held by `ClsJeuCartes` (§3.7). Combat coefficients are module
variables of `ModCombat` (§3.6). Game rules and speed are held by `ClsJeu`
(§3.4). Option lists (sizes, difficulties, resurrections) live only in
`FrmParam` arrays (§3.5).

---

## 8. Post-load derivations (`Verification_Donnees`, `ClsJeuParam.cls:2196`)

They run once, after all mods.

### 8.1 Buildings

For each building i:

1. If `Prix = 0`: `Prix = Σ_r PrixRessources[r] × Ressource[r].PrixVente`. The
   Long accumulator is re-rounded at each step. If `Empereur`, add 100000.
2. **Always** `Prix += PrixArgent`.
3. `Image ||= Nom`; `CheminImage ||= CheminImageDefaut`; `ImageBouton ||= Image`;
   `CoefVitesseConstruction ||= 1`.
4. `RessourceFabrique -= 1` (to 0-based; −1 = none).
5. `Jouable = (NoEpoque ≤ NombreEpoques)`; count `NombreTypeBatimentsJouables`.

**Display / AI order `BatimentsTri`** (a selection sort over the list `Temp` of
indices in creation order, `:2230-2245`). Repeat until `Temp` is empty: start
with `k = 1`. For `j = 1..|Temp|`, set `k = j` when

```
( j = 1  OR  (Prix[Temp j] < Prix[Temp k] AND Ep[Temp j] <= Ep[Temp k])  OR  Ep[Temp j] < Ep[Temp k] )
AND ( NOT Empereur[Temp j]  OR  Ep[Temp k] = NombreEpoques + 1 )
```

Then append `Temp k` to the sorted list and remove it. (`Ep` = `NoEpoque`.)
`Batiment_Tri(n)` gives the n-th building; `Batiment_TriInverse` gives the
inverse.

### 8.2 Resources

* `Image ||= Nom`.
* For each TerrainRessource t: `Ressource[TypeRessource].Recoltable = True` and
  `.TypeTerrainRessource = t` (the last t wins).

### 8.3 Objects

For each object i, in index order:

1. `NomPluriel ||= Nom`; `Image ||= Nom`.
2. If `PerteRessourceQuantite > 0`:
   `Ressource[PerteRessourceIndice].NecessairePourAttaquer = True`.
3. If `Secret`: `Epoque = -1`. Otherwise:
   * Find the **first building in index order** with `Fabrique = True` whose
     `ObjetsFabriques` contains i. If one exists:
     `Epoque = max(Epoque, building.NoEpoque)`. The loops are broken by
     assigning the loop variables, so only that first factory counts, not the
     earliest one.
   * Then, for each resource r with `PrixRessources[r] > 0`: if
     `Ressource[r].Epoque > Epoque − 1`, set `Epoque = Ressource[r].Epoque + 1`.
     This converts the 0-based resource epoch to the 1-based object epoch;
     effectively it takes the max.
   * Objects not crafted anywhere keep their file `Epoque` (default 0), adjusted
     by their resources.
4. Then, for every building with objects, the `ObjetsFabriques` list is
   **bubble-sorted by `Objet.Epoque` ascending** (stable: swaps only on strict
   `<`).

### 8.4 Races

For each race:

* Count `NombreRacesJouables`.
* Size and stat defaults: `Largeur ||= PersosLargeurDefaut`,
  `Hauteur ||= PersosHauteurDefaut`; `Vie`, `Energie`, `Magie`, `Moral`,
  `Attaque`, `Defense` `||= Persos*` (0 ⇒ default).
* `Coef*` and every `CoefComp*[k]` (for CoefCompServices only indices
  0..NbServices−2) `||= 1`.
* If `Infatigable`: `VitesseRegenerationEnergie = VitesseRegenerationMoral = 0.01`,
  overriding the file.
* If `Vitesse < 0`: `Vitesse = PersosVitesse`. If `Acceleration ≤ 0`:
  `Acceleration = PersosAcceleration`. (A race can set `Vitesse=0`
  explicitly.)
* **Common items**: `Objet[] = race items ++ common items`. The `ObjetRace`
  collection keeps only the racial ones that came from `Objet(j)` keys.
* **Statuses**: for each status in insertion order and each resistance k, if
  `Statut.Resistances[k] ≠ ResistanceDefaut` then `Race.Resistances[k] =` that
  value. This overrides explicit race values; later statuses win.
* **Clamp** every resistance to `[ResistanceMinimum, ResistanceMaximum]`
  (0.2..3).

Finally `Bestiaire_Tri_Alphabetique` bubble-sorts race indices by `NomMPluriel`
using **binary (UTF-16 code unit) comparison**: case-sensitive, accented
capitals after `z`.

---

## 9. Data quirks that must be reproduced

These are faithful behaviours of the shipped data under the VB loader. Do not
"fix" them, or gameplay will differ.

1. **Ignored misspelled keys** (case-sensitive `Select Case`):
   * `RestrictionMasculin` (races 44 Amazone, 75 Chauve souris, 77 Dryade,
     78 Licorne, 82 Mine, 83 Boule de feu, 84 Flêche, 86 Pierre, 87 Balle,
     89 Orbe de givre, 102 Hache) and `RestrictionMasulin` (races 81
     Flammerole, 105) are ignored. **These races can spawn male**, e.g.
     Amazones. Only `RestrictionMaculin` works.
   * `quantite` (lowercase) on objects 184-189, 192, 197, 227 (the apprentices
     and Milicien) is ignored, so `Quantite = 0`.
   * `NomPlurielRN` (object 200), `NomplurielEN` (object 213) are ignored.
2. **Decimal commas** give truncated values: `Experience = 1,75` (objects 148
   Célérité and 179 Lévitation) → 1; race 104 `CoefVie = 3,375` → 3.
3. Epoch 6 is defined but `NombreEpoques=5`, so the 4 epoch-6 buildings are
   unplayable. `VitesseEpoqueDefaut` and the per-epoch starting resources are
   not read.
4. "Immunity" status values (0) become 0.2 after clamping.
5. `TerrainRessource.Quantite` is missing, so it is 0 (infinite nodes).
6. The Service `Effet` field is a 0-based effect index, while every other
   effect reference is 1-based.
7. Race `EffetAttaque` and `AttaqueResistance` default to 0 (Coup simple,
   Physique), while the object defaults are −1.
8. Object `ApparenceAvantPlan*` is stored inverted.
9. Service-skill indices: race `CoefCompService(n)` → `[n−1]`, object
   `BonusCompetencesServices(n)` → `[n−2]`.
10. `LanceProjectile` depends on `Jeu.ini AfficherProjectiles`, and
    `DureeRecharge` on `Regle2`. These options are applied **at load time
    only**.
11. Shipped player mods sit in `Mods\Vrac\*\` (depth 2), so they are not
    loaded.
12. The names lists and random race/appearance draws use banker's rounding
    (§3.3, §10.4).

---

## 10. Maps: `Cartes\<name>\` and how a map becomes a world

### 10.1 Discovery and selection

`ClsJeuCartes.Charger_Cartes` (`:129`) lists the sub-directories of
`App.Path\Cartes\` whose name contains no `.`, in `Dir` order. The display name
is the folder name, e.g. `(04) Cross rivers`. Shipped maps:

| Map | Terrain size (tiles) | Castles | Ressources.bmp | Informations NombreRessources |
|---|---|---|---|---|
| (02) Confrontation | 128×128 | 2 | no | — |
| (02) Duel | 100×75 | 2 | yes | 58 |
| (03) Triangle | 192×192 | 3 | yes | 140 |
| (04) Cross rivers | 256×256 | 4 | yes | 155 |
| (04) Delta | 256×256 | 4 | no | — |
| (04) Team Battle | 900×675 | 4 | no | — |
| (06) Dunes | 384×384 | 6 | yes (9-colour palette, bits offset 90) | 96 |
| (08) Surdira | 252×400 | 8 | no | — |
| (10) Continent | 640×640 | 10 | no | — |

No map ships a `Chateaux.bmp`. Castle positions come only from
`Informations.ini`.

Selecting a map (`FrmParam.LstCartes_Click`, `:2555`):

* loads the preview: `Apercu.bmp`, falling back to `Terrain.bmp`;
* loads the description text: `Description<Suffixe>.txt`, falling back to
  `Description.txt`;
* forces the fief count to `Definir_Nombre_Chateaux`.

Starting the game calls `CmdChargerCarte_Click` (`:2266`): `Charger_Monde`,
then `Charger_Chateaux`, then `Charger_Ressources`. Then
`Placer_Chateaux False` applies people stats and the epoch HP bonuses to castles
(`ModGenererMonde.bas:118`), and `Cartes.Chargee = True` (`ModMain.bas:467-471`).

### 10.2 Files in a map folder

| File | Required | Use |
|---|---|---|
| `Terrain.bmp` | yes | terrain category per pixel = per tile |
| `Chateaux.bmp` | only if `Informations.ini` lacks castles | white pixels (255,255,255) = castle centres |
| `Ressources.bmp` | no | non-black pixels = resource nodes |
| `Informations.ini` | no (generated) | cache, see §10.5 |
| `Apercu.bmp` | no | preview (UI) |
| `Carte.bmp` | no | minimap image, nominally 288 px |
| `Description.txt`, `DescriptionFR.txt`, `DescriptionEN.txt` | no | text shown next to the preview. Lines are joined with `Chr(10)&Chr(13)`; the leading 2 characters are removed |

`Cartes\Comment creer une carte.txt` documents the format.
`References terrain.bmp` and `Reference ressources.bmp` are palettes for map
authors. All three bitmaps must have equal dimensions.

### 10.3 BMP decoding (exact behaviour of `Charger_Monde`, `:228`)

* Read the 14-byte `BITMAPFILEHEADER` (`Offset` = bfOffBits) and the 40-byte
  `BITMAPINFOHEADER`. `Width` and `Height` are the map size in tiles.
* **`Width mod 4 ≠ 0` ⇒ fatal error** ("dimensions must be multiples of 4").
  This guarantees no row padding for 8 bpp and 24 bpp.
* `biCompression ≠ 0` ⇒ fatal error.
* Height is assumed positive (bottom-up).
* **24 bpp**: pixels are read sequentially as B, G, R from `Offset`.
* **otherwise (8 bpp assumed)**:
  * read **exactly 256** BGRA palette entries starting at file offset 54,
    regardless of `biClrUsed`;
  * then read 1 byte per pixel from `Offset` to EOF (`Input(LOF)` after
    positioning at `Offset`). It returns the remaining bytes.
  * For `Dunes\Ressources.bmp`, palette entries 9..255 contain pixel bytes, but
    only indices 0..8 occur, so this is harmless.
* Pixel order: the first stored row is the bottom row. `x` runs 0..W−1. The row
  counter starts at `y = H−1` for terrain and decrements after each row. Terrain
  stops when `y < 0`.

`Type_Terrain(R,G,B)` (`:696`) returns the first k with exact equality to
`Cartes.ini [Terrain_{k+1}]`, **otherwise 0 (Plaine)**.

The terrain grid representation (`ClsJeuMonde`):

* `Monde.Largeur = W`, `Monde.Hauteur = H` in tiles. Tiles are
  `LargeurCase × HauteurCase = 16×16` world pixels.
* `NombreCases = (W+1)·(H+1)`. The arrays are resized with `ReDim Preserve`, so
  entries beyond `W·H` keep values from a previously loaded or generated world.
* Two parallel `Long` arrays: `TerrainCategorie[]` (terrain type 0..11) and
  `TerrainApparence[]` (autotile shape 0..15).
* **Linear index = `y·W + x`.** The stride is W, even though the allocation is
  (W+1)(H+1).
* World pixel → tile: `TerrainIndice(X, Y) = CLng(X/16) + CLng(Y/16)·W`
  (`:498`). This **rounds to nearest (half-even)**, not floor. A pixel at x=8
  maps to tile column 0 (`CLng(0.5)=0`); x=24 maps to column 2 (`CLng(1.5)=2`).
* Queries:
  * `Constructible(x, y, l, h)` (`:416`) is true only if the index is in
    `(0, UBound)`. **Index 0 is never constructible.** For an area, it tests
    every `(x+i, y+j)` with `i = 0..l` and `j = 0..h` in steps of 16.
  * `CoefVitesse` and `CoefVitesseSimple` return 1 outside `(0, UBound)`.

After decoding, `Monde.Reformer_Terrain` (`:327`) runs in place, for
`i = 0..UBound`, in order:

* If `cat[i] > 0`, look at the 4 neighbours
  `N = cat[i−W]`, `S = cat[i+W]`, `W' = cat[i−1]`, `E = cat[i+1]`. Out-of-range
  neighbours are 0. W'/E wrap across rows. A neighbour counts as "set" if its
  category > 0, of **any** type.
* The shape index:

  | N S W E set? | shape |
  |---|---|
  | all four | 1 (full) |
  | only S | 2 (top tip) |
  | only N | 3 (bottom tip) |
  | only E | 4 (left tip) |
  | only W | 5 (right tip) |
  | N+S | 6 (vertical bridge) |
  | W+E | 7 (horizontal bridge) |
  | S+W+E | 8 |
  | N+W+E | 9 |
  | N+S+E | 10 |
  | N+S+W | 11 |
  | S+E | 12 (top-left corner) |
  | S+W | 13 (top-right corner) |
  | N+E | 14 (bottom-left corner) |
  | N+W | 15 (bottom-right corner) |
  | none (isolated) | **cat[i] := 0** (removed; earlier modifications affect later cells) |

* Category-0 cells keep their previous `TerrainApparence`.
* It then recomputes `Diagonale = √((W·16)² + (H·16)²)`.

### 10.4 Castles (`Charger_Chateaux`, `:452`)

1. If `Informations.ini` exists, read `[Chateaux] NombreChateaux` and, for
   n = 1..N, `Chateau(n)_PositionX` and `Chateau(n)_PositionY`. These are
   **world pixel** coordinates of the castle's top-left corner. If any is
   missing, the count resets to 0.
2. If the count is 0, scan `Chateaux.bmp` with the same decoder. For each pixel
   equal to (255,255,255), create a castle at:
   * `X = x·16 − Peuple[0].ChateauLargeur/2`
   * `Y = y·16 − Peuple[0].ChateauHauteur/2`

   Here **y starts at `H`** (not H−1) for the bottom row; that off-by-one must be
   preserved. Then write the positions back if `CreerFichierInformations`.
3. `Echanger_Emplacement_Chateaux` (`:572`) **shuffles** the positions: for
   i = 0..N−1, `j = Int(Rnd·remaining) + 1`; castle i takes the j-th remaining
   position, which is removed. This consumes N `Rnd` calls.
4. `Placer_Chateaux False` (`ModGenererMonde.bas:138-148`) sets, per castle i:
   * `MaxVie = Peuple[fief i].ChateauMaxVie + Σ_{e ≤ fief epoch} Epoque[e].BonusVieChateau`;
   * `Largeur`, `Hauteur` and the entrance offsets from the people;
   * `Vie = MaxVie`.

`Definir_Nombre_Chateaux` (`:609`) gives the count. It uses `NombreChateaux`
from `Informations.ini`, or else counts white pixels (written back).

### 10.5 Resources (`Charger_Ressources`, `:318`) and `Informations.ini`

If `Ressources.bmp` exists:

* If `Informations.ini [Ressources] NombreRessources` is present, preallocate
  exactly that many nodes. More non-black pixels than that would overflow
  (crash).
* Scan the bitmap. Here **y starts at H** and there is **no early exit**. For
  each pixel ≠ (0,0,0) create a node at:
  * `PositionX = x·16`
  * `PositionY = y·16 − 32` (32 = resource sprite height).
* Type = `Type_Ressource(R,G,B)` (`:709`):
  * white ⇒ `i = CInt(Rnd·8)`, with 8 ↦ 0. The type is uniform over 0..7 and
    consumes one `Rnd`.
  * exact match with `Cartes.ini [Ressource_{k+1}]` ⇒ k.
  * **no match ⇒ 0** (Herbes). The help text claims "random"; the code does not
    do that.
* `Definir_Type(t)`:
  * `Nom = TerrainRessource[t].Nom`, `Apparence = t`;
  * `TypeRessource = TerrainRessource[t].TypeRessource`;
  * `MaxQuantite = Quantite = TerrainRessource[t].Quantite` (= 0).
* If the count was not fixed and `CreerFichierInformations`, write
  `NombreRessources`.

If there is no `Ressources.bmp`:

* allocate `CLng(NombreCasesConstructibles / 300) + 1` nodes;
* `NombreCasesConstructibles` counts all `(W+1)(H+1)` cells whose category is
  not `Inconstructible`;
* call `Placer_Ressources` (`ModGenererMonde.bas:151`): random positions, retried
  until `Obstacle(...) = 0`, and random types via `Definir_Type_Aleatoire`
  (`CInt(Rnd·8)`, 8 ↦ 0).

`Informations.ini` is a GetPrivateProfileString file. Its path is
`App.Path\Cartes\<map>\Informations.ini`, and it is written by the game.

| Section | Key | Meaning |
|---|---|---|
| `[Chateaux]` | `NombreChateaux` | count |
| | `Chateau(n)_PositionX`, `Chateau(n)_PositionY` | top-left corner, world px |
| `[Ressources]` | `NombreRessources` | count of non-black pixels in `Ressources.bmp` |

### 10.6 Generated worlds (pointer)

When `TypeCarte=1`, `Monde.Generer_Monde(type)` (`ClsJeuMonde.cls:121`) seeds
random categories:

* per cell, if `Rnd > 1 − Relief`: `cat = CLng((N−1)·Rnd) + 1` with N = 11
  (rounded half-even, so category 11 has half the weight), forced to 0 if
  `SurfaceTerrain[cat] = 0`;
* then grows each category c for `SurfaceTerrain[c]+1` passes of
  `Faire_Pousser`;
* then `Reformer_Terrain`.

After that come `Placer_Chateaux` (via `ClsPlaceObject`),
`Contruire_Routes2` (road category = `Monde.Route`) and `Placer_Ressources`. The
exact algorithm belongs to the world-generation spec.

---

## 11. `Images\` conventions

### 11.1 Loading and transparency

* All in-game sprites go through `ClsAffSprite.Charger` (`ClsAffSprite.cls:63`):
  `dd.CreateSurfaceFromFile(path & ".bmp")`. The path is either
  `App.Path & "\Images\" & rel` or, when `CheminAbsolu`,
  `App.Path & "\" & rel`.
  * **The extension `.bmp` is always appended**, so names in data files have no
    extension.
  * Exceptions that carry the extension themselves: race `Portrait` (`.gif`,
    loaded with `LoadPicture`) and object `FichierApparence*` (matched by file
    name).
* **Colour key: source colour 0.** The surface is created with `DDSD_CKSRCBLT`
  and a zero colour key, and drawn with `DDBLTFAST_SRCCOLORKEY`. Black
  (0,0,0) is transparent.
  * Because surfaces are converted to the **16-bit (RGB565)** display format,
    any colour that quantises to 0 is also transparent: R<8, G<4, B<8 after
    truncation. To match, convert the colours to 565 and key on 0.
  * The translucent path (`Afficher` with `Translucide`) treats pixels with value
    > 0 as opaque and alpha-blends them at 50 %.
* The source files are 8-bit palettised BMPs (a few are 24-bit, e.g.
  `Effets\Horloge.bmp`; `Fond*.bmp` are 1-bit).

### 11.2 Per-entity paths and sheet layouts

| Entity | Path | Sheet layout (verified sizes) |
|---|---|---|
| Object icon | `Images\Objets\<Image>` or `<CheminImage><Image>` (absolute) | 32×32 frames, stacked vertically. Row 0 = icon; objects with a temporary-effect icon have a 2nd row (e.g. `Celerite.bmp`, `Eau.bmp` 32×64). `IndiceApparence` selects the row |
| Resource icon | `Images\Ressources\<Image>` | 32×32 |
| Map resource node | `Images\Decors\Ressources` | 256×32: 8 frames of 32×32, frame = TerrainRessource type |
| Treasure chest | `Images\Decors\Tresors` | 182×26: 7 frames of 26×26, frame = treasure tier |
| Decor | `Images\Decors\<Image>` (or `.`path) | horizontal frames of `Largeur`×`Hauteur` (Fontaine 356×86 = 4×89) |
| Effect | `Images\Effets\<Fichier>` (or `.`path) | one row of `NombreEtapes` frames, `Largeur`×`Hauteur` each (Lame 480×96 = 5×96) |
| Building | `<CheminImage><Image>` (absolute, default `Images\Batiments\`) | columns = animation frames (`NombreEtapeAnimation`), rows = state 0..`NombreEtapeFabrication`, each `Largeur`×`Hauteur`. Scierie 64×384 = 6 rows (N=5); Tour de défense 48×320 = 4 (N=3); Palais 80×270 = 2 (N=1) |
| Castle | `<Peuple.CheminImage><Peuple.Image>` (default `Images\Batiments\Chateaux\`) | 128×128 per state. Row = fief epoch **only for people 0** (Humains.bmp 128×640, 5 rows); other peoples always row 0 (`FrmMoteur2D.frm:1786-1792`) |
| Construct button | `Images\Boutons\Construire\<ImageBouton>` (or `CheminImageBouton`) | 28×84: 3 states of 28×28; demolish = `_Demolir` |
| Other buttons | `Images\Boutons\<NomBoutonX>` | sizes from `Boutons.ini` |
| Terrain atlas | `Images\Terrains\Plaine` (`ClsAffTerrain.cls:63`) | 256×224 = 16 columns × 14 rows of 16×16: **row = category, column = TerrainApparence shape** (0..15) |
| Day/night | `Images\Temps\<Image>` | 512×512 tiles |
| Cursor | `Images\Curseurs\ArrowCursor` | 24×24 frames in one row |
| Window | `Images\Fenetres\<ImageN>` | 320×368 |
| Background | `Images\Fond<W>` | screen size |
| Portrait | `App.Path\<Race.Portrait>` (GIF) | UI |
| Loading screen | `Images\Chargement.bmp`, `Images\Chargement\*` | |

### 11.3 Building state row

`ClsJeuBatiment.Etat` (`ClsJeuBatiment.cls:388`), with
N = `NombreEtapeFabrication`:

* not built: `Int(Vie/MaxVie · N)`;
* built: N, or **N−1 when it is a shop/factory/stocked service with
  `Stock < 1`** (the "empty" look).

The animation column cycles `0..NombreEtapeAnimation−1`.

### 11.4 Character sprites (`Images\Persos`, `Images\Apparences`)

* **Whole-sheet characters** (`DefinirApparence=1`, monsters, projectiles):
  * `ClsAffPerso` (`:39`) indexes every file in every sub-folder of
    `Images\Persos\` and one level below (`Monstres\X`, `Projectiles\X`), plus
    folders registered by races with a `.` path.
  * A sprite is identified by its **file name with extension** (`Cecil.bmp`).
    The first match wins across folders.
  * A new character draws `Int(Rnd·count)` over the files of its
    `CheminApparence[M|F]` folder, in `Dir` order (`Tirer_Apparence_Au_Hasard`).
* **Sheet layout**: 3 columns × 4 rows of `Race.Largeur × Race.Hauteur` (24×32 →
  72×128; Caravane 104×96 → 312×384).
  * Row = direction − 1: 1 up, 2 right, 3 down, 4 left.
  * Column = step `NumeroPas` 0..2; step 3 is drawn as column 1 (walk cycle
    0,1,2,1).
  * Idle (direction 0) = row 2, column 1 (`ClsAffPerso.cls:190-215`).
* **Layered appearance** (`DefinirApparence=0`, playable humanoids):
  * `ClsAffApparence.Init1` (`:55`) indexes the sub-folders of
    `Images\Apparences\` and one level below (`Corps\M`, `Cheveux\F`,
    `Habits\M`, `Races\Elfes`, `Objets\Casques`, …). `Init2` adds registered
    `.`-folders and counts files.
  * For each category j of the race, a random file is drawn from folder
    `DossiersCategories[M|F](j)`: `Int(Rnd·n)` in `Dir` order. The layers have
    the same 72×128 layout and are drawn in category order.
  * Object overlays (`FichierApparenceM/F`, e.g. `hut.bmp`, `Cheval de selle.bmp`)
    are resolved by file name over all appearance files. **The last match wins**
    (`Definir_Indices_Generaux`, `:194`).
  * Mounts use the per-direction offsets and foreground flags (§6.4).

---

## 12. Mods (summary)

A mod is any `.ini` in the mod folders (§2.5), parsed with the custom parser.
Typical usage:

* `[Objet_+]`, `[Race_+]`, `[Peuple_+]`, `[Effet_+]`, `[Decor_+]`,
  `[Resistance_+]`, `[Statut_+]` append new entries. Relative references
  (§2.3) link them, e.g. `Invocation = +` (the race just added),
  `EffetAttaque = +`, `MonstreIndice(5) = +`.
* `[Batiment_NEWID]` or `[CompetenceObjet_NEWID]` with a string ID creates a
  building or skill. Objects attach to them via `FabricationBatiment = NEWID`
  and `Competence = NEWID`.
* `[Race_78]`, `[Batiment_1]` modify existing entries in place. Index stability
  matters for saves: `.per` files store race indices, so mods replace a race
  rather than insert one.
* `TresorRace = 0` adds an object to the common items of all races;
  `TresorRace = n` adds it to race n's items; `EquipementRace = n` adds it to
  race n's starting equipment.
* Paths starting with `.` are relative to the install root: `.\Mods\Pack\…`.
  How many characters are stripped depends on the field: 1 for effects, decors,
  race paths and appearance folders; 2 for building `CheminImage`.
* Every `Verification_Donnees` derivation runs after mods. The Init 5/6
  defaulting does not (§3).

---

## 13. Index base cheat sheet

| Reference (file → model) | File base | Model |
|---|---|---|
| `[X_N]` header for Ressource, TerrainRessource, Objet, Peuple, Effet, Race, Decor, Resistance, Statut | 1 | N−1 |
| `[Batiment_ID]`, `[CompetenceObjet_ID]` | string | creation order |
| `Terrain.ini [Terrain_c]`, `SurfaceTerrain(c)` | **0** | c |
| `Cartes.ini [Terrain_k]`, `[Ressource_k]` | 1 | k−1 |
| `Terrain.ini [Monde_k]`, `Partie.ini TerrainChoisi` | 1 | k−1 |
| `Epoque_k`, `RessourcesDepart_k`, `VitesseEpoque_k`, `Tresor_k`, `Service_k`, `Langue_k`, `Resolution_k`, `Temp_k`, `Fenetre_k`, `Commentaires_t` | 1 | k−1 |
| `Batiment.NoEpoque`, `Objet.Epoque` | 1 | kept 1-based (compare with `NombreEpoques`); `Objet.Epoque = -1` for secret objects |
| `Ressource.Epoque` | 1 | 0-based (Val−1) |
| Indexed keys `(j)`: `PrixRessources`, `GainRessource`, `BonusCompetencesRessources`, `CoefCompRessource`, `Ressource(s)`, `BonusCompetencesSpeciales`, `CoefCompSpeciale`, `BonusCompetencesObjets`, `CoefCompObjet`, `BonusResistance`, `Resistance(s)` | 1 | j−1 |
| `BonusCompetencesServices(n)` | Service_N number (2..) | n−2 |
| `CoefCompService(n)` | service-skill number (1..) | n−1 |
| Value refs: `TypeMatierePremiere`, `TypeRessource`, `RessourceFabrique`, `ObjetsFabriques`, `GainObjet`, `PerteObjet`, `CreerProprietaire`, `MonstreIndice`, `Invocation`, `LanceProjectile`, `EffetAttaque`, `EffetMort`, `AttaqueResistance`, `AttaqueCauseEffet`, `CreerObjet*`, `PerteRessourceIndice`, `Objet(j)`, `Equipement(j)`, `Tombe`, `Statut`, `CommentairesSpecifique`, `NoListe`, `Terrain.Effet`, `[Effets]` globals | 1 (0 ⇒ −1 none) | v−1 |
| `Batiment.Service` | 0-based service index (0 = none) | raw |
| `Service.Effet` | **0-based** effect | raw |
| `Objet.Competence`, `FabricationBatiment` | string ID | lookup |
| `IndiceBestiaire` | 1-based, stored raw | raw |
| `Partie.ini` `PeupleFiefN`, `EquipeFiefN`, `TailleChoisie`, `EpoqueChoisie`, …`Choisie` | 1 | k−1 |
| `Partie.ini` `TypePartie`, `DernierPersonnage`, `PersonnageJoueurN`, `Personnage<i>` | 0 | raw |
