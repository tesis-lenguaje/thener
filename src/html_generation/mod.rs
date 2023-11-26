use std::fs;
use std::io::Write;

use fs_extra::file;

fn resolve_variable(variable_name: &str, content: &str, variable_value: &str) -> String {
    let result = content.to_string();

    result.replace(&format!("#{{{variable_name}}}#"), variable_value)
}

pub fn resolve_template(template_path: &str, content_html: &str) -> Result<String, Box<dyn std::error::Error>>{
    let template = fs::read_to_string(&template_path)?;

    let complete_template = resolve_variable("contenido", &template, content_html);

    Ok(complete_template)
}

pub fn mermaid_to_svg(code: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Have to output to a temp file, read it, and then delete it.
    // Create a directory inside of `std::env::temp_dir()`.
    let dir = tempfile::tempdir()?;

    let file_path = dir.path().join("graph.svg");

    let mut cmd = std::process::Command::new("mmdc");
    cmd.arg("--input").arg("-")
        .arg("--backgroundColor").arg("transparent")
        .arg("--output").arg(file_path.to_str().unwrap());

    let mut child = cmd.stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).spawn().unwrap();

    let child_stdin = child.stdin.as_mut().unwrap();
    write!(child_stdin, "{}", code)?;

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, String::from_utf8(output.stderr).unwrap())));
    }

    let svg = fs::read_to_string(file_path.clone())?;

    // Remove the temp file
    file::remove(file_path.clone())
    .unwrap_or_else(|_| {
        println!("[INFO] No se pudo eliminar el archivo temporal: {:?}", file_path)
    });

    Ok(svg)
}