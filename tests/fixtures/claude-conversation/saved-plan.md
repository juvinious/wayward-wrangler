Here's a draft plan for a Rust hello world. It's only a plan: I can't run any of these steps in this increment, and this doesn't create or authorize a task.

**Goal:** A minimal Rust program that prints `Hello, world!` and runs with Cargo.

**Steps**

1. **Check the toolchain.** Confirm `rustc` and `cargo` are installed (`rustc --version`, `cargo --version`). If not, install via `rustup`.
2. **Create the project.** Run `cargo new hello_world`. This generates `Cargo.toml` and `src/main.rs`.
3. **Write the code.** `src/main.rs` should contain:
   ```rust
   fn main() {
       println!("Hello, world!");
   }
   ```
4. **Build and run.** From the project directory, run `cargo run`. Expected output: `Hello, world!`
5. **Verify.** Optionally run `cargo build --release` and execute `target/release/hello_world`. You can also run `cargo clippy` and `cargo fmt --check` for lint and format checks.

**Done when:** `cargo run` prints `Hello, world!` and exits with code 0.

**Open questions that could change the plan**

- Do you want a binary crate (as above) or a library with a test?
- Should it live in a specific directory or an existing repo?
- Do you want extras like a `README`, a unit or integration test, or CI config?

If you tell me your preferences, I can revise the plan. Note that actually executing it would need worker capabilities that aren't available here, so you'd run the steps yourself for now.