use clap::Parser;
use serde;

mod md_compiler;
mod file_utils;
mod html_generation;
mod project_builder;
mod pdf_exporter;

#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Clone, Debug)]
enum OutputFormat {
    PDF,
    HTML
}

// Implement from str on output format
impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pdf" => Ok(OutputFormat::PDF),
            "html" => Ok(OutputFormat::HTML),
            _ => Err(format!("Formato de salida no soportado: {}", s))
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version)]
#[command(about = "Toma un projecto thener y genera un pdf o html", long_about = None)]
struct Args {
    #[arg(value_name="project_file")]
    project: String,

    #[arg(short, long, value_name="format", default_value="pdf")]
    format: Option<OutputFormat>
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Args::parse();

    let project = project_builder::read_configuration(&cli.project)?;
    project_builder::build_project(&project)?;

    Ok(())
}


/*
  A general description of this library:
  This library is a tool for compiling a project written in markdown, down into a pdf or html file.
  It's made to help me write my thesis, but it's also made to be easily extensible and usable for other projects.

  The way it works:
 */