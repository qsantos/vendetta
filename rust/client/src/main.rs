mod assets;
mod hud;
mod layout;
mod loading;
mod render;
mod storage;
mod text;

use macroquad::prelude::*;
use layout::{MIN_H, MIN_W, screen_h, screen_w};
use macroquad::experimental::coroutines::start_coroutine;
use sim::files::Files;
use sim::rules::Rules;
use sim::world::{Order, World, WorldSetup};

fn window_conf() -> Conf {
    Conf {
        window_title: "Vendetta".to_owned(),
        window_width: 1024,
        window_height: 768,
        high_dpi: false,
        ..Default::default()
    }
}

/// Scale from game pixels to window pixels: 1, unless the window is smaller than the
/// minimum logical screen.
fn scale() -> f32 {
    (screen_width() / MIN_W).min(screen_height() / MIN_H).min(1.0)
}

/// Draws the game frame to the window.
fn present(target: &RenderTarget) {
    set_default_camera();
    clear_background(BLACK);
    let s = scale();
    draw_texture_ex(
        &target.texture,
        0.0,
        0.0,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(screen_w() as f32 * s, screen_h() as f32 * s)),
            flip_y: true,
            ..Default::default()
        },
    );
}

/// Mouse position in game pixels (`SourisX/Y`), clamped to the screen like the original.
fn mouse_in_frame() -> (i32, i32) {
    let (mx, my) = mouse_position();
    let s = scale();
    (((mx / s) as i32).clamp(0, screen_w()), ((my / s) as i32).clamp(0, screen_h()))
}

/// `Scrolling_Immediat`: centre the player's top-left corner, clamped to the map.
fn follow(world: &World, player: usize) -> (i32, i32) {
    let p = &world.persos[player];
    let max_x = world.terrain.width as i32 * 16 - screen_w();
    let max_y = world.terrain.height as i32 * 16 - screen_h();
    let x = (p.x as i32 - screen_w() / 2).min(max_x).max(0);
    let y = (p.y as i32 - screen_h() / 2).min(max_y).max(0);
    (x, y)
}

const TICKS_PER_SECOND: f64 = 10.0; // Jeu.ini Vitesse

/// How often the game in progress is saved, in seconds.
const AUTOSAVE_INTERVAL: f64 = 5.0;

struct Loaded {
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
    let setup = WorldSetup { seed: (macroquad::miniquad::date::now() * 1000.0) as u32, ..Default::default() };
    // Resume the saved game if there is one that fits these rules.
    let saved = storage::load().and_then(|bytes| match World::load(&rules, &bytes) {
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
    Ok(Loaded { files, rules, world, textures })
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
    let Loaded { files, rules, mut world, textures } = match task.retrieve().unwrap() {
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
    let mut last_save = get_time();
    let mut hud = hud::Hud::new(hud::Strings::load(&files, "Francais"), hud::WindowStrings::load(&files, "Francais"));


    let mut target = render_target(1, 1);
    let mut frame_cam = Camera2D::default();

    let mut cam = follow(&world, player);
    let mut orders = Vec::new();
    let mut last_tick = get_time();
    loop {
        // The game screen follows the window size.
        let s = scale();
        let (w, h) = ((screen_width() / s) as i32, (screen_height() / s) as i32);
        if (w, h) != (target.texture.width() as i32, target.texture.height() as i32) {
            layout::set_screen(w, h);
            target = render_target(w as u32, h as u32);
            target.texture.set_filter(FilterMode::Nearest);
            frame_cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, w as f32, h as f32));
            frame_cam.render_target = Some(target.clone());
            cam = follow(&world, player);
        }
        let (mx, my) = mouse_in_frame();
        let input = hud::Input::poll((mx, my));
        // What is under the cursor (`Obstacle` on the cursor's top-left).
        let (wx, wy) = (mx + cam.0, my + cam.1);
        let (hover, hit) = world.obstacle_at(&rules, player, wx, wy, 0, 0);

        set_camera(&frame_cam);
        clear_background(BLACK);
        render::draw_world(&world, &rules, &textures, cam, player);
        let on_ui = hud.frame(&world, &rules, &textures, player, &input, &mut orders);
        let placed = !on_ui && hud.map_input(&world, &rules, player, &input, cam, &mut orders);
        if !on_ui && !placed && input.right_pressed {
            // Right click on the map: the radial buttons (`FrmMoteur2D.frm:943-961`).
            match hover {
                0 | 16 | 999 => hud.open_radial((mx, my), 4),
                1 => hud.open_radial((mx, my), 1),
                4..=8 | 12 => hud.open_radial((mx, my), 0),
                // TODO: 9-11 "Chargez!" (army attack)
                _ => {}
            }
        }
        if !on_ui && !placed && input.left_released {
            // Left-release on the map (`FrmMoteur2D.frm:828-916`).
            let p = &world.persos[player];
            let perso = player;
            orders.push(match hover {
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
        hud.overlay(&world, &rules, &textures, player, &input, cam, (!on_ui).then_some((hover, hit)));
        present(&target);

        // One tick whenever a tick's worth of time has passed; extra time is dropped.
        if !hud.paused() && get_time() - last_tick > 1.0 / TICKS_PER_SECOND {
            last_tick = get_time();
            world.step(&rules, &orders);
            orders.clear();
            cam = follow(&world, player);
        }
        if get_time() - last_save > AUTOSAVE_INTERVAL {
            last_save = get_time();
            storage::store(&world.save());
        }
        // Ctrl+N: a new game, until the setup screens exist.
        if (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl)) && is_key_pressed(KeyCode::N) {
            storage::clear();
            storage::restart();
        }
        next_frame().await;
    }
}
