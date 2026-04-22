use regex::Regex;
use scraper::{Html, Selector};

use crate::types::Framework;

pub fn normalize_markdown(input: &str, framework: &Framework) -> String {
    let mut output = input.replace("\r\n", "\n");

    if matches!(
        framework,
        Framework::GitBookModern | Framework::GitBookClassic
    ) {
        output = normalize_gitbook_blocks(&output);
    }

    let multi_newline = Regex::new(r"\n{3,}").expect("valid regex");
    output = multi_newline.replace_all(&output, "\n\n").to_string();

    output.trim().to_string() + "\n"
}

fn normalize_gitbook_blocks(input: &str) -> String {
    let hint_re =
        Regex::new(r#"\{%\s*hint\s+style="([^"]+)"\s*%\}\s*(?s)(.*?)\s*\{%\s*endhint\s*%\}"#)
            .expect("valid regex");
    let tab_open_re = Regex::new(r#"\{%\s*tab\s+title="([^"]+)"\s*%\}"#).expect("valid regex");

    let mut output = hint_re
        .replace_all(input, |captures: &regex::Captures| {
            let style = captures
                .get(1)
                .map(|m| m.as_str().to_uppercase())
                .unwrap_or_else(|| "NOTE".to_string());
            let body = captures
                .get(2)
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim()
                .lines()
                .map(|line| format!("> {line}"))
                .collect::<Vec<_>>()
                .join("\n");
            format!("> [!{style}]\n{body}\n")
        })
        .to_string();

    output = output.replace("{% tabs %}", "");
    output = output.replace("{% endtabs %}", "");
    output = output.replace("{% endtab %}", "");
    output = tab_open_re
        .replace_all(&output, |captures: &regex::Captures| {
            format!("\n### {}\n", &captures[1])
        })
        .to_string();

    output
}

pub fn html_to_markdown(html: &str, framework: &Framework) -> String {
    let html_fragment = extract_main_html(html);
    let markdown = html2md::parse_html(&html_fragment);
    normalize_markdown(&markdown, framework)
}

pub fn extract_title_from_html(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    let selectors = ["h1", "title"];

    for selector in selectors {
        let selector = Selector::parse(selector).ok()?;
        if let Some(element) = document.select(&selector).next() {
            let title = element.text().collect::<Vec<_>>().join(" ");
            let title = clean_whitespace(&title);
            if !title.is_empty() {
                return Some(title);
            }
        }
    }

    None
}

fn extract_main_html(html: &str) -> String {
    let document = Html::parse_document(html);
    let selectors = [
        "main",
        "[role='main']",
        "article",
        ".theme-doc-markdown",
        ".markdown",
        ".content",
        ".page",
        "body",
    ];

    for raw_selector in selectors {
        let selector = Selector::parse(raw_selector).expect("valid selector");
        if let Some(element) = document.select(&selector).next() {
            let fragment = element.html();
            if !fragment.trim().is_empty() {
                return fragment;
            }
        }
    }

    html.to_string()
}

fn clean_whitespace(input: &str) -> String {
    input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}
