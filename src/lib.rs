mod browser;
mod client;
mod error;
mod extractors;
pub mod normalize;
mod types;

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

use browser::fetch_rendered_html;
use client::HttpClient;
pub use error::{DocsiteError, Result};
use extractors::{detect_extractor, normalize_page_url};
pub use types::{
    BrowserOptions, BundleOptions, CrawlManifest, CrawlOptions, ExportOptions, ExportResult,
    ExportedPage, Framework, PageError, PageRef, SiteProfile, SourceFormat,
};

pub async fn detect_site(url: &str) -> Result<SiteProfile> {
    let client = HttpClient::new(2, 0)?;
    let response = client.fetch_text(url).await?;
    let (_, profile) = detect_extractor(&response)
        .ok_or_else(|| DocsiteError::DetectionFailed(url.to_string()))?;
    Ok(profile)
}

pub async fn crawl_site(url: &str, options: CrawlOptions) -> Result<CrawlManifest> {
    let client = HttpClient::new(options.retry_attempts, options.rate_limit_ms)?;
    let root_response = client.fetch_text(url).await?;
    let (extractor, site) = detect_extractor(&root_response)
        .ok_or_else(|| DocsiteError::DetectionFailed(url.to_string()))?;
    let base_url = Url::parse(&root_response.final_url)?;
    let root_normalized = normalize_page_url(&base_url);
    let root_markdown = extractor.preferred_markdown_url(&base_url);

    let mut pages_by_url = HashMap::new();
    pages_by_url.insert(
        root_normalized.clone(),
        PageRef {
            url: base_url.to_string().trim_end_matches('/').to_string(),
            normalized_url: root_normalized.clone(),
            title: normalize::extract_title_from_html(&root_response.body),
            depth: 0,
            parent_url: None,
            markdown_url: root_markdown,
        },
    );

    let mut queue = VecDeque::from([base_url.to_string().trim_end_matches('/').to_string()]);
    let mut visited = HashSet::new();
    let mut skipped_urls = Vec::new();

    while let Some(current_url) = queue.pop_front() {
        if !visited.insert(current_url.clone()) {
            continue;
        }

        if let Some(max_pages) = options.max_pages {
            if pages_by_url.len() >= max_pages
                && current_url != base_url.to_string().trim_end_matches('/')
            {
                skipped_urls.push(current_url);
                break;
            }
        }

        let response = match client.fetch_text(&current_url).await {
            Ok(response) => response,
            Err(_) => {
                skipped_urls.push(current_url);
                continue;
            }
        };

        let current = Url::parse(&response.final_url)?;
        for mut page in extractor.discover_links(&current, &response.body, &options) {
            if page.markdown_url.is_none() {
                page.markdown_url = extractor.preferred_markdown_url(&Url::parse(&page.url)?);
            }

            if pages_by_url.contains_key(&page.normalized_url) {
                continue;
            }

            if let Some(max_pages) = options.max_pages {
                if pages_by_url.len() >= max_pages {
                    skipped_urls.push(page.url.clone());
                    continue;
                }
            }

            queue.push_back(page.url.clone());
            pages_by_url.insert(page.normalized_url.clone(), page);
        }
    }

    let mut pages: Vec<PageRef> = pages_by_url.into_values().collect();
    pages.sort_by(|left, right| left.url.cmp(&right.url));

    Ok(CrawlManifest {
        site,
        generated_at_epoch: now_epoch(),
        pages,
        skipped_urls,
    })
}

pub async fn export_site(url: &str, options: ExportOptions) -> Result<ExportResult> {
    let manifest = crawl_site(url, options.crawl.clone()).await?;
    tokio::fs::create_dir_all(&options.output_dir).await?;

    let client = HttpClient::new(options.crawl.retry_attempts, options.crawl.rate_limit_ms)?;

    let previous_manifest = if options.resume {
        load_previous_manifest(&options.output_dir.join("manifest.json")).await?
    } else {
        HashMap::new()
    };

    let site = manifest.site.clone();
    let pages = manifest.pages.clone();
    let browser = options.browser.clone();
    let output_root = options.output_dir.clone();

    let results = stream::iter(pages.into_iter())
        .map(|page| {
            let client = client.clone();
            let site = site.clone();
            let browser = browser.clone();
            let output_root = output_root.clone();
            let resume_entry = previous_manifest.get(&page.url).cloned();
            async move {
                export_single_page(&client, &site, page, &output_root, &browser, resume_entry).await
            }
        })
        .buffer_unordered(options.crawl.max_concurrency.max(1))
        .collect::<Vec<_>>()
        .await;

    let mut exported_pages = Vec::new();
    let mut errors = Vec::new();
    let mut seen_hashes = HashMap::new();
    let mut bundle_entries = Vec::new();
    let mut successful_results = Vec::new();

    for result in results {
        match result {
            Ok(success) => successful_results.push(success),
            Err(error) => errors.push(error),
        }
    }

    successful_results.sort_by(|left, right| left.0.page.url.cmp(&right.0.page.url));

    for (mut exported, content) in successful_results {
        if let Some(hash) = exported.content_hash.clone() {
            if let Some(existing) = seen_hashes.get(&hash).cloned() {
                exported.skipped = true;
                exported.duplicate_of = Some(existing);
            } else if !exported.skipped {
                seen_hashes.insert(hash, exported.output_file.clone());
                bundle_entries.push((exported.title.clone(), exported.page.url.clone(), content));
            }
        }
        exported_pages.push(exported);
    }

    exported_pages.sort_by(|left, right| left.page.url.cmp(&right.page.url));

    let manifest_path = output_root.join("manifest.json");
    let links_path = output_root.join("links.txt");

    let serialized_manifest = SerializableManifest {
        site: manifest.site.clone(),
        generated_at_epoch: now_epoch(),
        pages: exported_pages.clone(),
        errors: errors.clone(),
    };

    tokio::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&serialized_manifest)?,
    )
    .await?;
    tokio::fs::write(
        &links_path,
        exported_pages
            .iter()
            .map(|page| page.page.url.as_str())
            .collect::<Vec<_>>()
            .join("\n")
            + "\n",
    )
    .await?;

    let bundle_path = if let Some(bundle_output) = options.bundle_output.clone() {
        let bundle_content = render_bundle(&bundle_entries);
        tokio::fs::write(&bundle_output, bundle_content).await?;
        Some(bundle_output)
    } else {
        None
    };

    Ok(ExportResult {
        site: manifest.site,
        output_dir: output_root,
        manifest_path,
        links_path,
        bundle_path,
        pages: exported_pages,
        errors,
    })
}

pub async fn bundle_site(url: &str, options: BundleOptions) -> Result<PathBuf> {
    let tempdir = tempfile::tempdir().map_err(DocsiteError::Io)?;
    let export_options = ExportOptions {
        output_dir: tempdir.path().join("tree"),
        crawl: options.crawl,
        resume: false,
        bundle_output: Some(options.output_file.clone()),
        browser: options.browser,
    };

    let result = export_site(url, export_options).await?;
    result
        .bundle_path
        .ok_or_else(|| DocsiteError::DetectionFailed("bundle path missing".to_string()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SerializableManifest {
    site: SiteProfile,
    generated_at_epoch: u64,
    pages: Vec<ExportedPage>,
    errors: Vec<PageError>,
}

async fn export_single_page(
    client: &HttpClient,
    site: &SiteProfile,
    page: PageRef,
    output_root: &Path,
    browser: &BrowserOptions,
    resume_entry: Option<ExportedPage>,
) -> std::result::Result<(ExportedPage, String), PageError> {
    if let Some(existing) = resume_entry {
        let path = output_root.join(&existing.output_file);
        if path.exists() {
            return Ok((existing, String::new()));
        }
    }

    let page_url = match Url::parse(&page.url) {
        Ok(url) => url,
        Err(error) => {
            return Err(PageError {
                url: page.url,
                phase: "parse".to_string(),
                message: error.to_string(),
            });
        }
    };

    let fetch_targets = preferred_targets(site, &page, &page_url);
    let mut last_error = None;

    for (target, source_format) in fetch_targets {
        match client.fetch_text(&target).await {
            Ok(response) => {
                let extractor = find_extractor_for_site(site).map_err(|error| PageError {
                    url: page.url.clone(),
                    phase: "extractor".to_string(),
                    message: error.to_string(),
                })?;
                let extraction =
                    extractor
                        .extract(&page, &response)
                        .map_err(|error| PageError {
                            url: page.url.clone(),
                            phase: "extract".to_string(),
                            message: error.to_string(),
                        })?;

                if browser.enabled && extractor.should_fallback_to_browser(&extraction) {
                    last_error =
                        Some("static extraction too weak; trying browser fallback".to_string());
                    continue;
                }

                let markdown = extraction.markdown;
                let output_file = output_path_for_page(output_root, &page_url);
                if let Some(parent) = output_file.parent() {
                    tokio::fs::create_dir_all(parent)
                        .await
                        .map_err(|error| PageError {
                            url: page.url.clone(),
                            phase: "write".to_string(),
                            message: error.to_string(),
                        })?;
                }
                tokio::fs::write(&output_file, &markdown)
                    .await
                    .map_err(|error| PageError {
                        url: page.url.clone(),
                        phase: "write".to_string(),
                        message: error.to_string(),
                    })?;

                let content_hash = sha256(&markdown);
                return Ok((
                    ExportedPage {
                        page,
                        output_file: relative_output_path(output_root, &output_file),
                        title: extraction.title,
                        source_format: if matches!(source_format, SourceFormat::BrowserHtml) {
                            SourceFormat::BrowserHtml
                        } else {
                            extraction.source_format
                        },
                        used_browser_fallback: false,
                        content_hash: Some(content_hash),
                        skipped: false,
                        duplicate_of: None,
                    },
                    markdown,
                ));
            }
            Err(error) => {
                last_error = Some(error.to_string());
            }
        }
    }

    if browser.enabled {
        match fetch_rendered_html(&page.url, browser.webdriver_url.as_deref()).await {
            Ok(body) => {
                let markdown = normalize::html_to_markdown(&body, &site.framework);
                let output_file = output_path_for_page(output_root, &page_url);
                if let Some(parent) = output_file.parent() {
                    tokio::fs::create_dir_all(parent)
                        .await
                        .map_err(|error| PageError {
                            url: page.url.clone(),
                            phase: "write".to_string(),
                            message: error.to_string(),
                        })?;
                }
                tokio::fs::write(&output_file, &markdown)
                    .await
                    .map_err(|error| PageError {
                        url: page.url.clone(),
                        phase: "write".to_string(),
                        message: error.to_string(),
                    })?;

                return Ok((
                    ExportedPage {
                        page,
                        output_file: relative_output_path(output_root, &output_file),
                        title: None,
                        source_format: SourceFormat::BrowserHtml,
                        used_browser_fallback: true,
                        content_hash: Some(sha256(&markdown)),
                        skipped: false,
                        duplicate_of: None,
                    },
                    markdown,
                ));
            }
            Err(error) => {
                last_error = Some(error.to_string());
            }
        }
    }

    Err(PageError {
        url: page.url,
        phase: "fetch".to_string(),
        message: last_error.unwrap_or_else(|| "unknown failure".to_string()),
    })
}

fn preferred_targets(
    site: &SiteProfile,
    page: &PageRef,
    page_url: &Url,
) -> Vec<(String, SourceFormat)> {
    let mut targets = Vec::new();

    if site.supports_markdown_endpoints {
        if let Some(markdown_url) = &page.markdown_url {
            targets.push((markdown_url.clone(), SourceFormat::Markdown));
        }
    }

    targets.push((page_url.to_string(), SourceFormat::Html));
    targets
}

fn find_extractor_for_site(site: &SiteProfile) -> Result<Box<dyn extractors::Extractor>> {
    for extractor in extractors::default_extractors() {
        if extractor.name() == site.extractor {
            return Ok(extractor);
        }
    }
    Err(DocsiteError::DetectionFailed(site.extractor.clone()))
}

async fn load_previous_manifest(path: &Path) -> Result<HashMap<String, ExportedPage>> {
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let content = tokio::fs::read_to_string(path).await?;
    let manifest: SerializableManifest = serde_json::from_str(&content)?;
    Ok(manifest
        .pages
        .into_iter()
        .map(|page| (page.page.url.clone(), page))
        .collect())
}

fn output_path_for_page(output_root: &Path, page_url: &Url) -> PathBuf {
    if page_url.path().trim_matches('/').is_empty() {
        return output_root.join("index.md");
    }

    output_root.join(format!("{}.md", page_url.path().trim_start_matches('/')))
}

fn relative_output_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

fn render_bundle(entries: &[(Option<String>, String, String)]) -> String {
    let mut output = String::from("# Documentation Bundle\n\n");
    for (title, url, markdown) in entries {
        let heading = title.clone().unwrap_or_else(|| url.clone());
        output.push_str(&format!("## {heading}\n\n"));
        output.push_str(&format!("Source: {url}\n\n"));
        output.push_str(markdown.trim());
        output.push_str("\n\n");
    }
    output
}

fn sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should move forward")
        .as_secs()
}
