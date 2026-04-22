use url::Url;

use crate::client::FetchResult;
use crate::extractors::Extractor;
use crate::types::Framework;

pub struct GenericDocsFallback;

const GENERIC_NAV_SELECTORS: &[&str] = &["aside a[href]", "nav a[href]", "main a[href]", "a[href]"];
const GENERIC_CONTENT_SELECTORS: &[&str] =
    &["main", "[role='main']", "article", ".content", "body"];

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

    fn nav_selectors(&self) -> &'static [&'static str] {
        GENERIC_NAV_SELECTORS
    }

    fn content_selectors(&self) -> &'static [&'static str] {
        GENERIC_CONTENT_SELECTORS
    }

    fn browser_fallback_recommended(&self) -> bool {
        true
    }

    fn preferred_markdown_url(&self, _page_url: &Url) -> Option<String> {
        None
    }
}
