//! Two clients make a room, pick fiefs, start and exchange orders through the ticks.

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;

type Ws = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn connect(port: u16) -> Ws {
    tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}/ws")).await.unwrap().0
}

async fn send(ws: &mut Ws, msg: Value) {
    ws.send(Message::Text(msg.to_string().into())).await.unwrap();
}

/// The next message of type `t`, skipping others.
async fn next(ws: &mut Ws, t: &str) -> Value {
    loop {
        let msg = tokio::time::timeout(std::time::Duration::from_secs(5), ws.next()).await.expect("timeout").unwrap().unwrap();
        let v: Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
        if v["t"] == t {
            return v;
        }
    }
}

/// No tick for a while.
async fn quiet(ws: &mut Ws) -> bool {
    tokio::time::timeout(Duration::from_millis(400), next(ws, "tick")).await.is_err()
}

async fn serve() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, server::app(None)).await.unwrap() });
    port
}

#[tokio::test]
async fn lobby_and_lockstep() {
    let port = serve().await;

    let mut a = connect(port).await;
    send(&mut a, json!({"t": "create", "name": "Alice"})).await;
    let lobby = next(&mut a, "lobby").await;
    let code = lobby["room"].as_str().unwrap().to_owned();
    assert_eq!(code.len(), 4);
    assert_eq!(lobby["you"], 0);

    // the code in lower case finds the room
    let mut b = connect(port).await;
    send(&mut b, json!({"t": "join", "room": code.to_lowercase(), "name": "Bob"})).await;
    let lobby = next(&mut b, "lobby").await;
    assert_eq!(lobby["you"], 1);
    assert_eq!(lobby["players"][1]["fief"], 1, "the first fief, with the others");

    send(&mut b, json!({"t": "fief", "fief": 3})).await;
    // only the host sets the options
    send(&mut b, json!({"t": "settings", "items": ["fiefs=6"]})).await;
    assert_eq!(next(&mut b, "error").await["t"], "error");
    send(&mut a, json!({"t": "settings", "items": ["fiefs=6", "seed=7", "god"]})).await;
    let lobby = loop {
        let l = next(&mut a, "lobby").await;
        if l["items"] == json!(["fiefs=6", "seed=7"]) && l["players"][1]["fief"] == 3 {
            break l;
        }
    };
    assert_eq!(lobby["host"], 0);

    send(&mut a, json!({"t": "start"})).await;
    let start = next(&mut b, "start").await;
    assert_eq!(start["seed"], 7);
    assert_eq!(start["items"], json!(["fiefs=6"]));
    assert_eq!(start["players"], json!([1, 3]));
    assert_eq!(start["you"], 1);

    // a late comer is turned away
    let mut c = connect(port).await;
    send(&mut c, json!({"t": "join", "room": code, "name": "Carol"})).await;
    assert_eq!(next(&mut c, "error").await["message"], "la partie a déjà commencé");

    // no tick until both worlds are made
    assert!(quiet(&mut a).await);
    send(&mut a, json!({"t": "ready"})).await;
    let wait = next(&mut b, "wait").await;
    assert_eq!((wait["ready"].clone(), wait["players"].clone()), (json!(1), json!(2)));
    assert!(quiet(&mut a).await);
    send(&mut b, json!({"t": "ready"})).await;

    // an order reaches everyone, with its sender, in the same tick
    send(&mut b, json!({"t": "order", "order": {"GoOwnCastle": {"perso": 12}}})).await;
    let find = |t: &Value| t["orders"].as_array().unwrap().first().cloned();
    let (ta, oa) = loop {
        let t = next(&mut a, "tick").await;
        if let Some(o) = find(&t) {
            break (t["n"].clone(), o);
        }
    };
    let (tb, ob) = loop {
        let t = next(&mut b, "tick").await;
        if let Some(o) = find(&t) {
            break (t["n"].clone(), o);
        }
    };
    assert_eq!((ta, oa.clone()), (tb, ob));
    assert_eq!(oa, json!([1, {"GoOwnCastle": {"perso": 12}}]));

    // different digests for one tick: everyone is told
    send(&mut a, json!({"t": "hash", "tick": 50, "hash": 1})).await;
    send(&mut b, json!({"t": "hash", "tick": 50, "hash": 2})).await;
    assert_eq!(next(&mut a, "desync").await["tick"], 50);
}

/// With `wait=2`, the ticks start once the first player's loading time has passed again,
/// without the slow one.
#[tokio::test]
async fn slow_players_are_not_waited_for_forever() {
    let port = serve().await;
    let mut a = connect(port).await;
    send(&mut a, json!({"t": "create", "name": "Alice"})).await;
    let code = next(&mut a, "lobby").await["room"].as_str().unwrap().to_owned();
    let mut b = connect(port).await;
    send(&mut b, json!({"t": "join", "room": code, "name": "Bob"})).await;
    next(&mut b, "lobby").await;
    send(&mut a, json!({"t": "settings", "items": ["wait=2"]})).await;
    send(&mut a, json!({"t": "start"})).await;
    next(&mut a, "start").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    send(&mut a, json!({"t": "ready"})).await;
    // Bob never says it is ready: the ticks come after about 200 ms more
    let t = std::time::Instant::now();
    next(&mut a, "tick").await;
    assert!(t.elapsed() >= Duration::from_millis(150), "{:?}", t.elapsed());
    next(&mut b, "tick").await;
}
