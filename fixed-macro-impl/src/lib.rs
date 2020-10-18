mod dispatch;

use std::str::FromStr;

use proc_macro::TokenStream;
use proc_macro_error::{abort, proc_macro_error};

use quote::{format_ident, quote};
use syn::{
    parse::{self, Parse, ParseStream},
    parse_macro_input, Ident, Lit, Token,
};

struct FixedType {
    signed: bool,
    int_bits: u8,
    frac_bits: u8,
}

impl FixedType {
    pub fn type_ident(&self) -> Ident {
        let int_name = if self.signed { 'I' } else { 'U' };
        format_ident!("{}{}F{}", int_name, self.int_bits, self.frac_bits)
    }

    pub fn from_ident(ident: &Ident) -> Result<Self, &'static str> {
        fn parse_size(s: &str) -> Option<u8> {
            if s.chars().next()?.is_digit(10) {
                let num = u8::from_str(s).ok()?;
                if num <= 128 {
                    return Some(num);
                }
            }
            None
        }
        let name = ident.to_string();
        let signed = match name.chars().next().ok_or("?")? {
            'I' => true,
            'U' => false,
            _ => return Err("type name must start with `I` or `U`"),
        };
        let f_pos = name.find('F').ok_or("type name must contain `F`")?;
        let int_bits = parse_size(&name[1..f_pos]).ok_or("invalid number of integer bits")?;
        let frac_bits =
            parse_size(&name[f_pos + 1..]).ok_or("invalid number of fractional bits")?;
        if ![8, 16, 32, 64, 128].contains(&((int_bits as u16) + (frac_bits as u16))) {
            return Err("total number of bits must be 8, 16, 32, 64 or 128");
        }
        Ok(FixedType {
            signed,
            int_bits,
            frac_bits,
        })
    }
}

fn normalize_float(float: &str) -> Result<String, &'static str> {
    let mut float = float.to_owned();
    let mut exp = match float.find('e') {
        Some(idx) => {
            let exp = i8::from_str(&float[idx + 1..]).or(Err("exponent out of range"))?;
            float.truncate(idx);
            exp
        }
        _ => 0,
    };
    let idx = float.find('.').unwrap_or_else(|| float.len());
    let mut int = float[..idx].to_owned();
    let mut frac = float[idx + 1..].to_owned();
    while exp > 0 {
        if !frac.is_empty() {
            int.push(frac.remove(0));
        } else {
            int.push('0');
        }
        exp -= 1;
    }
    while exp < 0 {
        if !int.is_empty() {
            frac.insert(0, int.remove(int.len() - 1));
        } else {
            frac.insert(0, '0');
        }
        exp += 1;
    }
    Ok(format!("{}.{}", int, frac))
}

fn parse_fixed_literal(lit: &Lit) -> Result<String, &'static str> {
    match *lit {
        Lit::Int(ref int) => {
            if !int.suffix().is_empty() {
                Err("unexpected suffix")
            } else {
                Ok(int.base10_digits().into())
            }
        }
        Lit::Float(ref float) => {
            if !float.suffix().is_empty() {
                Err("unexpected suffix")
            } else {
                let float = normalize_float(float.base10_digits())?;
                Ok(float)
            }
        }
        _ => Err("expected int or float"),
    }
}

struct FixedInput {
    ident: Ident,
    neg: bool,
    lit: Lit,
}

impl Parse for FixedInput {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        let mut neg = false;
        if input.peek(Token![-]) {
            neg = true;
            let _ = input.parse::<Token![-]>();
        } else if input.peek(Token![+]) {
            let _ = input.parse::<Token![+]>();
        }
        let lit = input.parse()?;
        input.parse::<Token![:]>()?;
        let ident = input.parse()?;
        Ok(Self { ident, neg, lit })
    }
}

#[proc_macro]
#[proc_macro_error]
pub fn fixed(input: TokenStream) -> TokenStream {
    let FixedInput { ident, neg, lit } = parse_macro_input!(input as FixedInput);
    let ty = match FixedType::from_ident(&ident) {
        Ok(ty) => ty,
        Err(err) => abort!(ident.span(), "invalid fixed type: {}", err),
    };
    if !ty.signed && neg {
        abort!(lit.span(), "negative value for an unsigned fixed type");
    }
    let literal = match parse_fixed_literal(&lit) {
        Ok(lit) => format!("{}{}", (if neg { "-" } else { "" }), lit),
        Err(err) => abort!(lit.span(), "invalid fixed value: {}", err),
    };
    let bits = match dispatch::fixed_to_literal(ty.int_bits, ty.frac_bits, ty.signed, &literal) {
        Ok(bits) => bits,
        Err(err) => abort!(lit.span(), "invalid fixed value: {}", err),
    };
    let type_ident = ty.type_ident();
    let code = quote! { ::fixed::types::#type_ident::from_bits(#bits) };
    code.into()
}
