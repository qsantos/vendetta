//! Network play (see the `server` crate): the page holds the connection; orders go to the
//! server instead of the world, and the world steps once per tick the server sends, with the
//! orders of every player in it. Only in the browser.

use serde_json::{Value, json};
use sim::world::{Order, World};
use std::collections::VecDeque;

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn vendetta_net_send(data: *const u8, len: usize);
    /// Size of the next message from the server, 0 for none; kept for `vendetta_read`.
    fn vendetta_net_size() -> usize;
    fn vendetta_read(data: *mut u8);
}

fn send(msg: &Value) {
    let text = msg.to_string();
    #[cfg(target_arch = "wasm32")]
    // SAFETY: the page copies the text out of wasm memory before returning.
    unsafe {
        vendetta_net_send(text.as_ptr(), text.len())
    };
    #[cfg(not(target_arch = "wasm32"))]
    let _ = text;
}

fn receive() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        // SAFETY: the page writes exactly the announced length into the buffer.
        let len = unsafe { vendetta_net_size() };
        if len == 0 {
            return None;
        }
        let mut bytes = vec![0; len];
        unsafe { vendetta_read(bytes.as_mut_ptr()) };
        String::from_utf8(bytes).ok()
    }
    #[cfg(not(target_arch = "wasm32"))]
    None
}

/// How often the world's digest is reported, in ticks.
const DIGEST_EVERY: u64 = 50;

pub struct Net {
    /// Each player's character, by their place in the room.
    humans: Vec<Option<usize>>,
    me: usize,
    /// The ticks received and not played yet, each with its orders.
    ticks: VecDeque<Vec<Order>>,
    /// How many ticks were received (played or queued).
    received: u64,
    /// Own orders sent and not back yet.
    in_flight: usize,
    /// The tick where the copies of the world differed, when the server said so.
    pub desync: Option<u64>,
    /// The connection was lost (the page says so, and reconnects).
    pub closed: bool,
    /// The page reconnected: the game goes on.
    pub reconnected: bool,
    /// How many players are ready, of how many, before the first tick.
    pub waiting: (u64, u64),
}

impl Net {
    /// `have`: how many ticks the world played (from a save, or none).
    pub fn new(world: &World, players: &[usize], me: usize, have: u64) -> Net {
        Net { humans: world.humans(players), me, ticks: VecDeque::new(), received: have, in_flight: 0, desync: None, closed: false, reconnected: false, waiting: (0, 0) }
    }

    /// The local player's character.
    pub fn player(&self) -> Option<usize> {
        self.humans.get(self.me).copied().flatten()
    }

    /// The world is made: the ticks may start (once the others are ready too).
    pub fn ready(&self) {
        send(&json!({"t": "ready"}));
    }

    /// Back in a game in progress: the ticks after those received.
    pub fn since(&self) {
        send(&json!({"t": "since", "have": self.received}));
    }

    pub fn send_order(&mut self, order: &Order) {
        match serde_json::to_value(order) {
            Ok(order) => {
                send(&json!({"t": "order", "order": order}));
                self.in_flight += 1;
            }
            Err(e) => {
                macroquad::logging::warn!("order not sent: {e}");
            }
        }
    }

    /// Whether every order sent came back: automatic orders wait for it, so that the same need
    /// is not ordered again while the first order travels.
    pub fn settled(&self) -> bool {
        self.in_flight == 0
    }

    /// Takes in what the server sent.
    pub fn poll(&mut self) {
        while let Some(text) = receive() {
            let Ok(msg) = serde_json::from_str::<Value>(&text) else { continue };
            match msg["t"].as_str() {
                Some("tick") => {
                    // each one once, in order (none twice after a reconnection)
                    let n = msg["n"].as_u64().unwrap_or(0);
                    if n != self.received {
                        if n > self.received {
                            macroquad::logging::warn!("tick {} received before {}", n, self.received);
                        }
                        continue;
                    }
                    self.received = n + 1;
                    let mut orders = Vec::new();
                    for o in msg["orders"].as_array().into_iter().flatten() {
                        let k = o[0].as_u64().unwrap_or(u64::MAX) as usize;
                        if k == self.me {
                            self.in_flight = self.in_flight.saturating_sub(1);
                        }
                        // an order counts only for its sender's own character
                        match serde_json::from_value::<Order>(o[1].clone()) {
                            Ok(order) if self.humans.get(k).copied().flatten() == Some(order.perso()) => orders.push(order),
                            _ => {}
                        }
                    }
                    self.ticks.push_back(orders);
                }
                Some("desync") => self.desync = self.desync.or(msg["tick"].as_u64()),
                Some("closed") => self.closed = true,
                // the page reconnected (see `index.html`): the orders sent meanwhile are lost
                Some("start") => {
                    self.since();
                    self.in_flight = 0;
                    self.reconnected = true;
                }
                Some("wait") => self.waiting = (msg["ready"].as_u64().unwrap_or(0), msg["players"].as_u64().unwrap_or(0)),
                _ => {}
            }
        }
    }

    /// The next tick's orders, if it came.
    pub fn next_tick(&mut self) -> Option<Vec<Order>> {
        self.ticks.pop_front()
    }

    pub fn backlog(&self) -> usize {
        self.ticks.len()
    }

    /// After a tick: now and then, the world's digest for the server to compare (not while
    /// catching up, as the digest costs a save).
    pub fn after_tick(&self, world: &World) {
        if world.tick % DIGEST_EVERY == 0 && self.ticks.len() < 10 {
            send(&json!({"t": "hash", "tick": world.tick, "hash": world.digest()}));
        }
    }
}
