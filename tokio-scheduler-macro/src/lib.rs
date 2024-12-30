use proc_macro::TokenStream;
use quote::ToTokens;
use syn::parse::Parser;
use syn::Ident;

macro_rules! into_compile_error {
    ($($tt:tt)*) => {
        syn::Error::new(proc_macro2::Span::call_site(), format!($($tt)*))
            .to_compile_error()
        .into()
    };
}

/// This macro is used to annotate a struct or a function as a job.
/// Every job marked by this macro can be automatically registered to the job manager by calling `auto_register_job` fn from the job manager.
///
/// The struct must implement the Job trait.
///
/// The function must have the signature `async fn(ctx: JobContext) -> anyhow::Result<JobReturn>`.
///
/// The function will be converted to a struct that implements the Job trait. Struct name will be
/// converted to UpperCamelCase.
///
/// If you want to use custom name for the fn type job, you can use `name` argument.
///
/// ```rust
/// # use tokio_scheduler_macro::job;
/// #[job(name="CustomName")]
/// # fn foo(){}
/// ```
///
/// # Example for fn
/// ```rust
/// # use tokio_scheduler_macro::job;
/// # use tokio_scheduler_types::job::{JobContext, JobReturn};
///
/// #[job]
/// async fn example_job(ctx: JobContext) -> anyhow::Result<JobReturn> {
///    println!("Hello from example job");
///   Ok(JobReturn::default())
/// }
/// ```
///
/// The code above equivalent to:
/// ```rust
/// # use tokio_scheduler_types::job::{Job, JobContext, JobFuture, JobReturn};
///
/// struct ExampleJob;
///
/// impl Job for ExampleJob {
///    fn get_job_name(&self) -> &'static str {
///      "ExampleJob"
/// }
///
/// fn execute(&self, ctx: JobContext) -> JobFuture {
///   Box::pin(async move {
///         println!("Hello from example job");
///         Ok(JobReturn::default())
///         })
///     }
/// }
/// ```
/// # Example for struct
/// ```rust
/// # use tokio_scheduler_macro::job;
/// # use tokio_scheduler_types::job::{Job, JobContext, JobFuture, JobReturn};
///
/// #[job]
/// struct ExampleJob;
///
/// impl Job for ExampleJob {
///    fn get_job_name(&self) -> &'static str {
///       "ExampleJob"
///   }
///
///  fn execute(&self, ctx: JobContext) -> JobFuture {
///     Box::pin(async move {
///       println!("Hello from example job");
///       Ok(JobReturn::default())
///     })
///     }
///  }
/// ```
#[proc_macro_attribute]
pub fn job(args: TokenStream, input: TokenStream) -> TokenStream {
    let parsed_input = syn::parse::<syn::Item>(input).unwrap();

    // Parse the arguments, find #[job(name="xxx")]
    let args_parsed =
        syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated.parse(args);

    if args_parsed.is_err() {
        return into_compile_error!("Invalid arguments.");
    }

    let args_parsed = args_parsed.unwrap();

    let mut custom_name = None;

    for arg in args_parsed {
        match arg {
            syn::Expr::Assign(assign) => {
                let left = assign.left.to_token_stream().to_string();
                let right = assign.right.to_token_stream().to_string();

                if left == "name" {
                    let name = right.trim_matches('"');
                    if name.is_empty() {
                        return into_compile_error!("Invalid name.");
                    }

                    custom_name = Some(name.to_owned());
                } else {
                    return into_compile_error!("Invalid arguments.");
                }
            }
            _ => {
                return into_compile_error!("Invalid arguments.");
            }
        }
    }

    match parsed_input {
        syn::Item::Struct(item_struct) => {
            if let Some(_) = custom_name {
                return into_compile_error!("Struct with custom job name is not supported. Please edit get_job_name() fn directly.");
            }

            let name = item_struct.ident.to_owned();

            let output = quote::quote! {
                #item_struct

                ::tokio_scheduler_rs::inventory::submit!(&#name as &dyn ::tokio_scheduler_rs::job::Job);
            };
            output.into()
        }
        syn::Item::Fn(item_fn) => {
            // get first argument name
            let first_arg = item_fn.sig.inputs.first().unwrap();
            let first_arg = match first_arg {
                syn::FnArg::Typed(pat) => pat,
                _ => {
                    return into_compile_error!("Invalid function signature.");
                }
            };

            let first_arg = match first_arg.pat.as_ref() {
                syn::Pat::Ident(pat) => pat.ident.to_owned(),
                _ => {
                    return into_compile_error!("Invalid function signature.");
                }
            };

            let body = item_fn.block.clone().stmts;

            let body = quote::quote! {
                #(#body)*
            };

            let fn_name = match custom_name {
                Some(name) => name,
                None => convert_case::Converter::new()
                    .to_case(convert_case::Case::UpperCamel)
                    .convert(item_fn.sig.ident.to_string().as_str()),
            };

            let struct_ident = Ident::new(&fn_name, item_fn.sig.ident.span());

            let fn_vis = item_fn.vis.clone();

            let output = quote::quote! {

                #fn_vis struct #struct_ident;

                impl ::tokio_scheduler_rs::job::Job for #struct_ident {
                    fn get_job_name(&self) -> &'static str {
                        #fn_name
                    }

                    fn execute(&self, #first_arg: ::tokio_scheduler_rs::job::JobContext) -> ::tokio_scheduler_rs::job::JobFuture {
                        Box::pin(async move {
                            #body
                        })
                    }
                }

                ::tokio_scheduler_rs::inventory::submit!(&#struct_ident as &dyn ::tokio_scheduler_rs::job::Job);
            };

            output.into()
        }
        _ => {
            into_compile_error!("Only struct and fn can be annotated with #[job]")
        }
    }
}
