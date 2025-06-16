use color_eyre::owo_colors::OwoColorize;
use ratatui::{prelude::*, widgets::StatefulWidget};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{descriptor::DescriptorAllocator, instructions::Instruction, interpreter::Interpreter}; // remember to add `unicode-width = "0.2"` in Cargo.toml // remember to add `unicode-segmentation = "1.10"` in Cargo.toml

/// Widget (pure data – no mutable state inside)
pub struct InstructionView<'a> {
    pub descriptors: &'a DescriptorAllocator,
    pub interpreter: &'a Interpreter,
    pub instructions: &'a [crate::instructions::Instruction],
}

/// Mutable state the application owns (how far we scrolled, etc.)
#[derive(Default, Debug, Clone)]
pub struct InstructionViewState {
    pub scroll: u16, // first visible line (0-based)
}

impl<'a> StatefulWidget for InstructionView<'a> {
    type State = InstructionViewState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let tab_width = 2;
        let line_no_style = ratatui::style::Style::default().fg(Color::LightCyan);
        let style = ratatui::style::Style::default().fg(Color::White);
        let line_no_indicator_style = ratatui::style::Style::default().fg(Color::White);

        let text = self
            .instructions
            .iter()
            .map(|i| match i {
                Instruction::PutStructure {
                    structure,
                    register,
                } => {
                    format!(
                        "put_structure {}, X{}",
                        self.descriptors.get(*structure).pretty_name(),
                        register.0 + 1
                    )
                }
                Instruction::SetVariable { register } => {
                    format!("set_variable X{}", register.0 + 1)
                }
                Instruction::SetValue { register } => {
                    format!("set_value X{}", register.0 + 1)
                }
                Instruction::DebugComment { message } => {
                    format!(";; {}", message)
                }
                Instruction::GetStructure {
                    structure,
                    register,
                } => {
                    format!(
                        "get_structure {}, X{}",
                        self.descriptors.get(*structure).pretty_name(),
                        register.0 + 1
                    )
                }
                Instruction::UnifyVariable { register } => {
                    format!("unify_variable X{}", register.0 + 1)
                }
                Instruction::UnifyValue { register } => {
                    format!("unify_value X{}", register.0 + 1)
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        // ----------- Pre-compute some invariants ---------
        let mut lines: Vec<String> = text
            .split('\n')
            .map(|l| l.replace('\t', &" ".repeat(tab_width)))
            .collect();
        // keep one empty element when the string ends with '\n'
        if text.ends_with('\n') {
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
        buf.set_style(area, style);

        // ---------- Draw each visible line ----------
        for (row, idx) in (state.scroll..state.scroll + area.height).enumerate() {
            if idx >= total {
                break;
            }

            let with_active_style = |style: Style| {
                if self.interpreter.instruction_index == row {
                    style.bg(Color::LightGreen).fg(Color::Black)
                } else {
                    style
                }
            };

            let y = area.y + row as u16;
            let indicator = if self.interpreter.instruction_index == row {
                " ▶ ".to_string()
            } else {
                "   ".to_string()
            }; // right-aligned
            buf.set_stringn(
                area.x,
                y,
                indicator,
                3,
                with_active_style(line_no_indicator_style),
            ); // number + space

            let ln = format!("{:>width$} ", idx + 1, width = no_digits); // right-aligned
            buf.set_stringn(area.x + 3, y, &ln, gutter, with_active_style(line_no_style)); // number + space

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
                area.x + gutter as u16 + 3,
                y,
                &rendered,
                text_cols as usize,
                with_active_style(style),
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
