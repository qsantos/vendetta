//! usage: server [PORT] [DIST]
//!
//! Listens on 127.0.0.1:PORT (8766 by default) for `/ws`; with DIST, also serves the built
//! page from that folder, so that one process is enough to play locally. In production a web
//! server serves the page and forwards `/ws` here.

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let port: u16 = args.next().map_or(8766, |p| p.parse().expect("PORT"));
    let dist = args.next().map(std::path::PathBuf::from);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await.expect("bind");
    println!("listening on http://127.0.0.1:{port}/");
    axum::serve(listener, server::app(dist)).await.expect("serve");
}
