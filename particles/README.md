# Screen Disintegrator
![preview](../assets/particles.gif)
The GIF compression ruined the quality a bit; it looks much better in real-time!

# Usage
if you have cargo and git:
```cmd
git clone https://github.com/hananel42/screen-tricks.git
cd screen-tricks
cargo run --release -p particles
```

the exe file will be found at `target/release/particles.exe`


* Try playing with the parameters! For example, try changing gravity or making the tiles smaller.

Command line args:
```
  -r, --random              starting with random values
  --tile-size <int>         Size of tiles (default: 16)
  --hold-jitter <float>     Hold jitter duration (default: 0.7)
  --vx-jitter <float>       X velocity jitter (default: 100.0)
  --vy-jitter <float>       Y velocity jitter (default: 100.0)
  --gravity <float>         Gravity in px/s^2 (default: 2000.0)
  --drag-x <float>          X drag coefficient (default: 0.995)
  --drag-y <float>          Y drag coefficient (default: 0.998)
  --darken-alpha <int>      Darken alpha 0-255 (default: 255)
  --max-particles <int>     Max particle count (default: 25000000)
  --seconds-per-step <f32>  Seconds per step (default: 0.1)
  -h, --help                Print this help message
```

for example :
```cmd
particles.exe --tile-size 4 --hold-jitter 10 --second-per-step 0
```