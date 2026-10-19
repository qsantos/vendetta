//! The eight 320×368 windows (`ClsIntFenetre`), positions relative to the window origin.

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
const DEFAULT_POS: [(i32, i32); 8] = [(704, 0), (704, 368), (64, 0), (64, 368), (384, 0), (384, 368), (64, 0), (64, 0)];
/// Draw order (later on top).
const DRAW_ORDER: [usize; 8] = [4, 0, 1, 2, 3, 5, 6, 7];
const ROWS: usize = 11;

fn background(i: usize, female: bool) -> String {
    let name = match i {
        2 => "Fenetre-Personnage",
        3 => "Fenetre-Competences",
        4 if female => "Fenetre-InventaireF",
        4 => "Fenetre-InventaireH",
        6 => "Fenetre-Carte",
        _ => "Fenetre-Defaut",
    };
    format!("Images/Fenetres/{name}.bmp")
}

pub fn images(rules: &Rules) -> Vec<String> {
    let mut v: Vec<String> = [ACTION, COMMERCE, TRANSFERT, DEFILEMENT].map(String::from).to_vec();
    v.extend((0..8).flat_map(|i| [background(i, false), background(i, true)]));
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
}

/// Opaque blit (black drawn) or colour-keyed blit of a sheet cell.
fn cell(textures: &Textures, sheet: &str, x: i32, y: i32, col: i32, row: i32, size: (i32, i32), opaque: bool) {
    let Some(tex) = textures.get(sheet) else { return };
    if opaque {
        draw_rectangle(x as f32, y as f32, size.0 as f32, size.1 as f32, BLACK);
    }
    let src = Rect::new((col * size.0) as f32, (row * size.1) as f32, size.0 as f32, size.1 as f32);
    draw_texture_ex(tex, x as f32, y as f32, WHITE, DrawTextureParams { source: Some(src), ..Default::default() });
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

/// What a window button does, collected while drawing and applied by `interact`.
struct Hit {
    tip: String,
    left: Option<Order>,
    right: Option<Order>,
    /// Left click queues this crafting slot, right click unqueues it (with Ctrl/Shift).
    queue: Option<usize>,
    /// Hovering draws an orange frame around it (inventory slots).
    frame: bool,
}

impl Windows {
    pub fn new(strings: WindowStrings) -> Windows {
        Windows { strings, pos: DEFAULT_POS, moved: [false; 8], dragging: None, scroll: 0, craft_scroll: 0 }
    }
}

impl Hud {
    /// The list window `i` scrolls, if any: (is the crafting list, last offset).
    fn scroll_list(&self, world: &World, rules: &Rules, player: usize, i: usize) -> Option<(bool, usize)> {
        let p = &world.persos[player];
        if i == 0 && (p.in_house || p.in_own_house) {
            let h = if p.in_own_house { player } else { p.house_target };
            let b = &rules.buildings[world.houses[h].kind];
            if !b.market {
                // a house: only a workshop with more than 8 objects scrolls
                return (b.factory && b.objects.len() > 8).then(|| (true, b.objects.len() - 8));
            }
        }
        Some((false, rules.resources.len().saturating_sub(ROWS)))
    }

    /// Whether the mouse is over an open window, and whether that window scrolls.
    pub fn over_window(&self, world: &World, rules: &Rules, player: usize, m: (i32, i32)) -> Option<bool> {
        (0..8)
            .filter(|&i| self.window_visible(world, player, i))
            .find(|&i| inside(m, self.windows.pos[i].0, self.windows.pos[i].1, W, H))
            .map(|i| i <= 1 && self.scroll_list(world, rules, player, i).is_some())
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
            if let Some(tex) = textures.get(&background(i, female)) {
                draw_texture(tex, x as f32, y as f32, WHITE);
            }
            match i {
                0 => self.building_window(world, rules, textures, player, (x, y), &mut hits),
                1 => self.resources_window(world, rules, textures, player, (x, y), &mut hits),
                4 => self.inventory_window(world, rules, textures, player, (x, y), &mut hits),
                _ => {
                    // TODO: windows 2-7 contents
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
            for i in [0, 1] {
                let (x, y) = self.windows.pos[i];
                if !self.window_visible(world, player, i) || !inside(m, x, y, W, H) {
                    continue;
                }
                if let Some((crafting, n)) = self.scroll_list(world, rules, player, i) {
                    let offset = if crafting { &mut self.windows.craft_scroll } else { &mut self.windows.scroll };
                    *offset = if wheel > 0.0 { offset.saturating_sub(1) } else { (*offset + 1).min(n) };
                }
                break;
            }
        }

        // Window buttons, in the original's precedence: contents, then close/move.
        for (r, hit) in &hits {
            if inside(m, r.x as i32, r.y as i32, r.w as i32, r.h as i32) {
                *tip = hit.tip.clone();
                if hit.frame {
                    let (x, y) = (r.x as i32, r.y as i32);
                    outline(x, y, x + 32, y + 32, Color::from_rgba(255, 128, 0, 255));
                }
                if let Some(slot) = hit.queue {
                    if input.left_pressed || input.right_pressed {
                        self.change_craft_queue(player, slot, input.left_pressed, input, orders);
                    }
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
        // Scroll bars of windows 0/1: the resource lists (shared), or a workshop's crafting list.
        for i in [0, 1] {
            if !self.window_visible(world, player, i) {
                continue;
            }
            let Some((crafting, n)) = self.scroll_list(world, rules, player, i) else { continue };
            let (x, y) = self.windows.pos[i];
            let offset = if crafting { &mut self.windows.craft_scroll } else { &mut self.windows.scroll };
            if inside(m, x + 300, y + 66, 16, 44) {
                *tip = self.windows.strings.scroll[0].clone();
                if input.left_pressed {
                    *offset = offset.saturating_sub(1);
                }
                return true;
            }
            if inside(m, x + 300, y + 308, 16, 44) {
                *tip = self.windows.strings.scroll[1].clone();
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
    fn scroll_bar(&self, textures: &Textures, origin: (i32, i32), n: usize) {
        let (x, y) = origin;
        cell(textures, DEFILEMENT, x + 300, y + 66, 0, 0, (16, 44), false);
        cell(textures, DEFILEMENT, x + 300, y + 308, 1, 0, (16, 44), false);
        cell(textures, DEFILEMENT, x + 300, y + 110, 2, 0, (16, 198), false);
        let thumb = if n > 0 { 110 + 182 * self.windows.scroll as i32 / n as i32 } else { 110 };
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
            hits.push((rect(216 + 24 * j, 48, 24, 24), Hit { frame: false, queue: None, tip: label.clone(), left: None, right: None }));
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
                hits.push((rect(184, y - 6, 24, 24), Hit { frame: false, queue: None, tip, left: Some(eat), right: None }));
            }
            let quantities = [1.0, 5.0, p.resource(k).floor()];
            if own_house && product == k as i64 && world.houses[player].max_stock > 0.0 {
                for j in 0..3 {
                    cell(textures, TRANSFERT, o.0 + 216 + 24 * j, o.1 + y - 6, 3 + j, 0, (24, 24), true);
                    let deposit = Order::DepositStock { perso, quantity: quantities[j as usize], all: j == 2 };
                    let tip = s.deposit_stock[j as usize].clone();
                    hits.push((rect(216 + 24 * j, y - 6, 24, 24), Hit { frame: false, queue: None, tip, left: Some(deposit), right: None }));
                }
                continue;
            }
            for j in 0..3 {
                let col = j + if in_castle { 3 } else { 0 };
                cell(textures, COMMERCE, o.0 + 216 + 24 * j, o.1 + y - 6, col, dead, (24, 24), true);
                let q = quantities[j as usize];
                let hit = if in_castle {
                    let order = Order::SellResource { perso, resource: k, quantity: q };
                    Hit { frame: false, queue: None,
                        tip: format!("Vendre {q} {}  #{} : {}", res.name, s.price, q * res.price),
                        left: Some(order.clone()),
                        right: Some(order),
                    }
                } else {
                    Hit { frame: false, queue: None,
                        tip: format!("Prendre {q} {}  #{} : {}", res.name, s.price, q * res.price),
                        left: Some(Order::SelectResource { perso, resource: k, quantity: q }),
                        right: Some(Order::UnselectResource { perso, resource: k, quantity: q }),
                    }
                };
                hits.push((rect(216 + 24 * j, y - 6, 24, 24), hit));
            }
        }
        self.scroll_bar(textures, o, rules.resources.len().saturating_sub(ROWS));

        // Money line, with deposit buttons in the own house.
        self.text(o.0 + 32, o.1 + 54, &format!("{} : {}", s.money, p.money().floor()), BLACK);
        if own_house {
            for j in 0..3 {
                cell(textures, TRANSFERT, o.0 + 216 + 24 * j, o.1 + 48, 3 + j, 0, (24, 24), true);
                let q = [1.0, 5.0, p.money().floor()][j as usize];
                let tip = s.deposit_money[j as usize].clone();
                hits.insert(0, (rect(216 + 24 * j, 48, 24, 24), Hit { frame: false, queue: None, tip, left: Some(Order::DepositMoney { perso, quantity: q }), right: None }));
            }
        }
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
                draw_texture_ex(tex, x as f32, y as f32, WHITE, DrawTextureParams { source: Some(src), ..Default::default() });
            }
        };
        // TODO: equipping (right click) once items exist
        for (k, &(ex, ey)) in EQUIPMENT.iter().enumerate() {
            if let Some(Some(item)) = p.equipped.get(k) {
                icon(item.object, o.0 + ex, o.1 + ey);
            }
        }
        // `Ctrl+left`: discard at once — into the own house, sold in a castle, else dropped.
        let in_castle = p.in_castle || p.in_own_castle;
        let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        for slot in 0..p.inventory.len().min(32) {
            let (x, y) = (32 + 32 * (slot as i32 % 8), 209 + 32 * (slot as i32 / 8));
            let Some(item) = p.inventory[slot] else {
                // an empty slot puts back what is held
                let back = (p.selected_object >= 0).then_some(Order::Unselect { perso });
                hits.push((Rect::new((o.0 + x) as f32, (o.1 + y) as f32, 32.0, 32.0), Hit { frame: true, queue: None, tip: String::new(), left: back, right: None }));
                continue;
            };
            icon(item.object, o.0 + x, o.1 + y);
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
            } else {
                Order::HoldObject { perso, slot }
            };
            // TODO: the full object tooltip (`Definir_InfoBulle_Objet`)
            let tip = rules.objects[item.object].name.clone();
            hits.push((Rect::new((o.0 + x) as f32, (o.1 + y) as f32, 32.0, 32.0), Hit { frame: true, queue: None, tip, left: Some(left), right: None }));
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
                draw_texture_ex(tex, (o.0 + 168) as f32, (o.1 + y - 6) as f32, WHITE, params);
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
            if own {
                let queued = self.craft_queue.iter().filter(|&&q| q == slot).count();
                if queued > 0 {
                    self.text(o.0 + 232, o.1 + y, &format!("+{queued}"), WHITE);
                }
                let queue = (state == 0 && p.alive()).then_some(slot);
                hits.push((rect(232, y - 4, 24, 24), Hit { frame: false, queue, tip: tip.clone(), left: None, right: None }));
                // The icon takes made objects, or queues some when there are none.
                if stock >= 1.0 {
                    let count = queue_quantity(is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl), is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift));
                    let take = Some(Order::TakeObjects { perso, slot, count: count as i64 });
                    hits.push((rect(168, y - 6, 48, 24), Hit { frame: false, queue: None, tip: t.name.clone(), left: take, right: None }));
                } else {
                    hits.push((rect(168, y - 6, 48, 24), Hit { frame: false, queue, tip: t.name.clone(), left: None, right: None }));
                }
            } else if stock >= 1.0 {
                hits.push((rect(168, y - 6, 48, 24), Hit { frame: false, queue: None, tip: t.name.clone(), left: Some(Order::BuyObject { perso, house: h, slot }), right: None }));
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
                        hits.push((rect(216 + 24 * j, 90, 24, 24), Hit { frame: false, queue: None, tip, left: Some(Order::TakeMoney { perso, quantity: q }), right: None }));
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
                            Hit { frame: false, queue: None, tip: s.take_stock[j as usize].clone(), left: Some(Order::TakeStock { perso, quantity: q }), right: None }
                        } else {
                            cell(textures, COMMERCE, o.0 + 216 + 24 * j, o.1 + 132, j, dead, (24, 24), true);
                            let price = q * res.price * rules.buy_coef;
                            let red = if price > p.money() { "&" } else { "" };
                            Hit { frame: false, queue: None,
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
                hits.push((rect(216, 136, 24, 24), Hit { frame: false, queue: None, tip: " (Entrée)".to_owned(), left: Some(Order::UseService { perso, house: h }), right: None }));
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
                hits.push((rect(216 + 24 * j, 48, 24, 24), Hit { frame: false, queue: None, tip: s.restore[j as usize].clone(), left: Some(Order::DonateToCastle { perso, quantity: q }), right: None }));
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
                hits.push((rect(216 + 24 * j, y - 6, 24, 24), Hit { frame: false, queue: None,
                    tip: format!("Acheter {q} {}  #{red}{} : {}", res.name, s.price, q * price),
                    left: Some(order),
                    right: None,
                }));
            }
        }
        self.scroll_bar(textures, o, rules.resources.len().saturating_sub(ROWS));
    }
}
