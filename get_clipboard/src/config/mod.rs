pub mod io;
pub mod model;

pub use io::{ensure_data_dir, load_config, save_config};
pub use model::AppConfig;
