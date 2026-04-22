use url::Url;

use crate::client::FetchResult;
use crate::extractors::{Extractor, discover_nav_links};
use crate::types::{CrawlOptions, Framework, PageRef};

pub struct GenericDocsFallback;

impl Extractor for GenericDocsFallback {
    fn framework(&self) -> Framework {
        Framework::GenericDocsFallback
    }

    fn name(&self) -> &'static str {
        "GenericDocsFallback"
    }

    fn detect(&self, _response: &FetchResult) -> Option<Vec<String>> {
        Some(vec!["fallback:same-domain-html".to_string()])
    }

    fn supports_markdown_endpoints(&self) -> bool {
        false
    }

    fn browser_fallback_recommended(&self) -> bool {
        true
    }

    fn discover_links(&self, base_url: &Url, html: &str, options: &CrawlOptions) -> Vec<PageRef> {
        discover_nav_links(base_url, html, options)
    }

    fn preferred_markdown_url(&self, _page_url: &Url) -> Option<String> {
        None
    }
}
