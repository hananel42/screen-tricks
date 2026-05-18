#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod overlay;

use std::process;
use lexopt::ValueExt;
use overlay::{Canvas, CaptureSession, OverlayApp, EventResult, OverlayEvent, OverlayContext, run, FrameImage};



#[derive(Clone)]
struct PhysicParticleInfo {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
}
struct WaitingParticle {
    x:i32,
    y:i32,
    w:i32,
    h:i32,
    delay:f32
}
enum Particle {
    Image(FrameImage,PhysicParticleInfo),
    Waiting(WaitingParticle)
}

struct State {
    particles: Vec<Particle>,
    revealed_px: i32,
    time_accum: f32,
    freeze:bool,
}
impl State {
    fn new() -> Self {
        Self {
            particles: Vec::with_capacity(20_000),
            revealed_px: 0,
            time_accum: 0.0,
            freeze: false,
        }
    }
}

fn hash_u32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    x
}
fn jitter(seed: u32, amount: f32) -> f32 {
    let r = (hash_u32(seed) as f32) / (u32::MAX as f32);
    (r - 0.5) * 2.0 * amount
}

fn clamp_rect(x: i32, y: i32, w: i32, h: i32, fw: i32, fh: i32) -> (i32, i32, i32, i32) {
    let x = x.clamp(0, fw.max(1) - 1);
    let y = y.clamp(0, fh.max(1) - 1);
    let w = w.min(fw - x).max(1);
    let h = h.min(fh - y).max(1);
    (x, y, w, h)
}

fn spawn_tile_particle(state: &mut State, width:i32,height:i32, src_x: i32, src_y: i32,settings: &Settings) {
    let (x, y, w, h) = clamp_rect(src_x, src_y, settings.tile_size, settings.tile_size, width, height);

    let seed = (x as u32)
        ^ ((y as u32).wrapping_mul(0x9E37_79B9))
        ^ ((state.revealed_px as u32).wrapping_mul(0x85EB_CA6B));

    let delay = jitter(seed ^ 0xA53A_9F1C, settings.hold_jitter).abs();


    state.particles.push(Particle::Waiting (WaitingParticle{ x,y,w,h,delay }));
}

fn spawn_band_particles(state: &mut State, width:i32,height:i32, band_top_y: i32, band_h: i32,settings: &Settings) {


    let actual_h = band_h.clamp(1, height.max(1));
    let top_y = band_top_y.clamp(0, height - actual_h);

    for sy in (0..actual_h).step_by(settings.tile_size as usize) {
        let src_y = top_y + sy;
        for sx in (0..width).step_by(settings.tile_size as usize) {
            spawn_tile_particle(state, width,height, sx, src_y,settings);
        }
    }

    if state.particles.len() > settings.max_particles {
        let excess = state.particles.len() - settings.max_particles;
        state.particles.drain(0..excess);
    }
}

struct App {
    capture:CaptureSession,
    state:State,
    settings:Settings

}
impl App {
    fn new(settings:Settings) -> Self {
        let capture = CaptureSession::new().expect("failed to init capture");
        let state = State::new();
        App{capture, state,settings}
    }
    fn reset(&mut self) {
        self.state = State::new();
    }
}

impl OverlayApp for App{

    fn handler(&mut self, event: OverlayEvent, c: &mut OverlayContext) -> EventResult {
        match event {
            OverlayEvent::KeyDown { vk } => {
                match vk {
                    0x1B => { c.close() } //ESC
                    0x20 => {
                        self.state.freeze = !self.state.freeze;
                        return EventResult::Consumed
                    } //SPACE
                    0x52 => {
                        self.reset();
                        return EventResult::Consumed
                    } //R
                    _ => {}
                }


            }
            OverlayEvent::KeyUp { .. } => {}
            OverlayEvent::MouseMove { .. } => {}
            OverlayEvent::MouseDown { .. } => {}
            OverlayEvent::MouseUp { .. } => {}
            OverlayEvent::MouseWheel { .. } => {}
        };
        EventResult::Propagated
    }
    fn update(&mut self, overlay_context: &mut OverlayContext, delta: f32) {
        let fw = overlay_context.width();
        let fh = overlay_context.height();
        if let Some(frame) = self.capture.capture() {

            if !self.state.freeze {
                self.state.time_accum += delta;
            }

            while self.state.time_accum >= self.settings.seconds_per_step && self.state.revealed_px < fh && !self.state.freeze {
                let band_h = self.settings.tile_size.min(fh - self.state.revealed_px);
                let band_top_y = fh - self.state.revealed_px - band_h;
                spawn_band_particles(&mut self.state, fw, fh, band_top_y, band_h,&self.settings);
                self.state.revealed_px += band_h;
                self.state.time_accum -= self.settings.seconds_per_step;
            }


            // physics
            for p in &mut self.state.particles {
                match p {
                    Particle::Image(_, PhysicParticleInfo { x, y, vx, vy }) => {
                        *vy += self.settings.gravity * delta;
                        *vx *= self.settings.drag_x.powf(delta);
                        *vy *= self.settings.drag_y.powf(delta);
                        *x += *vx * delta;
                        *y += *vy * delta;
                    }
                    Particle::Waiting(WaitingParticle { x, y, w, h, delay }) => {
                        if !self.state.freeze {
                            *delay -= delta;
                        }
                        if *delay < 0.0 {
                            let seed = (*x as u32)
                                ^ ((*y as u32).wrapping_mul(0x9E37_79B9))
                                ^ ((self.state.revealed_px as u32).wrapping_mul(0x85EB_CA6B));
                            *p = Particle::Image(frame.crop(*x, *y, *w, *h).unwrap().to_owned(), PhysicParticleInfo {
                                x: *x as f32,
                                y: *y as f32,
                                vx: jitter(seed.rotate_left(5), self.settings.vx_jitter),
                                vy: jitter(seed.rotate_left(11), self.settings.vy_jitter)
                            })
                        }
                    }
                }
            }
        }
        //deAlloc the particles which is out of the screen
        self.state.particles.retain(|p| {
            match p {
                Particle::Image(_, PhysicParticleInfo { x, y, .. }) => {
                    *x > -self.settings.tile_size as f32
                        && *x < fw as f32 + self.settings.tile_size as f32
                        && *y < fh as f32 + self.settings.tile_size as f32
                }
                Particle::Waiting(_) => { true }
            }
        });
    }

fn render(&mut self, canvas: &mut Canvas) {
    canvas.clear();

    let dark_h = self.state.revealed_px;
        if dark_h > 0 {
            canvas.fill_rect(
                0,
                canvas.height - dark_h,
                canvas.width,
                dark_h,
                (0, 0, 0, self.settings.darken_alpha),
            );
        }

        for p in &self.state.particles {

            match &p {
                Particle::Image(img, PhysicParticleInfo{x,y,..}) => {
                    canvas.draw_image(img,*x as i32,*y as i32);
                }
                Particle::Waiting(WaitingParticle{x,y,..}) => {    canvas.clear_rect(*x,*y,self.settings.tile_size,self.settings.tile_size)}
            }




        }

    }
}





struct Settings {
    tile_size: i32,
    hold_jitter: f32,
    vx_jitter: f32,
    vy_jitter: f32,
    gravity: f32,
    drag_x: f32,
    drag_y: f32,
    darken_alpha: u8,
    max_particles: usize,
    seconds_per_step: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            tile_size: 16,
            hold_jitter: 0.7,
            vx_jitter: 10.0,
            vy_jitter: 10.0,
            gravity: 0.0,
            drag_x: 0.995,
            drag_y: 0.998,
            darken_alpha: 255,
            max_particles: 25_000_000,
            seconds_per_step: 0.0,
        }
    }
}


fn parse_args() -> Result<Settings, lexopt::Error> {
    let mut parser = lexopt::Parser::from_env();
    let mut settings = Settings::default();

    while let Some(arg) = parser.next()? {
        match arg {
            lexopt::Arg::Long("tile-size") => {
                settings.tile_size = parser.value()?.parse()?;
            }
            lexopt::Arg::Long("hold-jitter") => {
                settings.hold_jitter = parser.value()?.parse()?;
            }
            lexopt::Arg::Long("vx-jitter") => {
                settings.vx_jitter = parser.value()?.parse()?;
            }
            lexopt::Arg::Long("vy-jitter") => {
                settings.vy_jitter = parser.value()?.parse()?;
            }
            lexopt::Arg::Long("gravity") => {
                settings.gravity = parser.value()?.parse()?;
            }
            lexopt::Arg::Long("drag-x") => {
                settings.drag_x = parser.value()?.parse()?;
            }
            lexopt::Arg::Long("drag-y") => {
                settings.drag_y = parser.value()?.parse()?;
            }
            lexopt::Arg::Long("darken-alpha") => {
                settings.darken_alpha = parser.value()?.parse()?;
            }
            lexopt::Arg::Long("max-particles") => {
                settings.max_particles = parser.value()?.parse()?;
            }
            lexopt::Arg::Long("seconds-per-step") => {
                settings.seconds_per_step = parser.value()?.parse()?;
            }
            lexopt::Arg::Short('h') | lexopt::Arg::Long("help") => {
                print_help();
                process::exit(0);
            }
            _ => return Err(arg.unexpected()),
        }
    }

    Ok(settings)
}

fn print_help() {
    println!("Screen Disintegrator Options:");
    println!("  --tile-size <int>         Size of tiles (default: 16)");
    println!("  --hold-jitter <float>     Hold jitter duration (default: 0.7)");
    println!("  --vx-jitter <float>       X velocity jitter (default: 10.0)");
    println!("  --vy-jitter <float>       Y velocity jitter (default: 10.0)");
    println!("  --gravity <float>         Gravity in px/s^2 (default: 0.0)");
    println!("  --drag-x <float>          X drag coefficient (default: 0.995)");
    println!("  --drag-y <float>          Y drag coefficient (default: 0.998)");
    println!("  --darken-alpha <int>      Darken alpha 0-255 (default: 255)");
    println!("  --max-particles <int>     Max particle count (default: 25000000)");
    println!("  --seconds-per-step <f32>  Seconds per step (default: 0.0)");
    println!("  -h, --help                Print this help message");
}

fn main() {

    let settings = match parse_args() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error parsing arguments: {}", e);
            eprintln!("Run with --help for usage.");
            process::exit(1);
        }
    };


    run(App::new(settings));
}
