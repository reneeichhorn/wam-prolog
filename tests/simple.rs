use prolog_wan::{
    compiler::{Compiler, NamedReferenceStore},
    end_user_executor::{EndUserExecutor, EndUserResult},
    parsing::parse,
};

fn helper_execute(program: &str, query: &str) -> EndUserResult {
    let program = parse(program).unwrap();
    let query = parse(query).unwrap();

    let mut reference_store = NamedReferenceStore::default();
    let mut compiler = Compiler::default();
    let program = compiler.compile_program(&program, &mut reference_store);
    let query = compiler.compile_query(&query, &mut reference_store);

    let mut executor = EndUserExecutor::new();
    executor.set_program(program);
    executor.set_query(query);
    executor.execute(&mut reference_store).unwrap()
}

#[test]
fn test_execute() {
    assert_eq!(helper_execute("p(Z, Z)", "p(Z, Z)").success, true);
    assert_eq!(helper_execute("p(Z, Z)", "p(Z, z)").success, true);
    assert_eq!(helper_execute("p(Z, Z)", "p(Z, w)").success, true);
    assert_eq!(helper_execute("p(Z, Z)", "p(z, w)").success, false);
    assert_eq!(helper_execute("p(Z, Z)", "p(z, z)").success, true);
}
