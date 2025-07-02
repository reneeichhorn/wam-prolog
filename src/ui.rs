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
    compiler::{CompileArtifact, Compiler, ProgramTarget, QueryTarget},
    descriptor::{self, DescriptorAllocator},
    instructions::{DescriptorId, RegisterId},
    interpreter::{Cell, CellAddress, InspectionResult, InspectionView, Interpreter},
    parsing::{AbstractProgram, AbstractTerm, parse},
    ui::{
        instructionview::{InstructionView, InstructionViewState, format_register},
        textview::{TextView, TextViewState},
    },
};

mod instructionview;
mod textview;

#[derive(Debug)]
pub struct App {
    query: String,
    ast: AbstractProgram,
    program: Vec<String>,
    program_ast: Vec<AbstractProgram>,
    instructions: Vec<crate::instructions::Instruction>,
    interpreter: Interpreter,
    compiler: Compiler,
    compile_artifact_query: CompileArtifact,
    counter: u8,
    show_ast: bool,
    show_ast_program: bool,
    ast_state: TextViewState,
    exit: bool,
}

impl App {
    pub fn new(query_str: String, program_str: &[&str]) -> Result<Self> {
        let mut compiler = Compiler::new();

        let program = program_str
            .iter()
            .map(|a| a.to_string())
            .collect::<Vec<String>>();

        let program_ast = program
            .iter()
            .map(|a| parse(a))
            .collect::<Result<Vec<AbstractProgram>>>()?;

        for abstract_program in &program_ast {
            compiler.add_program(abstract_program);
        }

        let query = parse(&query_str)?;
        let compile_artifact_query = compiler.compile(&query);

        let instructions = compile_artifact_query.instructions.clone();

        let interpreter = Interpreter::new(
            instructions.clone(),
            compile_artifact_query.start_instruction_index,
            compile_artifact_query.max_registers,
            compiler.descriptor_allocator.descriptors.clone(),
            &compile_artifact_query.inspection_variables,
        );

        Ok(Self {
            query: query_str,
            ast: query,
            program,
            program_ast,
            interpreter,
            instructions,
            compile_artifact_query,
            compiler,
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
                text: if self.show_ast {
                    &format!("{:#?}", &self.ast)
                } else {
                    &format!("{:#?}", &self.program_ast)
                },
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
            KeyCode::Char('b') => {
                self.interpreter.try_backtrack();
            }
            KeyCode::Char('r') => {
                self.interpreter = Interpreter::new(
                    self.instructions.clone(),
                    self.compile_artifact_query.start_instruction_index,
                    self.interpreter.registers.len(),
                    self.compiler.descriptor_allocator.descriptors.clone(),
                    &self.compile_artifact_query.inspection_variables,
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
            .constraints(vec![
                Constraint::Fill(2),
                Constraint::Fill(2),
                Constraint::Fill(1),
            ])
            .split(main_layout[2]);

        // Instructions view
        let block = Block::bordered()
            .title(Line::from(vec![
                " Instruction View - press ".into(),
                "<Enter>".blue().bold(),
                " to backtrack, press ".into(),
                "<B>".blue().bold(),
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
            descriptors: &self.compiler.descriptor_allocator,
        }
        .render(
            block.inner(main_layout[0]),
            buf,
            &mut InstructionViewState::default(),
        );

        // Rigth side global stack
        let global_stack_text = format_cells(
            &self.interpreter.global_stack,
            &self.compiler.descriptor_allocator,
        );
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
        let registers_text = format_cells(
            &self.interpreter.registers,
            &self.compiler.descriptor_allocator,
        );
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
            "Exec State: {:?}\nMode: {:?}\nS (next subterm): {}\nTrail: {}\nProceed: {}",
            self.interpreter.execution_state,
            self.interpreter.mode,
            self.interpreter.next_sub_term_address,
            self.interpreter
                .trail
                .iter()
                .map(|i| match i {
                    CellAddress::GlobalStack { index } => format!("Stack({})", index),
                    CellAddress::Register { index } => format_register(index).content.to_string(),
                })
                .collect::<Vec<String>>()
                .join(", "),
            self.interpreter.proceed_return_address + 1,
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

        // Rigth right side environment
        let globals_text = format!("{:#?}", self.interpreter.environment_stack.inspect());
        let block = Block::bordered()
            .title(" Environment Stack ")
            .padding(ratatui::widgets::Padding::proportional(1));
        block.clone().render(right_side_layout[0], buf);
        TextView {
            line_no_style: ratatui::style::Style::default().fg(Color::Gray),
            style: ratatui::style::Style::default().fg(Color::White),
            tab_width: 2,
            start_line: 1,
            text: &globals_text,
        }
        .render(
            block.inner(right_side_layout[0]),
            buf,
            &mut TextViewState::default(),
        );

        // Choice point
        let globals_text = format!("{:#?}", self.interpreter.choice_point_stack.inspect());
        let block = Block::bordered()
            .title(" Choice PointStack ")
            .padding(ratatui::widgets::Padding::proportional(1));
        block.clone().render(right_side_layout[1], buf);
        TextView {
            line_no_style: ratatui::style::Style::default().fg(Color::Gray),
            style: ratatui::style::Style::default().fg(Color::White),
            tab_width: 2,
            start_line: 1,
            text: &globals_text,
        }
        .render(
            block.inner(right_side_layout[1]),
            buf,
            &mut TextViewState::default(),
        );

        // Rigth right side solution
        let globals_text = format_inspection(
            self.interpreter.inspect(),
            &self.compiler.descriptor_allocator,
        );
        let block = Block::bordered()
            .title(" Solutions ")
            .padding(ratatui::widgets::Padding::proportional(1));
        block.clone().render(right_side_layout[2], buf);
        TextView {
            line_no_style: ratatui::style::Style::default().fg(Color::Gray),
            style: ratatui::style::Style::default().fg(Color::White),
            tab_width: 2,
            start_line: 1,
            text: &globals_text,
        }
        .render(
            block.inner(right_side_layout[2]),
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

        Paragraph::new(Line::from(self.program.join("\n").clone()))
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

fn format_inspection_view(view: &InspectionView, descriptors: &DescriptorAllocator) -> String {
    match view {
        InspectionView::Undefined => "undefined".to_string(),
        InspectionView::UnboundVariable { index } => format!("_{}", index),
        InspectionView::Structure {
            descriptor_id,
            arguments,
        } => {
            let inner_name = descriptors.get(*descriptor_id).pretty_name();
            let args = arguments
                .iter()
                .map(|i| format_inspection_view(i, descriptors))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}({})", inner_name, args)
        }
    }
}

fn format_inspection(result: InspectionResult, descriptors: &DescriptorAllocator) -> String {
    let mut output = String::new();

    for (id, variable) in result.variables {
        let name = descriptors.get(id).pretty_name();
        let value = format_inspection_view(&variable, descriptors);
        output += &format!("{} = {}\n", name, value);
    }

    output
}

fn format_cells(cells: &[Cell], descriptors: &DescriptorAllocator) -> String {
    let formatted_cells = cells
        .iter()
        .map(|cell| format_cell(cell, descriptors))
        .collect::<Vec<_>>();
    formatted_cells.join("\n")
}

fn format_cell(cell: &Cell, descriptors: &DescriptorAllocator) -> String {
    match cell {
        Cell::Undefined => "undefined".to_string(),
        Cell::Reference(re) => format!("REF({})", re),
        Cell::StructureRef(struc) => format!("STR({})", struc),
        Cell::Structure(struc) => {
            format!("{}", descriptors.get(*struc).pretty_name())
        }
    }
}
