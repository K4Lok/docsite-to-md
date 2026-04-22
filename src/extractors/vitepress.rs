use url::Url;

use crate::client::FetchResult;
use crate::extractors::{Extractor, discover_nav_links};
use crate::types::{CrawlOptions, Framework, PageRef};

pub struct VitePress;

const VITEPRESS_NAV_SELECTORS: &[&str] = &[
    ".VPSidebar a[href]",
    ".VPNav a[href]",
    "aside a[href]",
    "nav a[href]",
    "main a[href]",
    "a[href]",
];

const VITEPRESS_CONTENT_SELECTORS: &[&str] = &[".VPDoc .content", ".vp-doc", "main", "body"];

impl Extractor for VitePress {
    fn framework(&self) -> Framework {
        Framework::VitePress
    }

    fn name(&self) -> &'static str {
        "VitePress"
    }

    fn detect(&self, response: &FetchResult) -> Option<Vec<String>> {
        let body = response.body.to_lowercase();
        let mut signals = Vec::new();

        if body.contains("vitepress") {
            signals.push("body:vitepress".to_string());
        }
        if body.contains("vpsidebar") || body.contains("vpdoc") || body.contains("vpnav") {
            signals.push("body:vp-layout".to_string());
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
        VITEPRESS_NAV_SELECTORS
    }

    fn content_selectors(&self) -> &'static [&'static str] {
        VITEPRESS_CONTENT_SELECTORS
    }

    fn discover_links(&self, base_url: &Url, html: &str, options: &CrawlOptions) -> Vec<PageRef> {
        let mut pages = discover_nav_links(base_url, html, options, self.nav_selectors());
        pages.sort_by(|left, right| {
            vitepress_link_priority(&left.url)
                .cmp(&vitepress_link_priority(&right.url))
                .then_with(|| left.url.cmp(&right.url))
        });
        pages
    }

    fn browser_fallback_recommended(&self) -> bool {
        true
    }

    fn preferred_markdown_url(&self, _page_url: &Url) -> Option<String> {
        None
    }
}

fn vitepress_link_priority(url: &str) -> (u8, usize) {
    let parsed = match Url::parse(url) {
        Ok(value) => value,
        Err(_) => return (1, usize::MAX),
    };
    let segments = parsed
        .path_segments()
        .map(|parts| {
            parts
                .filter(|segment| !segment.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let first = segments.first().copied().unwrap_or("");
    let is_core_docs = matches!(first, "" | "guide" | "api" | "reference" | "config");
    let is_locale = is_locale_segment(first);
    let bucket = if is_core_docs {
        0
    } else if is_locale {
        2
    } else {
        1
    };

    (bucket, segments.len())
}

fn is_locale_segment(segment: &str) -> bool {
    let normalized = segment.to_ascii_lowercase();
    let parts = normalized.split('-').collect::<Vec<_>>();
    match parts.as_slice() {
        [single] => single.len() == 2 && single.chars().all(|ch| ch.is_ascii_alphabetic()),
        [language, region] => {
            language.len() == 2
                && region.len() == 2
                && language.chars().all(|ch| ch.is_ascii_alphabetic())
                && region.chars().all(|ch| ch.is_ascii_alphabetic())
        }
        _ => false,
    }
}
