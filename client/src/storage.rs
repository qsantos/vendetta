//! Small persistent blobs (the game in progress, settings): the browser's local storage
//! (through `index.html`), or files natively.

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn vendetta_store(key: *const u8, key_len: usize, data: *const u8, len: usize);
    /// Size of the stored value, 0 for none; the page keeps it decoded for `vendetta_read`.
    fn vendetta_stored_size(key: *const u8, key_len: usize) -> usize;
    fn vendetta_read(data: *mut u8);
    /// Size of the URL's hash (without '#'), kept for `vendetta_read`.
    fn vendetta_hash_size() -> usize;
    /// Whether a game was chosen in the page's menus (and so its options are known).
    fn vendetta_started() -> bool;
    /// Whether the main pointer is a finger (`pointer: coarse`).
    fn vendetta_touch_screen() -> bool;
    /// The page's in-game menu: opens it; whether it is open; what it asks of the game (0:
    /// nothing, 1: quit, 2/3: zoom the map out/in, 4/5: the interface smaller/bigger, 100 + N:
    /// the speed N); the
    /// game has saved and can be left.
    fn vendetta_open_menu();
    fn vendetta_menu_open() -> bool;
    fn vendetta_menu_command() -> u32;
    fn vendetta_quit();
    /// The volume the menu's sound settings give the music (0) or the sound effects (1),
    /// 0 to 1, 0 when turned off.
    fn vendetta_volume(kind: u32) -> f32;
    /// The interface the menu chose: 0 the screen's, 1 touch, 2 desktop.
    fn vendetta_layout() -> u32;
    /// The language the menu chose: its `Langues.ini` suffix, a letter a byte from the lowest,
    /// or 0 for none.
    fn vendetta_language() -> u32;
    /// The game's speed, for the menu to show (and, in network play, that it cannot change).
    fn vendetta_speed(speed: u32, network: bool);
}

#[cfg(not(target_arch = "wasm32"))]
fn file(key: &str) -> String {
    format!("vendetta-{key}.dat")
}

pub fn store(key: &str, bytes: &[u8]) {
    if crate::options::test_mode() {
        return;
    }
    #[cfg(target_arch = "wasm32")]
    // SAFETY: the page copies the key and bytes out of wasm memory before returning.
    unsafe {
        vendetta_store(key.as_ptr(), key.len(), bytes.as_ptr(), bytes.len())
    };
    #[cfg(not(target_arch = "wasm32"))]
    if let Err(e) = std::fs::write(file(key), bytes) {
        eprintln!("saving {}: {e}", file(key));
    }
}

pub fn load(key: &str) -> Option<Vec<u8>> {
    if crate::options::test_mode() {
        return None;
    }
    #[cfg(target_arch = "wasm32")]
    {
        // SAFETY: the page writes exactly the announced length into the buffer.
        let len = unsafe { vendetta_stored_size(key.as_ptr(), key.len()) };
        if len == 0 {
            return None;
        }
        let mut bytes = vec![0; len];
        unsafe { vendetta_read(bytes.as_mut_ptr()) };
        Some(bytes)
    }
    #[cfg(not(target_arch = "wasm32"))]
    std::fs::read(file(key)).ok()
}

/// Whether a game was chosen in the page's menus; at once natively.
pub fn started() -> bool {
    #[cfg(target_arch = "wasm32")]
    // SAFETY: no arguments.
    return unsafe { vendetta_started() };
    #[cfg(not(target_arch = "wasm32"))]
    true
}

/// Whether the screen is mainly used by touch (a phone or a tablet); never natively.
pub fn touch_screen() -> bool {
    #[cfg(target_arch = "wasm32")]
    // SAFETY: no arguments.
    return unsafe { vendetta_touch_screen() };
    #[cfg(not(target_arch = "wasm32"))]
    false
}

/// What the page's in-game menu asks of the game (natively, nothing).
#[derive(PartialEq)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub enum MenuCommand {
    None,
    /// Back to the pre-game menus, once the game is saved.
    Quit,
    /// The map's zoom a step in (true) or out.
    Zoom(bool),
    /// The interface's size a step bigger (true) or smaller.
    Interface(bool),
    /// The game's speed, in ticks per second.
    Speed(i64),
    /// The page's language changed (see `language`).
    Language,
}

/// Opens the page's in-game menu (natively, there is none).
pub fn open_menu() {
    #[cfg(target_arch = "wasm32")]
    // SAFETY: no arguments.
    unsafe {
        vendetta_open_menu()
    };
}

/// Whether the page's in-game menu is open.
pub fn menu_open() -> bool {
    #[cfg(target_arch = "wasm32")]
    // SAFETY: no arguments.
    return unsafe { vendetta_menu_open() };
    #[cfg(not(target_arch = "wasm32"))]
    false
}

/// The next thing the page's in-game menu asks of the game.
pub fn menu_command() -> MenuCommand {
    #[cfg(target_arch = "wasm32")]
    // SAFETY: no arguments.
    return match unsafe { vendetta_menu_command() } {
        1 => MenuCommand::Quit,
        2 => MenuCommand::Zoom(false),
        3 => MenuCommand::Zoom(true),
        4 => MenuCommand::Interface(false),
        5 => MenuCommand::Interface(true),
        6 => MenuCommand::Language,
        n @ 100.. => MenuCommand::Speed(n as i64 - 100),
        _ => MenuCommand::None,
    };
    #[cfg(not(target_arch = "wasm32"))]
    MenuCommand::None
}

/// The game is saved: the page goes back to its menus (natively, the game ends).
pub fn quit() {
    #[cfg(target_arch = "wasm32")]
    // SAFETY: no arguments.
    unsafe {
        vendetta_quit()
    };
    #[cfg(not(target_arch = "wasm32"))]
    std::process::exit(0);
}

/// Tells the page the game's speed, for its menu.
pub fn show_speed(speed: i64, network: bool) {
    #[cfg(target_arch = "wasm32")]
    // SAFETY: plain numbers.
    unsafe {
        vendetta_speed(speed as u32, network)
    };
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (speed, network);
}

/// The interface the menu chose: touch (true), desktop (false), or none.
pub fn layout() -> Option<bool> {
    #[cfg(target_arch = "wasm32")]
    // SAFETY: no arguments.
    return match unsafe { vendetta_layout() } {
        1 => Some(true),
        2 => Some(false),
        _ => None,
    };
    #[cfg(not(target_arch = "wasm32"))]
    None
}

/// The language the menu chose, by its `Langues.ini` suffix.
pub fn language() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        // SAFETY: no arguments.
        let letters = unsafe { vendetta_language() }.to_le_bytes();
        let suffix: String = letters.iter().take_while(|&&b| b != 0).map(|&b| b as char).collect();
        return (!suffix.is_empty()).then_some(suffix);
    }
    #[cfg(not(target_arch = "wasm32"))]
    None
}

/// What a volume setting is for.
pub enum Sounds {
    Music = 0,
    Effects = 1,
}

/// The volume the player chose for `kind`, 0 to 1 (0 when turned off); natively, full.
pub fn volume(kind: Sounds) -> f32 {
    #[cfg(target_arch = "wasm32")]
    // SAFETY: a plain number.
    return unsafe { vendetta_volume(kind as u32) }.clamp(0.0, 1.0);
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = kind;
        1.0
    }
}

/// The options given to the game: the URL's hash in the browser, the arguments natively.
pub fn options_text() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        // SAFETY: the page writes exactly the announced length into the buffer.
        let len = unsafe { vendetta_hash_size() };
        let mut bytes = vec![0; len];
        if len > 0 {
            unsafe { vendetta_read(bytes.as_mut_ptr()) };
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }
    #[cfg(not(target_arch = "wasm32"))]
    std::env::args().skip(1).collect::<Vec<_>>().join("&")
}
