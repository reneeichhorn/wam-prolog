use prolog_wan::ui::App;

fn main() {
    color_eyre::install().unwrap();

    let input_query = "p(f(X, g(Y), c), g(Z), h).";

    let input_program = vec!["q(X, Y).", "p(f(f(a), g(b), X), g(b), h) :- q(X, Y)."];

    let mut ui_app = App::new(input_query.to_string(), &input_program).unwrap();

    let mut terminal = ratatui::init();
    ui_app.run(&mut terminal).unwrap();

    ratatui::restore();
}
