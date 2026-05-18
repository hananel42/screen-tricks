// use fontdue::{Font, FontSettings, Metrics};
// use unicode_segmentation::UnicodeSegmentation;
// use crate::overlay::Canvas;
// 
// #[derive(Clone, Copy, Debug, PartialEq, Eq)]
// pub enum TextDirection {
//     Auto,
//     Ltr,
//     Rtl,
// }
// 
// #[derive(Clone, Copy, Debug, PartialEq, Eq)]
// pub enum TextAlign {
//     Left,
//     Center,
//     Right,
// }
// 
// #[derive(Clone, Copy, Debug)]
// pub struct TextStyle {
//     pub size: f32,
//     pub color: (u8, u8, u8, u8),
//     pub direction: TextDirection,
//     pub align: TextAlign,
//     pub wrap_width: Option<i32>,
//     pub letter_spacing: f32,
//     pub line_gap: f32,
//     pub line_height: Option<f32>,
// }
// 
// impl Default for TextStyle {
//     fn default() -> Self {
//         Self {
//             size: 16.0,
//             color: (255, 255, 255, 255),
//             direction: TextDirection::Auto,
//             align: TextAlign::Left,
//             wrap_width: None,
//             letter_spacing: 0.0,
//             line_gap: 2.0,
//             line_height: None,
//         }
//     }
// }
// 
// #[derive(Clone, Debug)]
// pub struct FontFace {
//     pub name: String,
//     pub font: Font,
// }
// 
// #[derive(Clone, Debug, Default)]
// pub struct TextEngine {
//     pub faces: Vec<FontFace>,
// }
// 
// #[derive(Clone, Copy, Debug)]
// pub struct TextMetrics {
//     pub width: i32,
//     pub height: i32,
//     pub lines: usize,
// }
// 
// #[derive(Clone, Debug)]
// enum Token<'a> {
//     Text(&'a str),
//     Space(&'a str),
// }
// 
// #[derive(Clone, Debug)]
// struct Line<'a> {
//     tokens: Vec<Token<'a>>,
//     width: f32,
// }
// 
// impl TextEngine {
//     pub fn new() -> Self {
//         Self { faces: Vec::new() }
//     }
// 
//     pub fn add_font_bytes(
//         &mut self,
//         name: impl Into<String>,
//         bytes: impl AsRef<[u8]>,
//     ) -> Result<usize, String> {
//         let font = Font::from_bytes(bytes.as_ref(), FontSettings::default())
//             .map_err(|e| format!("failed to load font: {e:?}"))?;
// 
//         let id = self.faces.len();
//         self.faces.push(FontFace {
//             name: name.into(),
//             font,
//         });
//         Ok(id)
//     }
// 
//     pub fn is_empty(&self) -> bool {
//         self.faces.is_empty()
//     }
// 
//     fn pick_font_for_char(&self, ch: char) -> usize {
//         if self.faces.is_empty() {
//             return 0;
//         }
// 
//         for (i, face) in self.faces.iter().enumerate() {
//             if face.font.lookup_glyph_index(ch) != 0 {
//                 return i;
//             }
//         }
// 
//         0
//     }
// 
//     fn pick_font_for_text(&self, s: &str) -> usize {
//         if self.faces.is_empty() {
//             return 0;
//         }
// 
//         for ch in s.chars() {
//             let i = self.pick_font_for_char(ch);
//             if i != 0 || self.faces[0].font.lookup_glyph_index(ch) != 0 {
//                 return i;
//             }
//         }
// 
//         0
//     }
// 
//     fn is_rtl_strong(ch: char) -> bool {
//         matches!(
//             ch,
//             '\u{0590}'..='\u{08FF}'
//                 | '\u{FB1D}'..='\u{FDFF}'
//                 | '\u{FE70}'..='\u{FEFF}'
//         )
//     }
// 
//     fn resolve_direction(&self, text: &str, requested: TextDirection) -> TextDirection {
//         match requested {
//             TextDirection::Ltr => TextDirection::Ltr,
//             TextDirection::Rtl => TextDirection::Rtl,
//             TextDirection::Auto => {
//                 for ch in text.chars() {
//                     if ch.is_whitespace() {
//                         continue;
//                     }
//                     if Self::is_rtl_strong(ch) {
//                         return TextDirection::Rtl;
//                     }
//                     if ch.is_alphabetic() || ch.is_ascii_digit() {
//                         return TextDirection::Ltr;
//                     }
//                 }
//                 TextDirection::Ltr
//             }
//         }
//     }
// 
//     fn tokenize_line<'a>(&self, line: &'a str) -> Vec<Token<'a>> {
//         let mut tokens = Vec::new();
// 
//         for piece in line.split_word_bounds() {
//             if piece.is_empty() {
//                 continue;
//             }
//             if piece.chars().all(char::is_whitespace) {
//                 tokens.push(Token::Space(piece));
//             } else {
//                 tokens.push(Token::Text(piece));
//             }
//         }
// 
//         tokens
//     }
// 
//     fn measure_cluster_width(&self, cluster: &str, size: f32, letter_spacing: f32) -> f32 {
//         if self.faces.is_empty() {
//             return 0.0;
//         }
// 
//         let mut width = 0.0f32;
//         let mut first = true;
// 
//         for ch in cluster.chars() {
//             if !first {
//                 width += letter_spacing;
//             }
//             first = false;
// 
//             let face_index = self.pick_font_for_char(ch);
//             let face = &self.faces[face_index].font;
//             let metrics: Metrics = face.metrics(ch, size);
//             width += metrics.advance_width;
//         }
// 
//         width
//     }
// 
//     fn measure_token_width(&self, token: &Token<'_>, size: f32, letter_spacing: f32) -> f32 {
//         match token {
//             Token::Space(s) => self.measure_cluster_width(s, size, letter_spacing),
//             Token::Text(s) => self.measure_cluster_width(s, size, letter_spacing),
//         }
//     }
// 
//     fn layout_lines<'a>(&self, text: &'a str, style: &TextStyle) -> (Vec<Line<'a>>, TextDirection) {
//         let dir = self.resolve_direction(text, style.direction);
//         let wrap_width = style.wrap_width.map(|w| w.max(1) as f32);
// 
//         let mut lines: Vec<Line<'a>> = Vec::new();
// 
//         for raw_line in text.split('\n') {
//             let tokens = self.tokenize_line(raw_line);
// 
//             if wrap_width.is_none() {
//                 let mut width = 0.0;
//                 for t in &tokens {
//                     width += self.measure_token_width(t, style.size, style.letter_spacing);
//                 }
//                 lines.push(Line { tokens, width });
//                 continue;
//             }
// 
//             let max_w = wrap_width.unwrap();
//             let mut current: Vec<Token<'a>> = Vec::new();
//             let mut current_w = 0.0f32;
// 
//             for token in tokens {
//                 let token_w = self.measure_token_width(&token, style.size, style.letter_spacing);
// 
//                 let would_wrap = !current.is_empty() && current_w + token_w > max_w;
// 
//                 if would_wrap {
//                     lines.push(Line {
//                         tokens: current,
//                         width: current_w,
//                     });
//                     current = Vec::new();
//                     current_w = 0.0;
//                 }
// 
//                 if token_w <= max_w || current.is_empty() {
//                     current_w += token_w;
//                     current.push(token);
//                 } else {
//                     match token {
//                         Token::Text(s) | Token::Space(s) => {
//                             let mut buf = String::new();
//                             let mut buf_w = 0.0f32;
// 
//                             for g in s.graphemes(true) {
//                                 let gw = self.measure_cluster_width(g, style.size, style.letter_spacing);
// 
//                                 if !buf.is_empty() && buf_w + gw > max_w {
//                                     lines.push(Line {
//                                         tokens: vec![Token::Text(Box::leak(buf.into_boxed_str()))],
//                                         width: buf_w,
//                                     });
//                                     buf = String::new();
//                                     buf_w = 0.0;
//                                 }
// 
//                                 buf.push_str(g);
//                                 buf_w += gw;
//                             }
// 
//                             if !buf.is_empty() {
//                                 current_w += buf_w;
//                                 current.push(Token::Text(Box::leak(buf.into_boxed_str())));
//                             }
//                         }
//                     }
//                 }
//             }
// 
//             lines.push(Line {
//                 tokens: current,
//                 width: current_w,
//             });
//         }
// 
//         (lines, dir)
//     }
// 
//     pub fn measure(&self, text: &str, style: &TextStyle) -> TextMetrics {
//         if self.faces.is_empty() {
//             return TextMetrics {
//                 width: 0,
//                 height: 0,
//                 lines: 0,
//             };
//         }
// 
//         let (lines, _) = self.layout_lines(text, style);
// 
//         let mut max_w = 0.0f32;
//         for line in &lines {
//             max_w = max_w.max(line.width);
//         }
// 
//         let line_h = style.line_height.unwrap_or(style.size * 1.35 + style.line_gap);
//         let total_h = (lines.len() as f32 * line_h).ceil() as i32;
// 
//         TextMetrics {
//             width: max_w.ceil() as i32,
//             height: total_h,
//             lines: lines.len(),
//         }
//     }
// 
//     fn line_start_x(&self, box_x: i32, box_w: Option<i32>, line_w: f32, align: TextAlign) -> i32 {
//         match align {
//             TextAlign::Left => box_x,
//             TextAlign::Center => {
//                 let w = box_w.unwrap_or(line_w.ceil() as i32);
//                 box_x + ((w as f32 - line_w) * 0.5).round() as i32
//             }
//             TextAlign::Right => {
//                 let w = box_w.unwrap_or(line_w.ceil() as i32);
//                 box_x + (w as f32 - line_w).round() as i32
//             }
//         }
//     }
// 
//     fn ordered_tokens_for_direction<'a>(tokens: &'a [Token<'a>], dir: TextDirection) -> Vec<Token<'a>> {
//         match dir {
//             TextDirection::Ltr => tokens.to_vec(),
//             TextDirection::Rtl => {
//                 let mut out = Vec::with_capacity(tokens.len());
//                 for token in tokens.iter().rev() {
//                     match token {
//                         Token::Text(s) => {
//                             let rev: String = s.graphemes(true).rev().collect();
//                             out.push(Token::Text(Box::leak(rev.into_boxed_str())));
//                         }
//                         Token::Space(s) => out.push(Token::Space(s)),
//                     }
//                 }
//                 out
//             }
//             TextDirection::Auto => tokens.to_vec(),
//         }
//     }
// 
// 
// 
//     fn draw_cluster(
//         &self,
//         canvas: &mut Canvas,
//         x: i32,
//         baseline_y: i32,
//         cluster: &str,
//         style: &TextStyle,
//     ) -> i32 {
//         if self.faces.is_empty() || cluster.is_empty() {
//             return x;
//         }
// 
//         let mut pen_x = x;
//         let mut first = true;
// 
//         for ch in cluster.chars() {
//             if !first {
//                 pen_x += style.letter_spacing.round() as i32;
//             }
//             first = false;
// 
//             let face_index = self.pick_font_for_char(ch);
//             let face = &self.faces[face_index].font;
// 
//             let (metrics, bitmap) = face.rasterize(ch, style.size);
// 
//             canvas.blit_glyph( pen_x, baseline_y, &metrics, &bitmap, style.color);
//             pen_x += metrics.advance_width.round() as i32;
//         }
// 
//         pen_x
//     }
// 
//     pub fn draw(
//         &self,
//         canvas: &mut Canvas,
//         x: i32,
//         y: i32,
//         text: &str,
//         style: &TextStyle,
//     ) {
//         if self.faces.is_empty() || text.is_empty() {
//             return;
//         }
// 
//         let (lines, dir_resolved) = self.layout_lines(text, style);
//         let line_h = style.line_height.unwrap_or(style.size * 1.35 + style.line_gap);
//         let line_h_px = line_h.ceil() as i32;
//         let baseline_offset = style.size.ceil() as i32;
// 
//         for (line_idx, line) in lines.iter().enumerate() {
//             let ordered = Self::ordered_tokens_for_direction(&line.tokens, dir_resolved);
//             let start_x = self.line_start_x(
//                 x,
//                 style.wrap_width,
//                 line.width,
//                 style.align,
//             );
// 
//             let baseline_y = y + (line_idx as i32 * line_h_px) + baseline_offset;
//             let mut pen_x = start_x;
// 
//             for token in ordered {
//                 match token {
//                     Token::Space(s) | Token::Text(s) => {
//                         for cluster in s.graphemes(true) {
//                             pen_x = self.draw_cluster(canvas, pen_x, baseline_y, cluster, style);
//                         }
//                     }
//                 }
//             }
//         }
//     }
// 
//     pub fn draw_box(
//         &self,
//         canvas: &mut Canvas,
//         x: i32,
//         y: i32,
//         w: i32,
//         text: &str,
//         mut style: TextStyle,
//     ) {
//         style.wrap_width = Some(w);
//         self.draw(canvas, x, y, text, &style);
//     }
// }
