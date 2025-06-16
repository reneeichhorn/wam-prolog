use prolog_wan::{
    compiler::{ProgramTarget, QueryTarget, compile},
    descriptor::DescriptorAllocator,
    end_user_executor::{EndUserExecutor, EndUserResult},
    parsing::parse,
};

fn helper_execute(program: &str, query: &str) -> EndUserResult {
    let program = parse(program).unwrap();
    let query = parse(query).unwrap();

    let mut descriptors = DescriptorAllocator::default();
    let program = compile::<ProgramTarget>(&program, &mut descriptors);
    let query = compile::<QueryTarget>(&query, &mut descriptors);

    let mut executor = EndUserExecutor::new();
    executor.set_program(program);
    executor.set_query(query);
    executor.execute(&mut descriptors).unwrap()
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
