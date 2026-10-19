# 05 — In-game rendering and interface

Scope: everything drawn on screen while a game is running (`FrmMoteur2D.MoteurJeu`), plus
mouse/keyboard handling and sounds/music. Line numbers refer to the VB6 sources of
Vendetta 3.0.0.17 (`FrmMoteur2D.frm:123` = file:line). "INI" files live in
`Vendetta/Donnees/` (language-specific ones in `Donnees/Francais/`), images in
`Vendetta/Images/`.

VB colours are written `&HBBGGRR` (blue in the high byte). This document always gives the
converted **RGB** value as well, e.g. `&HC00000` = RGB(0,0,192) (dark blue).

## 0. Global facts you must reproduce

* **Display**: DirectDraw7 fullscreen, `SetDisplayMode W, H, 16` (`ClsPerEcran.cls:190`):
  the original runs in **16-bit colour (RGB565)**. All colours (bitmaps, fills, text) are
  therefore quantised to 5/6/5 bits. For a pixel-exact look, quantise final colours
  (`r & 0xF8, g & 0xFC, b & 0xF8`), or at least be aware of it.
* **Resolution**: `Ecran.ini` `Resolution=3` → 1024×768 (the 5 choices are 640×480,
  800×600, 1024×768, 1280×1024, 1600×1200). Each resolution has its own
  `Interface<W>.ini` (HUD layout) and `Images/Fond<W>.bmp` (initial back-buffer
  content; `Fond1024.bmp` is 1024×768 plain black).
* **No camera zoom**: `Zoom` keys exist (`*`, `+`, `-`) but `Changer_Zoom` is never called
  (`FrmMoteur2D.frm:1319` commented out). Ignore zoom.
* **The back buffer is never cleared** between frames (`BltColorFill` commented out at
  `FrmMoteur2D.frm:1182`). The terrain blit covers the whole screen every frame, so in
  practice it doesn't matter as long as the world is at least as large as the screen.
* **Colour key transparency**: every sprite surface is created with `DDSD_CKSRCBLT` and a
  zero colour key (`ClsAffSprite.cls:68`, `ClsAffTerrain.cls:57`), and blitted with
  `DDBLTFAST_SRCCOLORKEY`. ⇒ **pure black RGB(0,0,0) is transparent** in every image
  (terrain, sprites, buttons, windows, cursors…), unless a blit is explicitly done with
  `DDBLTFAST_NOCOLORKEY` (`ClsAffSprite.Afficher` with `Transparent=False`). In 16-bit mode
  any colour that quantises to 0 (r<8, g<4, b<8) is also transparent — e.g. near-black
  RGB(4,2,6) pixels in BMPs become transparent too. Implement as: after loading, set alpha=0
  for pixels whose RGB565 value is 0.
* **Translucency (ghosts)**: `ClsAffSprite.Afficher(..., Translucide=True)`
  (`ClsAffSprite.cls:100-124`) blends every non-zero source pixel 50 %/50 % with the
  destination, per channel (`new = src*0.5 + dst*0.5`, masked per 565 channel). Used for
  dead characters (`Not .Vivant`, `FrmMoteur2D.frm:1952,1965`). Note: the translucent path
  ignores the `DecalageX/DecalageY` crop parameters.
* **Timing** (`Jeu.ini`): `Vitesse=10` → the simulation ("tick") runs at 10 Hz
  (`FrmMoteur2D.frm:988`: a tick happens when `Timer - TempsDeb > 1/Vitesse`).
  `AffichageRapide=1` → a frame is rendered on **every** main-loop iteration
  (`FrmMoteur2D.frm:1179-1180`), i.e. as fast as possible (`FPS=60` is only used when
  `AffichageRapide=0`). All animations (walk cycle, buildings, effects, damage numbers,
  terrain water) advance per **tick**, not per frame. Mouse/keyboard are polled every
  loop iteration.
* **Clipping**: sprites are clipped against `ScreenWidth`×`ScreenHeight` (whole screen),
  there is no other viewport (`ClsAffSprite.cls:126-151`).

## 1. Screen layout at 1024×768 (overview)

There is **no fixed HUD panel**: the map fills the entire 1024×768 screen and every HUD
element is drawn on top of it (see the screenshot `bw-game.png`). From `Interface1024.ini`:

| Element | Position at 1024×768 |
|---|---|
| Map view | (0,0)–(1023,767), whole screen |
| Movement/order buttons (18, 2 columns × 28 px, auto-hidden to the left) | x=0, y=0, pitch 28 px |
| Order buttons (5) | x=56, y=112, pitch 28 px vertical |
| Build buttons (3 columns of 28×28, auto-hidden) | x=0, y=232, rows every 28 px, columns every 28 px |
| Temporary-effect icons | x=56, y=0, every 32 px to the right |
| 9 object shortcut slots ("1".."9") | top-left, see HUD section |
| Epoch name box ("Age de bois") | x=528, y=0, colour &H808080 = RGB(128,128,128) |
| Info buttons (9, 24×24, right-to-left) | x=888, y=0, step −30 px |
| Menu buttons (2, 32×32) | x=678, y=24, step +32 px vertical |
| FPS "FPS :12" | x=940, y=0, white |
| Windows (320×368 each) | slot 1 (704,0), slot 2 (704,368), slot 3 (64,0), slot 4 (64,368), slot 5 (384,0), slot 6 (384,368), slots 7/8 (64,0) |
| Status bars Vie/Energie/Magie/Moral (136×16) | x=10, y=691/709/727/745 |
| Action message ("En attente") | x=152, y=745, white |
| Team statistics ("(1) Humains 50/50 - 100%") | x=816, y=745, white |
| Message log | see HUD section |

Details of every element are in section 4 (HUD) and section 5 (windows; the minimap is section 5 → "Window 6 — Carte").

Split screen (2 players on one computer, `Jeu.ini Mode2Joueurs`) is out of scope for the
first port; where relevant: each half is `W/2` wide, a 2-px separator is drawn at x=W/2−1..W/2+1
(`FrmMoteur2D.frm:2761`), and `Interface1024_J1/J2.ini` give the layouts.

## 2. Map projection and camera

### 2.1 Projection

* **Orthogonal top-down, NOT isometric.** One map cell = **16×16 px**
  (`Donnees/Francais/Terrain.ini`: `LargeurCase=16`, `HauteurCase=16`; stored in
  `Monde.LargeurCase/HauteurCase`, copied into `AffTerrain.Largeur/Hauteur`,
  `FrmMoteur2D.frm:349`).
* World coordinates are pixels. Cell `(cx, cy)` covers world pixels
  `[16cx, 16cx+16) × [16cy, 16cy+16)`; its array index is `k = cy * Monde.Largeur + cx`.
* World → screen: `screen = world − Decalage` (+ `ScreenPositionX` = 0 in single-player).
  Every entity has a top-left `PositionX/PositionY` and is drawn with its sprite's top-left
  at `(PositionX − DecalageX, PositionY − DecalageY)` (`FrmMoteur2D.frm:1762, 1820, 1832,
  1844, 1942`).
* Screen → world (mouse): `world = mouse + Decalage`.
* World → cell (`ClsJeuMonde.TerrainIndice`, `ClsJeuMonde.cls:494`):
  `k = CLng(X/16) + CLng(Y/16) * Monde.Largeur` — note VB `CLng` **rounds** (banker's
  rounding), it does not truncate, so this lookup is offset by half a cell. Used for
  terrain speed/constructibility/sprite offset, not for drawing.

### 2.2 Camera

* Camera = `DecalageX(0), DecalageY(0)` = world coordinate of the screen's top-left pixel.
* **The camera is locked on the player's character**, recomputed **once per tick** (10 Hz)
  by `Scrolling_Immediat` (`FrmMoteur2D.frm:1080, 1644-1666`):
  ```
  DecalageX = Perso.PositionX − W/2      (W = 1024)
  clamp DecalageX to [0, Monde.Largeur*16 − W]
  DecalageY = Perso.PositionY − H/2      (H = 768)
  clamp DecalageY to [0, Monde.Hauteur*16 − H]
  ```
  (VB integer assignment rounds the result). Note it centres the character's **top-left
  corner**, not its centre. Upper clamp is applied first, then the `< 0` clamp.
* **There is no edge scrolling and no keyboard scrolling.** Arrow keys move the
  character (see §6.3), not the camera.
* The only manual camera control: while the **Map window** (window 7 / info button 6) is
  open, **right-clicking (held) on the minimap** sets
  `DecalageX = mapX/mapW * Monde.Largeur*16 − W/2` (same clamping, `FrmMoteur2D.frm:2904`)
  and likewise for Y, and sets `ScrollingAuto = False` (`ClsInterface.cls:2790-2797`).
  While `ScrollingAuto` is False, `Scrolling_Immediat` does nothing (camera frozen).
  `ScrollingAuto` returns to True when the map window is closed
  (`ClsInterface.cls:4731`) or all windows are closed (Space, `ClsInterface.cls:4750`).
* Because the camera only updates at 10 Hz while the character also moves at 10 Hz, the
  view moves in steps of the character's step length; this is the original look.

## 3. Draw order, sprites, terrain

### 3.1 Frame composition (single player) — `FrmMoteur2D.frm:1189-1232, 1325-1508`

Each rendered frame, in this exact order (later = on top):

1. **Terrain** — `AffTerrain.AfficheFast` (or `AfficheVeryFast` when
   `TerrainOptimise=1`; default is 0) (`:1190-1194`).
2. **Resources** (harvestable nodes) — `Afficher_Ressources` (`:1816`), array order.
3. **Buildings** (houses, `Maisons()` in index order) and their **life bar** when damaged,
   then **castles** (`Chateaux()`) with life bar and the vertical **epoch-change progress
   bar** (`Afficher_Batiments`, `:1756-1814`).
4. **Decors** (bridges, flags, fountains, graves) — `Afficher_Decors` (`:1840`), collection
   order.
5. **Treasures** — `Afficher_Tresors` (`:1828`).
6. **Selection marker of the player** (if no army is selected): ellipse at the feet, or a
   rounded box around the building he is in — `Afficher_CadreSelection Noperso, 0, True`.
7. **Drag rectangle + selection ellipses of selected soldiers** —
   `Afficher_CadreSelection_Soldats` (`:2186`).
8. **Work-progress bars** — `Afficher_Etat_Travail` (`:2342`).
9. **Characters** — `Afficher_Persos` (`:1853`), iterating `i = UBound(Persos) To 0 Step −1`
   (**reverse index order, no Y-sorting**). For each visible character: its 5-px status
   bars are drawn *before* its sprite (so its own sprite covers nothing of them since they
   are below the sprite).
10. **Visual effects** — `Afficher_Effets_Visuel` (`:2042`): house effects (fire),
    per-character effects, then free effects; plays their sound on step 1.
11. **Day/night overlay** — `AffTemps.Afficher` (disabled by default: `Temps.ini Afficher=0`).
12. **Floating damage numbers** — `AffDegat.Afficher_Degats`.
13. **Speech comments** above characters — `Afficher_Commentaires` (`:2451`).
14. Window contour while a window is being dragged (computed), window repositioning.
15. **Message log** — `Messages(0).Afficher_Messages`.
16. **HUD + windows** — `Interface(0).Afficher ...` (`:1222-1230`), then
    `Interface(0).Interieur_Boutons` (hover/click handling, which also draws tooltips).
17. If a window is being moved: its outline, then the cursor (type 0) (`:1345-1350`).
    Otherwise: the **object being carried** by the mouse (`Afficher_Objet_Curseur`, at
    `mouse − 12`), the **construction footprint** (`Afficher_Surface_A_Batir`), then the
    **mouse cursor**, then the **terrain info-bubble** (`Afficher_Infos_Bulles_Terrain`).
18. **FPS** text (`Interface.Afficher_FPS`).
19. **Victory / defeat banner** if applicable: `Images/Fenetres/VictoireDefaiteFR.bmp`
    (320×400: rows 0–199 = victory, 200–399 = defeat), drawn colour-keyed at
    `x = (W − 320)/2`, `y = (H − 200)/2 − 128` → **(352, 156)** at 1024×768
    (`FrmMoteur2D.frm:1495-1499, 1169-1171`).
20. Flip (`Rafraichir_Ecran`).

### 3.2 Terrain — `ClsAffTerrain.cls`, `ClsJeuMonde.cls`

**Tileset**: `Images/Terrains/Plaine.bmp`, 256×224, 8-bit. It is a grid of 16×16 tiles:
**column = "apparence" (shape, 0..15), row = "catégorie" (terrain type, 0..13)**
(`ClsAffTerrain.Afficher`, `:83-86`: src rect = `(16*apparence, 16*categorie, 16, 16)`).

Rows (terrain types from `Terrain.ini [Terrain_n]`):

| Row | Type | Notes |
|---|---|---|
| 0 | Plaine (plain grass) | the background; all 16 columns are plain grass |
| 1 | Herbe (tall grass) | |
| 2 | Terre (dirt) | |
| 3 | Désert (sand) | |
| 4 | Montagne | inconstructible, speed 0.5 |
| 5 | Route (cobblestone road) | speed 1.25 |
| 6–9 | Muraille1..4 (walls) | row 6 columns 2–7 are pure black = transparent |
| 10 | Neige (snow) | |
| 11 | Eau (water) | `Animation=1`, `ImgPersoDecalageY=12`, effect 159 |
| 12, 13 | water animation frames 2 and 3 | used only if `TerrainAnime=1` |

Every tile already contains its transition to the plain-grass background (the shape is
painted over green grass), so **there is no runtime blending or layering**: each cell is
exactly one 16×16 tile blitted with colour key (black transparent; no tile except row 6
has black, so effectively opaque).

**Shape ("apparence") selection** — `ClsJeuMonde.Reformer_Terrain` (`ClsJeuMonde.cls:328-418`),
run after map generation/road building. For each cell `i` whose category `> 0`, look at
the 4 orthogonal neighbours **U = i−Largeur, D = i+Largeur, L = i−1, R = i+1**; a
neighbour "is set" if its category is **> 0 (any non-plain terrain, not necessarily the
same type!)**. Out-of-array indexes count as 0. Note: L/R are computed on the flat array,
so they wrap to the previous/next row at the map's left/right edge.

| U | D | L | R | apparence (column) | meaning |
|---|---|---|---|---|---|
| 1 | 1 | 1 | 1 | 1 | full |
| 0 | 1 | 0 | 0 | 2 | tip pointing up |
| 1 | 0 | 0 | 0 | 3 | tip pointing down |
| 0 | 0 | 0 | 1 | 4 | tip pointing left |
| 0 | 0 | 1 | 0 | 5 | tip pointing right |
| 1 | 1 | 0 | 0 | 6 | vertical bridge |
| 0 | 0 | 1 | 1 | 7 | horizontal bridge |
| 0 | 1 | 1 | 1 | 8 | notch at top (top edge) |
| 1 | 0 | 1 | 1 | 9 | bottom edge |
| 1 | 1 | 0 | 1 | 10 | left edge |
| 1 | 1 | 1 | 0 | 11 | right edge |
| 0 | 1 | 0 | 1 | 12 | top-left corner |
| 0 | 1 | 1 | 0 | 13 | top-right corner |
| 1 | 0 | 0 | 1 | 14 | bottom-left corner |
| 1 | 0 | 1 | 0 | 15 | bottom-right corner |
| 0 | 0 | 0 | 0 | — | cell is turned into plain: `TerrainCategorie(i) = 0` |

Cells with category 0 keep apparence 0 (row 0 is uniform grass, so any column looks the
same). Column 0 of the other rows (an isolated blob) is only reachable through saved/loaded
maps. The pass is a single sequential sweep over `i = 0..UBound`; a cell reset to 0 in the
sweep affects the cells processed after it (reproduce the sequential order).
Apparence/category arrays are saved with the world, so loaded worlds use stored values.

**Drawing** — `AfficheFast(PositionX=0, PositionY=0, DecalageX, DecalageY, W, H)`
(`ClsAffTerrain.cls:289-345`):
```
for i = 0 to W/16 (inclusive, VB-rounded):         # columns
    k  = Int(DecalageY/16) * Monde.Largeur + i + Int(DecalageX/16)
    cX = i*16 − (DecalageX mod 16)
    for j = 0 to H/16 (inclusive):                  # rows
        cY = j*16 − (DecalageY mod 16)
        src = (16*apparence[k], 16*(categorie[k] + (animated[k] ? EtapeAnimation2 : 0)), 16, 16)
        clip src to the screen; blit at (max(cX,0), max(cY,0)) with colour key
        k += Monde.Largeur
```
i.e. a plain tile map; 65×49 tiles at 1024×768.

**Water animation** (`ClsAffTerrain.Animation_Suivante`, `:91-108`; only when
`Terrain.ini TerrainAnime=1` — **default 0, so water is static, row 11**): a counter
increases each tick; every `Vitesse+1` ticks (11 ticks = 1.1 s) the frame index
`EtapeAnimation2` ping-pongs 0→1→2→1→0… (`NombreEtapesAnimation=3`), added to the row of
cells whose terrain type has `Animation=1`.

`AfficheVeryFast`/`Modeliser_Carte` (`TerrainOptimise=1`) only pre-renders the same tiles
into ≤512×512 chunks for speed; visually identical. Do not port.

**Character sinking in water**: when a living, non-flying character's feet cell
(`TerrainIndice(PositionX, PositionY + Hauteur − 16)`) has `ImgPersoDecalageX/Y`
(water: 0/12), the sprite is drawn shifted by that offset **and cropped**: with
DecalageY=12 the sprite's frame rows 0..Hauteur−13 are drawn at `Y+12` (the bottom 12 rows
are cut off), giving a wading effect (`FrmMoteur2D.frm:1953-1954`,
`ClsAffSprite.cls:130-152`). Same for X (crop on the right for positive offsets).

### 3.3 Generic sprite blit — `ClsAffSprite.Afficher` (`ClsAffSprite.cls:90-156`)

`Selection(top, bottom, left, right)` selects the source rectangle; `Afficher(X, Y,
Transparent, Translucide, DecalageX, DecalageY)`:
* if `X<0` crop the left part and draw at x=0; crop right part beyond `ScreenWidth`;
  same for Y. `DecalageX<0` crops `−DecalageX` columns on the left; `DecalageX>0` crops
  `DecalageX` columns on the right; final position `X+DecalageX, Y+DecalageY`.
* `Transparent` → colour key black.

### 3.4 Characters — `ClsAffApparence.cls`, `ClsAffPerso.cls`

**Frame size**: `Largeur × Hauteur` of the race (`Parametres_Races.ini`, e.g. 24×32 for
humans). All character sheets are **72×128 = 3 columns × 4 rows of 24×32** frames.

**Sheet layout** (`ClsAffApparence.Afficher_Element`, `:314-350`; `ClsAffPerso.Afficher2`,
`:204-243`):
* row = `Direction − 1`, with `Direction` **1 = up (back view), 2 = right, 3 = down (front
  view), 4 = left**;
* column = `NumeroPas` for `NumeroPas ∈ {0,1,2}`, and **column 1 when `NumeroPas = 3`** ⇒
  the walk cycle shows columns **0,1,2,1,0,1,2,1…**;
* `Direction = 0` (idle/no direction) → row 2 (front), column 1.

**Walk-cycle timing**: `NumeroPas` increments by 1 (mod 4) **once per tick** while the
character is moving (`EnDeplacement`) or is a flying unit (always flapping)
(`ClsJeuPerso.cls:1186-1195`, `Changer_Pas` `:1262-1268`). At 10 ticks/s one full
cycle = 0.4 s. When stopped, the last `NumeroPas` stays (so the idle pose may be
column 0, 1 or 2). Initial `NumeroPas = Int(Rnd*3)`.

**Direction** (`ClsJeuPerso.Orienter_Direction`, `:1404-1462`): with `dx = target −
PositionX`, `dy = target − PositionY` and dead-zone `s = Longueur_Pas/2`:
if `dy < −s` → up unless `|dx| > s` and `|dx| ≥ |dy|` (then left/right);
if `dy > s` → down unless `|dx| > s` and `|dx| ≥ |dy|`; else left/right if `|dx| > s`;
else not moving.

**Layered appearance** (`AffApparence.Afficher`, `:254-312`; used when the character has
no single-file "ApparenceChoisie"): a character is drawn as a stack of 72×128 sheets,
each blitted with the same frame rectangle at the same position:
1. the race's base layers `ApparenceBase(0..Nombre_ApparenceBase−1)` (e.g. body, clothes,
   hair; folder list per race from `Parametres_Races.ini`), in that order;
2. if `ApparenceObjetAfficher`, the equipment layers `ApparenceObjet(1..N)` (weapons,
   shields, wings, capes — `Images/Apparences/Objets/...`), in order.
Sheets are loaded from all files in all sub-folders of `Images/Apparences/` (Cheveux,
Corps, Habits, Monstres, Objets, Races…).

**Mounts** (`Monture > 0`): the mount sheet (size `ApparenceLargeur × ApparenceHauteur`)
is drawn at `(X + ApparenceDecalageXTemp, Y + ApparenceDecalageXTemp)` (**sic**: the VB
code uses the X offset for Y too, `:276, :290`) with the walk `NumeroPas`; the rider
layers are drawn with **fixed column 1** at `(X + ApparenceDecalageX(Direction), Y +
ApparenceDecalageY(Direction))` where `Definir_ApparenceDecalageY` subtracts 1 when
`NumeroPas=1` (bobbing, `ClsJeuPerso.cls:5238`). Draw order: if
`ApparenceAvantPlan(Direction)` is true the mount is drawn **first** (i.e. behind the
rider); otherwise it is drawn at its position in the equipment-layer loop (after the base
layers, among the objects; only if `ApparenceObjetAfficher`).

**Single-file heroes** (`ChoisirApparence = True`, `ClsAffPerso`): one 72×128 sheet from
`Images/Persos/<folder>/<file>.bmp`, same frame layout.

**Dead characters** (ghosts) are drawn translucent 50 %. Visibility condition
(`FrmMoteur2D.frm:1862-1865`): `Visible And (Vivant Or (isPlayer And AfficherFantomes) Or
(AfficherFantomesPNJ And sameFiefAsPlayer)) And (Not Invocation Or Vivant)`. With the
default `Jeu.ini` (`AfficherFantomes=0`, `AfficherFantomesPNJ=0`) **dead characters are not
drawn at all**, not even the player's ghost.

**No player colours**: nothing in the renderer tints sprites per player/team. Teams are
distinguished only by appearance and by cursor/tooltip semantics.

**Overhead bars per character** (`FrmMoteur2D.frm:1898-1940`) — drawn with
`Interface.Afficher_Barre_Statut` (exact bar style in the HUD section), all `Largeur` px
wide and 5 px high, just **below** the sprite:
* life bar at `y = PositionY + Hauteur` if `Vie < MaxVie` or `Attaque < MaxAttaque` or
  `Defense < MaxDefense` or attack reloading;
* if attack/defence not full or reloading: attack bar (red `&HFF&` = RGB(255,0,0)) at
  `+5`, or, while reloading, a default-colour bar showing
  `(ReloadMax − Reload)/ReloadMax`; then defence bar (blue `&HFF0000` = RGB(0,0,255)) at
  `+10`.

**Selection ellipse** (`Afficher_CadreSelection`, `FrmMoteur2D.frm:2110-2184`): hollow
(`FillStyle 1`), 1-px outline colour: **red RGB(255,0,0)** if the character is in combat,
**white** if inactive (`Inactif`), else **dark blue `&HC00000` = RGB(0,0,192)**. For a
character of size L×H at screen (x,y), GDI ellipse bounding box:
`(x + L/12, y + H − L/3 − H/8) – (x + L − L/12, y + H + L/3 − H/6)`
(24×32 → (x+2, y+20)–(x+22, y+34.67)). For mounted characters the same formula uses
`ApparenceLargeur/Hauteur` and `ApparenceDecalageX/YTemp`. If the character is inside a
building (own house / a house / own castle / a castle), a **rounded rectangle** (corner
radius 10) around that building's rectangle is drawn instead (that's the white frame in
the screenshot around the player's house). Drawn for the player (when no army is selected)
and for every selected soldier.

### 3.5 Buildings — `ClsAffBatiment.cls`

* Houses: one sheet per building type, `Parametres_Batiments.ini` `Image=` under
  `Images/Batiments/`. Frame size = `Largeur×Hauteur` of the type (mostly 64×64 or 96×96).
  **Row = state (`Etat`), column = animation step**
  (`Sprites(Type).Selection Etat*H, (Etat+1)*H, L*Anim, L*(Anim+1)`, `:79-84`).
* `Etat` (`ClsJeuBatiment.cls:388-399`): while under construction
  `Int(Vie / MaxVie * NombreEtapeFabrication)` (rows 0..N−1 = construction stages);
  once built: row `NombreEtapeFabrication` (normally `N=5` → row 5, sheet 6 rows tall),
  or row `N−1` if it is a shop/factory/service with stock and `Stock < 1` (empty shop).
* Animation: `NumeroAnimation` increments every tick and wraps at
  `NombreEtapeAnimation` (default 1 → static; a few buildings have 4)
  (`ClsJeuBatiment.cls:401-406`).
* Castles: `Images/Batiments/Chateaux/<Peuple>.bmp`, 128 px wide. Type = people
  (`Fiefs(i).Definir_TypePeuple`), **row = epoch for people 0 (Humains, 128×640 = 5
  epochs), otherwise row 0** (`FrmMoteur2D.frm:1786-1792`).
* Damaged building: 5-px life bar at its bottom edge, full width
  (`y = PositionY + Hauteur`). Castle changing epoch: vertical 5-px bar (white,
  `&HFFFFFF`) at the castle's right edge (`x = PositionX + Largeur`), height = castle height.
* Burning: when a built house has `Vie < MaxVie/2`, the fire effect
  (`EffetMaisonEnFeu = 4` → "Mur de feu") is (re)started centred on it.

### 3.6 Resources, treasures, decors, effects

* **Resources** (`ClsAffRessources`): `Images/Decors/Ressources.bmp` 256×32 = 8 frames of
  32×32 (bush/herbs, apple tree, tree, stone, ore, crystals, gold, rock), frame =
  `Ressources(i).Apparence`. Positioned with top-left at `(16*cx, 16*cy − 32)` for map
  cell (cx,cy) (`ClsJeuCartes.cls:374-375`).
* **Treasures** (`ClsAffTresors`): `Images/Decors/Tresors.bmp` 182×26 = 7 frames of 26×26,
  frame = `Tresors(i).IndiceImage`.
* **Decors** (`ClsAffDecors`): one image per decor type (`Parametres_Decors.ini`):
  Pont 71×88, Drapeau 31×81, Fontaine 89×86 (4 frames horizontally, 356×86), Tombe
  ("Pierre tombale3") 23×17, Tombe2 ("Croix") 14×22; frame = `EtapeAnimation`, advanced
  every tick, wrapping at `NombreEtapeAnimation` (`ModDecors.bas:54-61`). Graves are
  temporary (`Duree=60` ticks).
* **Effects** (`ClsAffEffet`, `ClsJeuEffet`): `Images/Effets/<Fichier>.bmp`, frames laid out
  horizontally, `Largeur×Hauteur` each (`Parametres_Effets.ini`). Activation centres the
  effect on its target rectangle: `PositionX = X + (Ltarget − Leffet)/2`, same for Y
  (`ClsJeuEffet.cls:40-55`). Each tick `Temps += 1/DureeEtape`; active while
  `Temps < NombreEtapes`; displayed frame = `CInt(Temps)` (VB rounding to nearest even!)
  — with `DureeEtape=1` frames 0,1,2,…,N−1, one per tick. Note `Effet_n` sections are
  1-based in the INI; code indices are `n−1` (`ClsJeuParam.cls:1810`).

## 4. HUD (ClsInterface and friends)

Sources: the VB6 sources (UTF-8 copies). File:line citations refer to those files.
Target resolution: **1024x768**, single player, `Interface1024.ini`, unless noted otherwise.
The contents of the 8 windows are covered elsewhere (ClsIntFenetre). This file covers how ClsInterface
positions and calls them.

---------------------------------------------------------------------------------------------------
### 0. Conventions and primitives

#### 0.1 Colours
VB colour literals are `&HBBGGRR`. Converted values used by the HUD:

| VB literal | RGB | Used for |
|---|---|---|
| `&HFFFFFF` | (255,255,255) | most HUD text, FPS, statistics, message log, speech, epoch progress bar, temp-effect duration bar |
| `&H808080` (`EpoqueCouleur`) | (128,128,128) | epoch box outline and epoch text |
| `&HC0C0C0` (`BarresStatutContourCouleur1`) | (192,192,192) | status bar outer contour (auto-restore OFF) |
| `&H80FF` (`BarresStatutContourCouleur2`) | (255,128,0) | status bar outer contour and threshold marker (auto-restore ON) |
| `&H80` | (128,0,0) | bar fill when ratio < 0.10 |
| `&HFF` | (255,0,0) | bar fill when ratio < 0.25; `&` red text; attack bar; damage numbers |
| `&H80FF` | (255,128,0) | bar fill when ratio < 0.50 |
| `&HC000` | (0,192,0) | bar fill when ratio >= 0.50 |
| `&HFF0000` | (0,0,255) | defence bar; `$` blue text; heal numbers; chat input text |
| `&HFFFF` | (255,255,0) | "sale" damage numbers (`CouleurVente`) |
| `&HC0FFFF` (`CouleurRectangleFond`) | (255,255,192) | tooltip / info-bubble background (pale yellow) |
| `&H0` | (0,0,0) | tooltip text and outline, bar frames, "Nb:" soldier count, confirmation text |
| `&HE0E0E0` | (224,224,224) | chat input box fill |

The Boxedwine screenshot runs in 16-bit colour, so you will see e.g. `c5c2c5` for C0C0C0 and `00c200`
for 00C000. Use the exact 24-bit values above.

#### 0.2 Text: `ClsIntMessage` (global instance `AffMessage`, ModMain.bas:80,192)
Source: ClsIntMessage.cls.
- Constants (ClsIntMessage.cls:33-44): line-break char `#`; colour char 1 `&` gives `&HFF`, i.e. **red (255,0,0)**;
  colour char 2 `$` gives `&HFF0000`, i.e. **blue (0,0,255)**; `InterligneY = 16` (line pitch 16 px, no X shift);
  `LargeurCaractere = 7` and `HauteurCaractere = 16`. These are the nominal char cell sizes used by every
  layout formula (tooltip box size, centring, and so on).
- `Afficher(X, Y, Message, Surf, [Couleur], [ChangerCouleur])` (ClsIntMessage.cls:46-77), a recursive algorithm:
  1. If `ChangerCouleur` is false, `Couleur := current surface ForeColor`.
  2. Look at the **first char of the (remaining) string only**. If it is `&`, strip it and use red. If it is `$`,
     strip it and use blue. Otherwise use `Couleur`.
  3. If the string contains `Chr(13)` (CR), draw the text before the first CR at (X,Y), then recurse on the rest at
     (X, Y+16) with `Couleur` (so the colour resets to the base colour on every new line unless that line starts
     with `&`/`$`). CR is tested **before** `#`. If a string has both, it is split at the first CR, and any `#`
     before it is printed literally.
  4. Otherwise, if it contains `#`, split the same way.
  5. Otherwise draw the whole string. Finally restore ForeColor to `Couleur`.
  - `&`/`$` only work at the start of a line. Elsewhere they are printed literally.
- `Nombre_Lignes(s)` = (number of `#`) + 1 (CR not counted) (ClsIntMessage.cls:79-87).
- Drawing is `DirectDrawSurface7.DrawText x, y, s, False`. GDI text uses a **transparent background** (see
  the screenshot), with no shadow and no outline.

#### 0.3 Fonts (ClsAffDegats.cls:50-77, FrmChargement.frm:490-545, Combat.ini [Degats])
Three IFont objects are taken from hidden labels. All are **Arial, Bold (weight 700)**:
- `FontNormale`: Arial Bold **10 pt** (size forced to 10 at ClsAffDegats.cls:76). This is the **default HUD font**.
  `Afficher_Degats` ends with `SetFont FontNormale` every frame (ClsAffDegats.cls:144), so everything drawn
  after that point uses it.
- `FontDegats`: Arial Bold **8 pt** (`TailleNormale=8`). Used for damage numbers, slot counters "x5"/"(07)",
  the inventory "(1)" key labels, and the "n/m" charge counter.
- `FontCritique`: Arial Bold **10 pt** (`TailleCritique=10`). Used for critical damage numbers.

Measured in the screenshot for 10 pt bold at 96 dpi (13 px em): text drawn at (x,y) has its cap top at **y+2**
and its baseline at **y+11** (the bottom of the "V" in "Vie :" drawn at y=691 is row 702). A descender reaches
y+14 ("Energie" `g` at 723 for y=709). The first glyph's ink starts at x+1 for most letters ("Vie :" at x=10 has ink
from 10; "15 / 15" at x=87 has ink from 88). Layout code assumes a 7 px advance and 16 px lines. The real Arial
advance is about 7 px on average but varies, because the font is proportional.

#### 0.4 Rectangles (`DrawBox`) and GDI semantics. Important for pixel-exactness
`DrawBox l,t,r,b` is GDI `Rectangle`. The outline uses the ForeColor pen (1 px unless `setDrawWidth`). The
**right and bottom edges are exclusive**: the outline covers x∈[l, r-1], y∈[t, b-1].
When the pen is invisible (`setDrawStyle 5`), the fill covers x∈[l, r-2], y∈[t, b-2].
- Fill model (it is consistent with all observations): `SetFillColor c` creates a **solid** brush.
  `SetFillStyle 1` selects a **transparent** (null) brush. `SetFillStyle 0` is solid. `SetFillStyle 4` is an
  upward-diagonal hatch (build placement).
- Coordinates passed as Single/Double are converted to Long with **VB rounding (round-half-to-even)**.

#### 0.5 Sprites (ClsAffSprite.cls)
- All images are 8-bit BMP files under `Images\`. The surface is created with a source colour key equal to 0.
  **Pure black (0,0,0) is transparent** when the blit is "Transparent" (`DDBLTFAST_SRCCOLORKEY`).
  Otherwise the blit is opaque (ClsAffSprite.cls:63-66,147).
- `Selection(top, bottom, left, right)` picks the source rectangle (ClsAffSprite.cls:174-182).
- Clipping to [0, ScreenWidth] x [0, ScreenHeight] is done by shrinking the source rectangle
  (ClsAffSprite.cls:122-146). The blit position adds `ScreenPositionX/Y` (split-screen offset).
- Translucent path (ClsAffSprite.cls:94-121): for each source pixel `> 0` (non-black),
  `dst = src*a + dst*(1-a)` per channel. When `a <= 0` is requested, the call falls through to the default `a = 0.5`.

#### 0.6 Button sprite sheets (ClsAffBouton.cls, Boutons.ini)
`ClsAffBouton.Afficher(X, Y, TypeBouton, Etat, Surf, Transparent)` (ClsAffBouton.cls:199-217) selects
**column = TypeBouton (x = Largeur*TypeBouton)** and **row = Etat (y = Hauteur*Etat)**.

| Kind (Charger type) | File (Images\Boutons\) | Cell | Sheet | Used for |
|---|---|---|---|---|
| 0 construire | `Construire\<Batiment_ImageBouton>.bmp` (one file per building, demolish = `_Demolir`) | 28x28 | 28x84 (1 col x 3 rows) | build buttons |
| 1 Infos | `Infos.bmp` | 24x24 | 216x72 (9x3) | info buttons |
| 3 Deplacement | `Deplacement.bmp` | 28x28 | 644x140 (23x5) | move/order buttons |
| 7 Menu | `Menu.bmp` | 32x32 | 128x64 (4x2) | save/quit + confirmation yes/no |
| 10 Objets | `Objets.bmp` | 32x32 | 288x192 (9x6) | object shortcut slots |
| (others) | Action 24 (264x72), Commerce 24 (216x48), Transfert 24 (144x48), Defilement 16x44/16x198/16x16 (64x198), Competences 28x16 (112x32) | | | used inside the windows |

Sheet contents (checked visually):
- **Infos.bmp**: columns 0..8 = key (building), apple (resources), black figure (character), stone tablet
  (skills), money bag (inventory), skull (bestiary), parchment (map), blue down arrow (menu/pause),
  sealed letter (objectives). Row 0 = normal (bright), row 1 = "pressed / window open" (darker). Row 2 exists only
  in column 8: letter with a blue "N" for new objectives.
- **Deplacement.bmp**: columns 0..22 = crown + red arrow (castle), house + arrow (home), herbs, apples, wood
  (tree), stone, iron, crystals, gold, sulphur (each with a red arrow), sword (attack enemy), sword + house
  (attack house), sword + castle (attack castle), treasure chest, stone face (auto mode), red button (attack
  allies), red "no" circle (stop), 9 yellow dots + red X (formation), red cape (cover), boots (wait),
  shield (defence), crossed swords (attack), skull (berserk). Row 0 = coloured (normal). Row 1 = greyscale
  (pressed). For column 14, row 1 is a computer monitor (auto mode ON). Column 17 uses rows 0..4 =
  formation 0..4 (the red X moves around the dot grid). The other cells of rows 2..4 are empty black frames.
- **Menu.bmp**: col 0 = tablet "I" (save), col 1 = arrow into door (quit), col 2 = check mark (Oui), col 3 =
  cross (Non). Row 0 = bright, row 1 = dark (row = MouseOver).
- **Objets.bmp**: rows 0/1 = "1".."9" bright/dim, rows 2/3 = "Alt 1".."Alt 9" bright/dim, rows 4/5 = "Maj1".."Maj9"
  bright/dim. Each cell is a black square with a lilac double frame and the label at the top-left. The black
  interior is transparent.
- **Construire\X.bmp**: row 0 = grey hatched (cannot build), row 1 = coloured icon in a gold frame
  (buildable), row 2 = grey with red "II" bars (epoch not reached). `_Demolir`: row 0 grey, row 1 hammer in a
  green ring, gold frame.

`Fenetres\Fenetre-Confirmation.bmp` is 320x160. The `Temps\*.bmp` images are 512x512 each.

#### 0.7 Hit test (ClsInterface.cls:4623-4633)
`Interieur_Bouton(x,y,X2,Y2,W,H) = x>=X2 && x<=X2+W && y>=Y2 && y<=Y2+H`. Both ends are **inclusive**, so a
28 px button reacts on 29 px.

---------------------------------------------------------------------------------------------------
### 1. Frame composition (FrmMoteur2D.frm:1192-1497, single-screen branch)

The order in which things are drawn each frame:
1. Terrain, resources, buildings (+ building/castle life bars, castle epoch bar), decors, treasures, selection
   frames, work-state bars, characters (+ life/attack/defence bars), world effects.
2. `AffTemps.Afficher` day/night overlay (§10). It is OFF by default: `Temps.ini Afficher=0`.
3. `AffDegat.Afficher_Degats` floating damage numbers (§11).
4. `Afficher_Commentaires` speech text over characters (§12).
5. `Messages(i).Afficher_Messages` message log (§9).
6. `Interface(i).Afficher` (ClsInterface.cls:821-878), in this exact order:
   1. `Afficher_Cadre` (background frame image, `AfficherCadre=0` at 1024, so nothing).
   2. If `AfficherBarresStatut`: ForeColor white, then **Moral, Magie, Energie, Vie** (bottom to top), then
      `Afficher_Barre_Statut_Auto` (the 4 outer contours) (§4).
   3. `Afficher_Stastistiques` (§7).
   4. `Afficher_Epoques` (§5).
   5. `Afficher_Raccourcis_Objets` (object shortcut slots, §3.5).
   6. `Afficher_Effets_Temporaires` (§3.6).
   7. `Afficher_BoutonsDeplacement` (§3.3).
   8. `Afficher_BoutonsConstruire` (§3.1).
   9. `Afficher_ModeAuto` (§6).
   10. `Afficher_Action` (§6).
   11. `Afficher_BoutonsInfos` (§3.2).
   12. `Afficher_BoutonsMenu` (§3.4).
   13. `Afficher_Fenetres` (the 8 windows, §8).
   14. Radial buttons if active (§13).
   15. `Afficher_Effets_Visuels` (click / "new objective" animations).
   16. `Afficher_Dialogue` if chat input is open (§14.2).
   17. Confirmation window if shown (with a forced night overlay in 1-player mode, §14.1).
7. `Interface(i).Interieur_Boutons(...)` does hit-testing **and draws the tooltip of the hovered element**,
   so tooltips are on top of the HUD and windows (§15).
8. Window-drag outline + cursor, or: selected object under the cursor (`Afficher_Objet_Curseur`, §16),
   build-placement rectangle, mouse cursor (24x24), terrain info-bubbles (`Afficher_Infos_Bulles_Terrain`,
   FrmMoteur2D.frm:2510+).
9. `Afficher_FPS` (§6).
10. Victory/defeat image (320x200 from `VictoireDefaite*.bmp`) at ((W-320)/2, (H-200)/2-128).

Update cadence:
- The main loop runs as fast as it can. With `Jeu.ini AffichageRapide=1` it renders on every iteration;
  otherwise it renders at `FPS=60`.
- The game tick runs at `Vitesse=10` ticks/s (FrmMoteur2D.frm:988).
- `ActualiserRapide` is called **on every main-loop iteration, outside the tick and frame gates**
  (FrmMoteur2D.frm:976). With AffichageRapide=1 this means once per rendered frame.
- `Actions_Automatiques`, `ActualiserLent`, `Actualiser_Degats`, `AffTemps.Actualiser` and
  `Actualiser_Effets_Visuels` run once per game tick (FrmMoteur2D.frm:1132-1175, 2035-2040).

---------------------------------------------------------------------------------------------------
### 2. Interface1024.ini values (single player)

```
RectangleSelectionMinX=16  RectangleSelectionMinY=16
AfficherCadre=0  Bordure{Gauche,Droite,Haut,Bas}=0
Fenetre1..8 Position (index 0..7): 0:(704,0) 1:(704,368) 2:(64,0) 3:(64,368) 4:(384,0) 5:(384,368) 6:(64,0) 7:(64,0)
AfficherEpoques=1  EpoqueCouleur=&H808080  EpoquePositionX=528 EpoquePositionY=0
AfficherFPS=1  FPSCouleur=&HFFFFFF  FPSPositionX=940 FPSPositionY=0
AfficherAction=1  MessageActionPositionX=152 MessageActionPositionY=745  MessageActionCouleur=&HFFFFFF
Vie (10,691)  Energie (10,709)  Magie (10,727)  Moral (10,745)
AfficherBarresStatut=1 AfficherBarresStatutScore=1 BarresStatutScoreCouleur=&HFFFFFF
BarresStatutScoreDecalageX=108 BarresStatutScoreDecalageY=0 BarresStatutScoreLargeurTexte=4
NombreBoutonsBarresStatut=4 BarresStatutDecalageX=-3 BarresStatutDecalageY=0
BarresStatutLargeur=136 BarresStatutHauteur=16 BarresStatutContourEpaisseur=1
BarresStatutContourCouleur1=&HC0C0C0 BarresStatutContourCouleur2=&H80FF BarresStatutFormatCarac=0.00
AfficherBoutonsConstruire=1 BoutonConstruirePosition=(0,232) IntervalX=0 IntervalY=28
  NombreColonnes=3 ColonneIntervalX=28 ColonneIntervalY=0
  MasquerAutoBoutonsConstruire=1 DecalageX=-80 DecalageY=0 Duree=20
AfficherBoutonsInfos=1 NombreBoutonsInfos=9 BoutonInfosTransparents=1 Position=(888,0) IntervalX=-30 IntervalY=0
AfficherBoutonsMenu=1 NombreBoutonsMenu=2 BoutonMenuTransparents=1 Position=(678,24) IntervalX=0 IntervalY=32
AfficherBoutonsDeplacement=1 NombreBoutonsDeplacement=18 Position=(0,0) IntervalX=0 IntervalY=28
  NombreColonnes=2 ColonneIntervalX=28 ColonneIntervalY=0
  MasquerAutoBoutonsDeplacement=1 DecalageX=-52 DecalageY=0 Duree=13
  NombreBoutonsOrdre=5 BoutonOrdrePosition=(56,112) IntervalX=0 IntervalY=28
AfficherRaccourciObjets=1 NombreRaccourciObjets=9 RaccourciObjetsTransparent=1
AfficherEffetsTemporaires=1 EffetTemporatireTransparent=1 EffetTemporatireDureeEffacement=5
  EffetTemporatirePosition=(56,0) IntervalX=32 IntervalY=0
AfficherMessageModeAuto=0 MessageModeAutoCouleur=&H0 Position=(28,192)
AfficherInfoBulleTerrain=1
AfficherStastistiques=1 CouleurStastistiques=&HFFFFFF StastistiquesPosition=(816,745)
```
Language strings come from `Donnees\<Langue>\Interface.ini` (ClsInterface.cls:570-609). French values:
`FPSMessage=FPS : `. The INI reader trims the trailing space, so the screen shows `FPS :12`. Action messages:
`En attente, Se déplacer, Récolter ressources, Aller à, Construire, Aller construire, En repos, Rentrer à la
maison, Au travail, Entrer dans, Retourner château, Aller château, Aller trésor, Donner, Aller vendre,
Attaquer, Détruire maison, Détruire château, Ressusciter, Fabriquer, Préparer Service, Au travail` (indices 0..21).
Info tooltips: `Fenêtre Bâtiment (B)`, `Fenêtre Ressources (R)`, `Fenêtre Personnage (P)`,
`Fenêtre Compétences (T)`, `Fenêtre Inventaire (I)`, `Fenêtre Bestiaire (U)`, `Carte (Tab)`,
`Menu / Pause (F10)`, `Objectifs (F11)`. Menu: `Sauvegarder le monde (F12)`, `Quitter (Esc)`.
Deplacement tooltips 1..23: `Rentrer au château (G)`, `Rentrer à la maison (M)`, `Chercher des herbes (F1)`,
`…pommes (F2)`, `…du bois (F3)`, `…de la pierre (F4)`, `…du fer (F5)`, `…des critaux (F6)` [sic],
`…de l'or (F7)`, `…du souffre (F8)`, `Attaquer ennemi proche (W)`, `Attaquer maison proche (X)`,
`Attaquer château proche (C)`, `Ramasser un trésor (V)`, `Mode automatique (A)`, `Attaquer ses alliés (H)`,
`Stop (S)`, `Changer la formation (F)`, `En couverture (D)`, `En attente (Z)`, `En défense (L)`,
`En attaque (Q)`, `En berserk (K)`.
`BoutonRaccourciObjetEtiquette="Equiper Objets "`, `MessageModeAutoMessage=Mode# Auto`,
`BoutonDemolirEtiquette=Démolir maison (Suppr)`, `BarresStatutSeuilMessage1="Auto restauration à#"`,
`BarresStatutSeuilMessage2="#(Roulette souris)"`.
Confirmation: `MessageConfirmationQuitter="#     Voulez-vous vraiment quitter la partie ?"`,
`MessageConfirmationAttaquer="#  Voulez-vous vraiment attaquer vos alliés ?#  (Cette action vous rendera
hostile à tout le# monde jusqu'à la mort de votre personnage)"`, `Oui (Entrée)`, `Non (Echap)`.

The other resolutions (640/800/1280/1600) only move the anchors. Notably, at 1280 and 1600 the build buttons
have 2 columns with `DecalageX=-52, Duree=13`, and the status bars are at x=20.

---------------------------------------------------------------------------------------------------
### 3. Buttons

#### 3.1 Building ("Construire") buttons, left edge, auto-hidden
Init (ClsInterface.cls:623-668):
- `NombreBoutonsConstruire = NombreTypeBatimentsJouables + 1`. Index 0 = **Demolish** (`_Demolir`).
  Indices 1..N = the playable buildings in `Parametres.Batiment_Tri` order (`BoutonsConstruire_IndiceBatiment`).
  The arrays have one extra element (index N+1). It is never drawn but is still hit-tested (phantom).
- Grid: `l` (row) starts at 0 and is incremented **before** the first button. For index i: `k = i mod 3`,
  `l = i div 3 + 1`.
  - `X = 0 + 0*l + 28*k + (-80 if auto-hide)`
  - `Y = 232 + 28*l + 0*k`
  - So the first row is at **Y=260**. Shown X = 0/28/56; hidden X = -80/-52/-24.
- After the loop: `BoutonConstruirePositionX/Y += IntervalX/Y`, which gives the "shown" reference **(0,260)** for
  button 0.
- Drawing (ClsInterface.cls:1286-1295): for i = 0..N, `AffBoutonsConstruire(i).Afficher X, Y, col 0, row Etat(i),
  transparent`. Each sprite is clipped at x<0, so when hidden only the 3rd column shows its **rightmost 4 px**
  (x=0..3). This is the thin strip visible at the left edge from y=260 down in the screenshot.
- `Etat` (ClsInterface.cls:1259-1284, refreshed by ActualiserLent or on hover):
  - `Etat(0) = 1` if own house visible, or (leader of the house the character is in and inside it).
    Otherwise 0.
  - For buildings: if `fiefEpoch >= Batiment_NoEpoque-1`, then `Etat = 1` if (alive and `PeutConstruire_Maison`),
    else 0. Otherwise (epoch not reached) `Etat = 2`.
- **Slide animation** (`Deplacer_BoutonsConstruire`, ClsInterface.cls:2497-2516), called from `ActualiserRapide`,
  so once per main-loop iteration (once per frame):
  - If the cursor is over any build button (`CurseurSurBoutonConstruire`, computed by hit-testing the *current*
    positions, so only the 4-px strip at first) or forced: while `btn0 != (0,260)`, move all buttons by
    `-DecalageX/Duree = +4 px` in X.
  - Otherwise, while `btn0 != (0-80, 260)`, move them by `-4 px`.
  - 20 steps, 80 px, linear, **4 px per frame**. Equality on integer positions is exact because 80/20=4.
- Hover tooltip (`Definir_InfoBulle_Batiment`, ClsInterface.cls:4392-4478). Lines are separated by **Chr(13)**:
  - Building name.
  - `&(<epoch name>)` if the epoch is not reached.
  - `(<price> <EtiquettePieces>)`, or `&(<have> / <price> <EtiquettePieces>)` if you cannot afford it.
  - One line per resource cost: `(<n> <resource lowercase>)` or `&(<have> / <n> <resource>)`.
  - `&<EtiquetteBatimentEmpereur>` if the building is emperor-only and you are not emperor.
  - The description.
  - After hovering the same button for **2 s**, for workshop buildings, lines
    `- <object plural> (<cost>[, …])[ (<epoch>)]` are appended, prefixed by `&` if not affordable or too
    early. One more line appears every **0.3 s**: `Details = (|Timer-T0| - 2)/0.3` lines.
  - Demolish: `BoutonDemolirEtiquette`, prefixed by `&` if disabled.
- Clicks: see §15.

#### 3.2 Info buttons (9, top right)
- Position `X = 888 - 30*i`, `Y = 0` (ClsInterface.cls:697-704). Gives x = 888, 858, 828, 798, 768, 738, 708, 678, 648.
  24x24, transparent.
- Draw (ClsInterface.cls:1700-1721): col i. Row = `1` if `BoutonsInfos(i).Enabled` (window open / paused),
  otherwise 0. Button 8 (objectives) is drawn **only if `AfficherObjectifs`** (at least one objective exists).
  It uses row 2 if `NouveauxObjectifs` and not open.
- Windows are drawn after the info buttons, so the building window at (704,0) covers buttons 0..6 when open
  (as in the screenshot).
- Action (`Actionner_Boutons_Infos`, ClsInterface.cls:4726-4742). Toggles `Enabled`, and:
  - i=0..6: `Fenetres(i).Visible = not Enabled` → 0 building/castle (Acheter/Vendre), 1 resources,
    2 character, 3 skills, 4 inventory, 5 bestiary, 6 map. Closing the map re-enables auto scrolling.
  - i=7: toggles `Jeu.EnPause`. When enabled (paused) the **menu buttons** are shown.
  - i=8: `Fenetres(7)` (objectives), and clears `NouveauxObjectifs`.
  - Note: `Fenetres(0).Visible` is **recomputed every frame** in Afficher_Fenetres (ClsInterface.cls:1133-1139):
    `(alive AND (in a house OR in own house)) OR in a castle OR in own castle OR (in a house AND house.Service=1)`,
    all of it ANDed with `BoutonsInfos(0).Enabled`.
- `Objectifs_Ajouter` (ClsInterface.cls:4949-4969) sets `AfficherObjectifs`. If the map window is not visible
  (sic: it tests Fenetres(6)), it sets `NouveauxObjectifs` and plays effect `EffetNouvelObjectif` (0-based 38)
  centred on button 8 (648+12, 12).

#### 3.3 Move ("Deplacement") buttons (18) + order buttons (5), top left, auto-hidden
- Init (ClsInterface.cls:738-770). For i<18: `k = i mod 2`, `l = i div 2` (starting at 0):
  `X = 0 + 28*k (-52 if hidden)`, `Y = 0 + 28*l`. So the grid is 2 columns x 9 rows, y = 0..224.
  For i = 18..22 (orders): `X = 56 (-52 → 4 when hidden)`, `Y = 112 + 28*(i-18)` (112..224).
- Sprite: 28x28, **opaque blit** (no colour key, ClsInterface.cls:1319-1323). col = i.
  Row: i=14 → `-IA` (1 = auto mode on, monitor icon); i=17 → `Perso.Formation` (0..4); else
  `-Enabled`, where `Enabled := CliqueEnfonceGauche` while hovered (pressed look while the left button is held;
  the value is not reset when leaving).
- Visibility (ClsInterface.cls:1305-1309):
  - i=0 only if own castle visible.
  - i=1 only if own house visible.
  - i>16 (17..22) only if `NombreSoldats>0`.
  - i>=18 (orders) only if `ArmeeSelectionnee`.
  - Hidden buttons are not hit-testable (`.Visible=False`).
- When hidden, column k=1 shows its rightmost 4 px (x=0..3). This is the strip at the left edge in the
  screenshot (y≈28..252, starting at row 1 if there is no house).
- If `NombreSoldats>0`: text `"Nb:#<n>"` (2 lines) in `MessageModeAutoCouleur` (black) at
  `(btn18.X, btn18.Y - 28)` = (56 or 4, 84) (ClsInterface.cls:1333-1338).
- Slide (`Deplacer_BoutonsDeplacement`, ClsInterface.cls:2517-2538): shown while the cursor is over a move button
  **or an army is selected** (or forced). Step `52/13 = 4 px` per frame, 13 steps. The 5 order buttons move with
  them.
- Actions (Interieur_Boutons, ClsInterface.cls:3955-4073):

| i | NoBouton | Action |
|---|---|---|
| 0 | 200 | return to castle (on left **release**, FrmMoteur2D.frm:803) |
| 1 | 101 (if alive or own house is a temple/service 1), else 999 | go home (Ctrl = only if built) |
| 2..9 | 999 | left click: `Cmd_Perso_TrouverRessource(i-2)` (herbs, apples, wood, stone, iron, crystals, gold, sulphur). Tooltip gains `#<res> : <have> / <max>` |
| 10 | 999 | attack nearest enemy (army: order attack). Right/Ctrl/Shift/Alt modifiers forwarded |
| 11 | 999 | attack nearest enemy house |
| 12 | 999 | attack nearest enemy castle |
| 13 | 999 | find treasure |
| 14 | 999 | toggle auto mode (`Cmd_Perso_Definir_IA`) |
| 15 | 999 | opens confirmation "attack allies" (§14.1) |
| 16 | 999 | Stop (left: stop self, or the army if selected; right: stop army) |
| 17 | 999 | cycle formation |
| 18..22 | (0) | `Bouton_ChangerAttitude(i-18)` → attitudes Couverture / Attente / Défense / Attaque / Berserk for the selected soldiers, then deselect the army |

#### 3.4 Menu buttons (2), visible only while paused (info button 7 enabled)
- (ClsInterface.cls:727-735, 1843-1866). `X=678`, `Y = 24 + 32*i` gives (678,24) save, (678,56) quit.
  32x32, transparent. col = i, row = `MouseOver`.
- In a **campaign** only button 1 (quit) is drawn and hit-tested, shifted by one interval to (678,24).
- Click: 0 → `Enregistrer_Sauvegarde_Monde`. 1 → confirmation "quit".

#### 3.5 Object shortcut slots (9), top left of the HUD
- Position: **reuses the temporary-effect anchors** (ClsInterface.cls:773-777): `X = 56 + 32*i`, `Y = 0`
  → x = 56, 88, …, 312. 32x32, transparent (only the lilac frame and label are drawn; the interior shows the
  map).
- Sheet row = `-Int(Enabled) + 2*BoutonRaccourciObjetDecalageY` → `Enabled` (an object bound to this key is
  **currently equipped**) gives row 1 (dim label), otherwise row 0 (bright label). The decal is 0 normally, 1 while
  **Alt** is held (Alt 1..9 rows), 2 while **Shift** is held (Maj rows) (ClsInterface.cls:4149-4168).
- Content (`Afficher_Raccourcis_Objets`, ClsInterface.cls:1548-1610). For each object type bound to that key:
  - Draw its 32x32 icon (row 0 of the object image) at the slot (inventory ones first, then the equipped one on top).
  - If the slot is enabled and the object category is 0 or 1, and attack<max, defence<max or a reload is running:
    - attack bar at (X, Y+22) 32x5. Red, or the reload progress `(Max-remaining)/Max` with auto colour.
    - defence bar at (X, Y+27) 32x5, blue. ForeColor black (frame).
  - `Afficher_Duree_Recharge` (ClsInterface.cls:1612-1660): if reloading, overlay animation effect
    `EffetDureeRestante` (0-based 43), or `EffetDureeRestante2` (46) when some charges remain, frame
    `Int((rem mod D)/D * nbFrames)`. If there are multiple charges and the total is >1, draw `"<avail>/<total>"`
    in FontDegats at `(X+15-len*2, Y+9)`.
  - Text `BoutonsRaccourciObjetsMessages(i)` in **FontDegats, white** at **(X+3, Y+18)**: `"x<count>"` for
    stackable single-use items, or `"(NN)"` ammo count (`&(00)` red when 0).
  - Computed in `Actualiser_Raccourcis_Objets` (ClsInterface.cls:1500-1546), once per second or immediately when
    `ActualiserRaccourciObjets` is set.
- Tooltip: `"Equiper Objets (" & ["Alt+"|"Maj+"] & n & ")"`, then for each bound object `"#- <name>"`
  (`"#&- <name>"` if not owned).
- Clicks: right click unbinds all objects of that key. Left click with nothing on the cursor equips everything
  bound (`Raccourcis_Objets_Equiper`, max one per hand). Left click with an object on the cursor binds that
  object to the key.

#### 3.6 Temporary effects icons
- (ClsInterface.cls:1662-1698, 2392-2416). For effect i (1-based), slot index `j = i-1 + 9` (because the shortcut
  bar is shown): `X = 56 + 32*j` → **x = 344, 376, …**, `Y = 0`.
- Icon = the object image's **row 1** (y 32..63) of `Definir_EffetTemp_Numero(i)`, colour-keyed.
- Blink when remaining duration `d < 5` (and d ≠ -1, alive): translucent with
  `alpha = (cos((5 - (d + Compteur*10) + 0.1)/1.5) + 1)/2`. `Compteur` is the 0..1 sub-second counter of
  ActualiserLent, incremented by 1/Vitesse per tick.
- Duration bar (if d ≠ -1): at `(X+2, Y+25)`, size **28x5**, white, value `d/dMax`.
- Hover: object tooltip from the inventory window. Right click cancels the effect (unless
  `Objet_EffetAnnulationImpossible`). Left click re-equips the inventory item with that appearance.

---------------------------------------------------------------------------------------------------
### 4. Status bars (Vie / Energie / Magie / Moral), bottom left

#### 4.1 Per bar (`Afficher_Vie` etc., ClsInterface.cls:1962-1997)
For Vie with `P=(ViePositionX, ViePositionY)=(10,691)`:
1. `SetForeColor 0`, then `Afficher_Barre_Statut(Min1=clamp(Vie,0,Definir_Vie), Min2=Definir_Vie,
   Max=Definir_MaxVie, X=P.x-3=7, Y=P.y=691, W=136, H=16)`. For the other bars, Min1 = max(value,0) and
   Min2 = Definir_*.
2. Label `EtiquetteVie & " :"` ("Vie :", "Energie :", "Magie :", "Moral :") in white at **P** = (10,691).
3. Score `Int(Vie) & " / " & Definir_MaxVie` in `BarresStatutScoreCouleur` (white) at
   **x = P.x - 3 + 108 - Len(score)*4**, y = P.y. Right-aligned approximately: "15 / 15" (7 chars) → x=87.
   This draws over the bar (black-frame/green interior), as in the screenshot.

Bars are drawn in the order Moral (y=745), Magie (727), Energie (709), Vie (691), then the contours.

#### 4.2 `Afficher_Barre_Statut(Min1, Min2, Max, X, Y, W, H, [Couleur], [Vertical])` (ClsInterface.cls:1998-2080)
1. Clip to the screen (shrink W/H if X/Y<0 or past ScreenWidth/Height). Skip if W<=0 or H<=0.
   Add ScreenPositionX/Y.
2. `setDrawStyle 10` (pen is effectively solid 1 px, in **current ForeColor**, usually black). `SetFillColor 0`.
   `DrawBox X,Y,X+W,Y+H` gives a **black filled rectangle with a 1-px frame**: frame at x=X and x=X+W-1,
   y=Y and y=Y+H-1.
3. `setDrawStyle 5` (no pen). `SetFillColor white`. If Min2>0:
   - horizontal: `DrawBox X+1, Y+1, X+W*Min2/Max, Y+H` → white fill x∈[X+1, X+W*Min2/Max-2], y∈[Y+1, Y+H-2].
   - vertical: `DrawBox X, Y+H, X+W, Y+H-H*Min2/Max` (GDI normalises).
4. Fill colour: if `Couleur=0` (auto), by `Min1/Max`: `<0.1 → (128,0,0)`, `<0.25 → (255,0,0)`, `<0.5 → (255,128,0)`,
   else `(0,192,0)`. Otherwise use `Couleur`.
   - horizontal: `DrawBox X+1, Y+1, X+W*Min1/Max, Y+H` → fill x∈[X+1, X+W*Min1/Max-2], y∈[Y+1, Y+H-2].
   - vertical: `DrawBox X+1, Y+H, X+W, Y+H-H*Min1/Max`, which fills from the bottom up.
5. `SetFillStyle 1` (transparent), `setDrawStyle 0`.
Verified on the screenshot for Vie at full: black frame x=7..142, y=691..706, green (0,192,0) x=8..141,
y=692..705. That is a 134x14 fill inside a 136x16 frame.

#### 4.3 Outer contour / auto-restore (`Afficher_Barre_Statut_Auto`, `Afficher_Contour_Barre_Statut`, ClsInterface.cls:2082-2168)
For each bar: ForeColor = orange (255,128,0) if auto-restore is ON, otherwise grey (192,192,192). Fill is
transparent. With thickness `e=1`:
- `DrawBox X-1, Y-1, X+W+1, Y+H+1` → a 1-px outline at x=6..143, y=690..707 for Vie. The contours of consecutive
  bars touch (707 / 708) because the bar pitch is 18 = H+2.
- Threshold marker: `DrawBox X+W*pct, Y, X+W*pct+1, Y+H` → a 1-px vertical line at x = X+W*pct, y = Y..Y+H-1.
  When auto-restore is OFF, pct = **100** (not 1.0), so the marker is far off-screen (invisible).
- Mouse (§15): right click toggles auto-restore. Left click restores now (`Cmd_Perso_RecupererCarac`).
  The wheel over a bar enables auto-restore and changes pct by `Int(Z/120)*0.025` (2.5 % per notch; tiny Z is
  rounded up to ±120). It clamps pct to [0,1] and snaps the mouse X onto the marker.
- Tooltips: `Format(value,"0.00") & " / " & max` (e.g. "15.00 / 15"). On the marker:
  `"Auto restauration à#->" & Int(max*pct) & " / " & max & " (" & Int(pct*100) & "%)" & "#(Roulette souris)"`.
- `Actions_Automatiques` (ClsInterface.cls:2438-2495, per tick): if alive and `auto[i]` and
  `current < max*pct`, call `Cmd_Perso_RecupererCarac(i)`. It also handles the auto-fabrication queue and the auto
  building service.

The same `Afficher_Barre_Statut` is used for unit bars (FrmMoteur2D.frm:1773-1810, 1903-1940). Those are
W = sprite width, H = 5, at y = bottom of the sprite (+5 attack red / reload, +10 defence blue), with ForeColor
black. The vertical white epoch bar sits at the castle's right side (5 px wide).

---------------------------------------------------------------------------------------------------
### 5. Epoch box ("Age de bois") (ClsInterface.cls:1054-1107)
Constants: `H=22, W=180, dx=-90, dy=-4`. Anchor `(528,0)`. ForeColor = `EpoqueCouleur` grey (128,128,128).
- Normal (`Not Fief.ChangerEpoque`): `SetFillColor 0; SetFillStyle 0` (solid).
  `DrawBox 438, 0, 618, 18` → **grey 1-px outline x=438..617, y=0..17, black interior 439..616 x 1..16**
  (matches the screenshot). Then fill style 1.
- Changing epoch: `Afficher_Barre_Statut(D-T, 0, D, 438, -4, 180, 22, white)` with
  `D = Epoque_Duree(next)*VitesseEpoque_Coef`, `T = TempsChangementEpoque`. It is clipped to y=0, h=18. The frame is
  in the current ForeColor (grey) and the progress is a white fill.
- Text: epoch name, in grey (current ForeColor), at **x = 528 - Len(name)*(7-0.5)/2 = 528 - 3.25*len**
  (VB-rounded to Long), y=0. "Age de bois" (11 chars) → x=492.25 → 492 (ink starts at 492, cap top y=2).
- If the fief has an emperor: a life bar `Afficher_Barre_Statut(clamp(vie), vie, maxVie, 438, 17, 180, 18)` with
  auto colours, and the text `"<EmpereurNom> <Nom>"` centred the same way at y=17.

---------------------------------------------------------------------------------------------------
### 6. Single-line texts
- **FPS** (ClsInterface.cls:1109-1115, FrmMoteur2D.frm:1480-1493): `FPSMessage & Format(MaxFPS,"00")` → "FPS :12".
  It is white at (940,0) and drawn after the cursor. It updates once per second. In network games, 2 extra lines are
  added: `#E : n#R : n`.
- **Action message** (ClsInterface.cls:1875-1960): white at **(152,745)**, the same baseline row as "Moral :".
  The mapping `Perso.Action → MessagesActions index` is: 0→0, 1→1, 2→2, 3→3, 6→4, 7→5, 100→6, 101→7, 102→8,
  201→9, 202→10, 203→11, 204→12, 205→13, 206/207→14, 300→15, 301→16, 302→17, 400→18, 500→19,
  600→20, else→21. Suffixes:
  - Action 6 with a damaged house: `" : <pct>%"`.
  - Action 2/102 with a current resource: replaced by `"<skill name> : <have> / <max>"`.
  - 3: `" : <resource name>"`.
  - 201: `" : <building name>"`.
  - 300: `" : <name or race> (<EtiquetteVie>:<pct>%)"`.
  - 400: `" : <100-DureeRessurection>%"`.
  - 500: `" <object lowercase> : <pct>%"`.
  - 600: replaced by `"<service skill> : <stock> / <max>"`.
- **Auto-mode message** (ClsInterface.cls:1868-1873): only if `AfficherMessageModeAuto` (0 at 1024) and IA is on.
  It shows `"Mode# Auto"` in black at (28,192).

---------------------------------------------------------------------------------------------------
### 7. Statistics block (bottom right) (ClsInterface.cls:2213-2221, 2418-2436)
- Recomputed once per second (not when networked). For each fief still "active" (`NombreMorts < NombreCitoyens`
  or castle life > 0) it appends:
  `"#" & ["&" if all dead] & "(" & NumeroEquipe & ") " & Nom & " " & (alive) & "/" & citizens & [" - " & Format(castleLife/max*100,"00") & "%" if castle visible]`
  Example: `(1) Humains 50/50 - 100%`.
- `Y0 = 745 - 16*nActive`. It is drawn with white at `(816, Y0)`. Because the string starts with `#`, the first
  line is empty, so the lines land at **y = 745-16*(n-1) … 745**. With 4 fiefs that gives 697, 713, 729, 745
  (cap tops measured at 699). Lines of eliminated fiefs (all citizens dead but castle alive) are red.

---------------------------------------------------------------------------------------------------
### 8. Windows: how ClsInterface lays them out (ClsInterface.cls:355-371, 395-405, 790-800, 1117-1203)
- `Fenetres.ini`: 8 windows, all **320x368**. Images: 0 `Fenetre-Defaut`, 1 `Fenetre-Defaut`,
  2 `Fenetre-Personnage`, 3 `Fenetre-Competences`, 4 `Fenetre-InventaireH/F/HM/FM` (4 images),
  5 `Fenetre-Defaut`, 6 `Fenetre-Carte`, 7 `Fenetre-Defaut`. They are blitted transparent (black key)
  (ClsAffFenetre.cls:28-29,65).
- Default positions are given in §2. `RePositionner_Fenetres` clamps each window inside the screen every frame.
  Window drag: the move button sets `DeplacerFenetre`. While dragging, a **white 1-px outline 320x368** (no fill)
  is drawn at the mouse (`Afficher_Contour_Fenetre`). The position is clamped so the window fits. Right click on the
  move button resets it to the INI position.
- Draw order and calls (`Afficher_Fenetres`):
  1. **4 Inventory**: image index = `(Bascule and in own house and alive) ? (female?3:2) : (female?1:0)`, then
     `Afficher_InfosInventaire("Inventaire", …)`, then `Afficher_TouchesObjets`. That last one draws the shortcut
     key labels `(n)` / `(+)` at each inventory slot, plus `x<count>` at (+4,+19), in FontDegats.
  2. **0 Building**: frame, then `Afficher_InfosMaison(<building name>)` if in a non-market house, otherwise
     `Afficher_InfosChateau("Acheter")`.
  3. **1 Resources**: frame, then `Afficher_InfosRessourcesPerso("Vendre" if in a castle, else "Ressources")`.
  4. **2 Character**: `Afficher_InfosCaracPerso1` (drawn *under* the frame), frame, then
     `Afficher_InfosCaracPerso2`.
  5. **3 Skills**: `Afficher_InfosCompetences1` (if any) under the frame, frame, then `…2`.
  6. **5 Bestiary**, **6 Map** (`Afficher_Carte`), **7 Objectives**: frame, then content.
- The windows' empty area is **not** an interface hit zone (that code is commented out,
  ClsInterface.cls:4318-4327). Only their buttons capture the mouse.

---------------------------------------------------------------------------------------------------
### 9. Message log (ClsJeuMessages.cls; values from Donnees\Francais\Messages.ini)
- `PositionX=60, PositionY=30, DecalageX=0, DecalageY=16` → line i is drawn at **(60, 30+16*i)**, in white
  (`Couleur=&HFFFFFF`), default font. In 2-player mode, player 2's log is shifted by `i*Width/2`.
- `NombreMaxMessages=12`: extra lines are removed from the top immediately.
- **Expiry** (ClsJeuMessages.cls:463-505): only the **oldest** line expires. It is removed when
  `now - TempsDernierMessage > Len(oldest) * Duree` with **`Duree=0.16` s per character**. Then the timer
  restarts, so lines disappear one by one, each after 0.16 s x its length. There is no fading.
- Multi-line messages (`#`): `Ajouter_Message` appends the message and then `Nombre_Lignes` empty strings
  (one more than needed). The empty entries have length 0 and vanish on the next frame once they reach the top.
- A chronometer (`Jeu.IntituleChrono`) is always kept as line 0 when active.
- Content (`Actualiser_Messages`, once per second): it diffs the player's state. Examples: `"- Vous avez gagné 12
  pièces d'argent."`, `"+ Vous avez gagné 1 niveau supplémentaire à vos points de vie."`, kills, soldiers,
  resources, objects, emperor changes (upper case), death/resurrection. At game start a random tip from
  `Info_Conseils.txt` is added.

---------------------------------------------------------------------------------------------------
### 10. Day/night overlay (ClsAffTemps.cls, Temps.ini)
- `Afficher=0` by default (no overlay during play). `Largeur=Hauteur=512`. 11 periods by start hour:
  0 Nuit, 3 Soir4, 4 Soir3, 5 Soir2, 6 Soir1, 7 (none = day), 17 Soir1, 18 Soir2, 19 Soir3, 20 Soir4, 21 Nuit.
  `JourneeDebut=9`, `JourneeVitesse=0.08` minutes per tick, so 60 min = 750 ticks = 75 s at 10 ticks/s.
- The images are **ordered dithers with period 8**. They are colour-keyed: black pixels are transparent and the
  others are painted opaque. There is **no alpha blending**. Patterns (row-major 8x8, `#` = painted):
  - Soir1 (12.5 %, colour (74,82,115)): rows `#...#...`, `........`, `..#...#.`, `........` (repeats).
  - Soir2 (25 %, (66,74,99)): `#.#.#.#.`, `........` (repeats).
  - Soir3 (37.5 %, (49,57,82)): `#...#...`, `.#.#.#.#`, `..#...#.`, `.#.#.#.#`.
  - Soir4 (50 %, (41,49,66)): checkerboard `#.#.#.#.`, `.#.#.#.#`.
  - Nuit (50 %, (24,24,33)): the same checkerboard.
- Tiling (ClsAffTemps.cls:128-152): `for x = EcranDecalageX - (camX mod 512) to W step 512` (same in y), blit
  512x512. The pattern is anchored to **world coordinates**: pixel painted iff `pattern[(worldX) mod 8][(worldY) mod 8]`.
- It is also forced (`ForcerAffichage`) with **Nuit** behind the confirmation dialog in 1-player mode
  (ClsInterface.cls:869-876), over the whole screen including HUD and windows.

---------------------------------------------------------------------------------------------------
### 11. Floating damage numbers (ClsAffDegats.cls, Combat.ini [Degats])
- `AfficherDegats=1`, `TailleNormale=8`, `TailleCritique=10`. Colours: damage `&HFF` red (255,0,0),
  heal `&HFF0000` blue (0,0,255), sale `&HFFFF` yellow (255,255,0). `DureeMax=15`, `DecalageX=0`,
  `DecalageY=-6`, `DeplacementX=0`, `DeplacementY=-1`.
- Spawn (`Ajouter_Degat`): score = CLng(score), skipped if 0. `x = X + 0 - Len(score)*size/2` (size 8, or 10
  if critical), `y = Y - 6`. These are world coordinates.
- `Ajouter_Texte`: `x = X + 4 - Len*8/2`, `y = Y - 16`, red.
- Per game tick: `y -= 1`, `Duree += 1`. The number is removed when `Duree > 15`, so it lives **16 ticks = 1.6 s**
  and rises 16 px. No fade.
- Draw: FontCritique (Arial Bold 10) or FontDegats (Arial Bold 8), at the screen position, only if strictly
  inside the view.

---------------------------------------------------------------------------------------------------
### 12. Speech comments over characters (ClsJeuCommentaires.cls, FrmMoteur2D.frm:2451-2484)
- `Donnees\Francais\Commentaires.ini`: `Afficher=1`, `Duree=16` (stored /100 → **0.16 s per char**),
  `Couleur=&HFFFFFF`, `DecalageX=24`, `DecalageY=-16`. There are 53 message types with random variants.
- Text drawn with `AffMessage` (default 10-pt bold, white, `#`/`&` markup) at
  `(charX - camX + 24, charY - camY - 16)` (sprite top-left + offset), only if the character is alive and on
  screen. There is no bubble box and no shadow.
- A comment is set when an action produces a message: always for the player's character, and with probability 1 %
  for the others (FrmMoteur2D.frm:1587-1591). It is cleared when `now - TempsDernierCommentaire > Len*0.16`.

---------------------------------------------------------------------------------------------------
### 13. Radial buttons (ClsInterface.cls:1723-1841)
- Triggered by a **right click on the terrain** with no army selected and no object on the cursor
  (FrmMoteur2D.frm:943-961). On empty ground: `Activer_BoutonsRadiaux(mouse, 4)`. On a resource: `…, 1`.
  On houses/castles/temples: `…, 0`.
- The first call sets `ActiverBoutonsRadiaux = True`. The category is **not** reset (0 initially). While the
  ring is active, another right click advances the category `0 → 1 → 0`. A left click anywhere closes it. While
  active, `Interieur_Boutons` is modal (always returns True, NoBouton 999 when nothing is hit).
- On activation the centre is the mouse position. In category 0 **the mouse cursor is warped** to the centre
  of the requested info button.
- Geometry:
  - Category 0 (info): the 7 info buttons 0..6 (24x24). Angle step `2*3.14/7`, radius **40**,
    `angle0 = -1.57` (top). Top-left = `centre + (cos(a)*40 - 12, sin(a)*40 - 12)`, with `a = step*i + angle0`.
  - Category 1 (build): all `NombreBoutonsConstruire` buttons (demolish first), on concentric rings. Ring j holds
    `7 + 7*j` buttons at radius `40 + 40*j` (7 @40, 14 @80, 21 @120, …). Step `2*3.14/(7+7j)`.
    Top-left = `centre + (cos*r - 14, sin*r - 14)`.
- Rotation (`Pivoter_BoutonsRadiaux`, called when `SourisZ ≠ 0` while active, FrmMoteur2D.frm:640-641):
  `angle[cat] += 3.14*Z/2`, wrapped once by ±6.28, then the positions are recomputed. DirectInput gives Z = ±120 per
  wheel notch, so one notch adds ±188.4 rad (≈ ±6.19 rad mod 2π, i.e. about −5° / +5° visually). Reproduce the
  formula literally.
- Draw: buttons (info: row = Enabled; build: row = Etat), then for hovered ones a white rounded box
  `(x-1, y-1, x+w+1, y+h+1)` with pen width **3**. Then a circle of radius 2 at the centre and a **line from the
  centre to the mouse**, white, width 3.
- Click on info → `Actionner_Boutons_Infos`. Click on a build button → same as the edge buttons (NoBouton =
  building+1). Build tooltips with the delayed details as in §3.1.

---------------------------------------------------------------------------------------------------
### 14. Modal dialogs

#### 14.1 Confirmation (ClsIntConfirmation.cls)
- Window 320x160 `Fenetres\Fenêtre-Confirmation.bmp` (green rounded frame, dithered interior), colour-keyed.
  It is centred: `X = splitOffset + (W-320)/2`, `Y = (H-160)/2` → **(352,304)** at 1024x768.
  Note: `Definir_Nombre_Sprites` is called while W/H=0, so the source rect is empty and DirectDraw blits the
  whole image.
- Buttons (Menu.bmp type 7, transparent, 32x32): **Oui** col 2 at (X+64, Y+100) = (416,404). **Non** col 3 at
  (X+224, Y+100) = (576,404). Row = MouseOver. Hit size 32x32.
- Message in **black** at (X+16, Y+16) = (368,320), using `#` line breaks. The leading `#` makes the first
  line empty, so the text starts at y=336.
- Hover tooltips "Oui (Entrée)" / "Non (Echap)". Oui: quit (category 0) or `Cmd_Perso_AttaquerAllies`
  (category 1). Non: closes, and unpauses if the menu was open.
- While shown, nothing else on the HUD is hit-testable (NoBouton 999 only over the buttons).

#### 14.2 Chat input dialogue (`Afficher_Dialogue`, ClsInterface.cls:2224-2249)
- Box `l=480, h=20`: `DrawBox (W-480)/2, (H-20)/2+1, (W-480)/2+480, (H-20)/2+20`. The outline is black and the
  fill is (224,224,224). At 1024x768 that is (272,375)-(752,394): outline x 272..751, y 375..393.
- Text in **blue (0,0,255)** at `((W-480)/2+4, (H-20)/2+2)` = (276,376):
  `Texte1 & (CursorVisible ? "  " : "<") & Texte2`. `DialogueCurseurVisible` toggles once per second.
- History (max 10 entries), shown when browsing with up/down: the older ones are drawn above, at y - 16*k, and
  the newer ones below, also in blue (the colour stays set).
- Commands: `_heure=N`, `_regner sans partage`, `_test`. Anything else is sent as a comment.

---------------------------------------------------------------------------------------------------
### 15. Tooltips and info-bubbles (ClsIntBouton.cls, Boutons.ini)
`Boutons.ini`: `AfficherInfoBulle=1, InfoBulleDecalageX=24, InfoBulleDecalageY=20, CouleurInfoBulle=&H0
(black), CouleurRectangleFond=&HC0FFFF (255,255,192), BordureRectangleFond=2`.

`Afficher_Info_Bulle(x, y, AffMessage, Surf, [Forcer])` (ClsIntBouton.cls:57-117). It is drawn only if (enabled
and the text is non-empty) or forced:
1. `x += 24; y += 20` (relative to the mouse position).
2. Size: iterate over lines split at `#` (or else CR). For each line, take the length **including the
   separator char** (and including any leading `&`). The last line uses its plain length.
   `W = maxLen*7`, `H = nLines*16`.
3. Keep it on screen: if `x < ScreenPositionX` → `x = ScreenPositionX`; elif `x+W+2 > ScreenWidth` →
   `x = ScreenWidth-W-2`. The same for y with H.
4. `SetFillColor (255,255,192)`, ForeColor black. `DrawBox x-2, y-2, x+W+2, y+H+2` → black 1-px outline on
   x∈[x-2, x+W+1], y∈[y-2, y+H+1], pale yellow inside.
5. Text in black at (x,y) with `#`/CR/`&`/`$` markup (red lines start with `&`).

`Afficher_Info_Bulles_Terrain(X, Y, msg)` (ClsInterface.cls:2170-2178) temporarily puts `msg` into the move
button 0's tooltip and calls `Afficher_Info_Bulle` **forced**. It is the same box. Callers:
- `Afficher_Objet_Curseur` (§16).
- Map hover on an enemy: `"&<name> "`.
- Confirmation buttons, character-window characteristic buttons, emperor button.
- `FrmMoteur2D.Afficher_Infos_Bulles_Terrain` (FrmMoteur2D.frm:2510+), when `AfficherInfoBulleTerrain=1`, at
  the mouse position, by cursor type:
  - resource: `"<res> : have / max"`.
  - own house: `"<name> #<res> : have / max "` or the production %.
  - allied house: name (+ `#&<Fermé(e)>` / `#&<Abandonné(e)>`).
  - own castle: `"<castle name>  "` + `#<next epoch> : xx.xx %` + resurrection info (`#&<lives> : 0` red when 0).
  - enemy: `"&<name>  "` + `#&(<effect>)` per active effect.
  - enemy house / castle in red; temple; treasure; ally; tomb.
  - With an object on the cursor: the "Vendre"/"Déposer" labels.

Text uses the default font (Arial Bold 10). The tooltip of the element under the mouse is drawn in the same
frame, after the HUD.

---------------------------------------------------------------------------------------------------
### 16. `Afficher_Objet_Curseur` (ClsInterface.cls:880-912)
Called with `(mouseX - 12, mouseY - 12)` (cursor 24x24, centred) when the character has something selected
(FrmMoteur2D.frm:1355-1357):
- If money/resources are selected, an info-bubble (§15) at `(X+24, Y+20)` with lines
  `"Argent : <n>"` and then `"<Ressource> : <n>"` joined by `#`.
- If an object is selected, its 32x32 icon (row 0, colour-keyed) at (X,Y). That is drawn after the bubble, so the
  icon is on top. The mouse cursor itself is drawn afterwards.

---------------------------------------------------------------------------------------------------
### 17. ActualiserRapide / ActualiserLent / Actions_Automatiques
- `ActualiserRapide` (ClsInterface.cls:2361-2364), once per main-loop iteration: slide the build buttons and the
  move buttons (4 px steps, §3.1/§3.3).
- `ActualiserLent` (ClsInterface.cls:2366-2390), once per tick: `Compteur += 1/Vitesse`. When it reaches 1 (≈ every
  second) it resets to 0 and:
  - refreshes the build-button states (only if not auto-hide, or forced, or the radial build ring is open);
  - refreshes the statistics (if not networked);
  - refreshes the object shortcut slots;
  - updates the message log (`Messages.Actualiser_Messages`);
  - toggles the chat cursor blink;
  - refreshes the skills list if window 3 is open.
  On the other ticks it refreshes the shortcut slots only if `ActualiserRaccourciObjets` is set.
- `Actions_Automatiques` (§4.3), per tick: auto-restore, auto-fabrication queue, auto-service.
- `Actualiser_Effets_Visuels`, per tick: advances the click effects and removes at most one finished effect per
  tick. Click effect = `Parametres.EffetClique` (0-based 37), played at the click point for most UI clicks.

---------------------------------------------------------------------------------------------------
### 18. `Interieur_Boutons` – hit-testing order and NoBouton (ClsInterface.cls:2540-4334)
Called once per rendered frame after `Afficher`, with the mouse (X,Y,Z), click/hold flags and modifiers. It
returns `CurseurSurInterface`. It **draws the hovered tooltip**. `NoBouton` meanings:
- `0`: nothing hit (the mouse acts on the map).
- `1..99`: **building type index + 1**, set on a *left click* on an enabled build button. FrmMoteur2D then enters
  build-placement mode (cursor 2/3, a rounded rectangle hatched green (0,192,0) or red (255,0,0) under the cursor,
  FrmMoteur2D.frm:2420-2448, 1352-1376). `NoBatiment` is set the same way.
- `101`: move button 1 (go home). Acted on left **release** (FrmMoteur2D.frm:801-802).
- `200`: move button 0 (go to castle). Acted on left release.
- `900`: mouse over the minimap in the map window (cursor type 15). Right drag recentres the camera, left click
  moves or attacks.
- `999`: generic "UI consumed".

Order (the first match wins; most blocks are skipped once `Interieur_Boutons` is True):
1. Confirmation dialog (modal: if shown, nothing else is tested, not even the Alt/Shift slot-row switch).
2. Radial ring (modal).
3. Menu buttons (only while paused).
4. Build buttons. They are always hit-tested to compute `CurseurSurBoutonConstruire`, even if something earlier
   matched. On a hit: refresh states. Left click: demolish (i=0) or select building (NoBouton = type+1).
   Right click: `Trouver_Maison` (walk to the next allied house of that type; Ctrl = repair). Hover: tooltip.
5. Map window minimap (900).
6. Character window buttons.
7. Resources window (eat buttons, scrollbars, stock transfer, sell/take buttons, money transfer, global
   transactions).
8. Building window (emperor button, castle restoration, buy buttons, scroll, money/stock transfer, fabrication,
   shop purchase, service button (right click = auto service), workshop objects).
9. Skills window (sort buttons, scroll).
10. Inventory window (transfer, object slots incl. shortcut-key assignment by pressing a digit while hovering,
    house-inventory toggle, tabs). After that, **Ctrl+digit** stores the current equipment on that key
    (always evaluated).
11. Bestiary scroll.
12. All windows: close (window 7's close maps to info button 8) and move buttons.
13. "Take money" buttons (windows 1 and 4): left = 100, right = all.
14. Move/order buttons (`CurseurSurBoutonDeplacement`).
15. Info buttons (button 8 only if objectives exist).
16. Object shortcut slots. Then **the Alt/Shift row switch** (`BoutonRaccourciObjetDecalageY` = 1 for Alt, 2 for
    Shift, else 0) is evaluated unconditionally.
17. Temporary effects.
18. Status bars (+ wheel threshold).
19. Screen border: if `X<=BordureGauche` or `X>=W-BordureDroite` or `Y<=BordureHaut` or `Y>=H-BordureBas`
    (all 0 at 1024), it counts as interface. So the mouse at x=0 or y=0 (screen edge, used for edge-scrolling) is
    "on the interface".

---------------------------------------------------------------------------------------------------
### 19. Full-screen toggle (`Affichage_PleinEcran`, ClsInterface.cls:4594-4621)
It hides the frame, epoch, action, status bars, build/info/move buttons, statistics, shortcuts and temp effects
(FPS, windows and message log stay). It restores the INI values afterwards.

---------------------------------------------------------------------------------------------------
### 20. Two-player / split screen (brief)
- `ModeDuoPossible` checks that `Interface<W>_J1.ini` exists. With 2 players, player n uses
  `Interface<W>_J<n+1>.ini` (ClsInterface.cls:339-347, 373-377).
- 1024 J1 versus solo:
  - all windows at x=56;
  - epoch hidden (anchor 496);
  - FPS at 912;
  - info buttons at x=482, step -24;
  - menu at 314;
  - 5 shortcut slots instead of 9;
  - statistics at y=725;
  - tooltip strings for the menu are inline.
- J2 uses right-half coordinates expressed in full-screen space:
  - windows x=648;
  - bars x=522, action x=664;
  - build buttons x=940 with `DecalageX=+80` (they slide out to the **right**);
  - move buttons x=968, `DecalageX=+52`, orders x=940;
  - temporary effects/shortcuts x=512;
  - info buttons start 888, step -24.
- Split rendering sets `ScreenPositionX = W/2` and `ScreenWidth = W/2` for player 2 (FrmMoteur2D.frm:1240-1300).
  Sprite blits and `Afficher_Barre_Statut` add `ScreenPositionX`, but `ClsIntMessage.DrawText` does not.
- Each player's message log is shifted by `i*W/2`. The confirmation window is centred within the player's half.
  The night overlay behind the confirmation dialog appears only in 1-player mode.

---------------------------------------------------------------------------------------------------
### 21. Gotchas to reproduce exactly
- Hit-test boxes are inclusive (W+1 x H+1).
- The shortcut slots use the `EffetTemporaire*` anchors. The temporary effects start at slot index 9 (x=344).
- Only `Etat(0..N)` are drawn for the build buttons, but N+1 buttons are hit-tested. The extra button never moves
  (it sits at its initial hidden position in grid order).
- The move buttons are drawn **opaque**. Every other HUD button sheet is colour-keyed on black.
- When auto-restore is OFF, the contour marker uses pct=100, so it is invisible.
- Bar frames use the *current ForeColor* (black for the stat bars, grey for the epoch progress bar).
- VB `Long` conversions round half to even (epoch text x, bar widths).
- Tooltip width counts the separator and `&` chars. The CR (Chr 13) and `#` separators are both recognised.

## 5. Windows ("Fenêtres", ClsIntFenetre) and minimap

Sources: `u8/ClsIntFenetre.cls` (CIF), `u8/ClsAffFenetre.cls` (CAF), `u8/ClsAffBouton.cls` (CAB),
`u8/ClsIntBouton.cls` (CIB), `u8/ClsInterface.cls` (CI), `u8/ClsIntMessage.cls` (CIM),
`u8/ClsAffSprite.cls` (CAS), `u8/ClsAffObjets.cls` (CAO), `u8/FrmMoteur2D.frm` (FM2D),
INI files in `u8/`, language files in `overlay/.../Vendetta/Donnees/Francais/` (FR is the selected
language: `Langues.ini` `LangueChoisie=1` → index 0 → `Francais\`). Line numbers = original files.

All coordinates below are **relative to the window origin** `(PositionX, PositionY)` unless stated
otherwise. VB colours are `&HBBGGRR` (BGR); RGB equivalents are given.

---------------------------------------------------------------------------------------------------

### 0. Quick reference table

| idx (code) | INI section | Window | Background image(s) | Default pos @1024x768 | Title text / position |
|---|---|---|---|---|---|
| 0 | Fenetre_1 | Bâtiment (house / castle / market) | Fenetre-Defaut | (704, 0) | house: `Batiment_Nom(type)`; castle/market: "Acheter" — at (128,32) |
| 1 | Fenetre_2 | Ressources | Fenetre-Defaut | (704, 368) | castle: "Vendre", else "Ressources" — (128,32) |
| 2 | Fenetre_3 | Caractéristiques (Personnage) | Fenetre-Personnage | (64, 0) | **character name** `Perso.Nom` at (128,32) |
| 3 | Fenetre_4 | Compétences | Fenetre-Competences | (64, 368) | "Compétences" (128,32) |
| 4 | Fenetre_5 | Inventaire | Fenetre-InventaireH / F / HM / FM | (384, 0) | "Inventaire" at **(32,32)** |
| 5 | Fenetre_6 | Bestiaire | Fenetre-Defaut | (384, 368) | "Bestiaire" (128,32) |
| 6 | Fenetre_7 | Carte (minimap) | Fenetre-Carte | (64, 0) | "Carte" (128,32) |
| 7 | Fenetre_8 | Objectifs | Fenetre-Defaut | (64, 0) | "Objectifs" (128,32) |

All windows: 320 x 368 (`Fenetres.ini`). Every window has the close button (X) at (280,16) and the
move button (+) at (256,16), both 24x24.

English titles (Donnees/English/Interface.ini): Inventory, Buy, Sell, Resources, Caracteristics,
Skills, Bestiary, Map, Objectives.

---------------------------------------------------------------------------------------------------

### 1. Fenetres.ini / ClsAffFenetre semantics

`Fenetres.ini` (u8/Fenetres.ini:1-57):
```
[Fenetres] NombreFenetres=8
[Fenetre_N]  (N = 1..8 → window index N-1)
  Largeur=320  Hauteur=368
  NombreImages=k   (optional; missing/0 → 1)
  Image1..Imagek = file stem in Images\Fenetres\ (".bmp" appended)
```
Loading (CI:354-370 `Init1`): for each window i, `AffFenetre(i).Largeur/Hauteur` are set **before**
`Definir_Nombre_Sprites` so every sprite gets `Selection(top=0, bottom=Hauteur, left=0, right=Largeur)`
(CAF:71-84) i.e. the top-left 320x368 of the bmp. `Definir_Nombre_Sprites = 0` is coerced to 1
(CAF:73). Images loaded in `Init2` (CI:776-778) via `ClsAffSprite.Charger` → path
`App.Path\Images\Fenetres\<ImageK>.bmp` (CAS:40-41,72-76; CAF:32,51).

Drawing (CAF:55-66): `Sprites(TypeFenetre).Afficher X, Y, SurfBack, Transparent=True, Translucide=False`
→ `BltFast` with source colour key. Only the inventory passes a non-zero `TypeFenetre` (see §8).

**Colour key**: surfaces are created with `DDSD_CKSRCBLT` and the default key 0 (CAS:66-67) → pure
black RGB(0,0,0) pixels are transparent. All window bitmaps (8-bit) use black for transparency.

**Stipple / "glass" effect**: the content area of every window bitmap is a 1-pixel checkerboard of
colour RGB(173,214,115) (#ADD673) and black (transparent). So the game world (or anything drawn under
the window before it) shows through every other pixel = a 50 % dithered translucency (visible in
`bw-game.png`). Parity (in image coordinates, colour pixel where `(x+y)%2 == p`):
- Fenetre-Defaut, Fenetre-Carte: colour on even `(x+y)` (p = 0), black on odd.
- Fenetre-Personnage, Fenetre-Competences, Fenetre-Inventaire*: colour on odd (p = 1).
Just blit the bmp with black as transparent; the parity then follows the window position.

Opaque bounding boxes (frame extents inside the 320x368 bitmap):
| image | x range | y range |
|---|---|---|
| Fenetre-Defaut | 6..313 | 6..361 |
| Fenetre-Carte | 6..313 | 6..350 |
| Fenetre-Personnage | 4..318 | 1..354 |
| Fenetre-Competences | 10..319 | 1..351 |
| Fenetre-InventaireH/F/HM/FM | 18..319 | 1..350 |

Image files (Images/Fenetres): Fenetre-Defaut, -Carte, -Competences, -InventaireF, -InventaireFM,
-InventaireH, -InventaireHM, -Personnage: all 320x368x8bpp. Others (not in-game windows):
Fenetre-Confirmation 320x160, Dialogue 640x240, VictoireDefaiteFR/EN 320x400, Feline / Golem d acier /
Tectomagicienne 1024x768.

Fenetre-Defaut layout: green rounded frame, top-right "button pocket" at about (248..304, 14..44)
behind the + and X buttons, the rest is checker. Fenetre-Carte: thicker title bar (y 6..~52) with a
title "pocket" around x 112..180, map area checker from y≈54 to ≈342. Fenetre-Personnage /
-Competences / -Inventaire have pre-drawn row boxes matching the text rows described below.

---------------------------------------------------------------------------------------------------

### 2. Boutons.ini / ClsAffBouton: sprite-sheet layout

`Boutons.ini` gives, per button family, the bmp name (`Images\Boutons\<Nom>.bmp`) and the cell size.
`ClsAffBouton.Charger dd, TypeBouton` (CAB:139-196) picks the family:

| Charger type | family | bmp | cell WxH |
|---|---|---|---|
| 0 | Construire | Boutons\Construire\<file> (48 files, one per building) | 28x28 |
| 1 | Infos (top bar window toggles) | Infos.bmp 216x72 | 24x24 → 9 cols x 3 rows |
| 2 | Action | Action.bmp 264x72 | 24x24 → 11 cols x 3 rows |
| 3 | Deplacement | Deplacement.bmp 644x140 | 28x28 → 23 x 5 |
| 4 | Commerce | Commerce.bmp 216x48 | 24x24 → 9 cols x 2 rows |
| 5 | Transfert | Transfert.bmp 144x48 | 24x24 → 6 cols x 2 rows |
| 6 | scroll up/down ("Defilement", hard-coded name) | Defilement.bmp 64x198 | 16x44 |
| 7 | Menu | Menu.bmp 128x64 | 32x32 → 4 x 2 |
| 8 | scroll bar | Defilement.bmp | 16x198 |
| 9 | scroll square | Defilement.bmp | 16x16 |
| 10 | Objets (hotkey bar) | Objets.bmp 288x192 | 32x32 → 9 x 6 |
| 11 | Competences (sort buttons) | Competences.bmp 112x32 | 28x16 → 4 cols x 2 rows |

**Cell selection** (CAB:198-217): `Afficher X, Y, TypeBouton, Etat, …` selects
`left = Largeur*TypeBouton`, `right = Largeur*(TypeBouton+1)`, `top = Hauteur*Etat`,
`bottom = Hauteur*(Etat+1)`. ⇒ **column = button kind, row = state**. There is NO hover/pressed
state in these sheets for window buttons; rows encode logical states (normal / disabled / …).
`Transparent` (optional, default False) → colour-keyed blit; many window buttons are drawn opaque
(their black pixels are drawn black).

#### Action.bmp (24x24, 11 columns x 3 rows)
| col | meaning (where used) |
|---|---|
| 0 | Manger (eat) – plate of food. row0 normal, row1 grey (dead) |
| 1 | Fabriquer (hammer). row0 can build, row1 red-crossed (can't), row2 = hourglass (era not reached) |
| 2 | Castle restore "X5" (coins) |
| 3 | Castle restore "X50" |
| 4 | Castle restore "MAX" |
| 5 | Service button (blue pipe). row0 usable, row1 grey |
| 6 | Close "X" |
| 7 | Inventory house toggle: row0 chest, row1 bag (state = `Bascule`) |
| 8 | Move "+" (4-arrow cross) |
| 9 | Inventory tab (grey circle). row0 = selected tab, row1 = other tab |
| 10 | Emperor crown: row0 gold (can become emperor), row1 silver (cannot) |
Rows: 0 = normal, 1 = disabled/grey/alternate, 2 = only used for col 1 (hourglass).

#### Commerce.bmp (24x24, 9 cols x 2 rows)
cols 0,1,2 = "take/buy" sack (x1, "X5", "MAX"); cols 3,4,5 = "sell" gold pile (x1, "X5", "MAX");
cols 6,7,8 = global "Tout", "I&C", ">20". row0 = alive, row1 = grey (dead).

#### Transfert.bmp (24x24, 6 cols x 2 rows)
cols 0,1,2 = red arrow down "récupérer" (from house) x1 / X5 / MAX; cols 3,4,5 = arrow up "déposer"
(to house) x1 / X5 / MAX. row1 = grey (never used: state always 0).

#### Defilement.bmp (64x198)
Each scroll part is a separate ClsAffBouton on the same bmp, with its own cell size; the column index
times *its own* width gives:
- up button: type 6, col 0 → src x 0..16, y 0..44 (triangle at top + bar cap)
- down button: type 6, col 1 → src x 16..32, y 0..44 (bar cap + triangle at bottom)
- bar: type 8, col 2 → src x 32..48, y 0..198
- square (thumb): type 9, col 3 → src x 48..64, y 0..16
All drawn transparent, state 0 (CIF:2491-2524).

#### Competences.bmp (28x16, 4 cols x 2 rows)
cols: "---" (no sort), "A-B" (alpha), "0-1" (by score), ">20" (show all). row0 = not selected,
row1 = selected. Drawn transparent.

#### Infos.bmp (24x24, 9 cols x 3 rows) – top-bar toggles (not inside windows, but they open them)
col i = button i: 0 Bâtiment, 1 Ressources, 2 Personnage, 3 Compétences, 4 Inventaire, 5 Bestiaire,
6 Carte, 7 Menu/Pause, 8 Objectifs. row0 off, row1 on (= window open), row2 = "new objectives"
highlight (only col 8, when `NouveauxObjectifs And Not Enabled`) (CI:1700-1721). Positions:
`(888 - 30*i, 0)` (Interface1024.ini BoutonInfos*). Col 8 only drawn when `AfficherObjectifs`.

#### Tooltip settings (Boutons.ini `[Boutons]`, CIB:54-64)
`AfficherInfoBulle=1`, `InfoBulleDecalageX=24`, `InfoBulleDecalageY=20`,
`CouleurInfoBulle=&H0&` (text black), `CouleurRectangleFond=&HC0FFFF` (= RGB(255,255,192) light
yellow), `BordureRectangleFond=2`.

---------------------------------------------------------------------------------------------------

### 3. Text rendering (fonts, colours, line breaks)

- All window text goes through `ClsIntMessage.Afficher X, Y, Message, SurfBack` (CIM:44-80) →
  `DirectDrawSurface7.DrawText X, Y, text, False` (GDI TextOut at top-left (X,Y), transparent
  background — no `SetFontTransparency`/background call anywhere).
- Font: `AffDegat.FontNormale` = `FrmChargement.LblFontNormale.Font` (Arial, Weight 700/bold in the
  form, FrmChargement.frm:526-536) with size forced to **10** pt (ClsAffDegats.cls:73,76). It is set on
  the back surface each frame at the end of `Afficher_Degats` (ClsAffDegats.cls:144; `AfficherDegats=1`
  in Combat.ini) which runs before the interface, and restored after the inventory hotkey labels
  (CI:1256). Measured in `bw-game.png`: cap height = 10 px, text cell top at Y, capital tops at
  **Y+2**, baseline (last ink row of caps) at **Y+11**, first ink column ≈ X+1. Glyphs are
  antialiased; the black window text looks like ~1–1.5 px strokes (white FPS text looks bolder).
  ≈ Arial 13.33 px em. Average advance ≈ 6 px/char (e.g. "Vie : 2000 / 2000" spans x 737..835).
- Small font `FontDegats` = Arial bold size `TailleNormale` = 8 (Combat.ini:32) used for the inventory
  hotkey labels "(n)", "x<N>" stack counts and recharge "a/b" counters.
- Logical char metrics used for layout computations: `LargeurCaractere = 7`, `HauteurCaractere = 16`,
  line spacing `InterligneY = 16` (CIM:38-42).
- Default window text colour: ForeColor 0 = black (set by `Afficher_Titre`, CIF:1483, and elsewhere).
- Line breaks: `#` (CaractereRetourLigne) or Chr(13) → next line at Y+16 (CIM:64-74), recursive.
- Per-line colour prefix (first char of each line only): `&` → `&HFF&` = RGB(255,0,0) red;
  `$` → `&HFF0000` = RGB(0,0,255) blue (CIM:33-36,54-63). The prefix char is not printed; following
  lines revert to the base colour unless they have their own prefix.
- Special colours inside windows: bonus values / active skill: `&H80&` = RGB(128,0,0) dark red
  (CIF:257,279,280); fabrication queue "+N": white `&HFFFFFF` (CIF:1795).
- Numbers are concatenated with VB `&` → no thousands separators; `Int()` = floor.

---------------------------------------------------------------------------------------------------

### 4. Common chrome: title, close, move

Constants (CIF:48-57):
- Title: `TitrePositionX = 128`, `TitrePositionY = 32` (left-aligned, NOT centred), black
  (`Afficher_Titre`, CIF:1480-1488).
- Move "+": `BoutonDeplacerPositionX = 256+40-16-24 = 256`, `Y = 16` → (256,16).
- Close "X": `BoutonFermerPositionX = 256+40-16 = 280`, `Y = 16` → (280,16).
- `Afficher_BoutonFermer` (CIF:1490-1501) draws Action col 6 (X) at (280,16) and Action col 8 (+) at
  (256,16), state 0, **opaque** (no Transparent flag). Size = AffBoutonAction 24x24.
- Tooltips: close "Fermer (espace)", move "Déplacer fenêtre" (Donnees/Francais/Fenetres.ini).

Click semantics (CI:3873-3923, loop over windows i = 0..7 in index order, first visible hit wins):
- Close, left click → `Actionner_Boutons_Infos(i)` (window 7 → index 8). This toggles
  `BoutonsInfos(i).Enabled` and `Fenetres(i).Visible = Not Enabled(before)` (CI:4726-4742).
  Closing the map (6) re-enables auto-scrolling.
- Move, left click → `Fenetres(i).Deplace = True`, `DeplacerFenetre = i` (enter drag mode).
- Move, right click → `Deplace = False` and reset to the INI default position.
- Hover → tooltip.

---------------------------------------------------------------------------------------------------

### 5. Window positions, dragging, clamping, persistence

- Defaults from `Interface<width>.ini` (`Interface1024.ini` lines 15-30): see table §0.
  Other resolutions: 640: 1,3,5,7,8 at (320,0), 2,4,6 at (0,0); 800: 1,3,5,7,8 (480,0), 2,4,6 (160,0);
  1280: 1 (960,0) 2 (960,368) 3 (64,0) 4 (64,368) 5 (512,0) 6 (512,368) 7,8 (64,0);
  1600: 1 (1280,0) 2 (1280,368) 3 (64,0) 4 (64,368) 5 (1280,736) 6 (64,736) 7,8 (64,0).
  Split screen 1024 J1: all x=56, J2: all x=648 (y 0 or 368).
- **Drag-move** (FM2D:599-610, 1217, 1345-1350; CI:914-935,1020-1025):
  1. Click "+" → `DeplacerFenetre = i`.
  2. Each frame `Position_Contour_Fenetre` **clamps the mouse cursor itself** (SourisX/Y passed ByRef)
     to `X ≤ ScreenW - 320`, `Y ≤ ScreenH - 368` (only max, no min).
  3. Drawn: the window still at its old place; then a white (`&HFFFFFF`) 1-px hollow rectangle
     (`SetFillStyle 1`, `DrawBox X, Y, X+320, Y+368`) with its top-left **at the mouse cursor**,
     then the normal arrow cursor (type 0). The hit-test/tooltip pass still runs during the drag.
  4. Left click → `Positionner_Fenetre` sets window top-left = cursor position, ends drag.
     Right click → cancel (`DeplacerFenetre = -1`). The click is consumed (not passed to the game).
- **Clamping** every frame (`RePositionner_Fenetres`, CI:1027-1046) for all 8 windows:
  `x = clamp(x, ScreenPositionX, ScreenPositionX + ScreenW - 320)`,
  `y = clamp(y, ScreenPositionY, ScreenPositionY + ScreenH - 368)` → at 1024x768: x∈[0,704], y∈[0,400].
- Persistence (ModSavPerso.bas:322-329, 356-365): for windows with `Deplace=True`, the per-character
  save stores `Fenetre(N)_Deplace=1`, `Fenetre(N)_X`, `Fenetre(N)_Y`.

---------------------------------------------------------------------------------------------------

### 6. Visibility, toggling, z-order, mutual exclusion

- There is **no mutual exclusion**: every window is independent; several share default positions
  (3, 7, 8 at (64,0)) and simply overlap.
- Toggling: top-bar Infos buttons (left or right click), hotkeys (Touches.ini DirectInput scancodes):
  B (48) bâtiment, R (19) ressources, P (25) personnage, T (20) compétences, I (23) inventaire,
  U (22) bestiaire, Tab (15) carte, F11 (87) objectifs (only when `AfficherObjectifs`), Space (57)
  closes all (`Fermer_Toutes_les_fenetres`, CI:4744-4752: hides windows 0..7 and clears
  `BoutonsInfos(0..7).Enabled` — note this includes the pause button 7 but not the objectives
  button 8; a quirk). ClsIntTouches.cls:505-514.
- At game start `Actionner_Boutons_Infos 0` → building window "enabled" (FM2D:388).
- **Window 0 (bâtiment) visibility is recomputed each frame** (CI:1134-1139):
  `Visible = ((Vivant And (DansUneMaison Or DansSaMaison)) Or DansUnChateau Or DansSonChateau
  Or ((DansSaMaison Or DansUneMaison) And Maison.Service = 1)) And BoutonsInfos(0).Enabled`
  → it only appears while inside a building (house of anybody / castle) and toggled on.
- **Draw order** (CI:1117-1203; later = on top): 4 Inventaire, 0 Bâtiment, 1 Ressources,
  2 Caractéristiques, 3 Compétences, 5 Bestiaire, 6 Carte, 7 Objectifs. No "bring to front".
- Where windows sit in the frame (CI:821-878, FM2D:1217-1350, 1493): world, damages, comments →
  `RePositionner_Fenetres` → messages → HUD (status bars, statistics, epoch, hotkey bar, temp effects,
  move/build buttons, action text, Infos buttons, Menu buttons) → **windows** → radial buttons →
  visual effects → dialog → confirmation → then `Interieur_Boutons` (hit-test pass, which draws
  hover frames and **tooltips on top**) → drag outline / cursor → FPS text.
- Input precedence inside `Interieur_Boutons` (CI:2540-4333), first match wins:
  confirmation dialog → radial buttons → menu buttons → build buttons → **map area (win 6)** →
  caracs resistances zone (win 2) → ressources (win 1) → bâtiment (win 0) → compétences (win 3) →
  inventaire (win 4) → bestiaire (win 5) → close/move of all windows (0..7) → "take money" zones
  (win 1 and 4) → move/order buttons → Infos buttons → hotkey bar → temp effects → status bars.
  Precedence is by this fixed order, **not** by window z-order.
- **Clicks on empty window areas are NOT swallowed**: the "inside a window" test is commented out
  (CI:4314-4329), so a click on the window background that misses every button goes through to the
  game world (move/attack). Only the screen border (`BordureGauche..` all 0) is treated as interface.

---------------------------------------------------------------------------------------------------

### 7. Scroll bars (Défilement)

Constants (CIF:212-223):
- up button (index 0): `(320-16-4, 66)` = **(300, 66)**, 16x44
- down button (index 1): `(300, 368-44-16)` = **(300, 308)**, 16x44
- bar (index 2): `(300, 66+44)` = **(300, 110)**, 16x198 (y 110..308)
- square/thumb: x 300, `y = 110 + (198 - 16) * offset / DefilementNombreLigne` = `110 + 182*offset/N`
  (CIF:2520-2522; `offset` = `DefilementObjetDecalageX` for the crafting list, else
  `DefilementDecalageX`).
`Afficher_BoutonsDefilement(N, …)` (CIF:2491-2524) sets `DefilementNombreLigne = N` and always draws
all four parts (transparent). Who calls it and with which N:
- Ressources lists (win 0 castle/market, win 1): N = `NombreRessources - 11` (27 resources → 16),
  always drawn.
- Crafting list (win 0, fabrique): N = `NombreObjetsFabriques - 8`, drawn only if > 8 items, else
  the three scroll buttons are made invisible.
- Compétences: N = `Count - 1 - 15`, drawn only if `Count > 16`.
- Bestiaire: N = `Nombre_de_lignes_du_bestiaire - 1 - 15`, always drawn.
Tooltips: up "Défilement haut (Roulette souris)", down "Défilement bas (Roulette souris)", bar "".

Behaviour (`Interieur_BoutonsDefilement`, CI:4480-4536; hit boxes inclusive):
- up: left click (edge) → `Defilement_Haut 1`; down: left click → `Defilement_Bas 1`.
- bar: while the left button is **held** → `offset = Int(N * (mouseY - (winY+110)) / 198 + 0.5)`
  (direct jump, drag-able). Windows 3 and 5 set their own offset; windows 0/1 set **both**
  window 0 and window 1 offsets (shared resource scroll); crafting sets `DefilementObjetDecalageX`.
- `Defilement_Haut(v, w)` (CI:4538-4562): v clamped to [-1,1]. Crafting: if off>0 then off -= v.
  Windows 3/5: off -= v, min 0. Windows 0/1: `F1.off = Fw.off - v` (min 0), `F0.off = F1.off`.
- `Defilement_Bas(v, w)` (CI:4564-4588): crafting: if off < N then off += v. Windows 3/5: off += v,
  max N. Windows 0/1: `F0.off = Fw.off + v` (max N), `F1.off = F0.off`.
- Crafting offset is additionally clamped to [0, NbObjets-8] at draw time (CIF:1728-1733).
- **Mouse wheel** (FM2D:640-678): each wheel event (any cursor position, unless radial menu open or
  cursor on a status bar) scrolls **every visible scrollable window by exactly one line**
  (|Z| clamped to 1): Z>0 = up. Windows 0/1: if win 0 visible → resource list scroll on window 0
  (+ crafting list if in own house / in a fabrique); else window 1. Plus window 3 and window 5 if
  visible.

---------------------------------------------------------------------------------------------------

### 8. Tooltips (info-bulles)

`ClsIntBouton.Afficher_Info_Bulle(X, Y, …)` (CIB:66-125), X,Y = mouse position:
```
if AfficherInfoBulle and text != "" (or forced):
  X += 24; Y += 20
  // size: split on '#' and Chr(13); W = max segment length in chars (segment length includes its
  // trailing separator, colour-prefix chars are counted) ; H = number of segments
  W *= 7 ; H *= 16
  if X < ScreenPosX: X = ScreenPosX  elif X + W + 2 > ScreenW: X = ScreenW - W - 2
  if Y < ScreenPosY: Y = ScreenPosY  elif Y + H + 2 > ScreenH: Y = ScreenH - H - 2
  ForeColor = black, FillColor = RGB(255,255,192)
  DrawBox(X-2, Y-2, X+W+2, Y+H+2)      // 1-px black border, light-yellow fill
  draw text at (X, Y) in black with '#'/'&'/'$' rules (16 px lines)
```
`Afficher_Info_Bulles_Terrain` (CI:2170-2178) uses the same routine with forced display (used for
emperor button, map enemy names, resistances, confirmation, etc.).

Note on fills: the code relies on dx7vb `SetFillStyle` (VB constants 0 solid, 1 transparent,
4 upward diagonal, 6 cross-hatch). Status bars and tooltips are drawn with whatever fill style was
last set, yet appear filled in screenshots; hollow outlines (drag contour, map viewport, inventory
hover frames) explicitly set style 1. Port rule: tooltips/status bars filled; contours hollow.

Hover highlight: window buttons have **no hover image state**. The only hover visuals inside windows
are the orange inventory frames (§12) and tooltips.

---------------------------------------------------------------------------------------------------

### 9. Window 0 — Bâtiment

Background Fenetre-Defaut. Branch (CI:1153-1162):
- `(DansUneMaison Or DansSaMaison) And Not Batiment_Marche(type)` → **house view**
  `Afficher_InfosMaison(Batiment_Nom(type), …)` (CIF:676-694).
- else (castle — own or other — or a *market* building) → **buy view**
  `Afficher_InfosChateau("Acheter", …)` (CIF:696-715).

#### 9.1 House view (own house / someone else's house)
Draw order: title (128,32) = building name; close/move; état; [money if own]; stock; objects.

| element | position | content / source |
|---|---|---|
| Life (`EtatBatiment`, CIF:69-70,1556-1562) | (32,54) | `EtiquetteVie & " : " & Int(Maison.Definir_Vie) & " / " & Maison.MaxVie` → "Vie : 120 / 200" |
| House money (own house only, CIF:61-62,1565-1587) | (32,96) | `"Argent : " & Int(Maison.Argent)` (first letter of `EtiquetteArgent`="argent" upper-cased) |
| House-money transfer buttons (own house & built) | (216+24i, 90), i=0..2 | Transfert cols 0,1,2 (arrow down) opaque. Tooltips: "Récupérer l'argent : 1 pièce" / "… : 5 pièces" / "… : tout (Page Préc)". Left click → take 1 / 5 / all money from house (CI:3226-3251) |
| Stock line (CIF:71-72,1589-1701) | (32,138) | see below |

Stock line variants (`StockBatimentPositionY = 96+42 = 138`):
- Fabrique (crafting building): `"Objets : " & "Stock vide"` if Int(stock)<1 else
  `"Objets : " & Int(stock) & " / " & MaxStock`.
- Magasin (resource producer): `Ressources_Nom(res) & " : Stock vide"` or
  `Ressources_Nom(res) & " : " & Int(stock) & " / " & MaxStock`; if built, 3 buttons at
  **(216+24i, 132)**: own house → Transfert cols 0..2 (retrieve stock: left click 1 / 5 / all,
  tooltips "Récupérer Ressource : 1 unité|5 unités|tout"); someone else's shop → Commerce cols 0..2
  (buy: left click 1 / 5 / Max-carry; tooltip "Acheter 1 unité|Acheter 5 unités|Acheter tout"
  + "#Prix : P"; actual tooltip built by `BoutonCommerce_Afficher_InfoBulle`, CIF:2712-2784, i.e.
  "Acheter <q> <Nom>  #Prix : <q*PrixVente*CoefficientAchat>" with "&" (red) if price > money).
- Service building: if MaxStock>0: `Service_Nom & " : Indisponible"` (stock<1) or
  `Service_Nom & " : " & Int(stock) & " / " & MaxStock`; service button Action col 5 at
  **(216,136)** (`ServiceBoutonPosition = (184+32, 96+16+24)`), state 0 if
  `Service_Utilisable` else 1, opaque. If `AutoBatimentService`: orange (`BarresStatutContourCouleur2`
  = &H80FF& = RGB(255,128,0)) outline box (x-1,y-1)-(x+25,y+25) plus a 1-px vertical tick at x+24
  (`Afficher_Contour_Barre_Statut` with PCent=1, CI:2153-2168). Then, if MaxStock>0 or own house,
  the service description `Service_Description(service)` at **(32,160)** (multi-line with '#').
  Tooltip: "" & "Prix : <price>" & " (Entrée)" (price haggled in other houses). Left click → use
  service (if usable); right click → toggle AutoBatimentService (CI:3377-3399).

#### 9.2 Crafting list (fabrique), `Afficher_Objets_Batiment` (CIF:1703-1858)
Constants (CIF:100-116): 8 rows (`NombreLignesObjets`), row y = `160 + 24*r` (r = 0..7,
FabricationPositionY = 96+96-32 = 160), x base 32.
Per item `i` from `DefilementObjetDecalageX` (rows r=0..7):
| element | position (window rel.) | details |
|---|---|---|
| name | (32, y) | `Objet_NomPluriel(obj)` black |
| separator line | (32, y+18) → (248, y+18) | black `DrawLine`, length 216 (`SeparateurRessource*`, CIF:145-147, 2484-2489) |
| icon | (168, y-6), 24x24 | object sprite 32x32 scaled 0.75 (`AffObjets.Afficher2`, Blt stretch, colour-keyed) |
| quantity | (200, y) | `"x" & Int(Maison.Stock_Objets(i))` |
| fabricate button | (232, y-4) | Action col 1, transparent. Own house: state 2 (hourglass) if `Objet_Epoque-1 > Fief.Epoque`, 0 if `PeutConstruire_Objet`, else 1. Other houses: only drawn (state 2) when era-locked |
| queue count (own house) | (232, y) | `"+" & n` in **white** if n queued (count of item in `FileFabricationObjets`) |
| progress (own house) | (264, y) | `Int(EtatFabricationObjet / Objet_DureeFabrication * 100) & "%"` for the item being made |
Scroll bar (§7) if > 8 items.

Fabricate-button tooltip (CIF:1791-1840): `"Fabriquer : " & Objet_Nom` then, if era-locked,
`Chr(13) & "&(" & Epoque_Nom(era-1) & ")"` (red); then for each resource cost:
`Chr(13) & ["&" if insufficient] & "(" & [Int(have) & " / " if insufficient] & cost & " " & lcase(name) & ")"`.

Clicks (CI:3283-3344, 3401-3478), quantity modifier q = Shift+Ctrl→20, Shift→10, Ctrl→5, else 1
(`FileFabricationObjets_Changer`, CI:4336-4377; same mapping for object take/buy):
- Fabricate button: left (alive, enabled) → enqueue q; right → dequeue q (if queue empty → stop
  fabrication).
- Icon hit box (168, y-6) **48x24** (icon width*2): left in own house → enqueue q if fabricable and
  stock 0, otherwise take q objects from stock; in another's shop → buy q if stock>0 and price>0;
  right in own house → dequeue q; hover → full object tooltip (`Definir_InfoBulle_Objet`,
  CIF:1860-2330).

#### 9.3 Buy view (castle / market) — the screenshot case
`Afficher_InfosChateau` (CIF:696-715): title "Acheter" at (128,32); close/move; état; resources list.
- État: if in a house (market) → as 9.1 (house life). Else castle (`Chateau=True`, CIF:1534-1555):
  - text at (32,54): `"Vie : " & Int(Vie) & [" (" & Int(Definir_Vie) & ")" if different] & " / " & MaxVie`
    (screenshot: "Vie : 2000 / 2000").
  - Emperor button: Action col 10 at **(140,16)** (`160-12-8`, `24-16+8`), 24x24, transparent,
    state 0 if `PeutDevenir_Empereur` else 1 (silver crown in screenshot). Click (L or R) → become
    emperor. Tooltip: current emperor `"<title> <name>"`, or
    `"Devenir " & lcase(title) & " (Entrée)" & "#" & ["&"] & price & " argent"`.
  - Restoration buttons: Action cols 2,3,4 at **(216+24i, 48)**, opaque, state 0 alive / 1 dead.
    Left click → restore castle with 5 / 50 / all money. Tooltips "Restaurer la forteresse#+1 vie
    pour 5 argent", "…#+10 vie pour 50 argent", "…#Tout donner".
- Resource list `Afficher_ListeRessourcesFief` (CIF:2439-2482), 11 rows:
  row r (resource k = r + offset), `y = 88 + 24*r` (`RessourcesPositionY = 112-24`):
  | element | position | content |
  |---|---|---|
  | label | (32, y) | castle: `"Prix : " & PrixVente(k)*CoefficientAchat` ; market: `Ressources_Nom(k) & " :"` |
  | amount | (136, y) | `Int(Fief.Definir_ressources(k))` (castle stock) |
  | separator | (32,y+18)-(248,y+18) | black |
  | icon | (104, y-10) | resource sprite 32x32, colour-keyed |
  | 3 buttons | (216+24j, y-6), j=0..2 | Commerce cols 0,1,2, opaque, state 0 alive / 1 dead |
  Button tooltip set to `"Prix : " & PrixVente*Coef` at draw, but the hover routine rebuilds it:
  `"Acheter <q> <Nom>  #[&]Prix : <q*PrixVente*Coef>"` with q = 1 / 5 / (Max - have) capped by
  castle stock (CIF:2712-2784).
  Left click (alive) → buy 1 / 5 / MaxRessources(k) (CI:3189-3219).
- Scroll bar (§7) at x=300, N = NombreRessources - 11; shares its offset with window 1.

Screenshot check (`bw-game.png`, window at 704,0): "Acheter" glyphs start x=833 (=704+128+1),
"Vie : 2000 / 2000" caps rows 56..65 (=54+2..+11), "Prix : 6" caps rows 90..99, separator at y=106,
icons at x≈808, amount "0" at x≈840, button triplets at x=920/944/968 and y=82,106,…; X5/X50/MAX
restoration at (920..992, 48..72); crown at (844,16); scroll parts at x=1004: up 66..110, bar
110..308 with the thumb at 110, down 308..352.

---------------------------------------------------------------------------------------------------

### 10. Window 1 — Ressources (`Afficher_InfosRessourcesPerso`, CIF:718-730, 2332-2437)

Background Fenetre-Defaut. Title "Vendre" if `DansUnChateau Or DansSonChateau`, else "Ressources".
Draw order: title, close/move, global buttons, rows, scroll bar, then money line (+ transfer buttons).

- Global buttons (CIF:2346-2357): `BoutonsCommerce(UBound-i)` at **(216+24i, 48)**, i=0..2 drawn
  with Commerce col `6+i` ("Tout", "I&C", ">20"), state 1 if dead, opaque.
- Rows r = 0..10, k = r + Int(offset), `y = 88 + 24*r`:
  | element | position | content |
  |---|---|---|
  | label | (32, y) | in castle: `"Prix : " & PrixVente(k)` (sale price); else `Ressources_Nom(k) & " :"` |
  | amount | (136, y) | `Int(have) & (" / " if Max<100 else "/") & Int(Max)` |
  | separator | (32,y+18)-(248,y+18) | black |
  | icon | (104, y-10) | resource sprite |
  | 3 commerce buttons | (216+24j, y-6) | Commerce col `j + (3 if in castle else 0)` (sell piles vs take sacks), opaque, state 1 if dead |
  | eat button (edible resources) | (184, y-6) | Action col 0, state 0 alive / 1 dead, opaque |
  | deposit-to-house buttons | (216+24j, y-6) (drawn over the commerce buttons) | only if in own house and k = the house's produced resource and MaxStock>0: Transfert cols 3..5 (arrow up). Left → deposit 1 / 5 / all. Tooltips "Déposer Ressource : 1 unité/5 unités/tout" |
- Scroll bar: N = NombreRessources - 11 (always).
- Money line (`Afficher_Argent_Perso`, CIF:1503-1525): `"Argent : " & Int(Perso.Definir_Argent)` at
  **(32,54)**; if in own house: Transfert cols 3..5 at **(216+24i, 48)** (drawn over the global
  buttons; they take hit priority). Left → deposit 1 / 5 / all money. Tooltips
  "Déposer l'argent : 1 pièce" / "… : 5 pièces" / "… : tout  (Page Suiv)".
- Money "take" zone (CI:3924-3953): rect (32,54) 100x24 (window 1 and 4): left click → put 100 money
  on the cursor, right click → all money; tooltip "Prendre Argent".

Clicks on row buttons (CI:2928-2990), L or R, alive:
- in castle / own castle / market building: sell 1 / 5 / all (left or right same).
- elsewhere: left = "take" (put on cursor to drop/give) 1 / 5 / all; right = "laisser" (put back).
Global buttons (CI:3020-3096): "Tout" (idx case 2) → sell all / take all; "I&C" (case 1) → only
resources that are not edible, not needed to build, not needed to attack; ">20" (case 0) → surplus
above 20 per resource. Tooltips: castle/market "Vendre toutes les#ressources", "Vendre toutes
les#ressources immangeables,#inutiles pour attaquer#et inutiles à la construction", "Vendre tout
le#surplus de#ressources (>20)"; elsewhere "Prendre …" variants (Fenetres.ini).
Row-button hover tooltip: `("Vendre " | "Acheter " | "Prendre ") & q & " " & Nom & "  #" & "Prix : " & P`.
Eat tooltip (CIF:2616-2650): `"Manger : " & lcase(nom) & "  "` + lines `Chr(13) & "+" &
Format(gain*coef,"0.0") & " Vie|Energie|Magie|Moral"`. Left click (alive) → eat.

---------------------------------------------------------------------------------------------------

### 11. Window 2 — Caractéristiques (Fenetre-Personnage)

Drawn in 3 passes (CI:1172-1176): `Afficher_InfosCaracPerso1` (progress bars) → **window bitmap on
top** → `Afficher_InfosCaracPerso2` (texts). The bars therefore show through the checkerboard (50 %).

Row Y constants (CIF:226-250, `70 + 24*n - 12` …):
| row | Y | label (Parametres.Etiquette*, FR) | value at X=104 (32+72) |
|---|---|---|---|
| Métier | 58 | "Métier :" | `Batiment_Metier(type, feminin)` (only if house built); if emperor: no label, value `Epoque_EmpereurNom(epoch, feminin)` |
| Niveau | 82 | "Niveau :" | `Int(Niveau)` |
| Vie | 106 | "Vie :" | `Int(Vie) & " / " & Definir_MaxVie` |
| Energie | 130 | "Energie :" | `Int(Energie) & " / " & MaxEnergie` |
| Magie | 154 | "Magie :" | `Int(Magie) & " / " & MaxMagie` |
| Moral | 178 | "Moral :" | `Int(Moral) & " / " & MaxMoral` |
| Attaque | 214 | "Attaque :" | `Int(Definir_Attaque) & " / " & Definir_MaxAttaque` |
| Défense | 238 | "Défense :" | `Int(Definir_Defense) & " / " & MaxDefense` |
| Armure | 262 | "Armure :" | `Definir_Armure` |
| Vitalité | 298 | "Vitalité :" | `Int(Vitalite*100) & " %"` |
| Vitesse | 322 | "Vitesse :" | `UCase("Max")` if at max step, else alive: `Longueur_PasTemp & " / " & Int(BonusDeplacement*Vitalite)`, ghost: `Definir_VitesseDeplacementFantome` |
- Labels at X=32, all text black.
- Name `Perso.Nom` at (128,32) (no title text).
- "Résistances" (no colon) at **(176,262)**; hover zone (176,262) 120x16 (CIF:249-252, 3138-3189;
  CI:2853-2869) → tooltip: attack-resistance type name, then per resistance ≠ default:
  `"#" & ["&" if worse] & NomComplet & " : " & (MAX | MIN | ["+"]Format(-(r-1)*100,"0") & "%")`.
- "Niv+ :" at **(240,58)** (`CaracNivPlusPosition = (128+32+64+16, 70-16+16-12)`).
- Progress percentages at X = 32+212 = **244**, row Y: Niveau `Int((Format(Niveau,"0.00") -
  Int(Niveau))*100) & " %"`; Vie `Int(Progression_Vie*100) & " %"`; Energie/Magie/Moral/Attaque/
  Défense `Int(p*100) & "%"` (no space).
- Progress bars (CIF:739-764) for Niveau, Vie, Energie, Magie, Moral, Attaque, Défense at
  **(215, rowY-4)**, size **80x24** (`CaracNivPlusProgression*`: 32+183, -4, 78+2, 22+2), drawn by
  `Afficher_Barre_Statut(value, 0, 1, …, colour=&HFFFFFF)` (CI:1998-2080): black rectangle
  (x,y)-(x+80,y+24), then white fill (x+1,y+1)-(x+80*value, y+24).
- Bonuses in dark red RGB(128,0,0) at X = 32+147 = **179**, row Y (CIF:881-914): for Vie, Energie,
  Magie, Moral, Attaque, Défense if |bonus| ≥ 1: `["+"] & Int(bonus)`; Vitalité if BonusVitalite≠1
  and alive: `("+"|"-") & Int((BonusVitalite-1)*100) & "%"`; Vitesse if |BonusDeplacement| ≥ 1 and
  alive: `["+"] & bonus`.

---------------------------------------------------------------------------------------------------

### 12. Window 3 — Compétences (Fenetre-Competences)

Also 3 passes (CI:1178-1182): bars → bitmap → texts. The skill table `TabComp` is rebuilt only
once per second while the window is visible (`ActualiserLent`, CI:2384) and when sort buttons are
clicked; each entry = {Nom (with " :"), Valeur, NivPlus (0..1), Active, Bonus} (CI:1341-1493).
Default sort `CompetencesTri = 1` (alphabetical), saved per character.

- Title "Compétences" (128,32); close/move.
- "Niv+" (no colon) at **(240,58)**.
- Sort buttons (CIF:286-300, 1027-1031): Competences.bmp col i at (43,61), (75,61), (106,61),
  (146,61), 28x16, transparent; state = 1 if (i<3 and CompetencesTri = i) or (i=3 and
  AfficherToutes) else 0. Tooltips "Aucun tri", "Tri alphabétique", "Tri croissant",
  "Afficher toutes". Click (L or R): i<3 → set sort (if changed), re-sort, scroll to 0; i=3 → toggle
  "show all" (also includes skills with level 0), scroll to 0 (CI:3481-3515).
- Rows: up to **16** rows (loop exits when Y > 15), `y = 86 + 16*Y` (Y=0..15 → 86..326):
  | element | position | content |
  |---|---|---|
  | name | (32, y) | `Nom` ("Bûcheron :"…), dark red RGB(128,0,0) if Active else black |
  | value | (216 - 8*Len(Str(Valeur)), y) | right-ish aligned: `CompetencesScoreDecalageX = 184`, `CompetenceLargeurTexte = 8`; `Str()` adds a leading space for positives, so x = 216 - 8*(digits+1). Dark red if Bonus else black |
  | percent | (240, y) | `Format(Int(NivPlus*100),"00") & "%"` black |
  | bar (under bitmap) | (215, y) size **81x17** | `CaracNivPlusProgression*2`: 183, 0, 78+2+1, 22-2-3; white fill = NivPlus |
- Scroll bar if Count > 16, N = Count - 16; hit-tested only when visible.

---------------------------------------------------------------------------------------------------

### 13. Window 4 — Inventaire (Fenetre-Inventaire*)

Background variant (CI:1146): `TypeFenetre = (Bascule and DansSaMaison and Vivant) ? (feminin ? 3 : 2)
: (feminin ? 1 : 0)` → Image1 InventaireH (male), Image2 InventaireF (female), Image3 InventaireHM
(male, house inventory shown), Image4 InventaireFM.

`Afficher_InfosInventaire` (CIF:1180-1260) + `Afficher_TouchesObjets` (CI:1205-1257):
- close/move; `"Argent : N"` at (32,54) (+ deposit buttons at (216+24i,48) in own house, same as
  window 1; money "take" zone 100x24 at (32,54)); title "Inventaire" at **(32,32)**.
- Equipment slots (32x32 icons, CIF:413-436), index → (x,y):
  Main1 (right hand, 0) (38,124); Main2 (1) (238,124); Armure (2) (137,88); Tête (3) (137,39);
  Pieds (4) (137,162); Dos (5) (187,88); Anneau1 (6) (38,162); Anneau2 (7) (238,162);
  Bras (8) (88,104); Orbe (9) (38,72); Monture (10) (187,162).
  Two-handed weapon: also drawn **translucent (50 % alpha)** in slot Main2 (CIF:1203-1209).
- Backpack grid (CIF:186-193, 609-626): 32 slots, 8 columns x 4 rows, pitch 32,
  `x = 32 + 32*col`, rows y = **209, 241, 273, 305** (the init loop starts the first row at Y=-1,
  i.e. `241 - 32`; for slot ≥ 32 — further tabs — rows would be 241..337, an original bug; with the
  shipped 32-slot inventory there is a single tab and tab buttons are not drawn).
  Content: own inventory unless (Bascule=1 and in own house and alive) → house inventory
  (`Maison.Definir_ObjetsInventaire`). Recharge overlay animation (`Afficher_Duree_Recharge`).
- Tab buttons (if >1 tab): Action col 9 at (8, 212 + 25*i), state 0 selected / 1 other.
- House toggle (own house & alive): Action col 7 at **(288,208)**, state = Bascule (chest / bag),
  transparent. Tooltip "Baculer vers#Inventaire Maison" / "Baculer vers#Inventaire Personnage", or
  with an object on cursor "Déposer Dans la maison" / "Récupérer l'objet". Click toggles
  Bascule, or deposits/retrieves the cursor object.
- Hotkey labels (Arial bold 8, black): `"(n)"` (or `"(+)"`) at slot top-left for objects bound to
  hotkeys; stack count `"x" & n` at (slotX+4, slotY+19) when n>1.
- Hover (CI:3707-3735): orange `&H80FF&` = RGB(255,128,0) 1-px hollow frame (slotX,slotY)-(+32,+32)
  around the hovered slot; for an item, the object tooltip, and the destination equipment slot(s)
  framed (x-2,y-2)-(x+34,y+34) (both hands for weapons, both rings for rings) (CIF:2994-3028).
- Clicks (CI:3548-3789): left = pick up / drop / swap / stack on cursor; Ctrl+left = quick discard
  (in own house deposit, in castle/market sell, else drop on ground); right = equip (backpack) or
  unequip (equipment); hotkey digit while hovering assigns that object to the hotkey.

---------------------------------------------------------------------------------------------------

### 14. Window 5 — Bestiaire (Fenetre-Defaut, CIF:1262-1302)

- Title "Bestiaire" (128,32), close/move.
- Iterates i = 0..Nombre_de_lignes_du_bestiaire-1 over `Bestiaire_Ordre_Alpha(i)`; shows only races
  with kill count > 0 **and** `i >= DefilementDecalageX` (offset compares raw list index, not the
  visible index — quirk). Up to 16 rows, `y = 86 + 16*X`:
  name `Race_Nom(r, False, plural = count>=2) & " :"` at (32,y); count at **(192,y)**. Black.
- Scroll bar always, N = Nombre_de_lignes - 16.

---------------------------------------------------------------------------------------------------

### 15. Window 6 — Carte (minimap) (Fenetre-Carte, CIF:1304-1449)

Constants (CIF:311-332): `CartePositionXDepart = 16`, `CartePositionYDepart = 86-32 = 54`,
`CarteDimension = 304-16 = 288`.

Geometry (recomputed every draw; Integer vars → VB rounding, banker's on .5):
```
WW = Monde.Largeur * Monde.LargeurCase      // world size in pixels
WH = Monde.Hauteur * Monde.HauteurCase
if Monde.Largeur > Monde.Hauteur:            // tiles count comparison
    CL = 288 ; CH = 288 * Hauteur / Largeur
    dx = winX ; dy = winY + (288 - CH)/2
else:
    CL = 288 * Largeur / Hauteur ; CH = 288
    dx = winX + (288 - CL)/2 ; dy = winY
Ox = 16 + dx ; Oy = 54 + dy                  // map top-left on screen
sx(wx) = Ox + wx / WW * CL ;  sy(wy) = Oy + wy / WH * CH
```
Drawing order:
1. Title "Carte" (128,32) and close/move.
2. Background:
   - Scenario map loaded (`Cartes.Chargee`) and `Cartes\<map>\Carte.bmp` exists → that bitmap
     (pre-rendered, 288 x 288*H/W, e.g. 288x288 or 288x216) blitted **opaque** at (Ox,Oy). It is
     loaded lazily on the first draw (nothing drawn that frame).
   - Otherwise (random world / no image): `DrawBox(Ox, Oy, Ox+CL, Oy+CH)` with fore & fill colour
     `&HC0FFC0` = RGB(192,255,192) and **FillStyle 6 (cross-hatch)** → light-green grid over the
     window's checkerboard. There is no per-terrain colouring in code; terrain only appears via
     Carte.bmp.
3. Castles (FillStyle 0 solid; fore = fill), for each visible castle i:
   colour = own fief → `&HE0E0E0` RGB(224,224,224); allied team → `&HE0E0E0`; enemy → `&H404040`
   RGB(64,64,64). Box from `(sx(cx + w/4), sy(cy + h/4))` to `+8,+8` (`CarteTailleChateau = 8`).
4. Houses, i from UBound down to 0, `Maisons(i).Visible`: own → `&H8000&` RGB(0,128,0);
   allied → `&HFFC0C0` RGB(192,192,255); enemy → `&HC0C0FF` RGB(255,192,192).
   Box from `(sx(mx)+1, sy(my)+2)` to `+5,+5` (`CarteTailleMaison = 5`).
5. Characters, i from UBound down to 0, `Persos(i).Attaquable`: self or own summons →
   `&H8000&` RGB(0,128,0); allied → `&HC00000` RGB(0,0,192); enemy → `&HC0&` RGB(192,0,0);
   enemy in combat: with prob 0.5 if `attack < maxAttack`, then 50/50 `&HC0C0FF` RGB(255,192,192)
   or red → random flicker each frame. Filled circle centre `(sx(px)+2, sy(py)+2)`, radius 3.
6. Cross-hair through the player, colour `&H8000&` RGB(0,128,0) (1 px):
   horizontal `(Ox, sy(Perso.y)+2)`–`(Ox+CL, same)`, vertical `(sx(Perso.x)+2, Oy)`–`(same, Oy+CH)`.
7. Viewport rectangle (FillStyle 1 hollow, still green `&H8000&`):
   `(sx(camX), sy(camY))`–`(sx(camX+ScreenW), sy(camY+ScreenH))` where camX/camY = scroll offset
   (top-left of the view, `DecalageX/Y`) and ScreenW/H = 1024/768 (single player).
(Positions are top-left of entities; the +2 offsets approximately centre the dots.)

Click / hover (CI:2781-2851), active whenever the map is toggled on (`BoutonsInfos(6).Enabled`),
hit box `(Ox, Oy, CL, CH)` inclusive, world point `w = ((mx-Ox)/CL*WW, (my-Oy)/CH*WH)`:
- **Right button held** (continuous) → camera top-left := w (not centred), auto-scroll off.
- If an enemy obstacle is at w (`ObstacleEnnemi`): left click → attack it (army attacks if an
  army is selected); hover → tooltip `"&" & name & " "` (red).
- Else left click → move the player (or the selected army) to w (ghosts too).
- Returns NoBouton = 900 (cursor over interface). Closing the map re-enables auto-scrolling.

---------------------------------------------------------------------------------------------------

### 16. Window 7 — Objectifs (Fenetre-Defaut, CIF:1451-1478)

- Title "Objectifs" (128,32), close/move (close maps to Infos button 8).
- Objectives i = 1..Count (deduplicated strings added by scenario, CI:4949-4970) at
  **(32, 96 + 24*i)** → first at y=120, pitch 24. Every objective **except the last** is struck
  through: black line (32, y+8)–(292, y+8) (`ObjectifsFinisLongueur = 260`).
- Toggle via F11 / Infos button 8 only when `AfficherObjectifs` (set when the first objective is
  added). New objectives highlight Infos button 8 (row 2) until opened.

---------------------------------------------------------------------------------------------------

### 17. Hit testing helpers

- `Interieur_Bouton(x, y, bx, by, w, h)` = `x >= bx And x <= bx+w And y >= by And y <= by+h`
  (CI:4623-4633) — inclusive on both ends (w+1 x h+1 pixels).
- Button sizes for hit tests: close/move/eat/restore/service/fabricate/house-toggle/tab = 24x24
  (AffBoutonAction); commerce 24x24; transfer 24x24; scroll up/down 16x44; bar 16x198;
  emperor 24x24; competence sort 28x16; crafting icon 48x24; inventory slot 32x32;
  money zone 100x24; resistances zone 120x16; map (CL x CH).
- Clicks are edge-triggered (`CliqueGauche/Droit`) except the scroll bar and the map's
  right-button camera move, which use the held state.
- `Activer_EffetVisuel_Clique` spawns the click visual effect (`Parametres.EffetClique`) at the mouse
  on most button clicks.

---------------------------------------------------------------------------------------------------

### 18. Port checklist / gotchas

1. Blit window bmps with black colour key; content areas are 50 % checkerboards.
2. Caracs & Compétences: draw progress bars **before** the window bitmap.
3. Titles left-aligned at (128,32) (inventory at (32,32); caracs shows the character name).
4. Text top-left anchored; caps from Y+2 to Y+11; Arial 10 (bold per code), black; 16 px line step.
5. Windows 0 and 1 share the resource scroll offset; wheel scrolls all visible lists by one line.
6. Click on empty window background passes through to the world.
7. Hit-test precedence is fixed by code order, not by z-order.
8. Drag: outline top-left at cursor, cursor clamped to keep the outline on screen, left = drop,
   right = cancel; right-click "+" = reset to default.
9. Windows clamped to the screen each frame: x∈[0,704], y∈[0,400] at 1024x768.
10. No hover image states on window buttons; disabled/dead states are sprite rows.

## 6. Mouse and keyboard

### 6.1 Mouse device semantics (`ClsPerSouris.cls`)

* Fullscreen: DirectInput relative mouse; `SourisX += dx`, `SourisY += dy`, clamped to
  `[0, W] × [0, H]` (inclusive, `:147-155`); wheel `SourisZ += dz` (120 per notch, reset to
  0 after every rendered frame, `FrmMoteur2D.frm:1500`). Start position: screen centre
  (`Centrer_curseur`). Port: use the absolute OS mouse position.
* Per button b (0 = left, 1 = right, 2 = middle), evaluated once per loop iteration:
  * `Clique(b)` — true only on the **press edge** (pressed now, not pressed previous poll);
  * `Maintenu_Enfonce(b)` — pressed now **and** at the previous poll (held);
  * `Bouton_Relache(b)` — **release edge** (not pressed now, pressed previously).
* **Cursor hot spot = top-left of the 24×24 cursor image.** Hover hit-tests use
  `(SourisX + DecalageX, SourisY + DecalageY)` as the world point
  (`FrmMoteur2D.frm:1429`). Commands that target a location centre the character on the
  mouse point: target = `mouse + Decalage − (Largeur/2, Hauteur/2)` (`:860-861`).
* Keyboard emulation of the mouse (`ClsPerClavier.Positions/Clique`): numpad 4/6/8/2(5)
  (DIK 75/77/72/80/76) move the cursor with speed starting at 40 px/s and growing by
  7.5 px/s per poll up to 450 px/s; **numpad 0** (82) = left click, **numpad Enter** (156)
  = right click, **numpad .** (83) = middle click; numpad 9/7 (73/71) and 3/1 (81/79) act
  as the wheel (±5/s). Any numpad click switches input to "keyboard cursor" mode until a
  real mouse button is pressed (`FrmMoteur2D.frm:544-591`).

### 6.2 Mouse cursor — `ClsAffSouris.cls`, `Souris.ini`

`Images/Curseurs/ArrowCursor.bmp` is 432×24 = **18 frames of 24×24** (`LargeurImage=24`,
`NombreCurseurs=17`; types > 17 fall back to 0). The cursor is drawn with colour key at
`(SourisX, SourisY)` (top-left), frame `TypeCurseur`, every frame, above the HUD
(`FrmMoteur2D.frm:1471`).

`TypeCurseur` = what the mouse is over (computed every frame, `FrmMoteur2D.frm:1352-1474`):

| Type | Frame look | Meaning (left-click action) |
|---|---|---|
| 0 | golden arrow | nothing / plain ground → **move there** (or drop/sell carried object) |
| 1 | sack with green leaves | harvestable resource → go harvest (`Cmd_Perso_Chercher_Ressource`) |
| 2 | hammer, green square | placing a building, location OK |
| 3 | hammer, red square | placing a building, location blocked |
| 4 | hammer | own house not finished/damaged → go (re)build it; also allied house with Ctrl when damaged (repair) |
| 5 | key | own house built & full life → enter it (also own house with Ctrl) |
| 6 | crown + red arrow | own castle → go to castle (or go sell the carried object) |
| 7 | wooden door + arrow | allied house → go there |
| 8 | stone gate + arrow | allied castle → go there (or sell) |
| 9 | sword | enemy character → attack |
| 10 | sword | enemy house → attack |
| 11 | sword | enemy castle → attack |
| 12 | golden cross + arrow | allied temple (ghost only) → go there |
| 13 | bird/treasure icon | treasure → pick up |
| 14 | " | allied character/house while carrying an object/resources/money → give it |
| 15 | flame | over interface button 900 (special drag area) |
| 16 | golden arrow | decor (grave): same as 0, shows the grave's name |
| 17 | shield | army selected and hovering an ally → soldiers guard that ally |
| 999 | (→0) | inconstructible terrain (internally; displayed as 0) |

Hover detection = `Obstacle(NoPerso, mouseWorldX, mouseWorldY, 0, 0, True)`
(`ModIA.bas:2407-2578`) — priority order: **enemy characters (9) → treasures (13) →
castles (6 own / 8 allied / 11 enemy) → houses (5/4 own, 7 allied, 10 enemy) → resources
(1, 32×32 box) → decors (16) → inconstructible terrain (999)**. Point-in-rect test is
inclusive on both edges (`Est_Superpose`, `X2 ≤ x ≤ X2+L2`). Dead player: `ObstacleMort`
(only own castle 6, own temple 5, allied temple 12, named graves 16). While carrying an
object: `ObstacleAmi` → 14/6/8. Ctrl modifies 4→5 (own built house: enter without
repairing) and 7→4 (repair damaged allied house).

### 6.3 Mouse actions (main loop `FrmMoteur2D.frm:599-977`)

* **Left click on ground (type 0)**: on **release** of the left button after a press that
  started on the terrain: `Cmd_Perso_Deplacer` to `mouse + Decalage − (L/2, H/2)`.
  (No click marker is drawn on the ground. Clicking HUD/window buttons spawns the
  "Clique" effect #38 — 0 frames, so only its sound `032-Switch01.wav` plays —
  `ClsInterface.cls:937-944`, called from the button handlers at `:2603-2939`.)
* Left release on other types: see table above (`:828-916`).
* **Right click (release) on terrain**: if the player has soldiers, (re)select soldiers
  under the cursor (single click selects one unit). If no army is selected afterwards:
  carrying an object → drop the selection; else depending on the cursor:
  type 0 → open the **radial menu** of "move/orders" buttons (group 4) around the
  cursor; type 1 → radial menu of resource buttons (group 1); types 2/3 → cancel building
  placement; types 4–8 and 12 → radial building menu (group 0); types 9–11 →
  "Chargez!" (send your army to attack the target, with Ctrl/Shift/Alt variants)
  (`:924-963`).
* **Middle click**: attack the ground at the cursor (`Cmd_Perso_AttaquerTerrain`, with
  mode 1/2 when hovering a house/castle) (`:681-685`).
* **Drag-selection of soldiers** (only if the player is alive and has ≥1 soldier):
  holding left (or right) draws a 1-px rectangle from the press point to the cursor,
  **white** for left, **red RGB(255,0,0)** for right (`Afficher_CadreSelection_Soldats`,
  `:2186-2198`). The drag counts as a selection when `|dx| ≥ 16` or `|dy| ≥ 16`
  (`RectangleSelectionMinX/Y=16`). On release: `Selectionner_Soldats` selects own
  soldiers (`NumeroChef = player`, alive, not `Incommandable`) whose rectangle overlaps
  the drag rectangle; **Shift = add**, **Ctrl = remove**; the player himself is included
  if a left drag covers him. An "army" is selected when ≥1 soldier other than the player
  is selected; selected soldiers get `IA=False` (manual control).
* **With an army selected** (left-release on terrain): type 0/16 → move the soldiers
  (`Ordonner_Deplacement`), 9/10/11 → soldiers attack, 17 → soldiers guard the ally;
  4–8 → the player himself goes there. Right-release on terrain with an army: reselect;
  if nothing was hit, deselect the army and give it back to the AI.
* **Mouse wheel**: if the radial menu is open → rotate it; else, unless over a status bar,
  scroll the building/resources window (windows 0/1), skills (3) or bestiary (5) by
  `SourisZ` (see windows notes).
* **Building placement**: clicking a build button (NoBouton 1..99) sets cursor type 2;
  each frame the footprint `Batiment_Largeur × Batiment_Hauteur` centred on
  `mouse + 12` (cursor centre) is tested with `Obstacle(...)` and the map bounds; the
  rounded rectangle (radius 10) is drawn with **diagonal hatch** (`FillStyle 4`) and outline
  in **green `&HC000&` = RGB(0,192,0)** if OK or **red RGB(255,0,0)** if blocked
  (`Afficher_Surface_A_Batir`, `:2423-2449`). Left release places it
  (`Cmd_Perso_Creer_Maison` at `mouse − L/2 + 12 + Decalage`); without Ctrl/Shift the
  cursor then becomes type 4. Right click cancels.
* While a window is being dragged (`DeplacerFenetre ≥ 0`): left click drops it at the
  cursor, right click cancels (`:599-612`).

### 6.4 Terrain info-bubbles (hover tooltips) — `Afficher_Infos_Bulles_Terrain` (`:2510-2704`)

Shown next to the cursor when `AfficherInfoBulleTerrain=1` (box style: see HUD notes).
`#` = line break, `&` = switch to the "hostile" colour (see HUD notes for the markup).
* 0 while carrying an object: "sell" / "cannot sell" / "drop" / "cannot drop" labels.
* 1 resource: `"<Resource> : <int(have)> / <max>"`.
* 4/5 own house: `"<Building> #<Resource> : <have> / <max> "` for producers, or
  `"<Building> #<Object> : <pct>%"` while crafting, else `"<Building> "`.
* 7 allied house: name (+ `" (<owner of the summoner>)"` for summons) + `"#&<Fermé(e)>"`
  if closed/empty + `"#&<Abandonné(e)>"` if the owner is dead.
* 6 own castle: castle name of the people + epoch-change % + resurrection % + lives left.
* 8 allied castle / 11 enemy castle (`"&"` prefix): castle name.
* 9 enemy: `"&<Name>  "` followed by `"#&(<effect name>)"` per temporary effect.
* 10 enemy house: `"&<Building>  "` (+ abandoned).
* 12 temple, 13 treasure name, 14/17 ally name (or race name), 16 grave name.

### 6.5 Keyboard shortcuts (`Touches.ini`, `ClsIntTouches.Presser_Touche`)

Key codes are DirectInput scan codes. `Pression_Clavier` (`ClsPerClavier.cls:89-131`)
returns the **lowest-numbered** non-modifier, non-arrow key that is down, and only on the
frame it goes down (any held key blocks new ones). Arrows and Ctrl/Shift/Alt are read as
levels.

| Key | Action |
|---|---|
| Arrow keys (held) | **move the character**: every loop iteration the move target is set to `Pos ± (BonusDeplacement+1)` px (8 directions with two arrows), clamped to the map; the walk itself happens at tick rate (`ClsIntTouches.cls:302-345`) |
| Esc | quit confirmation dialog |
| B / R / P / T / I / U / Tab | toggle window Building(0) / Resources(1) / Character(2) / Skills(3) / Inventory(4) / Bestiary(5) / Map(6) |
| F11 | toggle Objectives window (if available) |
| F10 | menu = toggle **pause** (info button 7) |
| Space | close all windows + clear messages |
| Enter | context action on what the character stands on (enter house, castle, become emperor, use service, harvest, attack…) |
| G / M | go back to castle / house (Ctrl+M: enter without repairing) |
| F1…F8 | find nearest resource of type 1…8 |
| W / X / C | find & attack nearest enemy character / house / castle (army does it if selected) |
| V | find treasure |
| A | toggle auto mode (AI plays the character) |
| H | "call allies to attack" confirmation dialog |
| S | stop (Ctrl/Shift/Alt+S: stop whole army) |
| D Z L Q K | set attitude 1..5 |
| F | formation |
| PgUp / PgDn | deposit / take all money in own house |
| Del | demolish own house |
| F9 | show a hint |
| F12 | save world |
| Backspace | toggle full-screen interface (hide HUD) |
| ² (41) | switch to next controlled character (Shift: previous) |
| 1…9 | equip object shortcut 1…9 (unless the cursor is over an object) |
| `<` (86) | open chat/dialogue line |
| Ctrl+A/Z/W/E/R/S/Q/X/C/V/D/F/I | army selection: all / same race / armed / on screen / rested / idle / fast / unassigned / ground / flying / defensive / offensive / invert |
| Ctrl+Del | kill selected soldiers |
| Alt+F4 | exit immediately |

While a confirmation box is shown only its validate/cancel keys work
(`TouchesDialogue.ini`); while the chat line is open, letters are typed (auto-repeat
0.16 s).

## 7. Sounds and music

### 7.1 Sound effects — `ClsPerSon.cls`

* **Every sound effect belongs to a visual effect**: `Parametres_Effets.ini [Effet_n] Son=`
  (files in `Vendetta/Sons/`). One DirectSound buffer per effect type
  (`ClsPerSon.cls:57-79`). `Sons.ini Jouer=1`.
* A sound is played when an effect is **displayed at frame index 1** and its rectangle
  intersects the screen (`FrmMoteur2D.frm:2057-2063, 2076-2082, 2096-2102`), not while
  paused. Because it is called every rendered frame during that tick, and DirectSound
  `Play` on an already-playing buffer does nothing, the result is: the sound starts once
  when the effect reaches frame 1 (it restarts if the WAV is shorter than one tick and still
  on frame 1). Only one instance per effect type can play at a time (same buffer).
* Interface effects (`ClsInterface.cls:992-1015`): same rule; effects with 0 frames (e.g.
  #38 "Clique" → `032-Switch01.wav`, #45 "Miaulement" → `067-Animal02.wav`) play their sound
  immediately once and are removed.
* Effect → sound list (index = INI section number):
  1 Coup simple `089-Attack01`, 2 Lame `094-Attack06`, 3 Flamerole `117-Fire01`,
  4 Mur de feu (none), 5 Résurrection `116-Raise02`, 6 Jet de flammes `118-Fire02`,
  7 Coup de griffes `093-Attack05`, 8 Explosion `Explosion1`, 9 Pointe `101-Attack13`,
  10 Gain d'argent (none), 11 Brasier `119-Fire03`, 12 Choc `099-Attack11`,
  13 Sphère noire `174-Skill18`, 14 Tornade `132-Wind01`, 15 Gaz `140-Darkness03`,
  16 Coup de hache `098-Attack10`, 17 Guérison `105-Heal01`, 18 Pierres `129-Earth01`,
  19 Explosion 2 `053-Cannon02`, 20 Grands coups de griffes `163-Skill07`,
  21 Crâne `079-Monster01`, 22 Colonne de feu `142-Burst02`, 23 Etincelles vertes
  `136-Light02`, 24 Destruction `051-Explosion04`, 25 Cercle bleu `Cold5`, 26 Dévorer
  `Bite`, 27 Explosion 3 `Explosion7`, 28 Dépense `013-Move01`, 29 Eclair `123-Thunder01`,
  30 Tir `102-Attack14`, 31 Flamme `141-Burst01`, 32 Célérité `018-Teleport01`,
  33 Bonus `015-Jump01`, 34 Outil `162-Skill06`, 35/36 Sphère `108-Heal04`,
  37 Explosion 4 `054-Cannon03`, 38 Clique `032-Switch01`, 39 Courrier `055-Right01`,
  40 Glace `120-Ice01`, 41 Lame pure `095-Attack07`, 42 Génération `114-Remedy02`,
  43 Musique `059-Applause01`, 44 Horloge rouge (none), 45 Miaulement `067-Animal02`,
  46 Ténèbres `139-Darkness02`, 47 Horloge blanche (none).
  Special effect roles (`[Effets]`): click = 38, house on fire = 4, house destroyed = 24,
  castle destroyed = 24, new objective = 39, remaining-time clocks = 44 / 47.
  Which game event triggers which effect is decided by the game rules (attacks use the
  weapon/skill's effect); see the combat spec.

### 7.2 Music — `ClsPerMusique.cls`, `Musiques.ini`

* MCI playback (`UtiliserDirectMusic=0`) of a **random file** (.mp3/.mid/.wav) from the
  category folder `Vendetta/Musiques/<Folder>/`. Categories: 0 Menu, 1 Debut, 2 Ville,
  3 Combat, 4 Mort, 5 Victoire, 6 Defaite, 7 Boss, 8 Empereur.
* `Jouer_Musique(cat, facultative)`: starts a new random track if the category has files
  and (`Timer > start + IntervalMusiques (256 s)` **or** (`cat ≠ current` and not
  facultative)). Tracks are not looped; a new random track of the current category starts
  256 s after the previous start. Combat does not interrupt a Boss track.
* Game start: category 1 "Debut" (`FrmMoteur2D.frm:405`). Then **every loop iteration**
  (`:426-438`): player in combat → 7 Boss if `Action = 302`, else 3 Combat (non-facultative
  unless victory/defeat); dead → 4 Mort; emperor → 8 (facultative); else 2 Ville
  (facultative — so the Debut track keeps playing until it would be replaced after 256 s).
  Victory → 5 (non-facultative), defeat → 6 (`:1152, 1162`).
* Ctrl+M toggles music if `ToucheMusique` is bound (−1 by default = unbound).

## 8. Unresolved points / port decisions

* **Fill of rectangles**: the DirectDraw `SetFillStyle`/`SetFillColor` interplay is not
  fully explained by the code (status bars and tooltip boxes are drawn after
  `SetFillStyle 1` = transparent, yet they appear filled in the screenshot). Working rule
  matching the screenshot: status bars and tooltip/info-bubble boxes are **filled**;
  selection ellipses, the drag rectangle, window drag outline, minimap viewport and
  inventory hover frames are **hollow**; the building footprint is **diagonal-hatched**.
* **Font weight**: code requests Arial 10 pt bold; the window text in the Boxedwine
  screenshot looks thinner (antialiased) than the bold white FPS text. Compare against the
  screenshot when choosing the bitmap font.
* **Empty part of a status bar** is assumed black (all bars in the screenshot are full).
* Original quirks worth keeping for fidelity (decide per case): clicks on the empty area of
  a window fall through to the map; window click priority is a fixed order, not z-order;
  camera updates only at tick rate (10 Hz); characters are not Y-sorted (reverse index
  order); `CLng` rounding in `TerrainIndice`; the mount Y offset uses the X offset in
  `ClsAffApparence`.
