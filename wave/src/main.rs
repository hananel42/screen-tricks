#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn AttachConsole(dw_process_id: u32) -> i32;
}
const ATTACH_PARENT_PROCESS: u32 = 0xFFFF_FFFF;

use random::Random;

use lexopt::ValueExt;
use overlay::{Canvas, CaptureSession, EventResult, FrameImage, OverlayApp, OverlayContext, OverlayEvent, run, MouseButton, ImageSource};
use std::process;
// מבנה נתונים לייצוג גל מעגלי בודד
struct Ripple {
    center_x: f32,
    center_y: f32,
    radius: f32,
    amplitude: f32, // עוצמת העיוות הנוכחית (דועכת עם הזמן)
}

struct State {
    ripples: Vec<Ripple>,
    current_frame: Option<FrameImage>,
    freeze: bool,
}

impl State {
    fn new() -> Self {
        Self {
            ripples: Vec::with_capacity(10),
            current_frame: None,
            freeze: false,
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
    settings: Settings,
}

impl App {
    fn new(settings: Settings) -> Self {
        let capture = CaptureSession::new().expect("failed to init capture");
        let state = State::new();
        App {
            capture,
            state,
            settings,
        }
    }

    fn reset(&mut self) {
        self.state.ripples.clear();
        self.state.current_frame = None;
        self.state.freeze = false;
    }
}

impl OverlayApp for App {
    fn init(&mut self, overlay_context: &mut OverlayContext) {
        overlay_context.hide_from_capture(true);
    }

    fn handler(&mut self, event: OverlayEvent, c: &mut OverlayContext) -> EventResult {
        match event {
            OverlayEvent::KeyDown { vk } => match vk {
                0x1B => c.close(), // ESC
                0x20 => {
                    self.state.freeze = !self.state.freeze;
                    return EventResult::Consumed;
                } // SPACE
                0x52 => {
                    self.reset();
                    return EventResult::Consumed;
                } // R
                _ => {}
            },
            // הערה: בהנחה שרכיב ה-Backend שלכם מעביר את קואורדינטות העכבר בתוך ה-Enum של הלחצן
            // במידה והם מונגשים דרך פונקציה אחרת (למשל context.mouse_pos()), ניתן להתאים זאת בקלות.
            OverlayEvent::MouseDown { button: MouseButton::Left } => {
                if !self.state.freeze {
                    let (x,y) = c.mouse_position();
                    self.state.ripples.push(Ripple {
                        center_x: x as f32,
                        center_y: y as f32,
                        radius: 0.0,
                        amplitude: self.settings.max_amplitude,
                    });
                }
            }
            _ => {}
        };
        EventResult::Propagated
    }

    fn update(&mut self, _overlay_context: &mut OverlayContext, delta: f32) {
        if !self.state.freeze {
            // עדכון מצב הגלים הקיימים וסינון גלים שדעכו לחלוטין
            let speed = self.settings.wave_speed;
            let decay = self.settings.decay;

            self.state.ripples.retain_mut(|ripple| {
                ripple.radius += speed * delta;
                ripple.amplitude -= decay * delta;
                ripple.amplitude > 0.1 // נשמור על הגל רק אם הוא עדיין משפיע
            });

            // לכידת מסך רציפה
            if let Some(frame) = self.capture.capture() {
                self.state.current_frame = Some(frame.to_owned());
            }
        }
    }

    fn render(&mut self, canvas: &mut Canvas) {
        canvas.clear();

        if let Some(ref frame) = self.state.current_frame {
            let fw = canvas.width();
            let fh = canvas.height();

            // אופטימיזציה קריטית: אם אין גלים פעילים, נצייר את הפריים המלא ישירות ללא חישובים מיותרים
            if self.state.ripples.is_empty() {
                canvas.draw_image(frame, 0, 0);
                return;
            }

            let tile_size = self.settings.tile_size;
            let thickness = self.settings.wave_thickness;

            // מעבר על פני כל המסך בגריד של אריחים
            for ty in (0..fh).step_by(tile_size as usize) {
                for tx in (0..fw).step_by(tile_size as usize) {

                    let mut shift_x = 0.0;
                    let mut shift_y = 0.0;

                    // חישוב ההשפעה המצטברת של כל הגלים הפעילים על האריח הנוכחי
                    for ripple in &self.state.ripples {
                        let dx = tx as f32 - ripple.center_x;
                        let dy = ty as f32 - ripple.center_y;
                        let distance = (dx * dx + dy * dy).sqrt();

                        if distance > 0.0 {
                            // בדיקה האם האריח נמצא בטווח חזית הגל
                            let dist_from_wave = (distance - ripple.radius).abs();
                            if dist_from_wave < thickness {
                                // יצירת עיוות סינוס חלק שחולף בתוך עובי הגל
                                let normalized_dist = dist_from_wave / thickness;
                                let wave_factor = (normalized_dist * std::f32::consts::PI).cos();

                                // כיוון ההיסט הוא כלפי חוץ ממרכז הגל
                                let force = wave_factor * ripple.amplitude;
                                shift_x += (dx / distance) * force;
                                shift_y += (dy / distance) * force;
                            }
                        }
                    }

                    // חישוב מיקום המקור ממנו נחתוך את התמונה (מיושר לפי ההיסט שחושב)
                    let src_x = tx + shift_x as i32;
                    let src_y = ty + shift_y as i32;

                    // הגנה מפני יציאה מגבולות הפריים שנלכד
                    let (cx, cy, cw, ch) = clamp_rect(src_x, src_y, tile_size, tile_size, fw, fh);

                    // חיתוך האריח המעוות וציורו במיקום המקורי שלו על המסך
                    if let Some(strip) = frame.crop(cx, cy, cw, ch) {
                        canvas.draw_image(&strip, tx, ty);
                    }
                }
            }
        }
    }
}

struct Settings {
    tile_size: i32,       // גודל האריח (ככל שקטן יותר - האיכות גבוהה יותר, אך דורש יותר כוח עיבוד)
    wave_speed: f32,      // מהירות התפשטות הגל בפיקסלים לשנייה
    wave_thickness: f32,  // עובי רצועת הגל
    max_amplitude: f32,   // עוצמת העיוות ההתחלתית
    decay: f32,           // קצב דעיכת הגל
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            tile_size: 8,         // איזון מעולה בין רזולוציית גל לביצועים
            wave_speed: 600.0,    // מהירות התפשטות מהירה וטבעית
            wave_thickness: 60.0, // רוחב הגל עצמו
            max_amplitude: 30.0,  // הסטה מקסימלית של עד 30 פיקסלים
            decay: 40.0,          // הגל ייעלם תוך פחות משנייה
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
            lexopt::Arg::Long("wave-speed") => {
                settings.wave_speed = parser.value()?.parse()?;
            }
            lexopt::Arg::Long("wave-thickness") => {
                settings.wave_thickness = parser.value()?.parse()?;
            }
            lexopt::Arg::Long("amplitude") => {
                settings.max_amplitude = parser.value()?.parse()?;
            }
            lexopt::Arg::Long("decay") => {
                settings.decay = parser.value()?.parse()?;
            }
            lexopt::Arg::Short('h') | lexopt::Arg::Long("help") => {
                print_help();
                process::exit(0);
            }
            lexopt::Arg::Short('r') | lexopt::Arg::Long("random") => {
                let mut random = Random::new();
                settings = Settings {
                    tile_size: *random.choose(&[4, 8, 16]),
                    wave_speed: random.range(300.0, 1000.0),
                    wave_thickness: random.range(30.0, 120.0),
                    max_amplitude: random.range(10.0, 60.0),
                    decay: random.range(20.0, 80.0),
                }
            }
            _ => return Err(arg.unexpected()),
        }
    }

    Ok(settings)
}

fn print_help() {
    println!("Screen Ripple Disintegration Options:");
    println!("  --tile-size <int>       Size of distortion tiles (default: 8)");
    println!("  --wave-speed <float>    Speed of the wave in px/s (default: 600.0)");
    println!("  --wave-thickness <float>Thickness of the wave ripple (default: 60.0)");
    println!("  --amplitude <float>     Max distortion amplitude (default: 30.0)");
    println!("  --decay <float>         How fast the wave fades out (default: 40.0)");
    println!("  -r, --random            Generate random ripple characteristics");
    println!("  -h, --help              Print this help message");
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
