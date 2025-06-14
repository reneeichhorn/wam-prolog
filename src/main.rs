use prolog_wan::ui::App;

fn main() {
    color_eyre::install().unwrap();

    let input_query = "p(Z,h(Z,W),f(W))";
    let input_program = "p(f(X),h(Y,f(a)),Y)";

    let mut terminal = ratatui::init();
    let mut ui_app = App::new(input_query.to_string(), input_program.to_string()).unwrap();
    ui_app.run(&mut terminal).unwrap();

    ratatui::restore();
}
