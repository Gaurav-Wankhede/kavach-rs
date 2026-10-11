use std::fs;
use std::path::Path;
use svg::node::element::tag::Type;
use svg::parser::Event;

pub(crate) fn run(
    path_str: &str,
    paths_only: bool,
) -> i32 {
    let p = Path::new(path_str);
    if !p.exists() {
        eprintln!("kavach svg: file '{}' does not exist", path_str);
        return 2;
    }

    let content = match fs::read_to_string(p) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("kavach svg: error reading '{}': {}", path_str, e);
            return 1;
        }
    };

    let mut path_defs = Vec::new();
    let mut total_elements = 0usize;
    let mut view_box = None;
    let mut width = None;
    let mut height = None;

    for event in svg::read(&content).unwrap_or_else(|_| svg::parser::Parser::new("")) {
        match event {
            Event::Tag("svg", Type::Start, attrs) => {
                if let Some(vb) = attrs.get("viewBox") {
                    view_box = Some(vb.to_string());
                }
                if let Some(w) = attrs.get("width") {
                    width = Some(w.to_string());
                }
                if let Some(h) = attrs.get("height") {
                    height = Some(h.to_string());
                }
            }
            Event::Tag("path", _, attrs) => {
                total_elements += 1;
                if let Some(d) = attrs.get("d") {
                    path_defs.push(d.to_string());
                }
            }
            Event::Tag(_, Type::Start, _) => {
                total_elements += 1;
            }
            _ => {}
        }
    }

    if paths_only {
        for (i, d) in path_defs.iter().enumerate() {
            println!("path[{}]: {}", i, d);
        }
        return 0;
    }


    // Default inspection summary
    println!("── SVG Vector Profile: {} ──", path_str);
    println!("  viewBox:   {}", view_box.as_deref().unwrap_or("none"));
    println!("  width:     {}", width.as_deref().unwrap_or("auto"));
    println!("  height:    {}", height.as_deref().unwrap_or("auto"));
    println!("  elements:  {} (paths: {})", total_elements, path_defs.len());

    0
}
