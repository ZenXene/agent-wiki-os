use reqwest;

pub struct WebAdapter {
    pub url: String,
}

impl WebAdapter {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
        }
    }

    pub async fn fetch(&self) -> anyhow::Result<String> {
        let jina_url = format!("https://r.jina.ai/{}", self.url);
        let client = reqwest::Client::new();
        let res = client
            .get(&jina_url)
            .header("Accept", "text/plain")
            .send()
            .await?;
            
        if !res.status().is_success() {
            anyhow::bail!("Failed to fetch URL: HTTP {}", res.status());
        }
        
        let markdown = res.text().await?;
        
        // Clean up markdown to save context (remove base64 images, redundant CSS/scripts if Jina didn't fully clean)
        let mut clean_markdown = String::with_capacity(markdown.len());
        for line in markdown.lines() {
            if line.contains("data:image/") && line.len() > 200 {
                // Skip base64 image data
                continue;
            }
            if line.contains("<style>") || line.contains("</style>") || line.contains("<script>") {
                continue;
            }
            clean_markdown.push_str(line);
            clean_markdown.push('\n');
        }

        Ok(clean_markdown)
    }
}
