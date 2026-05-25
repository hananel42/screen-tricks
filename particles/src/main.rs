#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn AttachConsole(dw_process_id: u32) -> i32;
}
const ATTACH_PARENT_PROCESS: u32 = 0xFFFF_FFFF;

use random::Random;

use lexopt::ValueExt;
use overlay::{
    Canvas, EventResult, OverlayApp, OverlayContext, OverlayEvent, run,
    capture::{
        CaptureSession,FrameImage
    }
};
use std::process;

#[derive(Clone)]
struct AliveParticle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    image: FrameImage,
}
struct WaitingParticle {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    delay: f32,
}

struct State {
    waiting_particles: Vec<WaitingParticle>,
    alive_particles: Vec<AliveParticle>,
    revealed_px: i32,
    time_accum: f32,
    freeze: bool,
    skip_update: bool,
}
impl State {
    fn new() -> Self {
        Self {
            waiting_particles: Vec::with_capacity(20_000),
            alive_particles: Vec::with_capacity(20_000),
            revealed_px: 0,
            time_accum: 0.0,
            freeze: false,
            skip_update: false,
        }
    }
}

fn clamp_rect(x: i32, y: i32, w: i32, h: i32, fw: i32, fh: i32) -> (i32, i32, i32, i32) {
    let x = x.clamp(0, fw.max(1) - 1);
    let y = y.clamp(0, fh.max(1) - 1);
    let w = w.min(fw - x).max(1);
    let h = h.min(fh - y).max(1);
    (x, y, w, h)
}

struct App {
    capture: CaptureSession,
    state: State,
    random: Random,
    settings: Settings,
}
impl App {
    fn new(settings: Settings) -> Self {
        let capture = CaptureSession::new().expect("failed to init capture");
        let state = State::new();
        let random = Random::new();
        App {
            capture,
            state,
            settings,
            random,
        }
    }

    fn spawn_band_particles(&mut self, width: i32, height: i32, band_top_y: i32, band_h: i32) {
        let actual_h = band_h.clamp(1, height.max(1));
        let top_y = band_top_y.clamp(0, height - actual_h);

        for sy in (0..actual_h).step_by(self.settings.tile_size as usize) {
            let src_y = top_y + sy;
            for sx in (0..width).step_by(self.settings.tile_size as usize) {
                self.spawn_tile_particle(width, height, sx, src_y);
            }
        }
    }

    fn spawn_tile_particle(&mut self, width: i32, height: i32, src_x: i32, src_y: i32) {
        let (x, y, w, h) = clamp_rect(
            src_x,
            src_y,
            self.settings.tile_size,
            self.settings.tile_size,
            width,
            height,
        );

        let delay = self.random.positive_jitter(self.settings.hold_jitter);

        self.state
            .waiting_particles
            .push(WaitingParticle { x, y, w, h, delay });
    }
    fn reset(&mut self) {
        self.state.alive_particles.clear();
        self.state.waiting_particles.clear();
        self.state.revealed_px = 0;
        self.state.time_accum = 0.0;
        self.state.freeze = false;
        self.state.skip_update = true;
    }
}

impl OverlayApp for App {
    fn init(&mut self, overlay_context: &mut OverlayContext) {
        overlay_context.hide_from_capture(true);
    }
    fn handler(&mut self, event: OverlayEvent, c: &mut OverlayContext) -> EventResult {
        match event {
            OverlayEvent::KeyDown { vk } => {
                match vk {
                    0x1B => c.close(), //ESC - Exit
                    0x20 => {
                        self.state.freeze = !self.state.freeze;
                        return EventResult::Consumed;
                    } //SPACE - Stop
                    0x52 => {
                        self.reset();
                        return EventResult::Consumed;
                    } //R - Reset

                    x if x == 'D' as u32 => {
                        println!("----Debug----");
                        println!("alive particles: {:#?}", self.state.alive_particles.len());
                        println!(
                            "waiting particles: {:#?}",
                            self.state.waiting_particles.len()
                        );
                        println!("screen size: {},{}", c.height(), c.width());
                        println!("tile size: {}", self.settings.tile_size);
                        return EventResult::Consumed;
                    } //D - Debug
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
        if self.state.skip_update {
            self.state.skip_update = false;
            return;
        }
        let fw = overlay_context.width();
        let fh = overlay_context.height();

        if !self.state.freeze {
            self.state.time_accum += delta;
        }

        while self.state.time_accum >= self.settings.seconds_per_step
            && self.state.revealed_px < fh
            && !self.state.freeze
        {
            let band_h = self.settings.tile_size.min(fh - self.state.revealed_px);
            let band_top_y = fh - self.state.revealed_px - band_h;
            self.spawn_band_particles(fw, fh, band_top_y, band_h);
            self.state.revealed_px += band_h;
            self.state.time_accum -= self.settings.seconds_per_step;
        }

        for particle in &mut self.state.alive_particles {
            let AliveParticle { x, y, vx, vy, .. } = particle;
            *vy += self.settings.gravity * delta;
            *vx *= self.settings.drag_x.powf(delta);
            *vy *= self.settings.drag_y.powf(delta);
            *x += *vx * delta;
            *y += *vy * delta;
        }

        if let Some(frame) = self.capture.capture()
            && !self.state.freeze
        {
            self.state
                .waiting_particles
                .retain_mut(|WaitingParticle { x, y, w, h, delay }| {
                    *delay -= delta;
                    if *delay < 0.0
                        && self.state.alive_particles.len() < self.settings.max_particles
                    {
                        self.state.alive_particles.push(AliveParticle {
                            x: *x as f32,
                            y: *y as f32,
                            vx: self.random.jitter(self.settings.vx_jitter),
                            vy: self.random.jitter(self.settings.vy_jitter),
                            image: frame.crop(*x, *y, *w, *h).unwrap().to_owned(),
                        });
                        false
                    } else {
                        true
                    }
                });
        }

        //deAlloc the particles which is out of the screen
        self.state
            .alive_particles
            .retain(|AliveParticle { x, y, .. }| {
                *x > -self.settings.tile_size as f32
                    && *x < fw as f32 + self.settings.tile_size as f32
                    && *y < fh as f32 + self.settings.tile_size as f32
            });
    }

    fn render(&mut self, canvas: &mut Canvas) {
        canvas.clear();

        let dark_h = self.state.revealed_px;
        if dark_h > 0 {
            canvas.fill_rect(
                0,
                canvas.height() - dark_h,
                canvas.width(),
                dark_h,
                (0, 0, 0, self.settings.darken_alpha),
            );
        }

        for WaitingParticle { x, y, .. } in self.state.waiting_particles.iter_mut() {
            canvas.clear_rect(*x, *y, self.settings.tile_size, self.settings.tile_size)
        }
        for AliveParticle { x, y, image, .. } in &self.state.alive_particles {
            canvas.draw_image(image, *x as i32, *y as i32);
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
            vx_jitter: 100.0,
            vy_jitter: 100.0,
            gravity: 2000.0,
            drag_x: 0.995,
            drag_y: 0.998,
            darken_alpha: 255,
            max_particles: 25_000_000,
            seconds_per_step: 0.1,
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
            lexopt::Arg::Short('r') | lexopt::Arg::Long("random") => {
                let mut random = Random::new();
                settings = Settings {
                    tile_size: *random.choose(&[1, 4, 16, 32, 64, 256]),
                    hold_jitter: *random.choose(&[1.0, 0.1, 0.0, 0.7, 2.0]),
                    vx_jitter: random.positive_jitter(700.0),
                    vy_jitter: random.positive_jitter(700.0),
                    gravity: random.range(-1000.0, 3000.0),
                    drag_x: random.range(0.6, 1.0),
                    drag_y: random.range(0.6, 1.0),
                    darken_alpha: random.integer(100) as u8 + 155,
                    seconds_per_step: *random.choose(&[0.0, 0.05, 1.0, 0.1]),
                    ..settings
                }
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
    println!("  --vx-jitter <float>       X velocity jitter (default: 100.0)");
    println!("  --vy-jitter <float>       Y velocity jitter (default: 100.0)");
    println!("  --gravity <float>         Gravity in px/s^2 (default: 2000.0)");
    println!("  --drag-x <float>          X drag coefficient (default: 0.995)");
    println!("  --drag-y <float>          Y drag coefficient (default: 0.998)");
    println!("  --darken-alpha <int>      Darken alpha 0-255 (default: 255)");
    println!("  --max-particles <int>     Max particle count (default: 25000000)");
    println!("  --seconds-per-step <f32>  Seconds per step (default: 0.1)");
    println!("  -h, --help                Print this help message");
}

fn main() {
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }

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
