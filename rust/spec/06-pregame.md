# 06 — Pre-game: launch, menus, game setup, characters, options, save/load

Scope: everything that happens before `FrmMoteur2D.MoteurJeu` takes over, plus the
persistent file formats (character `.per`, saved world `.mnd`) and the Options dialog.
Source references are `File:line` in the VB6 tree
(`vb/Vendetta-3.0.0.17/`, read as Latin-1/CRLF). Shipped data values come from
`drive_c/Vendetta/` (identical to the installer's defaults).

Conventions:

* Positions are given in twips (VB) and pixels (`twips / 15`, at 96 DPI). The `.frm` files store positions in twips.
  FrmParam, FrmChargement and FrmApparence use `ScaleMode=Pixel` at runtime for controls placed directly on the form,
  but controls inside Frames stay in twips (e.g. the 380-twip fief row pitch).
* "ini `File`/`[Section]` `Key`" means `drive_c/Vendetta/Donnees/<File>.ini` unless the path
  contains `Francais\` (language folder `Langues.Dossier`, `Donnees/Langues.ini`
  `LangueChoisie=1` → `Francais\`).
* All UI captions are loaded at runtime from the language ini files. The French captions
  quoted here come from those files, **not** from the `.frm` design-time captions, which
  are sometimes outdated (e.g. the design-time `.frm` says "Fiefs", the ini says "Peuples").
* INI access goes through `ClsFicINI.Parametre` = `GetPrivateProfileString`/`WritePrivateProfileString`
  with a 255-char buffer, file path `App.Path & "\" & Fichier & ".ini"` (`ClsFicINI.cls`).
  Values are read without quotes or surrounding whitespace, and writes go straight to disk. Many settings
  are therefore persisted **immediately on change**, not on "OK".

---

## 1. Screen flow, from launch to in-game

```
Sub Main (ModMain.bas:123)
 ├─ MDIFrmMain.Show                     (ModMain.bas:152)  main MDI window, menus disabled
 ├─ FrmChargement.Show                  (ModMain.bas:154)  splash: Images\Chargement.bmp
 │    title bar shows " Vendetta III - Imperator" + loading step  (ModMain.bas:157-263,
 │    labels Francais\Chargement.ini [Chargement_1] EtiquetteChargement1..21)
 ├─ load all classes/params, Init_Sauvegarde_Perso/Monde (ModMain.bas:164-261)
 ├─ FrmParam.Hide  → triggers FrmParam.Form_Load, restores last setup   (ModMain.bas:266)
 ├─ enable menus Partie, Mondes, Personnages, Options, Documents, ?    (ModMain.bas:270-275)
 └─ FrmChargement.Rafraichir_Boutons   (ModMain.bas:284, FrmChargement.frm:708)
        if FrmChargement.Afficher = False → hide the menu, FrmParam.Show   (FrmChargement.frm:743-746)
        else → the splash becomes the **main menu** (8 big buttons)
```

`FrmChargement.Afficher` is read from `Francais\Fenetre_Menu.ini` `Afficher`
(FrmChargement.frm:696) and written back on unload (FrmChargement.frm:758-762).
**The shipped value is `Afficher =0`**, so out of the box the game goes straight to
the setup screen (`FrmParam`). This matches the `bw-menu.png` screenshot. The toggling works like this:

* `FrmParam` → **Retour** (`CmdRetour_Click`, FrmParam.frm:2448-2453): hide FrmParam, show FrmChargement,
  set `Afficher = True`. The next launches then start on the main menu.
* Main menu → any game-mode button (FrmChargement.frm:591-596): select the matching
  FrmParam tab, show FrmParam, set `Afficher = False`.

From FrmParam, **Jouer** (`CmdCommencer_Click`, FrmParam.frm:3183-3216; also menu
*Partie > Commencer* F5, MDIFrmMain.frm:499-501) does the following:

1. `Partie.EnCours = 1`.
2. If the selected tab is *Mode 2 joueurs* or *Créer serveur* (settings tabs), switch to
   tab `DernierePartie + 1`, the last real game mode chosen (see §3.1).
3. Stop the appearance timer, unload FrmInfoRaces, hide FrmParam and the MDI window.
4. `Jeu.Campagne = (tab "Campagnes" selected)`.
5. `FrmMoteur2D.Show` → `FrmMoteur2D.MoteurJeu` (game loop). MoteurJeu decides 1 or 2 local
   players (FrmMoteur2D.frm:201-213), creates interfaces, then shows the in-game loading
   screen (a full-screen artwork with a "Chargement en cours" banner and a progress text box,
   labels `Francais\Chargement.ini [Chargement_2]`) and calls **`ModMain.Commencer`**
   (FrmMoteur2D.frm:~315), which builds the world from FrmParam's controls (§3).
6. When the game loop returns, the title becomes "… - déchargement…", and the window
   unloads and reloads FrmParam (this persists and then re-reads `Partie.ini`), shows the MDI window again, and
   if `Definir_AfficherStastistiquesEnQuittant` opens the statistics HTML (FrmParam.frm:3199-3215).
   The player is back on the setup screen.

Quit: menu *Partie > Quitter* (Ctrl+Q) or closing the MDI window → close the network session,
unload FrmParam (persists settings), stop the music, `End` (MDIFrmMain.frm:509-516).

### 1.1 Main window (MDIFrmMain)

MDI container. If the screen is taller than 11500 twips it is sized 15469×11336 twips
(≈1031×756 px), otherwise maximised (MDIFrmMain.frm:377-386). Title " Vendetta III - Imperator".
All child forms are placed at (0,0) or centred in it.

Menu bar (captions from `Francais\Fenetre_MDIFrmMain.ini`, structure MDIFrmMain.frm:11-268):

| Menu | Item | Shortcut | Action |
|---|---|---|---|
| **Partie** | Campagne / Monde personnalisé / Champ de bataille / Charger scénario / Charger partie / Mode 2 joueurs | –, Ctrl+P, Ctrl+B, Ctrl+S, Ctrl+G, Ctrl+J | select FrmParam world tab `Index+1` (MDIFrmMain.frm:503-507) |
| | Commencer | F5 | = Jouer |
| | Quitter | Ctrl+Q | quit |
| **Mondes** (enabled only on the tabs *Charger scénario*/*Charger partie*, FrmParam.frm:3605-3620) | Renommer | | InputBox "Entrez le nouveau nom du monde :", copies all companion files to the new name and deletes the old ones (`Monde_Changer_Nom`, ModSAVMonde.bas:1030-1045) |
| | Commentaires | | opens `<name>.txt` in Notepad (ModSAVMonde.bas:1058) |
| | Editer | | only visible on *Charger scénario*: `Scenario_Editer` |
| | Supprimer | | confirm "Etes-vous sûr de vouloir effacer <name> ?" then delete `.snr .mnd .txt .bmp .scr` (ModSAVMonde.bas:1047-1056) |
| **Personnages** | Nouveau (Ctrl+N) / Charger | | select the right-panel tab 1 / 2 |
| | Exporter | | `Document_Perso` → HTML sheet `Doc_Perso_<Nom>.html` |
| | Sélectionner ▸ Tous (Ctrl+A) | | select every character in the list |
| | Sélectionner ▸ Meilleur niveau (Ctrl+I) / compétences / tueur / guerrier (max vie) / archer (max énergie) / magicien (max magie) / seigneur (max moral) | | load each `.per`, select the single best by Niveau / mean skill / bestiary total / MaxVie / MaxEnergie / MaxMagie / MaxMoral (MDIFrmMain.frm:559-620) |
| | Compétences ▸ Afficher toutes; Aucun tri / Tri par nom / Tri par score; Score / Niveau / Expérience | | options of the skills list in the character panel (radio groups, MDIFrmMain.frm:522-549) |
| | Supprimer | Shift+Del | = delete character(s) |
| **Documents** | Bâtiments F1, Bestiaire F2, Objets F3, Ressources F4, Races F6, Peuples F7, Statistiques F8, Artworks F9 | | §7 |
| **Options** | Paramètres (Ctrl+O) | | frmOptions (§6) |
| | Musique (Ctrl+M) | | toggle `Musique.Definir_JouerMusique` (checked item) |
| | Editer cartes (Ctrl+E) | | invisible (unfinished map editor, §8) |
| | Editer IAs (Ctrl+F) | | FrmEditeurIA (AI editor, not covered here) |
| **?** | Résumé F11 / Aide de jeu F12 / A propos | | `Doc_Resume.html` / Notepad `Francais\Info_FAQ.txt` / FrmAPropos2 |

The context menus reuse these: right-clicking LstPerso opens *Personnages*, the skills list
opens *Compétences*, LstMonde opens *Mondes*, and FileIA opens *Options* (FrmParam.frm:2483-2592).

### 1.2 Main menu (FrmChargement, when `Afficher` = 1)

Form ~625×561 px, centred, background `Images\Chargement.bmp` (ImgChargement 9375×8415 twips).
A column of 8 buttons, `CmdChoix(0..7)`, at x = 184 px, width 257, height 33, y = 152, 200, 248, 296, 344, 392, 440, 488 px
(FrmChargement.frm:153-312). Captions are set in `Rafraichir_Boutons` (FrmChargement.frm:728-733):

| i | Caption (FR) | Enabled | Action |
|---|---|---|---|
| 0 | Nouveau personnage (= FrmParam right tab 1) | always | start the 3-step creation wizard |
| 1 | Campagne | ≥1 character exists | FrmParam, tab 1 |
| 2 | Monde personnalisé | idem | tab 2 |
| 3 | Champ de bataille | idem | tab 3 |
| 4 | Charger scénario | idem | tab 4 |
| 5 | Charger partie | idem | tab 5 |
| 6 | Options (= menu caption) | always | frmOptions; the menu hides |
| 7 | Quitter | always | quit, or "cancel" while in the wizard |

**Creation wizard** (`CmdEtape_Click`, FrmChargement.frm:612-687). Buttons 1–6 are hidden during the wizard,
button 0 is disabled, and the step widgets appear in the same area:

* Step 1 "Entrez un nom :" + TextBox (initial text "Entrez un nom"). The first time, a random name is drawn
  (`Noms.Tirer_Nom_Aleatoire(Rnd<0.5)`; double-click redraws, FrmChargement.frm:795-800).
  Each change is forwarded to `FrmParam.TxtNomPerso` and the FrmParam validation message is shown;
  "Suite->" is enabled only when there is no error (FrmChargement.frm:787-793).
* Step 2 "Choisissez sa race :" — race list (copied from FrmParam.LstRaces) plus a 137×201 px
  box that shows the race portrait (`Race_Portrait`, e.g. `Images\Portraits\Humain.gif`).
  Clicking the box toggles it to the race description text (FrmChargement.frm:764-785).
* Step 3 "Choisissez son sexe :" — Masculin/Féminin radios, possibly forced by the race
  ("Ce choix est imposé par votre race"); the button "Terminer" replaces "Suite->".
* Finish: copy name and sex into FrmParam and call `FrmParam.CmdCreerPerso_Click` (§2.4).
  The wizard then returns to step 0 and the buttons refresh.
* Navigation: "<-Retour" (prev), "Suite->" (next), "Terminer" (finish); captions from
  `Francais\Fenetre_Menu.ini`.

---

## 2. The setup screen (FrmParam)

Single non-MDI-tabbed form at (0,0), ≈ 1010×690 px. Two independent halves:

* **Left (0,0)–(697,689) px**: a TabStrip of world/game types plus the frames that each tab shows.
  There are two copies of the strip: `TabStripMonde` (8 tabs, used when the network engine is active)
  and `TabStripMonde2` (6 tabs, used otherwise). Only one is visible (FrmParam.frm:2621-2622). The shipped
  `Reseau.ini Actif=1` shows all 8 tabs. `TabStripMonde2` simply forwards its selection to `TabStripMonde`
  (FrmParam.frm:3644-3652).
* **Right (704,0)–(1009,649) px**: TabStrip `TabStripPersonnage` with the tabs "Nouveau personnage" and
  "Charger personnage". The *Charger* frame is shown only if at least one `.per` file exists
  (FrmParam.frm:3654-3660). With no character, *Nouveau* is forced (FrmParam.frm:3754-3764).
* Bottom right: **Retour** (704,656) 89×33 px, a small status area (`LblMode2J` "Mode 2J activé/désactivé"
  at x=800 and `LblReseau` "Serveur réseau"/"Client réseau" at x=864, normally hidden), and
  **Jouer** (920,656) 89×33 px.

**Jouer is enabled** (`Actualiser_Bouton_Commencer`, FrmParam.frm:4158-4174) when **at least one character file exists**, and also:

* Campagne: a campaign and a scenario are selected.
* Charger scénario / Charger partie: the file list is not empty.
* Créer serveur / Rejoindre partie: the network is connected.
* other tabs: always.

A 250 ms timer (`TimerApparence_Timer`, FrmParam.frm:3662-3743) does four things:

* It animates the character sprite preview: `ImgApparence.Left` steps −360, −360, +360, +360 twips.
  This is a 4-phase walk cycle shown through a 24 px window.
* It keeps the menu music playing (`Musique.Jouer_Musique 0, True`).
* Once per second it blinks the red error labels and makes the "create character" button flash green when it is enabled.
* It blinks the network buttons that the player should press next, and polls the network.

### 2.1 Left tabs

Tab captions (`Francais\Fenetre_Partie.ini`), keys and frames shown (`TabStripMonde_Click`,
FrmParam.frm:3580-3642):

| # | Caption | Key | Frames visible |
|---|---|---|---|
| 1 | Campagne | `Campagnes` | FrameCampagnes |
| 2 | Monde personnalisé | `NouveauMonde` | FrameMonde + FrameFiefs |
| 3 | Champ de bataille | `NouvelleBataille` | FrameMonde (map forced to "Générer") + FrameArmees |
| 4 | Charger scénario | `ChargerScenario` | FrameChargerMonde (scenario list) |
| 5 | Charger partie | `ChargerMonde` | FrameChargerMonde (saved game list) |
| 6 | Mode 2 joueurs | `Mode2Joueurs` | FrameMode2Joueurs |
| 7 | Créer serveur | `CreerServeur` | FrameServeur + FrameReseau + FrameInfoReseau |
| 8 | Rejoindre partie | `RejoindrePartie` | FrameClient + FrameReseau + FrameInfoReseau |

If the network is inactive and tab 7 or 8 gets selected, the selection falls back to *Campagne* (FrmParam.frm:3582-3587).
When tab index (0-based) `i` is in 1..4, `DernierePartie = i` and `Partie.IndiceMode = i`
(FrmParam.frm:3627-3635). **Campaign (0) is not recorded**, so `IndiceMode` keeps the previous value.

#### 2.1.1 Campagne

* `FrameCampagnes` "Sélectionnez une campagne" (8,24) 681×657 px:
  * `LstCampagnes` (8,16) 241×134: sub-folders of `Sauvegardes\Campagnes\<Langue>\`
    (shipped FR: "Initiation", "Le 4e age"), FrmParam.frm:4098-4114.
  * `LblCampagneDescription` (264,16) 409×129: text of `<campagne>\<campagne>.txt`.
  * Inner frame "Sélectionnez un scénario" (16,160) 657×489:
    * `LstCampagnesScenarios` (24,24) 233×446: the `*.snr` files in the campaign folder, without extension.
    * `LblCampagnesScenariosDescription` (264,24) 385×449: `<scenario>.txt` (FrmParam.frm:2499-2553).
* Restored from `Partie.ini [Campagnes] DerniereCampagne / DernierScenario` (list indices).

#### 2.1.2 Monde personnalisé — `FrameMonde` "Monde" (8,24) 681×169 px

Left part: frame "Carte" (8,16) 417×145 with two radios in its title line,
**Charger** (`OptionCarte(1)`, x=80) and **Générer** (`OptionCarte(0)`, x=152). The choice
swaps between two frames (FrmParam.frm:3439-3449):

* **Charger** → `FrameCarteCharger`:
  * `LstCartes` (8,19) 121×121: the sub-folders of `\Cartes\` (`ClsJeuCartes.Charger_Cartes`, ClsJeuCartes.cls:129-152);
    shipped: (02) Confrontation, (02) Duel, (03) Triangle, (04) Cross rivers, (04) Delta, (04) Team Battle,
    (06) Dunes, (08) Surdira, (10) Continent.
  * `ImgCarte` (136,19) up to 121×121: `Apercu.bmp`, falling back to `Terrain.bmp`, letter-boxed to the map aspect ratio
    (ClsJeuCartes.cls:154-190).
  * `LblCarteDescription` (268,20) 137×119 on a light-yellow box: `Description<FR|EN>.txt`, falling back to
    `Description.txt` (ClsJeuCartes.cls:192-226). Example: "Carte officielle / Dimensions : 256*256 / Le delta…".
  * Selecting a map forces *Nombre de peuples* to the number of castles of the map
    (`Informations.ini [Chateaux] NombreChateaux`, otherwise it counts the white pixels of `Chateaux.bmp`)
    and **disables** that slider, whose value box turns grey (FrmParam.frm:2555-2568, ClsJeuCartes.cls:609-694).
* **Générer** → `FrameCarteGeneree` (re-enables the peuples slider):

  | Control | Pos px | Label (FR) | Range / items | Default |
  |---|---|---|---|---|
  | `ComboTaille` | (72,39) w121 | Taille : | 8 presets (below) | `TailleChoisie=3` (Moyenne) |
  | `SliderLargeurCarte` + value | (248,30) w129 | Largeur : | `MinLargeur`..`MaxLargeur` = 128..1024 | `ValLargeur=256` |
  | `SliderHauteurCarte` + value | (248,56) w129 | Hauteur : | 128..1024 | `ValHauteur=256` |
  | `ComboTerrain` | (72,83) w121 | Relief : | `Francais\Terrain.ini [Monde_1..5] Nom`: Classique, Plaines, Désert, Montagneux, Grands Lacs | `TerrainChoisi=2` (Plaines) |
  | `ComboRessources` | (72,115) w121 | Ressources : | Faibles ×0.5, Moyennes ×1, Nombreuses ×2 | `RessourceChoisie=2` |
  | `SliderNombreArbres` + value | (192,118) w185 | (no visible label) | 20..6400 resource nodes | `ValQuantitéRessources=200` |

  Size presets (`Partie.ini [Monde] TailleNDimensions/Ressources/Fiefs`, names in `Francais\Partie.ini`):

  | # | Nom | Dimensions | Ressources | Peuples |
  |---|---|---|---|---|
  | 1 | Minuscule (2 peuples) | 128×128 | 50 | 2 |
  | 2 | Petite (3 peuples) | 192×192 | 100 | 3 |
  | 3 | Moyenne (4 peuples) | 256×256 | 200 | 4 |
  | 4 | Grande (6 peuples) | 384×384 | 400 | 6 |
  | 5 | Large (8 peuples) | 512×512 | 800 | 8 |
  | 6 | Vaste (10 peuples) | 640×640 | 1200 | 10 |
  | 7 | Enorme (12 peuples) | 768×768 | 2000 | 12 |
  | 8 | Gigantesque (16 peuples) | 1024×1024 | 3200 | 16 |

  Changing *Taille* or *Ressources* (`ComboTaille_Click`, FrmParam.frm:3144-3162) sets
  Largeur/Hauteur to the preset and the resource count to `round(TailleRessources × multiplier)`.
  If the peuples slider is enabled, it also sets *Nombre de peuples* to the preset.
  The sliders can then be tuned by hand.

Right part of FrameMonde: six combos, labels at x=432, combos at x=568 w105, rows every 24 px
from y=16 (FrmParam.frm:640-937):

| Row | Label (FR) | Control | Items (from data) | Default (`Partie.ini [Monde]`) | Game effect |
|---|---|---|---|---|---|
| 1 | Ressources de départ : | `ComboRessourcesDepart` | `Francais\Parametres_Fiefs.ini [RessourcesDepart_1..5]`: Rien, Peu, Moyen, Beaucoup, Maximum | `RessourcesDepartsChoisie=1` (Rien) | castle stock per resource = `ChateauxRessourcesDepart` (0,0,100,300,500); every NPC gets `PersosRessourcesDepart` (0,5,10,15,20) of each buildable resource of its epoch and `PersosArgentDepart` gold (0,100,300,1000,1600) |
| 2 | Epoque de départ : | `ComboEpoque` | `[Epoque_1..5]`: Age de bois, Age de fer, Age de l'éther, Age d'or, Ere impériale | `EpoqueChoisie=1` | `Fiefs(i).Epoque = index` |
| 3 | Changement d'époque : | `ComboVitesseEpoque` | `[VitesseEpoque_1..5]`: Très Lent (coef 2), Lent (1.5), Normal (1), Rapide (0.5), Très Rapide (0.25) | `VitesseEpoqueChoisie=3` (Normal) | `Parametres.VitesseEpoqueSelectionnee = index` |
| 4 | Résurrections au château : | `ComboResurrections(0)` | `[Resurrections_Nombre]`: Interdite 0, 1 fois, 3 fois, 10 fois, 25 fois, 100 fois, Illimitées −1 | `ResurrectionChoisie=7` (Illimitées) | every perso `.ResurrectionNombre = value` |
| 5 | Vitesse de résurrection : | `ComboResurrections(1)` | `[Resurrections_Vitesse]`: Très lente 0.25, Lente 0.5, Normale 1, Rapide 2, Très rapide 4, Instantanée 100 | `ResurrectionChoisie=3` (Normale) | every perso `.VitesseResurrection *= coef` |
| 6 | Difficulté : | `ComboDifficulte` | 12 levels: Normale, Entraîné (Niv 10), Champion (Niv 20), Expert (30), Maître (40), Héros (50), Elite (60), Elu (70), Légende (80), Demi-dieu (90), Divinité (100), Cinglé (Niv 1000) with `BonusXP` 0, 1100, 4200, 9300, 16400, 25500, 36600, 49700, 64800, 81900, 101000, 10010000 and `CoefVieChateaux` 1, 1.5, 2, …, 6 (step 0.5), 51 | `DifficulteChoisie=1` | §3.3 |

#### 2.1.3 Monde personnalisé — `FrameFiefs` "Peuples" (8,200) 681×481 px

FrmParam.frm:309-458, rows built at load time (FrmParam.frm:2802-2847):

* "Nombre de peuples :" (48,24) + `SliderFiefs` (176,24) w305, range `[Fiefs] MinFiefs..MaxFiefs` = 2..16,
  default `ValFiefs=4`, plus a value box `LblFiefs` (488,24). Rows `0..N-1` are visible; the others are hidden
  (FrmParam.frm:3463-3482).
* Header row at y=56: "N°" (x=24), "Nombre de villageois :" (x=48), checkbox **"Equilibrer les peuples"**
  (x=160, `[Fiefs] EquilibreFiefs=1`), "Peuple :" (x=520), "Equipe :" (x=624).
* 16 rows from y=72 with a pitch of 380 twips (≈25 px). Each row has:
  * `LblNumeroFief(i)` "i+1" (x=16).
  * `SliderVillageois(i)` (x=40, w441), range `MinVillageoisFief..MaxVillageoisFief` = 1..250, default 50 each
    (`ValVillageoisFief<n>`). Its value is shown in `LblVillageois(i)` (x=488, w25).
  * `ComboPeuple(i)` (x=520, w97): the 13 peoples from `Parametres_Peuples.ini` (Humains, Animaux, Bêtes sauvages,
    Morts-vivants, Sorciers, Hommes-bêtes, Technologistes, Sylvestres, Pillards, Impériaux, Démons,
    Elémentaux, Dragons), followed by **"Aléatoire"** (index 13). Defaults: `PeupleFief1=1` (Humains), all others `14` (Aléatoire).
  * `ComboEquipe(i)` (x=624, w41): team numbers 1..16. Defaults `EquipeFief<n>=n`, so every fief is its own team.
    **Row 1's team combo is disabled** (FrmParam.frm:2834).
* With *Equilibrer* checked, moving any villager slider copies its value to all 16 sliders
  (FrmParam.frm:3505-3520). `LblTotalVillageois` (hidden) = the sum over the visible rows; it sizes the population.

#### 2.1.4 Champ de bataille — FrameMonde + `FrameArmees` "Armées" (8,200) 681×481 px

FrameMonde is shown with "Charger" disabled and "Générer" forced (FrmParam.frm:3599-3603). Its combos still
apply (Epoque, Difficulté coef, Résurrections, Changement d'époque; Ressources de départ is **unused**).
There are two side-by-side frames (FrmParam.frm:1874-2069), "Armée alliée" (24,24) and "Armée adverse" (352,24), each 313×441 px:

* Label "Equipe : 1" / "Equipe : 2" (16,24); "Peuple :" (200,24) with `ComboArmee(i)` (200,40) w97.
  The items are the 13 peoples + "Aléatoire". Defaults: `[Armée_1] Peuple=1` (Humains), `[Armée_2] Peuple=14` (Aléatoire).
* "Nombre de soldats :" (16,40) with value `LblNombreSoldats(i)` (128,40). `SliderNombreSoldats(i)` (8,64) w297,
  range `MinSoldats..MaxSoldats` = 1..300, default `ValSoldats=100`.
* Checkbox **"Equipement :"** (16,88), default `Equipement=0`. When checked, it shows `LstObjets(i)` (16,104) 281×319:
  a sorted check-list of every non-unique object with a sale price > 0 (FrmParam.frm:2974-2988).
  The checks are stored as `Objet<j>=0/1` by list position.

#### 2.1.5 Charger scénario / Charger partie — `FrameChargerMonde` "Sélectionner une partie" (8,24) 681×657 px

* `LstMonde` (16,24) 233×134: the names without extension, mirrored from a hidden `FileListBox FileMonde`
  (FrmParam.frm:4116-4156).
  * Scenarios: `Sauvegardes\Scenarios\<Langue>\*.snr`.
  * Saved games: `Sauvegardes\Mondes\*.mnd`.
* `LblDescriptionMonde` (256,24) 401×137: the `<name>.txt` next to the file.
* `ImgApercu` (16,160) 657×489: `<name>.bmp` (the screenshot taken when saving) (FrmParam.frm:3164-3181).
* The last index used is remembered separately for each tab (`DernierScenario`, `DernierMonde`).

#### 2.1.6 Mode 2 joueurs — `FrameMode2Joueurs` (8,24) 681×657 px

This tab is a settings page, not a game type. Jouer uses the last real tab instead.

* Checkbox "Activer le mode 2 joueurs sur cet ordinateur." (16,32) → ini `Jeu.ini [Jeu] Mode2Joueurs`
  (default 0), written immediately (FrmParam.frm:2240-2255). Activation is refused with a MsgBox titled "Attention",
  "Le mode duo est impossible avec la résolution d'affichage actuelle.", unless the file
  `Donnees\Interface<width>_J1.ini` exists (1024, 1280 and 1600 do; ClsInterface.cls:339-347).
* Checkbox "Toujours maintenir les 2 écrans séparés." (16,56) → `Jeu.ini Mode2JoueursEcranSepares` (default 1).
* Two frames, "Joueur 1" (16,88) and "Joueur 2" (344,88), each 321×145. Each holds a character combo
  `ComboNomJoueur(0|1)` (104,24) w113, listing **all** characters, and a read-only help list:
  * Player 1: "Situé sur la partie gauche de l'écran. / Utilise uniquement le clavier. / Le pavé numérique sert à
    diriger le curseur. / "0 inser" correspond au clique gauche… / "Entr" … clique droit / ". Suppr" … clique central".
  * Player 2: "Situé sur la partie droite de l'écran. / Utilise uniquement la souris."
* If both combos name the same character while 2P is on, a blinking red message "Sélectionnez 2 personnages
  différents." appears (256,32), the checkbox is disabled, and the status label reads "Mode 2J désactivé"
  (FrmParam.frm:2460-2472). `Actualiser_Joueurs` makes sure that player 2 defaults to a different character
  (FrmParam.frm:3768-3795).
* Effective at game start only if checked, enabled and not a campaign. Both characters are then auto-selected
  in LstPerso (FrmMoteur2D.frm:201-213).

#### 2.1.7 Créer serveur / Rejoindre partie (DirectPlay; candidate for omission in the browser port)

* `FrameServeur` "Serveur" (8,24) 385×145: "Adresse en ligne :" (public IP), "Nom de la session :" textbox,
  "Nombre de joueurs maximum :" slider (hidden when min = max = 2), and the buttons "Créer un serveur" / "Fermer le serveur".
* `FrameClient` "Parties en cours" (8,24): "IP du serveur (facultative) :" textbox, a games list, and the buttons
  "Rafraichir", "Se connecter", "Se déconnecter".
* `FrameReseau` "Paramètres réseaux" (400,24) 289×145: "Votre adresse IP locale :", "Votre pseudo :",
  "Numéro de port :", and the radios "Réseau local" / "Jouer en ligne".
* `FrameInfoReseau` "Informations" (8,176) 681×505: two info lines ("En réseau, vous jouez en mode confrontation…"),
  a players list, "Détails du personnage", "Détails de la partie", and a chat ("Messages :" + textbox + "Envoyer").
* All of these write `Reseau.ini` immediately (ClsComReseau.cls:660-735). Under `Regle4=1`
  (`Jeu.Regle_ModeConfrontation`), a network game ignores the setup and always uses map index 0, with one
  character and no standard victory or defeat (ModMain.bas:314-371).

### 2.2 Right panel — "Nouveau personnage" (`FrameNouveauPersonnage`, (712,24) 289×617 px)

FrmParam.frm:1581-1737. The frame is titled "Créez un nouveau personnage" and contains these sub-frames, from top to bottom:

1. **"Nom :"** (16,16) 257×73: a red message `LblErreurNom` (8,16) and the textbox `TxtNomPerso` (16,40) w217, centred.
   Validation (`TxtNomPerso_Change`, FrmParam.frm:4233-4264):
   * empty → "Entrez un nom.";
   * contains any of `\ / : * ? " < > | .` → "Ce nom comporte un caractère incorrect.";
   * equal, ignoring case, to an existing character → "Un autre personnage possède déjà ce nom.".
   The create button is enabled only when there is no error. Double-clicking the textbox draws a random name for the selected sex.
2. **"Race :"** (16,96) 257×201: `LstRaces` (16,16) 225×69, which renders as a multi-column list in the screenshots.
   It lists every race with `Jouable=1`, or all races if `JouerToutesLesRaces`. Shipped list: Humain, Nain, Elfe,
   Demi-Elfe, Colosse, Kobold, Félin, Fée, Drake, Garou, Ombre, Celeste. Below it, `TxtRaceDecription` (16,96) 225×97
   is a read-only multi-line description. Clicking a race (FrmParam.frm:3371-3393):
   * shows its description;
   * applies the sex restriction, which disables both radios, forces one, and shows "Ce choix est imposé par votre race.";
   * opens or refreshes the **FrmInfoRaces** window (§5).
3. **"Sexe :"** (16,304) 257×57: radios "Masculin" (32,32), the default, and "Féminin" (144,32).
4. **"Validation :"** (16,368) 257×57: `CmdCreerPerso` (40,16) 172×33, a graphical button with a check-mark
   picture and no text, which blinks green when enabled.
5. **"Génération automatiquement de personnages :"** (16,464) 257×145: three buttons centred at x=48 w153,
   "Générer 1 personnage", "Générer 10 personnage" and "Générer 50 personnages". Each generated character gets
   (FrmParam.frm:2325-2366):
   * a random race;
   * a random sex if allowed;
   * a random name;
   * then a create, a random "fixed" appearance file and a random AI file.
   Name collisions are retried, up to 1000 attempts.

### 2.3 Right panel — "Charger personnage" (`FrameChargerPersonnage`, (712,24) 289×617 px)

FrmParam.frm:1035-1579. Titled "Sélectionnez un ou plusieurs personnages".

* `LstPerso` (8,24) 105×212: an **extended multi-select** list of `Sauvegardes\Persos\*.per` (names without extension,
  alphabetical, the order of `FileListBox`).
  * The *focused* item (`ListIndex`) is the character shown in the details pane and the one that the
    single local player controls (`LblNom`).
  * All *selected* items join the game (§3.2).
  * `LblNombrePersos` (8,240) shows "N Personnages" when more than one is selected.
* Details box `FramePersonnage` (128,16) 145×233, filled from the `.per` (`Actualiser_Infos_Perso`, FrmParam.frm:3797-3922):
  * Name (top, centred), then the job/title `Metier` (centred).
  * "Niveau :" `int(level)`, plus a "Niv+" column showing `frac(level)` in %.
  * "Peuple :" (the label really says *Peuple*) with `ComboEquipePerso` (88,61) w33, items 1..16. This is the character's
    **team number**. On validate it writes `Equipe=<index>` into the `.per` (FrmParam.frm:2455-2458).
  * "Race :" (gendered name), "Sexe :".
  * "Vie / Energie / Magie / Moral / Attaque / Défense :", whose values depend on the
    *Personnages > Compétences* display mode:
    * Score: the Max values.
    * Niveau: level + 1.
    * Expérience: XP, with attack and defence XP divided by 20.
  * a padlock icon (green "sécurisé" vs open "non sécurisé", the CRC check of §4.1);
  * an "IA" marker shown when the character uses an AI;
  * the animated sprite preview `ImgApparence` (112,96) 24×128 px.
* Bottom-left TabStrip `TabStripFichiers` (8,256) 113×345 with the tabs **"Apparence"** and **"IA"**:
  * *Apparence*: the radios **"Personnalisée"** (`OptionApparence(0)`, `ChoisirApparence=False`) and **"Fixée"**
    (`OptionApparence(1)`, `ChoisirApparence=True`). A button **"Personnaliser"** is drawn over the first radio and opens
    FrmApparence (§5.1). It is enabled only with Personnalisée and when the race has appearance categories.
    * The `FileApparence` list (8,312) 113×292 shows the race's fixed sprite folder (`Race_CheminApparence`, e.g.
      `HumainsM`/`HumainsF`). It is enabled only with *Fixée*; picking a file sets `FichierApparence`.
    * Races with `DefinirApparence=1` force *Fixée* (FrmParam.frm:3924-3940).
    * Preview: *Fixée* shows the chosen file. *Personnalisée* shows the composite `Sauvegardes\Persos\<Nom>.bmp`
      written by FrmApparence (FrmParam.frm:3404-3437).
  * *IA*: the radios "Standard" (`ChoisirIA=True`) and "Scriptée" (`ChoisirIAScript=True`; visible only if the
    `IAScripts\` folder exists). The hidden radio "Aucune" (`ChoisirIA=False`) is the creation default.
    The visible file list is `IAs\*.ia` or `IAScripts\*`, and selecting a file sets `FichierIA` / `FichierIAScript`
    (FrmParam.frm:3451-3461, 3545-3578).
  * **Every change on these two tabs is saved to the `.per` immediately** (`Enregistrer_Sauvegarde_Perso`).
* Bottom-right TabStrip `TabStripCompBest` (120,256) 169×345 with three tabs (FrmParam.frm:3522-3543):
  * **Compétences**: lines "Nom : valeur" for every skill with level > 0, or all of them with *Afficher toutes*.
    Sorted by name or by score if requested, then a separator and "MINIMUM / MOYENNE / MAXIMUM : x"
    (FrmParam.frm:3942-4061).
  * **Bestiaire**: "Race(plural if > 1) : count" in alphabetical order, separator, "TOTAL : n" (FrmParam.frm:4063-4090).
  * **Trésors**: a header list with "Fortune totale : F" and "Fortune restante : R", then a check-list of the race's
    special objects (`Parametres_Races.ini Race_n Objet(k)`), labelled "Nom (prix)", or just "Nom" when free.
    * F = `Defnir_Score_Tresor`, the sum of skill levels.
    * Checking objects whose `PrixTalents` exceeds what remains shows "Vous ne disposez pas d'assez de fortune pour : X"
      and unchecks them.
    * The checks are saved immediately to `[Talents] Tresor<k>` (FrmParam.frm:4369-4437).
    * At creation, all free talents are pre-checked (FrmParam.frm:3248-3253).
* `CmdSupprimerPerso` exists but is hidden. Deletion goes through *Personnages > Supprimer* with a confirmation
  "Etes-vous sûr de vouloir effacer X ?" (or "…les personnages sélectionés ?" for a multi-selection).
  It deletes `<Nom>.per` and `<Nom>.bmp` (FrmParam.frm:3260-3293, ModSavPerso.bas:382-388).

### 2.4 Character creation (`CmdCreerPerso_Click`, FrmParam.frm:3218-3258)

1. `Persos(NoPerso).Init`; name = text with the first letter upper-cased; `Changer_Race_Personnage(race)`;
   `Feminin` from the radio; `ChoisirIA=False`; `FichierIA="_Defaut"` (`FichierIADefaut`, ModIA2.bas:24).
2. `AffApparence.Tirer_Apparence_Au_Hasard`: for each of the race's appearance categories (e.g. Corps, Cheveux, Habits ×
   M/F), pick a random image name (ClsAffApparence.cls:171-211).
3. `Enregistrer_Sauvegarde_Perso(perso, NouveauPerso:=True)`, which writes the CRC key, and then
   `Enregistrer_Sauvegarde_Interface_Defaut`, which writes `[Interface] AutoBatimentService=1, CompetencesTri=2`.
4. Refresh the list, select the new character, switch to the *Trésors* tab, auto-check the free talents,
   clear the name and go to the *Charger personnage* tab.

All XP starts at 0 (level 1). No `Equipe` key is written, so the team defaults to index 0 (team 1).

---

## 3. How the setup becomes a world (ModMain.Commencer)

There is **no parameter object**. `Commencer` (ModMain.bas:290-770) reads FrmParam's controls directly.
For the Rust port, gather them into a struct like:

```text
GameSetup {
  mode: Campaign{campaign, scenario} | CustomWorld | Battle | Scenario{name} | SavedGame{file},
  map: Generated{width, height, resource_nodes, terrain_type} | Loaded{map_folder},
  start_resources: idx, start_epoch: idx, epoch_speed: idx,
  resurrections: count(-1=∞), resurrection_speed_coef, difficulty: {bonus_xp, castle_life_coef},
  fiefs: [ {villagers, people: idx|Random, team} ; n ],          // CustomWorld
  armies: [ {soldiers, people: idx|Random, equipment: [object]} ; 2 ],  // Battle
  characters: [name...]  (all selected in LstPerso, list order),
  controlled: name (LstPerso focus)  | two names (2-player mode),
  two_player: bool, split_screens: bool,
}
```

### 3.1 Which mode

`FrmParam.TabStripMonde.SelectedItem.Key` (ModMain.bas:402). If the user pressed Jouer on the *Mode 2 joueurs*
or *Créer serveur* tab, the tab `DernierePartie+1` is selected first (FrmParam.frm:3186-3188).
`DernierePartie` is one of 1..4 (Monde perso, Bataille, Scénario, Charger partie) and **never 0 (Campagne)**.

Common start (ModMain.bas:302-312, 393-400):

* `Randomize`, then the effects, time, damage, treasure, decor and events subsystems are initialised, and `Jeu.Init`.
* `Monde.Largeur = LblLargeurCarte`, `Monde.Hauteur = LblHauteurCarte` (the generated-map sliders, even when a map is loaded).
* `Ressources` is sized to `LblNombreArbres + 1` new objects.

### 3.2 Per mode

**Campagne** (ModMain.bas:403-406): `Charger_Scenario("<campagne>\<scenario>", Campagne:=True)` reads
`Sauvegardes\Campagnes\<Langue>\<campagne>\<scenario>.snr` (ModScenario.bas:28-50; the format is covered elsewhere).

**Monde personnalisé** (ModMain.bas:407-479):

1. `N = LblFiefs`, giving `Fiefs(0..N-1)` and `Chateaux(0..N-1)`. For fief i:
   * `NumeroEquipe = ComboEquipe(i)+1`;
   * `TypePeuple = ComboPeuple(i)`, or `Int(Rnd*NombrePeuples)` if "Aléatoire";
   * `NombreCitoyens = LblVillageois(i)`.
2. `Persos`, `Maisons` and the effect arrays are sized to the total of villagers. Persos are assigned to fiefs **sequentially**
   (the first `v0` go to fief 0, the next `v1` to fief 1, …). Each one gets `Init` and
   `Tirer_Monstre_Aleatoire(i, peuple)`, which draws its race and stats from the fief's people. `Maisons(i).Numero = i`.
3. For each fief: `Init`, `Epoque = ComboEpoque`, and for each resource whose `Ressources_Epoque <= Epoque`,
   castle stock = `RessourcesDeparts_ChateauxRessourcesDepart(ComboRessourcesDepart)`.
   `Partie.RessourcesDeDepart = ComboRessourcesDepart`.
4. Every perso: `Perso_Charger_Ressources_Depart` (only when `Partie.IndiceMode = 1`, ModGenererMonde.bas:198-211),
   then `NumeroEquipe = fief team`.
5. Map:
   * *Charger*: `Cartes.Charger_Monde / Charger_Chateaux / Charger_Ressources` (from
     `Cartes\<name>\Terrain.bmp`, `Ressources.bmp`, `Chateaux.bmp`, `Informations.ini`; ModMain.bas:467-471,
     FrmParam.frm:2266-2273), then `Placer_Chateaux False`, and `Cartes.Chargee = True`.
   * *Générer*: `Monde.Generer_Monde(ComboTerrain)`, `Placer_Chateaux`, `Contruire_Routes2`, `Placer_Ressources`.
   * Then `Placer_Joueurs`: every perso starts at the entrance of its fief's castle with `DansSonChateau=True`
     (ModGenererMonde.bas:173-196).
6. `Parametres.VitesseEpoqueSelectionnee = ComboVitesseEpoque`.

**Champ de bataille** (ModMain.bas:481-607):

1. Two fiefs, with `NombreCitoyens = soldiers(i)`, `NumeroEquipe = i+1`, people from `ComboArmee(i)` (random if "Aléatoire"),
   epoch from ComboEpoque.
2. Castles: `MaxVie = Peuples_ChateauMaxVie(people)` and `Vie = 0` (no castles in practice).
3. Persos `0..s0+s1-1`: the first `s0` belong to fief 0 and the rest to fief 1. They are laid out on a grid with 20 rows
   per column (row `j`, column `k`), at pixel `(k*w+640, j*h+640)` for army 1 and `(k*w+1640, j*h+640)` for army 2.
   Each perso:
   * gets a random monster of its people (except index `NoPerso`);
   * gets `Action = 300`;
   * gets `FichierIA="Bataille.ia"` and `ChoisirIA=True`;
   * if it is not the player, levels up (`Gestion_Tous_Niveaux`, `Ajuster_Bonus`);
   * starts with Attack and Defence at max;
   * has `IndicePerso` set to an enemy index, as the initial target.
4. The player is swapped with soldier `s0/2`, to stand in the middle of the allied army.
5. Equipment: for each army with *Equipement* checked, every checked object is added to each soldier's inventory.
   If the object consumes ammunition (`Objet_PerteRessourceQuantite > 0`), the matching resource is set to `PersosMaxRessources`.
6. Map: always `Generer_Monde(ComboTerrain)` + `Placer_Ressources`; `VitesseEpoqueSelectionnee` as above.

**Charger scénario**: `Charger_Scenario(LstMonde item)` from `Sauvegardes\Scenarios\<Langue>\<name>.snr`.

**Charger partie**: `Charger_Sauvegarde_Monde("<name>.mnd")` (§4.2).

### 3.3 After the mode-specific build (all non-network modes)

* **AI assignment** (ModMain.bas:617-630):
  * every perso gets `IA = Not Projectile`;
  * every NPC (`Joueur=0`) without its own AI whose race has `IA_Selectionner=1` gets `ChoisirIA=True` and a
    **random file from `IAs\*.ia`** (the FileIA list). `NiveauIA = 1` makes this always happen (ModIA.bas:22).
* **Characters** (ModMain.bas:635-703):
  * Campaign: only the focused character (`LblNom`) is loaded, into `Persos(NoPerso)`, with team 1, fief 0, its talents,
    and `Joueur = PlayerID`.
  * Otherwise, for every **selected** LstPerso item in list order:
    1. read its `Equipe` (0-based) and take `team = Equipe+1`;
    2. take the **first** perso `j` with `Joueur=0` and `NumeroEquipe=team`, and **replace it** with the saved character:
       `Joueur = PlayerID`, `Init_Carac`, `Charger_Sauvegarde_Perso`, `Charger_Sauvegarde_Tresor`, `IA = ChoisirIA`;
       the slot keeps its position, fief and house;
    3. if that character's name equals `LblNom`, or in 2P mode `ComboNomJoueur(0)`/`(1)`, it becomes `NoPerso` /
       `Nopersos(0|1)` with `ControleJoueur=True`;
    4. if no slot is free, MsgBox "Il n'y a pas assez de place pour tous vos personnages dans le fief N." and the game
       aborts (`Quitter=True`).
  * Selected companions without an AI stay player-owned but uncontrolled (`IA=False`).
  * This also applies to scenarios and **saved games**: the `.per` data replaces the slot in the loaded world (§4.2).
* The controlled persos have their AI forced off (ModMain.bas:705-708).
* **Difficulty**, only for *Monde personnalisé* and *Champ de bataille* (ModMain.bas:710-757):
  * every castle: `MaxVie *= CoefVieChateaux`, and in Monde perso also `Vie = MaxVie`;
  * every NPC (`Joueur = 0`): every XP pool (Vie/Energie/Magie/Moral, special, resource, service and object skills) is set
    to `BonusXP`, Attack/Defence XP to `BonusXP*20`, then the levels are recomputed. The Difficulté names advertise the
    resulting level ("Niv 10" …);
  * **every** perso, players included: `ResurrectionNombre = value`, `VitesseResurrection *= coef`.
  * Scenarios, campaigns and saved games keep their own values.

---

## 4. Save files

Paths come from `Donnees\SavPerso.ini` and `Francais\SavMonde.ini`. They are relative to the game folder.

### 4.1 Character file `Sauvegardes\Persos\<Nom>.per` (INI, Latin-1)

INI sections `[Personnage]`, `[Interface]` and `[Talents]` (`SectionPerso/SectionInterface/SectionTalents`). There is also an optional
`<Nom>.bmp` composite appearance, written by FrmApparence. Writer: `Enregistrer_Sauvegarde_Perso`
(ModSavPerso.bas:167-231); reader: `Charger_Sauvegarde_Perso` (ModSavPerso.bas:50-165).

`[Personnage]`:

| Key | Type / meaning |
|---|---|
| `Sexe` | `Masculin` / `Féminin` (literal; anything other than "Féminin" loads as male) |
| `Race` | race index (0-based, into `Parametres_Races.ini`) |
| `Equipe` | team index 0..15; written **only** by the team combo (`Changer_Equipe_Perso`, ModSavPerso.bas:419-425); missing → 0 |
| `Apparence` | 1 = fixed sprite file (`ChoisirApparence`), 0 = composite |
| `FichierApparence` | fixed sprite file name, e.g. `Elfe-F3.bmp` |
| `FichierApparenceBase1..n` | composite layer images, one per race appearance category (e.g. `elf-normal.bmp`) |
| `IA` | 1/0 `ChoisirIA` |
| `FichierIA` | e.g. `Porte-bouclier.ia` (`_Defaut` at creation) |
| `ChoisirIAScript` | 1/0 |
| `FichierIAScript` | script file name |
| `Attitude`, `Formation` | integers |
| `Vie`, `Energie`, `Magie`, `Moral`, `Attaque`, `Défense` | **experience** of the primary/combat stats |
| `CompétenceSpéciale1..n` | XP of the special skills |
| `CompétenceRessource1..n` | XP of the resource skills (26 shipped) |
| `CompétenceService1..n` | XP of the service skills |
| `<SAVPerso key>` | XP of each object skill. The key name comes from `Parametres_CompetencesObjets.ini [CompetenceObjet_i] SAVPerso` (e.g. `CompétenceObjet25`, `CompétenceObjet1`, …; not sequential) |
| `BestiaireMonstre<k>` | kill count per race, `k = Race_IndiceBestiaire(i)`. Races from mods (`Race_FichierMod`) are not written |
| `XPBestiaire` | **anti-cheat checksum** (disguised name), see below |

Levels are not stored; they are recomputed from XP (`Gestion_Tous_Niveaux`). On load, the current
Vie/Energie/… keep the ratio they had before the reload (ModSavPerso.bas:66-71, 146-151).

`[Talents]`: `Tresor<k>=0/1` for the k-th special object of the race (ModSavPerso.bas:233-293).

`[Interface]`, the in-game UI preferences of this character (ModSavPerso.bas:295-380):

* `AutoRecuperationCarac<i>` 0/1 and `AutoRecuperationCaracPcent<i>` (percent; out-of-range values reset to 100 for
  i=0 and 50 otherwise);
* `AutoBatimentService` 0/1;
* `ToucheRaccourciObjet<i>` = key+1;
* `Fenetre(<i>)_Deplace` 0/1, with `Fenetre(<i>)_X` and `_Y`;
* `CompetencesTri`, `CompetencesAfficherToutes`.
* Defaults at creation: `AutoBatimentService=1`, `CompetencesTri=2`.

**Checksum** (`PersoToCRC` + `CRC16`, ModSavPerso.bas:465-523):

* `s = Nom & Hex(Val(ChoisirIA)) & Hex(Val(Feminin))`, followed by `Hex(Val(x))` for each of ExpVie, ExpEnergie, ExpMagie,
  ExpMoral, ExpAttaque, ExpDefense.
* Then `Hex(Val(x))` for each special, resource, service and (non-mod) object skill XP that is `>= 1`, and for each
  (non-mod) bestiary count that is `>= 1`, in that order.
* `Val()` of a Boolean is always 0, so the two flags always contribute `"00"`. `Val()` of a Double stringifies it
  with the locale separator, so on a French system it **truncates at the decimal comma**. Hex is upper-case without
  leading zeros.
* `S = Σ ANSI code of each char of s`; `x = S mod 65536`; `v = (-x) & 0xFFFF`;
  result = `(v & 0xFF) << 8 | (v >> 8)` (byte-swapped), interpreted as a **signed 16-bit** integer
  (VB `CLng("&hHHHH")`), and 0 if `x = 0`.
* Stored as decimal (e.g. `XPBestiaire=14332`).
* `Protege = (CStr(CRC) = stored)`. It is shown as the padlock. The CRC is rewritten only when the character is
  new or is still protected, so a tampered file stays "non sécurisé" forever.

**Writes during play**: every 60 s when `[SavPerso] SauvegardePeriodique=1` / `PeriodeSauvegarde=60`
(`Enregistrement_Periodique`, ModSavPerso.bas:390-402; not in network confrontation mode). The setup screen
also saves on every appearance/AI/talent change.

### 4.2 Saved world `Sauvegardes\Mondes\<name>.mnd` (+ `.txt`, `.bmp`)

Writer: `Enregistrer_Sauvegarde_Monde` (ModSAVMonde.bas:578-1001). Reader: `Charger_Sauvegarde_Monde`
(ModSAVMonde.bas:67-576). The writer does nothing on a network client.

When it is called (all in-game):

* the in-game save button (ClsInterface.cls:2710) or the save hotkey (ClsIntTouches.cls:663, not in a campaign), with the default
  name `dd_mm_yyyy__hh_mm`;
* on quitting, when `SavMondeEnQuittant=1`, the game was not won or lost, and it is not a campaign, with the name
  `SavMondeEnQuittantNom` = "Sauvegarde automatique" (FrmMoteur2D.frm:1553-1554).

Companion files:

* `<name>.bmp`: a screenshot, when `SavImageEnQuittant=1` (`Capture_Ecran`).
* `<name>.txt` (`Capture_Texte`, ModSAVMonde.bas:1012-1028), 10 lines:

  ```
  Sauvegarde personnelle - Date : dd/mm/yyyy   Heure : hh:mm
  
  Dimensions de la carte : W * H
  
  Personnage utilisé : <player name>
  Nombres de joueurs max : <villagers of the player's fief>
  Peuple : n° <player fief+1>
  Epoque : <epoch name>
  Nombre de fiefs : N
  Nombre de personnages : P
  ```

Both files are first created by copying the empty file `\Sauvegardes\Monde vierge` (`FichierVierge`).

**`.mnd` format**: a plain text file, **one value per line**, written with VB `Print #`. Format details for the reader:

* Numbers carry a leading space when positive.
* Decimals use the locale separator, so a comma on French Windows.
* Booleans are written `True`/`False`.
* Strings are raw.

A robust reader must trim lines, accept both `.` and `,` as the decimal separator, and parse `True/False/-1/0` as booleans.
In order:

1. **Version header** (19 lines, each compared with the current data; a mismatch gives the error
   "Le fichier de monde X est invalide / La version ou les mods dans cette sauvegarde sont différents de ceux actuels"):
   NombreCompetencesObjets, NombreCompetencesSpeciales, NombreDecors, NombreEffets, NombreEpoques, NombreObjets,
   NombrePeuples, NombreRaces, NombreRacesJouables, NombreResistances, NombreRessources, NombreRessourcesDepart,
   NombreServices, NombreStatuts, NombreTerrainRessource, NombreTresors, NombreTypeBatiments,
   NombreTypeBatimentsJouables, NombreVitesseEpoques.
2. **Counts**: fiefs, castles, resources, treasures, decors, persos, houses.
3. `VitesseEpoqueSelectionnee`.
4. Loaded map folder name (empty line if the map was generated or has no `Carte.bmp`). On load it sets `Cartes.DerniereCarte`
   and `Chargee=True`, for the minimap.
5. `Monde.Largeur`, `Monde.Hauteur`, then for each of `(W+1)*(H+1)` tiles: `TerrainApparence`, `TerrainCategorie`.
6. `AffTemps.Heures`, `AffTemps.Minutes`, `Jeu.Chrono`, `Jeu.IntituleChrono`.
7. Per **fief**: NumeroEquipe, TypePeuple, Nom, Epoque, ChangerEpoque, TempsChangementEpoque, NombreCitoyens,
   NombreMorts, Empereur, Argent, then the stock of each of the `NombreRessources` resources.
8. Per **castle**: Numero, PositionX, PositionY, Largeur, Hauteur, EntreePositionX, EntreePositionY, Construit,
   EnConstruction, Vie, RecuperationVie, MaxVie.
9. Per **resource node**: PositionX, PositionY, Nom, Apparence, TypeRessource, Quantite, MaxQuantite.
10. Per **treasure**: X, Y, Fortune, Argent, each resource, object count n, then n object ids.
11. Per **decor**: Categorie, X, Y, Largeur, Hauteur, EtapeAnimation, NombreEtapeAnimation, Nom, Temporaire (written
    as −1/0), Duree.
12. Per **perso** (ModSAVMonde.bas:707-965; 230-529), in this order:
    * movement: PositionX, PositionY, CibleX, CibleY, LongueurPas, Acceleration, BonusDeplacement, NumeroPas, Apparence,
      DirectionX, DirectionY;
    * identity: Numero, Nom, Largeur, Hauteur, NumeroFief, NumeroEquipe, NumeroChef, Incommandable, ChefDecalageX,
      ChefDecalageY, ChefPositionX, ChefPositionY, Attitude, Formation, Empereur, NombreSoldats, Race, Parole, Volant,
      ChoisirApparence, FichierApparence, ApparenceChoisie, Titre, Feminin, GainExperience, DureeRessurection,
      ResurrectionNombre, VitesseResurrection, Commentaires, CommentairesSpecifique, CommentairesSpecifiqueIndice,
      TempsDernierCommentaire;
    * regeneration speeds: VitesseRegeneration{Vie,Energie,Magie,Moral};
    * for each of Vie, Energie, Magie, Moral: value, Max, Recuperation, Exp, Niv, ExpNiv, ExpNivPlus;
    * Attaque: value, Max, Exp, Niv, ExpNiv, ExpNivPlus, PorteeAttaque, EffetAttaque, ForcerAttaque, Kamikaze,
      DureeRechargeAttaque, DureeRechargeAttaqueMax;
    * Defense: value, Max, Exp, Niv, ExpNiv, ExpNivPlus, Armure;
    * Projectile, ProjectileCible, Pietinement, Inattaquable;
    * AI: ChoisirIA, FichierIA, IA_Equiper, IA_Manger, IA_Pacifique, IA_Temeraire, IA_Berserk, IA_Ordre, IA_Cible;
    * state: ControleJoueur, Argent, Action, Direction, EnDeplacement, DureeImmobilisation, DansUneMaison, DansSaMaison,
      IndiceMaison, DansUnChateau, DansSonChateau, IndiceRessource, IndiceChateau, IndicePerso, IndiceTresor,
      RessourceCourante, ServiceCourant;
    * skills: 5 lines per skill (Carac, Exp, Niveau, ExpNiv, ExpNivPlus) for each special skill, each resource
      (`NombreRessources`), each service (`NombreServices-1`), and each object skill;
    * race modifiers: CoefRace{Vie,Energie,Magie,Moral,Attaque,Defense}, then one coef per special skill, resource,
      service (`NombreServices-1`) and object skill, then BonusMaxRessourcesRace;
    * objects: IndiceObjet, ObjetSelectionne, EtatFabricationObjet, then (Actif, Type) for each equipped slot and for
      each inventory slot. **Unique objects are written as `False`, `-1`** (not saved);
    * the stock of each resource; bestiary count for each race (`NombreRaces`);
    * temporary effects: count n, then n × (Numero, Duree, DureeMax, Proprietaire); unique-object effects are skipped on load;
    * recharging effects: count n, then n × (Numero, Duree);
    * ratios Vie/MaxVie, Energie/Max, Magie/Max, Moral/Max, Attaque/Max, Defense/Max, applied after the max values are recomputed.
13. Per **house**: Numero, TypeBatiment, PositionX, PositionY, Largeur, Hauteur, EntreePositionX, EntreePositionY,
    Construit, EnConstruction, Vie, RecuperationVie, MaxVie, Argent, MaxStock, then
    `Batiment_NombreObjetsFabriques(type)` × Stock_Objets(j), then Definir_Stock, then
    `Definir_Nombre_Objet_Inventaire` × ObjetsInventaire(j), then Magasin, Fabrique, Service, Marche.

Not saved: who the human player is (`Joueur`), the game mode, difficulty settings (already baked into the persos),
and which characters were used. On load, the selected `.per` characters are re-inserted into the first free perso of their team
(§3.3), which overwrites that perso's stats with the `.per` contents.

### 4.3 Setup memory `Donnees\Partie.ini` (written by `FrmParam.Form_Unload`, FrmParam.frm:4266-4363)

| Section | Keys |
|---|---|
| `[Campagnes]` | `DerniereCampagne`, `DernierScenario` (list indices) |
| `[Personnages]` | `FichiersSelection` (0 Apparence / 1 IA tab), `CompetencesSelection` (0..2), `CompetencesAfficherToutes`, `CompetencesTri` (0 none/1 name/2 score), `CompetencesAffichage` (0 score/1 level/2 XP), `NombrePersonnages`, `Personnage<i>=0/1` (selection by list position), `DernierPersonnage` (focused index), `PersonnageJoueur1`, `PersonnageJoueur2` (2P combo indices) |
| `[Monde]` | `TypePartie` (selected tab 0..7; restored only if 0..5), `TypeCarte` (1 Générer / 2 Charger), `DerniereCarte` (1-based), `DernierePartie`, `DernierScenario`, `DernierMonde`, `TailleChoisie`, `TerrainChoisi`, `RessourceChoisie`, `EpoqueChoisie`, `RessourcesDepartsChoisie`, `VitesseEpoqueChoisie`, `ValLargeur`, `ValHauteur`, `ValQuantitéRessources`, `DifficulteChoisie` (all combos 1-based) |
| `[Resurrections_Nombre]`, `[Resurrections_Vitesse]` | `ResurrectionChoisie` |
| `[Fiefs]` | `ValFiefs`, `EquilibreFiefs`, `ValVillageoisFief<n>`, `PeupleFief<n>`, `EquipeFief<n>` (n = 1..16) |
| `[Armée_1]`, `[Armée_2]` | `MinSoldats`, `ValSoldats`, `MaxSoldats`, `Peuple`, `Equipement`, `Objet<j>` |

The same file also holds the static tables read at load: the size presets, resource multipliers, difficulty levels,
resurrection values, and the slider min/max. Their names live in `Francais\Partie.ini`.

---

## 5. Secondary pre-game windows

### 5.1 Appearance editor (FrmApparence, caption "Choisissez une apparence")

FrmApparence.frm, ClsAffApparence.cls. It is opened by "Personnaliser", and FrmParam is hidden while it is open.

* One column per appearance category of the race (`Parametres_Races.ini NombreCategoriesApparences`, e.g. 3 for Humain:
  `Corps\M|F`, `Cheveux\M|F`, `Habits\M|F` under `Images\Apparences\`; a path starting with `.` is game-relative).
  Columns are 120 px apart (`LargeurColonne`, in pixels because the form uses `ScaleMode=Pixel`; the form grows
  by 1780 twips per extra column), and each one contains:
  * a file list (113×214 px) of the category folder;
  * under it, the 72×128 px layer image (`PicApparence`), showing the whole sprite sheet column.
* On the right: "Aperçu" and a 72×128 preview `PicApercu`. It is a **pixel composite**: layer 0 is copied, and every
  later layer's non-black pixels (`Point > 0`) are drawn over it (FrmApparence.frm:197-217). The preview is refreshed
  1 s after a change (`VitesseApercu=1000`).
* Buttons "Apparence aléatoire" (a random file in every column) and "OK" (close). There is also a menu *Apparence > Aléatoire / Quitter*.
* On close: the selected file names go into `FichierApparenceBase1..n`, the preview is saved as
  `Sauvegardes\Persos\<Nom>.bmp`, the `.per` is saved, and FrmParam is shown with *Personnalisée* selected
  (FrmApparence.frm:234-281).
* Initial selection comes from the saved `FichierApparenceBase<i>`, or the first file if it is missing.

### 5.2 Races info window (FrmInfoRaces, caption "Races")

FrmInfoRaces.frm. It opens next to FrmParam when a race is clicked, or with Documents > Races (F6). It sits at the top of the
screen, about 1/3 across. The race list is synced in both directions with FrmParam.LstRaces.

* Left: the race list (8,8) 225×69, "Aperçu :" frame with the portrait (or "Aperçu indisponible"), and
  "Progression des caractéristiques :" with 6 progress bars (max 200) and labels for Vie, Energie, Magie, Moral,
  Attaque, Défense. Value = `Race_Coef<X>*100` followed by "%".
* Right column:
  * the description textbox;
  * **"Bonus :"** list: "+N% <skill>" for each skill coefficient > 1;
  * **"Malus :"** list: "-N% <skill>" for each < 1. Skills cover special, resource, service and object skills;
  * **"Particularités :"**: cannot use a mount, attack resistance type, regeneration rates (×1000, "0.00"), Volant, Muet,
    no XP, specific comments, carrying bonus, attack range, attack effect, armour, speed ±, Infatigable,
    full restoration, Kamikaze, starting resources, resistances (+/−%, or MAX/MIN), and "Equipement : …" (free items);
  * **"Capacités spéciales :"**: the race's own objects (FrmInfoRaces.frm:404-586).
* Quirk: the starting-resources line is only added inside the resistance loop, and only when a resistance differs.

### 5.3 About box (FrmAPropos2)

FrmAPropos.frm.

* Title "À propos de Imperator", "Version <Major>.<Minor>".
* Clickable mail labels (sharlaut@netcourrier.com, varaxor@free.fr) and the site http://franck.leveque2.free.fr/Vendetta.
* Credits in two lists: Conception, Programmation, Site Internet, Béta testeurs, Remerciements / Graphismes,
  Sons et musiques (FrmAPropos.frm:190-285).
* A disclaimer paragraph.
* Buttons "Lisez moi" (opens `Francais\Info_Lisez moi.txt`) and "OK".

### 5.4 Map editor (FrmEditeurCarte) — unfinished

Menu entry invisible. A layout-only stub: cards list, Nouveau/Duppliquer/Renommer/Supprimer buttons,
Largeur/Hauteur fields, an 11-tile terrain palette cut from `Images\terrains\Plaine.bmp` (16×16), and a scrollable canvas.
There is no save logic (FrmEditerCarte.frm:273-316). Maps are authored as BMP files instead (`Cartes\Comment creer une carte.txt`;
colour tables in `Donnees\Cartes.ini`). **Skip in the port.**

---

## 6. Options dialog (frmOptions, caption "Options")

frmOptions.frm:967-1532. A modal-like form centred in the MDI window (~427×340 px). FrmParam and FrmInfoRaces are hidden
while it is open. On close it returns to the main menu if `FrmChargement.Afficher`, otherwise to FrmParam
(frmOptions.frm:1342-1348).

* Left: TreeView (8,8) 177×265 with this hierarchy (frmOptions.frm:1092-1116):

  ```
  Jeu
   ├─ Règles
   ├─ Affichage
   │   ├─ Interface
   │   ├─ Messages d'informations
   │   ├─ Personnages
   │   └─ Terrain
   ├─ Sons
   ├─ Cartes
   └─ Réseau
  Sauvegardes
  Statistiques
  ```

  Clicking a node shows the matching right-hand frame (192,8) 225×265.
* Footnotes at the bottom-left depend on the page; for example "(1) Si cette option est modifiée, relancez le programme…".
  A red "Redémarrez le jeu pour activer les modifications." appears when a restart-only option changes.
* Buttons **OK** (264,312) and **Annuler** (344,312). Nothing is applied before OK, and OK writes everything
  (frmOptions.frm:1401-1480).

| Page | Control (FR caption) | Type | Persisted as | Shipped default |
|---|---|---|---|---|
| Jeu | Langue (1) : | combo | `Langues.ini LangueChoisie` (1-based): Français, English | 1 |
| Jeu | Vitesse : | slider `VitesseMin..VitesseMax` = 5..30 + value | `Jeu.ini Vitesse` | 10 |
| Jeu | FPS Max : | slider 5..60 + value | `Jeu.ini FPS` | 60 |
| Jeu | Désactiver la limite de FPS (hidden) | check; hides the FPS slider | `Jeu.ini AffichageRapide` | 1 |
| Règles | "Invocations multiples. (Le nombre d'invocations dépend du niveau de la compétence)" | slider 0..20 ×10 → 0..200 "%" | `Jeu.ini Regle1` | 50 |
| Règles | Délais de rechargement pour les objets. (1) | check | `Jeu.ini Regle2` | 1 |
| Règles | Lancer des projectiles. (1) (hidden) | check | `Jeu.ini AfficherProjectiles` | 1 |
| Affichage | Résolution (1) : | combo "640 * 480" … "1600 * 1200" | `Ecran.ini Resolution` (1-based) | 3 (1024×768) |
| Affichage | Plein ecran. (1) | check (checked = **not** windowed) | `Ecran.ini ModeFenetre` (inverted) | ModeFenetre=0 → checked |
| Affichage | Optimiser l'affichage. | check | `Francais\Terrain.ini TerrainOptimise` | 0 |
| Interface | Afficher les infos bulles. | check | `Boutons.ini AfficherInfoBulle` | 1 |
| Interface | Afficher les infos bulles sur le terrain. / les frames par secondes. / les actions. / les barres de statut. / les boutons d'information. / les boutons de déplacement. / Glisser les boutons de déplacement. / les boutons de construction. / Glisser les boutons de construction. / Afficher l'époque. / les informations des fiefs. / les effets temporaires. / les raccourcis des objets. | checks | `Interface<width>.ini [Interface]` `AfficherInfoBulleTerrain, AfficherFPS, AfficherAction, AfficherBarresStatut, AfficherBoutonsInfos, AfficherBoutonsDeplacement, MasquerAutoBoutonsDeplacement, AfficherBoutonsConstruire, MasquerAutoBoutonsConstruire, AfficherEpoques, AfficherStastistiques, AfficherEffetsTemporaires, AfficherRaccourciObjets` | all 1 |
| Interface | Afficher le mode auto. (hidden) | check | `AfficherMessageModeAuto` | 0 |
| Messages d'informations | Afficher les messages d'information. | master check; enables the sub-frame | `Francais\Messages.ini Afficher` | 1 |
| ″ | Afficher un conseil en début de partie. + button "Consulter les conseils." (opens the tips file) | check | `AfficherConseil` | 1 |
| ″ | Afficher Vie/Mort. / combats gagnés. / gains/pertes argent. / gains/pertes ressources. / gains/pertes objets. / gains/pertes soldats. / niveaux supérieurs. / nouveau/mort empereur. | checks | `AfficherVie, AfficherBestiaire, AfficherArgent, AfficherRessources, AfficherObjets, AfficherSoldats, AfficherExperience, AfficherEmpereur` | all 1 |
| Personnages | Afficher les dégâts des combats. | check | `Combat.ini [Degats] AfficherInfoBulle` | 1 |
| Personnages | Afficher votre fantôme. | check; enables the next one | `Jeu.ini AfficherFantomes` | 0 |
| Personnages | Afficher les fantômes alliés. (1) | check | `Jeu.ini AfficherFantomesPNJ` | 0 |
| Personnages | Afficher les commentaires. + Durée : [slider] "x.xx s" | check + slider `DureeMin/DureeIntervale..DureeMax/DureeIntervale` | `Francais\Commentaires.ini Afficher`, `Duree` (stored ×100) | 1, 16 (=0.16) |
| Personnages | Pathfinding : (Cherche le chemin le plus court) | combo Aucun / Simple / Complexe | `Jeu.ini Regle3` | 1 (Simple) |
| Terrain | Eau animée. (1) (2) | check | `Francais\Terrain.ini TerrainAnime` | 0 |
| Terrain | Afficher le temps de la journée. | check | `Temps.ini [Temps] Afficher` | 0 |
| Sons | Jouer les bruitages. + "Accéder aux bruitages" (opens the folder) | check | `Sons.ini Jouer` | 1 |
| Sons | Jouer les musiques. + "Accéder aux musiques" | check (also the *Options > Musique* menu tick) | `Musiques.ini Jouer` | 1 |
| Sons | Lecteur de musique (1): Windows Media Player (Lent, lit les MIDI, WAVE, MP3) / DirectMusic (Rapide, ne lit que les MIDI) | radios | `Musiques.ini UtiliserDirectMusic` | 0 (WMP) |
| Cartes | Générer le fichier "Informations.ini". + "Accéder aux Cartes" | check | `Cartes.ini CreerFichierInformations` | 1 |
| Réseau | Activer le moteur réseau. (1) | check (shows the 2 network tabs) | `Reseau.ini Actif` | 1 |
| Réseau | Ouvrir un port de communication. (1) | check | `Reseau.ini OuverturePort` | 1 |
| Réseau | Adresse du serveur : | text | `Reseau.ini URL_Serveur` | lobby URL |
| Sauvegardes | Sauvegarder le personnage actif toutes les minutes. | check | `SavPerso.ini SauvegardePeriodique` | 1 |
| Sauvegardes | Sauvegarder la partie en quittant. | check | `Francais\SavMonde.ini SavMondeEnQuittant` | 1 |
| Sauvegardes | Sauvegarder un aperçu de la partie. | check | `Francais\SavMonde.ini SavImageEnQuittant` | 1 |
| Statistiques | Enregistrer les statistiques. | check; enables the next one | `Documents.ini EnregistrerStastistiques` | 1 |
| Statistiques | Afficher en fin de partie. | check | `Documents.ini AfficherStastistiquesEnQuittant` | 0 |
| Statistiques | Consulter les statistiques. | button → `Doc_Stastistiques.html` | – | – |
| Statistiques | Navigateur web : (hidden) | combo IExplorer / FireFox | `Documents.ini NavigateurChoisi` | 1 |

The Options pages that only act in game (Interface, Messages, Personnages, Terrain…) are documented here only as persisted settings.
Their in-game effects are specified with the corresponding subsystems.

---

## 7. Documents (ModDocuments.bas)

Each entry of the *Documents* menu generates a static HTML file in the game folder and opens it in the browser
(`Ouvir_PageWeb`). Templates come from `Documents.ini`:

* `PageDebut="<HTML><BODY>"`, `PageFin="</TABLE></BODY></HTML>"`;
* rows `<TR>…</TR>`, cells `<td align=center>`;
* bold `<B>`, line break `<BR>`;
* the first cell opens `<table width=100% border=1 cellspacing=0;>` (ModDocuments.bas:1482-1507).

| Menu | File | Columns |
|---|---|---|
| Bâtiments F1 | `Doc_Batiments.html` | EPOQUE, NOM, PRIX, METIER M / F, TAILLE, VIE, STOCK, FABRIQUE (sorted by `Batiment_Tri`, an epoch heading when it changes) |
| Bestiaire F2 | `Doc_Bestiaire.html` | NOM, VIE, ATTAQUE, DEFENSE, VITESSE, EFFET ATTAQUE, XP, PARTICULARITE |
| Objets F3 | `Doc_Objets.html` | COMPETENCE, IMAGE, NOM, DISPONIBILITE, COUT, PRIX, BONUS, EFFET, PARTICULARITE |
| Ressources F4 | `Doc_Resssources.html` (sic) | IMAGE, NOM, COMPETENCE, MATIERES PREMIERES, VALEUR, VITESSE RECOLTE, REGAIN, PARTICULARITE, OBJETS |
| Races F6 | – | opens FrmInfoRaces (§5.2) |
| Peuples F7 | `Doc_Peuples.html` | NOM, FORTERESSE, CREATURES, IA (one row per monster of the people) |
| Statistiques F8 | `Doc_Stastistiques.html` | written at the end of a game if enabled (PERSONNAGES, FRAGS, MORTS, NIV+, GAIN CARACS, GAIN COMPETENCES, BESTIAIRE). F8 only opens it, or shows "Aucun fichier de stastistiques n'a été trouvé." |
| Artworks F9 | `Doc_Artworks.html` | every image of `Images\Chargement\` with its file name |
| ? > Résumé F11 | `Doc_Resume.html` | "Imperator x.y", then counts of epochs, playable races, buildings, skills, peoples, monsters, resources and objects |
| Personnages > Exporter | `Doc_Perso_<Nom>.html` | single-column sheet: name; level/race/job; CARACTERISTIQUES (max values); COMPETENCES (copied from the skills list); BESTIAIRE with total |

In the browser port these can be in-app pages generated from the same data. A literal HTML file is not needed.

---

## 8. Port notes / quirks to keep or decide on

* The organisation to reproduce: a menu bar, a world tab strip on the left, a character panel on the right, and Retour/Jouer
  buttons at the bottom right. The 8-button main menu is optional, since it is off by default.
* World creation is driven by the **focused** character (the controlled one) plus the **selected** characters, which are all
  inserted as team members (§3.3). The team of each character is a per-character setting (the "Peuple" combo in the character
  box, `Equipe` in the `.per`), which must match a fief or army team in the setup. Otherwise the game aborts with
  "pas assez de place".
* Fief 1's team combo is locked. Every fief defaults to its own team, so by default all selected characters go to team 1 = fief 1,
  and fief 1 must have at least as many villagers as selected characters.
* The Width/Height sliders are used even when a map is loaded (`Monde.Largeur/Hauteur` are then overwritten by `Charger_Monde`).
* With *Charger*, the number of fiefs equals the castles in the map. The peuple/team rows still come from the UI rows 1..N.
* The setup screen saves character changes instantly. Options only save on OK. `Jeu.ini` 2P flags and `Reseau.ini` save instantly.
* Network tabs, DirectPlay and the lobby (`Reseau.ini`) are probably out of scope in the browser. If so, use
  `TabStripMonde2` (6 tabs), i.e. `Actif=0`.
* All file lists are alphabetical. `Partie.ini` remembers list **positions**, not names, so restoring after files are
  added or removed can select the wrong item. A port may store names instead.
