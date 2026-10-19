//! Drawing the world, following `FrmMoteur2D`'s frame composition.

use crate::assets::Textures;
use crate::hud::{status_bar, status_bar_v};
use macroquad::prelude::*;
use sim::rules::Rules;
use sim::terrain::CELL;
use sim::perso::{Look, Perso};
use sim::treasure::TREASURE_SIZE;
use sim::world::{RESOURCE_SIZE, World};


pub const TILESET: &str = "Images/Terrains/Plaine.bmp";
pub const RESOURCES: &str = "Images/Decors/Ressources.bmp";
pub const TREASURES: &str = "Images/Decors/Tresors.bmp";

pub fn castle_image(rules: &Rules, people: usize) -> String {
    format!("{}.bmp", rules.peoples[people].image)
}

/// Images the world needs, grouped by the loading screen's messages: (terrain and
/// decors, characters).
pub fn images(rules: &Rules, world: &World) -> (Vec<String>, Vec<String>) {
    let mut terrain: Vec<String> = [TILESET, RESOURCES, TREASURES].map(String::from).to_vec();
    terrain.extend(rules.buildings.iter().map(|b| format!("{}.bmp", b.image)));
    terrain.extend(rules.decors.iter().map(|d| decor_image(&d.image)));
    terrain.extend((0..rules.peoples.len()).map(|p| castle_image(rules, p)));
    let persos = world.persos.iter().flat_map(|p| look_sheets(&p.look).to_vec()).collect();
    (terrain, persos)
}

fn decor_image(image: &str) -> String {
    match image.strip_prefix('.') {
        Some(path) => format!("{path}.bmp"),
        None => format!("Images/Decors/{image}.bmp"),
    }
}

fn look_sheets(look: &Look) -> &[String] {
    match look {
        Look::Sheet(s) => std::slice::from_ref(s),
        Look::Layers(l) => l,
    }
}

/// `Afficher_CadreSelection` for a character: a 1-px ellipse at its feet.
fn draw_selection(p: &Perso, cam: (i32, i32)) {
    let (l, h) = (p.w as f32, p.h as f32);
    let (x, y) = ((p.x as i32 - cam.0) as f32, (p.y as i32 - cam.1) as f32);
    let (x1, y1) = (x + l / 12.0, y + h - l / 3.0 - h / 8.0);
    let (x2, y2) = (x + l - l / 12.0, y + h + l / 3.0 - h / 6.0);
    // TODO: red in combat, white when inactive
    let colour = Color::from_rgba(0, 0, 192, 255);
    draw_ellipse_lines((x1 + x2) / 2.0, (y1 + y2) / 2.0, (x2 - x1) / 2.0, (y2 - y1) / 2.0, 0.0, 1.0, colour);
}

/// `Afficher_Etat_Travail`: the player's work progress bars (white).
fn draw_work_state(world: &World, rules: &Rules, i: usize, cam: (i32, i32)) {
    let Some(p) = world.persos.get(i) else { return };
    let m = &world.houses[i];
    let (mx, my) = (m.x - cam.0, m.y - cam.1);
    if p.in_own_castle && !p.alive() {
        // Resurrection progress above the castle.
        if p.resurrections != 0 {
            let c = &world.castles[p.fief];
            status_bar(100.0 - p.resurrection_time, 0.0, 100.0, c.x - cam.0, c.y - cam.1 - 5, c.w, 5, Some(WHITE));
        }
        return;
    }
    if p.current_resource >= 0 {
        let r = p.current_resource as usize;
        match p.action {
            // TODO: mount offsets
            2 => status_bar(p.resource(r), 0.0, p.max_resource(r), p.x as i32 - cam.0, p.y as i32 - cam.1 - 5, p.w, 5, Some(WHITE)),
            102 => {
                status_bar(p.resource(r), 0.0, p.max_resource(r), mx, my - 5, m.w, 5, Some(WHITE));
                let raw = rules.resources[r].raw;
                if raw >= 0 {
                    let raw = raw as usize;
                    status_bar_v(p.resource(raw), 0.0, p.max_resource(raw), mx - 5, my, 5, m.h, Some(WHITE));
                }
            }
            _ => {}
        }
    } else if p.action == 500 {
        if let Some(&o) = rules.buildings[m.kind].objects.get(p.craft_slot.max(0) as usize) {
            let d = rules.objects[o].craft_time as f64;
            status_bar(p.craft_progress, 0.0, d, mx, my - 5, m.w, 5, Some(WHITE));
        }
    }
    if p.in_own_house && m.max_stock > 0.0 {
        status_bar_v(m.stock(), 0.0, m.max_stock, mx + m.w, my, 5, m.h, Some(WHITE));
    }
}

fn draw_persos(world: &World, textures: &Textures, cam: (i32, i32)) {
    for p in world.persos.iter().rev() {
        if !p.visible() || !p.alive() {
            continue;
        }
        // Bars below the sprite, drawn before it (`FrmMoteur2D.frm:1898-1940`): life, then
        // attack (or the reload) and defence while any is not full.
        let (x, y) = (p.x as i32 - cam.0, p.y as i32 - cam.1 + p.h);
        let max = p.bonus_max[0] as f64;
        let (max_attack, max_defense) = (p.bonus_max[4] as f64, p.bonus_max[5] as f64);
        let fighting = p.attack < max_attack || p.defense < max_defense || p.reload > 0;
        if p.bars[0] < max || fighting {
            status_bar(p.bars[0], p.bars[0], max, x, y, p.w, 5, None);
        }
        if fighting {
            if p.reload == 0 {
                status_bar(p.attack, p.attack, max_attack, x, y + 5, p.w, 5, Some(Color::from_rgba(255, 0, 0, 255)));
            } else {
                let done = (p.reload_max - p.reload) as f64;
                status_bar(0.0, done, p.reload_max as f64, x, y + 5, p.w, 5, None);
            }
            status_bar(p.defense, p.defense, max_defense, x, y + 10, p.w, 5, Some(Color::from_rgba(0, 0, 255, 255)));
        }
        let (col, row) = p.sprite_cell();
        let src = Rect::new((col * p.w) as f32, (row * p.h) as f32, p.w as f32, p.h as f32);
        for sheet in look_sheets(&p.look) {
            if let Some(tex) = textures.get(sheet) {
                blit(tex, p.x as i32 - cam.0, p.y as i32 - cam.1, src);
            }
        }
    }
}

/// Downloads the sheets of characters that appeared during the game (summons, projectiles).
pub fn request_looks(world: &World, textures: &mut Textures) {
    for p in world.persos.iter().filter(|p| p.visible() && p.alive()) {
        for sheet in look_sheets(&p.look) {
            textures.request(sheet);
        }
    }
}

fn blit(tex: &Texture2D, x: i32, y: i32, src: Rect) {
    draw_texture_ex(
        tex,
        x as f32,
        y as f32,
        WHITE,
        DrawTextureParams { source: Some(src), ..Default::default() },
    );
}

/// `ClsAffTerrain.AfficheFast`: one 16×16 tile per visible cell.
fn draw_terrain(world: &World, textures: &Textures, cam: (i32, i32), view: (i32, i32)) {
    let Some(tiles) = textures.get(TILESET) else { return };
    let t = &world.terrain;
    let (cx0, cy0) = (cam.0.div_euclid(CELL), cam.1.div_euclid(CELL));
    // Every cell overlapping the view, also when the camera is not on a cell boundary.
    let (cx1, cy1) = ((cam.0 + view.0 - 1).div_euclid(CELL), (cam.1 + view.1 - 1).div_euclid(CELL));
    for cy in cy0..=cy1 {
        if cy < 0 || cy >= t.height as i32 {
            continue;
        }
        for cx in cx0..=cx1 {
            if cx < 0 || cx >= t.width as i32 {
                continue;
            }
            let k = cy as usize * t.width + cx as usize;
            let src = Rect::new(
                (t.shape[k] as i32 * CELL) as f32,
                (t.category[k] as i32 * CELL) as f32,
                CELL as f32,
                CELL as f32,
            );
            blit(tiles, cx * CELL - cam.0, cy * CELL - cam.1, src);
        }
    }
}

/// `view` is the size of the visible area in world pixels.
pub fn draw_world(world: &World, rules: &Rules, textures: &Textures, cam: (i32, i32), player: usize, view: (i32, i32)) {
    draw_terrain(world, textures, cam, view);
    if let Some(tex) = textures.get(RESOURCES) {
        let s = RESOURCE_SIZE as f32;
        for r in &world.resources {
            blit(tex, r.x - cam.0, r.y - cam.1, Rect::new(r.kind as f32 * s, 0.0, s, s));
        }
    }
    // Afficher_Batiments: houses, then castles.
    for m in world.houses.iter().filter(|m| m.visible()) {
        let Some(tex) = textures.get(&format!("{}.bmp", rules.buildings[m.kind].image)) else { continue };
        let (w, h) = (m.w as f32, m.h as f32);
        let src = Rect::new(m.animation as f32 * w, m.state(rules) as f32 * h, w, h);
        blit(tex, m.x - cam.0, m.y - cam.1, src);
    }
    // Life bars of damaged houses (drawn after all houses here; they never overlap a house).
    for m in world.houses.iter().filter(|m| m.visible()) {
        let v = m.life_with_repair();
        if v < m.max_life as f64 {
            status_bar(v, v, m.max_life as f64, m.x - cam.0, m.y - cam.1 + m.h, m.w, 5, None);
        }
    }
    for (fief, c) in world.fiefs.iter().zip(&world.castles) {
        if !c.visible() {
            continue;
        }
        let Some(tex) = textures.get(&castle_image(rules, fief.people)) else { continue };
        // Only the Humans' sheet has one row per epoch.
        let row = if fief.people == 0 { fief.epoch as f32 } else { 0.0 };
        let src = Rect::new(0.0, row * c.h as f32, c.w as f32, c.h as f32);
        blit(tex, c.x - cam.0, c.y - cam.1, src);
        let (x, y) = (c.x - cam.0, c.y - cam.1);
        if c.life < c.max_life as f64 {
            status_bar(c.life, c.life_with_repair(), c.max_life as f64, x, y + c.h, c.w, 5, None);
        }
        if fief.changing_epoch {
            // Vertical white bar on the castle's right: progress toward the next age.
            if let Some(next) = rules.epochs.get(fief.epoch + 1) {
                let d = next.duration as f64 * world.epoch_speed;
                status_bar_v(d - fief.epoch_timer as f64, 0.0, d, x + c.w, y, 5, c.h, Some(WHITE));
            }
        }
    }
    for d in &world.decors {
        let Some(tex) = textures.get(&decor_image(&rules.decors[d.kind].image)) else { continue };
        let src = Rect::new(d.frame as f32 * d.w as f32, 0.0, d.w as f32, d.h as f32);
        blit(tex, d.x - cam.0, d.y - cam.1, src);
    }
    if let Some(tex) = textures.get(TREASURES) {
        let s = TREASURE_SIZE as f32;
        for t in &world.treasures {
            blit(tex, t.x - cam.0, t.y - cam.1, Rect::new(t.tier as f32 * s, 0.0, s, s));
        }
    }
    if let Some(p) = world.persos.get(player) {
        if p.visible() && p.alive() {
            draw_selection(p, cam);
        }
    }
    draw_work_state(world, rules, player, cam);
    draw_persos(world, textures, cam);
}
