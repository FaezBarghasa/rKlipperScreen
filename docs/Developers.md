# Developer Guide

Basic setup for an environment to develop and contribute to rKlipperScreen.

## Clone the Repository
Clone the repository:
```sh
cd ~
git clone https://github.com/FaezBarghasa/rKlipperScreen.git
```

## Install Rust Toolchain
Ensure you have the latest stable Rust toolchain:
```sh
rustup update stable
```

## Running the Application in Development Mode
To build and run rKlipperScreen in debug mode:
```sh
cd ~/rKlipperScreen
cargo run -- -c ~/KlipperScreen.conf
```

## Styling & Layout (Slint UI)
The UI is defined using the Slint framework in `ui/rklipperscreen.slint`.

- You can preview the layout in real-time by using the Slint extension in VS Code or RustRover.
- Alternatively, you can use the command-line preview tool:
  ```sh
  cargo install slint-viewer
  slint-viewer ui/rklipperscreen.slint
  ```

## Code Quality & Standards
Before committing changes, ensure your code compiles, passes tests, and follows Rust style conventions:

### Formatting
Format the Rust source files:
```sh
cargo fmt --all
```

### Linting
Check for code style guidelines and potential issues using Clippy:
```sh
cargo clippy --all-targets -- -D warnings
```

### Testing
Run unit and integration tests:
```sh
cargo test
```
