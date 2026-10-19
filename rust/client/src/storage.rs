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
