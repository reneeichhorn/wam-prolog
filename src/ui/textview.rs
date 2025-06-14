use ratatui::{prelude::*, widgets::StatefulWidget};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr; // remember to add `unicode-width = "0.2"` in Cargo.toml // remember to add `unicode-segmentation = "1.10"` in Cargo.toml

/// Widget (pure data – no mutable state inside)
pub struct TextView<'a> {
    pub text: &'a str,
    pub tab_width: usize,     // how many spaces one `\t` becomes
    pub style: Style,         // text background / foreground
    pub line_no_style: Style, // style for the numbers
    pub start_line: usize,
}

/// Mutable state the application owns (how far we scrolled, etc.)
#[derive(Default, Debug, Clone)]
pub struct TextViewState {
    pub scroll: u16, // first visible line (0-based)
}

impl<'a> StatefulWidget for TextView<'a> {
    type State = TextViewState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // ----------- Pre-compute some invariants ---------
        let mut lines: Vec<String> = self
            .text
            .split('\n')
            .map(|l| l.replace('\t', &" ".repeat(self.tab_width)))
            .collect();
        // keep one empty element when the string ends with '\n'
        if self.text.ends_with('\n') {
            lines.push(String::new());
        }
        let total = lines.len() as u16;

        // clamp scroll to valid range
        let max_scroll = total.saturating_sub(area.height);
        state.scroll = state.scroll.min(max_scroll);

        // dynamic width for line numbers
        let no_digits = ((total as f32).log10().floor() as usize) + 1;
        let gutter = no_digits + 1; // “NN␠”
        let text_cols = area.width.saturating_sub(gutter as u16 + 1); // −1 for the scrollbar

        // ---------- Paint background so that blanks keep the colour ----------
        buf.set_style(area, self.style);

        // ---------- Draw each visible line ----------
        for (row, idx) in (state.scroll..state.scroll + area.height).enumerate() {
            if idx >= total {
                break;
            }
            let y = area.y + row as u16;
            let ln = format!(
                "{:>width$} ",
                idx + self.start_line as u16,
                width = no_digits
            ); // right-aligned
            buf.set_stringn(area.x, y, &ln, gutter, self.line_no_style); // number + space

            let content = &lines[idx as usize];
            // cut to fit – account for real glyph widths
            let mut used = 0;
            let mut rendered = String::new();
            for g in content.graphemes(true) {
                let w = UnicodeWidthStr::width(g);
                if used + w > text_cols as usize {
                    break;
                }
                rendered.push_str(g);
                used += w;
            }
            buf.set_stringn(
                area.x + gutter as u16,
                y,
                &rendered,
                text_cols as usize,
                self.style,
            );
        }

        // ---------- Draw the scrollbar ----------
        draw_scrollbar(
            buf,
            Rect {
                x: area.right() - 1,
                y: area.y,
                width: 1,
                height: area.height,
            },
            state.scroll,
            total,
        );
    }
}

/// A minimal scrollbar (track = │, thumb = █)
fn draw_scrollbar(buf: &mut Buffer, track: Rect, offset: u16, total: u16) {
    if total <= track.height {
        // everything fits – gray track without a thumb
        for y in 0..track.height {
            buf.get_mut(track.x, track.y + y)
                .set_symbol("│")
                .set_style(Style::default().fg(Color::DarkGray));
        }
        return;
    }

    // paint track first
    for y in 0..track.height {
        buf.get_mut(track.x, track.y + y)
            .set_symbol("│")
            .set_style(Style::default().fg(Color::DarkGray));
    }

    // thumb geometry
    let ratio = track.height as f32 / total as f32;
    let thumb_h = (ratio * track.height as f32).ceil().max(1.0) as u16;
    let thumb_y = ((offset as f32 / total as f32) * track.height as f32).floor() as u16;

    for y in 0..thumb_h.min(track.height) {
        buf.get_mut(track.x, track.y + thumb_y + y)
            .set_symbol("█")
            .set_style(Style::default().fg(Color::Gray));
    }
}
