use std::{path::Path, sync::Arc};

use reqwest::blocking::multipart;

#[derive(Debug)]
pub struct Discord {
    webhook_url: Arc<String>,
    client: reqwest::blocking::Client,
}

impl Discord {
    pub fn new(webhook_url: String) -> Self {
        Self {
            webhook_url: Arc::new(webhook_url),
            client: reqwest::blocking::Client::new(),
        }
    }

    pub fn upload(&self, filename: &str, filepath: &Path) -> anyhow::Result<()> {
        let form = multipart::Form::new()
            .text("content", filename.to_string())
            .text("username", "scans")
            .file("file", filepath)?;

        self.client
            .post(self.webhook_url.as_str())
            .multipart(form)
            .send()?
            .error_for_status()?;

        Ok(())
    }
}

impl Clone for Discord {
    fn clone(&self) -> Self {
        Self {
            webhook_url: Arc::clone(&self.webhook_url),
            client: self.client.clone(),
        }
    }
}
