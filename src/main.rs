use prolog_wan::ui::App;

fn main() {
    color_eyre::install().unwrap();

    let input_program = r###"
p(f(X), h(Y, f(a)), Y).
    "###
    .trim()
    .split("\n")
    .collect::<Vec<_>>();

    let input_query = "p(Z, h(Z, W), f(W)).";

    let mut ui_app = App::new(input_query.to_string(), &input_program).unwrap();

    let mut terminal = ratatui::init();
    ui_app.run(&mut terminal).unwrap();

    ratatui::restore();
}
