use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use toml_edit::DocumentMut;

pub async fn execute(release: bool) -> Result<()> {
    if !PathBuf::from("Cargo.toml").exists() {
        anyhow::bail!("No Cargo.toml found in current directory. Please run `santity build` inside a plugin directory.");
    }

    let mut cargo_cmd = Command::new("cargo");
    cargo_cmd.args(["build", "--target", "wasm32-unknown-unknown"]);
    if release {
        cargo_cmd.arg("--release");
    }

    println!("⚙️  Compiling guest WASM binary with cargo...");
    let status = cargo_cmd.status().context("Failed to run cargo build")?;
    if !status.success() {
        anyhow::bail!("Cargo build failed!");
    }

    // Safely parse exact ["package"]["name"] using toml_edit AST
    let cargo_str = fs::read_to_string("Cargo.toml")?;
    let doc: DocumentMut = cargo_str.parse().context("Failed to parse Cargo.toml format")?;

    let raw_pkg_name = doc
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .context("Could not find valid [package].name key in Cargo.toml")?;

    let pkg_name = raw_pkg_name.replace('-', "_");

    let mode_dir = if release { "release" } else { "debug" };
    let wasm_file = PathBuf::from("target/wasm32-unknown-unknown")
        .join(mode_dir)
        .join(format!("{}.wasm", pkg_name));

    let component_wasm_file = PathBuf::from("target/wasm32-unknown-unknown")
        .join(mode_dir)
        .join(format!("{}.component.wasm", pkg_name));

    if !wasm_file.exists() {
        anyhow::bail!("Compiled WASM binary not found at {:?}", wasm_file);
    }

    println!("🧩 Converting WASM module to Component Model binary using wasm-tools...");
    let wt_status = Command::new("wasm-tools")
        .args([
            "component",
            "new",
            wasm_file.to_str().unwrap(),
            "-o",
            component_wasm_file.to_str().unwrap(),
        ])
        .status()
        .context("Failed to execute wasm-tools. Please make sure `wasm-tools` is installed.")?;

    if !wt_status.success() {
        anyhow::bail!("wasm-tools component creation failed!");
    }

    println!("✅ Successfully created WASM Component Model binary at:");
    println!("   {:?}\n", component_wasm_file);
    println!("Run `santity plugin add {:?}` to deploy it live to Santity Core!", component_wasm_file);

    Ok(())
}
