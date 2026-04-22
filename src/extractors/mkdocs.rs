use url::Url;

use crate::client::FetchResult;
use crate::extractors::Extractor;
use crate::types::Framework;

pub struct MkDocsMaterial;

const MKDOCS_NAV_SELECTORS: &[&str] = &[
    ".md-nav a[href]",
    ".md-sidebar a[href]",
    "nav a[href]",
    "main a[href]",
    "a[href]",
];

const MKDOCS_CONTENT_SELECTORS: &[&str] = &[
    ".md-content article",
    ".md-content",
    "article",
    "main",
    "body",
];

impl Extractor for MkDocsMaterial {
    fn framework(&self) -> Framework {
        Framework::MkDocsMaterial
    }

    fn name(&self) -> &'static str {
        "MkDocsMaterial"
    }

    fn detect(&self, response: &FetchResult) -> Option<Vec<String>> {
        let body = response.body.to_lowercase();
        let mut signals = Vec::new();

        if body.contains("mkdocs") {
            signals.push("body:mkdocs".to_string());
        }
        if body.contains("material for mkdocs") || body.contains("mkdocs-material") {
            signals.push("body:material-for-mkdocs".to_string());
        }
        if body.contains("md-sidebar") || body.contains("md-content") {
            signals.push("body:md-layout".to_string());
        }

        if signals.iter().any(|signal| signal.contains("material")) {
            Some(signals)
        } else {
            None
        }
    }

    fn supports_markdown_endpoints(&self) -> bool {
        false
    }

    fn nav_selectors(&self) -> &'static [&'static str] {
        MKDOCS_NAV_SELECTORS
    }

    fn content_selectors(&self) -> &'static [&'static str] {
        MKDOCS_CONTENT_SELECTORS
    }

    fn preferred_markdown_url(&self, _page_url: &Url) -> Option<String> {
        None
    }
}
