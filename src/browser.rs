use crate::error::{DocsiteError, Result};

#[cfg(feature = "browser")]
pub async fn fetch_rendered_html(url: &str, webdriver_url: Option<&str>) -> Result<String> {
    use fantoccini::ClientBuilder;

    let webdriver_url = webdriver_url
        .ok_or_else(|| DocsiteError::BrowserUnavailable("missing webdriver URL".to_string()))?;

    let client = ClientBuilder::rustls()
        .map_err(|error| DocsiteError::BrowserUnavailable(error.to_string()))?
        .connect(webdriver_url)
        .await
        .map_err(|error| DocsiteError::BrowserUnavailable(error.to_string()))?;

    client
        .goto(url)
        .await
        .map_err(|error| DocsiteError::BrowserUnavailable(error.to_string()))?;
    let html = client
        .source()
        .await
        .map_err(|error| DocsiteError::BrowserUnavailable(error.to_string()))?;
    client
        .close()
        .await
        .map_err(|error| DocsiteError::BrowserUnavailable(error.to_string()))?;

    Ok(html)
}

#[cfg(not(feature = "browser"))]
pub async fn fetch_rendered_html(_url: &str, _webdriver_url: Option<&str>) -> Result<String> {
    Err(DocsiteError::BrowserUnavailable(
        "compile with the `browser` feature to enable browser fallback".to_string(),
    ))
}
