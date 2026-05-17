#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod overlay;

use overlay::{Canvas, CaptureSession, ImageSource, OverlayApp};
use crate::overlay::FrameImage;

const TILE_SIZE: i32 = 4;
const HOLD_JITTER: i32 = 6;
const VX_JITTER: f32 = 9.0;
const VY_JITTER: f32 = 9.28;
const GRAVITY: f32 = 1.0;
const DRAG_X: f32 = 0.995;
const DRAG_Y: f32 = 0.998;
const DARKEN_ALPHA: u8 = 255;
const MAX_PARTICLES: usize = 25_000_000;
const FRAMES_PER_STEP: f32 = 1.0; //should be less than HOLD_JITTER


#[derive(Clone)]
struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    delay: i32,
    tile: FrameImage,
}
struct State {
    particles: Vec<Particle>,
    revealed_px: i32,
    tick_accum: f32,
}

impl State {
    fn new() -> Self {
        Self {
            particles: Vec::with_capacity(20_000),
            revealed_px: 0,
            tick_accum: 0.0,

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
fn rand_range(seed: u32, max_inclusive: i32) -> i32 {
    if max_inclusive <= 0 {
        return 0;
    }
    (hash_u32(seed) % ((max_inclusive + 1) as u32)) as i32
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


fn spawn_tile_particle(state: &mut State, frame: &impl ImageSource, src_x: i32, src_y: i32) {
    let fw = frame.width();
    let fh = frame.height();

    let (x, y, w, h) = clamp_rect(src_x, src_y, TILE_SIZE, TILE_SIZE, fw, fh);
    let tile = frame.crop(x,y,w,h).unwrap().to_owned();

    let seed = (x as u32)
        ^ ((y as u32).wrapping_mul(0x9E37_79B9))
        ^ ((state.revealed_px as u32).wrapping_mul(0x85EB_CA6B));

    let delay = rand_range(seed ^ 0xA53A_9F1C, HOLD_JITTER);
    let vx = jitter(seed.rotate_left(5), VX_JITTER);
    let vy = jitter(seed.rotate_left(11), VY_JITTER);

    state.particles.push(Particle {
        x: x as f32,
        y: y as f32,
        vx,
        vy,
        delay,
        tile,
    });
}

fn spawn_band_particles(state: &mut State, frame: &impl ImageSource, band_top_y: i32, band_h: i32) {
    let fw = frame.width();
    let fh = frame.height();

    let actual_h = band_h.clamp(1, fh.max(1));
    let top_y = band_top_y.clamp(0, fh - actual_h);

    for sy in (0..actual_h).step_by(TILE_SIZE as usize) {
        let src_y = top_y + sy;
        for sx in (0..fw).step_by(TILE_SIZE as usize) {
            spawn_tile_particle(state, frame, sx, src_y);
        }
    }

    if state.particles.len() > MAX_PARTICLES {
        let excess = state.particles.len() - MAX_PARTICLES;
        state.particles.drain(0..excess);
    }
}







struct App {
    capture:CaptureSession,
    state:State,
}
impl App {
    fn new() -> Self {
        let capture = CaptureSession::new().expect("failed to init capture");
        let state = State::new();
        App{capture, state }
    }
}



impl OverlayApp for App{

    fn render(&mut self, canvas: &mut Canvas) {

        canvas.clear();
        if let Some(frame) = self.capture.capture() {
            let fw = frame.width();
            let fh = frame.height();



            let dark_h = self.state.revealed_px;
            if dark_h > 0 {
                canvas.fill_rect(
                    0,
                    canvas.height - dark_h,
                    canvas.width,
                    dark_h,
                    (0, 0, 0, DARKEN_ALPHA),
                );
            }


            self.state.tick_accum += 1.0;


            while self.state.tick_accum >= FRAMES_PER_STEP && self.state.revealed_px < fh {
                let band_h = TILE_SIZE.min(fh - self.state.revealed_px);
                let band_top_y = fh - self.state.revealed_px - band_h;

                spawn_band_particles(&mut self.state, &frame, band_top_y, band_h);
                self.state.revealed_px += band_h;
                self.state.tick_accum -= FRAMES_PER_STEP;
            }


            // physics
            for p in &mut self.state.particles {
                if p.delay > 0 {
                    p.delay -= 1;
                } else {
                    p.vy += GRAVITY;
                    p.vx *= DRAG_X;
                    p.vy *= DRAG_Y;
                    p.x += p.vx;
                    p.y += p.vy;
                }


            }
            //deAlloc the particles which is out of the screen
            self.state.particles.retain(|p| {
                p.x > -200.0
                    && p.x < fw as f32 + 200.0
                    && p.y < fh as f32 + 300.0
            });


            for p in &self.state.particles {
                if p.delay <= 0 {
                    canvas.draw_image(&p.tile,p.x as i32,p.y as i32);
                }
                else {
                    canvas.clear_rect(p.x as i32,p.y as i32,p.tile.width,p.tile.height)
                }


            }
        }
    }
}


fn main() {
    overlay::run(App::new());
}