use prolog_wan::{
    compiler::Compiler,
    descriptor::DescriptorAllocator,
    interpreter::{ExecutionState, InspectionResult, InspectionView, Interpreter},
    parsing::parse,
};

struct Output {
    success: bool,
    output: String,
}

fn helper_inspection_format(view: &InspectionView, descriptors: &DescriptorAllocator) -> String {
    match view {
        InspectionView::Undefined => "undefined".to_string(),
        InspectionView::UnboundVariable { index } => format!("_{}", index),
        InspectionView::Constant { descriptor_id } => {
            let constant_name = descriptors.get(*descriptor_id).name.clone();
            format!("{}", constant_name)
        }
        InspectionView::Structure {
            descriptor_id,
            arguments,
        } => {
            let inner_name = descriptors.get(*descriptor_id).name.clone();
            format!(
                "{}{}",
                inner_name,
                if arguments.is_empty() {
                    "".to_string()
                } else {
                    format!(
                        "({})",
                        arguments
                            .iter()
                            .map(|arg| helper_inspection_format(arg, descriptors))
                            .collect::<Vec<String>>()
                            .join(", ")
                    )
                }
            )
        }
    }
}

fn helper_inspection(result: InspectionResult, descriptors: &DescriptorAllocator) -> String {
    let mut output = String::new();

    for (index, (id, variable)) in result.variables.iter().enumerate() {
        let name = descriptors.get(*id).name.clone();
        output += &format!(
            "{} = {}{}",
            name,
            helper_inspection_format(variable, descriptors),
            if index == result.variables.len() - 1 {
                ""
            } else {
                ", "
            }
        );
    }

    output
}

fn helper_execute_multi(program: &[&str], query: &str) -> Output {
    let query = parse(query).unwrap();

    let mut compiler = Compiler::new();
    for program in program {
        let program = parse(program).unwrap();
        compiler.add_program(&program);
    }
    let artifact = compiler.compile(&query);

    let mut interpreter = Interpreter::new(
        artifact.instructions,
        artifact.start_instruction_index,
        artifact.max_registers,
        compiler.descriptor_allocator.descriptors.clone(),
        &artifact.inspection_variables,
    );

    let mut suceeded_once = false;
    let mut output = String::new();
    let mut back_track_count = 0;

    loop {
        while interpreter.step() {}

        if interpreter.execution_state == ExecutionState::Normal {
            suceeded_once = true;

            if back_track_count > 0 {
                output.push_str("\n");
            }

            output.push_str(&helper_inspection(
                interpreter.inspect(),
                &compiler.descriptor_allocator,
            ));
        }

        if !interpreter.try_backtrack() {
            break;
        }

        back_track_count += 1;
    }
    Output {
        success: suceeded_once,
        output,
    }
}

fn helper_execute(program: &str, query: &str) -> Output {
    helper_execute_multi(&[program], query)
}

#[test]
fn test_execute() {
    assert_eq!(helper_execute("p(Z, Z).", "p(Z, Z).").success, true);
    assert_eq!(helper_execute("p(Z, Z).", "p(Z, z).").success, true);
    assert_eq!(helper_execute("p(Z, Z).", "p(Z, w).").success, true);
    assert_eq!(helper_execute("p(Z, Z).", "p(z, w).").success, false);
    assert_eq!(helper_execute("p(Z, Z).", "p(z, z).").success, true);

    assert_eq!(
        helper_execute("p(Z, h(Z, W), f(W)).", "p(z, h(z, z), f(w)).").success,
        false
    );
    assert_eq!(
        helper_execute("p(Z, h(Z, W), f(W)).", "p(z, h(z, w), f(w)).").success,
        true
    );
    assert_eq!(
        helper_execute("p(Z, h(Z, W), f(W)).", "p(Z, h(z, W), f(w)).").success,
        true
    );
    assert_eq!(
        helper_execute("p(Z, h(Z, W), f(W)).", "p(z, h(Z, w), f(w)).").success,
        true
    );
    assert_eq!(
        helper_execute("p(Z, h(Z, W), f(W)).", "p(z, h(Z, w), f(Z)).").success,
        false
    );
    assert_eq!(
        helper_execute("p(Z, h(Z, W), f(W)).", "p(z, h(z, W), f(w)).").output,
        "W = w"
    );
    assert_eq!(
        helper_execute("p(Z, h(Z, W), f(W)).", "p(Z, h(Z, w), f(Z)).").output,
        "Z = w"
    );

    assert_eq!(
        helper_execute("p(f(X), h(Y, f(a)), Y).", "p(Z, h(Z, W), f(W)).").output,
        "Z = f(f(a)), W = f(a)"
    );

    assert_eq!(
        helper_execute("clouds(are, nice).", "clouds(Z, Z).").success,
        false
    );
    assert_eq!(
        helper_execute("clouds(are, nice).", "clouds(Z, W).").success,
        true
    );
    assert_eq!(
        helper_execute("clouds(are, nice).", "clouds(are, W).").success,
        true
    );
    assert_eq!(
        helper_execute("clouds(are, nice).", "clouds(W, nice).").success,
        true
    );
    assert_eq!(
        helper_execute("clouds(are, nice).", "clouds(nice, are).").success,
        false
    );
}

#[test]
fn test_rules() {
    assert_eq!(
        helper_execute_multi(
            &["q(q, s).", "r(s, t).", "p(X, Y) :- q(X, Z), r(Z, Y)."],
            "p(X, Y)."
        )
        .output,
        "X = q, Y = t"
    );
    assert_eq!(
        helper_execute_multi(
            &["q(q, s).", "r(s, t).", "p(X, Y) :- q(X, Z), r(Z, Y)."],
            "p(q, t)."
        )
        .success,
        true
    );
    assert_eq!(
        helper_execute_multi(
            &["q(q, s).", "r(s, t).", "p(X, Y) :- q(X, Z), r(Z, Y)."],
            "p(t, q)."
        )
        .success,
        false
    );
    assert_eq!(
        helper_execute_multi(
            &["q(q, s).", "r(s, t).", "p(X, Y) :- q(X, Z), r(Z, Y)."],
            "p(q, T)."
        )
        .output,
        "T = t"
    );
    assert_eq!(
        helper_execute_multi(
            &["q(q, s).", "r(s, t).", "p(X, Y) :- q(X, Z), r(Z, Y)."],
            "p(Q, t)."
        )
        .output,
        "Q = q"
    );
    assert_eq!(
        helper_execute_multi(
            &[
                "q(f(f(X)), r).",
                "r(s, t).",
                "p(X, Y) :- q(f(f(X)), R), r(S, T)."
            ],
            "p(X, Y)."
        )
        .output,
        "X = _0, Y = _1"
    );
    assert_eq!(
        helper_execute_multi(
            &["q(X, Y).", "p(f(f(a), g(b), X), g(b), h) :- q(X, Y)."],
            "p(f(X, Y, Z), g(b), h)."
        )
        .output,
        "X = f(a), Y = g(b), Z = _3"
    );
    assert_eq!(
        helper_execute_multi(
            &["q(X, Y).", "p(f(f(a), g(b), X), g(b), h) :- q(X, Y)."],
            "p(f(X, g(Y), c), g(Z), X)."
        )
        .success,
        false
    );
    assert_eq!(
        helper_execute_multi(
            &["q(X, Y).", "p(f(f(a), g(b), X), g(b), h) :- q(X, Y)."],
            "p(f(X, g(Y), c), g(Z), h)."
        )
        .output,
        "X = f(a), Y = b, Z = b"
    );
    assert_eq!(
        helper_execute_multi(
            &["q(X, Y).", "p(f(f(a), g(b), X), g(b), h) :- q(X, Y)."],
            "p(Z, Y, X)."
        )
        .output,
        "Z = f(f(a), g(b), _7), Y = g(b), X = h"
    );
    assert_eq!(
        helper_execute_multi(
            &["q(X, Y).", "p(f(f(a), g(b), X), g(b), h) :- q(X, Y)."],
            "p(f(X, Y, Z), Y, h)."
        )
        .output,
        "X = f(a), Y = g(b), Z = _3"
    );
}

#[test]
fn test_backtracking() {
    assert_eq!(
        helper_execute_multi(&["h(x).", "h(y).", "h(z)."], "h(X).").output,
        "X = x\nX = y\nX = z"
    );

    let test = r###"
loves(vincent, mia).
loves(marcellus, mia).
loves(pumpkin, honeybunny).
loves(honeybunny, pumpkin).
jealous(X, Y) :- loves(X, Z), loves(Y, Z).
    "###
    .trim()
    .split("\n")
    .collect::<Vec<_>>();
    assert_eq!(
        helper_execute_multi(&test, "loves(X, mia).").output,
        "X = vincent\nX = marcellus"
    );

    assert_eq!(
        helper_execute_multi(&test, "jealous(X, Y).").output,
        "X = vincent, Y = vincent\nX = vincent, Y = marcellus\nX = marcellus, Y = vincent\nX = marcellus, Y = marcellus\nX = pumpkin, Y = pumpkin\nX = honeybunny, Y = honeybunny"
    );
}

/*
#[test]
fn test_movies() {
    let file = String::from_utf8(include_bytes!("./movies.pl").to_vec()).unwrap();
    let test = file.split("\n").collect::<Vec<_>>();
    assert_eq!(
        helper_execute_multi(&test, "loves(X, Y).").output,
        "X = vincent, Y = mia\nX = marcellus, Y = mia\nX = pumpkin, Y = honeybunny\nX = honeybunny, Y = pumpkin"
    );
}
*/
