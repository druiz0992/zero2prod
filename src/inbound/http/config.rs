use std::path::PathBuf;

pub const TEMPLATES_DIR: &str = "templates";

pub fn get_template_path(template_name: &str) -> PathBuf {
    PathBuf::from(TEMPLATES_DIR).join(template_name)
}
