use anyhow::Result;
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::{Color, Stylize},
    symbols::border::THICK,
    text::{Line, Text},
    widgets::{Block, Clear, Paragraph, StatefulWidget, Widget},
};

use crate::{
    compiler::{CompileArtifact, ProgramTarget, QueryTarget, compile},
    descriptor::{self, DescriptorAllocator},
    instructions::DescriptorId,
    interpreter::{Cell, Interpreter},
    parsing::{AbstractTerm, parse},
    ui::{
        instructionview::{InstructionView, InstructionViewState},
        textview::{TextView, TextViewState},
    },
};

mod instructionview;
mod textview;

#[derive(Debug)]
pub struct App {
    query: String,
    ast: AbstractTerm,
    program: String,
    program_ast: AbstractTerm,
    instructions: Vec<crate::instructions::Instruction>,
    descriptors: DescriptorAllocator,
    interpreter: Interpreter,
    compile_artifact_query: CompileArtifact,
    compile_artifact_program: CompileArtifact,
    counter: u8,
    show_ast: bool,
    show_ast_program: bool,
    ast_state: TextViewState,
    exit: bool,
}

impl App {
    pub fn new(query_str: String, program_str: String) -> Result<Self> {
        let query = parse(&query_str)?;
        let program = parse(&program_str)?;

        let mut descriptors = DescriptorAllocator::default();

        let compile_artifact_query = compile::<QueryTarget>(&query, &mut descriptors);
        let compile_artifact_program = compile::<ProgramTarget>(&program, &mut descriptors);

        let mut instructions = compile_artifact_query.instructions.clone();
        instructions.extend(compile_artifact_program.instructions.clone());

        let interpreter = Interpreter::new(
            instructions.clone(),
            compile_artifact_query
                .registers
                .len()
                .max(compile_artifact_program.registers.len()),
            descriptors.descriptors.clone(),
        );

        Ok(Self {
            query: query_str,
            ast: query,
            program: program_str,
            program_ast: program,
            interpreter,
            instructions,
            compile_artifact_program,
            compile_artifact_query,
            descriptors,
            ast_state: TextViewState::default(),
            counter: 0,
            exit: false,
            show_ast: false,
            show_ast_program: false,
        })
    }

    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        frame.render_widget(&mut *self, frame.area());

        let area = frame.area();
        if self.show_ast || self.show_ast_program {
            let block = Block::bordered()
                .title(" Abstract Syntax Tree ")
                .border_set(THICK)
                .padding(ratatui::widgets::Padding::proportional(1));

            let text_view = TextView {
                text: &format!(
                    "{:#?}",
                    if self.show_ast {
                        &self.ast
                    } else {
                        &self.program_ast
                    }
                ),
                tab_width: 2,
                style: ratatui::style::Style::default().fg(Color::White),
                line_no_style: ratatui::style::Style::default().fg(Color::Gray),
                start_line: 1,
            };

            let area = popup_area(area, 60, 60);
            frame.render_widget(Clear, area); //this clears out the background
            frame.render_widget(block.clone(), area);
            frame.render_stateful_widget(text_view, block.inner(area), &mut self.ast_state);
        }
    }

    fn handle_events(&mut self) -> std::io::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char('a') => {
                self.show_ast = !self.show_ast;
                self.show_ast_program = false;
            }
            KeyCode::Char('p') => {
                self.show_ast = false;
                self.show_ast_program = !self.show_ast_program;
            }
            KeyCode::Enter => {
                self.interpreter.step();
            }
            KeyCode::Char('r') => {
                self.interpreter = Interpreter::new(
                    self.instructions.clone(),
                    self.interpreter.registers.len(),
                    self.descriptors.descriptors.clone(),
                );
            }
            KeyCode::Left => self.decrement_counter(),
            KeyCode::Right => self.increment_counter(),
            KeyCode::Char('j') | KeyCode::Down => {
                self.handle_vertical_scroll(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.handle_vertical_scroll(-1);
            }
            _ => {}
        }
    }

    fn handle_vertical_scroll(&mut self, amount: i16) {
        if self.show_ast || self.show_ast_program {
            if amount > 0 {
                self.ast_state.scroll = self.ast_state.scroll.saturating_add(amount as u16);
            } else {
                self.ast_state.scroll = self.ast_state.scroll.saturating_sub((-amount) as u16);
            }
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn increment_counter(&mut self) {
        self.counter += 1;
    }

    fn decrement_counter(&mut self) {
        self.counter -= 1;
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Max(1),
                Constraint::Max(3),
                Constraint::Max(3),
                Constraint::Fill(1),
            ])
            .split(area);

        // Title bar
        let title_bar = Text::from(vec![Line::from(
            format!(
                " Prolog - Warren's abstract machine - compiler & debugger - v{}",
                env!("CARGO_PKG_VERSION")
            )
            .bold()
            .black()
            .bg(Color::LightRed),
        )]);
        Paragraph::new(title_bar)
            .block(Block::default().bg(Color::LightRed))
            .render(layout[0], buf);

        // Main content area
        let main_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Percentage(60),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
            ])
            .split(layout[3]);

        let right_main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Fill(1),
                Constraint::Fill(1),
                Constraint::Fill(1),
            ])
            .split(main_layout[1]);

        let right_side_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Fill(1), Constraint::Fill(1)])
            .split(main_layout[2]);

        // Instructions view
        let block = Block::bordered()
            .title(Line::from(vec![
                " Instruction View - press ".into(),
                "<Enter>".blue().bold(),
                " to step, press ".into(),
                "<R>".blue().bold(),
                " to reset ".into(),
            ]))
            .border_set(THICK)
            .padding(ratatui::widgets::Padding::proportional(1));
        block.clone().render(main_layout[0], buf);
        InstructionView {
            instructions: &self.instructions,
            interpreter: &self.interpreter,
            descriptors: &self.descriptors,
        }
        .render(
            block.inner(main_layout[0]),
            buf,
            &mut InstructionViewState::default(),
        );

        // Rigth side global stack
        let global_stack_text = format_cells(&self.interpreter.global_stack, &self.descriptors);
        let block = Block::bordered()
            .title(" Global Stack ")
            .padding(ratatui::widgets::Padding::proportional(1));
        block.clone().render(right_main_layout[0], buf);
        TextView {
            line_no_style: ratatui::style::Style::default().fg(Color::Gray),
            style: ratatui::style::Style::default().fg(Color::White),
            tab_width: 2,
            start_line: 0,
            text: &global_stack_text,
        }
        .render(
            block.inner(right_main_layout[0]),
            buf,
            &mut TextViewState::default(),
        );

        // Rigth side registers
        let registers_text = format_cells(&self.interpreter.registers, &self.descriptors);
        let block = Block::bordered()
            .title(" Registers ")
            .padding(ratatui::widgets::Padding::proportional(1));
        block.clone().render(right_main_layout[1], buf);
        TextView {
            line_no_style: ratatui::style::Style::default().fg(Color::Gray),
            style: ratatui::style::Style::default().fg(Color::White),
            tab_width: 2,
            start_line: 1,
            text: &registers_text,
        }
        .render(
            block.inner(right_main_layout[1]),
            buf,
            &mut TextViewState::default(),
        );

        // Rigth side globals
        let globals_text = format!(
            "Exec State: {:?}\nMode: {:?}\nS (next subterm): {}",
            self.interpreter.execution_state,
            self.interpreter.mode,
            self.interpreter.next_sub_term_address
        );
        let block = Block::bordered()
            .title(" Globals ")
            .padding(ratatui::widgets::Padding::proportional(1));
        block.clone().render(right_main_layout[2], buf);
        TextView {
            line_no_style: ratatui::style::Style::default().fg(Color::Gray),
            style: ratatui::style::Style::default().fg(Color::White),
            tab_width: 2,
            start_line: 1,
            text: &globals_text,
        }
        .render(
            block.inner(right_main_layout[2]),
            buf,
            &mut TextViewState::default(),
        );

        // Right side register view
        let registers_text =
            format_registers(&self.compile_artifact_program.registers, &self.descriptors);
        let block = Block::bordered()
            .title(" Register Allocation View (Program) ")
            .padding(ratatui::widgets::Padding::proportional(1));
        block.clone().render(right_side_layout[0], buf);
        TextView {
            line_no_style: ratatui::style::Style::default().fg(Color::Gray),
            style: ratatui::style::Style::default().fg(Color::White),
            tab_width: 2,
            start_line: 1,
            text: &registers_text,
        }
        .render(
            block.inner(right_side_layout[0]),
            buf,
            &mut TextViewState::default(),
        );

        let registers_text =
            format_registers(&self.compile_artifact_query.registers, &self.descriptors);
        let block = Block::bordered()
            .title(" Register Allocation View (Query) ")
            .padding(ratatui::widgets::Padding::proportional(1));
        block.clone().render(right_side_layout[1], buf);
        TextView {
            line_no_style: ratatui::style::Style::default().fg(Color::Gray),
            style: ratatui::style::Style::default().fg(Color::White),
            tab_width: 2,
            start_line: 1,
            text: &registers_text,
        }
        .render(
            block.inner(right_side_layout[1]),
            buf,
            &mut TextViewState::default(),
        );

        // Footer with query
        Paragraph::new(Line::from(self.query.clone()))
            .centered()
            .block(Block::bordered().title(Line::from(vec![
                " Query - press ".into(),
                "<A>".blue().bold(),
                " to view AST".into(),
            ])))
            .render(layout[1], buf);

        Paragraph::new(Line::from(self.program.clone()))
            .centered()
            .block(Block::bordered().title(Line::from(vec![
                " Program - press ".into(),
                "<P>".blue().bold(),
                " to view AST".into(),
            ])))
            .render(layout[2], buf);
    }
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

fn format_registers(registers: &[DescriptorId], descriptors: &DescriptorAllocator) -> String {
    let formatted_cells = registers
        .iter()
        .enumerate()
        .map(|(i, descriptor_id)| {
            format!(
                "X{}:  {}",
                i + 1,
                descriptors.get(*descriptor_id).pretty_name()
            )
        })
        .collect::<Vec<_>>();
    formatted_cells.join("\n")
}

fn format_cells(cells: &[Cell], descriptors: &DescriptorAllocator) -> String {
    let formatted_cells = cells
        .iter()
        .map(|cell| match cell {
            Cell::Undefined => "undefined".to_string(),
            Cell::Reference(re) => format!("REF({})", re),
            Cell::StructureRef(struc) => format!("STR({})", struc),
            Cell::Structure(struc) => {
                format!("{}", descriptors.get(*struc).pretty_name())
            }
        })
        .collect::<Vec<_>>();
    formatted_cells.join("\n")
}
