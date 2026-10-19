mod assets;
mod hud;
mod layout;
mod loading;
mod music;
mod render;
mod storage;
mod text;

use macroquad::prelude::*;
use layout::{MIN_H, MIN_W};
use macroquad::miniquad::window::dpi_scale as screen_dpi_scale;
use macroquad::experimental::coroutines::start_coroutine;
use sim::files::Files;
use sim::rules::Rules;
use sim::world::{Order, World, WorldSetup};

fn window_conf() -> Conf {
    Conf {
        window_title: "Vendetta".to_owned(),
        window_width: 1024,
        window_height: 768,
        // Draw at the screen's physical resolution; the scales include the pixel ratio.
        high_dpi: true,
        ..Default::default()
    }
}

/// Map zooms and interface scales the mouse wheel steps through.
const ZOOMS: [f32; 7] = [0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 4.0];
const UI_SCALES: [f32; 8] = [0.75, 1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0];

/// The next value of `steps` after `v`, upward or downward.
fn step(steps: &[f32], v: f32, up: bool) -> f32 {
    if up {
        steps.iter().copied().find(|&s| s > v + 1e-3).unwrap_or(v)
    } else {
        steps.iter().rev().copied().find(|&s| s < v - 1e-3).unwrap_or(v)
    }
}

/// Map zoom and interface scale, kept across sessions. Both are relative to the screen's
/// pixel ratio, so 1 looks the same size on any display.
struct Scales {
    zoom: f32,
    ui: f32,
}

impl Scales {
    fn load() -> Scales {
        let saved = storage::load("settings").and_then(|b| String::from_utf8(b).ok()).and_then(|text| {
            let mut v = text.split_whitespace().map(|x| x.parse::<f32>().ok());
            Some(Scales { zoom: v.next()??, ui: v.next()?? })
        });
        // By default the interface fits the original's minimum screen of 800×600.
        saved.unwrap_or_else(|| {
            Scales { zoom: 1.0, ui: (screen_width() / MIN_W).min(screen_height() / MIN_H).min(1.0) }
        })
    }

    /// Screen pixels per world pixel, and per interface pixel.
    fn effective(&self) -> (f32, f32) {
        (self.zoom * screen_dpi_scale(), self.ui * screen_dpi_scale())
    }

    fn store(&self) {
        storage::store("settings", format!("{} {}", self.zoom, self.ui).as_bytes());
    }
}

/// Draws the frame (in physical pixels) to the window (in CSS pixels).
fn present(target: &RenderTarget) {
    set_default_camera();
    clear_background(BLACK);
    let size = vec2(screen_width(), screen_height());
    draw_texture_ex(&target.texture, 0.0, 0.0, WHITE, DrawTextureParams { dest_size: Some(size), flip_y: true, ..Default::default() });
}

/// A camera drawing `size` units over the whole render target (fractional, so drawing and
/// mouse use exactly the same scale).
fn camera(target: &RenderTarget, size: (f32, f32)) -> Camera2D {
    let mut c = Camera2D::from_display_rect(Rect::new(0.0, 0.0, size.0, size.1));
    c.render_target = Some(target.clone());
    c
}

/// `Scrolling_Immediat`: centre the player's top-left corner, clamped to the map. `view` is
/// the visible area in world pixels. Zoomed out beyond the map's size, the player simply stays
/// centred on that axis instead of the map sticking to the top-left corner.
fn follow(world: &World, player: usize, view: (i32, i32)) -> (i32, i32) {
    let p = &world.persos[player];
    let axis = |pos: i64, view: i32, size: usize| {
        let centred = pos as i32 - view / 2;
        let max = size as i32 * 16 - view;
        if max < 0 { centred } else { centred.min(max).max(0) }
    };
    (axis(p.x, view.0, world.terrain.width), axis(p.y, view.1, world.terrain.height))
}

const TICKS_PER_SECOND: f64 = 10.0; // Jeu.ini Vitesse

/// How often the game in progress is saved, in seconds.
const AUTOSAVE_INTERVAL: f64 = 5.0;

struct Loaded {
    music: music::Music,
    files: Files,
    rules: Rules,
    world: World,
    textures: assets::Textures,
}

/// Downloads the data and images and creates the world, reporting progress.
async fn load_game(progress: loading::Progress) -> Result<Loaded, String> {
    let manifest = assets::Manifest::load().await?;
    progress.load_decoration(&manifest, "Francais").await;
    progress.set_total(manifest.data_bytes() + manifest.image_estimate());
    let files = assets::load_files(&manifest, &progress).await?;
    let rules = Rules::load(&files, "Francais")?;
    progress.message(&progress.label(2));
    let setup = WorldSetup { seed: (macroquad::miniquad::date::now() * 1000.0) as u64 as u32, ..Default::default() };
    // Resume the saved game if there is one that fits these rules.
    let saved = storage::load("save").and_then(|bytes| match World::load(&rules, &bytes) {
        Ok(world) => Some(world),
        Err(e) => {
            warn!("discarding the saved game: {e}");
            None
        }
    });
    let world = match saved {
        Some(world) => world,
        None => World::create(&rules, &files, &setup)?,
    };
    let mut textures = assets::Textures::new(&manifest);
    let (terrain, persos) = render::images(&rules, &world);
    let hud = hud::images(&rules);
    let all: Vec<String> = terrain.iter().chain(&persos).chain(&hud).cloned().collect();
    let images: u64 = textures.missing(&all).iter().map(|p| manifest.size(p)).sum();
    progress.set_total(manifest.data_bytes() + images);
    textures.load_all(&manifest, &terrain, &progress, &progress.label(7)).await;
    textures.load_all(&manifest, &persos, &progress, &progress.label(6)).await;
    textures.load_all(&manifest, &hud, &progress, &progress.label(10)).await;
    Ok(Loaded { music: music::Music::new(&manifest), files, rules, world, textures })
}

#[macroquad::main(window_conf)]
async fn main() {
    show_mouse(false);
    let screen = loading::LoadingScreen::new();
    let task = start_coroutine(load_game(screen.progress.clone()));
    while !task.is_done() {
        screen.draw();
        next_frame().await;
    }
    let Loaded { mut music, files, rules, mut world, textures } = match task.retrieve().unwrap() {
        Ok(l) => {
            screen.finish();
            l
        }
        Err(e) => loop {
            screen.progress.message(&format!("&Erreur#{e}"));
            screen.draw();
            next_frame().await;
        },
    };
    let player = world.persos.iter().position(|p| p.player).unwrap();
    // The game starts with a "Debut" track (`FrmMoteur2D.frm:405`).
    music.play(music::DEBUT, false);
    let mut last_save = get_time();
    let mut hud = hud::Hud::new(hud::Strings::load(&files, "Francais"), hud::WindowStrings::load(&files, "Francais"), hud::TooltipLabels::load(&files, "Francais"), rules.objects.len());


    let mut target = render_target(1, 1);
    let mut scales = Scales::load();
    let mut view = (0, 0);
    let mut cam = (0, 0);
    let mut orders = Vec::new();
    let mut last_tick = get_time();
    loop {
        // The frame covers the window; the map and the interface each have their own scale.
        // `screen_width()` and the mouse are in CSS pixels; the frame is in physical pixels.
        let dpi = screen_dpi_scale();
        let (w, h) = ((screen_width() * dpi) as i32, (screen_height() * dpi) as i32);
        if (w, h) != (target.texture.width() as i32, target.texture.height() as i32) {
            target = render_target(w as u32, h as u32);
            target.texture.set_filter(FilterMode::Nearest);
        }
        let (zoom, ui) = scales.effective();
        let view_f = (w as f32 / zoom, h as f32 / zoom);
        let new_view = (view_f.0.ceil() as i32, view_f.1.ceil() as i32);
        if new_view != view {
            view = new_view;
            cam = follow(&world, player, view);
        }
        let hud_f = (w as f32 / ui, h as f32 / ui);
        let hud_size = (hud_f.0 as i32, hud_f.1 as i32);
        layout::set_screen(hud_size.0, hud_size.1);
        hud.set_scale(ui);

        let (mx, my) = mouse_position();
        let (mx, my) = (mx * dpi, my * dpi);
        // The interface's mouse (`SourisX/Y`), clamped to the screen like the original.
        let (hx, hy) = (((mx / ui) as i32).clamp(0, hud_size.0), ((my / ui) as i32).clamp(0, hud_size.1));
        let input = hud::Input::poll((hx, hy));
        // What is under the cursor on the map (`Obstacle` on the cursor's top-left).
        let to_world = |p: f32, c: i32| c + (p / zoom).floor() as i32;
        let (wx, wy) = (to_world(mx, cam.0), to_world(my, cam.1));
        let (hover, hit) = world.obstacle_at(&rules, player, wx, wy, 0, 0);
        // The cursor image is drawn at the interface's scale: its centre on the map.
        let centre = (to_world(mx + 12.0 * ui, cam.0), to_world(my + 12.0 * ui, cam.1));

        set_camera(&camera(&target, view_f));
        clear_background(BLACK);
        render::draw_world(&world, &rules, &textures, cam, player, view);
        let placement = hud.draw_placement(&world, &rules, player, centre, cam);
        set_camera(&camera(&target, hud_f));
        let radial_was_open = hud.radial_open();
        let on_ui = hud.frame(&world, &rules, &textures, player, &input, &mut orders);
        let placed = !on_ui && hud.map_input(&world, &rules, player, &input, centre, &mut orders);
        let holding = world.persos[player].selected_object >= 0;
        if !on_ui && !placed && input.right_pressed && holding {
            // Right click while holding an object puts it back.
            orders.push(Order::Unselect { perso: player });
        } else if !on_ui && !placed && input.right_pressed {
            // Right click on the map: the radial buttons (`FrmMoteur2D.frm:943-961`).
            match hover {
                0 | 16 | 999 => hud.open_radial((hx, hy), 4),
                1 => hud.open_radial((hx, hy), 1),
                4..=8 | 12 => hud.open_radial((hx, hy), 0),
                // TODO: 9-11 "Chargez!" (army attack)
                _ => {}
            }
        }
        if !on_ui && !placed && input.left_released {
            // Left-release on the map (`FrmMoteur2D.frm:828-916`).
            let p = &world.persos[player];
            let perso = player;
            let held = p.selected_object;
            orders.push(match hover {
                // With an object on the cursor: sell it at a castle, drop it on the ground.
                6 if held >= 0 => Order::GoSell { perso, target: p.fief, market: false },
                8 if held >= 0 => Order::GoSell { perso, target: hit, market: false },
                0 | 16 | 999 if held >= 0 => Order::DropObject { perso, object: held as usize, from_inventory: false },
                1 => Order::Harvest { perso, node: hit },
                4 => Order::GoHome { perso, enter: false },
                5 => Order::GoHome { perso, enter: true },
                6 => Order::GoOwnCastle { perso },
                7 => Order::GoToHouse { perso, house: hit, repair: false },
                8 => Order::GoToCastle { perso, castle: hit },
                13 => Order::GoToTreasure { perso, treasure: hit },
                // TODO: 9/10/11 attack once combat exists
                _ => Order::Move {
                    perso,
                    x: wx as i64 - p.w as i64 / 2,
                    y: wy as i64 - p.h as i64 / 2,
                },
            });
        }
        hud.overlay(&world, &rules, &textures, player, &input, placement, (!on_ui).then_some((hover, hit)));
        present(&target);

        // The wheel: scrolls the window lists under it (done by the HUD), rotates the radial
        // buttons, zooms the map, or scales the rest of the interface.
        let wheel = mouse_wheel().1;
        if wheel != 0.0 && !radial_was_open {
            let up = wheel > 0.0;
            match hud.over_window(&world, &rules, player, (hx, hy)) {
                // Over a status bar the wheel sets the restoring threshold (done by the HUD).
                _ if hud.over_status_bar((hx, hy)) => {}
                Some(true) => {}
                Some(false) => scales.ui = step(&UI_SCALES, scales.ui, up),
                None if on_ui => scales.ui = step(&UI_SCALES, scales.ui, up),
                None => scales.zoom = step(&ZOOMS, scales.zoom, up),
            }
            scales.store();
        }

        // One tick whenever a tick's worth of time has passed; extra time is dropped.
        if !hud.paused() && get_time() - last_tick > 1.0 / TICKS_PER_SECOND {
            last_tick = get_time();
            world.step(&rules, &orders);
            orders.clear();
            hud.automatic_actions(&world, player, &mut orders);
            hud.craft_from_queue(&world, &rules, player, &mut orders);
            cam = follow(&world, player, view);
        }
        if get_time() - last_save > AUTOSAVE_INTERVAL {
            last_save = get_time();
            storage::store("save", &world.save());
        }
        // Ctrl+N: a new game, until the setup screens exist.
        if (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl)) && is_key_pressed(KeyCode::N) {
            storage::remove("save");
            storage::restart();
        }
        // The mood's music, every frame (`FrmMoteur2D.frm:426-438`).
        let p = &world.persos[player];
        if world.victory {
            music.play(music::VICTOIRE, false);
        } else if (300..400).contains(&p.action) {
            music.play(if p.action == 302 { music::BOSS } else { music::COMBAT }, false);
        } else if !p.alive() {
            music.play(music::MORT, false);
        } else if p.emperor {
            music.play(music::EMPEREUR, true);
        } else {
            music.play(music::VILLE, true);
        }
        music.update();
        next_frame().await;
    }
}
