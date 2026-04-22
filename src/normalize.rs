use regex::Regex;
use scraper::{Html, Selector};

use crate::types::Framework;

const DEFAULT_CONTENT_SELECTORS: &[&str] = &[
    "main",
    "[role='main']",
    "article",
    ".theme-doc-markdown",
    ".markdown",
    ".content",
    ".page",
    "body",
];

pub fn normalize_markdown(input: &str, framework: &Framework) -> String {
    let mut output = input.replace("\r\n", "\n");

    output = match framework {
        Framework::GitBookModern | Framework::GitBookClassic => normalize_gitbook_blocks(&output),
        Framework::Docusaurus => normalize_docusaurus_blocks(&output),
        Framework::MkDocsMaterial => normalize_mkdocs_material_blocks(&output),
        Framework::VitePress => normalize_vitepress_blocks(&output),
        Framework::Nextra => normalize_nextra_blocks(&output),
        Framework::GenericDocsFallback => output,
    };

    output = strip_inline_chrome(&output);
    output = strip_bootstrap_noise(&output);
    output = strip_common_chrome(&output);
    output = trim_leading_preamble(&output);
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

fn normalize_docusaurus_blocks(input: &str) -> String {
    let admonition_re = Regex::new(r":::(\w+)\s*(?s)(.*?)\s*:::").expect("valid regex");
    admonition_re
        .replace_all(input, |captures: &regex::Captures| {
            let style = captures
                .get(1)
                .map(|m| m.as_str().to_uppercase())
                .unwrap_or_else(|| "NOTE".to_string());
            let body = captures.get(2).map(|m| m.as_str()).unwrap_or("").trim();
            format!("> [!{style}]\n> {}\n", body.replace('\n', "\n> "))
        })
        .to_string()
}

fn normalize_mkdocs_material_blocks(input: &str) -> String {
    let mut output = String::new();
    let mut current_tab: Option<String> = None;
    let mut current_body: Vec<String> = Vec::new();

    let flush_tab =
        |output: &mut String, current_tab: &mut Option<String>, current_body: &mut Vec<String>| {
            if let Some(title) = current_tab.take() {
                let body = current_body.join("\n").trim().to_string();
                if !output.is_empty() && !output.ends_with("\n\n") {
                    output.push('\n');
                }
                output.push_str(&format!("### {title}\n\n"));
                output.push_str(body.trim());
                output.push_str("\n\n");
                current_body.clear();
            }
        };

    for line in input.lines() {
        let trimmed = line.trim();
        let maybe_tab = trimmed
            .strip_prefix("=== \"")
            .or_else(|| trimmed.strip_prefix("\\=== \""));

        if let Some(rest) = maybe_tab {
            flush_tab(&mut output, &mut current_tab, &mut current_body);
            let title = rest.trim_end_matches('"').to_string();
            current_tab = Some(title);
        } else if current_tab.is_some() {
            current_body.push(line.to_string());
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }

    flush_tab(&mut output, &mut current_tab, &mut current_body);

    strip_material_table_wrappers(output.trim_end())
}

fn normalize_vitepress_blocks(input: &str) -> String {
    let custom_container_re = Regex::new(r":::\s*(\w+)\s*(?s)(.*?)\s*:::").expect("valid regex");
    custom_container_re
        .replace_all(input, |captures: &regex::Captures| {
            let style = captures
                .get(1)
                .map(|m| m.as_str().to_uppercase())
                .unwrap_or_else(|| "NOTE".to_string());
            let body = captures.get(2).map(|m| m.as_str()).unwrap_or("").trim();
            format!("> [!{style}]\n> {}\n", body.replace('\n', "\n> "))
        })
        .to_string()
}

fn normalize_nextra_blocks(input: &str) -> String {
    let callout_re =
        Regex::new(r#"<Callout[^>]*type="([^"]+)"[^>]*>(?s)(.*?)</Callout>"#).expect("valid regex");
    callout_re
        .replace_all(input, |captures: &regex::Captures| {
            let style = captures
                .get(1)
                .map(|m| m.as_str().to_uppercase())
                .unwrap_or_else(|| "NOTE".to_string());
            let body = captures.get(2).map(|m| m.as_str()).unwrap_or("").trim();
            format!("> [!{style}]\n> {}\n", body.replace('\n', "\n> "))
        })
        .to_string()
}

pub fn html_to_markdown(html: &str, framework: &Framework) -> String {
    html_to_markdown_with_selectors(html, framework, DEFAULT_CONTENT_SELECTORS)
}

pub fn html_to_markdown_with_selectors(
    html: &str,
    framework: &Framework,
    selectors: &[&str],
) -> String {
    let html_fragment = extract_main_html(html, selectors);
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

fn extract_main_html(html: &str, selectors: &[&str]) -> String {
    let document = Html::parse_document(html);

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

fn strip_inline_chrome(input: &str) -> String {
    let empty_link_re = Regex::new(r#"\[\s*\]\((?:#|/|https?://)[^)]+\)"#).expect("valid regex");
    let anchor_glyph_re = Regex::new(r#"\[[^\w\s]{1,2}\]\(#.*?\)"#).expect("valid regex");
    let llms_banner_re =
        Regex::new(r"Are you an LLM\? View /llms(?:-full)?\.txt[^\n]*").expect("valid regex");

    let output = empty_link_re.replace_all(input, "").to_string();
    let output = anchor_glyph_re.replace_all(&output, "").to_string();
    llms_banner_re.replace_all(&output, "").to_string()
}

fn strip_bootstrap_noise(input: &str) -> String {
    input
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.contains("localStorage.getItem(")
                && !trimmed.contains("document.querySelector(")
                && !trimmed.contains("classList.toggle(")
                && !trimmed.contains("window.matchMedia(")
                && !trimmed.contains("document.documentElement")
                && !trimmed.contains("__NEXT_DATA__")
                && !trimmed.starts_with("((a,b,c,d,e,f,g,h)=")
                && !trimmed.starts_with("try{document.querySelector")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_common_chrome(input: &str) -> String {
    input
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.contains("On this page")
                && !trimmed.contains("Table of contents")
                && !trimmed.contains("Was this page helpful?")
                && !trimmed.contains("Thanks for your feedback!")
                && !trimmed.contains("Help us improve this page")
                && !trimmed.contains("Edit this page")
                && !trimmed.contains("Skip to Content")
                && !trimmed.contains("Copy page")
                && !trimmed.contains("CTRL K")
                && !trimmed.contains("Continue reading")
                && !trimmed.contains("Last updated on")
                && !matches!(
                    trimmed,
                    "Edit this page"
                        | "Last updated"
                        | "Previous page"
                        | "Next page"
                        | "Files"
                        | "Top-Level Files"
                        | "Top-Level Folders"
                        | "Other Components"
                        | "Layout Components"
                        | "Content Components"
                        | "Search"
                        | "Types"
                        | "Functions"
                )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn trim_leading_preamble(input: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();

    for (index, line) in lines.iter().enumerate().take(80) {
        let trimmed = line.trim();
        let starts_hash_heading = trimmed.starts_with("# ");
        let has_setext_heading = index + 1 < lines.len() && is_setext_underline(lines[index + 1]);

        if starts_hash_heading || has_setext_heading {
            return lines[index..].join("\n");
        }
    }

    input.to_string()
}

fn is_setext_underline(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.len() >= 3 && trimmed.chars().all(|ch| ch == '=' || ch == '-')
}

fn strip_material_table_wrappers(input: &str) -> String {
    let table_wrapper_re =
        Regex::new(r#"(?s)<div class="[^"]*md-typeset__table[^"]*">(.*?)</div>"#)
            .expect("valid regex");
    table_wrapper_re.replace_all(input, "$1").to_string()
}

fn clean_whitespace(input: &str) -> String {
    input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}
