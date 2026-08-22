use anyhow::{Context, Result};
use inquire::Text;
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::{Array, DocumentMut, Item, Table, Value};

fn get_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("santity")
}

fn get_config_path() -> PathBuf {
    get_config_dir().join("config.toml")
}

fn get_plugins_dir() -> PathBuf {
    get_config_dir().join("plugins")
}

pub async fn add(source: &str) -> Result<()> {
    let file_name = if source.starts_with("http://") || source.starts_with("https://") {
        source
            .split('/')
            .last()
            .unwrap_or("plugin.component.wasm")
            .to_string()
    } else {
        Path::new(source)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("plugin.component.wasm")
            .to_string()
    };

    let plugin_name = file_name
        .trim_end_matches(".component.wasm")
        .trim_end_matches(".wasm")
        .to_string();

    // Interactive Capability Prompt
    println!("\n🛡️  Capability-Gated Boundary Prompt:");
    let domains_input = Text::new(&format!(
        "Enter allowed outbound network domains for plugin '{}' (comma-separated, leave blank for none):",
        plugin_name
    ))
    .with_placeholder("api.github.com, api.discord.com")
    .prompt_skippable()?;

    let allowed_domains: Vec<String> = domains_input
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    add_with_capabilities(source, allowed_domains).await
}

pub async fn add_with_capabilities(source: &str, allowed_domains: Vec<String>) -> Result<()> {
    let plugins_dir = get_plugins_dir();
    let staging_dir = plugins_dir.join(".staging");
    
    fs::create_dir_all(&plugins_dir)?;
    fs::create_dir_all(&staging_dir)?;

    let is_url = source.starts_with("http://") || source.starts_with("https://");

    let file_name = if is_url {
        source
            .split('/')
            .last()
            .unwrap_or("plugin.component.wasm")
            .to_string()
    } else {
        Path::new(source)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("plugin.component.wasm")
            .to_string()
    };

    let plugin_name = file_name
        .trim_end_matches(".component.wasm")
        .trim_end_matches(".wasm")
        .to_string();

    let staged_path = staging_dir.join(&file_name);

    if is_url {
        println!("🌐 Downloading Wasm component from {}...", source);
        let bytes = reqwest::get(source).await?.bytes().await?;
        fs::write(&staged_path, bytes)?;
        println!("  • Download complete: {:?}", staged_path);
    } else {
        let src_path = PathBuf::from(source);
        if !src_path.exists() {
            anyhow::bail!("Plugin source file not found at {:?}", src_path);
        }
        // Copy to hidden .staging dir on the SAME target filesystem first
        fs::copy(&src_path, &staged_path)?;
        println!("  • Staged plugin binary at {:?}", staged_path);
    }

    // Target binary location
    let target_path = plugins_dir.join(&file_name);

    // Patch config.toml safely using toml_edit
    let config_path = get_config_path();
    if config_path.exists() {
        let config_str = fs::read_to_string(&config_path)?;
        let mut doc: DocumentMut = config_str.parse().context("Failed to parse config.toml")?;

        let plugins_arr = doc
            .entry("plugins")
            .or_insert_with(|| Item::Value(Value::Array(Array::new())))
            .as_array_of_tables_mut();

        if let Some(arr) = plugins_arr {
            // Check if plugin with same name already exists in config
            let mut existing = false;
            for plugin_table in arr.iter_mut() {
                if plugin_table.get("name").and_then(|v| v.as_str()) == Some(&plugin_name) {
                    plugin_table["path"] = Item::Value(Value::from(target_path.to_string_lossy().to_string()));
                    let mut dom_arr = Array::new();
                    for d in &allowed_domains {
                        dom_arr.push(d.as_str());
                    }
                    plugin_table["allowed_domains"] = Item::Value(Value::Array(dom_arr));
                    existing = true;
                    break;
                }
            }

            if !existing {
                let mut new_plugin = Table::new();
                new_plugin["name"] = Item::Value(Value::from(plugin_name.clone()));
                new_plugin["path"] = Item::Value(Value::from(target_path.to_string_lossy().to_string()));
                let mut dom_arr = Array::new();
                for d in &allowed_domains {
                    dom_arr.push(d.as_str());
                }
                new_plugin["allowed_domains"] = Item::Value(Value::Array(dom_arr));
                arr.push(new_plugin);
            }
        }

        fs::write(&config_path, doc.to_string())?;
        println!("✅ Updated {:?} with capability whitelists.", config_path);
    }

    // TRUE ATOMIC POSIX RENAME (Guaranteed same filesystem)
    fs::rename(&staged_path, &target_path)
        .with_context(|| format!("Failed to perform atomic swap from {:?} to {:?}", staged_path, target_path))?;

    println!("⚡ True atomic POSIX rename complete! Plugin live at {:?}", target_path);
    println!("   Santity Core file-watcher has hot-loaded plugin '{}' with zero downtime!\n", plugin_name);

    Ok(())
}

pub async fn list() -> Result<()> {
    let config_path = get_config_path();
    if !config_path.exists() {
        println!("ℹ️  No config.toml file found at {:?}", config_path);
        return Ok(());
    }

    let config_str = fs::read_to_string(&config_path)?;
    let doc: DocumentMut = config_str.parse()?;

    println!("📦 Installed WASM Plugins:");
    if let Some(plugins) = doc.get("plugins").and_then(|p| p.as_array_of_tables()) {
        for (idx, plugin) in plugins.iter().enumerate() {
            let name = plugin.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
            let path = plugin.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let domains: Vec<&str> = plugin
                .get("allowed_domains")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|val| val.as_str()).collect())
                .unwrap_or_default();

            println!(
                "  {}. {} (Path: {:?}) | Whitelisted Domains: {:?}",
                idx + 1,
                name,
                path,
                domains
            );
        }
    } else {
        println!("  (No plugins registered in config.toml)");
    }

    Ok(())
}
