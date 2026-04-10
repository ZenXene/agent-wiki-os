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
        Ok(markdown)
    }
}
