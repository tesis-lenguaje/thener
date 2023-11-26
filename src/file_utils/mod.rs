use std::path;
use path_absolutize::Absolutize;

pub fn try_absolute_based_on_path(base_path: &str, path_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let base_path = path::Path::new(&base_path);
    let joined_path = base_path.join(path_name);

    match joined_path.absolutize()?.to_str() {
        Some(path) => Ok(path.to_string()),
        None => Err("No se pudo leer el archivo".into())
    }
}

pub fn try_absolute(path_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let as_absolute = path::Path::new(&path_name)
        .absolutize()?;

    match as_absolute.to_str() {
        Some(path) => Ok(path.to_string()),
        None => Err("No se pudo leer el archivo".into())
    }
}