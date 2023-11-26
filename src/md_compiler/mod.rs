use std::{path, fs};
use defer_lite::defer;
use path_absolutize::Absolutize;
use comrak::{parse_document, format_html, Arena, Options, ExtensionOptions, ParseOptions, RenderOptions, RenderOptionsBuilder};
use comrak::nodes::{AstNode, NodeValue};

use crate::html_generation;

static IMPORT_PREFIX: &str = "@import ";
// static EVAL_START: &str = "@>";
// static EVAL_END: &str = "<@";

static TAG_ID_MARKER: &str = "@#";
static TAG_CLASS_MARKER: &str = "@.";
static TAG_COMMENT_MARKER: &str = "@//";
// static ERROR_CLASS_MARKER: &str = "@!";
// static QUESTION_CLASS_MARKER: &str = "@?";

pub struct MarkdownPreprocessor {
    max_import_stack: u8,
}

impl MarkdownPreprocessor {
    pub fn new() -> Self {
        MarkdownPreprocessor { max_import_stack: 100 }
    }

    fn resolve_inline_tag(&self, line: &str, tag_marker: &str, replacement: fn(&str) -> String) -> String {
        let mut start = line.find(tag_marker);

        let mut line = line.to_string();
    
        while let Some(tag_start) = start {
            if tag_start > line.len() {
                break;
            }
    
            let from_tag = line.split_at(tag_start).1;
            let mut next_whitespace = from_tag.split_whitespace();
            let first_chunk = next_whitespace.next();
    
            if let Some(val) = first_chunk {
                let tag_end = tag_start + val.len();
    
                let tag_name = &val[tag_marker.len()..];
    
                let expanded = replacement(tag_name);
    
                line = line[..tag_start].to_string() + &expanded + &line[tag_end..];
    
                let rest_of_line = line.split_at(tag_start + expanded.len()).1;
                start = rest_of_line
                    .find(tag_marker)
                    .and_then(|count| Some(count + tag_start + expanded.len()));
            } else {
                break;
            }
        }

        line
    }

    fn resolve_tags(&self, line: &str) -> String {
        let with_ids = self.resolve_inline_tag(line, TAG_ID_MARKER, |tag| {
            format!("<span id='{tag}'></span>")
        });

        let with_classes = self.resolve_inline_tag(&with_ids, TAG_CLASS_MARKER, |tag| {
            format!("<span class='{tag}'></span>")
        });
        
        // Find comment and ignore everything until the end of the line
        let result = if let Some(comment_start) = with_classes.find(TAG_COMMENT_MARKER) {
            let (_, rest) = with_classes.split_at(comment_start);
            let comment_end = rest.find('\n').unwrap_or(rest.len());
            let (_, rest) = rest.split_at(comment_end);
            rest.to_string()
        } else {
            with_classes
        };

        result
        // TODO: Add remaining tag handling
    }
    
    fn preprocess_markdown_recursively(&self, file_name: &str, code: &str, import_depth: u8) -> Result<String, Box<dyn std::error::Error>> {
        assert!(import_depth < self.max_import_stack, "Stack de importes excedido al importar {file_name}");

        let original_dir = std::env::current_dir()?;

        println!("[INFO] Preprocesando {}", file_name);
        let file_path = path::Path::new(file_name);
        let actual_file_name = file_path.file_name().expect("Could not read file");
        let parent_dir = match file_path.parent() {
            Some(parent) => {
                if parent.to_str().unwrap_or("") == "" {
                    Ok(original_dir.clone())
                } else {
                    Ok(parent.to_path_buf())
                }
            },
            None => Err("No se pudo leer el directorio.") // TODO: Expressive errors
        }?;
    
        let mut result: Vec<String> = vec![];
        result.push(format!("<!-- fin del archivo {} -->", actual_file_name.to_str().unwrap_or("<unknown path>")));
        let lines = code.lines().rev();
        std::env::set_current_dir(parent_dir)?;
        defer! { std::env::set_current_dir(original_dir).unwrap_or(()) }
        
        for line in lines {
            let without_tags = if line.is_empty() {
                line.to_string()
            } else {
                self.resolve_tags(line)
            };

            if line.starts_with(IMPORT_PREFIX) {
                let file = line[IMPORT_PREFIX.len()..].to_string();
                let mut file_path = path::Path::new(&file).to_path_buf();
                if file_path.is_relative() {
                    file_path = path::Path::new(".").join(file_path).absolutize()?.to_path_buf();
                }

                file_path.set_extension("md");

                println!("[INFO] Importing {}", file_path.to_str().unwrap_or("<unknown path>"));

                let file_contents = fs::read_to_string(&file_path)?;
                let content = self.preprocess_markdown_recursively(
                    file_path.to_str().unwrap_or("<unknown path>"),
                    &file_contents,
                    import_depth + 1
                )?;

                for line in content.lines().rev() {
                    result.push(line.to_string());
                }
            } else {
                result.push(without_tags);
            }
        }

        result.push(format!("<!-- Importado del archivo {} -->", actual_file_name.to_str().unwrap_or("<unknown path>")));
    
        result.reverse();
        Ok(result.join("\n"))
    }
    
    pub fn preprocess_markdown(&self, name: &str, code: &str) -> Result<String, Box<dyn std::error::Error>> {
        self.preprocess_markdown_recursively(name, code, 0)
    }
}

fn iter_nodes<'a, F>(node: &'a AstNode<'a>, f: &F) -> Result<(), Box<dyn std::error::Error>>
    where F : Fn(&'a AstNode<'a>) -> Result<(), Box<dyn std::error::Error>> {
    f(node)?;
    for c in node.children() {
        iter_nodes(c, f)?;
    }

    Ok(())
}

pub fn markdown_to_html(md: &str) -> Result<String, Box<dyn std::error::Error>> {
    // The returned nodes are created in the supplied Arena, and are bound by its lifetime.
    let arena = Arena::new();

    let mut options = Options::default();
    options.render.unsafe_ = true;
    let root = parse_document(
        &arena,
        md,
        &options);

    iter_nodes(root, &|node| {
        // Find a block code, check if the language is mermaid, and replace it with the svg.
        let should_replace_with_svg = if let NodeValue::CodeBlock(ref code) = node.data.borrow().value {
            code.info.starts_with("mermaid")
        } else {
            false
        };

        if should_replace_with_svg {
            println!("[INFO] Generating graph");
            let code = {
                let value = &node.data.borrow().value;
                match value {
                    NodeValue::CodeBlock(code) => code.clone(),
                    _ => unreachable!(),
                }
            };

            let svg = html_generation::mermaid_to_svg(&code.literal)?;
            let svg = format!("<figure class='mermaid-graph'>{}</figure>", svg);
            node.data.borrow_mut().value = NodeValue::HtmlInline(svg.into());
            println!("[INFO] Graph generated");
        }

        Ok(())
    })?;

    let mut html = vec![];
    format_html(root, &options, &mut html).unwrap();

    Ok(String::from_utf8(html).unwrap())
}