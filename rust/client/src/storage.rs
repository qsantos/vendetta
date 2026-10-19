//! Where the game in progress is kept: the browser's local storage (through `index.html`),
//! or a file natively.

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn vendetta_save_store(data: *const u8, len: usize);
    /// Size of the stored save, 0 for none; the page keeps it decoded for `vendetta_save_read`.
    fn vendetta_save_size() -> usize;
    fn vendetta_save_read(data: *mut u8);
    fn vendetta_save_clear();
    fn vendetta_reload();
}

#[cfg(not(target_arch = "wasm32"))]
const FILE: &str = "vendetta.sav";

pub fn store(bytes: &[u8]) {
    #[cfg(target_arch = "wasm32")]
    // SAFETY: the page copies the bytes out of wasm memory before returning.
    unsafe {
        vendetta_save_store(bytes.as_ptr(), bytes.len())
    };
    #[cfg(not(target_arch = "wasm32"))]
    if let Err(e) = std::fs::write(FILE, bytes) {
        eprintln!("saving {FILE}: {e}");
    }
}

pub fn load() -> Option<Vec<u8>> {
    #[cfg(target_arch = "wasm32")]
    {
        // SAFETY: the page writes exactly `len` bytes into the buffer.
        let len = unsafe { vendetta_save_size() };
        if len == 0 {
            return None;
        }
        let mut bytes = vec![0; len];
        unsafe { vendetta_save_read(bytes.as_mut_ptr()) };
        Some(bytes)
    }
    #[cfg(not(target_arch = "wasm32"))]
    std::fs::read(FILE).ok()
}

pub fn clear() {
    #[cfg(target_arch = "wasm32")]
    // SAFETY: a plain call into the page.
    unsafe {
        vendetta_save_clear()
    };
    #[cfg(not(target_arch = "wasm32"))]
    let _ = std::fs::remove_file(FILE);
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
