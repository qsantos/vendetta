//! Small persistent blobs (the game in progress, settings): the browser's local storage
//! (through `index.html`), or files natively.

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn vendetta_store(key: *const u8, key_len: usize, data: *const u8, len: usize);
    /// Size of the stored value, 0 for none; the page keeps it decoded for `vendetta_read`.
    fn vendetta_stored_size(key: *const u8, key_len: usize) -> usize;
    fn vendetta_read(data: *mut u8);
    fn vendetta_remove(key: *const u8, key_len: usize);
    fn vendetta_reload();
}

#[cfg(not(target_arch = "wasm32"))]
fn file(key: &str) -> String {
    format!("vendetta-{key}.dat")
}

pub fn store(key: &str, bytes: &[u8]) {
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

pub fn remove(key: &str) {
    #[cfg(target_arch = "wasm32")]
    // SAFETY: the page reads the key out of wasm memory before returning.
    unsafe {
        vendetta_remove(key.as_ptr(), key.len())
    };
    #[cfg(not(target_arch = "wasm32"))]
    let _ = std::fs::remove_file(file(key));
}
/// Starts over from the loading screen: reloads the page, or runs the game again natively.
pub fn restart() -> ! {
    #[cfg(target_arch = "wasm32")]
    {
        // SAFETY: a plain call into the page; the reload replaces this instance.
        unsafe { vendetta_reload() };
        loop {}
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let exe = std::env::current_exe().expect("own executable");
        let _ = std::process::Command::new(exe).args(std::env::args_os().skip(1)).spawn();
        std::process::exit(0)
    }
}
