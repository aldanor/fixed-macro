use std::fmt::{Debug, Display};
use std::str::FromStr;

use fixed::types as ft;
use fixed_macro_impl::fixed;

fn check<T>(x: T, repr: &str)
where
    T: FromStr + Display + Debug + PartialEq,
    <T as FromStr>::Err: Debug,
{
    assert_eq!(x, T::from_str(repr).unwrap());
    assert_eq!(format!("{}", x), repr);
}

#[test]
fn test_fixed_macro() {
    let x = fixed!(-0_1.2345_6_78E-3: I24F40);
    check::<ft::I24F40>(x, "-0.0012345678");
    let x = fixed!(0xff: U12F4);
    check::<ft::U12F4>(x, "255");
}
