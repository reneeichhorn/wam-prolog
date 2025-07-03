use color_eyre::owo_colors::OwoColorize;
use ratatui::{
    prelude::*,
    text::{Line, Span},
    widgets::StatefulWidget,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{
    descriptor::DescriptorAllocator,
    instructions::{Instruction, RegisterId},
    interpreter::Interpreter,
};

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

pub fn format_register(register: &RegisterId) -> Span<'static> {
    match register {
        RegisterId::Argument(i) => {
            Span::styled(format!("A{}", i + 1), Style::default().fg(Color::Yellow))
        }
        RegisterId::Temporary(i) => {
            Span::styled(format!("X{}", i + 1), Style::default().fg(Color::Cyan))
        }
        RegisterId::Permanent(i) => {
            Span::styled(format!("Y{}", i + 1), Style::default().fg(Color::Green))
        }
    }
}

impl<'a> StatefulWidget for InstructionView<'a> {
    type State = InstructionViewState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let tab_width = 2;
        let line_no_style = ratatui::style::Style::default().fg(Color::LightCyan);
        let style = ratatui::style::Style::default().fg(Color::White);
        let line_no_indicator_style = ratatui::style::Style::default().fg(Color::White);

        let lines = self
            .instructions
            .iter()
            .map(|i| match i {
                Instruction::PutStructure {
                    structure,
                    register,
                } => Line::from(vec![
                    Span::raw("put_structure "),
                    Span::styled(
                        self.descriptors.get(*structure).pretty_name(),
                        Style::default().fg(Color::LightRed),
                    ),
                    Span::raw(", "),
                    format_register(register),
                ]),
                Instruction::PutVariable {
                    argument_register,
                    variable_register,
                } => Line::from(vec![
                    Span::raw("put_variable "),
                    format_register(variable_register),
                    Span::raw(", "),
                    format_register(argument_register),
                ]),
                Instruction::PutValue {
                    argument_register,
                    value_register,
                } => Line::from(vec![
                    Span::raw("put_value "),
                    format_register(value_register),
                    Span::raw(", "),
                    format_register(argument_register),
                ]),
                Instruction::PutConstant { register, constant } => Line::from(vec![
                    Span::raw("put_constant "),
                    Span::styled(
                        self.descriptors.get(*constant).pretty_name(),
                        Style::default().fg(Color::LightRed),
                    ),
                    Span::raw(", "),
                    format_register(register),
                ]),

                Instruction::SetVariable { register } => {
                    Line::from(vec![Span::raw("set_variable "), format_register(register)])
                }
                Instruction::SetValue { register } => {
                    Line::from(vec![Span::raw("set_value "), format_register(register)])
                }
                Instruction::SetConstant { constant } => Line::from(vec![
                    Span::raw("set_constant "),
                    Span::styled(
                        self.descriptors.get(*constant).pretty_name(),
                        Style::default().fg(Color::LightRed),
                    ),
                ]),
                Instruction::DebugComment { message } => Line::from(vec![Span::styled(
                    format!(";; {}", message),
                    Style::default().fg(Color::DarkGray),
                )]),
                Instruction::GetStructure {
                    structure,
                    register,
                } => Line::from(vec![
                    Span::raw("get_structure "),
                    Span::styled(
                        self.descriptors.get(*structure).pretty_name(),
                        Style::default().fg(Color::LightRed),
                    ),
                    Span::raw(", "),
                    format_register(register),
                ]),
                Instruction::GetValue {
                    argument_register,
                    value_register,
                } => Line::from(vec![
                    Span::raw("get_value "),
                    format_register(value_register),
                    Span::raw(", "),
                    format_register(argument_register),
                ]),
                Instruction::GetVariable {
                    argument_register,
                    variable_register,
                } => Line::from(vec![
                    Span::raw("get_variable "),
                    format_register(variable_register),
                    Span::raw(", "),
                    format_register(argument_register),
                ]),
                Instruction::GetConstant { constant, register } => Line::from(vec![
                    Span::raw("get_constant "),
                    Span::styled(
                        self.descriptors.get(*constant).pretty_name(),
                        Style::default().fg(Color::LightRed),
                    ),
                    Span::raw(", "),
                    format_register(register),
                ]),

                Instruction::UnifyVariable { register } => Line::from(vec![
                    Span::raw("unify_variable "),
                    format_register(register),
                ]),
                Instruction::UnifyValue { register } => {
                    Line::from(vec![Span::raw("unify_value "), format_register(register)])
                }
                Instruction::UnifyConstant { constant } => Line::from(vec![
                    Span::raw("unify_constant "),
                    Span::styled(
                        self.descriptors.get(*constant).pretty_name(),
                        Style::default().fg(Color::LightRed),
                    ),
                ]),
                Instruction::Proceed => Line::from(vec![Span::raw("proceed")]),
                Instruction::Call { address, .. } => Line::from(vec![
                    Span::raw("call "),
                    match &self.instructions[*address] {
                        Instruction::DebugComment { message } => {
                            Span::styled(message.to_string(), Style::default().fg(Color::LightRed))
                        }
                        _ => Span::raw((address + 1).to_string()),
                    },
                ]),
                Instruction::TryMeElse { else_address } => Line::from(vec![
                    Span::raw("try_me_else "),
                    match &self.instructions[*else_address] {
                        Instruction::DebugComment { message } => {
                            Span::styled(message.to_string(), Style::default().fg(Color::LightRed))
                        }
                        _ => Span::raw((else_address + 1).to_string()),
                    },
                ]),
                Instruction::RetryMeElse { else_address } => Line::from(vec![
                    Span::raw("retry_me_else "),
                    match &self.instructions[*else_address] {
                        Instruction::DebugComment { message } => {
                            Span::styled(message.to_string(), Style::default().fg(Color::LightRed))
                        }
                        _ => Span::raw((else_address + 1).to_string()),
                    },
                ]),
                Instruction::TrustMe => Line::from(vec![Span::raw("trust_me")]),
                Instruction::NoOp => Line::from(vec![Span::raw("no_op")]),
                Instruction::Allocate { variables } => Line::from(vec![
                    Span::raw("allocate "),
                    Span::styled(
                        format!("{}", variables),
                        Style::default().fg(Color::Magenta),
                    ),
                ]),
                Instruction::Deallocate => Line::from(vec![Span::raw("deallocate")]),
            })
            .collect::<Vec<_>>();

        // ----------- Pre-compute some invariants ---------
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
                if self.interpreter.instruction_index == idx as usize {
                    style.bg(Color::LightGreen).fg(Color::Black)
                } else {
                    style
                }
            };

            let y = area.y + row as u16;
            let indicator = if self.interpreter.instruction_index == idx as usize {
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

            let line = &lines[idx as usize];
            // Apply active style to all spans in the line
            let styled_line = if self.interpreter.instruction_index == idx as usize {
                Line::from(
                    line.spans
                        .iter()
                        .map(|span| {
                            Span::styled(
                                span.content.clone(),
                                span.style.bg(Color::LightGreen).fg(Color::Black),
                            )
                        })
                        .collect::<Vec<_>>(),
                )
            } else {
                line.clone()
            };

            // Render the styled line with proper width handling
            let line_width = styled_line.width();
            if line_width <= text_cols as usize {
                buf.set_line(area.x + gutter as u16 + 3, y, &styled_line, text_cols);
            } else {
                // Truncate if too long - we'll need to implement proper truncation for spans
                let mut truncated_spans = Vec::new();
                let mut used_width = 0;
                for span in &styled_line.spans {
                    let span_width = UnicodeWidthStr::width(span.content.as_ref());
                    if used_width + span_width <= text_cols as usize {
                        truncated_spans.push(span.clone());
                        used_width += span_width;
                    } else if used_width < text_cols as usize {
                        // Partial span
                        let remaining = text_cols as usize - used_width;
                        let mut truncated_content = String::new();
                        let mut current_width = 0;
                        for g in span.content.graphemes(true) {
                            let g_width = UnicodeWidthStr::width(g);
                            if current_width + g_width <= remaining {
                                truncated_content.push_str(g);
                                current_width += g_width;
                            } else {
                                break;
                            }
                        }
                        truncated_spans.push(Span::styled(truncated_content, span.style));
                        break;
                    } else {
                        break;
                    }
                }
                let truncated_line = Line::from(truncated_spans);
                buf.set_line(area.x + gutter as u16 + 3, y, &truncated_line, text_cols);
            }
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
