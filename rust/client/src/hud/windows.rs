//! The eight 320×368 windows (`ClsIntFenetre`), positions relative to the window origin.

use super::object_tooltip::{InInventory, object_tooltip};
use super::{Hud, Input, outline};
use crate::assets::Textures;
use crate::layout::{ax, ay, screen_h, screen_w};
use macroquad::prelude::*;
use sim::files::Files;
use sim::ini::Ini;
use sim::perso::action;
use sim::rules::Rules;
use sim::world::{Order, World};

pub const W: i32 = 320;
pub const H: i32 = 368;
const ACTION: &str = "Images/Boutons/Action.bmp";
const COMMERCE: &str = "Images/Boutons/Commerce.bmp";
const TRANSFERT: &str = "Images/Boutons/Transfert.bmp";
const DEFILEMENT: &str = "Images/Boutons/Defilement.bmp";
const COMPETENCES: &str = "Images/Boutons/Competences.bmp";
const DEFAULT_POS: [(i32, i32); 8] = [(704, 0), (704, 368), (64, 0), (64, 368), (384, 0), (384, 368), (64, 0), (64, 0)];
/// Draw order (later on top).
const DRAW_ORDER: [usize; 8] = [4, 0, 1, 2, 3, 5, 6, 7];
const ROWS: usize = 11;

/// `house`: the inventory window shows the house's inventory.
fn background(i: usize, female: bool, house: bool) -> String {
    let name = match i {
        2 => "Fenetre-Personnage",
        3 => "Fenetre-Competences",
        4 if house && female => "Fenetre-InventaireFM",
        4 if house => "Fenetre-InventaireHM",
        4 if female => "Fenetre-InventaireF",
        4 => "Fenetre-InventaireH",
        6 => "Fenetre-Carte",
        _ => "Fenetre-Defaut",
    };
    format!("Images/Fenetres/{name}.bmp")
}

pub fn images(rules: &Rules) -> Vec<String> {
    let mut v: Vec<String> = [ACTION, COMMERCE, TRANSFERT, DEFILEMENT, COMPETENCES].map(String::from).to_vec();
    v.extend((0..8).flat_map(|i| [false, true].map(|f| [background(i, f, false), background(i, f, true)])).flatten());
    v.extend(rules.resources.iter().map(|r| resource_icon(&r.image)));
    // every object's icon: inventories can hold any of them (about 1 MB)
    v.extend((0..rules.objects.len()).map(|o| object_icon(rules, o)));
    v
}

fn object_icon(rules: &Rules, o: usize) -> String {
    format!("{}.bmp", rules.objects[o].image)
}

fn resource_icon(image: &str) -> String {
    format!("Images/Ressources/{image}.bmp")
}

/// Window texts (`Donnees/<lang>/Fenetres.ini`, `Interface.ini`).
pub struct WindowStrings {
    titles: [String; 9],
    close: String,
    move_: String,
    restore: [String; 3],
    total: [String; 6],
    deposit_money: [String; 3],
    take_money: [String; 3],
    deposit_stock: [String; 3],
    take_stock: [String; 3],
    empty: String,
    unavailable: String,
    objects: String,
    craft: String,
    scroll: [String; 2],
    price: String,
    money: String,
    /// The house toggle's tooltips: to the house, to the bag; with an object held, store it in
    /// the house, take it into the bag.
    house_toggle: [String; 4],
    /// `EtiquetteNivPlus`, the skills window's progress column.
    niv_plus: String,
    /// The skills window's sort buttons: none, alphabetical, by score, show all.
    skill_sorts: [String; 4],
}

impl WindowStrings {
    pub fn load(files: &Files, lang: &str) -> WindowStrings {
        let ini = |name: &str| Ini::parse(files.text(&format!("Donnees/{lang}/{name}")).unwrap_or(""));
        let (f, i, g) = (ini("Fenetres.ini"), ini("Interface.ini"), ini("Parametres_Globales.ini"));
        let s = |ini: &Ini, k: &str| ini.find(k).unwrap_or("").to_owned();
        let three = |prefix: &str, suffix: &str| std::array::from_fn(|n| s(&f, &format!("{prefix}{}{suffix}", n + 1)));
        let money = s(&g, "EtiquetteArgent");
        let mut chars = money.chars();
        WindowStrings {
            titles: [
                "TitreFenetreAchat",
                "TitreFenetreRessources",
                "TitreFenetreCaracteristiques",
                "TitreFenetreCompetences",
                "TitreFenetreInventaire",
                "TitreFenetreBestiaire",
                "TitreFenetreCarte",
                "TitreFenetreObjectifs",
                "TitreFenetreVente",
            ]
            .map(|k| s(&i, k)),
            close: s(&f, "BoutonFermerEtiquette"),
            move_: s(&f, "BoutonDeplacerEtiquette"),
            restore: three("BoutonsRestaurationEtiquette", ""),
            total: std::array::from_fn(|n| s(&f, &format!("BoutonsRessourcesTotalEtiquette{}", n + 1))),
            deposit_money: three("BoutonTransferArgentPerso", "Etiquette"),
            take_money: three("BoutonTransferArgentMaison", "Etiquette"),
            deposit_stock: three("BoutonTransferStockPerso", "Etiquette"),
            take_stock: three("BoutonTransferStockMaison", "Etiquette"),
            empty: s(&f, "EtiquetteStockVide"),
            unavailable: s(&f, "EtiquetteIndisponible"),
            objects: s(&f, "EtiquetteObjets"),
            craft: s(&f, "BoutonFabriquerEtiquette"),
            scroll: [s(&f, "BoutonDefilement1Etiquette"), s(&f, "BoutonDefilement2Etiquette")],
            price: "Prix".to_owned(),
            house_toggle: [
                "InventaireMaisonMessagePerso",
                "InventaireMaisonMessageMaison",
                "InventaireMaisonMessagePersoObjet",
                "InventaireMaisonMessageMaisonObjet",
            ]
            .map(|k| s(&f, k)),
            niv_plus: s(&g, "EtiquetteNivPlus"),
            skill_sorts: std::array::from_fn(|n| s(&f, &format!("BoutonCompetence{}Etiquette", n + 1))),
            // "Argent" from `EtiquetteArgent`, first letter upper-cased
            money: chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default(),
        }
    }
}

pub struct Windows {
    pub strings: WindowStrings,
    pub pos: [(i32, i32); 8],
    /// Moved by the player; other windows follow the screen edges.
    moved: [bool; 8],
    /// Window being dragged (`DeplacerFenetre`).
    dragging: Option<usize>,
    /// Shared scroll offset of the resource lists (windows 0 and 1).
    scroll: usize,
    /// Scroll offset of the crafting list (`DefilementObjetDecalageX`).
    craft_scroll: usize,
    /// `Bascule`: the inventory window shows the house's inventory (in the own house only).
    house_view: bool,
    /// Scroll offsets of the skills and bestiary windows.
    skill_scroll: usize,
    bestiary_scroll: usize,
    /// `CompetencesTri` (0 none, 1 alphabetical, 2 by score) and `CompetencesAfficherToutes`.
    skill_sort: usize,
    skill_all: bool,
    /// The skills table, with the tick and sort it was made for.
    skill_cache: std::cell::RefCell<Option<((u64, usize, bool), std::rc::Rc<Vec<SkillRow>>)>>,
}

/// Opaque blit (black drawn) or colour-keyed blit of a sheet cell.
fn cell(textures: &Textures, sheet: &str, x: i32, y: i32, col: i32, row: i32, size: (i32, i32), opaque: bool) {
    let Some(tex) = textures.get(sheet) else { return };
    if opaque {
        draw_rectangle(x as f32, y as f32, size.0 as f32, size.1 as f32, BLACK);
    }
    let src = Rect::new((col * size.0) as f32, (row * size.1) as f32, size.0 as f32, size.1 as f32);
    tex.draw_ex(x as f32, y as f32, WHITE, DrawTextureParams { source: Some(src), ..Default::default() });
}

/// How many objects one click queues or takes: 1, 5 with Ctrl, 10 with Shift, 20 with both.
fn queue_quantity(ctrl: bool, shift: bool) -> usize {
    match (ctrl, shift) {
        (true, true) => 20,
        (false, true) => 10,
        (true, false) => 5,
        (false, false) => 1,
    }
}

fn inside(m: (i32, i32), x: i32, y: i32, w: i32, h: i32) -> bool {
    m.0 >= x && m.0 <= x + w && m.1 >= y && m.1 <= y + h
}

/// A scrolling list, each with its own offset.
#[derive(Clone, Copy)]
enum List {
    /// The resource lists of windows 0 and 1, which share theirs.
    Resources,
    Crafting,
    Skills,
    Bestiary,
}

/// Rows shown by the skills and bestiary windows (`CompetencesNombreLignes + 1`).
const SKILL_ROWS: usize = 16;
/// The skills window's sort buttons: x in the window (y 61), 28×16 cells of `Competences.bmp`.
const SKILL_SORT_X: [i32; 4] = [43, 75, 106, 146];

/// A line of the skills window (`ClsJeuCompetences`).
struct SkillRow {
    name: String,
    value: i64,
    /// Progress toward the next level, 0 to 1 (`NivPlus`).
    progress: f64,
    /// In use: the name in dark red.
    active: bool,
    /// Raised by an item: the value in dark red.
    bonus: bool,
}

/// What a window button does, collected while drawing and applied by `interact`.
struct Hit {
    tip: String,
    left: Option<Order>,
    right: Option<Order>,
    /// Left click queues this crafting slot, right click unqueues it (with Ctrl/Shift).
    queue: Option<usize>,
    /// Hovering draws an orange frame around it (inventory slots).
    frame: bool,
    /// The object shown there, which a digit key binds while hovered.
    object: Option<usize>,
    /// Clicking switches the inventory window between the bag and the house's inventory.
    toggle: bool,
}

impl Windows {
    pub fn new(strings: WindowStrings) -> Windows {
        Windows { strings, pos: DEFAULT_POS, moved: [false; 8], dragging: None, scroll: 0, craft_scroll: 0, house_view: false, skill_scroll: 0, bestiary_scroll: 0, skill_sort: 1, skill_all: false, skill_cache: Default::default() }
    }
}

impl Hud {
    /// The list window `i` scrolls, if any, and its last offset.
    fn scroll_list(&self, world: &World, rules: &Rules, player: usize, i: usize) -> Option<(List, usize)> {
        let p = &world.persos[player];
        match i {
            0 if p.in_house || p.in_own_house => {
                let h = if p.in_own_house { player } else { p.house_target };
                let b = &rules.buildings[world.houses[h].kind];
                if !b.market {
                    // a house: only a workshop with more than 8 objects scrolls
                    return (b.factory && b.objects.len() > 8).then(|| (List::Crafting, b.objects.len() - 8));
                }
                Some((List::Resources, rules.resources.len().saturating_sub(ROWS)))
            }
            0 | 1 => Some((List::Resources, rules.resources.len().saturating_sub(ROWS))),
            // only with more than 16 skills
            3 => {
                let n = self.skill_rows(world, rules, player).len();
                (n > SKILL_ROWS).then(|| (List::Skills, n - SKILL_ROWS))
            }
            // always, over every race
            5 => Some((List::Bestiary, rules.races.len().saturating_sub(SKILL_ROWS))),
            _ => None,
        }
    }

    fn scroll_offset(&mut self, list: List) -> &mut usize {
        match list {
            List::Resources => &mut self.windows.scroll,
            List::Crafting => &mut self.windows.craft_scroll,
            List::Skills => &mut self.windows.skill_scroll,
            List::Bestiary => &mut self.windows.bestiary_scroll,
        }
    }

    /// Whether the mouse is over an open window, and whether that window scrolls.
    pub fn over_window(&self, world: &World, rules: &Rules, player: usize, m: (i32, i32)) -> Option<bool> {
        (0..8)
            .filter(|&i| self.window_visible(world, player, i))
            .find(|&i| inside(m, self.windows.pos[i].0, self.windows.pos[i].1, W, H))
            .map(|i| self.scroll_list(world, rules, player, i).is_some())
    }

    fn window_visible(&self, world: &World, player: usize, i: usize) -> bool {
        let p = &world.persos[player];
        match i {
            // Only inside a building (`Afficher_Fenetres`).
            0 => self.info_enabled[0] && ((p.alive() && (p.in_house || p.in_own_house)) || p.in_castle || p.in_own_castle),
            7 => self.info_enabled[8],
            _ => self.info_enabled[i],
        }
    }

    /// Draws the open windows and handles their buttons. Returns whether a button was hit.
    pub(super) fn windows(&mut self, world: &World, rules: &Rules, textures: &Textures, player: usize, input: &Input, orders: &mut Vec<Order>, tip: &mut String) -> bool {
        self.hovered_object = None;
        // `RePositionner_Fenetres`: default positions follow the screen edges; all are clamped.
        for i in 0..8 {
            let (x, y) = if self.windows.moved[i] { self.windows.pos[i] } else { (ax(DEFAULT_POS[i].0), ay(DEFAULT_POS[i].1)) };
            self.windows.pos[i] = (x.clamp(0, (screen_w() - W).max(0)), y.clamp(0, (screen_h() - H).max(0)));
        }
        let mut hits: Vec<(Rect, Hit)> = Vec::new();
        for i in DRAW_ORDER {
            if !self.window_visible(world, player, i) {
                continue;
            }
            let (x, y) = self.windows.pos[i];
            let female = world.persos[player].female;
            let house = i == 4 && self.house_view(world, player);
            if i == 3 {
                // the skills' progress bars show through the background
                self.skill_bars(world, rules, player, (x, y));
            }
            if let Some(tex) = textures.get(&background(i, female, house)) {
                tex.draw(x as f32, y as f32, WHITE);
            }
            match i {
                0 => self.building_window(world, rules, textures, player, (x, y), &mut hits),
                1 => self.resources_window(world, rules, textures, player, (x, y), &mut hits),
                3 => self.skills_window(world, rules, textures, player, (x, y)),
                4 => self.inventory_window(world, rules, textures, player, (x, y), &mut hits),
                5 => self.bestiary_window(world, rules, textures, player, (x, y)),
                _ => {
                    // TODO: windows 2, 6 and 7 contents
                    let title = &self.windows.strings.titles[i];
                    let tx = if i == 4 { 32 } else { 128 };
                    self.text(x + tx, y + 32, title, BLACK);
                }
            }
            // Close and move buttons, opaque.
            cell(textures, ACTION, x + 280, y + 16, 6, 0, (24, 24), true);
            cell(textures, ACTION, x + 256, y + 16, 8, 0, (24, 24), true);
        }

        let m = input.mouse;
        if let Some(i) = self.windows.dragging {
            // `Position_Contour_Fenetre`: white outline at the (clamped) cursor.
            let (cx, cy) = (m.0.min(screen_w() - W), m.1.min(screen_h() - H));
            outline(cx, cy, cx + W, cy + H, WHITE);
            if input.left_pressed {
                self.windows.pos[i] = (cx, cy);
                self.windows.moved[i] = true;
                self.windows.dragging = None;
            } else if input.right_pressed {
                self.windows.dragging = None;
            }
            return true;
        }

        // The wheel scrolls the list of the window under the cursor, whatever button it is on.
        let wheel = mouse_wheel().1;
        if wheel != 0.0 {
            // topmost first
            for &i in DRAW_ORDER.iter().rev() {
                let (x, y) = self.windows.pos[i];
                if !self.window_visible(world, player, i) || !inside(m, x, y, W, H) {
                    continue;
                }
                if let Some((list, n)) = self.scroll_list(world, rules, player, i) {
                    let offset = self.scroll_offset(list);
                    *offset = if wheel > 0.0 { offset.saturating_sub(1) } else { (*offset + 1).min(n) };
                }
                break;
            }
        }

        // Window buttons, in the original's precedence: contents, then close/move.
        for (r, hit) in &hits {
            if inside(m, r.x as i32, r.y as i32, r.w as i32, r.h as i32) {
                *tip = hit.tip.clone();
                self.hovered_object = hit.object;
                if hit.frame {
                    let (x, y) = (r.x as i32, r.y as i32);
                    outline(x, y, x + 32, y + 32, Color::from_rgba(255, 128, 0, 255));
                }
                if let Some(slot) = hit.queue {
                    if input.left_pressed || input.right_pressed {
                        self.change_craft_queue(player, slot, input.left_pressed, input, orders);
                    }
                } else if hit.toggle && (input.left_pressed || input.right_pressed) {
                    self.windows.house_view = !self.windows.house_view;
                } else if input.left_pressed {
                    orders.extend(hit.left.clone());
                } else if input.right_pressed {
                    orders.extend(hit.right.clone());
                }
                return true;
            }
        }
        for i in 0..8 {
            if !self.window_visible(world, player, i) {
                continue;
            }
            let (x, y) = self.windows.pos[i];
            if inside(m, x + 280, y + 16, 24, 24) {
                *tip = self.windows.strings.close.clone();
                if input.left_pressed {
                    let b = if i == 7 { 8 } else { i };
                    self.info_enabled[b] = !self.info_enabled[b];
                }
                return true;
            }
            if inside(m, x + 256, y + 16, 24, 24) {
                *tip = self.windows.strings.move_.clone();
                if input.left_pressed {
                    self.windows.dragging = Some(i);
                } else if input.right_pressed {
                    self.windows.moved[i] = false;
                }
                return true;
            }
        }
        // The skills window's sort buttons (`CI:3481-3515`): sort, or show all; back to the top.
        if self.window_visible(world, player, 3) {
            let (x, y) = self.windows.pos[3];
            for (k, bx) in SKILL_SORT_X.into_iter().enumerate() {
                if inside(m, x + bx, y + 61, 28, 16) {
                    *tip = self.windows.strings.skill_sorts[k].clone();
                    if input.left_pressed || input.right_pressed {
                        if k < 3 {
                            self.windows.skill_sort = k;
                        } else {
                            self.windows.skill_all = !self.windows.skill_all;
                        }
                        self.windows.skill_scroll = 0;
                    }
                    return true;
                }
            }
        }
        // Scroll bars: the resource lists of windows 0/1 (shared), a workshop's crafting list,
        // the skills and the bestiary.
        for i in [0, 1, 3, 5] {
            if !self.window_visible(world, player, i) {
                continue;
            }
            let Some((list, n)) = self.scroll_list(world, rules, player, i) else { continue };
            let (x, y) = self.windows.pos[i];
            let strings = &self.windows.strings.scroll;
            let (up, down) = (strings[0].clone(), strings[1].clone());
            let offset = self.scroll_offset(list);
            if inside(m, x + 300, y + 66, 16, 44) {
                *tip = up;
                if input.left_pressed {
                    *offset = offset.saturating_sub(1);
                }
                return true;
            }
            if inside(m, x + 300, y + 308, 16, 44) {
                *tip = down;
                if input.left_pressed {
                    *offset = (*offset + 1).min(n);
                }
                return true;
            }
            if inside(m, x + 300, y + 110, 16, 198) && is_mouse_button_down(MouseButton::Left) {
                let v = (n as f64 * (m.1 - (y + 110)) as f64 / 198.0 + 0.5).floor();
                *offset = (v.max(0.0) as usize).min(n);
                return true;
            }
        }
        // Clicks on empty window areas go through to the map, like the original.
        false
    }

    fn title(&self, origin: (i32, i32), title: &str) {
        self.text(origin.0 + 128, origin.1 + 32, title, BLACK);
    }

    /// `Afficher_BoutonsDefilement` for the resource lists.
    fn scroll_bar(&self, textures: &Textures, origin: (i32, i32), n: usize, offset: usize) {
        let (x, y) = origin;
        cell(textures, DEFILEMENT, x + 300, y + 66, 0, 0, (16, 44), false);
        cell(textures, DEFILEMENT, x + 300, y + 308, 1, 0, (16, 44), false);
        cell(textures, DEFILEMENT, x + 300, y + 110, 2, 0, (16, 198), false);
        let thumb = if n > 0 { 110 + 182 * offset as i32 / n as i32 } else { 110 };
        cell(textures, DEFILEMENT, x + 300, y + thumb, 3, 0, (16, 16), false);
    }

    fn separator(origin: (i32, i32), y: i32) {
        let yy = (origin.1 + y + 18) as f32 + 0.5;
        draw_line((origin.0 + 32) as f32, yy, (origin.0 + 248) as f32, yy, 1.0, BLACK);
    }

    /// Window 1 — Ressources (`Afficher_InfosRessourcesPerso`).
    fn resources_window(&self, world: &World, rules: &Rules, textures: &Textures, player: usize, o: (i32, i32), hits: &mut Vec<(Rect, Hit)>) {
        let s = &self.windows.strings;
        let p = &world.persos[player];
        let perso = player;
        let in_castle = p.in_castle || p.in_own_castle;
        self.title(o, if in_castle { &s.titles[8] } else { &s.titles[1] });
        let dead = !p.alive() as i32;
        let rect = |x: i32, y: i32, w: i32, h: i32| Rect::new((o.0 + x) as f32, (o.1 + y) as f32, w as f32, h as f32);

        // Global buttons: "Tout", "I&C", ">20"
        for j in 0..3 {
            cell(textures, COMMERCE, o.0 + 216 + 24 * j, o.1 + 48, 6 + j, dead, (24, 24), true);
            let label = &s.total[(2 - j) as usize + if in_castle { 0 } else { 3 }];
            // TODO: global sell/take orders
            hits.push((rect(216 + 24 * j, 48, 24, 24), Hit { toggle: false, object: None, frame: false, queue: None, tip: label.clone(), left: None, right: None }));
        }
        let own_house = p.in_own_house && world.houses[player].visible();
        let product = if own_house { rules.buildings[world.houses[player].kind].product } else { -1 };
        for r in 0..ROWS {
            let k = r + self.windows.scroll;
            let Some(res) = rules.resources.get(k) else { break };
            let y = 88 + 24 * r as i32;
            let label = if in_castle { format!("{} : {}", s.price, res.price) } else { format!("{} :", res.name) };
            self.text(o.0 + 32, o.1 + y, &label, BLACK);
            let max = p.max_resource(k);
            let sep = if max < 100.0 { " / " } else { "/" };
            self.text(o.0 + 136, o.1 + y, &format!("{}{sep}{}", p.resource(k).floor(), max.floor()), BLACK);
            Self::separator(o, y);
            cell(textures, &resource_icon(&res.image), o.0 + 104, o.1 + y - 10, 0, 0, (32, 32), false);
            if res.edible {
                cell(textures, ACTION, o.0 + 184, o.1 + y - 6, 0, dead, (24, 24), true);
                let gains: String = ["Vie", "Energie", "Magie", "Moral"]
                    .iter()
                    .zip(res.gain)
                    .filter(|(_, g)| *g > 0.0)
                    .map(|(n, g)| format!("#+{g:.1} {n}"))
                    .collect();
                let tip = format!("Manger : {}  {gains}", res.name.to_lowercase());
                let eat = Order::Eat { perso, resource: k };
                hits.push((rect(184, y - 6, 24, 24), Hit { toggle: false, object: None, frame: false, queue: None, tip, left: Some(eat), right: None }));
            }
            let quantities = [1.0, 5.0, p.resource(k).floor()];
            if own_house && product == k as i64 && world.houses[player].max_stock > 0.0 {
                for j in 0..3 {
                    cell(textures, TRANSFERT, o.0 + 216 + 24 * j, o.1 + y - 6, 3 + j, 0, (24, 24), true);
                    let deposit = Order::DepositStock { perso, quantity: quantities[j as usize], all: j == 2 };
                    let tip = s.deposit_stock[j as usize].clone();
                    hits.push((rect(216 + 24 * j, y - 6, 24, 24), Hit { toggle: false, object: None, frame: false, queue: None, tip, left: Some(deposit), right: None }));
                }
                continue;
            }
            for j in 0..3 {
                let col = j + if in_castle { 3 } else { 0 };
                cell(textures, COMMERCE, o.0 + 216 + 24 * j, o.1 + y - 6, col, dead, (24, 24), true);
                let q = quantities[j as usize];
                let hit = if in_castle {
                    let order = Order::SellResource { perso, resource: k, quantity: q };
                    Hit { toggle: false, object: None, frame: false, queue: None,
                        tip: format!("Vendre {q} {}  #{} : {}", res.name, s.price, q * res.price),
                        left: Some(order.clone()),
                        right: Some(order),
                    }
                } else {
                    Hit { toggle: false, object: None, frame: false, queue: None,
                        tip: format!("Prendre {q} {}  #{} : {}", res.name, s.price, q * res.price),
                        left: Some(Order::SelectResource { perso, resource: k, quantity: q }),
                        right: Some(Order::UnselectResource { perso, resource: k, quantity: q }),
                    }
                };
                hits.push((rect(216 + 24 * j, y - 6, 24, 24), hit));
            }
        }
        self.scroll_bar(textures, o, rules.resources.len().saturating_sub(ROWS), self.windows.scroll);

        // Money line, with deposit buttons in the own house.
        self.text(o.0 + 32, o.1 + 54, &format!("{} : {}", s.money, p.money().floor()), BLACK);
        if own_house {
            for j in 0..3 {
                cell(textures, TRANSFERT, o.0 + 216 + 24 * j, o.1 + 48, 3 + j, 0, (24, 24), true);
                let q = [1.0, 5.0, p.money().floor()][j as usize];
                let tip = s.deposit_money[j as usize].clone();
                hits.insert(0, (rect(216 + 24 * j, 48, 24, 24), Hit { toggle: false, object: None, frame: false, queue: None, tip, left: Some(Order::DepositMoney { perso, quantity: q }), right: None }));
            }
        }
    }

    /// The inventory window shows the house's inventory (`Bascule`, alive in the own house).
    fn house_view(&self, world: &World, player: usize) -> bool {
        let p = &world.persos[player];
        self.windows.house_view && p.in_own_house && p.alive()
    }

    /// Window 4 — Inventaire (`Afficher_InfosInventaire`): money, equipment and the 32-slot bag.
    fn inventory_window(&self, world: &World, rules: &Rules, textures: &Textures, player: usize, o: (i32, i32), hits: &mut Vec<(Rect, Hit)>) {
        // Equipment slots by index: hands, armour, head, feet, back, rings, arms, orb, mount.
        const EQUIPMENT: [(i32, i32); 11] =
            [(38, 124), (238, 124), (137, 88), (137, 39), (137, 162), (187, 88), (38, 162), (238, 162), (88, 104), (38, 72), (187, 162)];
        let s = &self.windows.strings;
        let p = &world.persos[player];
        let perso = player;
        self.text(o.0 + 32, o.1 + 32, &s.titles[4], BLACK);
        self.text(o.0 + 32, o.1 + 54, &format!("{} : {}", s.money, p.money().floor()), BLACK);
        let icon = |obj: usize, x: i32, y: i32| {
            if let Some(tex) = textures.get(&object_icon(rules, obj)) {
                let src = Rect::new(0.0, 0.0, 32.0, 32.0);
                tex.draw_ex(x as f32, y as f32, WHITE, DrawTextureParams { source: Some(src), ..Default::default() });
            }
        };
        let held = p.selected_object >= 0;
        let set = super::shortcuts::current_set();
        for (k, &(ex, ey)) in EQUIPMENT.iter().enumerate() {
            let (x, y) = (o.0 + ex, o.1 + ey);
            let worn = p.equipped.get(k).copied().flatten();
            if let Some(item) = worn {
                icon(item.object, x, y);
            } else if k == 1 && p.two_handed(rules) {
                // a two-handed weapon also shows, translucent, in the second hand
                if let Some(Some(item)) = p.equipped.first() {
                    if let Some(tex) = textures.get(&object_icon(rules, item.object)) {
                        let src = Rect::new(0.0, 0.0, 32.0, 32.0);
                        let colour = Color::new(1.0, 1.0, 1.0, 0.5);
                        tex.draw_ex(x as f32, y as f32, colour, DrawTextureParams { source: Some(src), ..Default::default() });
                    }
                }
            }
            // Left: drop the held object here (equipping it; slots 1 and 7 are the right
            // side), or take the worn one onto the cursor. Right: put it back in the bag.
            let right_side = k == 1 || k == 7;
            let left = if held {
                Some(Order::EquipHeld { perso, right: right_side })
            } else {
                worn.map(|_| Order::HoldEquipped { perso, slot: k })
            };
            let tip = worn
                .map(|it| object_tooltip(rules, &self.tooltip_labels, p, it.object, 0, None, Some(InInventory::of(world, player))))
                .unwrap_or_default();
            let store = worn.map(|_| Order::StoreEquipped { perso, slot: k });
            if let Some(label) = worn.and_then(|it| self.shortcuts.label(it.object, set)) {
                self.small_text(x, y, &label, BLACK);
            }
            hits.push((Rect::new(x as f32, y as f32, 32.0, 32.0), Hit { toggle: false, object: worn.map(|it| it.object), frame: true, queue: None, tip, left, right: store }));
        }
        // The house toggle, in the own house: switches the grid, or with an object held, stores
        // it in the house (bag shown) or takes it into the bag (house shown).
        let house_view = self.house_view(world, player);
        if p.in_own_house && p.alive() {
            let (x, y) = (o.0 + 288, o.1 + 208);
            cell(textures, ACTION, x, y, 7, house_view as i32, (24, 24), false);
            let store = held.then(|| if house_view { Order::Unselect { perso } } else { Order::StoreHeldInHouse { perso } });
            let tip = s.house_toggle[house_view as usize + 2 * held as usize].clone();
            let hit = Hit { toggle: !held, object: None, frame: false, queue: None, tip, left: store.clone(), right: store };
            hits.push((Rect::new(x as f32, y as f32, 24.0, 24.0), hit));
        }
        if house_view {
            let m = &world.houses[player];
            for slot in 0..m.inventory.len().min(32) {
                let (x, y) = (o.0 + 32 + 32 * (slot as i32 % 8), o.1 + 209 + 32 * (slot as i32 / 8));
                let obj = usize::try_from(m.inventory[slot]).ok();
                if let Some(obj) = obj {
                    icon(obj, x, y);
                }
                // Left: pick up, or put the held object here (swapping); right or Ctrl+left:
                // into the bag.
                let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
                let to_bag = obj.map(|_| Order::HouseObjectToBag { perso, slot });
                let left = if held {
                    Some(Order::PlaceInHouse { perso, slot })
                } else if ctrl {
                    to_bag.clone()
                } else {
                    obj.map(|_| Order::HoldHouseObject { perso, slot })
                };
                let tip = obj.map(|it| object_tooltip(rules, &self.tooltip_labels, p, it, 0, None, Some(InInventory::of(world, player)))).unwrap_or_default();
                hits.push((Rect::new(x as f32, y as f32, 32.0, 32.0), Hit { toggle: false, object: obj, frame: true, queue: None, tip, left, right: to_bag }));
            }
            return;
        }
        // `Ctrl+left`: discard at once — into the own house, sold in a castle, else dropped.
        let in_castle = p.in_castle || p.in_own_castle;
        let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        for slot in 0..p.inventory.len().min(32) {
            let (x, y) = (32 + 32 * (slot as i32 % 8), 209 + 32 * (slot as i32 / 8));
            let Some(item) = p.inventory[slot] else {
                // an empty slot receives what is held
                let back = held.then_some(Order::PlaceHeld { perso, slot });
                hits.push((Rect::new((o.0 + x) as f32, (o.1 + y) as f32, 32.0, 32.0), Hit { toggle: false, object: None, frame: true, queue: None, tip: String::new(), left: back, right: None }));
                continue;
            };
            icon(item.object, o.0 + x, o.1 + y);
            if let Some(label) = self.shortcuts.label(item.object, set) {
                self.small_text(o.0 + x, o.1 + y, &label, BLACK);
            }
            if item.count > 1 {
                self.small_text(o.0 + x + 4, o.1 + y + 19, &format!("x{}", item.count), BLACK);
            }
            let left = if ctrl {
                if p.in_own_house {
                    Order::DepositObject { perso, object: slot, from_inventory: true }
                } else if in_castle {
                    Order::SellObject { perso, object: slot, from_inventory: true }
                } else {
                    Order::DropObject { perso, object: slot, from_inventory: true }
                }
            } else if held {
                // stack with the same object, or swap with this one
                Order::PlaceHeld { perso, slot }
            } else {
                Order::HoldObject { perso, slot }
            };
            // TODO: the full object tooltip (`Definir_InfoBulle_Objet`)
            let tip = object_tooltip(rules, &self.tooltip_labels, p, item.object, item.count, None, Some(InInventory::of(world, player)));
            let equip = Order::EquipObject { perso, slot, right: false };
            hits.push((Rect::new((o.0 + x) as f32, (o.1 + y) as f32, 32.0, 32.0), Hit { toggle: false, object: Some(item.object), frame: true, queue: None, tip, left: Some(left), right: Some(equip) }));
        }
    }

    /// `FileFabricationObjets_Changer`: queue (or unqueue) 1, 5 (Ctrl), 10 (Shift) or 20 (both)
    /// of a crafting slot. Unqueuing the last one of an object stops the current craft.
    fn change_craft_queue(&mut self, player: usize, slot: usize, add: bool, input: &Input, orders: &mut Vec<Order>) {
        let perso = player;
        for _ in 0..queue_quantity(input.ctrl, input.shift) {
            if add {
                self.craft_queue.push(slot);
            } else if self.craft_queue.is_empty() {
                orders.push(Order::StopCraft { perso });
            } else if let Some(k) = self.craft_queue.iter().position(|&q| q == slot) {
                self.craft_queue.remove(k);
            } else {
                // none of this object queued: stop making it
                orders.push(Order::StopCraft { perso });
            }
        }
    }

    /// `Actions_Automatiques`, once per tick: while idle in the own workshop, take the next
    /// queued object and make it if possible (it is dropped otherwise); outside, the queue
    /// empties.
    pub fn craft_from_queue(&mut self, world: &World, rules: &Rules, player: usize, orders: &mut Vec<Order>) {
        let p = &world.persos[player];
        if !p.in_own_house {
            self.craft_queue.clear();
            return;
        }
        // `Inactif`: idle outside or resting at home
        let inactive = p.action == action::IDLE || p.action == action::AT_HOME;
        if inactive && !self.craft_queue.is_empty() {
            let slot = self.craft_queue.remove(0);
            if world.can_craft(rules, player, slot as i64, player) {
                orders.push(Order::Craft { perso: player, slot: slot as i64 });
            }
        }
    }

    /// `Afficher_Objets_Batiment`: what a workshop makes, 8 rows from y=160.
    fn crafting_list(&self, world: &World, rules: &Rules, textures: &Textures, player: usize, h: usize, o: (i32, i32), hits: &mut Vec<(Rect, Hit)>) {
        let p = &world.persos[player];
        let house = &world.houses[h];
        let objects = &rules.buildings[house.kind].objects;
        let own = h == player;
        let perso = player;
        let epoch = world.fiefs[p.fief].epoch as i64;
        let rect = |x: i32, y: i32, w: i32, hh: i32| Rect::new((o.0 + x) as f32, (o.1 + y) as f32, w as f32, hh as f32);
        let first = self.windows.craft_scroll.min(objects.len().saturating_sub(8));
        for (r, (slot, &obj)) in objects.iter().enumerate().skip(first).take(8).enumerate() {
            let t = &rules.objects[obj];
            let y = 160 + 24 * r as i32;
            self.text(o.0 + 32, o.1 + y, &t.plural, BLACK);
            Self::separator(o, y);
            if let Some(tex) = textures.get(&object_icon(rules, obj)) {
                // the 32×32 icon at three quarters
                let src = Rect::new(0.0, 0.0, 32.0, 32.0);
                let params = DrawTextureParams { source: Some(src), dest_size: Some(vec2(24.0, 24.0)), ..Default::default() };
                tex.draw_ex((o.0 + 168) as f32, (o.1 + y - 6) as f32, WHITE, params);
            }
            let stock = house.object_stock(slot);
            self.text(o.0 + 200, o.1 + y, &format!("x{}", stock.floor()), BLACK);
            let locked = t.epoch - 1 > epoch;
            let state = if locked {
                2
            } else if world.can_craft(rules, player, slot as i64, h) {
                0
            } else {
                1
            };
            if own || locked {
                cell(textures, ACTION, o.0 + 232, o.1 + y - 4, 1, state, (24, 24), false);
            }
            if own && p.craft_slot == slot as i64 && p.craft_progress > 0.0 && t.craft_time > 0 {
                let pct = (p.craft_progress / t.craft_time as f64 * 100.0).floor();
                self.text(o.0 + 264, o.1 + y, &format!("{pct}%"), BLACK);
            }
            // Tooltip: "Fabriquer : <name>", the epoch if locked, then the costs.
            let mut tip = format!("{}{}", self.windows.strings.craft, t.name);
            if locked {
                tip += &format!("#&({})", rules.epochs[(t.epoch - 1).max(0) as usize].name);
            }
            for (k, &q) in t.price_resources.iter().enumerate().filter(|(_, q)| **q > 0) {
                let name = rules.resources[k].name.to_lowercase();
                let have = p.resource(k);
                tip += &if have >= q as f64 { format!("#({q} {name})") } else { format!("#&({} / {q} {name})", have.floor()) };
            }
            let icon_tip = object_tooltip(rules, &self.tooltip_labels, p, obj, 0, None, None);
            if own {
                let queued = self.craft_queue.iter().filter(|&&q| q == slot).count();
                if queued > 0 {
                    self.text(o.0 + 232, o.1 + y, &format!("+{queued}"), WHITE);
                }
                let queue = (state == 0 && p.alive()).then_some(slot);
                hits.push((rect(232, y - 4, 24, 24), Hit { toggle: false, object: None, frame: false, queue, tip: tip.clone(), left: None, right: None }));
                // The icon takes made objects, or queues some when there are none.
                if stock >= 1.0 {
                    let count = queue_quantity(is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl), is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift));
                    let take = Some(Order::TakeObjects { perso, slot, count: count as i64 });
                    hits.push((rect(168, y - 6, 48, 24), Hit { toggle: false, object: None, frame: false, queue: None, tip: icon_tip, left: take, right: None }));
                } else {
                    hits.push((rect(168, y - 6, 48, 24), Hit { toggle: false, object: None, frame: false, queue, tip: icon_tip, left: None, right: None }));
                }
            } else if stock >= 1.0 {
                hits.push((rect(168, y - 6, 48, 24), Hit { toggle: false, object: None, frame: false, queue: None, tip: icon_tip, left: Some(Order::BuyObject { perso, house: h, slot }), right: None }));
            }
        }
        if objects.len() > 8 {
            let (x, y) = o;
            cell(textures, DEFILEMENT, x + 300, y + 66, 0, 0, (16, 44), false);
            cell(textures, DEFILEMENT, x + 300, y + 308, 1, 0, (16, 44), false);
            cell(textures, DEFILEMENT, x + 300, y + 110, 2, 0, (16, 198), false);
            let n = (objects.len() - 8) as i32;
            cell(textures, DEFILEMENT, x + 300, y + 110 + 182 * first as i32 / n, 3, 0, (16, 16), false);
        }
    }

    /// Window 0 — Bâtiment: the house view, or the buy view in a castle or a market.
    fn building_window(&self, world: &World, rules: &Rules, textures: &Textures, player: usize, o: (i32, i32), hits: &mut Vec<(Rect, Hit)>) {
        let s = &self.windows.strings;
        let p = &world.persos[player];
        let perso = player;
        let rect = |x: i32, y: i32, w: i32, h: i32| Rect::new((o.0 + x) as f32, (o.1 + y) as f32, w as f32, h as f32);
        let dead = !p.alive() as i32;
        let in_a_house = p.in_house || p.in_own_house;
        let h = if p.in_own_house { player } else { p.house_target };
        let house = &world.houses[h];
        let btype = &rules.buildings[house.kind];

        if in_a_house && !btype.market {
            // House view
            self.title(o, &btype.name);
            self.text(o.0 + 32, o.1 + 54, &format!("Vie : {} / {}", house.life.floor(), house.max_life), BLACK);
            let own = h == player;
            if own {
                self.text(o.0 + 32, o.1 + 96, &format!("{} : {}", s.money, house.money.floor()), BLACK);
                if house.built {
                    for j in 0..3 {
                        cell(textures, TRANSFERT, o.0 + 216 + 24 * j, o.1 + 90, j, 0, (24, 24), true);
                        let q = [1.0, 5.0, house.money.floor()][j as usize];
                        let tip = s.take_money[j as usize].clone();
                        hits.push((rect(216 + 24 * j, 90, 24, 24), Hit { toggle: false, object: None, frame: false, queue: None, tip, left: Some(Order::TakeMoney { perso, quantity: q }), right: None }));
                    }
                }
            }
            let stock = house.stock();
            if btype.factory {
                let line = if stock < 1.0 { s.empty.clone() } else { format!("{} / {}", stock.floor(), house.max_stock) };
                self.text(o.0 + 32, o.1 + 138, &format!("{}{line}", s.objects), BLACK);
                self.crafting_list(world, rules, textures, player, h, o, hits);
            } else if btype.product >= 0 {
                let res = &rules.resources[btype.product as usize];
                let line = if stock < 1.0 { format!("{} : {}", res.name, s.empty) } else { format!("{} : {} / {}", res.name, stock.floor(), house.max_stock) };
                self.text(o.0 + 32, o.1 + 138, &line, BLACK);
                if house.built {
                    for j in 0..3 {
                        let q = [1.0, 5.0, stock.floor()][j as usize];
                        let hit = if own {
                            cell(textures, TRANSFERT, o.0 + 216 + 24 * j, o.1 + 132, j, 0, (24, 24), true);
                            Hit { toggle: false, object: None, frame: false, queue: None, tip: s.take_stock[j as usize].clone(), left: Some(Order::TakeStock { perso, quantity: q }), right: None }
                        } else {
                            cell(textures, COMMERCE, o.0 + 216 + 24 * j, o.1 + 132, j, dead, (24, 24), true);
                            let price = q * res.price * rules.buy_coef;
                            let red = if price > p.money() { "&" } else { "" };
                            Hit { toggle: false, object: None, frame: false, queue: None,
                                tip: format!("Acheter {q} {}  #{red}{} : {price}", res.name, s.price),
                                left: Some(Order::BuyFromHouse { perso, house: h, quantity: q }),
                                right: None,
                            }
                        };
                        hits.push((rect(216 + 24 * j, 132, 24, 24), hit));
                    }
                }
            } else if btype.service > 0 {
                let svc = &rules.services[btype.service];
                let line = if house.max_stock > 0.0 {
                    if stock < 1.0 { format!("{} : {}", svc.name, s.unavailable) } else { format!("{} : {} / {}", svc.name, stock.floor(), house.max_stock) }
                } else {
                    svc.name.clone()
                };
                self.text(o.0 + 32, o.1 + 138, &line, BLACK);
                cell(textures, ACTION, o.0 + 216, o.1 + 136, 5, (stock < 1.0) as i32, (24, 24), true);
                hits.push((rect(216, 136, 24, 24), Hit { toggle: false, object: None, frame: false, queue: None, tip: " (Entrée)".to_owned(), left: Some(Order::UseService { perso, house: h }), right: None }));
                if house.max_stock > 0.0 || own {
                    self.text(o.0 + 32, o.1 + 160, &svc.description, BLACK);
                }
            }
            return;
        }

        // Buy view: castle (or market)
        self.title(o, &s.titles[0]);
        let f = &world.fiefs[p.fief];
        if in_a_house {
            self.text(o.0 + 32, o.1 + 54, &format!("Vie : {} / {}", house.life.floor(), house.max_life), BLACK);
        } else {
            let c = &world.castles[p.fief];
            self.text(o.0 + 32, o.1 + 54, &format!("Vie : {} / {}", c.life.floor(), c.max_life), BLACK);
            cell(textures, ACTION, o.0 + 140, o.1 + 16, 10, 1, (24, 24), false);
            // TODO: emperor button state and order
            for j in 0..3 {
                cell(textures, ACTION, o.0 + 216 + 24 * j, o.1 + 48, 2 + j, dead, (24, 24), true);
                let q = [5.0, 50.0, p.money().floor()][j as usize];
                hits.push((rect(216 + 24 * j, 48, 24, 24), Hit { toggle: false, object: None, frame: false, queue: None, tip: s.restore[j as usize].clone(), left: Some(Order::DonateToCastle { perso, quantity: q }), right: None }));
            }
        }
        for r in 0..ROWS {
            let k = r + self.windows.scroll;
            let Some(res) = rules.resources.get(k) else { break };
            let y = 88 + 24 * r as i32;
            let price = res.price * rules.buy_coef;
            let label = if in_a_house { format!("{} :", res.name) } else { format!("{} : {price}", s.price) };
            self.text(o.0 + 32, o.1 + y, &label, BLACK);
            let stock = f.resources[k];
            self.text(o.0 + 136, o.1 + y, &stock.floor().to_string(), BLACK);
            Self::separator(o, y);
            cell(textures, &resource_icon(&res.image), o.0 + 104, o.1 + y - 10, 0, 0, (32, 32), false);
            for j in 0..3 {
                cell(textures, COMMERCE, o.0 + 216 + 24 * j, o.1 + y - 6, j, dead, (24, 24), true);
                let q = match j {
                    0 => 1.0,
                    1 => 5.0,
                    _ => (p.max_resource(k) - p.resource(k)).min(stock).floor(),
                };
                let red = if q * price > p.money() { "&" } else { "" };
                let order = Order::BuyResource { perso, resource: k, quantity: [1.0, 5.0, p.max_resource(k)][j as usize] };
                hits.push((rect(216 + 24 * j, y - 6, 24, 24), Hit { toggle: false, object: None, frame: false, queue: None,
                    tip: format!("Acheter {q} {}  #{red}{} : {}", res.name, s.price, q * price),
                    left: Some(order),
                    right: None,
                }));
            }
        }
        self.scroll_bar(textures, o, rules.resources.len().saturating_sub(ROWS), self.windows.scroll);
    }
}

/// `Progression_Competence*`: how far a skill is toward its next level, 0 to 1.
fn progress(s: &sim::perso::Skill) -> f64 {
    let x = &s.xp;
    (x.exp - x.exp_level) / (x.exp_next - x.exp_level)
}

impl Hud {
    /// `Actualiser_Competences` (`CI:1341-1493`): the special, resource, service and item
    /// skills that have a level or some progress (all of them with "show all"), sorted. The
    /// original rebuilt it once a second; here once per tick.
    fn skill_rows(&self, world: &World, rules: &Rules, player: usize) -> std::rc::Rc<Vec<SkillRow>> {
        let key = (world.tick, self.windows.skill_sort, self.windows.skill_all);
        let mut cache = self.windows.skill_cache.borrow_mut();
        match &*cache {
            Some((k, rows)) if *k == key => rows.clone(),
            _ => {
                let rows = std::rc::Rc::new(self.make_skill_rows(world, rules, player));
                *cache = Some((key, rows.clone()));
                rows
            }
        }
    }

    /// The table [`Hud::skill_rows`] keeps for a tick.
    fn make_skill_rows(&self, world: &World, rules: &Rules, player: usize) -> Vec<SkillRow> {
        let p = &world.persos[player];
        let all = self.windows.skill_all;
        let shown = |s: &sim::perso::Skill| all || s.xp.level > 0 || progress(s) > 0.0;
        let mut rows = Vec::new();
        for (i, s) in p.special.iter().enumerate().filter(|(_, s)| shown(s)) {
            rows.push(SkillRow {
                name: format!("{} :", rules.special_skills[i]),
                value: s.bonus,
                progress: progress(s),
                // building (action 6), demolishing (attacking a house or a castle)
                active: (p.action == 6 && i == 0) || ((p.action == 301 || p.action == 302) && i == 1),
                bonus: p.special_boosted(rules, i),
            });
        }
        for (i, s) in p.resource_skills.iter().enumerate().filter(|(_, s)| shown(s)) {
            rows.push(SkillRow {
                name: format!("{} :", rules.resources[i].skill_name),
                value: p.harvest_level(i) + rules.start_max_resources,
                progress: progress(s),
                active: p.current_resource == i as i64,
                bonus: p.resource_boosted(rules, i),
            });
        }
        for (i, s) in p.services.iter().enumerate().filter(|(_, s)| shown(s)) {
            rows.push(SkillRow {
                name: format!("{} :", rules.services[i + 1].skill_name),
                value: s.bonus,
                progress: progress(s),
                active: p.action == 600 && p.current_service == i + 1,
                bonus: p.service_boosted(rules, i),
            });
        }
        // Item skills in their display order (`NoListe`), the first ("none") left out.
        let worn = || p.equipped.iter().flatten().map(|it| &rules.objects[it.object]);
        for k in 1..p.objects.len() {
            let j = self.tooltip_labels.item_skill_order.get(k).copied().unwrap_or(k);
            let Some(s) = p.objects.get(j).filter(|s| shown(s)) else { continue };
            let mut active = if p.action == action::CRAFT {
                // the skill of the object being made
                let m = &world.houses[player];
                let made = rules.buildings[m.kind].objects.get(p.craft_slot.max(0) as usize);
                made.is_some_and(|&o| rules.objects[o].skill == j)
            } else if let Ok(r) = usize::try_from(p.current_resource) {
                // a worn object of this skill helps harvesting
                worn().any(|o| o.bonus_resource.get(r).is_some_and(|&b| b > 0) && o.skill == j)
            } else {
                false
            };
            if !p.idle() && p.alive() {
                // a worn object of this skill raising Energie or Moral
                active |= worn().any(|o| (o.bonus[1] > 0 || o.bonus[3] != 0) && o.skill == j);
            }
            rows.push(SkillRow {
                name: format!("{} :", rules.item_skills[j]),
                value: s.bonus,
                progress: progress(s),
                active,
                bonus: p.object_boosted(rules, j),
            });
        }
        // `CompetencesTri`: by name, or by score then progress, highest first (stable sorts,
        // like the original's bubble sorts).
        match self.windows.skill_sort {
            1 => rows.sort_by(|a, b| a.name.cmp(&b.name)),
            2 => rows.sort_by(|a, b| (b.value as f64 + b.progress).total_cmp(&(a.value as f64 + a.progress))),
            _ => {}
        }
        rows
    }

    /// `Afficher_InfosCompetences1`: the white progress bars, drawn before the background
    /// (they show through its holes).
    fn skill_bars(&self, world: &World, rules: &Rules, player: usize, o: (i32, i32)) {
        let rows = self.skill_rows(world, rules, player);
        for (y, row) in rows.iter().skip(self.windows.skill_scroll).take(SKILL_ROWS).enumerate() {
            super::status_bar(row.progress, 0.0, 1.0, o.0 + 32 + 183, o.1 + 86 + 16 * y as i32, 81, 17, Some(WHITE));
        }
    }

    /// Window 3 — Compétences (`Afficher_InfosCompetences2`): sort buttons, then a line per skill:
    /// name, value and progress percentage.
    fn skills_window(&self, world: &World, rules: &Rules, textures: &Textures, player: usize, o: (i32, i32)) {
        let s = &self.windows.strings;
        self.text(o.0 + 128, o.1 + 32, &s.titles[3], BLACK);
        self.text(o.0 + 240, o.1 + 58, &s.niv_plus, BLACK);
        for (k, bx) in SKILL_SORT_X.into_iter().enumerate() {
            let on = if k < 3 { self.windows.skill_sort == k } else { self.windows.skill_all };
            cell(textures, COMPETENCES, o.0 + bx, o.1 + 61, k as i32, on as i32, (28, 16), false);
        }
        let dark_red = Color::from_rgba(128, 0, 0, 255);
        let rows = self.skill_rows(world, rules, player);
        for (y, row) in rows.iter().skip(self.windows.skill_scroll).take(SKILL_ROWS).enumerate() {
            let ty = o.1 + 86 + 16 * y as i32;
            self.text(o.0 + 32, ty, &row.name, if row.active { dark_red } else { BLACK });
            // `Str()` puts a space before positive numbers: x = 216 - 8 × Len(Str(value)).
            let value = row.value.to_string();
            let len = value.len() as i32 + (row.value >= 0) as i32;
            self.text(o.0 + 216 - 8 * len, ty, &value, if row.bonus { dark_red } else { BLACK });
            self.text(o.0 + 240, ty, &format!("{:02}%", (row.progress * 100.0).floor() as i64), BLACK);
        }
        if rows.len() > SKILL_ROWS {
            self.scroll_bar(textures, o, rows.len() - SKILL_ROWS, self.windows.skill_scroll);
        }
    }

    /// Window 5 — Bestiaire (`CIF:1262-1302`): kills per race, in alphabetical order of the
    /// races' plural names, singular or plural by count. As in the original, the scroll offset
    /// skips entries of the whole race list, killed or not.
    fn bestiary_window(&self, world: &World, rules: &Rules, textures: &Textures, player: usize, o: (i32, i32)) {
        let p = &world.persos[player];
        self.text(o.0 + 128, o.1 + 32, &self.windows.strings.titles[5], BLACK);
        let plural = &self.tooltip_labels.race_plural;
        let mut order: Vec<usize> = (0..rules.races.len()).collect();
        order.sort_by(|&a, &b| plural.get(a).cmp(&plural.get(b)));
        let mut y = 0;
        for (i, &r) in order.iter().enumerate() {
            let kills = p.bestiary.get(r).copied().unwrap_or(0);
            if kills <= 0 || i < self.windows.bestiary_scroll {
                continue;
            }
            let name = if kills >= 2 { plural.get(r).cloned().unwrap_or_default() } else { rules.races[r].name[0].clone() };
            let ty = o.1 + 86 + 16 * y;
            self.text(o.0 + 32, ty, &format!("{name} :"), BLACK);
            self.text(o.0 + 192, ty, &kills.to_string(), BLACK);
            y += 1;
            if y as usize >= SKILL_ROWS {
                break;
            }
        }
        self.scroll_bar(textures, o, rules.races.len().saturating_sub(SKILL_ROWS), self.windows.bestiary_scroll);
    }
}
