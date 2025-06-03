use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Settings)]
pub fn derive_settings(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    // 生成实现代码
    let expanded = quote! {
        impl settings::Settings for #name {
            type Config = #name;

            fn instance() -> anyhow::Result<std::sync::Arc<std::sync::Mutex<Self>>>
            where
                Self: Sized,
            {
                static INSTANCE: once_cell::sync::OnceCell<std::sync::Arc<std::sync::Mutex<#name>>> = once_cell::sync::OnceCell::new();

                Ok(INSTANCE.get_or_try_init(|| -> anyhow::Result<std::sync::Arc<std::sync::Mutex<#name>>> {
                    let default_path = std::path::PathBuf::from("config/settings.yaml");
                    Ok(std::sync::Arc::new(std::sync::Mutex::new(Self::load(&default_path)?)))
                })?.clone())
            }
        }
    };

    TokenStream::from(expanded)
}
