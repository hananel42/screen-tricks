use overlay::image::ImageSource;
use overlay::{
    Canvas, EventResult, MouseButton, OverlayApp, OverlayContext, OverlayEvent,
    image::capture::CaptureSession, run,
};
use std::sync::{Arc, Barrier};
use std::thread;

const PIXEL_SIZE: usize = 4;
const ITERATIONS: usize = 12;
const GRAVITY_Y: f32 = 0.0;
const DAMPING: f32 = 0.98;
const STIFFNESS: f32 = 0.95;
const MAX_DELTA: f32 = 0.02;
const NUM_THREADS: usize = 4;

const FAST_MATH_C: f32 = 0.25 * STIFFNESS;

struct ThreadSafeGrid {
    pos_ptr: usize,
    can_ptr: usize,
    width: usize,
    height: usize,
}
unsafe impl Send for ThreadSafeGrid {}
unsafe impl Sync for ThreadSafeGrid {}

struct MyOverlayApp {
    capture_session: CaptureSession,
    positions: Box<[(f32, f32)]>,
    old_positions: Box<[(f32, f32)]>,
    can_move: Box<[bool]>,

    mouse_pos: (f32, f32),
    dragged_node: Option<usize>,
    is_right_click: bool,

    barrier: Arc<Barrier>,
}

impl MyOverlayApp {
    fn find_nearest_node(&self, mx: f32, my: f32) -> Option<usize> {
        let mut min_dist = 400.0;
        let mut nearest = None;

        for (i, p) in self.positions.iter().enumerate() {
            let dx = p.0 - mx;
            let dy = p.1 - my;
            let dist_sq = dx * dx + dy * dy;

            if dist_sq < min_dist {
                min_dist = dist_sq;
                nearest = Some(i);
            }
        }
        nearest
    }
}

impl OverlayApp for MyOverlayApp {
    fn init(&mut self, overlay_context: &mut OverlayContext) {
        overlay_context.hide_from_capture(true);

        let width = (overlay_context.width() as usize) / PIXEL_SIZE;
        let height = (overlay_context.height() as usize) / PIXEL_SIZE;
        let size = width * height;

        let mut vec = Vec::with_capacity(size);
        let mut can_move_vec = Vec::with_capacity(size);

        for y in 0..height {
            for x in 0..width {
                vec.push(((x * PIXEL_SIZE) as f32, (y * PIXEL_SIZE) as f32));
                can_move_vec.push(!(y == 0 || y == height - 1 || x == 0 || x == width - 1));
            }
        }

        self.positions = vec.clone().into_boxed_slice();
        self.old_positions = vec.into_boxed_slice();
        self.can_move = can_move_vec.into_boxed_slice();
    }

    fn handler(
        &mut self,
        event: OverlayEvent,
        overlay_context: &mut OverlayContext,
    ) -> EventResult {
        match event {
            OverlayEvent::KeyDown { vk: 0x1B } => {
                overlay_context.close();
                EventResult::Consumed
            }
            OverlayEvent::MouseMove { x, y } => {
                self.mouse_pos = (x as f32, y as f32);
                EventResult::Propagated
            }
            OverlayEvent::MouseDown { button } => {
                let (x_, y_) = overlay_context.mouse_position();
                self.mouse_pos = (x_ as f32, y_ as f32);

                if let Some(idx) = self.find_nearest_node(self.mouse_pos.0, self.mouse_pos.1) {
                    self.dragged_node = Some(idx);
                    match button {
                        MouseButton::Left => self.is_right_click = false,
                        MouseButton::Right => {
                            self.is_right_click = true;
                            self.can_move[idx] = false;
                        }
                        MouseButton::Middle => {
                            self.can_move[idx] = true;
                            self.dragged_node = None;
                        }
                        _ => {}
                    }
                }
                EventResult::Propagated
            }
            OverlayEvent::MouseUp { .. } => {
                self.dragged_node = None;
                EventResult::Propagated
            }
            _ => EventResult::Propagated,
        }
    }

    fn update(&mut self, overlay_context: &mut OverlayContext, delta: f32) {
        let width = (overlay_context.width() as usize) / PIXEL_SIZE;
        let height = (overlay_context.height() as usize) / PIXEL_SIZE;
        let dt = delta.min(MAX_DELTA);
        let dt_sq = GRAVITY_Y * dt * dt;

        let rest_len = PIXEL_SIZE as f32;
        let rest_len_sq = rest_len * rest_len;

        if let Some(idx) = self.dragged_node {
            if !self.is_right_click {
                self.can_move[idx] = false;
            }
            self.positions[idx] = self.mouse_pos;
            self.old_positions[idx] = self.mouse_pos;
        }

        for ((p, old_p), &can_move) in self
            .positions
            .iter_mut()
            .zip(self.old_positions.iter_mut())
            .zip(self.can_move.iter())
        {
            if can_move {
                let vel_x = (p.0 - old_p.0) * DAMPING;
                let vel_y = (p.1 - old_p.1) * DAMPING;
                *old_p = *p;
                p.0 += vel_x;
                p.1 += vel_y + dt_sq;
            }
        }

        let grid_share = Arc::new(ThreadSafeGrid {
            pos_ptr: self.positions.as_mut_ptr() as usize,
            can_ptr: self.can_move.as_ptr() as usize,
            width,
            height,
        });

        let mut workers = Vec::with_capacity(NUM_THREADS);
        let rows_per_thread = height / NUM_THREADS;

        for t in 0..NUM_THREADS {
            let barrier = Arc::clone(&self.barrier);
            let grid = Arc::clone(&grid_share);

            let start_y = t * rows_per_thread;
            let end_y = if t == NUM_THREADS - 1 {
                height
            } else {
                (t + 1) * rows_per_thread
            };

            workers.push(thread::spawn(move || unsafe {
                let pos_ptr = grid.pos_ptr as *mut (f32, f32);
                let can_ptr = grid.can_ptr as *const bool;
                let w = grid.width;

                let solve_nodes = |i_b: usize, p_a: &mut (f32, f32), can_a: bool| {
                    let mut p_b = *pos_ptr.add(i_b);
                    let dx = p_b.0 - p_a.0;
                    let dy = p_b.1 - p_a.1;
                    let dist_sq = dx * dx + dy * dy;

                    if dist_sq > 0.0001 {
                        let pct = if dist_sq > rest_len_sq * 0.5 && dist_sq < rest_len_sq * 2.0 {
                            (rest_len_sq - dist_sq) * (FAST_MATH_C / rest_len_sq)
                        } else {
                            let dist = dist_sq.sqrt();
                            ((rest_len - dist) / dist) * 0.5 * STIFFNESS
                        };

                        let ox = dx * pct;
                        let oy = dy * pct;

                        if can_a {
                            p_a.0 -= ox;
                            p_a.1 -= oy;
                        }
                        if *can_ptr.add(i_b) {
                            p_b.0 += ox;
                            p_b.1 += oy;
                            *pos_ptr.add(i_b) = p_b;
                        }
                    }
                };

                for _ in 0..ITERATIONS {
                    for y in start_y..end_y {
                        let row_start = y * w;
                        for x in 0..w {
                            if (x + y) % 2 == 0 {
                                let i = row_start + x;
                                let mut p_a = *pos_ptr.add(i);
                                let can_a = *can_ptr.add(i);

                                if x < w - 1 {
                                    solve_nodes(i + 1, &mut p_a, can_a);
                                }
                                if y < grid.height - 1 {
                                    solve_nodes(i + w, &mut p_a, can_a);
                                }
                                *pos_ptr.add(i) = p_a;
                            }
                        }
                    }
                    barrier.wait();

                    for y in start_y..end_y {
                        let row_start = y * w;
                        for x in 0..w {
                            if (x + y) % 2 != 0 {
                                let i = row_start + x;
                                let mut p_a = *pos_ptr.add(i);
                                let can_a = *can_ptr.add(i);

                                if x < w - 1 {
                                    solve_nodes(i + 1, &mut p_a, can_a);
                                }
                                if y < grid.height - 1 {
                                    solve_nodes(i + w, &mut p_a, can_a);
                                }
                                *pos_ptr.add(i) = p_a;
                            }
                        }
                    }
                    barrier.wait();
                }
            }));
        }

        for worker in workers {
            let _ = worker.join();
        }

        if let Some(idx) = self.dragged_node {
            if !self.is_right_click {
                self.can_move[idx] = true;
            }
        }
    }

    fn render(&mut self, canvas: &mut Canvas) {
        canvas.fill((0, 0, 0, 255));
        if let Some(frame) = self.capture_session.capture() {
            let f_width = frame.width() as usize;
            let f_height = frame.height() as usize;
            let canvas_width = canvas.width();
            let canvas_height = canvas.height();

            let grid_width = f_width / PIXEL_SIZE;
            let grid_height = f_height / PIXEL_SIZE;

            let mut src_index = 0;

            for y in 0..grid_height {
                for x in 0..grid_width {
                    let (dst_x, dst_y) = self.positions[src_index];
                    src_index += 1;

                    let cx = dst_x as i32;
                    let cy = dst_y as i32;

                    if cx >= 0
                        && cx < canvas_width - (PIXEL_SIZE as i32)
                        && cy >= 0
                        && cy < canvas_height - (PIXEL_SIZE as i32)
                    {
                        for block_y in 0..PIXEL_SIZE {
                            for block_x in 0..PIXEL_SIZE {
                                let src_pixel_x = x * PIXEL_SIZE + block_x;
                                let src_pixel_y = y * PIXEL_SIZE + block_y;

                                if src_pixel_x < f_width && src_pixel_y < f_height {
                                    let dst_pixel_idx = ((cy + block_y as i32) as usize)
                                        * (canvas.width() as usize)
                                        + ((cx + block_x as i32) as usize);
                                    unsafe {
                                        canvas.put_raw_pixel(
                                            dst_pixel_idx,
                                            frame.get_pixel_unchecked(
                                                src_pixel_x as i32,
                                                src_pixel_y as i32,
                                            ),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn main() {
    let app = MyOverlayApp {
        capture_session: CaptureSession::new().unwrap(),
        positions: Box::new([]),
        old_positions: Box::new([]),
        can_move: Box::new([]),
        mouse_pos: (0.0, 0.0),
        dragged_node: None,
        is_right_click: false,
        barrier: Arc::new(Barrier::new(NUM_THREADS)),
    };
    run(app);
}
