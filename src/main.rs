use prolog_wan::ui::App;

fn main() {
    color_eyre::install().unwrap();

    let input_program = r###"
f(s).
z(Z).
g(G).
h(x).
h(y).
h(z).
p(f(f(X)), h(W), Y) :- g(W), h(W), f(X).
p(X, Y, Z) :- h(Y), z(Z).
    "###
    .trim()
    .split("\n")
    .collect::<Vec<_>>();

    let input_query = "p(X, Y, Z).";

    let mut ui_app = App::new(input_query.to_string(), &input_program).unwrap();

    let mut terminal = ratatui::init();
    ui_app.run(&mut terminal).unwrap();

    ratatui::restore();
}
