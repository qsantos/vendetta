mod assets;
mod atlas;
mod hud;
mod layout;
mod effects;
mod loading;
mod music;
mod options;
mod perf;
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
/// Zooming out stops once the whole map is in view.
const ZOOMS: [f32; 14] = [1.0 / 64.0, 1.0 / 32.0, 1.0 / 16.0, 0.125, 0.1875, 0.25, 0.375, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 4.0];
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

/// A camera drawing `size` units over the whole window (fractional, so drawing and mouse use
/// exactly the same scale). The window is drawn to directly, in physical pixels.
fn camera(size: (f32, f32)) -> Camera2D {
    let mut c = Camera2D::from_display_rect(Rect::new(0.0, 0.0, size.0, size.1));
    // `from_display_rect` is upside down on the window (it suits render targets).
    c.zoom.y = -c.zoom.y;
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

/// `TypeCurseur`: what the cursor's top-left (x, y) on the map is over, and its index
/// (`IndiceCible`), see `FrmMoteur2D.frm:1393-1460`. A ghost can only go to its castle, its
/// temples and graves (`ObstacleMort`); carrying something, the cursor gives it to allies or
/// sells it (`ObstacleAmi`).
fn cursor_type(world: &World, rules: &Rules, player: usize, x: i32, y: i32) -> (i32, usize) {
    let p = &world.persos[player];
    let carrying = p.holding();
    if !p.alive() {
        return if carrying { (0, 0) } else { world.obstacle_dead(player, x, y) };
    }
    let giveable = p.resources_selected || p.selected_money > 0.0 || !rules.objects.get(p.selected_object as usize).is_some_and(|o| o.inseparable);
    let (cursor, hit) = if carrying {
        match world.obstacle_ally(player, x, y) {
            (1, j) if giveable => (14, j),
            // An allied market buys it.
            (2, j) if world.houses[j].market && world.houses[j].built => (6, j),
            (2, j) if giveable => (14, j),
            (3, j) => (6, j),
            (4, j) => (8, j),
            _ => (0, 0),
        }
    } else {
        // Armies are not ported: with one selected, an ally would give 17 (guard).
        world.obstacle_at(rules, player, x, y, 0, 0)
    };
    // Ctrl enters the finished home without repairing it, and repairs a damaged allied house.
    let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
    match cursor {
        4 if ctrl && world.houses[hit].built => (5, hit),
        7 if ctrl && world.houses[hit].life < world.houses[hit].max_life as f64 => (4, hit),
        c => (c, hit),
    }
}

const TICKS_PER_SECOND: f64 = 10.0; // Jeu.ini Vitesse

/// How often the game in progress is saved, in seconds.
const AUTOSAVE_INTERVAL: f64 = 5.0;
/// Ticks a frame may run to catch up with the clock.
const MAX_TICKS_PER_FRAME: u32 = 5;

struct Loaded {
    god: bool,
    test_effects: Vec<(usize, Option<i64>)>,
    test_recharges: Vec<(usize, i64)>,
    test_emperor: bool,
    test_hold: Option<usize>,
    test_advance: Option<f64>,
    show_perf: bool,
    music: music::Music,
    effects: effects::Effects,
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
    let options = options::Options::load();
    let mut setup = WorldSetup { seed: (macroquad::miniquad::date::now() * 1000.0) as u64 as u32, ..Default::default() };
    setup.roads = if options.ring_roads { sim::world::Roads::Ring } else { sim::world::Roads::Nearest };
    if let Some(seed) = options.seed {
        setup.seed = seed;
    }
    if let Some((w, h)) = options.size {
        // as many resource nodes per cell as the default 128×128 world
        setup.resources = setup.resources * w * h / (setup.width * setup.height);
        (setup.width, setup.height) = (w, h);
    }
    if options.fiefs.is_some() || options.villagers.is_some() {
        // The default fiefs keep their people; extra ones get a random one, like the original's
        // "Aléatoire", each fief its own team.
        let mut rng = sim::vb::Rnd::new(setup.seed);
        let citizens = options.villagers.unwrap_or(setup.fiefs[0].citizens);
        setup.fiefs = (0..options.fiefs.unwrap_or(setup.fiefs.len()))
            .map(|i| {
                let random = (rng.next() * rules.peoples.len() as f64) as usize % rules.peoples.len();
                let people = setup.fiefs.get(i).map_or(random, |f| f.people);
                sim::world::FiefSetup { people, team: i as i64 + 1, epoch: 0, citizens }
            })
            .collect();
    }
    // A saved game is resumed only with the setup options it was made with.
    let same_setup = !options.new && storage::load("setup").is_some_and(|k| k == options.setup_key().as_bytes());
    // Resume the saved game if there is one that fits these rules.
    let saved = storage::load("save").filter(|_| same_setup).and_then(|bytes| match World::load(&rules, &bytes) {
        Ok(world) => Some(world),
        Err(e) => {
            warn!("discarding the saved game: {e}");
            None
        }
    });
    let world = match saved {
        Some(world) => world,
        None => {
            storage::store("setup", options.setup_key().as_bytes());
            World::create(&rules, &files, &setup)?
        }
    };
    let mut textures = assets::Textures::new(&manifest);
    let (terrain, persos) = render::images(&rules, &manifest);
    let hud = hud::images(&rules);
    let all: Vec<String> = terrain.iter().chain(&persos).chain(&hud).cloned().collect();
    let images: u64 = textures.missing(&all).iter().map(|p| manifest.size(p)).sum();
    progress.set_total(manifest.data_bytes() + images);
    textures.load_all(&manifest, &terrain, &progress, &progress.label(7)).await;
    textures.load_all(&manifest, &persos, &progress, &progress.label(6)).await;
    textures.load_all(&manifest, &hud, &progress, &progress.label(10)).await;
    let effects = effects::Effects::new(&files).await;
    Ok(Loaded { god: options.god, test_effects: options.effects, test_recharges: options.recharges, test_emperor: options.emperor, test_hold: options.hold, test_advance: options.advance, show_perf: options.perf, music: music::Music::new(&manifest), effects, files, rules, world, textures })
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
    let Loaded { god, test_effects, test_recharges, test_emperor, test_hold, test_advance, show_perf, mut music, mut effects, files, rules, mut world, textures } = match task.retrieve().unwrap() {
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
    if god {
        world.god = Some(player);
    }
    let fief = world.persos[player].fief;
    if let Some(object) = test_hold.filter(|&o| o < rules.objects.len()) {
        world.persos[player].selected_object = object as i64;
    }
    if test_emperor {
        world.change_emperor(&rules, fief, player as i64, false);
    }
    if let Some(pct) = test_advance {
        let speed = world.epoch_speed;
        world.fiefs[fief].change_epoch(&rules, &mut world.castles[fief], speed);
        let f = &mut world.fiefs[fief];
        f.epoch_timer = (f.epoch_timer as f64 * (1.0 - pct / 100.0)) as i64;
    }
    for (object, left) in test_effects.into_iter().filter(|&(o, _)| o < rules.objects.len()) {
        let p = &mut world.persos[player];
        let d = rules.objects[object].duration;
        p.add_effect(&rules, object, d, d, None);
        if let (Some(left), Some(e)) = (left, p.effects.last_mut()) {
            e.duration = left;
        }
    }
    // The game starts with a "Debut" track (`FrmMoteur2D.frm:405`).
    music.play(music::DEBUT, false);
    let mut last_save = get_time();
    let mut save_size = 0;
    let mut hud = hud::Hud::new(hud::Strings::load(&files, "Francais"), hud::WindowStrings::load(&files, "Francais"), hud::TooltipLabels::load(&files, "Francais"), &files, rules.objects.len());
    for (i, (object, left)) in test_recharges.into_iter().filter(|&(o, _)| o < rules.objects.len()).enumerate() {
        let p = &mut world.persos[player];
        p.add_object(&rules, object);
        p.add_recharge(object, left);
        hud.bind_object(object, i as i32 + 1);
    }


    // `Afficher_Conseil`: a tip to start with (not in the still scenes of the rendering tests).
    if !options::test_mode() {
        rand::srand((macroquad::miniquad::date::now() * 1000.0) as u64);
        hud.add_tip(rand::gen_range(0.0, 1.0));
    }
    let mut scales = Scales::load();
    let mut view = (0, 0);
    let mut cam = (0, 0);
    let mut orders = Vec::new();
    // When the next tick is due.
    let mut next_tick = get_time();
    // Made the first time the map is zoomed far out.
    let mut overview: Option<render::Overview> = None;
    // `CliqueSurTerrain`: the left button went down on the map, so its release acts there.
    let mut pressed_on_map = false;
    let mut perf = perf::Perf::default();
    loop {
        let frame_start = perf::now();
        perf.frame();
        // The frame covers the window; the map and the interface each have their own scale.
        // `screen_width()` and the mouse are in CSS pixels; the frame is in physical pixels.
        let dpi = screen_dpi_scale();
        let (w, h) = ((screen_width() * dpi) as i32, (screen_height() * dpi) as i32);
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

        if let Some(c) = hud.camera() {
            cam = c;
        }
        hud.set_view(cam, view);
        let (mx, my) = mouse_position();
        let (mx, my) = (mx * dpi, my * dpi);
        // The interface's mouse (`SourisX/Y`), clamped to the screen like the original.
        let (hx, hy) = (((mx / ui) as i32).clamp(0, hud_size.0), ((my / ui) as i32).clamp(0, hud_size.1));
        let input = hud::Input::poll((hx, hy));
        // What is under the cursor on the map (`Obstacle` on the cursor's top-left).
        let to_world = |p: f32, c: i32| c + (p / zoom).floor() as i32;
        let (wx, wy) = (to_world(mx, cam.0), to_world(my, cam.1));
        let (hover, hit) = cursor_type(&world, &rules, player, wx, wy);
        // The cursor image is drawn at the interface's scale: its centre on the map.
        let centre = (to_world(mx + 12.0 * ui, cam.0), to_world(my + 12.0 * ui, cam.1));

        set_camera(&camera(view_f));
        clear_background(BLACK);
        let map_start = perf::now();
        let overview = if zoom < render::OVERVIEW_ZOOM {
            if overview.is_none() {
                overview = render::Overview::new(&world, &textures);
            }
            overview.as_mut().map(|o| {
                o.update(&world);
                &*o
            })
        } else {
            None
        };
        // Zoomed far out, effects, numbers and speech would be illegible clutter.
        let far = overview.is_some();
        render::draw_world(&world, &rules, &textures, cam, player, view, overview);
        if !far {
            effects.draw(&textures, cam, view, zoom);
        }
        let placement = hud.draw_placement(&world, &rules, player, centre, cam);
        // (changing the camera submits the map's drawings)
        set_camera(&camera(hud_f));
        let hud_start = perf::now();
        perf.add(perf::Part::Map, hud_start - map_start);
        if !far {
            hud.draw_comments(&world, |x, y| ((((x as i32 - cam.0) as f32) * zoom / ui) as i32, (((y as i32 - cam.1) as f32) * zoom / ui) as i32));
        }
        let radial_was_open = hud.radial_open();
        let on_ui = hud.frame(&world, &rules, &textures, player, &input, &mut orders);
        if input.left_pressed {
            pressed_on_map = !on_ui;
        }
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
                // "Chargez!": the soldiers attack the enemy, house or castle.
                9..=11 => orders.push(Order::Charge {
                    perso: player,
                    kind: (hover - 9) as i64,
                    target: hit as i64,
                    ctrl: is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl),
                    shift: is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift),
                }),
                _ => {}
            }
        }
        if !on_ui && !placed && input.left_released && pressed_on_map {
            // Left-release on the map (`FrmMoteur2D.frm:828-916`).
            let p = &world.persos[player];
            let perso = player;
            let held = p.selected_object;
            let carrying = p.holding() && p.alive();
            match hover {
                // Carrying something: sell it in a castle or a market, else drop it, or put
                // the held object into the own house.
                0 | 16 | 999 if carrying => {
                    let market = world.houses.get(p.house_target).is_some_and(|m| m.market && m.built);
                    if p.in_own_castle || p.in_castle || p.in_house && market {
                        orders.push(Order::SellSelection { perso, fief: p.castle_target });
                    } else if held >= 0 && !rules.objects[held as usize].inseparable {
                        let object = held as usize;
                        orders.push(if p.in_own_house {
                            Order::DepositObject { perso, object, from_inventory: false }
                        } else {
                            Order::DropObject { perso, object, from_inventory: false }
                        });
                        orders.push(Order::SelectObject { perso, object: -1 });
                    } else {
                        orders.push(Order::DropSelection { perso });
                    }
                }
                // Quirk: the market test reads the house of the castle's index too.
                6 if carrying && world.houses.get(hit).is_some_and(|m| m.market && m.built) => {
                    orders.push(Order::GoSell { perso, target: hit, market: true })
                }
                6 if carrying => orders.push(Order::GoSell { perso, target: p.fief, market: false }),
                // The original reads a stale target here; the castle is meant.
                8 if carrying => orders.push(Order::GoSell { perso, target: hit, market: false }),
                1 => orders.push(Order::Harvest { perso, node: hit }),
                4 if hit == perso => orders.push(Order::GoHome { perso, enter: false }),
                4 => orders.push(Order::GoToHouse { perso, house: hit, repair: true }),
                5 => orders.push(Order::GoHome { perso, enter: true }),
                6 => orders.push(Order::GoOwnCastle { perso }),
                7 | 12 => orders.push(Order::GoToHouse { perso, house: hit, repair: false }),
                8 => orders.push(Order::GoToCastle { perso, castle: hit }),
                9 => orders.push(Order::AttackPerso { perso, target: hit }),
                10 => orders.push(Order::AttackHouse { perso, house: hit }),
                11 => orders.push(Order::AttackCastle { perso, castle: hit }),
                13 => orders.push(Order::GoToTreasure { perso, treasure: hit }),
                14 => orders.push(Order::Give { perso, target: hit }),
                _ => orders.push(Order::Move {
                    perso,
                    x: wx as i64 - p.w as i64 / 2,
                    y: wy as i64 - p.h as i64 / 2,
                }),
            }
        }
        if !on_ui && is_mouse_button_pressed(MouseButton::Middle) {
            // The middle button shoots at the ground, or at the house or castle under it
            // (`FrmMoteur2D.frm:681-685`).
            let p = &world.persos[player];
            let kind = match hover {
                10 => 1,
                11 => 2,
                _ => 0,
            };
            orders.push(Order::AttackGround { perso: player, x: wx as i64 - p.w as i64 / 2, y: wy as i64 - p.h as i64 / 2, kind });
        }
        hud.overlay(&world, &rules, &textures, player, &input, placement, (!on_ui).then_some((hover, hit)));
        if show_perf {
            hud.debug_lines(&perf.lines());
        }
        let present_start = perf::now();
        perf.add(perf::Part::Hud, present_start - hud_start);
        // (going back to the default camera submits the interface's drawings)
        set_default_camera();
        let end = perf::now();
        perf.add(perf::Part::Present, end - present_start);
        perf.add(perf::Part::Frame, end - frame_start);

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
                // out only while some of the map is out of view
                None if up || view.0 < world.terrain.width as i32 * sim::terrain::CELL || view.1 < world.terrain.height as i32 * sim::terrain::CELL => {
                    scales.zoom = step(&ZOOMS, scales.zoom, up)
                }
                None => {}
            }
            scales.store();
        }

        // Every tick due by now, so that the world keeps its pace when frames are slower than
        // ticks; beyond MAX_TICKS_PER_FRAME the backlog is dropped and the world slows down.
        let still = hud.paused() || options::test_mode();
        if still {
            next_tick = get_time() + 1.0 / TICKS_PER_SECOND;
        }
        let mut ticks = 0;
        while !still && get_time() >= next_tick && ticks < MAX_TICKS_PER_FRAME {
            next_tick += 1.0 / TICKS_PER_SECOND;
            ticks += 1;
            perf.time(perf::Part::Tick, || world.step(&rules, &orders));
            orders.clear();
            effects.tick(&world.events);
            hud.update_messages(&world, &rules, player);
            hud.automatic_actions(&world, player, &mut orders);
            hud.craft_from_queue(&world, &rules, player, &mut orders);
        }
        if ticks == MAX_TICKS_PER_FRAME {
            next_tick = next_tick.max(get_time());
        }
        if ticks > 0 {
            cam = follow(&world, player, view);
        }
        // `ToucheSauvegarderMonde` (F12) saves now and says so (`Enregistrer_Sauvegarde_Monde`).
        let save_now = is_key_pressed(KeyCode::F12) && !options::test_mode();
        // Big worlds are saved less often: a second per megabyte (51 MB at 15000 characters).
        if save_now || get_time() - last_save > AUTOSAVE_INTERVAL.max(save_size as f64 / 1e6) {
            last_save = get_time();
            perf.time(perf::Part::Save, || {
                let bytes = world.save();
                save_size = bytes.len();
                storage::store("save", &bytes);
            });
            if save_now {
                hud.add_saved_message();
            }
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
        if !options::test_mode() {
            music.update();
        }
        next_frame().await;
    }
}
