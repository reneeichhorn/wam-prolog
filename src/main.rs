use prolog_wan::ui::App;

fn main() {
    color_eyre::install().unwrap();

    let input_query = "p(z, h(Z, w), f(Z))";
    let input_program = "p(Z, h(Z, W), f(W))";

    let mut ui_app = App::new(input_query.to_string(), input_program.to_string()).unwrap();

    let mut terminal = ratatui::init();
    ui_app.run(&mut terminal).unwrap();

    ratatui::restore();
}
