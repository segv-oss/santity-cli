use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::PathBuf;

pub async fn execute(name: &str, local_pdk: Option<&str>) -> Result<()> {
    let target_dir = PathBuf::from(name);
    if target_dir.exists() {
        anyhow::bail!("Directory '{}' already exists!", name);
    }

    fs::create_dir_all(target_dir.join("src")).context("Failed to create plugin source directory")?;

    let pdk_override = local_pdk
        .map(String::from)
        .or_else(|| env::var("SANTITY_PDK_PATH").ok());

    let pdk_dependency = if let Some(pdk_path) = pdk_override {
        format!("santity-pdk = {{ path = \"{}\" }}", pdk_path)
    } else {
        "santity-pdk = \"0.1.0\"".to_string()
    };

    let cargo_toml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"

[lib]
crate-type = ["cdylib"]

[dependencies]
{pdk_dependency}
log = "0.4"
serde = {{ version = "1.0", features = ["derive"] }}
"#
    );

    let lib_rs = format!(
        r#"use santity_pdk::prelude::*;

struct MyPlugin;

#[plugin(name = "{name}", version = "0.1.0")]
impl MyPlugin {{
    #[command(name = "hello", description = "Sample command created with santity-cli")]
    fn hello(_event: InteractionEvent) -> ResponseAction {{
        info!("Executing hello command in {name} plugin!");
        ResponseAction::reply("Hello from your sandboxed WASM plugin! 🚀".to_string())
    }}
}}
"#
    );

    fs::write(target_dir.join("Cargo.toml"), cargo_toml)?;
    fs::write(target_dir.join("src").join("lib.rs"), lib_rs)?;

    println!("✨ Created new Santity WASM plugin project at ./{}/", name);
    println!("  • Cargo.toml created with {}", pdk_dependency);
    println!("  • src/lib.rs generated with sample #[command]");
    println!("\nNext steps:");
    println!("  cd {}", name);
    println!("  santity build");
    println!("  santity plugin add target/wasm32-unknown-unknown/release/{}.component.wasm", name);

    Ok(())
}
