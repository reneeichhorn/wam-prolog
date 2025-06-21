use prolog_wan::{
    compiler::{Compiler, ProgramTarget, QueryTarget},
    descriptor::DescriptorAllocator,
    interpreter::{ExecutionState, Interpreter},
    parsing::parse,
};

struct Output {
    success: bool,
}

fn helper_execute(program: &str, query: &str) -> Output {
    let program = parse(program).unwrap();
    let query = parse(query).unwrap();

    let mut compiler = Compiler::new();
    let fact_artifact = compiler.add_fact(&program);
    let query_artifact = compiler.compile(&query);

    let mut interpreter = Interpreter::new(
        query_artifact.instructions,
        fact_artifact
            .registers
            .len()
            .max(query_artifact.registers.len()),
        compiler.descriptor_allocator.descriptors.clone(),
    );
    while interpreter.step() {}
    Output {
        success: interpreter.execution_state == ExecutionState::Normal,
    }
}

#[test]
fn test_execute() {
    assert_eq!(helper_execute("p(Z, Z)", "p(Z, Z)").success, true);
    assert_eq!(helper_execute("p(Z, Z)", "p(Z, z)").success, true);
    assert_eq!(helper_execute("p(Z, Z)", "p(Z, w)").success, true);
    assert_eq!(helper_execute("p(Z, Z)", "p(z, w)").success, false);
    assert_eq!(helper_execute("p(Z, Z)", "p(z, z)").success, true);

    assert_eq!(
        helper_execute("p(Z, h(Z, W), f(W))", "p(z, h(z, z), f(w))").success,
        false
    );
    assert_eq!(
        helper_execute("p(Z, h(Z, W), f(W))", "p(z, h(z, w), f(w))").success,
        true
    );
    assert_eq!(
        helper_execute("p(Z, h(Z, W), f(W))", "p(Z, h(z, W), f(w))").success,
        true
    );
    assert_eq!(
        helper_execute("p(Z, h(Z, W), f(W))", "p(z, h(Z, w), f(w))").success,
        true
    );
    assert_eq!(
        helper_execute("p(Z, h(Z, W), f(W))", "p(z, h(Z, w), f(Z))").success,
        false
    );

    assert_eq!(
        helper_execute("clouds(are, nice)", "clouds(Z, Z)").success,
        false
    );
    assert_eq!(
        helper_execute("clouds(are, nice)", "clouds(Z, W)").success,
        true
    );
    assert_eq!(
        helper_execute("clouds(are, nice)", "clouds(are, W)").success,
        true
    );
    assert_eq!(
        helper_execute("clouds(are, nice)", "clouds(W, nice)").success,
        true
    );
    assert_eq!(
        helper_execute("clouds(are, nice)", "clouds(nice, are)").success,
        false
    );
}
