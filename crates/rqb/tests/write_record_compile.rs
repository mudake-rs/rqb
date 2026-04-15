#[test]
fn write_record_compile_failures() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/write_record/*.rs");
}
