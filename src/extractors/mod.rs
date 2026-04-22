mod generic;
mod gitbook;

use scraper::{Html, Selector};
use url::Url;

use crate::client::FetchResult;
use crate::error::Result;
use crate::normalize::{extract_title_from_html, html_to_markdown, normalize_markdown};
use crate::types::{CrawlOptions, Framework, PageRef, SiteProfile, SourceFormat};

pub use generic::GenericDocsFallback;
pub use gitbook::{GitBookClassic, GitBookModern};

#[derive(Debug, Clone)]
pub struct ExtractionResult {
    pub markdown: String,
    pub title: Option<String>,
    pub source_format: SourceFormat,
}

pub trait Extractor: Send + Sync {
    fn framework(&self) -> Framework;
    fn name(&self) -> &'static str;
    fn detect(&self, response: &FetchResult) -> Option<Vec<String>>;
    fn supports_markdown_endpoints(&self) -> bool;
    fn browser_fallback_recommended(&self) -> bool {
        false
    }
    fn discover_links(&self, base_url: &Url, html: &str, options: &CrawlOptions) -> Vec<PageRef>;
    fn preferred_markdown_url(&self, page_url: &Url) -> Option<String>;

    fn extract(&self, page: &PageRef, response: &FetchResult) -> Result<ExtractionResult> {
        let content_type = response.content_type.clone().unwrap_or_default();
        let is_markdown = content_type.contains("markdown")
            || response.body.trim_start().starts_with('#')
            || response.body.trim_start().starts_with("---");

        if is_markdown {
            Ok(ExtractionResult {
                markdown: normalize_markdown(&response.body, &self.framework()),
                title: page.title.clone(),
                source_format: SourceFormat::Markdown,
            })
        } else {
            Ok(ExtractionResult {
                markdown: html_to_markdown(&response.body, &self.framework()),
                title: extract_title_from_html(&response.body).or_else(|| page.title.clone()),
                source_format: SourceFormat::Html,
            })
        }
    }
}

pub fn default_extractors() -> Vec<Box<dyn Extractor>> {
    vec![
        Box::new(GitBookModern),
        Box::new(GitBookClassic),
        Box::new(GenericDocsFallback),
    ]
}

pub fn detect_extractor(response: &FetchResult) -> Option<(Box<dyn Extractor>, SiteProfile)> {
    for extractor in default_extractors() {
        if let Some(signals) = extractor.detect(response) {
            let profile = SiteProfile {
                url: response.final_url.clone(),
                framework: extractor.framework(),
                extractor: extractor.name().to_string(),
                supports_markdown_endpoints: extractor.supports_markdown_endpoints(),
                browser_fallback_recommended: extractor.browser_fallback_recommended(),
                detection_signals: signals,
            };

            return Some((extractor, profile));
        }
    }

    None
}

pub fn normalize_page_url(url: &Url) -> String {
    let mut normalized = format!(
        "{}://{}{}",
        url.scheme(),
        url.host_str().unwrap_or(""),
        url.path()
    );
    if let Some(port) = url.port() {
        normalized = format!("{normalized}:{port}");
    }
    normalized.trim_end_matches('/').to_string()
}

pub fn scope_prefix(base_url: &Url, options: &CrawlOptions) -> String {
    options
        .scope_prefix
        .clone()
        .unwrap_or_else(|| base_url.path().trim_end_matches('/').to_string())
}

pub fn should_include_url(candidate: &Url, base_url: &Url, options: &CrawlOptions) -> bool {
    if candidate.domain() != base_url.domain() {
        return false;
    }

    let path = candidate.path();
    if path.contains("/cdn-cgi/") {
        return false;
    }
    if [".png", ".jpg", ".jpeg", ".gif", ".svg", ".pdf", ".zip"]
        .iter()
        .any(|suffix| path.ends_with(suffix))
    {
        return false;
    }

    let scope = scope_prefix(base_url, options);
    if !scope.is_empty() && scope != "/" && !path.starts_with(&scope) {
        return false;
    }

    !options
        .excludes
        .iter()
        .any(|pattern| path.contains(pattern) || candidate.as_str().contains(pattern))
}

pub fn discover_nav_links(base_url: &Url, html: &str, options: &CrawlOptions) -> Vec<PageRef> {
    let document = Html::parse_document(html);
    let nav_selectors = [
        "aside a[href]",
        "nav a[href]",
        "[data-testid*='sidebar'] a[href]",
        "#SUMMARY a[href]",
        "main a[href]",
        "a[href]",
    ];
    let mut pages = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for selector in nav_selectors {
        let selector = Selector::parse(selector).expect("valid selector");
        let mut local_count = 0usize;
        for anchor in document.select(&selector) {
            let href = match anchor.value().attr("href") {
                Some(value) => value,
                None => continue,
            };
            let joined = match base_url.join(href) {
                Ok(value) => value,
                Err(_) => continue,
            };
            let mut candidate = joined;
            candidate.set_fragment(None);
            candidate.set_query(None);

            if !should_include_url(&candidate, base_url, options) {
                continue;
            }

            let normalized_url = normalize_page_url(&candidate);
            if !seen.insert(normalized_url.clone()) {
                continue;
            }

            let title = clean_link_title(&anchor.text().collect::<Vec<_>>().join(" "));
            let title = if title.is_empty() { None } else { Some(title) };

            let depth = candidate
                .path_segments()
                .map(|segments| segments.filter(|segment| !segment.is_empty()).count())
                .unwrap_or(0);

            pages.push(PageRef {
                url: candidate.to_string().trim_end_matches('/').to_string(),
                normalized_url,
                title,
                depth,
                parent_url: None,
                markdown_url: None,
            });
            local_count += 1;
        }

        if local_count > 0 {
            break;
        }
    }

    pages.sort_by(|left, right| left.url.cmp(&right.url));
    pages
}

fn clean_link_title(input: &str) -> String {
    input
        .split_whitespace()
        .filter(|token| !matches!(*token, "chevron-right" | "arrow-right" | "external-link"))
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}
