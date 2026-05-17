#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod overlay;

use overlay::{CaptureSession, ImageSource};

const TILE_SIZE: i32 = 5;
const HOLD_FRAMES: i32 = 0;
const HOLD_JITTER: i32 = 10;
const GRAVITY: f32 = 1.0;
const DRAG_X: f32 = 0.995;
const DRAG_Y: f32 = 0.998;
const DARKEN_ALPHA: u8 = 255;
const MAX_PARTICLES: usize = 25_000;
const FRAMES_PER_STEP: f32 = 5.0;

#[derive(Clone)]
struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    delay: i32,
    tile_w: i32,
    tile_h: i32,
    tile: Vec<u32>,
}

struct State {
    particles: Vec<Particle>,
    revealed_px: i32,   // כמה פיקסלים מלמטה כבר "נבלעו"
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

fn capture_tile(img: &impl ImageSource, x: i32, y: i32, w: i32, h: i32) -> Vec<u32> {
    let mut out = Vec::with_capacity((w * h) as usize);

    let pixels = img.pixels();
    let stride = img.stride();
    let origin = img.origin();

    for yy in 0..h {
        let row = origin + (y + yy) as usize * stride;
        for xx in 0..w {
            out.push(pixels[row + (x + xx) as usize]);
        }
    }

    out
}

fn spawn_tile_particle(state: &mut State, frame: &impl ImageSource, src_x: i32, src_y: i32) {
    let fw = frame.width();
    let fh = frame.height();

    let (x, y, w, h) = clamp_rect(src_x, src_y, TILE_SIZE, TILE_SIZE, fw, fh);
    let tile = capture_tile(frame, x, y, w, h);

    let seed = (x as u32)
        ^ ((y as u32).wrapping_mul(0x9E37_79B9))
        ^ ((state.revealed_px as u32).wrapping_mul(0x85EB_CA6B));

    let delay = HOLD_FRAMES + rand_range(seed ^ 0xA53A_9F1C, HOLD_JITTER);
    let vx = jitter(seed.rotate_left(5), 2.0);
    let vy = jitter(seed.rotate_left(11), 0.18);

    state.particles.push(Particle {
        x: x as f32,
        y: y as f32,
        vx,
        vy,
        delay,
        tile_w: w,
        tile_h: h,
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

struct TileImage<'a> {
    w: i32,
    h: i32,
    data: &'a [u32],
}

impl<'a> ImageSource for TileImage<'a> {
    fn width(&self) -> i32 {
        self.w
    }

    fn height(&self) -> i32 {
        self.h
    }

    fn stride(&self) -> usize {
        self.w as usize
    }

    fn pixels(&self) -> &[u32] {
        self.data
    }

    fn origin(&self) -> usize {
        0
    }
}

fn main() {
    let mut capture = CaptureSession::new().expect("failed to init capture");
    let mut state = State::new();

    overlay::run(move |x| {
        x.clear();

        if let Some(frame) = capture.capture() {
            let fw = frame.width().max(1);
            let fh = frame.height().max(1);

            let scale_x = x.width as f32 / fw as f32;
            let scale_y = x.height as f32 / fh as f32;




            let dark_h = ((state.revealed_px as f32) * scale_y) as i32;
            if dark_h > 0 {
                x.fill_rect(
                    0,
                    x.height - dark_h,
                    x.width,
                    dark_h,
                    (0, 0, 0, DARKEN_ALPHA),
                );
            }


            state.tick_accum += 1.0;
            while state.tick_accum >= FRAMES_PER_STEP && state.revealed_px < fh {
                let band_h = TILE_SIZE.min(fh - state.revealed_px);
                let band_top_y = fh - state.revealed_px - band_h;

                spawn_band_particles(&mut state, &frame, band_top_y, band_h);
                state.revealed_px += band_h;
                state.tick_accum -= FRAMES_PER_STEP;
            }

            for p in &mut state.particles {
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

            state.particles.retain(|p| {
                     p.x > -200.0
                    && p.x < fw as f32 + 200.0
                    && p.y < fh as f32 + 300.0
            });


            for p in &state.particles {
                let tile_img = TileImage {
                    w: p.tile_w,
                    h: p.tile_h,
                    data: &p.tile,
                };

                let dx = (p.x * scale_x) as i32;
                let dy = (p.y * scale_y) as i32;
                let dw = ((p.tile_w as f32) * scale_x).max(2.0) as i32;
                let dh = ((p.tile_h as f32) * scale_y).max(2.0) as i32;

                x.draw_image_scaled(&tile_img, dx, dy, dw, dh);
            }
        }
    });
}