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

            // fn load(path: &std::path::PathBuf) -> anyhow::Result<Self::Config> {
            //     // use std::fs;
            //     // use anyhow::Context;

            //     // if !path.exists() {
            //     //     let default_config = Self::Config::default();
            //     //     return Ok(default_config);
            //     // }

            //     // let config_content = fs::read_to_string(path)
            //     //     .context(format!("Failed to read settings file: {:?}", path))?;

            //     // let config: Self::Config = serde_yaml::from_str(&config_content)
            //     //     .context(format!("Failed to parse settings file: {:?}", path))?;
            //     let config = Self::Config::default();
            //     Ok(config)
            // }
        }
    };

    TokenStream::from(expanded)
}
