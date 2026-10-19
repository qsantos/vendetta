//! Drawing the world, following `FrmMoteur2D`'s frame composition.

use crate::assets::{Manifest, Sprite, Textures};
use crate::hud::{status_bar, status_bar_v};
use macroquad::prelude::*;
use sim::rules::Rules;
use sim::terrain::CELL;
use sim::perso::{Look, Perso};
use sim::treasure::TREASURE_SIZE;
use sim::vb::clng;
use sim::world::{RESOURCE_SIZE, World};


pub const TILESET: &str = "Images/Terrains/Plaine.bmp";
pub const RESOURCES: &str = "Images/Decors/Ressources.bmp";
pub const TREASURES: &str = "Images/Decors/Tresors.bmp";

pub fn castle_image(rules: &Rules, people: usize) -> String {
    format!("{}.bmp", rules.peoples[people].image)
}

/// Images the game needs, grouped by the loading screen's messages: (terrain and decors,
/// characters). Every character, equipment and effect sheet is loaded, those of summons and
/// projectiles too, so that nothing is missing mid-game.
pub fn images(rules: &Rules, manifest: &Manifest) -> (Vec<String>, Vec<String>) {
    let mut terrain: Vec<String> = [TILESET, RESOURCES, TREASURES].map(String::from).to_vec();
    terrain.extend(rules.buildings.iter().map(|b| format!("{}.bmp", b.image)));
    terrain.extend(rules.decors.iter().map(|d| decor_image(&d.image)));
    terrain.extend((0..rules.peoples.len()).map(|p| castle_image(rules, p)));
    let sheets = ["Images/Persos/", "Images/Apparences/", "Images/Effets/"];
    let persos = (manifest.entries.iter())
        .filter(|(p, _)| p.ends_with(".png") && sheets.iter().any(|s| p.starts_with(s)))
        .map(|(p, _)| p.clone())
        .collect();
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
/// Mounted, it uses the mount's size and offsets. Red in combat, white when inactive.
/// Inside a building, a rounded frame around it instead.
fn draw_selection(world: &World, p: &Perso, rules: &Rules, cam: (i32, i32)) {
    let colour = if p.fighting() {
        Color::from_rgba(255, 0, 0, 255)
    } else if p.idle() {
        WHITE
    } else {
        Color::from_rgba(0, 0, 192, 255)
    };
    let building = if p.in_own_house {
        world.houses.get(p.numero).map(|m| (m.x, m.y, m.w, m.h))
    } else if p.in_house {
        world.houses.get(p.house_target).map(|m| (m.x, m.y, m.w, m.h))
    } else if p.in_own_castle {
        world.castles.get(p.fief).map(|c| (c.x, c.y, c.w, c.h))
    } else if p.in_castle {
        world.castles.get(p.castle_target).map(|c| (c.x, c.y, c.w, c.h))
    } else {
        None
    };
    if let Some((x, y, w, h)) = building {
        let (x, y) = (x - cam.0, y - cam.1);
        crate::hud::rounded_outline(x, y, x + w, y + h, colour);
        return;
    }
    if !(p.visible() && p.alive()) {
        return;
    }
    let (mut l, mut h) = (p.w as f32, p.h as f32);
    let (mut x, mut y) = ((p.x as i32 - cam.0) as f32, (p.y as i32 - cam.1) as f32);
    if let Some(&o) = p.mount.checked_sub(1).and_then(|m| p.overlays.get(m)) {
        // `ApparenceLargeur/Hauteur`, `ApparenceDecalageX/YTemp`
        let (mw, mh) = rules.objects[o].appearance_size;
        if mw != 0 {
            x -= clng((mw as f64 - l as f64) / 2.0) as f32;
            l = mw as f32;
        }
        if mh != 0 {
            y -= clng((mh as f64 - h as f64) / 2.0) as f32;
            h = mh as f32;
        }
    }
    let (x1, y1) = (x + l / 12.0, y + h - l / 3.0 - h / 8.0);
    let (x2, y2) = (x + l - l / 12.0, y + h + l / 3.0 - h / 6.0);
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

fn draw_persos(world: &World, rules: &Rules, textures: &Textures, cam: (i32, i32), view: (i32, i32)) {
    for p in world.persos.iter().rev().filter(|p| in_view(p.x as i32, p.y as i32, p.w, p.h, cam, view)) {
        // The player's ghost is drawn, translucent: the original's `AfficherFantomes` option,
        // off by default there, but otherwise a dead player sees nothing of themselves.
        if !p.visible() || !(p.alive() || p.player) {
            continue;
        }
        let tint = if p.alive() { WHITE } else { Color::new(1.0, 1.0, 1.0, 0.5) };
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
        draw_look(p, rules, textures, (p.x as i32 - cam.0, p.y as i32 - cam.1), tint);
    }
}

/// `ClsAffApparence.Afficher`: the character's sheets, then the overlays of the objects it
/// wears. A mount is drawn with its own frame size and walk, behind or in front of the rider
/// by direction; the rider then stays on its middle frame, shifted per direction and bobbing.
fn draw_look(p: &Perso, rules: &Rules, textures: &Textures, (x, y): (i32, i32), tint: Color) {
    let overlay = |i: usize| {
        let o = &rules.objects[p.overlays[i]];
        textures.appearance(&o.appearance[p.female as usize]).and_then(|path| textures.get(path))
    };
    let blit = |tex: Option<&Sprite>, x: i32, y: i32, (col, row): (i32, i32), (w, h): (i32, i32)| {
        if let Some(tex) = tex {
            let src = Rect::new((col * w) as f32, (row * h) as f32, w as f32, h as f32);
            tex.draw_ex(x as f32, y as f32, tint, DrawTextureParams { source: Some(src), ..Default::default() });
        }
    };
    let cell = p.sprite_cell();
    let size = (p.w, p.h);
    let Some(mount) = p.mount.checked_sub(1).filter(|&m| m < p.overlays.len()) else {
        for sheet in look_sheets(&p.look) {
            blit(textures.get(sheet), x, y, cell, size);
        }
        for i in 0..p.overlays.len() {
            blit(overlay(i), x, y, cell, size);
        }
        return;
    };
    let o = &rules.objects[p.overlays[mount]];
    let (mw, mh) = (o.appearance_size.0 as i32, o.appearance_size.1 as i32);
    let mount_size = (if mw == 0 { p.w } else { mw }, if mh == 0 { p.h } else { mh });
    // `ApparenceDecalageXTemp`, which the original also adds to Y
    let centre = if mw == 0 { 0 } else { -clng((mw - p.w) as f64 / 2.0) as i32 };
    // direction 0 (still) uses the "down" values
    let d = match p.direction as usize {
        0 => 3,
        d => d,
    };
    let (dx, dy) = o.appearance_offset[d];
    let rider = (x + dx as i32, y + dy as i32 - (p.frame == 1) as i32);
    let rider_cell = if p.direction as usize == 0 { (1, 2) } else { (1, d as i32 - 1) };
    let front = o.appearance_front[d];
    if front {
        blit(overlay(mount), x + centre, y + centre, cell, mount_size);
    }
    for sheet in look_sheets(&p.look) {
        blit(textures.get(sheet), rider.0, rider.1, rider_cell, size);
    }
    for i in 0..p.overlays.len() {
        if i == mount {
            if !front {
                blit(overlay(i), x + centre, y + centre, cell, mount_size);
            }
        } else {
            blit(overlay(i), rider.0, rider.1, rider_cell, size);
        }
    }
}


fn blit(tex: &Sprite, x: i32, y: i32, src: Rect) {
    tex.draw_ex(
        x as f32,
        y as f32,
        WHITE,
        DrawTextureParams { source: Some(src), ..Default::default() },
    );
}

/// Below this many screen pixels per map pixel (from the 3/8 zoom step, where tiles are 6
/// pixels), the map is drawn from an [`Overview`] rather than sprite by sprite.
pub const OVERVIEW_ZOOM: f32 = 0.45;

/// What the map looks like zoomed far out (not in the original, which did not zoom out): one
/// pixel per cell, the terrain in each tile's average colour, with houses, resource nodes and
/// characters (in their fief's colour) stamped on it. Remade once per tick and drawn as one
/// image: tens of thousands of sprites would cover a pixel or two each.
pub struct Overview {
    /// The terrain alone, RGBA.
    terrain: Vec<u8>,
    width: usize,
    height: usize,
    texture: Texture2D,
    tick: Option<u64>,
}

impl Overview {
    pub fn new(world: &World, textures: &Textures) -> Option<Overview> {
        let tiles = textures.pixels(TILESET)?;
        let (cols, rows) = (tiles.width() / CELL as usize, tiles.height() / CELL as usize);
        let mut average = vec![[0u8; 4]; cols * rows];
        for (k, avg) in average.iter_mut().enumerate() {
            let (tx, ty) = (k % cols * CELL as usize, k / cols * CELL as usize);
            let (mut sum, mut n) = ([0u32; 3], 0);
            for y in ty..ty + CELL as usize {
                for x in tx..tx + CELL as usize {
                    let p = &tiles.bytes[(y * tiles.width() + x) * 4..][..4];
                    if p[3] > 0 {
                        (0..3).for_each(|c| sum[c] += p[c] as u32);
                        n += 1;
                    }
                }
            }
            if n > 0 {
                *avg = [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8, 255];
            }
        }
        let t = &world.terrain;
        let mut terrain = Vec::with_capacity(t.width * t.height * 4);
        for k in 0..t.width * t.height {
            let tile = t.category[k] as usize * cols + t.shape[k] as usize;
            terrain.extend(average.get(tile).copied().unwrap_or([0, 0, 0, 255]));
        }
        let texture = Texture2D::from_rgba8(t.width as u16, t.height as u16, &terrain);
        texture.set_filter(FilterMode::Nearest);
        Some(Overview { terrain, width: t.width, height: t.height, texture, tick: None })
    }

    /// Stamps what is on the map, once per tick.
    pub fn update(&mut self, world: &World) {
        if self.tick == Some(world.tick) {
            return;
        }
        self.tick = Some(world.tick);
        let mut bytes = self.terrain.clone();
        let (w, h) = (self.width as i32, self.height as i32);
        let mut fill = |x: i32, y: i32, bw: i32, bh: i32, colour: [u8; 4]| {
            for cy in (y / CELL).max(0)..((y + bh.max(1) - 1) / CELL + 1).min(h) {
                for cx in (x / CELL).max(0)..((x + bw.max(1) - 1) / CELL + 1).min(w) {
                    let k = (cy * w + cx) as usize * 4;
                    bytes[k..k + 4].copy_from_slice(&colour);
                }
            }
        };
        for r in &world.resources {
            fill(r.x, r.y, RESOURCE_SIZE, RESOURCE_SIZE, [96, 96, 96, 255]);
        }
        for m in world.houses.iter().filter(|m| m.visible()) {
            fill(m.x, m.y, m.w, m.h, [120, 84, 48, 255]);
        }
        for p in world.persos.iter().filter(|p| p.visible() && p.alive()) {
            fill(p.x as i32 + p.w / 2, p.y as i32 + p.h / 2, 1, 1, fief_colour(p.fief));
        }
        self.texture.update_from_bytes(self.width as u32, self.height as u32, &bytes);
    }
}

/// A bright colour per fief, the hues spread by the golden ratio.
fn fief_colour(fief: usize) -> [u8; 4] {
    let c = macroquad::color::hsl_to_rgb((fief as f32 * 0.618_034).fract(), 1.0, 0.5);
    [(c.r * 255.0) as u8, (c.g * 255.0) as u8, (c.b * 255.0) as u8, 255]
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
/// Whether a box of the map, give or take what sticks out of it (bars, mounts, worn objects),
/// is in the view at `cam` of size `view`: what is not is skipped before any drawing work.
fn in_view(x: i32, y: i32, w: i32, h: i32, cam: (i32, i32), view: (i32, i32)) -> bool {
    const MARGIN: i32 = 64;
    x + w + MARGIN > cam.0 && y + h + MARGIN > cam.1 && x - MARGIN < cam.0 + view.0 && y - MARGIN < cam.1 + view.1
}

/// `overview`: draw the map from it ([`Overview`]), with only the castles and a marker on the
/// player as sprites.
pub fn draw_world(world: &World, rules: &Rules, textures: &Textures, cam: (i32, i32), player: usize, view: (i32, i32), overview: Option<&Overview>) {
    if let Some(o) = overview {
        let size = vec2((world.terrain.width as i32 * CELL) as f32, (world.terrain.height as i32 * CELL) as f32);
        draw_texture_ex(&o.texture, -cam.0 as f32, -cam.1 as f32, WHITE, DrawTextureParams { dest_size: Some(size), ..Default::default() });
        draw_castles(world, rules, textures, cam, view);
        if let Some(p) = world.persos.get(player) {
            // a ring around the player, a hundredth of the view wide
            let r = view.0.max(view.1) as f32 / 100.0;
            let (x, y) = ((p.x as i32 - cam.0) as f32 + p.w as f32 / 2.0, (p.y as i32 - cam.1) as f32 + p.h as f32 / 2.0);
            draw_circle_lines(x, y, r, r / 4.0, WHITE);
        }
        return;
    }
    draw_terrain(world, textures, cam, view);
    if let Some(tex) = textures.get(RESOURCES) {
        let s = RESOURCE_SIZE as f32;
        for r in world.resources.iter().filter(|r| in_view(r.x, r.y, RESOURCE_SIZE, RESOURCE_SIZE, cam, view)) {
            blit(tex, r.x - cam.0, r.y - cam.1, Rect::new(r.kind as f32 * s, 0.0, s, s));
        }
    }
    // Afficher_Batiments: houses, then castles.
    let houses = || world.houses.iter().filter(|m| m.visible() && in_view(m.x, m.y, m.w, m.h, cam, view));
    for m in houses() {
        let Some(tex) = textures.get(&format!("{}.bmp", rules.buildings[m.kind].image)) else { continue };
        let (w, h) = (m.w as f32, m.h as f32);
        let src = Rect::new(m.animation as f32 * w, m.state(rules) as f32 * h, w, h);
        blit(tex, m.x - cam.0, m.y - cam.1, src);
    }
    // Life bars of damaged houses (drawn after all houses here; they never overlap a house).
    for m in houses() {
        let v = m.life_with_repair();
        if v < m.max_life as f64 {
            status_bar(v, v, m.max_life as f64, m.x - cam.0, m.y - cam.1 + m.h, m.w, 5, None);
        }
    }
    draw_castles(world, rules, textures, cam, view);
    for d in world.decors.iter().filter(|d| in_view(d.x, d.y, d.w, d.h, cam, view)) {
        let Some(tex) = textures.get(&decor_image(&rules.decors[d.kind].image)) else { continue };
        let src = Rect::new(d.frame as f32 * d.w as f32, 0.0, d.w as f32, d.h as f32);
        blit(tex, d.x - cam.0, d.y - cam.1, src);
    }
    if let Some(tex) = textures.get(TREASURES) {
        let s = TREASURE_SIZE as f32;
        for t in world.treasures.iter().filter(|t| in_view(t.x, t.y, TREASURE_SIZE, TREASURE_SIZE, cam, view)) {
            blit(tex, t.x - cam.0, t.y - cam.1, Rect::new(t.tier as f32 * s, 0.0, s, s));
        }
    }
    if let Some(p) = world.persos.get(player) {
        draw_selection(world, p, rules, cam);
    }
    draw_work_state(world, rules, player, cam);
    draw_persos(world, rules, textures, cam, view);
}

/// Castles, with their life and their next age's progress.
fn draw_castles(world: &World, rules: &Rules, textures: &Textures, cam: (i32, i32), view: (i32, i32)) {
    for (fief, c) in world.fiefs.iter().zip(&world.castles) {
        if !c.visible() || !in_view(c.x, c.y, c.w, c.h, cam, view) {
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
}
