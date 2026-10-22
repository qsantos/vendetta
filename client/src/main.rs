mod assets;
mod atlas;
mod hud;
mod lang;
mod layout;
mod effects;
mod speech;
mod loading;
mod music;
mod net;
mod options;
mod perf;
mod render;
mod sharp;
mod storage;
mod text;
mod touch;

use macroquad::prelude::*;
use layout::{MIN_H, MIN_W};
use macroquad::miniquad::window::dpi_scale as screen_dpi_scale;
use macroquad::experimental::coroutines::{Coroutine, start_coroutine};
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
/// See `load_game`.
const WORLD_WORK_PER_CELL: u64 = 4;

/// How far the mouse moves with its left button down (in CSS pixels) before it drags the map.
const PAN_SLOP: f32 = 6.0;

/// Whether an order sends the character somewhere, or to act there: the camera then follows
/// it again.
fn sends_character(order: &Order) -> bool {
    use Order::*;
    matches!(
        order,
        Move { .. }
            | Harvest { .. }
            | FindResource { .. }
            | GoOwnCastle { .. }
            | GoToCastle { .. }
            | GoHome { .. }
            | GoToHouse { .. }
            | BuildHouse { .. }
            | GoToTreasure { .. }
            | FindTreasure { .. }
            | GoSell { .. }
            | AttackPerso { .. }
            | AttackHouse { .. }
            | AttackCastle { .. }
            | AttackNearest { .. }
            | AttackGround { .. }
            | Give { .. }
            | Charge { .. }
    )
}

fn follow(world: &World, player: usize, view: (i32, i32)) -> (i32, i32) {
    let p = &world.persos[player];
    let axis = |pos: i64, view: i32, size: usize| {
        let centred = pos as i32 - view / 2;
        let max = size as i32 * 16 - view;
        if max < 0 || world.terrain.torus { centred } else { centred.min(max).max(0) }
    };
    wrap_cam(world, (axis(p.x, view.0, world.terrain.width), axis(p.y, view.1, world.terrain.height)))
}

/// On a torus, the camera brought back onto the map (the view continues across the edges).
fn wrap_cam(world: &World, cam: (i32, i32)) -> (i32, i32) {
    world.terrain.wrap32(cam)
}

/// The free camera's top-left kept on the map (or, zoomed out beyond the map's size, where it
/// is); on a torus, it goes round.
fn clamp_free_cam(world: &World, (x, y): (f32, f32), view: (i32, i32)) -> (f32, f32) {
    let (w, h) = world.terrain.size_px();
    let clamp = |v: f32, view: i32, size: i64| {
        let max = (size as i32 - view) as f32;
        if world.terrain.torus { v.rem_euclid(size as f32) } else if max < 0.0 { v } else { v.clamp(0.0, max) }
    };
    (clamp(x, view.0, w), clamp(y, view.1, h))
}

/// Whether the map can zoom out from showing `view`: while some of the map is out of view (on a
/// torus, the whole map then shows, with repeats around it, dimmed by `render::dim_copies`).
fn can_zoom_out(world: &World, view: (i32, i32)) -> bool {
    let (w, h) = world.terrain.size_px();
    view.0 < w as i32 || view.1 < h as i32
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

/// Beyond this many ticks waiting, a network game catches up at once rather than at its pace.
const CATCH_UP_BACKLOG: usize = 10;
/// How long a frame may play ticks when catching up with a network game.
const CATCH_UP_SLICE: f64 = 0.03;
/// The network game's save (one at a time), and its room's code.
const NET_SAVE: &str = "net-save";
const NET_ROOM: &str = "net-room";

/// How often the game in progress is saved, in seconds.
const AUTOSAVE_INTERVAL: f64 = 5.0;
/// Ticks a frame may run to catch up with the clock.
const MAX_TICKS_PER_FRAME: u32 = 5;

/// A language's data files and font (see `assets::load_language`).
type LanguageFiles = (Vec<(String, Vec<u8>)>, Option<Vec<u8>>);

struct Loaded {
    options: options::Options,
    /// The text's language: its folder in `Donnees/`.
    lang: String,
    /// Those of `Langues.ini`, and every file, to change language during the game.
    languages: Vec<sim::rules::Language>,
    manifest: assets::Manifest,
    /// The world was just made, not resumed from the save.
    created: bool,
    music: music::Music,
    effects: effects::Effects,
    files: Files,
    rules: Rules,
    world: World,
    textures: assets::Textures,
}

/// Another language's files, once downloaded: its rules, and its font for the text.
fn take_language(files: &mut Files, folder: &str, (loaded, font): LanguageFiles) -> Result<Rules, String> {
    for (path, data) in loaded {
        files.insert(&path, data);
    }
    let rules = Rules::load(files, folder)?;
    text::set_fallback(font.as_deref());
    Ok(rules)
}

/// Downloads the data and images and creates the world, reporting progress.
async fn load_game(progress: loading::Progress) -> Result<Loaded, String> {
    let mut manifest = assets::Manifest::load().await?;
    // the language first, for the loading screen's labels
    let languages = load_string(manifest.url("Donnees/Langues.ini").unwrap_or_default()).await.unwrap_or_default();
    let languages = sim::rules::Language::list(&sim::ini::Ini::parse(&languages));
    let mut lang = lang::choose(&languages).folder;
    manifest.only_language(&languages, &lang);
    progress.load_decoration(&manifest, &lang).await;
    progress.set_total(manifest.data_bytes() + assets::image_work(manifest.image_estimate()));
    let mut files = assets::load_files(&manifest, &progress).await?;
    let mut rules = Rules::load(&files, &lang)?;
    // the characters the interface's font lacks (Chinese, Japanese)
    if let Some(url) = manifest.url(&format!("Polices/{lang}.otf")) {
        match load_file(url).await {
            Ok(bytes) => text::set_fallback(Some(&bytes)),
            Err(e) => {
                warn!("Polices/{lang}.otf: {e}");
            }
        }
    }
    let mut textures = assets::Textures::new(&manifest);
    let (terrain, persos) = render::images(&rules, &manifest);
    let hud = hud::images(&rules);
    let all: Vec<String> = terrain.iter().chain(&persos).chain(&hud).cloned().collect();
    let images: u64 = textures.missing(&all).iter().map(|p| assets::image_work(manifest.size(p))).sum();
    progress.set_total(manifest.data_bytes() + images + effects::SOUND_WORK);
    textures.load_all(&manifest, &terrain, &progress, &progress.label(7)).await;
    render::add_border_tiles(&mut textures);
    textures.load_all(&manifest, &persos, &progress, &progress.label(6)).await;
    textures.load_all(&manifest, &hud, &progress, &progress.label(10)).await;
    progress.message(&progress.label(11));
    let effects = effects::Effects::new(&files, &progress).await;
    // The page loads the game behind its menus: the world waits for a game to be chosen.
    // (a frame at a time: `wait_seconds` takes off the last frame's duration, so it is over at
    // once after a slow frame, and would spin here forever)
    while !storage::started() {
        assets::yield_now().await;
    }
    // a language chosen in the menus meanwhile (the game then finds its command done)
    let chosen = lang::choose(&languages).folder;
    if chosen != lang {
        let font = manifest.url(&format!("Polices/{chosen}.otf")).map(str::to_owned);
        match assets::load_language(manifest.language_files(&chosen), font).await.and_then(|l| take_language(&mut files, &chosen, l)) {
            Ok(r) => (rules, lang) = (r, chosen),
            Err(e) => {
                warn!("{chosen}: {e}");
            }
        }
    }
    progress.message(&progress.label(2));
    let options = options::Options::load();
    let touch_screen = storage::touch_screen();
    layout::set_touch(storage::layout().or(options.touch).unwrap_or(touch_screen));
    let mut setup = WorldSetup { seed: (macroquad::miniquad::date::now() * 1000.0) as u64 as u32, ..Default::default() };
    setup.roads = if options.ring_roads { sim::world::Roads::Ring } else { sim::world::Roads::Nearest };
    setup.torus = !options.closed_edges;
    setup.landscape = if options.original_land { sim::landscape::Landscape::Original } else { sim::landscape::Landscape::Natural };
    // (a world type out of range: the last)
    setup.world_type = match options.world {
        Some(n) => Some((n - 1).min(rules.world_types.len().saturating_sub(1))),
        None if options.original_land => setup.world_type,
        None => None,
    };
    setup.shape = options.shape;
    setup.towns = if options.scattered_towns { sim::world::Towns::Scattered } else { sim::world::Towns::Compact };
    setup.monsters = if options.waiting_monsters { sim::world::Monsters::Wait } else { sim::world::Monsters::Roam };
    setup.paths = if options.greedy_paths { sim::path::Paths::Greedy } else { sim::path::Paths::Fastest };
    setup.chase = if options.direct_chase { sim::path::Chase::Direct } else { sim::path::Chase::Intercept };
    if let Some(speed) = options.speed {
        setup.speed = speed;
    }
    if let Some(seed) = options.seed {
        setup.seed = seed;
    }
    if let Some((w, h)) = options.size {
        // as many resource nodes per cell as the default 128×128 world
        setup.resources = setup.resources * w * h / (setup.width * setup.height);
        (setup.width, setup.height) = (w, h);
    }
    if !options.players.is_empty() {
        setup.players = options.players.clone();
    }
    setup.names = options.names.clone();
    setup.characters = options.characters.clone();
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
    // Making the world counts as so many bytes of data per cell (a 128×128 one takes about as
    // long as 65 KB of data, a 256×256 one as 260 KB).
    let world_work = (setup.width * setup.height) as u64 * WORLD_WORK_PER_CELL;
    progress.expect(world_work);
    // A saved game is resumed only with the setup options it was made with.
    let same_setup = !options.new && storage::load("setup").is_some_and(|k| k == options.setup_key().as_bytes());
    // Resume the saved game if there is one that fits these rules; in network play, the room's
    // own save when coming back to it.
    let saved = match options.me {
        None => storage::load("save").filter(|_| same_setup),
        Some(_) => storage::load(NET_SAVE).filter(|_| options.resume && storage::load(NET_ROOM).is_some_and(|r| r == options.room.as_bytes())),
    };
    let saved = saved.and_then(|bytes| match World::load(&rules, &bytes) {
        Ok(world) => Some(world),
        Err(e) => {
            warn!("discarding the saved game: {e}");
            None
        }
    });
    let mut created = false;
    let world = match saved {
        Some(world) => world,
        None => {
            created = true;
            if options.me.is_none() {
                storage::store("setup", options.setup_key().as_bytes());
            } else {
                // (no save for this room yet)
                storage::store(NET_ROOM, options.room.as_bytes());
                storage::store(NET_SAVE, &[]);
            }
            World::create(&rules, &files, &setup)?
        }
    };
    progress.work(world_work);
    Ok(Loaded { options, lang, languages, created, music: music::Music::new(&manifest), manifest, effects, files, rules, world, textures })
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
    let Loaded { options, mut lang, languages, manifest, created, mut music, mut effects, mut files, mut rules, mut world, textures } = match task.retrieve().unwrap() {
        Ok(l) => l,
        Err(e) => loop {
            screen.progress.message(&format!("&{}#{e}", lang::tr(["Erreur", "Error", "Error", "エラー", "错误"])));
            screen.draw();
            next_frame().await;
        },
    };
    let mut net = options.me.map(|me| net::Net::new(&world, &options.players, &options.names, me, world.tick));
    let mut player = net.as_ref().and_then(|n| n.player()).or_else(|| world.persos.iter().position(|p| p.player)).unwrap();
    // In network play, the world starts with the server's first tick, once the players have made their worlds.
    // Coming back to a game in progress, the ticks missed are played first, as fast as can be.
    if let Some(net) = &mut net {
        if options.resume {
            info!("resuming at tick {}", world.tick);
            net.since();
        } else {
            net.loaded();
        }
        while net.backlog() == 0 {
            net.poll();
            let (ready, players) = net.waiting;
            screen.progress.message(&if options.resume {
                lang::tr(["Reprise de la partie…", "Resuming the game…", "Reanudando la partida…", "ゲームを再開中…", "正在恢复游戏…"]).to_owned()
            } else {
                format!("{} ({ready} / {players} {})…", lang::tr(["En attente des autres joueurs", "Waiting for the other players", "Esperando a los demás jugadores", "他のプレイヤーを待っています", "正在等待其他玩家"]), lang::tr(["prêts", "ready", "listos", "準備完了", "已就绪"]))
            });
            screen.draw();
            next_frame().await;
        }
        while net.backlog() > MAX_TICKS_PER_FRAME as usize {
            let start = perf::now();
            while perf::now() - start < CATCH_UP_SLICE && let Some(tick) = net.next_tick() {
                world.step(&rules, &tick);
                net.after_tick(&world);
            }
            net.poll();
            let left = net.backlog() as f64 / world.speed as f64;
            screen.progress.message(&format!("{} {left} {}", lang::tr(["Rattrapage de la partie : encore", "Catching up with the game:", "Alcanzando la partida: quedan", "ゲームに追いついています：残り", "正在追赶游戏进度：还剩"]), lang::tr(["s de jeu…", "s of play left…", "s de juego…", "秒…", "秒…"])));
            screen.draw();
            next_frame().await;
        }
    }
    screen.finish();
    // A resumed solo game takes the speed it is started with (a network one keeps the room's).
    if let (Some(speed), None) = (options.speed, options.me) {
        world.speed = speed;
    }
    if options.god {
        world.god = Some(player);
        // at once, for the still scenes of the rendering tests
        world.refill_god(&rules);
    }
    let fief = world.persos[player].fief;
    if let Some(object) = options.hold.filter(|&o| o < rules.objects.len()) {
        world.persos[player].selected_object = object as i64;
    }
    if options.emperor {
        world.change_emperor(&rules, fief, player as i64, false);
    }
    if let Some(pct) = options.advance {
        let speed = world.epoch_speed;
        world.fiefs[fief].change_epoch(&rules, &mut world.castles[fief], speed);
        let f = &mut world.fiefs[fief];
        f.epoch_timer = (f.epoch_timer as f64 * (1.0 - pct / 100.0)) as i64;
    }
    for &(object, left) in options.effects.iter().filter(|&&(o, _)| o < rules.objects.len()) {
        let p = &mut world.persos[player];
        let d = rules.objects[object].duration;
        p.add_effect(&rules, object, d, d, None);
        if let (Some(left), Some(e)) = (left, p.effects.last_mut()) {
            e.duration = left;
        }
    }
    if created {
        for (name, count) in &options.inventory {
            let object = name.parse::<usize>().ok().filter(|&o| o < rules.objects.len()).or_else(|| {
                rules.objects.iter().position(|o| o.name.to_lowercase() == name.to_lowercase())
            });
            match object {
                Some(object) => (0..*count).for_each(|_| world.persos[player].add_object(&rules, object)),
                None => {
                    warn!("inventory: no object {name}");
                }
            }
        }
    }
    // The interface's random choices (tracks, tips), not the world's: different each game.
    rand::srand((macroquad::miniquad::date::now() * 1000.0) as u64);
    // The game starts with a "Debut" track (`FrmMoteur2D.frm:405`).
    music.play(music::DEBUT, false);
    let mut last_save = get_time();
    let mut save_size = 0;
    let mut hud = hud::Hud::new(hud::Strings::load(&files, &lang), hud::WindowStrings::load(&files, &lang), hud::TooltipLabels::load(&files, &lang), &files, &lang, rules.objects.len());
    hud.rebuild_elsewhere = options.rebuild_elsewhere;
    for (i, &(object, left)) in options.recharges.iter().filter(|&&(o, _)| o < rules.objects.len()).enumerate() {
        let p = &mut world.persos[player];
        p.add_object(&rules, object);
        p.add_recharge(object, left);
        hud.bind_object(object, i as i32 + 1);
    }


    // `Afficher_Conseil`: a tip to start with (not in the still scenes of the rendering tests).
    if !options::test_mode() {
        hud.add_tip(rand::gen_range(0.0, 1.0));
    }
    let mut scales = Scales::load();
    let mut view = (0, 0);
    let mut cam = (0, 0);
    let mut orders = Vec::new();
    // When the next tick is due, in solo and in network play.
    let mut next_tick = get_time();
    let mut net_due = 0.0f64;
    // Made the first time the map is zoomed far out.
    let mut overview: Option<render::Overview> = None;
    // `CliqueSurTerrain`: the left button went down on the map, so its release acts there.
    let mut pressed_on_map = false;
    let mut perf = perf::Perf::default();
    let touch_screen = storage::touch_screen();
    // The speed the page's menu was told of.
    let mut shown_speed = None;
    let mut gestures = touch::Gestures::default();
    // How far the fingers have pinched since the zoom last changed.
    let mut pinched = 1.0f32;
    // The camera dragged away from the character (top-left, map pixels), and the mouse's drag.
    let mut free_cam: Option<(f32, f32)> = None;
    let mut speech = speech::Speech::default();
    // The last frame's zoom, and the point of the screen (physical pixels) whose spot on the map
    // stays put when the zoom changes with the camera free (the middle of the screen if `None`).
    let mut last_zoom = 0.0f32;
    let mut zoom_anchor: Option<(f32, f32)> = None;
    let mut pan_from: Option<(f32, f32)> = None;
    let mut panning = false;
    // Another language's files on their way (see `MenuCommand::Language`).
    let mut new_language: Option<(String, Coroutine<Result<LanguageFiles, String>>)> = None;
    loop {
        let frame_start = perf::now();
        // The interface the menu chose, else the address's, else the screen's; with the touch
        // one the fingers are read as such, not as a mouse.
        let touch = storage::layout().or(options.touch).unwrap_or(touch_screen);
        layout::set_touch(touch);
        simulate_mouse_with_touch(!touch);
        perf.frame();
        // `ToucheChangerPersonnage`: control the next human character, or the previous one (the
        // original's key code 41, with Shift for the previous one, is not on today's keyboards;
        // here Page Down and Page Up).
        let back = is_key_pressed(KeyCode::PageUp);
        if back || is_key_pressed(KeyCode::PageDown) {
            let n = world.persos.len();
            let next = (1..n).map(|k| if back { (player + n - k) % n } else { (player + k) % n }).find(|&i| world.persos[i].player);
            if let Some(next) = next {
                player = next;
                hud.switch_character();
                cam = follow(&world, player, view);
            }
        }
        // The frame covers the window; the map and the interface each have their own scale.
        // `screen_width()` and the mouse are in CSS pixels; the frame is in physical pixels.
        let dpi = screen_dpi_scale();
        let (w, h) = ((screen_width() * dpi) as i32, (screen_height() * dpi) as i32);
        let (zoom, ui) = scales.effective();
        // The touch interface fits a window in a phone's shorter side, whatever the setting;
        // a tablet's has room to spare.
        let ui = if touch { (w.min(h) as f32 / layout::TOUCH_SIDE).min(layout::TOUCH_MAX_SCALE * dpi) } else { ui };
        let view_f = (w as f32 / zoom, h as f32 / zoom);
        let new_view = (view_f.0.ceil() as i32, view_f.1.ceil() as i32);
        if new_view != view {
            view = new_view;
            cam = follow(&world, player, view);
        }
        if let Some((fx, fy)) = free_cam
            && last_zoom > 0.0
            && zoom != last_zoom
        {
            let (ax, ay) = zoom_anchor.unwrap_or((w as f32 / 2.0, h as f32 / 2.0));
            let moved = (fx + ax / last_zoom - ax / zoom, fy + ay / last_zoom - ay / zoom);
            free_cam = Some(clamp_free_cam(&world, moved, view));
        }
        (last_zoom, zoom_anchor) = (zoom, None);
        let hud_f = (w as f32 / ui, h as f32 / ui);
        let hud_size = (hud_f.0 as i32, hud_f.1 as i32);
        layout::set_screen(hud_size.0, hud_size.1);
        hud.set_scale(ui);
        hud.set_zoom(scales.zoom, zoom);

        let fingers = if touch { gestures.poll(dpi) } else { touch::Frame::default() };
        let (mx, my) = if touch {
            fingers.pos
        } else {
            let (mx, my) = mouse_position();
            (mx * dpi, my * dpi)
        };
        // The interface's mouse (`SourisX/Y`), clamped to the screen like the original.
        let (hx, hy) = (((mx / ui) as i32).clamp(0, hud_size.0), ((my / ui) as i32).clamp(0, hud_size.1));
        let input = if touch { hud::Input::from_fingers((hx, hy), &fingers, ui) } else { hud::Input::poll((hx, hy)) };

        // Dragging the map looks elsewhere (not in the original, whose camera follows the
        // character but for the map window's): with the left button pressed on the map, once
        // the mouse has moved a few pixels (its release then does nothing), or with a finger
        // (unless it drags a building into place). The camera stays there until the character
        // is sent somewhere, Space, or the HUD's button or arrow.
        let pan = if touch {
            fingers.drag.filter(|_| hud.placing.is_none() && !hud.over_touch_ui(&world, &rules, player, (hx, hy)))
        } else if let (Some(from), true) = (pan_from, is_mouse_button_down(MouseButton::Left)) {
            if !panning && (mx - from.0).hypot(my - from.1) > PAN_SLOP * dpi {
                panning = true;
            }
            pan_from = Some((mx, my));
            panning.then_some((mx - from.0, my - from.1))
        } else {
            None
        };
        if let Some((dx, dy)) = pan {
            let (fx, fy) = free_cam.unwrap_or((cam.0 as f32, cam.1 as f32));
            free_cam = Some(clamp_free_cam(&world, (fx - dx / zoom, fy - dy / zoom), view));
        }
        // (a release that ends a drag is not a click)
        let input = if panning && input.left_released { hud::Input { left_released: false, ..input } } else { input };
        if !is_mouse_button_down(MouseButton::Left) && !input.left_released {
            pan_from = None;
            panning = false;
        }
        if hud.take_follow_request() || is_key_pressed(KeyCode::Space) {
            free_cam = None;
            cam = follow(&world, player, view);
        }
        if let Some((fx, fy)) = free_cam {
            cam = (fx.round() as i32, fy.round() as i32);
        }
        if let Some(c) = hud.camera() {
            cam = c;
        }
        cam = wrap_cam(&world, cam);
        let (mw, mh) = world.terrain.size_px();
        hud.set_view(cam, view, world.terrain.torus.then_some((mw as i32, mh as i32)));
        hud.set_free_camera(free_cam.map(|_| {
            let p = &world.persos[player];
            let (mw, mh) = world.terrain.size_px();
            let at = |v: i64, size: i64, c: i32, view: i32, map: i64| {
                (render::rel(world.terrain.torus, (v + size / 2) as i32, c, view, map as i32) as f32 * zoom / ui) as i32
            };
            (at(p.x, p.w as i64, cam.0, view.0, mw), at(p.y, p.h as i64, cam.1, view.1, mh))
        }));
        // What is under the cursor on the map (`Obstacle` on the cursor's top-left).
        let to_world = |p: f32, c: i32| c + (p / zoom).floor() as i32;
        let (wx, wy) = (to_world(mx, cam.0), to_world(my, cam.1));
        let (hover, hit) = cursor_type(&world, &rules, player, wx, wy);
        // The cursor image is drawn at the interface's scale: its centre on the map. With the
        // touch interface, the building being placed starts in the middle of the screen, then
        // goes where the fingers drag or tap it.
        let centre = if touch {
            let on_map = !hud.over_touch_ui(&world, &rules, player, (hx, hy));
            let drag = fingers.drag.filter(|_| on_map).map_or((0.0, 0.0), |(dx, dy)| (dx / zoom, dy / zoom));
            let tap = (on_map && fingers.tap).then_some((wx, wy));
            let (fw, fh) = (layout::screen_w() as f32 * ui, layout::screen_h() as f32 * ui);
            hud.touch_ghost(drag, tap, (to_world(fw / 2.0, cam.0), to_world(fh / 2.0, cam.1)))
        } else {
            (to_world(mx + 12.0 * ui, cam.0), to_world(my + 12.0 * ui, cam.1))
        };

        set_camera(&camera(view_f));
        clear_background(BLACK);
        sharp::begin_frame((w as f32, h as f32));
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
            for c in render::copies(&world, cam, view) {
                effects.draw(&textures, c, view, zoom);
            }
        }
        render::dim_copies(&world, player, cam, view);
        let placement = hud.draw_placement(&world, &rules, player, centre, cam);
        // (changing the camera submits the map's drawings)
        set_camera(&camera(hud_f));
        let hud_start = perf::now();
        perf.add(perf::Part::Map, hud_start - map_start);
        if !far {
            let (torus, (mw, mh)) = (world.terrain.torus, world.terrain.size_px());
            let at = |v: i64, c: i32, view: i32, map: i64| ((render::rel(torus, v as i32, c, view, map as i32) as f32) * zoom / ui) as i32;
            hud.draw_comments(&world, &speech, |x, y| (at(x, cam.0, view.0, mw), at(y, cam.1, view.1, mh)));
        }
        let radial_was_open = hud.radial_open();
        let given = orders.len();
        let on_ui = hud.frame(&world, &rules, &textures, player, &input, &mut orders);
        if input.left_pressed {
            pressed_on_map = !on_ui;
            if !touch && !on_ui && hud.placing.is_none() {
                pan_from = Some((mx, my));
            }
        }
        let placed = !on_ui && hud.map_input(&world, &rules, player, &input, centre, &mut orders);
        let holding = world.persos[player].selected_object >= 0;
        if !on_ui && !placed && input.right_pressed && holding {
            // Right click while holding an object puts it back.
            orders.push(Order::Unselect { perso: player });
        } else if !on_ui && !placed && input.right_pressed {
            // Right click on the map: the radial buttons (`FrmMoteur2D.frm:943-961`; the original
            // also moved the cursor onto one, which browsers do not allow).
            match hover {
                0 | 1 | 4..=8 | 12 | 16 | 999 => hud.open_radial((hx, hy)),
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
        // Sending the character somewhere follows it again.
        if orders[given..].iter().any(sends_character) {
            free_cam = None;
        }
        hud.overlay(&world, &rules, &textures, player, &input, placement, (!on_ui).then_some((hover, hit)));
        if options.perf {
            hud.debug_lines(&perf.lines());
        }
        if let Some(net) = &net {
            hud.absent_lines(&net.absent());
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
                None if up || can_zoom_out(&world, view) => {
                    scales.zoom = step(&ZOOMS, scales.zoom, up);
                    zoom_anchor = Some((mx, my));
                }
                None => {}
            }
            scales.store();
        }

        // Two fingers zoom the map, a step each time they have pinched by a quarter.
        if !fingers.down {
            pinched = 1.0;
        }
        pinched *= fingers.pinch;
        if touch && (pinched > 1.25 || pinched < 0.8) {
            let up = pinched > 1.0;
            pinched = 1.0;
            if up || can_zoom_out(&world, view) {
                scales.zoom = step(&ZOOMS, scales.zoom, up);
                zoom_anchor = Some((mx, my));
                scales.store();
            }
        }

        // Every tick due by now, so that the world keeps its pace when frames are slower than
        // ticks; beyond MAX_TICKS_PER_FRAME the backlog is dropped and the world slows down.
        // In network play, the server's ticks instead (and pausing does not stop the world).
        if let Some(net) = &mut net {
            net.poll();
            for order in orders.drain(..) {
                net.send_order(&order);
            }
            // The ticks are played on a local clock, from a small buffer that absorbs the jitter
            // of their arrival: played as they come, they would bunch up (two steps in a frame,
            // then none). The buffer adapts: starved, the clock waits a tick more (a tick more of
            // delay); with ticks piling up, it runs a little faster. Far behind (a hidden tab, a
            // reconnection), it catches up at once.
            let now = get_time();
            let period = 1.0 / world.speed as f64;
            let catch_up = net.backlog() > CATCH_UP_BACKLOG;
            let start = perf::now();
            net_due = net_due.max(now - period);
            let mut ticks = 0;
            while (if catch_up { perf::now() - start < CATCH_UP_SLICE } else { now >= net_due }) && let Some(tick) = net.next_tick() {
                ticks += 1;
                net_due = if catch_up { now + period } else { net_due + period * if net.backlog() >= 2 { 0.9 } else { 1.0 } };
                perf.time(perf::Part::Tick, || world.step(&rules, &tick));
                net.after_tick(&world);
                effects.tick(&world.events);
                speech.tick(&world, &rules);
                hud.update_messages(&world, &rules, player);
                if net.settled() {
                    hud.automatic_actions(&world, player, &mut orders);
                    hud.craft_from_queue(&world, &rules, player, &mut orders);
                }
            }
            if !catch_up && net.backlog() == 0 && now >= net_due {
                net_due = now + period;
            }
            if ticks > 0 {
                cam = follow(&world, player, view);
            }
            for notice in net.take_notices() {
                hud.add_notice(&notice);
            }
            if std::mem::take(&mut net.closed) {
                hud.add_notice(lang::tr(["&Connexion au serveur perdue : reconnexion…", "&Connection to the server lost: reconnecting…", "&Conexión con el servidor perdida: reconectando…", "&サーバーとの接続が切れました：再接続中…", "&与服务器的连接已断开：正在重新连接…"]));
            }
            if std::mem::take(&mut net.reconnected) {
                info!("reconnected at tick {}", world.tick);
                hud.add_notice(lang::tr(["Reconnecté au serveur.", "Reconnected to the server.", "Reconectado al servidor.", "サーバーに再接続しました。", "已重新连接到服务器。"]));
            }
            if let Some(tick) = net.desync.take() {
                warn!("desync at tick {}", tick);
                let desync = lang::tr(["&Désynchronisation au tick", "&Desync at tick", "&Desincronización en el tick", "&非同期：tick", "&不同步：tick"]);
                let differ = lang::tr([
                    " : les mondes des joueurs diffèrent.",
                    ": the players' worlds differ.",
                    ": los mundos de los jugadores difieren.",
                    "：プレイヤー間でワールドが異なります。",
                    "：玩家的世界不一致。",
                ]);
                hud.add_notice(&format!("{desync} {tick}{differ}"));
            }
        }
        // The in-game menu (the page's): it pauses a solo game, not a network one, where the
        // character stands still meanwhile. Quitting saves first.
        if hud.take_menu_request() {
            storage::open_menu();
        }
        hud.set_menu_open(storage::menu_open());
        if shown_speed != Some(world.speed) {
            shown_speed = Some(world.speed);
            storage::show_speed(world.speed, net.is_some());
        }
        let mut quit = false;
        let mut language_changed = false;
        loop {
            let out = can_zoom_out(&world, view);
            match storage::menu_command() {
                storage::MenuCommand::None => break,
                storage::MenuCommand::Quit => quit = true,
                storage::MenuCommand::Language => language_changed = true,
                storage::MenuCommand::Zoom(up) if up || out => scales.zoom = step(&ZOOMS, scales.zoom, up),
                storage::MenuCommand::Zoom(_) => {}
                storage::MenuCommand::Interface(up) => scales.ui = step(&UI_SCALES, scales.ui, up),
                // (in network play, the room's)
                storage::MenuCommand::Speed(v) if net.is_none() => world.speed = v.clamp(options::SPEEDS.0, options::SPEEDS.1),
                storage::MenuCommand::Speed(_) => {}
            }
            scales.store();
        }
        // The page's menu changed the language: the port's own texts follow at once, those of
        // the data once its files are there, the game going on meanwhile.
        if language_changed {
            let folder = lang::choose(&languages).folder;
            new_language = (folder != lang).then(|| {
                let font = manifest.url(&format!("Polices/{folder}.otf")).map(str::to_owned);
                let task = start_coroutine(assets::load_language(manifest.language_files(&folder), font));
                (folder, task)
            });
        }
        if let Some((folder, task)) = new_language.take_if(|(_, task)| task.is_done()) {
            match task.retrieve().unwrap().and_then(|l| take_language(&mut files, &folder, l)) {
                Ok(r) => {
                    hud.set_texts(hud::Strings::load(&files, &folder), hud::WindowStrings::load(&files, &folder), hud::TooltipLabels::load(&files, &folder), &files, &folder);
                    (rules, lang) = (r, folder);
                }
                Err(e) => {
                    warn!("{folder}: {e}");
                }
            }
        }
        let still = hud.paused() || options::test_mode() || net.is_some();
        if still {
            next_tick = get_time() + 1.0 / world.speed as f64;
        }
        let mut ticks = 0;
        while !still && get_time() >= next_tick && ticks < MAX_TICKS_PER_FRAME {
            next_tick += 1.0 / world.speed as f64;
            ticks += 1;
            perf.time(perf::Part::Tick, || world.step(&rules, &orders));
            orders.clear();
            effects.tick(&world.events);
            speech.tick(&world, &rules);
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
        let save_now = !options::test_mode() && (quit || is_key_pressed(KeyCode::F12) && net.is_none());
        // Big worlds are saved less often: a second per megabyte (51 MB at 15000 characters).
        if save_now || get_time() - last_save > AUTOSAVE_INTERVAL.max(save_size as f64 / 1e6) {
            last_save = get_time();
            perf.time(perf::Part::Save, || {
                let bytes = world.save();
                save_size = bytes.len();
                storage::store(if net.is_some() { NET_SAVE } else { "save" }, &bytes);
            });
            if save_now && !quit {
                hud.add_saved_message();
            }
        }
        if quit {
            storage::quit();
        }
        // The mood's music, every frame (`FrmMoteur2D.frm:426-438`).
        let p = &world.persos[player];
        if world.victorious(p.team) {
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
