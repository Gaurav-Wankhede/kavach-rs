use std::fs;
use std::io::{self, Read};
use std::path::Path;
use scraper::{Html, Selector};

pub(crate) fn run(
    selector_str: &str,
    path_opt: Option<&str>,
    attr_opt: Option<&str>,
    text_only: bool,
    count_only: bool,
) -> i32 {
    let input_content = match path_opt {
        Some(p) => {
            let path = Path::new(p);
            if !path.exists() {
                eprintln!("kavach html: file '{}' does not exist", p);
                return 2;
            }
            match fs::read_to_string(path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("kavach html: error reading file '{}': {}", p, e);
                    return 1;
                }
            }
        }
        None => {
            let mut buf = String::new();
            if let Err(e) = io::stdin().read_to_string(&mut buf) {
                eprintln!("kavach html: error reading stdin: {}", e);
                return 1;
            }
            buf
        }
    };

    let document = Html::parse_document(&input_content);
    let selector = match Selector::parse(selector_str) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("kavach html: invalid CSS selector '{}': {:?}", selector_str, e);
            return 2;
        }
    };

    let elements: Vec<_> = document.select(&selector).collect();

    if count_only {
        println!("{}", elements.len());
        return 0;
    }

    for el in elements {
        if let Some(attr) = attr_opt {
            if let Some(val) = el.value().attr(attr) {
                println!("{}", val);
            }
        } else if text_only {
            let text: String = el.text().collect::<Vec<_>>().join(" ");
            println!("{}", text.trim());
        } else {
            let inner = el.inner_html();
            println!("{}", inner.trim());
        }
    }

    0
}
