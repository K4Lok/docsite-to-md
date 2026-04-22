use url::Url;

use crate::client::FetchResult;
use crate::extractors::{Extractor, discover_nav_links};
use crate::types::{CrawlOptions, Framework, PageRef};

pub struct GitBookModern;
pub struct GitBookClassic;

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

    fn browser_fallback_recommended(&self) -> bool {
        true
    }

    fn discover_links(&self, base_url: &Url, html: &str, options: &CrawlOptions) -> Vec<PageRef> {
        discover_nav_links(base_url, html, options)
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

    fn discover_links(&self, base_url: &Url, html: &str, options: &CrawlOptions) -> Vec<PageRef> {
        discover_nav_links(base_url, html, options)
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
