fn main() {
    // Compiles the Slint UI into native Rust code.
    // This safely embeds and bundles all layout assets at compile-time to optimize I/O.
    slint_build::compile("ui/rklipperscreen.slint").expect("Failed to compile Slint UI");
}