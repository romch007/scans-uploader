use std::{path::Path, sync::Arc};

use eyre::Context;
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

    pub fn upload(&self, group: &str, filename: &str, filepath: &Path) -> eyre::Result<()> {
        let form = multipart::Form::new()
            .text(
                "content",
                format!("Received '{filename}' from folder '{group}'"),
            )
            .text("username", "scans")
            .file("file", filepath)
            .wrap_err("cannot add file to multipart form")?;

        self.client
            .post(self.webhook_url.as_str())
            .multipart(form)
            .send()
            .wrap_err("cannot send http request")?
            .error_for_status()
            .wrap_err("server returned an error")?;

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
