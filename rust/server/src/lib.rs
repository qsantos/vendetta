//! The network play server: a lobby, then a relay of the players' orders in lockstep.
//!
//! It never runs the simulation. Players gather in a room (a short code, case-insensitive),
//! each picking a fief, and the host sets the world's options and starts the game. Every client
//! then builds the same world from those options and a seed, and the server broadcasts a tick
//! every `TICK` with the orders received since the previous one: clients step their world once
//! per tick received, applying those orders, so all copies stay the same. Clients report a
//! digest of their world now and then, and everyone is told if two differ.
//!
//! Messages are JSON text, tagged by `t`. From a client:
//! - `{"t":"create","name":…}`, `{"t":"join","room":…,"name":…}`: enter a new or existing room;
//! - `{"t":"fief","fief":n}`: pick a fief (from 1);
//! - `{"t":"settings","items":[…]}` (host): the world options, as in the URL's hash;
//! - `{"t":"start"}` (host);
//! - `{"t":"ready"}`: its world is made, the ticks may start;
//! - `{"t":"rejoin","room":…,"token":…}`: take one's place back in a game in progress;
//! - `{"t":"since","have":n}`: after a rejoin, the ticks from the n-th on (n of them played);
//! - `{"t":"order","order":…}`: an order for the next tick, passed on as it is;
//! - `{"t":"hash","tick":n,"hash":h}`: the world's digest after tick n.
//!
//! From the server:
//! - `{"t":"lobby","room":…,"you":k,"token":…,"host":k,"players":[{"name","fief","online"}],"items":[…]}`;
//! - `{"t":"start","items":[…],"seed":s,"players":[fief…],"you":k}`, with `"resume":true`
//!   after a rejoin;
//! - `{"t":"wait","ready":n,"players":m}`: n of the m players are ready;
//! - `{"t":"tick","n":n,"orders":[[k,order]…]}`, n counting from 0;
//! - `{"t":"desync","tick":n}`, `{"t":"error","message":…}`.
//!
//! Players are numbered by their place in the room, which the start fixes.
//!
//! The ticks start once every player still connected is ready, or, with the room option
//! `wait=F`, once F times what the first one ready took has passed: slow players then catch up.
//! Always once one at least is ready.
//!
//! A player who leaves a game in progress keeps its place (its character stands still): with
//! the token the lobby gave it, it takes it back and gets the ticks it missed, which the room
//! keeps from the start. A game nobody is connected to lasts `GRACE`, then is forgotten.

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::hash::{BuildHasher, RandomState};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

/// The simulation's pace (`Jeu.ini Vitesse`, 10 ticks a second).
pub const TICK: Duration = Duration::from_millis(100);
/// Room codes: no letters that look alike (0/O, 1/I/L).
const CODE_CHARS: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
const CODE_LEN: usize = 4;
/// The world options the host may set (see the client's `options.rs`), and `wait`.
const SETTINGS: [&str; 6] = ["size", "fiefs", "villagers", "roads", "seed", "wait"];
/// The default number of fiefs (`WorldSetup`).
const DEFAULT_FIEFS: usize = 4;
/// How long a game in progress lasts with nobody connected.
const GRACE: Duration = Duration::from_secs(600);

#[derive(Deserialize)]
#[serde(tag = "t", rename_all = "lowercase")]
enum In {
    Create { name: String },
    Join { room: String, name: String },
    Fief { fief: usize },
    Settings { items: Vec<String> },
    Start,
    Ready,
    Rejoin { room: String, token: String },
    Since { have: usize },
    Order { order: Value },
    Hash { tick: u64, hash: u64 },
}

struct Player {
    id: u64,
    /// The secret to take one's place back (see `In::Rejoin`).
    token: String,
    /// The connection it plays from: another one taking its place over makes it stale.
    conn: u64,
    name: String,
    fief: usize,
    /// Its world is made (once started).
    ready: bool,
    /// Back in the game and not caught up yet: no tick until it asks for those it missed
    /// (`In::Since`), which then come in order.
    rejoining: bool,
    /// Its connection; none once gone (its character then stands still).
    tx: Option<UnboundedSender<String>>,
}

#[derive(Default)]
struct Room {
    players: Vec<Player>,
    host: u64,
    items: Vec<String>,
    started: bool,
    seed: u32,
    /// The orders for the next tick, by player.
    pending: Vec<(usize, Value)>,
    /// The first digest reported for each recent tick.
    hashes: HashMap<u64, u64>,
    desync: bool,
    /// When the game started, and how long the first player ready took.
    started_at: Option<Instant>,
    first_ready: Option<Duration>,
    /// Every tick sent, as sent.
    log: Vec<String>,
    /// Since when nobody is connected (a game in progress).
    empty_since: Option<Instant>,
}

impl Room {
    fn fiefs(&self) -> usize {
        self.items.iter().find_map(|o| o.strip_prefix("fiefs=")?.parse().ok()).unwrap_or(DEFAULT_FIEFS)
    }

    /// `wait=F`: how many times the first player's loading time to wait for the others.
    fn wait(&self) -> Option<f64> {
        self.items.iter().find_map(|o| o.strip_prefix("wait=")?.parse().ok()).filter(|f: &f64| *f > 0.0)
    }

    /// Whether the ticks may start (see the module's doc).
    fn all_ready(&self) -> bool {
        let Some(first) = self.first_ready else { return false };
        let online = self.players.iter().filter(|p| p.tx.is_some());
        online.clone().all(|p| p.ready)
            || self.wait().zip(self.started_at).is_some_and(|(f, at)| at.elapsed() >= first.mul_f64(f))
    }

    /// Player k's world is made (or caught up with the game): see `all_ready`.
    fn set_ready(&mut self, k: usize) {
        if std::mem::replace(&mut self.players[k].ready, true) {
            return;
        }
        let took = self.started_at.map_or(Duration::ZERO, |at| at.elapsed());
        self.first_ready.get_or_insert(took);
        let ready = self.players.iter().filter(|p| p.ready).count();
        self.broadcast(&json!({"t": "wait", "ready": ready, "players": self.players.len()}));
    }

    /// Nobody connected for longer than `GRACE`.
    fn abandoned(&self) -> bool {
        self.empty_since.is_some_and(|t| t.elapsed() > GRACE)
    }

    fn index(&self, id: u64) -> Option<usize> {
        self.players.iter().position(|p| p.id == id)
    }

    fn send(&self, k: usize, msg: &Value) {
        if let Some(tx) = &self.players[k].tx {
            let _ = tx.send(msg.to_string());
        }
    }

    fn broadcast(&self, msg: &Value) {
        let text = msg.to_string();
        for tx in self.players.iter().filter_map(|p| p.tx.as_ref()) {
            let _ = tx.send(text.clone());
        }
    }

    fn send_lobby(&self, code: &str) {
        let host = self.index(self.host).unwrap_or(0);
        let players: Vec<Value> = self.players.iter().map(|p| json!({"name": p.name, "fief": p.fief, "online": p.tx.is_some()})).collect();
        for k in 0..self.players.len() {
            let token = &self.players[k].token;
            self.send(k, &json!({"t": "lobby", "room": code, "you": k, "token": token, "host": host, "players": players, "items": self.items}));
        }
    }
}

#[derive(Default)]
struct Rooms {
    rooms: Mutex<HashMap<String, Room>>,
    random: RandomState,
}

impl Rooms {
    fn random(&self, n: u64) -> u64 {
        self.random.hash_one((n, std::time::SystemTime::now()))
    }

    fn player(&self, conn: u64, name: &str, tx: &UnboundedSender<String>) -> Player {
        let token = format!("{:016x}{:016x}", self.random(conn), self.random(conn ^ 1));
        Player { id: conn, token, conn, name: clean(name), fief: 1, ready: false, rejoining: false, tx: Some(tx.clone()) }
    }
}

/// The app: `/ws` for the game, and the files of `dist` (the built page) if given.
pub fn app(dist: Option<PathBuf>) -> Router {
    let router = Router::new().route("/ws", axum::routing::get(upgrade)).with_state(Arc::new(Rooms::default()));
    match dist {
        Some(dir) => router.fallback_service(tower_http::services::ServeDir::new(dir)).layer(axum::middleware::from_fn(caching)),
        None => router,
    }
}

/// The caching headers of `serve.py`: versioned requests (`?v=<hash>`) never change and are
/// cached for good, everything else is revalidated on each load.
async fn caching(request: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let versioned = request.uri().query().is_some_and(|q| q.split('&').any(|kv| kv.starts_with("v=")));
    let mut response = next.run(request).await;
    let value = if versioned { "public, max-age=31536000, immutable" } else { "no-cache" };
    response.headers_mut().insert(axum::http::header::CACHE_CONTROL, axum::http::HeaderValue::from_static(value));
    response
}

async fn upgrade(ws: WebSocketUpgrade, State(rooms): State<Arc<Rooms>>) -> Response {
    ws.on_upgrade(move |socket| connection(socket, rooms))
}

fn error(tx: &UnboundedSender<String>, message: &str) {
    let _ = tx.send(json!({"t": "error", "message": message}).to_string());
}

async fn connection(mut socket: WebSocket, rooms: Arc<Rooms>) {
    let (tx, mut rx) = unbounded_channel::<String>();
    let conn = rooms.random(0);
    // The room code and the player's id, once in one.
    let mut joined: Option<(String, u64)> = None;
    loop {
        tokio::select! {
            out = rx.recv() => {
                let Some(text) = out else { break };
                if socket.send(Message::Text(text.into())).await.is_err() {
                    break;
                }
            }
            msg = socket.recv() => {
                let text = match msg {
                    Some(Ok(Message::Text(text))) => text,
                    Some(Ok(_)) => continue,
                    _ => break,
                };
                match serde_json::from_str::<In>(&text) {
                    Ok(msg) => handle(&rooms, conn, &tx, &mut joined, msg),
                    Err(e) => error(&tx, &format!("message invalide : {e}")),
                }
            }
        }
    }
    // Gone: a player of a game in progress keeps its place, one in the lobby leaves it (unless
    // another connection took its place over).
    let Some((code, id)) = joined else { return };
    let mut all = rooms.rooms.lock().unwrap();
    let Some(room) = all.get_mut(&code) else { return };
    let Some(k) = room.index(id).filter(|&k| room.players[k].conn == conn) else { return };
    if room.started {
        room.players[k].tx = None;
        if room.players.iter().all(|p| p.tx.is_none()) {
            room.empty_since = Some(Instant::now());
        }
    } else {
        room.players.remove(k);
        if room.host == id
            && let Some(p) = room.players.first()
        {
            room.host = p.id;
        }
        if room.players.is_empty() {
            all.remove(&code);
        } else {
            room.send_lobby(&code);
        }
    }
}

fn handle(rooms: &Arc<Rooms>, conn: u64, tx: &UnboundedSender<String>, joined: &mut Option<(String, u64)>, msg: In) {
    let mut all = rooms.rooms.lock().unwrap();
    match msg {
        In::Create { .. } | In::Join { .. } | In::Rejoin { .. } if joined.is_some() => error(tx, "déjà dans une partie"),
        In::Create { name } => {
            let code = loop {
                let mut n = rooms.random(1);
                let code: String = (0..CODE_LEN)
                    .map(|_| {
                        let c = CODE_CHARS[(n % CODE_CHARS.len() as u64) as usize] as char;
                        n /= CODE_CHARS.len() as u64;
                        c
                    })
                    .collect();
                if !all.contains_key(&code) {
                    break code;
                }
            };
            let room = Room { players: vec![rooms.player(conn, &name, tx)], host: conn, ..Default::default() };
            room.send_lobby(&code);
            all.insert(code.clone(), room);
            *joined = Some((code, conn));
        }
        In::Join { room: code, name } => {
            let code = code.trim().to_uppercase();
            let Some(room) = all.get_mut(&code) else { return error(tx, "partie introuvable") };
            if room.started {
                return error(tx, "la partie a déjà commencé");
            }
            // everyone starts in the first fief, together
            room.players.push(rooms.player(conn, &name, tx));
            room.send_lobby(&code);
            *joined = Some((code, conn));
        }
        In::Rejoin { room: code, token } => {
            let code = code.trim().to_uppercase();
            let place = all.get_mut(&code).filter(|r| r.started).and_then(|r| Some((r.players.iter().position(|p| p.token == token)?, r)));
            let Some((k, room)) = place else { return error(tx, "plus dans cette partie") };
            let p = &mut room.players[k];
            p.conn = conn;
            p.tx = Some(tx.clone());
            p.rejoining = true;
            *joined = Some((code, p.id));
            room.empty_since = None;
            let fiefs: Vec<usize> = room.players.iter().map(|p| p.fief).collect();
            room.send(k, &json!({"t": "start", "items": room.items, "seed": room.seed, "players": fiefs, "you": k, "resume": true}));
        }
        msg => {
            let Some((code, id)) = joined.as_ref() else { return error(tx, "pas dans une partie") };
            let (code, id) = (code.as_str(), *id);
            let Some(room) = all.get_mut(code) else { return };
            let Some(k) = room.index(id) else { return };
            if room.players[k].conn != conn {
                return error(tx, "partie reprise depuis une autre connexion");
            }
            match msg {
                In::Fief { fief } if !room.started && (1..=room.fiefs()).contains(&fief) => {
                    room.players[k].fief = fief;
                    room.send_lobby(code);
                }
                In::Settings { items } if !room.started && room.host == id => {
                    room.items = items.into_iter().filter(|o| SETTINGS.iter().any(|s| o.starts_with(&format!("{s}=")))).collect();
                    let fiefs = room.fiefs();
                    for p in &mut room.players {
                        p.fief = p.fief.min(fiefs);
                    }
                    room.send_lobby(code);
                }
                In::Start if !room.started && room.host == id => {
                    room.started = true;
                    room.started_at = Some(Instant::now());
                    room.seed = room.items.iter().find_map(|o| o.strip_prefix("seed=")?.parse::<u32>().ok()).unwrap_or(rooms.random(2) as u32);
                    room.items.retain(|o| !o.starts_with("seed="));
                    let fiefs: Vec<usize> = room.players.iter().map(|p| p.fief).collect();
                    for k in 0..room.players.len() {
                        room.send(k, &json!({"t": "start", "items": room.items, "seed": room.seed, "players": fiefs, "you": k}));
                    }
                    tokio::spawn(ticker(rooms.clone(), code.to_owned()));
                }
                In::Ready if room.started => room.set_ready(k),
                In::Since { have } if room.started => {
                    for text in room.log.iter().skip(have) {
                        if let Some(tx) = &room.players[k].tx {
                            let _ = tx.send(text.clone());
                        }
                    }
                    room.players[k].rejoining = false;
                    room.set_ready(k);
                }
                In::Order { order } if room.started => room.pending.push((k, order)),
                In::Hash { tick, hash } if room.started => {
                    let first = *room.hashes.entry(tick).or_insert(hash);
                    room.hashes.retain(|&t, _| t + 1000 > tick);
                    if first != hash && !room.desync {
                        room.desync = true;
                        room.broadcast(&json!({"t": "desync", "tick": tick}));
                    }
                }
                _ => error(tx, "pas maintenant"),
            }
        }
    }
}

/// A name of 1 to 24 printable characters.
fn clean(name: &str) -> String {
    let name: String = name.chars().filter(|c| !c.is_control()).take(24).collect();
    if name.trim().is_empty() { "Joueur".to_owned() } else { name.trim().to_owned() }
}

/// Once the players are ready, broadcasts a tick every `TICK` while the room lasts.
async fn ticker(rooms: Arc<Rooms>, code: String) {
    loop {
        {
            let mut all = rooms.rooms.lock().unwrap();
            let Some(room) = all.get(&code) else { return };
            if room.abandoned() {
                all.remove(&code);
                return;
            }
            if room.all_ready() {
                break;
            }
        }
        tokio::time::sleep(TICK / 4).await;
    }
    let mut interval = tokio::time::interval(TICK);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    for n in 0u64.. {
        interval.tick().await;
        let mut all = rooms.rooms.lock().unwrap();
        let Some(room) = all.get_mut(&code) else { return };
        if room.abandoned() {
            all.remove(&code);
            return;
        }
        let orders: Vec<Value> = room.pending.drain(..).map(|(k, o)| json!([k, o])).collect();
        let text = json!({"t": "tick", "n": n, "orders": orders}).to_string();
        for tx in room.players.iter().filter(|p| !p.rejoining).filter_map(|p| p.tx.as_ref()) {
            let _ = tx.send(text.clone());
        }
        room.log.push(text);
    }
}
