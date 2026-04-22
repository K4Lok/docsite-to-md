use std::fs;

use docsite_to_md::{Framework, normalize::normalize_markdown};

fn fixture(path: &str) -> String {
    fs::read_to_string(format!(
        "/Users/k4lok/Development/OpenSources/docsite-to-md/tests/fixtures/{path}"
    ))
    .expect("fixture should exist")
}

#[test]
fn golden_docusaurus_normalization() {
    let input = ":::note Docusaurus note. :::\nOn this page\n";
    let output = normalize_markdown(input, &Framework::Docusaurus);
    assert_eq!(output, "> [!NOTE]\n> Docusaurus note.\n");
}

#[test]
fn golden_mkdocs_material_normalization() {
    let input = "=== \"Python\"\n\n`pip install`\n";
    let output = normalize_markdown(input, &Framework::MkDocsMaterial);
    assert!(output.contains("### Python"));
}

#[test]
fn golden_vitepress_normalization() {
    let input = ":::tip Helpful advice. :::\nTable of contents\nAre you an LLM? View /llms.txt for optimized Markdown documentation\n";
    let output = normalize_markdown(input, &Framework::VitePress);
    assert!(output.contains("[!TIP]"));
    assert!(!output.contains("Table of contents"));
    assert!(!output.contains("/llms.txt"));
}

#[test]
fn golden_nextra_normalization() {
    let input =
        "((a,b,c,d,e,f,g,h)=>{})\nCopy page\n<Callout type=\"warning\">Be careful.</Callout>\n";
    let output = normalize_markdown(input, &Framework::Nextra);
    assert!(output.contains("[!WARNING]"));
    assert!(!output.contains("Copy page"));
    assert!(!output.contains("((a,b,c,d,e,f,g,h)=>{})"));
}

#[test]
fn strips_anchor_glyph_noise() {
    let input = "Heading[¶](#heading)\n=======\n";
    let output = normalize_markdown(input, &Framework::Docusaurus);
    assert!(!output.contains("[¶](#heading)"));
}

#[test]
fn fixtures_exist_for_new_frameworks() {
    for path in [
        "docusaurus/root.html",
        "mkdocs/root.html",
        "vitepress/root.html",
        "nextra/root.html",
    ] {
        assert!(!fixture(path).is_empty());
    }
}
