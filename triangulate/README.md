
# Screen Shatter
![preview](../assets/triangulate.gif)

# Usage
If you have cargo and git:
```cmd
git clone [https://github.com/hananel42/screen-tricks.git](https://github.com/hananel42/screen-tricks.git)
cd screen-tricks
cargo run --release -p triangulate

```

The exe file will be found at `target/release/triangulate.exe`

# CLI Configuration Options

You can customize the physics and simulation behavior by passing optional flags to the application.

### Available Options:

* `-g, --gravity <f32>`: Set environmental gravity force [default: 60.0]
* `-r, --rotation-speed-jitter <f32>`: Set multiplier value for spin jitters [default: 3.0]
* `--max-speed <f32>`: Set absolute maximum explosion limits [default: 500.0]
* `--min-speed <f32>`: Set base minimum structural velocity [default: 120.0]
* `-j, --speed-jitter <f32>`: Randomized speed offset variation [default: 50.0]
* `-p, --points <usize>`: Quantity of triangulation vertices generated [default: 100]
* `-h, --help`: Display the configuration manual and exit

### Example:

```cmd
cargo run --release -p triangulate -- --gravity 120.0 --points 250 --max-speed 700.0
```