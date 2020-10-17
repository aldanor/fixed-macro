#[test]
fn test_fixed_macro() {
    use fixed_macro::types::I6F2 as Foo;
    println!("{}", Foo!(-1.234));
}
