use defer_lite::defer;
use fs_extra;
use indicatif;
use std::{fs, path};

use markdown::{self, CompileOptions, Options, ParseOptions};

use crate::{file_utils, md_compiler, html_generation, pdf_exporter};

/* Contents of 'project.thn':
{
    "path": ".",
    "assets": "./assets",
    "template": "./templates/main.html",
    "output": "./build",
    "entry": "start.md"
}

This struct is serializable from this file format (location should be "." if not provided):
*/
#[derive(serde::Serialize, serde::Deserialize)]
struct ReadProject {
    // Path is where the actual project contents are located.
    path: String,
    assets: String,
    entry: String,
    output: Option<String>,
    template: String
}

// This struct is the actual project, takes a ReadProject and makes it a Project with the location set to the path of the project file.
pub struct Project {
    path: String,
    assets: std::path::PathBuf,
    entry: std::path::PathBuf,
    template: std::path::PathBuf,
    output: std::path::PathBuf,
    location: std::path::PathBuf
}

impl Project {
    fn from_read_project(read_project: &ReadProject, project_location: &str) -> Project {
        let project_path = path::Path::new(&read_project.path).to_path_buf();
        let project_assets = path::Path::new(&read_project.assets).to_path_buf();
        let project_entry = path::Path::new(&read_project.entry).to_path_buf();
        let project_template = path::Path::new(&read_project.template).to_path_buf();
        let project_output = path::Path::new(&read_project.output.clone().unwrap_or("./build".to_string())).to_path_buf();
        let project_location = path::Path::new(project_location).to_path_buf();

        Project {
            path: project_path.to_str().unwrap_or(".").to_string(),
            assets: project_assets,
            entry: project_entry,
            template: project_template,
            output: project_output,
            location: project_location
        }
    }
}

pub fn build_project(project: &Project) -> Result<(), Box<dyn std::error::Error>> {
    // Change the working directory to the project's path, remembering the original one.
    let original_dir = std::env::current_dir()?;
    std::env::set_current_dir(&project.location)?;
    defer! { std::env::set_current_dir(original_dir.clone()).unwrap_or(()) }

    // Create the build directory if it doesn't exist.
    println!("[INFO] Creating build directory");
    let build_path = file_utils::try_absolute(&project.path)?;
    if fs::File::open(&build_path).is_err() {
        fs::create_dir_all(&build_path)?;
    }

    // Load the markdown file.
    println!("[INFO] Reading entry point");
    let entry_path = path::Path::new(&project.path).join(&project.entry);
    let entry_md = fs::read_to_string(&entry_path)?;

    // Preprocess the markdown.
    println!("[INFO] Preprocessing markdown");
    let preprocessor = md_compiler::MarkdownPreprocessor::new();
    let preprocessed = preprocessor.preprocess_markdown(
        project.entry.to_str().ok_or("Could not read path for entry point")?,
        &entry_md
    )?;

    /*This is how the output tree will look like:
    .
    ├── {project.output}
    │   ├── html
    │   │   ├── {project.assets}
    │   │   │   └── img
    │   │   │       └── logo.png
    │   │   ├── index.html (this is the compiled markdown and exported into template)
    │   │── pdf
    │       └── index.pdf
    ├── project.thn
    ├── start.md
     */

    let html_assets_path = path::Path::new(&project.path).join(&project.output).join("html");
    let absolute_html_assets_path = file_utils::try_absolute(&html_assets_path.to_string_lossy())?;

    let pdf_assets_path = path::Path::new(&project.path).join(&project.output).join("pdf");
    let absolute_pdf_assets_path = file_utils::try_absolute(&pdf_assets_path.to_string_lossy())?;
    
    // Copy all the contents of assets to the build directory. Files and directories
    // are copied recursively.
    println!("[INFO] Copying assets");
    let assets_path = path::Path::new(&project.path).join(&project.assets);
    let absolute_assets_path = file_utils::try_absolute(&assets_path.to_string_lossy())?;

    // Create the build assets directory if it doesn't exist.
    if fs::File::open(&absolute_html_assets_path).is_err() {
        fs::create_dir_all(&absolute_html_assets_path)?;
    }

    if fs::File::open(&absolute_pdf_assets_path).is_err() {
        fs::create_dir_all(&absolute_pdf_assets_path)?;
    }

    fs_extra::dir::copy_with_progress(&absolute_assets_path, &absolute_html_assets_path, &fs_extra::dir::CopyOptions::new(), |x| {
        // Show the file being copied. Tabbed so that it's clear it's a sub process.
        print!("[INFO]\t Copying {} ({}/{} bytes)", x.file_name, x.file_bytes_copied, x.file_total_bytes);
        print!("{}",
            if x.file_bytes_copied < x.file_total_bytes {
                "\r"
            } else {
                "\n"
            }
        );

        fs_extra::dir::TransitProcessResult::Overwrite
    })?;

    // Generate the HTML from the markdown.
    println!("[INFO] Generating HTML");
    let pure_html = md_compiler::markdown_to_html(&preprocessed)?;
    // let mut pure_html = markdown::to_html_with_options(&preprocessed, &Options {
    //     compile: CompileOptions {
    //       allow_dangerous_html: true,
    //       ..CompileOptions::default()
    //     },
    //     ..Options::default()
    // }).unwrap();

    // Resolve the template.
    println!("[INFO] Resolving template");
    let template_path = path::Path::new(&project.path).join(&project.template);
    let absolute_template_path = file_utils::try_absolute(&template_path.to_string_lossy())?;
    let wrapped_html = html_generation::resolve_template(
        &absolute_template_path,
        &pure_html
    )?;

    // Write the HTML to the build directory.
    println!("[INFO] Writing HTML");
    let build_html_path = path::Path::new(&absolute_html_assets_path).join("index.html");
    fs::write(&build_html_path, &wrapped_html)?;
    
    // Generate the PDF from the HTML.
    println!("[INFO] Generating PDF");

    let build_pdf_path = path::Path::new(&absolute_pdf_assets_path).join("index.pdf");

    pdf_exporter::export_to_pdf(&build_html_path, &build_pdf_path)?;

    // TODO: Fix the table of contents.

    println!("[INFO] Done");

    Ok(())
}

pub fn read_configuration(project_path: &str) -> Result<Project, Box<dyn std::error::Error>> {
    let project_path = file_utils::try_absolute(&project_path)?;
    let project_parent = path::Path::new(&project_path).parent()
        .ok_or(format!("No se pudo encontrar el directorio padre de {project_path}"))?;

    println!("[INFO] Reading project");
    let project_file = fs::File::open(&project_path)?;

    let read_project: ReadProject = serde_json::from_reader(project_file)?;

    println!("[INFO] Building project");

    Ok(
        Project::from_read_project(&read_project, project_parent.to_str().unwrap_or("."))
    )
}