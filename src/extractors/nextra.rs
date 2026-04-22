use url::Url;

use crate::client::FetchResult;
use crate::extractors::{ExtractionResult, Extractor};
use crate::types::{Framework, SourceFormat};

pub struct Nextra;

const NEXTRA_NAV_SELECTORS: &[&str] = &[
    "[data-nextra-sidebar] a[href]",
    ".nextra-sidebar-container a[href]",
    "aside a[href]",
    "nav a[href]",
    "main a[href]",
    "a[href]",
];

const NEXTRA_CONTENT_SELECTORS: &[&str] =
    &["main article", ".nextra-content", "article", "main", "body"];

impl Extractor for Nextra {
    fn framework(&self) -> Framework {
        Framework::Nextra
    }

    fn name(&self) -> &'static str {
        "Nextra"
    }

    fn detect(&self, response: &FetchResult) -> Option<Vec<String>> {
        let body = response.body.to_lowercase();
        let mut signals = Vec::new();

        if body.contains("nextra") {
            signals.push("body:nextra".to_string());
        }
        if body.contains("nextra-theme-docs") {
            signals.push("body:nextra-theme-docs".to_string());
        }
        if body.contains("__next_data__") || body.contains("__next") {
            signals.push("body:next-runtime".to_string());
        }

        if signals.iter().any(|signal| signal.contains("nextra")) {
            Some(signals)
        } else {
            None
        }
    }

    fn supports_markdown_endpoints(&self) -> bool {
        false
    }

    fn nav_selectors(&self) -> &'static [&'static str] {
        NEXTRA_NAV_SELECTORS
    }

    fn content_selectors(&self) -> &'static [&'static str] {
        NEXTRA_CONTENT_SELECTORS
    }

    fn browser_fallback_recommended(&self) -> bool {
        true
    }

    fn preferred_markdown_url(&self, _page_url: &Url) -> Option<String> {
        None
    }

    fn should_fallback_to_browser(&self, extraction: &ExtractionResult) -> bool {
        let markdown = extraction.markdown.trim();
        let line_count = markdown.lines().count();
        let looks_hydration_only = markdown.contains("Loading")
            || markdown.contains("hydration")
            || markdown.contains("Enable JavaScript")
            || markdown.contains("__NEXT_DATA__");
        let too_thin = markdown.len() < 24 || line_count < 2;

        extraction.source_format == SourceFormat::Html && (looks_hydration_only || too_thin)
    }
}
