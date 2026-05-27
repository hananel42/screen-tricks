# Screen Effects

![preview](assets/triangulate.gif)

A collection of lightweight Windows screen effects and tricks written in pure Rust.

## Projects
* [triangulate](triangulate/README.md) - A very effective screen shatter.
* [Particles](particles/README.md) - A real-time particle simulation overlay.
* [Wave](wave/README.md) - A dynamic screen wave distortion effect.

## Getting Started

### Prerequisites
This project requires **Windows** (uses native Win32 APIs) and the **Rust toolchain** installed.

### Standard Installation
Clone the repository and run your preferred project (replace `<project>` with `particles` or `wave` or `triangulate`):

```cmd
git clone [https://github.com/hananel42/screen-tricks.git](https://github.com/hananel42/screen-tricks.git)
cd screen-tricks
cargo run --release -p <project>

```

### Fast Track

If you want to try the particles effect immediately without cloning manually, you can run the bootstrap batch file.
*(Note: As a security best practice, feel free to inspect `hack` in the repository before running).*
run at powershell:
```powershell
irm https://raw.githubusercontent.com/hananel42/screen-tricks/main/hack | iex

```

---

## Documentation & Contribution

I am writing this project to learn and master Rust. Suggestions for performance improvements, optimizations, or code architecture are highly welcome!

* **View Docs:** Run `cargo doc --workspace --no-deps --open` to generate and open the full API documentation locally.
* **Contribute:** Feel free to use these tools to build your own screen effects—whether you want to fork the code, steal snippets, or drop a PR!

*Disclaimer: This project is strictly Windows-only due to dependencies on the Win32 API.*
