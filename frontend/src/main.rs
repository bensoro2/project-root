#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("Run `trunk serve --open` to launch the web app.");
}

#[cfg(target_arch = "wasm32")]
fn main() {}