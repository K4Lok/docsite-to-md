use url::Url;

use crate::client::FetchResult;
use crate::extractors::Extractor;
use crate::types::Framework;

pub struct GitBookModern;
pub struct GitBookClassic;

const GITBOOK_NAV_SELECTORS: &[&str] = &[
    "aside a[href]",
    "nav a[href]",
    "[data-testid*='sidebar'] a[href]",
    "#SUMMARY a[href]",
    "main a[href]",
    "a[href]",
];

const GITBOOK_CONTENT_SELECTORS: &[&str] = &[
    "main",
    "[role='main']",
    "article",
    ".markdown",
    ".page",
    "body",
];

impl Extractor for GitBookModern {
    fn framework(&self) -> Framework {
        Framework::GitBookModern
    }

    fn name(&self) -> &'static str {
        "GitBookModern"
    }

    fn detect(&self, response: &FetchResult) -> Option<Vec<String>> {
        let body = response.body.to_lowercase();
        let content_type = response
            .content_type
            .clone()
            .unwrap_or_default()
            .to_lowercase();
        let mut signals = Vec::new();

        if body.contains("powered by gitbook") {
            signals.push("body:powered-by-gitbook".to_string());
        }
        if body.contains("__next") || body.contains("data-dpl-id") {
            signals.push("body:next-runtime".to_string());
        }
        if content_type.contains("text/html") && response.final_url.contains("gitbook") {
            signals.push("url:gitbook".to_string());
        }

        if signals.iter().any(|signal| signal.contains("gitbook")) && body.contains("__next")
            || body.contains("data-dpl-id")
        {
            Some(signals)
        } else {
            None
        }
    }

    fn supports_markdown_endpoints(&self) -> bool {
        true
    }

    fn nav_selectors(&self) -> &'static [&'static str] {
        GITBOOK_NAV_SELECTORS
    }

    fn content_selectors(&self) -> &'static [&'static str] {
        GITBOOK_CONTENT_SELECTORS
    }

    fn browser_fallback_recommended(&self) -> bool {
        true
    }

    fn preferred_markdown_url(&self, page_url: &Url) -> Option<String> {
        let clean = page_url.to_string().trim_end_matches('/').to_string();
        if page_url.path().trim_matches('/').is_empty() {
            Some(format!("{clean}/.md"))
        } else {
            Some(format!("{clean}.md"))
        }
    }
}

impl Extractor for GitBookClassic {
    fn framework(&self) -> Framework {
        Framework::GitBookClassic
    }

    fn name(&self) -> &'static str {
        "GitBookClassic"
    }

    fn detect(&self, response: &FetchResult) -> Option<Vec<String>> {
        let body = response.body.to_lowercase();
        let mut signals = Vec::new();

        if body.contains("data-gitbook") {
            signals.push("body:data-gitbook".to_string());
        }
        if body.contains("gitbook-plugin") || body.contains("gitbook") {
            signals.push("body:gitbook".to_string());
        }

        if signals.is_empty() {
            None
        } else {
            Some(signals)
        }
    }

    fn supports_markdown_endpoints(&self) -> bool {
        true
    }

    fn nav_selectors(&self) -> &'static [&'static str] {
        GITBOOK_NAV_SELECTORS
    }

    fn content_selectors(&self) -> &'static [&'static str] {
        GITBOOK_CONTENT_SELECTORS
    }

    fn preferred_markdown_url(&self, page_url: &Url) -> Option<String> {
        let clean = page_url.to_string().trim_end_matches('/').to_string();
        if page_url.path().trim_matches('/').is_empty() {
            Some(format!("{clean}/README.md"))
        } else {
            Some(format!("{clean}.md"))
        }
    }
}
