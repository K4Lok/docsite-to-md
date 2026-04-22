use url::Url;

use crate::client::FetchResult;
use crate::extractors::Extractor;
use crate::types::Framework;

pub struct Docusaurus;

const DOCUSAURUS_NAV_SELECTORS: &[&str] = &[
    "nav[aria-label='Docs sidebar'] a[href]",
    ".theme-doc-sidebar-menu a[href]",
    "aside nav a[href]",
    "nav a[href]",
    "main a[href]",
    "a[href]",
];

const DOCUSAURUS_CONTENT_SELECTORS: &[&str] = &[
    "main article",
    ".theme-doc-markdown",
    "article",
    "main",
    "body",
];

impl Extractor for Docusaurus {
    fn framework(&self) -> Framework {
        Framework::Docusaurus
    }

    fn name(&self) -> &'static str {
        "Docusaurus"
    }

    fn detect(&self, response: &FetchResult) -> Option<Vec<String>> {
        let body = response.body.to_lowercase();
        let mut signals = Vec::new();

        if body.contains("__docusaurus") {
            signals.push("body:__docusaurus".to_string());
        }
        if body.contains("docusaurus") {
            signals.push("body:docusaurus".to_string());
        }
        if body.contains("theme-doc-sidebar") || body.contains("theme-doc-markdown") {
            signals.push("body:theme-doc".to_string());
        }

        if signals.is_empty() {
            None
        } else {
            Some(signals)
        }
    }

    fn supports_markdown_endpoints(&self) -> bool {
        false
    }

    fn nav_selectors(&self) -> &'static [&'static str] {
        DOCUSAURUS_NAV_SELECTORS
    }

    fn content_selectors(&self) -> &'static [&'static str] {
        DOCUSAURUS_CONTENT_SELECTORS
    }

    fn preferred_markdown_url(&self, _page_url: &Url) -> Option<String> {
        None
    }
}
