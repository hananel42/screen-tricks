// use crate::overlay::{Canvas, EventResult, MouseButton, OverlayEvent};
//
// pub type Color = (u8, u8, u8, u8);
//
// pub mod colors {
//     use super::Color;
//
//     pub const TRANSPARENT: Color = (0, 0, 0, 0);
//     pub const WHITE: Color = (255, 255, 255, 255);
//     pub const BLACK: Color = (0, 0, 0, 255);
//
//     pub const BG: Color = (24, 24, 28, 255);
//     pub const BG_2: Color = (34, 34, 40, 255);
//     pub const BORDER: Color = (70, 70, 82, 255);
//
//     pub const TEXT: Color = (240, 240, 245, 255);
//     pub const MUTED: Color = (170, 170, 180, 255);
//
//     pub const BLUE: Color = (75, 140, 255, 255);
//     pub const BLUE_HOVER: Color = (95, 160, 255, 255);
//     pub const BLUE_PRESSED: Color = (55, 115, 235, 255);
//
//     pub const GREEN: Color = (70, 190, 120, 255);
//     pub const RED: Color = (235, 90, 90, 255);
//     pub const YELLOW: Color = (240, 200, 70, 255);
// }
//
// #[derive(Clone, Copy, Debug, PartialEq, Eq)]
// pub enum AlignH {
//     Left,
//     Center,
//     Right,
// }
//
// #[derive(Clone, Copy, Debug, PartialEq, Eq)]
// pub enum AlignV {
//     Top,
//     Middle,
//     Bottom,
// }
//
// #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
// pub struct Rect {
//     pub x: i32,
//     pub y: i32,
//     pub w: i32,
//     pub h: i32,
// }
//
// impl Rect {
//     #[inline]
//     pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
//         Self { x, y, w, h }
//     }
//
//     #[inline]
//     pub fn contains(&self, x: i32, y: i32) -> bool {
//         x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
//     }
// }
//
// #[inline]
// fn clamp_f32(v: f32, lo: f32, hi: f32) -> f32 {
//     v.max(lo).min(hi)
// }
//
// #[inline]
// fn clamp_i32(v: i32, lo: i32, hi: i32) -> i32 {
//     v.max(lo).min(hi)
// }
//
// #[inline]
// fn line_width(line: &str, scale: i32) -> i32 {
//     let scale = scale.max(1);
//     let count = line.chars().count() as i32;
//     if count <= 0 {
//         0
//     } else {
//         count * (8 * scale + scale) - scale
//     }
// }
//
// #[inline]
// fn measure_text(text: &str, scale: i32, line_gap: i32) -> (i32, i32) {
//     let scale = scale.max(1);
//     let lines: Vec<&str> = text.split('\n').collect();
//
//     let mut max_w = 0;
//     for line in &lines {
//         max_w = max_w.max(line_width(line, scale));
//     }
//
//     let line_h = 8 * scale;
//     let total_h = if lines.is_empty() {
//         0
//     } else {
//         (lines.len() as i32) * line_h + ((lines.len() as i32 - 1).max(0)) * line_gap.max(0)
//     };
//
//     (max_w, total_h)
// }
//
// #[inline]
// fn draw_text_box(
//     canvas: &mut Canvas,
//     x: i32,
//     y: i32,
//     w: i32,
//     h: i32,
//     text: &str,
//     scale: i32,
//     color: Color,
//     align_h: AlignH,
//     align_v: AlignV,
//     line_gap: i32,
// ) {
//     let scale = scale.max(1);
//     let lines: Vec<&str> = text.split('\n').collect();
//     let (_, text_h) = measure_text(text, scale, line_gap);
//
//     let start_y = match align_v {
//         AlignV::Top => y,
//         AlignV::Middle => y + (h - text_h) / 2,
//         AlignV::Bottom => y + h - text_h,
//     };
//
//     let mut cy = start_y;
//     for line in lines {
//         let lw = line_width(line, scale);
//         let cx = match align_h {
//             AlignH::Left => x,
//             AlignH::Center => x + (w - lw) / 2,
//             AlignH::Right => x + w - lw,
//         };
//
//         canvas.draw_text(cx, cy, line, scale, color);
//         cy += 8 * scale + line_gap.max(0);
//     }
// }
//
// fn char_to_byte_idx(s: &str, char_idx: usize) -> usize {
//     if char_idx == 0 {
//         return 0;
//     }
//
//     let total = s.chars().count();
//     if char_idx >= total {
//         return s.len();
//     }
//
//     s.char_indices()
//         .nth(char_idx)
//         .map(|(i, _)| i)
//         .unwrap_or(s.len())
// }
//
// fn vk_to_char(vk: u32) -> Option<char> {
//     match vk {
//         0x20 => Some(' '),
//         0x30..=0x39 => char::from_u32(vk), // '0'..'9'
//         0x41..=0x5A => char::from_u32(vk + 32), // 'A'..'Z' -> lowercase
//         0xBD => Some('-'),
//         0xBB => Some('='),
//         0xBC => Some(','),
//         0xBE => Some('.'),
//         0xBF => Some('/'),
//         0xBA => Some(';'),
//         0xDE => Some('\''),
//         0xDB => Some('['),
//         0xDD => Some(']'),
//         0xDC => Some('\\'),
//         0xC0 => Some('`'),
//         _ => None,
//     }
// }
//
// pub trait UIObject<Ctx> {
//     fn bounds(&self) -> Rect;
//
//     fn visible(&self) -> bool {
//         true
//     }
//
//     fn enabled(&self) -> bool {
//         true
//     }
//
//     fn hit_test(&self, x: i32, y: i32) -> bool {
//         self.visible() && self.bounds().contains(x, y)
//     }
//
//     fn render(&self, canvas: &mut Canvas, ctx: &Ctx);
//
//     fn handle_event(&mut self, _event: &OverlayEvent, _ctx: &mut Ctx) -> EventResult {
//         EventResult::Propagated
//     }
//
//     fn update(&mut self, _delta: f32) {}
// }
//
// pub struct UI<'a, Ctx> {
//     objects: Vec<Box<dyn UIObject<Ctx> + 'a>>,
//     active: Option<usize>,
//     mouse_x: i32,
//     mouse_y: i32,
// }
//
// impl<'a, Ctx> UI<'a, Ctx> {
//     pub fn new() -> Self {
//         Self {
//             objects: Vec::new(),
//             active: None,
//             mouse_x: 0,
//             mouse_y: 0,
//         }
//     }
//
//     pub fn clear(&mut self) {
//         self.objects.clear();
//         self.active = None;
//     }
//
//     pub fn len(&self) -> usize {
//         self.objects.len()
//     }
//
//     pub fn is_empty(&self) -> bool {
//         self.objects.is_empty()
//     }
//
//     pub fn add<O: UIObject<Ctx> + 'a>(&mut self, object: O) -> usize {
//         self.objects.push(Box::new(object));
//         self.objects.len() - 1
//     }
//
//     pub fn render(&self, canvas: &mut Canvas, ctx: &Ctx) {
//         for o in &self.objects {
//             o.render(canvas, ctx);
//         }
//     }
//
//     pub fn update(&mut self, delta: f32) {
//         for o in &mut self.objects {
//             o.update(delta);
//         }
//     }
//
//     fn send_focus_lost(&mut self, ctx: &mut Ctx, idx: usize) {
//         if idx < self.objects.len() {
//             let _ = self.objects[idx].handle_event(&OverlayEvent::KeyUp { vk: 0 }, ctx);
//             let _ = self.objects[idx].handle_event(&OverlayEvent::MouseUp { button: MouseButton::Left }, ctx);
//         }
//     }
//
//     fn clear_active(&mut self, ctx: &mut Ctx) {
//         if let Some(old) = self.active.take() {
//             if old < self.objects.len() {
//                 let _ = self.objects[old].handle_event(&OverlayEvent::KeyUp { vk: 0 }, ctx);
//                 let _ = self.objects[old].handle_event(&OverlayEvent::MouseUp { button: MouseButton::Left }, ctx);
//                 let _ = self.objects[old].handle_event(&OverlayEvent::MouseWheel { delta: 0 }, ctx);
//             }
//         }
//     }
//
//     fn dispatch_to_active(&mut self, event: &OverlayEvent, ctx: &mut Ctx) -> Option<EventResult> {
//         let idx = self.active?;
//         if idx >= self.objects.len() {
//             self.active = None;
//             return None;
//         }
//         Some(self.objects[idx].handle_event(event, ctx))
//     }
//
//     fn topmost_hit_index(&self, x: i32, y: i32) -> Option<usize> {
//         for (idx, obj) in self.objects.iter().enumerate().rev() {
//             if obj.hit_test(x, y) {
//                 return Some(idx);
//             }
//         }
//         None
//     }
//
//     pub fn handle_event(&mut self, event: &OverlayEvent, ctx: &mut Ctx) -> EventResult {
//         match *event {
//             OverlayEvent::MouseMove { x, y } => {
//                 self.mouse_x = x;
//                 self.mouse_y = y;
//
//                 if let Some(active) = self.active {
//                     if active < self.objects.len() {
//                         if self.objects[active].handle_event(event, ctx) == EventResult::Consumed {
//                             return EventResult::Consumed;
//                         }
//                     }
//                 }
//
//                 for idx in (0..self.objects.len()).rev() {
//                     if Some(idx) == self.active {
//                         continue;
//                     }
//                     if self.objects[idx].hit_test(x, y) {
//                         if self.objects[idx].handle_event(event, ctx) == EventResult::Consumed {
//                             return EventResult::Consumed;
//                         }
//                     }
//                 }
//
//                 EventResult::Propagated
//             }
//
//             OverlayEvent::MouseDown { .. } => {
//                 let mut consumed_by: Option<usize> = None;
//
//                 if let Some(idx) = self.topmost_hit_index(self.mouse_x, self.mouse_y) {
//                     for i in (0..=idx).rev() {
//                         if !self.objects[i].hit_test(self.mouse_x, self.mouse_y) {
//                             continue;
//                         }
//                         if self.objects[i].handle_event(event, ctx) == EventResult::Consumed {
//                             consumed_by = Some(i);
//                             break;
//                         }
//                     }
//                 }
//
//                 if let Some(new_active) = consumed_by {
//                     if self.active != Some(new_active) {
//                         if let Some(old) = self.active.take() {
//                             if old < self.objects.len() {
//                                 let _ = self.objects[old].handle_event(&OverlayEvent::KeyUp { vk: 0 }, ctx);
//                                 let _ = self.objects[old].handle_event(&OverlayEvent::MouseUp { button: MouseButton::Left }, ctx);
//                                 let _ = self.objects[old].handle_event(&OverlayEvent::MouseWheel { delta: 0 }, ctx);
//                             }
//                         }
//                     }
//                     self.active = Some(new_active);
//                     return EventResult::Consumed;
//                 }
//
//                 if let Some(old) = self.active.take() {
//                     if old < self.objects.len() {
//                         let _ = self.objects[old].handle_event(&OverlayEvent::KeyUp { vk: 0 }, ctx);
//                         let _ = self.objects[old].handle_event(&OverlayEvent::MouseUp { button: MouseButton::Left }, ctx);
//                         let _ = self.objects[old].handle_event(&OverlayEvent::MouseWheel { delta: 0 }, ctx);
//                     }
//                 }
//
//                 EventResult::Propagated
//             }
//
//             OverlayEvent::MouseUp { .. } => {
//                 if let Some(active) = self.active {
//                     if active < self.objects.len() {
//                         if self.objects[active].handle_event(event, ctx) == EventResult::Consumed {
//                             return EventResult::Consumed;
//                         }
//                     }
//                 }
//
//                 for idx in (0..self.objects.len()).rev() {
//                     if self.objects[idx].hit_test(self.mouse_x, self.mouse_y) {
//                         if self.objects[idx].handle_event(event, ctx) == EventResult::Consumed {
//                             return EventResult::Consumed;
//                         }
//                     }
//                 }
//
//                 EventResult::Propagated
//             }
//
//             OverlayEvent::KeyDown { .. } | OverlayEvent::KeyUp { .. } | OverlayEvent::MouseWheel { .. } => {
//                 self.dispatch_to_active(event, ctx).unwrap_or(EventResult::Propagated)
//             }
//         }
//     }
// }
//
// #[derive(Clone, Debug)]
// pub struct Panel {
//     pub rect: Rect,
//     pub bg: Color,
//     pub border: Color,
//     pub border_thickness: i32,
//     pub visible: bool,
// }
//
// impl Panel {
//     pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
//         Self {
//             rect: Rect::new(x, y, w, h),
//             bg: colors::BG,
//             border: colors::BORDER,
//             border_thickness: 1,
//             visible: true,
//         }
//     }
//
//     pub fn with_bg(mut self, bg: Color) -> Self {
//         self.bg = bg;
//         self
//     }
//
//     pub fn with_border(mut self, border: Color) -> Self {
//         self.border = border;
//         self
//     }
//
//     pub fn with_border_thickness(mut self, thickness: i32) -> Self {
//         self.border_thickness = thickness.max(0);
//         self
//     }
//
//     pub fn with_rect(mut self, rect: Rect) -> Self {
//         self.rect = rect;
//         self
//     }
// }
//
// impl<Ctx> UIObject<Ctx> for Panel {
//     fn bounds(&self) -> Rect {
//         self.rect
//     }
//
//     fn visible(&self) -> bool {
//         self.visible
//     }
//
//     fn render(&self, canvas: &mut Canvas, _ctx: &Ctx) {
//         if !self.visible {
//             return;
//         }
//         canvas.fill_rect(self.rect.x, self.rect.y, self.rect.w, self.rect.h, self.bg);
//         if self.border_thickness > 0 {
//             canvas.draw_rect_outline(
//                 self.rect.x,
//                 self.rect.y,
//                 self.rect.w,
//                 self.rect.h,
//                 self.border,
//                 self.border_thickness,
//             );
//         }
//     }
// }
//
// #[derive(Clone, Debug)]
// pub struct Label {
//     text: String,
//     rect: Rect,
//     scale: i32,
//     color: Color,
//     background_color: Color,
//     border_color: Color,
//     border_thickness: i32,
//     padding: i32,
//     line_gap: i32,
//     align_h: AlignH,
//     align_v: AlignV,
//     auto_size: bool,
//     visible: bool,
// }
//
// impl Label {
//     pub fn new(text: &str, x: i32, y: i32) -> Self {
//         let scale = 1;
//         let padding = 0;
//         let line_gap = scale.max(1);
//         let (w, h) = measure_text(text, scale, line_gap);
//
//         Self {
//             text: text.to_string(),
//             rect: Rect::new(x, y, w + padding * 2, h + padding * 2),
//             scale,
//             color: colors::TEXT,
//             background_color: colors::TRANSPARENT,
//             border_color: colors::TRANSPARENT,
//             border_thickness: 0,
//             padding,
//             line_gap,
//             align_h: AlignH::Left,
//             align_v: AlignV::Top,
//             auto_size: true,
//             visible: true,
//         }
//     }
//
//     pub fn with_text(mut self, text: &str) -> Self {
//         self.text = text.to_string();
//         self.reflow();
//         self
//     }
//
//     pub fn with_pos(mut self, x: i32, y: i32) -> Self {
//         self.rect.x = x;
//         self.rect.y = y;
//         self
//     }
//
//     pub fn with_size(mut self, w: i32, h: i32) -> Self {
//         self.rect.w = w.max(0);
//         self.rect.h = h.max(0);
//         self.auto_size = false;
//         self
//     }
//
//     pub fn with_scale(mut self, scale: i32) -> Self {
//         self.scale = scale.max(1);
//         self.reflow();
//         self
//     }
//
//     pub fn with_color(mut self, color: Color) -> Self {
//         self.color = color;
//         self
//     }
//
//     pub fn with_background_color(mut self, color: Color) -> Self {
//         self.background_color = color;
//         self
//     }
//
//     pub fn with_border(mut self, color: Color, thickness: i32) -> Self {
//         self.border_color = color;
//         self.border_thickness = thickness.max(0);
//         self
//     }
//
//     pub fn with_padding(mut self, padding: i32) -> Self {
//         self.padding = padding.max(0);
//         self.reflow();
//         self
//     }
//
//     pub fn with_line_gap(mut self, gap: i32) -> Self {
//         self.line_gap = gap.max(0);
//         self.reflow();
//         self
//     }
//
//     pub fn with_align(mut self, h: AlignH, v: AlignV) -> Self {
//         self.align_h = h;
//         self.align_v = v;
//         self
//     }
//
//     pub fn with_visible(mut self, visible: bool) -> Self {
//         self.visible = visible;
//         self
//     }
//
//     fn reflow(&mut self) {
//         if self.auto_size {
//             let (w, h) = measure_text(&self.text, self.scale, self.line_gap);
//             self.rect.w = w + self.padding * 2;
//             self.rect.h = h + self.padding * 2;
//         }
//     }
// }
//
// impl<Ctx> UIObject<Ctx> for Label {
//     fn bounds(&self) -> Rect {
//         self.rect
//     }
//
//     fn visible(&self) -> bool {
//         self.visible
//     }
//
//     fn render(&self, canvas: &mut Canvas, _ctx: &Ctx) {
//         if !self.visible {
//             return;
//         }
//
//         if self.background_color.3 != 0 {
//             canvas.fill_rect(
//                 self.rect.x,
//                 self.rect.y,
//                 self.rect.w,
//                 self.rect.h,
//                 self.background_color,
//             );
//         }
//
//         if self.border_thickness > 0 && self.border_color.3 != 0 {
//             canvas.draw_rect_outline(
//                 self.rect.x,
//                 self.rect.y,
//                 self.rect.w,
//                 self.rect.h,
//                 self.border_color,
//                 self.border_thickness,
//             );
//         }
//
//         let inner_x = self.rect.x + self.padding;
//         let inner_y = self.rect.y + self.padding;
//         let inner_w = (self.rect.w - self.padding * 2).max(0);
//         let inner_h = (self.rect.h - self.padding * 2).max(0);
//
//         draw_text_box(
//             canvas,
//             inner_x,
//             inner_y,
//             inner_w,
//             inner_h,
//             &self.text,
//             self.scale,
//             self.color,
//             self.align_h,
//             self.align_v,
//             self.line_gap,
//         );
//     }
// }
//
// pub struct Button<'a, Ctx> {
//     rect: Rect,
//     text: String,
//     scale: i32,
//     padding: i32,
//     line_gap: i32,
//     text_color: Color,
//     bg: Color,
//     bg_hover: Color,
//     bg_pressed: Color,
//     border: Color,
//     border_thickness: i32,
//     align_h: AlignH,
//     align_v: AlignV,
//     enabled: bool,
//     hovered: bool,
//     pressed: bool,
//     visible: bool,
//     on_click: Box<dyn FnMut(&mut Ctx) + 'a>,
// }
//
// impl<'a, Ctx> Button<'a, Ctx> {
//     pub fn new<F: FnMut(&mut Ctx) + 'a>(
//         text: &str,
//         x: i32,
//         y: i32,
//         w: i32,
//         h: i32,
//         on_click: F,
//     ) -> Self {
//         Self {
//             rect: Rect::new(x, y, w.max(0), h.max(0)),
//             text: text.to_string(),
//             scale: 1,
//             padding: 8,
//             line_gap: 1,
//             text_color: colors::TEXT,
//             bg: colors::BLUE,
//             bg_hover: colors::BLUE_HOVER,
//             bg_pressed: colors::BLUE_PRESSED,
//             border: colors::BORDER,
//             border_thickness: 1,
//             align_h: AlignH::Center,
//             align_v: AlignV::Middle,
//             enabled: true,
//             hovered: false,
//             pressed: false,
//             visible: true,
//             on_click: Box::new(on_click),
//         }
//     }
//
//     pub fn with_text(mut self, text: &str) -> Self {
//         self.text = text.to_string();
//         self
//     }
//
//     pub fn with_pos(mut self, x: i32, y: i32) -> Self {
//         self.rect.x = x;
//         self.rect.y = y;
//         self
//     }
//
//     pub fn with_size(mut self, w: i32, h: i32) -> Self {
//         self.rect.w = w.max(0);
//         self.rect.h = h.max(0);
//         self
//     }
//
//     pub fn with_scale(mut self, scale: i32) -> Self {
//         self.scale = scale.max(1);
//         self
//     }
//
//     pub fn with_padding(mut self, padding: i32) -> Self {
//         self.padding = padding.max(0);
//         self
//     }
//
//     pub fn with_text_color(mut self, color: Color) -> Self {
//         self.text_color = color;
//         self
//     }
//
//     pub fn with_bg(mut self, color: Color) -> Self {
//         self.bg = color;
//         self
//     }
//
//     pub fn with_hover_bg(mut self, color: Color) -> Self {
//         self.bg_hover = color;
//         self
//     }
//
//     pub fn with_pressed_bg(mut self, color: Color) -> Self {
//         self.bg_pressed = color;
//         self
//     }
//
//     pub fn with_border(mut self, color: Color, thickness: i32) -> Self {
//         self.border = color;
//         self.border_thickness = thickness.max(0);
//         self
//     }
//
//     pub fn with_align(mut self, h: AlignH, v: AlignV) -> Self {
//         self.align_h = h;
//         self.align_v = v;
//         self
//     }
//
//     pub fn with_enabled(mut self, enabled: bool) -> Self {
//         self.enabled = enabled;
//         self
//     }
//
//     pub fn with_visible(mut self, visible: bool) -> Self {
//         self.visible = visible;
//         self
//     }
//
//     fn inside(&self, x: i32, y: i32) -> bool {
//         self.rect.contains(x, y)
//     }
// }
//
// impl<Ctx> UIObject<Ctx> for Button<'_, Ctx> {
//     fn bounds(&self) -> Rect {
//         self.rect
//     }
//
//     fn visible(&self) -> bool {
//         self.visible
//     }
//
//     fn enabled(&self) -> bool {
//         self.enabled
//     }
//
//     fn render(&self, canvas: &mut Canvas, _ctx: &Ctx) {
//         if !self.visible {
//             return;
//         }
//
//         let bg = if !self.enabled {
//             colors::BG_2
//         } else if self.pressed {
//             self.bg_pressed
//         } else if self.hovered {
//             self.bg_hover
//         } else {
//             self.bg
//         };
//
//         canvas.fill_rect(self.rect.x, self.rect.y, self.rect.w, self.rect.h, bg);
//
//         if self.border_thickness > 0 {
//             canvas.draw_rect_outline(
//                 self.rect.x,
//                 self.rect.y,
//                 self.rect.w,
//                 self.rect.h,
//                 self.border,
//                 self.border_thickness,
//             );
//         }
//
//         let ix = self.rect.x + self.padding;
//         let iy = self.rect.y + self.padding;
//         let iw = (self.rect.w - self.padding * 2).max(0);
//         let ih = (self.rect.h - self.padding * 2).max(0);
//
//         draw_text_box(
//             canvas,
//             ix,
//             iy,
//             iw,
//             ih,
//             &self.text,
//             self.scale,
//             self.text_color,
//             self.align_h,
//             self.align_v,
//             self.line_gap,
//         );
//     }
//
//     fn handle_event(&mut self, event: &OverlayEvent, ctx: &mut Ctx) -> EventResult {
//         if !self.enabled {
//             return EventResult::Propagated;
//         }
//
//         match *event {
//             OverlayEvent::MouseMove { x, y } => {
//                 self.hovered = self.inside(x, y);
//                 if self.pressed && !self.hovered {
//                     return EventResult::Consumed;
//                 }
//             }
//
//             OverlayEvent::MouseDown { .. } => {
//                 if self.hovered {
//                     self.pressed = true;
//                     return EventResult::Consumed;
//                 }
//             }
//
//             OverlayEvent::MouseUp { .. } => {
//                 let clicked = self.pressed && self.hovered;
//                 self.pressed = false;
//                 if clicked {
//                     (self.on_click)(ctx);
//                     return EventResult::Consumed;
//                 }
//             }
//
//             OverlayEvent::MouseWheel { .. } => {}
//
//             OverlayEvent::KeyDown { .. } | OverlayEvent::KeyUp { .. } => {}
//
//         }
//
//         EventResult::Propagated
//     }
// }
//
// pub struct Checkbox<'a, Ctx> {
//     rect: Rect,
//     size: i32,
//     checked: bool,
//     label: String,
//     label_scale: i32,
//     box_bg: Color,
//     checked_color: Color,
//     border: Color,
//     text_color: Color,
//     enabled: bool,
//     hovered: bool,
//     visible: bool,
//     on_change: Box<dyn FnMut(bool, &mut Ctx) + 'a>,
// }
//
// impl<'a, Ctx> Checkbox<'a, Ctx> {
//     pub fn new<F: FnMut(bool, &mut Ctx) + 'a>(
//         x: i32,
//         y: i32,
//         label: &str,
//         checked: bool,
//         on_change: F,
//     ) -> Self {
//         Self {
//             rect: Rect::new(x, y, 18, 18),
//             size: 18,
//             checked,
//             label: label.to_string(),
//             label_scale: 1,
//             box_bg: colors::BG_2,
//             checked_color: colors::GREEN,
//             border: colors::BORDER,
//             text_color: colors::TEXT,
//             enabled: true,
//             hovered: false,
//             visible: true,
//             on_change: Box::new(on_change),
//         }
//     }
//
//     pub fn with_size(mut self, size: i32) -> Self {
//         self.size = size.max(10);
//         self.rect.w = self.size;
//         self.rect.h = self.size;
//         self
//     }
//
//     pub fn with_label_scale(mut self, scale: i32) -> Self {
//         self.label_scale = scale.max(1);
//         self
//     }
//
//     pub fn with_box_bg(mut self, color: Color) -> Self {
//         self.box_bg = color;
//         self
//     }
//
//     pub fn with_checked_color(mut self, color: Color) -> Self {
//         self.checked_color = color;
//         self
//     }
//
//     pub fn with_border(mut self, color: Color) -> Self {
//         self.border = color;
//         self
//     }
//
//     pub fn with_text_color(mut self, color: Color) -> Self {
//         self.text_color = color;
//         self
//     }
//
//     pub fn with_enabled(mut self, enabled: bool) -> Self {
//         self.enabled = enabled;
//         self
//     }
//
//     pub fn with_visible(mut self, visible: bool) -> Self {
//         self.visible = visible;
//         self
//     }
//
//     pub fn checked(&self) -> bool {
//         self.checked
//     }
//
//     pub fn set_checked(&mut self, checked: bool) {
//         self.checked = checked;
//     }
//
//     fn inside_box(&self, x: i32, y: i32) -> bool {
//         self.rect.contains(x, y)
//     }
// }
//
// impl<Ctx> UIObject<Ctx> for Checkbox<'_, Ctx> {
//     fn bounds(&self) -> Rect {
//         Rect::new(self.rect.x, self.rect.y, self.rect.w + 8 + line_width(&self.label, self.label_scale), self.rect.h)
//     }
//
//     fn visible(&self) -> bool {
//         self.visible
//     }
//
//     fn enabled(&self) -> bool {
//         self.enabled
//     }
//
//     fn render(&self, canvas: &mut Canvas, _ctx: &Ctx) {
//         if !self.visible {
//             return;
//         }
//
//         let box_bg = if self.enabled { self.box_bg } else { colors::BG_2 };
//
//         canvas.fill_rect(self.rect.x, self.rect.y, self.size, self.size, box_bg);
//         canvas.draw_rect_outline(self.rect.x, self.rect.y, self.size, self.size, self.border, 1);
//
//         if self.checked {
//             let pad = (self.size / 5).max(3);
//             canvas.fill_rect(
//                 self.rect.x + pad,
//                 self.rect.y + pad,
//                 self.size - pad * 2,
//                 self.size - pad * 2,
//                 self.checked_color,
//             );
//         }
//
//         let text_x = self.rect.x + self.size + 8;
//         let text_h = 8 * self.label_scale;
//         let text_y = self.rect.y + ((self.size - text_h) / 2).max(0);
//         canvas.draw_text(text_x, text_y, &self.label, self.label_scale, self.text_color);
//     }
//
//     fn handle_event(&mut self, event: &OverlayEvent, ctx: &mut Ctx) -> EventResult {
//         if !self.enabled {
//             return EventResult::Propagated;
//         }
//
//         match *event {
//             OverlayEvent::MouseMove { x, y } => {
//                 self.hovered = self.inside_box(x, y);
//             }
//
//             OverlayEvent::MouseDown { .. } => {
//                 if self.hovered {
//                     self.checked = !self.checked;
//                     (self.on_change)(self.checked, ctx);
//                     return EventResult::Consumed;
//                 }
//             }
//
//             OverlayEvent::MouseUp { .. } => {}
//
//             OverlayEvent::MouseWheel { .. } => {}
//
//             OverlayEvent::KeyDown { .. } | OverlayEvent::KeyUp { .. } => {}
//         }
//
//         EventResult::Propagated
//     }
// }
//
// pub struct Slider<'a, Ctx> {
//     rect: Rect,
//     value: f32,
//     track_bg: Color,
//     track_fill: Color,
//     thumb: Color,
//     border: Color,
//     border_thickness: i32,
//     enabled: bool,
//     hovered: bool,
//     dragging: bool,
//     visible: bool,
//     on_change: Box<dyn FnMut(f32, &mut Ctx) + 'a>,
// }
//
// impl<'a, Ctx> Slider<'a, Ctx> {
//     pub fn new<F: FnMut(f32, &mut Ctx) + 'a>(
//         x: i32,
//         y: i32,
//         w: i32,
//         h: i32,
//         value: f32,
//         on_change: F,
//     ) -> Self {
//         Self {
//             rect: Rect::new(x, y, w.max(1), h.max(8)),
//             value: clamp_f32(value, 0.0, 1.0),
//             track_bg: colors::BG_2,
//             track_fill: colors::BLUE,
//             thumb: colors::TEXT,
//             border: colors::BORDER,
//             border_thickness: 1,
//             enabled: true,
//             hovered: false,
//             dragging: false,
//             visible: true,
//             on_change: Box::new(on_change),
//         }
//     }
//
//     pub fn with_value(mut self, value: f32) -> Self {
//         self.value = clamp_f32(value, 0.0, 1.0);
//         self
//     }
//
//     pub fn with_track_bg(mut self, color: Color) -> Self {
//         self.track_bg = color;
//         self
//     }
//
//     pub fn with_track_fill(mut self, color: Color) -> Self {
//         self.track_fill = color;
//         self
//     }
//
//     pub fn with_thumb(mut self, color: Color) -> Self {
//         self.thumb = color;
//         self
//     }
//
//     pub fn with_border(mut self, color: Color, thickness: i32) -> Self {
//         self.border = color;
//         self.border_thickness = thickness.max(0);
//         self
//     }
//
//     pub fn with_enabled(mut self, enabled: bool) -> Self {
//         self.enabled = enabled;
//         self
//     }
//
//     pub fn with_visible(mut self, visible: bool) -> Self {
//         self.visible = visible;
//         self
//     }
//
//     pub fn value(&self) -> f32 {
//         self.value
//     }
//
//     pub fn set_value(&mut self, value: f32) {
//         self.value = clamp_f32(value, 0.0, 1.0);
//     }
//
//     fn inside(&self, x: i32, y: i32) -> bool {
//         self.rect.contains(x, y)
//     }
//
//     fn set_from_mouse_x(&mut self, x: i32) {
//         let denom = self.rect.w.max(1) as f32;
//         self.value = clamp_f32((x - self.rect.x) as f32 / denom, 0.0, 1.0);
//     }
// }
//
// impl<Ctx> UIObject<Ctx> for Slider<'_, Ctx> {
//     fn bounds(&self) -> Rect {
//         self.rect
//     }
//
//     fn visible(&self) -> bool {
//         self.visible
//     }
//
//     fn enabled(&self) -> bool {
//         self.enabled
//     }
//
//     fn render(&self, canvas: &mut Canvas, _ctx: &Ctx) {
//         if !self.visible {
//             return;
//         }
//
//         canvas.fill_rect(self.rect.x, self.rect.y, self.rect.w, self.rect.h, self.track_bg);
//
//         let fill_w = ((self.rect.w as f32) * self.value).round() as i32;
//         if fill_w > 0 {
//             canvas.fill_rect(self.rect.x, self.rect.y, fill_w, self.rect.h, self.track_fill);
//         }
//
//         if self.border_thickness > 0 {
//             canvas.draw_rect_outline(
//                 self.rect.x,
//                 self.rect.y,
//                 self.rect.w,
//                 self.rect.h,
//                 self.border,
//                 self.border_thickness,
//             );
//         }
//
//         let thumb_w = 10;
//         let thumb_h = (self.rect.h - 4).max(4);
//         let thumb_x = self.rect.x + ((self.rect.w - thumb_w) as f32 * self.value).round() as i32;
//         let thumb_y = self.rect.y + (self.rect.h - thumb_h) / 2;
//
//         let thumb_color = if self.dragging {
//             colors::WHITE
//         } else if self.hovered {
//             self.thumb
//         } else {
//             self.thumb
//         };
//
//         canvas.fill_rect(thumb_x, thumb_y, thumb_w, thumb_h, thumb_color);
//         canvas.draw_rect_outline(thumb_x, thumb_y, thumb_w, thumb_h, self.border, 1);
//     }
//
//     fn handle_event(&mut self, event: &OverlayEvent, ctx: &mut Ctx) -> EventResult {
//         if !self.enabled {
//             return EventResult::Propagated;
//         }
//
//         match *event {
//             OverlayEvent::MouseMove { x, y } => {
//                 self.hovered = self.inside(x, y);
//                 if self.dragging {
//                     self.set_from_mouse_x(x);
//                     (self.on_change)(self.value, ctx);
//                     return EventResult::Consumed;
//                 }
//             }
//
//             OverlayEvent::MouseDown { .. } => {
//                 if self.hovered {
//                     self.dragging = true;
//                     self.set_from_mouse_x(self.rect.x + self.rect.w / 2);
//                     (self.on_change)(self.value, ctx);
//                     return EventResult::Consumed;
//                 }
//             }
//
//             OverlayEvent::MouseUp { .. } => {
//                 if self.dragging {
//                     self.dragging = false;
//                     return EventResult::Consumed;
//                 }
//             }
//
//             OverlayEvent::MouseWheel { delta } => {
//                 if self.hovered {
//                     let step = if delta > 0 { 0.03 } else { -0.03 };
//                     self.value = clamp_f32(self.value + step, 0.0, 1.0);
//                     (self.on_change)(self.value, ctx);
//                     return EventResult::Consumed;
//                 }
//             }
//
//             OverlayEvent::KeyDown { .. } | OverlayEvent::KeyUp { .. } => {}
//         }
//
//         EventResult::Propagated
//     }
// }
//
// pub struct ProgressBar {
//     rect: Rect,
//     value: f32,
//     bg: Color,
//     fill: Color,
//     border: Color,
//     border_thickness: i32,
//     visible: bool,
// }
//
// impl ProgressBar {
//     pub fn new(x: i32, y: i32, w: i32, h: i32, value: f32) -> Self {
//         Self {
//             rect: Rect::new(x, y, w.max(1), h.max(1)),
//             value: clamp_f32(value, 0.0, 1.0),
//             bg: colors::BG_2,
//             fill: colors::GREEN,
//             border: colors::BORDER,
//             border_thickness: 1,
//             visible: true,
//         }
//     }
//
//     pub fn with_value(mut self, value: f32) -> Self {
//         self.value = clamp_f32(value, 0.0, 1.0);
//         self
//     }
//
//     pub fn with_bg(mut self, color: Color) -> Self {
//         self.bg = color;
//         self
//     }
//
//     pub fn with_fill(mut self, color: Color) -> Self {
//         self.fill = color;
//         self
//     }
//
//     pub fn with_border(mut self, color: Color, thickness: i32) -> Self {
//         self.border = color;
//         self.border_thickness = thickness.max(0);
//         self
//     }
//
//     pub fn with_visible(mut self, visible: bool) -> Self {
//         self.visible = visible;
//         self
//     }
// }
//
// impl<Ctx> UIObject<Ctx> for ProgressBar {
//     fn bounds(&self) -> Rect {
//         self.rect
//     }
//
//     fn visible(&self) -> bool {
//         self.visible
//     }
//
//     fn render(&self, canvas: &mut Canvas, _ctx: &Ctx) {
//         if !self.visible {
//             return;
//         }
//
//         canvas.fill_rect(self.rect.x, self.rect.y, self.rect.w, self.rect.h, self.bg);
//
//         let filled = ((self.rect.w as f32) * self.value).round() as i32;
//         if filled > 0 {
//             canvas.fill_rect(self.rect.x, self.rect.y, filled, self.rect.h, self.fill);
//         }
//
//         if self.border_thickness > 0 {
//             canvas.draw_rect_outline(
//                 self.rect.x,
//                 self.rect.y,
//                 self.rect.w,
//                 self.rect.h,
//                 self.border,
//                 self.border_thickness,
//             );
//         }
//     }
// }
//
// pub struct TextField<'a, Ctx> {
//     rect: Rect,
//     text: String,
//     placeholder: String,
//     cursor: usize, // char index
//     scale: i32,
//     padding: i32,
//     line_gap: i32,
//     text_color: Color,
//     placeholder_color: Color,
//     bg: Color,
//     bg_focus: Color,
//     border: Color,
//     border_focus: Color,
//     caret: Color,
//     border_thickness: i32,
//     enabled: bool,
//     focused: bool,
//     hovered: bool,
//     visible: bool,
//     on_change: Box<dyn FnMut(&str, &mut Ctx) + 'a>,
//     on_submit: Box<dyn FnMut(&str, &mut Ctx) + 'a>,
// }
//
// impl<'a, Ctx> TextField<'a, Ctx> {
//     pub fn new<F1, F2>(
//         x: i32,
//         y: i32,
//         w: i32,
//         h: i32,
//         text: &str,
//         on_change: F1,
//         on_submit: F2,
//     ) -> Self
//     where
//         F1: FnMut(&str, &mut Ctx) + 'a,
//         F2: FnMut(&str, &mut Ctx) + 'a,
//     {
//         let text = text.to_string();
//         let cursor = text.chars().count();
//
//         Self {
//             rect: Rect::new(x, y, w.max(1), h.max(10)),
//             text,
//             placeholder: String::new(),
//             cursor,
//             scale: 1,
//             padding: 6,
//             line_gap: 1,
//             text_color: colors::TEXT,
//             placeholder_color: colors::MUTED,
//             bg: colors::BG_2,
//             bg_focus: colors::BG,
//             border: colors::BORDER,
//             border_focus: colors::BLUE,
//             caret: colors::WHITE,
//             border_thickness: 1,
//             enabled: true,
//             focused: false,
//             hovered: false,
//             visible: true,
//             on_change: Box::new(on_change),
//             on_submit: Box::new(on_submit),
//         }
//     }
//
//     pub fn with_placeholder(mut self, placeholder: &str) -> Self {
//         self.placeholder = placeholder.to_string();
//         self
//     }
//
//     pub fn with_scale(mut self, scale: i32) -> Self {
//         self.scale = scale.max(1);
//         self
//     }
//
//     pub fn with_padding(mut self, padding: i32) -> Self {
//         self.padding = padding.max(0);
//         self
//     }
//
//     pub fn with_colors(
//         mut self,
//         text: Color,
//         placeholder: Color,
//         bg: Color,
//         bg_focus: Color,
//         border: Color,
//         border_focus: Color,
//         caret: Color,
//     ) -> Self {
//         self.text_color = text;
//         self.placeholder_color = placeholder;
//         self.bg = bg;
//         self.bg_focus = bg_focus;
//         self.border = border;
//         self.border_focus = border_focus;
//         self.caret = caret;
//         self
//     }
//
//     pub fn with_border_thickness(mut self, thickness: i32) -> Self {
//         self.border_thickness = thickness.max(0);
//         self
//     }
//
//     pub fn with_enabled(mut self, enabled: bool) -> Self {
//         self.enabled = enabled;
//         self
//     }
//
//     pub fn with_visible(mut self, visible: bool) -> Self {
//         self.visible = visible;
//         self
//     }
//
//     pub fn text(&self) -> &str {
//         &self.text
//     }
//
//     pub fn set_text(&mut self, text: &str) {
//         self.text = text.to_string();
//         self.cursor = self.text.chars().count();
//     }
//
//     fn inside(&self, x: i32, y: i32) -> bool {
//         self.rect.contains(x, y)
//     }
//
//     fn clamp_cursor(&mut self) {
//         let len = self.text.chars().count();
//         self.cursor = clamp_i32(self.cursor as i32, 0, len as i32) as usize;
//     }
//
//     fn notify_change(&mut self, ctx: &mut Ctx) {
//         (self.on_change)(&self.text, ctx);
//     }
//
//     fn notify_submit(&mut self, ctx: &mut Ctx) {
//         (self.on_submit)(&self.text, ctx);
//     }
//
//     fn insert_char(&mut self, ch: char) {
//         self.clamp_cursor();
//         let idx = char_to_byte_idx(&self.text, self.cursor);
//         self.text.insert(idx, ch);
//         self.cursor += 1;
//     }
//
//     fn backspace(&mut self) -> bool {
//         self.clamp_cursor();
//         if self.cursor == 0 {
//             return false;
//         }
//         let remove_at_char = self.cursor - 1;
//         let idx = char_to_byte_idx(&self.text, remove_at_char);
//         self.text.remove(idx);
//         self.cursor -= 1;
//         true
//     }
//
//     fn delete_at_cursor(&mut self) -> bool {
//         self.clamp_cursor();
//         let len = self.text.chars().count();
//         if self.cursor >= len {
//             return false;
//         }
//         let idx = char_to_byte_idx(&self.text, self.cursor);
//         self.text.remove(idx);
//         true
//     }
//
//     fn move_left(&mut self) -> bool {
//         if self.cursor > 0 {
//             self.cursor -= 1;
//             return true;
//         }
//         false
//     }
//
//     fn move_right(&mut self) -> bool {
//         let len = self.text.chars().count();
//         if self.cursor < len {
//             self.cursor += 1;
//             return true;
//         }
//         false
//     }
//
//     fn prefix_width(&self) -> i32 {
//         let idx = char_to_byte_idx(&self.text, self.cursor);
//         line_width(&self.text[..idx], self.scale)
//     }
// }
//
// impl<Ctx> UIObject<Ctx> for TextField<'_, Ctx> {
//     fn bounds(&self) -> Rect {
//         self.rect
//     }
//
//     fn visible(&self) -> bool {
//         self.visible
//     }
//
//     fn enabled(&self) -> bool {
//         self.enabled
//     }
//
//     fn render(&self, canvas: &mut Canvas, _ctx: &Ctx) {
//         if !self.visible {
//             return;
//         }
//
//         let bg = if self.focused { self.bg_focus } else { self.bg };
//         let border = if self.focused { self.border_focus } else { self.border };
//
//         canvas.fill_rect(self.rect.x, self.rect.y, self.rect.w, self.rect.h, bg);
//
//         if self.border_thickness > 0 {
//             canvas.draw_rect_outline(
//                 self.rect.x,
//                 self.rect.y,
//                 self.rect.w,
//                 self.rect.h,
//                 border,
//                 self.border_thickness,
//             );
//         }
//
//         let inner_x = self.rect.x + self.padding;
//         let inner_y = self.rect.y + self.padding;
//         let inner_w = (self.rect.w - self.padding * 2).max(0);
//         let inner_h = (self.rect.h - self.padding * 2).max(0);
//
//         let draw_text = if self.text.is_empty() {
//             &self.placeholder
//         } else {
//             &self.text
//         };
//
//         let draw_color = if self.text.is_empty() {
//             self.placeholder_color
//         } else {
//             self.text_color
//         };
//
//         canvas.draw_text(inner_x, inner_y, draw_text, self.scale, draw_color);
//
//         if self.focused {
//             let caret_x = inner_x + self.prefix_width();
//             let caret_y = inner_y;
//             let caret_h = (8 * self.scale).max(1);
//             let caret_w = 1;
//
//             if caret_x >= inner_x && caret_x < inner_x + inner_w {
//                 canvas.fill_rect(caret_x, caret_y, caret_w, caret_h.min(inner_h), self.caret);
//             }
//         }
//     }
//
//     fn handle_event(&mut self, event: &OverlayEvent, ctx: &mut Ctx) -> EventResult {
//         if !self.enabled {
//             return EventResult::Propagated;
//         }
//
//         match *event {
//             OverlayEvent::MouseMove { x, y } => {
//                 self.hovered = self.inside(x, y);
//             }
//
//             OverlayEvent::MouseDown { .. } => {
//                 if self.hovered {
//                     self.focused = true;
//                     self.cursor = self.text.chars().count();
//                     return EventResult::Consumed;
//                 }
//             }
//
//             OverlayEvent::MouseUp { .. } => {}
//
//             OverlayEvent::MouseWheel { .. } => {}
//
//             OverlayEvent::KeyDown { vk } => {
//                 if !self.focused {
//                     return EventResult::Propagated;
//                 }
//
//                 const VK_BACK: u32 = 0x08;
//                 const VK_TAB: u32 = 0x09;
//                 const VK_RETURN: u32 = 0x0D;
//                 const VK_LEFT: u32 = 0x25;
//                 const VK_RIGHT: u32 = 0x27;
//                 const VK_HOME: u32 = 0x24;
//                 const VK_END: u32 = 0x23;
//                 const VK_DELETE: u32 = 0x2E;
//
//                 let mut changed = false;
//
//                 match vk {
//                     VK_BACK => {
//                         changed = self.backspace();
//                     }
//                     VK_DELETE => {
//                         changed = self.delete_at_cursor();
//                     }
//                     VK_LEFT => {
//                         self.move_left();
//                     }
//                     VK_RIGHT => {
//                         self.move_right();
//                     }
//                     VK_HOME => {
//                         self.cursor = 0;
//                     }
//                     VK_END => {
//                         self.cursor = self.text.chars().count();
//                     }
//                     VK_RETURN => {
//                         self.notify_submit(ctx);
//                         return EventResult::Consumed;
//                     }
//                     VK_TAB => {
//                         return EventResult::Consumed;
//                     }
//                     _ => {
//                         if let Some(ch) = vk_to_char(vk) {
//                             self.insert_char(ch);
//                             changed = true;
//                         }
//                     }
//                 }
//
//                 if changed {
//                     self.notify_change(ctx);
//                     return EventResult::Consumed;
//                 }
//             }
//
//             OverlayEvent::KeyUp { .. } => {}
//
//         }
//
//         EventResult::Propagated
//     }
// }