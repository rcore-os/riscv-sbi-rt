use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use rand::{Rng, SeedableRng};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use syn::{
    parse, parse_macro_input, spanned::Spanned, FnArg, Ident, ItemFn, ReturnType, Type, Visibility,
};

/// Attribute to declare the entry point of the supervisor program
///
/// The specified function will be called by the runtime's init function,
/// after a heap allocator is created. See `init` function in `src/runtime.rs`
/// for details.
///
/// Users should provide specified function with signature `fn main(usize, usize)`.
/// For example, you may consider using `fn main(hartid: usize, dtb: usize)`.
///
/// # Examples
///
/// - Simple entry point
///
/// ```no_run
/// #[opensbi_rt::entry]
/// fn main(hartid: usize, dtb: usize) {
///     println!("Hello, OpenSBI!");
///     println!("hartid={}, dtb={:#x}", hartid, dtb);
/// }
/// ```
// Ref: https://github.com/rust-embedded/riscv-rt/blob/master/macros/src/lib.rs
#[proc_macro_attribute]
pub fn entry(args: TokenStream, input: TokenStream) -> TokenStream {
    let f = parse_macro_input!(input as ItemFn);

    // check the function signature

    let valid_input_types = if f.sig.inputs.len() == 2 {
        let mut param_1_is_usize = false;
        if let FnArg::Typed(pat_type) = &f.sig.inputs[0] {
            if let Type::Path(type_path) = pat_type.ty.as_ref() {
                if type_path.path.segments.len() == 1
                    && type_path.path.segments[0].ident == "usize"
                    && type_path.path.segments[0].arguments.is_empty()
                {
                    param_1_is_usize = true;
                }
            }
        }
        let mut param_2_is_usize = false;
        if let FnArg::Typed(pat_type) = &f.sig.inputs[1] {
            if let Type::Path(type_path) = pat_type.ty.as_ref() {
                if type_path.path.segments.len() == 1
                    && type_path.path.segments[0].ident == "usize"
                    && type_path.path.segments[0].arguments.is_empty()
                {
                    param_2_is_usize = true;
                }
            }
        }
        param_1_is_usize && param_2_is_usize
    } else {
        false
    };

    let valid_signature = f.sig.constness.is_none()
        && f.vis == Visibility::Inherited
        && f.sig.abi.is_none()
        && valid_input_types
        && f.sig.generics.params.is_empty()
        && f.sig.generics.where_clause.is_none()
        && f.sig.variadic.is_none()
        && match f.sig.output {
            ReturnType::Default => true,
            ReturnType::Type(_, ref ty) => match ty.as_ref() {
                Type::Tuple(tuple) => tuple.elems.is_empty(),
                _ => false,
            },
        };

    if !valid_signature {
        return parse::Error::new(
            f.span(),
            "`#[opensbi_rt::entry]` function must have signature `fn main(hartid: usize, dtb: usize)`",
        )
        .to_compile_error()
        .into();
    }

    if !args.is_empty() {
        return parse::Error::new(Span::call_site(), "This attribute accepts no arguments")
            .to_compile_error()
            .into();
    }

    let attrs = f.attrs;
    let generated_name = random_ident();
    let unsafety = f.sig.unsafety;
    let inputs = f.sig.inputs;
    let stmts = f.block.stmts;

    quote!(
        #[export_name = "main"]
        #(#attrs)*
        pub #unsafety fn #generated_name(#inputs) {
            #(#stmts)*
        }
    )
    .into()
}

static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

// Creates a random identifier
// Ref: https://github.com/rust-embedded/riscv-rt/blob/master/macros/src/lib.rs
fn random_ident() -> Ident {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let count: u64 = CALL_COUNT.fetch_add(1, Ordering::SeqCst) as u64;
    let mut seed: [u8; 16] = [0; 16];

    for (i, v) in seed.iter_mut().take(8).enumerate() {
        *v = ((secs >> (i * 8)) & 0xFF) as u8
    }

    for (i, v) in seed.iter_mut().skip(8).enumerate() {
        *v = ((count >> (i * 8)) & 0xFF) as u8
    }

    let mut rng = rand::rngs::SmallRng::from_seed(seed);
    Ident::new(
        &(0..16)
            .map(|i| {
                if i == 0 || rng.gen() {
                    (b'a' + rng.gen::<u8>() % 25) as char
                } else {
                    (b'0' + rng.gen::<u8>() % 10) as char
                }
            })
            .collect::<String>(),
        Span::call_site(),
    )
}
