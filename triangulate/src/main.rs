#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn AttachConsole(dw_process_id: u32) -> i32;
}
const ATTACH_PARENT_PROCESS: u32 = 0xFFFF_FFFF;
mod delaunay;
use clinc::{ParseError, Parser, Token};
use delaunay::*;
use overlay::image::{FrameImage, ImageSource};
use overlay::{
    Canvas, EventResult, MouseButton, OverlayApp, OverlayContext, OverlayEvent,
    image::capture::CaptureSession, run,
};
use random::*;
pub const fn max_triangles(num_vertices: usize) -> usize {
    if num_vertices < 3 {
        0
    } else {
        2 * num_vertices - 5
    }
}

/// פונקציית עזר למיון קודקודים לפי ציר ה-Y עבור אלגוריתם ה-Scanline
#[inline(always)]
fn sort_vertices(
    p1: Point,
    p2: Point,
    p3: Point,
    t1: Point,
    t2: Point,
    t3: Point,
) -> (Point, Point, Point, Point, Point, Point) {
    let (mut a, mut b, mut c) = (p1, p2, p3);
    let (mut ta, mut tb, mut tc) = (t1, t2, t3);

    if b.y < a.y {
        std::mem::swap(&mut a, &mut b);
        std::mem::swap(&mut ta, &mut tb);
    }
    if c.y < a.y {
        std::mem::swap(&mut a, &mut c);
        std::mem::swap(&mut ta, &mut tc);
    }
    if c.y < b.y {
        std::mem::swap(&mut b, &mut c);
        std::mem::swap(&mut tb, &mut tc);
    }

    (a, b, c, ta, tb, tc)
}

pub fn render_textured_triangle(
    src_image: &impl ImageSource,
    src_tri: Triangle,
    dest_tri: Triangle,
    canvas: &mut Canvas,
) {
    let width = src_image.width();
    let height = src_image.height();
    let src_pixels = src_image.pixels();

    let (a, b, c, ta, tb, tc) = sort_vertices(
        dest_tri.p1,
        dest_tri.p2,
        dest_tri.p3,
        src_tri.p1,
        src_tri.p2,
        src_tri.p3,
    );

    let y_a = a.y.round() as i32;
    let y_b = b.y.round() as i32;
    let y_c = c.y.round() as i32;

    if y_a == y_c || y_c < 0 || y_a >= height {
        return;
    }

    let den = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
    if den.abs() < 0.00001 {
        return;
    }
    let inv_den = 1.0 / den;

    let mut draw_scanline = |y: i32, x1: f32, x2: f32| {
        if y < 0 || y >= height {
            return;
        }

        let (mut start_x, mut end_x) = (x1.round() as i32, x2.round() as i32);
        if start_x > end_x {
            std::mem::swap(&mut start_x, &mut end_x);
        }

        start_x = start_x.max(0);
        end_x = end_x.min(width - 1);

        if start_x > end_x {
            return;
        }

        let y_f = y as f32 + 0.5;
        let dst_row_offset = (y * width) as usize;

        let x_f = start_x as f32 + 0.5;
        let w0 = ((b.y - c.y) * (x_f - c.x) + (c.x - b.x) * (y_f - c.y)) * inv_den;
        let w1 = ((c.y - a.y) * (x_f - c.x) + (a.x - c.x) * (y_f - c.y)) * inv_den;
        let w2 = 1.0 - w0 - w1;

        let mut tex_x = w0 * ta.x + w1 * tb.x + w2 * tc.x;
        let mut tex_y = w0 * ta.y + w1 * tb.y + w2 * tc.y;

        let dw0_dx = (b.y - c.y) * inv_den;
        let dw1_dx = (c.y - a.y) * inv_den;
        let dw2_dx = -dw0_dx - dw1_dx;

        let dtex_x = dw0_dx * ta.x + dw1_dx * tb.x + dw2_dx * tc.x;
        let dtex_y = dw0_dx * ta.y + dw1_dx * tb.y + dw2_dx * tc.y;

        for x in start_x..=end_x {
            let src_x = (tex_x as i32).clamp(0, width - 1);
            let src_y = (tex_y as i32).clamp(0, height - 1);

            let src_idx = (src_y * width + src_x) as usize;
            let dst_idx = dst_row_offset + x as usize;

            unsafe {
                let pixel = *src_pixels.get_unchecked(src_idx);
                canvas.put_raw_pixel(dst_idx, pixel);
            }

            tex_x += dtex_x;
            tex_y += dtex_y;
        }
    };

    if y_b > y_a {
        let slope_ac = (c.x - a.x) / (c.y - a.y);
        let slope_ab = (b.x - a.x) / (b.y - a.y);

        let start_y = y_a.max(0);
        let end_y = y_b.min(height - 1);

        for y in start_y..=end_y {
            let dy = y as f32 - a.y;
            let x1 = a.x + dy * slope_ac;
            let x2 = a.x + dy * slope_ab;
            draw_scanline(y, x1, x2);
        }
    }

    if y_c > y_b {
        let slope_ac = (c.x - a.x) / (c.y - a.y);
        let slope_bc = (c.x - b.x) / (c.y - b.y);

        let start_y = y_b.max(0);
        let end_y = y_c.min(height - 1);

        for y in start_y..=end_y {
            let dy_ac = y as f32 - a.y;
            let dy_bc = y as f32 - b.y;
            let x1 = a.x + dy_ac * slope_ac;
            let x2 = b.x + dy_bc * slope_bc;
            draw_scanline(y, x1, x2);
        }
    }
}

struct TriangleState {
    pub src_tri: Triangle,
    pub pos: Triangle,
    pub center_x: f32,
    pub center_y: f32,
    pub vel_x: f32,
    pub vel_y: f32,
    pub rot_speed: f32,
    pub current_rot: f32,
    pub trans_x: f32,
    pub trans_y: f32,
}

impl TriangleState {
    pub fn new(
        src_tri: &Triangle,
        x: i32,
        y: i32,
        r: &mut Random,
        settings: &Settings,
    ) -> TriangleState {
        let cx = (src_tri.p1.x + src_tri.p2.x + src_tri.p3.x) / 3.0;
        let cy = (src_tri.p1.y + src_tri.p2.y + src_tri.p3.y) / 3.0;

        let mut dir_x = cx - x as f32;
        let mut dir_y = cy - y as f32;

        let dist = (dir_x * dir_x + dir_y * dir_y).sqrt().max(1.0);
        dir_x /= dist;
        dir_y /= dist;

        let speed = r.range(settings.min_speed, settings.max_speed);

        TriangleState {
            src_tri: src_tri.clone(),
            pos: src_tri.clone(),
            center_x: cx,
            center_y: cy,
            vel_x: dir_x * speed + r.jitter(settings.speed_jitter),
            vel_y: dir_y * speed + r.jitter(settings.speed_jitter),
            rot_speed: r.jitter(settings.rotation_speed_jitter),
            current_rot: 0.0,
            trans_x: 0.0,
            trans_y: 0.0,
        }
    }

    #[inline(always)]
    pub fn rotate_and_translate(&mut self) {
        let cos_r = self.current_rot.cos();
        let sin_r = self.current_rot.sin();
        let cx = self.center_x;
        let cy = self.center_y;
        let tx = self.trans_x;
        let ty = self.trans_y;

        let transform = |p: Point| -> Point {
            let dx = p.x - cx;
            let dy = p.y - cy;
            Point {
                x: (dx * cos_r - dy * sin_r) + cx + tx,
                y: (dx * sin_r + dy * cos_r) + cy + ty,
            }
        };

        self.pos.p1 = transform(self.src_tri.p1);
        self.pos.p2 = transform(self.src_tri.p2);
        self.pos.p3 = transform(self.src_tri.p3);
    }

    #[inline(always)]
    pub fn render(&self, canvas: &mut Canvas, frame: &impl ImageSource) {
        render_textured_triangle(frame, self.src_tri, self.pos, canvas);
    }
}

struct MyOverlayApp {
    capture_session: CaptureSession,
    captured_image: Option<FrameImage>,
    triangles: Vec<TriangleState>,
    is_shattered: bool,
    settings: Settings,
}

impl OverlayApp for MyOverlayApp {
    fn handler(
        &mut self,
        event: OverlayEvent,
        overlay_context: &mut OverlayContext,
    ) -> EventResult {
        match event {
            OverlayEvent::KeyDown { vk } => {
                match vk {
                    0x1B => {
                        overlay_context.close();
                    } //ESC
                    0x52 => {
                        self.is_shattered = false;
                        self.captured_image = None;
                        self.triangles.clear();
                    } //R
                    _ => {}
                }
            }
            OverlayEvent::MouseDown {
                button: MouseButton::Left,
            } => {
                if !self.is_shattered {
                    self.captured_image = self.capture_session.capture().map(|t| t.to_owned());
                    self.is_shattered = self.captured_image.is_some();
                    if self.is_shattered {
                        let width = overlay_context.width() as f32;
                        let height = overlay_context.height() as f32;

                        let mut points = vec![Point { x: 0.0, y: 0.0 }; self.settings.points + 5];
                        points[0] = Point { x: 0.0, y: 0.0 };
                        points[1] = Point { x: width, y: 0.0 };
                        points[2] = Point {
                            x: width,
                            y: height,
                        };
                        points[3] = Point { x: 0.0, y: height };
                        let (x, y) = overlay_context.mouse_position();
                        points[4] = Point {
                            x: x as f32,
                            y: y as f32,
                        };

                        let mut r = Random::new();
                        for i in 5..self.settings.points + 5 {
                            points[i] = Point {
                                x: r.range(0.0, width),
                                y: r.range(0.0, height),
                            };
                        }

                        self.triangles = triangulate(&points, width, height)
                            .iter()
                            .map(|triangle| {
                                TriangleState::new(triangle, x, y, &mut r, &self.settings)
                            })
                            .collect();
                        return EventResult::Consumed;
                    }
                }
            }
            _ => {}
        }

        EventResult::Propagated
    }

    fn update(&mut self, _overlay_context: &mut OverlayContext, delta: f32) {
        if self.is_shattered {
            let gravity = self.settings.gravity;

            let num_threads = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4);

            let chunk_size = (self.triangles.len() + num_threads - 1) / num_threads;

            if chunk_size > 0 {
                std::thread::scope(|s| {
                    for chunk in self.triangles.chunks_mut(chunk_size) {
                        s.spawn(move || {
                            for triangle in chunk {
                                triangle.vel_y += gravity * delta;
                                triangle.trans_x += triangle.vel_x * delta;
                                triangle.trans_y += triangle.vel_y * delta;
                                triangle.current_rot += triangle.rot_speed * delta;

                                triangle.rotate_and_translate();
                            }
                        });
                    }
                });
            }
        }
    }

    fn render(&mut self, canvas: &mut Canvas) {
        if !self.is_shattered {
            canvas.clear();
            return;
        }

        if let Some(ref frame) = self.captured_image {
            canvas.fill((0, 0, 0, 255));
            for triangle in &self.triangles {
                triangle.render(canvas, frame);
            }
        }
    }
}

struct Settings {
    gravity: f32,
    rotation_speed_jitter: f32,
    max_speed: f32,
    min_speed: f32,
    speed_jitter: f32,
    points: usize,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            gravity: 60.0,
            rotation_speed_jitter: 3.0,
            max_speed: 500.0,
            min_speed: 120.0,
            speed_jitter: 50.0,
            points: 100,
        }
    }
}

/// Prints a formatted application help manual directly to the standard output stream.
fn print_help() {
    let default_settings = Settings::default();
    println!("Shatter Overlay Application - Configuration Manual\n");
    println!("USAGE:");
    println!("  overlay_app [OPTIONS]\n");
    println!("OPTIONS:");
    println!(
        "  -g, --gravity <f32>                 Set environmental gravity force [default: {}]",
        default_settings.gravity
    );
    println!(
        "  -r, --rotation-speed-jitter <f32>   Set multiplier value for spin jitters [default: {}]",
        default_settings.rotation_speed_jitter
    );
    println!(
        "      --max-speed <f32>               Set absolute maximum explosion limits [default: {}]",
        default_settings.max_speed
    );
    println!(
        "      --min-speed <f32>               Set base minimum structural velocity [default: {}]",
        default_settings.min_speed
    );
    println!(
        "  -j, --speed-jitter <f32>            Randomized speed offset variation [default: {}]",
        default_settings.speed_jitter
    );
    println!(
        "  -p, --points <usize>                Quantity of triangulation vertices generated [default: {}]",
        default_settings.points
    );
    println!("  -h, --help                          Display this help information manual and exit");
}

/// Parses command line arguments, handles automated help prints, or returns custom setup configuration errors.
fn parse() -> Result<Option<Settings>, ParseError> {
    let mut settings = Settings::default();
    let mut parser = Parser::from_env();

    while let Some(token) = parser.next() {
        match token {
            Token::Long("help") | Token::Short("h") => {
                print_help();
                return Ok(None); // Signifies that execution handled a command option successfully but should exit
            }
            Token::Long("gravity") | Token::Short("g") => {
                settings.gravity = parser.parse()?;
            }
            Token::Long("rotation-speed-jitter") | Token::Short("r") => {
                settings.rotation_speed_jitter = parser.parse()?;
            }
            Token::Long("max-speed") => {
                settings.max_speed = parser.parse()?;
            }
            Token::Long("min-speed") => {
                settings.min_speed = parser.parse()?;
            }
            Token::Long("speed-jitter") | Token::Short("j") => {
                settings.speed_jitter = parser.parse()?;
            }
            Token::Long("points") | Token::Short("p") => {
                settings.points = parser.parse()?;
            }
            _ => {}
        }
    }

    Ok(Some(settings))
}

fn main() {
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }

    let capture_session = CaptureSession::new().expect("Failed to initialize capture session");

    match parse() {
        Ok(Some(settings)) => {
            let app = MyOverlayApp {
                capture_session,
                captured_image: None,
                triangles: Vec::new(),
                is_shattered: false,
                settings,
            };
            run(app);
        }
        Ok(None) => {
            // Help was successfully displayed; exit gracefully
            std::process::exit(0);
        }
        Err(err) => {
            eprintln!(
                "Initialization Error: {}\nRun with --help to view valid option specifications.",
                err
            );
            std::process::exit(1);
        }
    }
}
