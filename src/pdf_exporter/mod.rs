use std::path::PathBuf;
use headless_chrome::{self, types::PrintToPdfOptions};
use anyhow::Result;
use std::fmt;
use url::Url;

#[derive(Debug)]
enum Error {
    InvalidPath
}

impl Error {
    fn to_str(&self) -> &str {
        match self {
            Error::InvalidPath => "Invalid path"
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        self.to_str()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = self.to_str();
        write!(f, "{}", description)
    }
}

pub fn export_to_pdf(html_path: &PathBuf, output_path: &PathBuf) -> Result<()> {
    let browser = headless_chrome::Browser::default()?;
    let tab = browser.new_tab()?;
    
    let html_url = Url::from_file_path(html_path).map_err(|_| Error::InvalidPath)?;

    tab.navigate_to(html_url.as_str())?;
    tab.wait_until_navigated()?;

    /*
    This is the previous code. Must use the same configurations.

    let mut cmd = std::process::Command::new("wkhtmltopdf");
    cmd.arg("--page-offset").arg("-1");
    cmd.arg("--page-width").arg("8.5in");
    cmd.arg("--page-height").arg("11in");
    cmd.arg("--enable-local-file-access");
    cmd.arg("--print-media-type");
    cmd.arg(build_html_path);
    cmd.arg(build_pdf_path.clone()); */
    let pdf_data = tab
        .print_to_pdf(Some(PrintToPdfOptions {
            landscape: Some(false),
            display_header_footer: Some(false),
            print_background: Some(true),
            scale: Some(1.0),
            paper_width: Some(8.5),
            paper_height: Some(11.0),
            margin_top: Some(0.0),
            margin_bottom: Some(0.0),
            margin_left: Some(0.0),
            margin_right: Some(0.0),
            page_ranges: None,
            ignore_invalid_page_ranges: Some(false),
            header_template: None,
            footer_template: None,
            prefer_css_page_size: Some(true),
            transfer_mode: None,
        }))?;

    std::fs::write(output_path, pdf_data)?;

    Ok(())
}