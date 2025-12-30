# Configuration Module

You are a Rust specialist working on get_clipboard's configuration management.

## Project Knowledge

- **Purpose:** Load, save, and manage user configuration
- **Location:** `~/Library/Application Support/com.clipboard/config.json`

### File Structure

| File | Purpose |
|------|---------|
| `mod.rs` | Module exports |
| `io.rs` | File reading/writing |
| `model.rs` | Config struct definitions |

## Code Style

### Config Model
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default)]
    pub data_dir: Option<PathBuf>,
    
    #[serde(default = "default_port")]
    pub api_port: u16,
    
    #[serde(default)]
    pub excluded_apps: Vec<String>,
}

fn default_port() -> u16 { 3016 }

impl Default for Config {
    fn default() -> Self {
        Self {
            data_dir: None,
            api_port: default_port(),
            excluded_apps: vec![],
        }
    }
}
```

### Loading with Defaults
```rust
pub fn load_config() -> Result<Config> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = fs::read_to_string(&path)
        .context("Failed to read config file")?;
    serde_json::from_str(&content)
        .context("Invalid config JSON")
}
```

### Directory Helpers
```rust
pub fn ensure_data_dir() -> Result<PathBuf> {
    let config = load_config()?;
    let dir = config.data_dir.unwrap_or_else(default_data_dir);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn default_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap()
        .join("com.clipboard")
        .join("data")
}
```

## Conventions

- **Default Values**: Use `#[serde(default)]` and `Default` impl
- **camelCase JSON**: Use `#[serde(rename_all = "camelCase")]`
- **Optional Overrides**: Allow users to override defaults
- **Create on Access**: Create directories when first accessed

## Boundaries

- ✅ **Always do:**
  - Provide sensible defaults
  - Use `camelCase` for JSON fields
  - Handle missing config gracefully

- ⚠️ **Ask first:**
  - Adding new config fields
  - Changing default values
  - Changing config file location

- 🚫 **Never do:**
  - Require config file to exist
  - Use different casing in JSON
  - Store sensitive data without encryption
