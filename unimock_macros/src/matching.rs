use crate::doc::SynDoc;

use quote::quote;
use syn::spanned::Spanned;

pub struct MatchingInput {
    patterns: Vec<syn::PatTuple>,
    guard: Option<(syn::token::If, syn::Expr)>,
}

impl syn::parse::Parse for MatchingInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(syn::token::Or) {
            let _: syn::token::Or = input.parse()?;
        }

        let mut patterns = Vec::new();
        let mut guard = None;

        if input.peek(syn::token::Paren) {
            patterns.push(syn_pat_to_pattern(input.parse()?)?);
            while input.peek(syn::token::Or) {
                let _: syn::token::Or = input.parse()?;
                patterns.push(syn_pat_to_pattern(input.parse()?)?);
            }
        } else if !input.is_empty() {
            let mut elems = syn::punctuated::Punctuated::<syn::Pat, syn::token::Comma>::new();
            elems.push(parse_pat_potential_or(input)?);
            while input.peek(syn::token::Comma) {
                let _: syn::token::Comma = input.parse()?;
                elems.push(parse_pat_potential_or(input)?);
            }
            patterns.push(syn::PatTuple {
                attrs: vec![],
                paren_token: syn::token::Paren(proc_macro2::Span::call_site()),
                elems,
            });
        }

        if input.peek(syn::token::If) {
            let if_token: syn::token::If = input.parse()?;
            let expr: syn::Expr = input.parse()?;

            guard = Some((if_token, expr));
        }

        Ok(Self { patterns, guard })
    }
}

fn parse_pat_potential_or(input: syn::parse::ParseStream) -> syn::Result<syn::Pat> {
    let pat: syn::Pat = input.parse()?;
    if input.peek(syn::token::Or) {
        let mut cases: syn::punctuated::Punctuated<syn::Pat, syn::token::Or> =
            syn::punctuated::Punctuated::new();
        cases.push(pat);

        while input.peek(syn::token::Or) {
            cases.push_punct(input.parse()?);
            cases.push(input.parse()?);
        }

        Ok(syn::Pat::Or(syn::PatOr {
            attrs: vec![],
            leading_vert: None,
            cases,
        }))
    } else {
        Ok(pat)
    }
}

fn syn_pat_to_pattern(pat: syn::Pat) -> syn::Result<syn::PatTuple> {
    match pat {
        syn::Pat::Tuple(pat_tuple) => Ok(pat_tuple),
        _ => Err(syn::Error::new(pat.span(), "Unsupported pattern")),
    }
}

struct Arg {
    ident: syn::Ident,
    kind: ArgKind,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ArgKind {
    Unknown,
    LitStr,
}

pub fn generate(input: MatchingInput) -> proc_macro2::TokenStream {
    if input.patterns.is_empty() {
        return quote! {
            &|_m| {
                _m.func(|()| true);
                _m.pat_debug("()", file!(), line!());
            }
        };
    }

    let args = analyze_args(&input.patterns);
    let pattern_debug_lit_str = generate_pat_debug(&input);
    let tuple_pats = input
        .patterns
        .into_iter()
        .map(|pattern| match pattern.elems.len() {
            1 => {
                let unwrapped = pattern.elems.into_iter().next().unwrap();
                quote! { #unwrapped }
            }
            _ => {
                quote! { #pattern }
            }
        });
    let guard = if let Some((if_token, expr)) = input.guard {
        Some(quote! { #if_token #expr })
    } else {
        None
    };

    let arg_pat = concat_args_parenthesized(&args, |arg| {
        let ident = &arg.ident;
        quote! { #ident }
    });
    let arg_expr = concat_args_parenthesized(&args, |arg| {
        let ident = &arg.ident;
        match arg.kind {
            ArgKind::LitStr => quote! { ::unimock::macro_api::as_str_ref(#ident) },
            _ => quote! { #ident },
        }
    });

    quote! {
        &|_m| {
            _m.func(
                |#arg_pat| match #arg_expr {
                    #(#tuple_pats)|* #guard => true,
                    _ => false
                },
            );
            _m.pat_debug(#pattern_debug_lit_str, file!(), line!());
        }
    }
}

fn concat_args_parenthesized<F>(args: &Vec<Arg>, f: F) -> proc_macro2::TokenStream
where
    F: Fn(&Arg) -> proc_macro2::TokenStream,
{
    let streams = args.iter().map(f);

    if args.len() == 1 {
        quote! { #(#streams),* }
    } else {
        quote! { (#(#streams),*) }
    }
}

/// Analyze the arguments to the function by looking at the pattern(s).
fn analyze_args(patterns: &Vec<syn::PatTuple>) -> Vec<Arg> {
    let mut args = vec![];

    if patterns.is_empty() {
        return args;
    }

    let first_pat = patterns.first().unwrap();
    let arg_count = first_pat.elems.len();

    for i in 0..arg_count {
        args.push(Arg {
            ident: quote::format_ident!("a{}", i),
            kind: guess_arg_kind(i, patterns),
        });
    }

    args
}

fn guess_arg_kind(index: usize, patterns: &[syn::PatTuple]) -> ArgKind {
    fn guess_from_pattern(index: usize, pattern: &syn::PatTuple) -> ArgKind {
        if index >= pattern.elems.len() {
            return ArgKind::Unknown;
        }
        match &pattern.elems[index] {
            syn::Pat::Lit(lit) => match lit.expr.as_ref() {
                syn::Expr::Lit(expr_lit) => match &expr_lit.lit {
                    syn::Lit::Str(_) => ArgKind::LitStr,
                    _ => ArgKind::Unknown,
                },
                _ => ArgKind::Unknown,
            },
            _ => ArgKind::Unknown,
        }
    }

    let mut result_kind = ArgKind::Unknown;
    let mut conflicting = false;

    for pattern in patterns.iter() {
        let next_kind = guess_from_pattern(index, pattern);
        match (result_kind, next_kind) {
            (ArgKind::Unknown, next) => {
                result_kind = next;
            }
            (_, ArgKind::Unknown) => {}
            (prev, next) if prev == next => {}
            (_, _) => {
                conflicting = true;
            }
        }
    }

    if conflicting {
        ArgKind::Unknown
    } else {
        result_kind
    }
}

fn generate_pat_debug(input: &MatchingInput) -> syn::LitStr {
    let mut debug = String::new();

    if input.patterns.is_empty() {
        debug.push_str("()");
    } else {
        let len = input.patterns.len();

        for (index, pattern) in input.patterns.iter().enumerate() {
            pattern.doc(&mut debug);
            if index < len - 1 {
                debug.push_str(" | ");
            }
        }
    }

    if input.guard.is_some() {
        debug.push_str(" if {guard}");
    }

    syn::LitStr::new(debug.as_str(), proc_macro2::Span::call_site())
}

#[cfg(test)]
mod tests {
    use super::*;

    use syn::parse_quote;

    fn test_doc(input: MatchingInput) -> String {
        generate_pat_debug(&input).value()
    }

    #[test]
    fn test_parsing_by_doc_output() {
        assert_eq!("()", test_doc(parse_quote!()));
        assert_eq!("()", test_doc(parse_quote!(())));

        assert_eq!("(1)", test_doc(parse_quote!(1)));
        assert_eq!("(1)", test_doc(parse_quote!((1))));
        assert_eq!("((1))", test_doc(parse_quote!(((1,)))));

        assert_eq!("(1, 2)", test_doc(parse_quote!(1, 2)));
        assert_eq!("(1, 2)", test_doc(parse_quote!((1, 2))));
        assert_eq!("((1, 2))", test_doc(parse_quote!(((1, 2)))));

        assert_eq!("(1 | 2)", test_doc(parse_quote!(1 | 2)));
        assert_eq!("(1 | 2)", test_doc(parse_quote!((1 | 2))));
        assert_eq!("(1 | 2) | (3)", test_doc(parse_quote!((1 | 2) | (3))));
        assert_eq!("(1) | (2)", test_doc(parse_quote!((1) | (2))));
        assert_eq!("((1) | (2))", test_doc(parse_quote!(((1) | (2)))));
        assert_eq!("(1 | 2, 3 | 4)", test_doc(parse_quote!(1 | 2, 3 | 4)));
        assert_eq!(
            "(1 | 2, 3 | 4) | (4 | 5, 6 | 7)",
            test_doc(parse_quote!((1 | 2, 3 | 4) | (4 | 5, 6 | 7)))
        );

        assert_eq!("(1) if {guard}", test_doc(parse_quote!(1 if expr())));
        assert_eq!("(1) if {guard}", test_doc(parse_quote!((1) if expr())));
    }
}
