use std::path::Path;

use anyhow::{bail, Context};
use reqwest::multipart;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

const BASE_URL: &str = "https://slack.com/api";

#[derive(Debug, Clone)]
pub struct Client {
    token: String,
    client: reqwest::Client,
}

impl Client {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            client: reqwest::Client::new(),
        }
    }

    async fn get<T, U>(&self, uri: &str, query_string: &T) -> anyhow::Result<U>
    where
        T: Serialize,
        U: DeserializeOwned,
    {
        let response = self
            .client
            .get(format!("{BASE_URL}{uri}"))
            .header("Authorization", format!("Bearer {}", self.token))
            .query(query_string)
            .send()
            .await?
            .error_for_status()?;

        let slack_response: Response<U> = response.json().await?;

        match slack_response.content {
            ResponseContent::Error { error } => bail!("slack api error: {error}"),
            ResponseContent::Success(content) => Ok(content),
        }
    }

    async fn post<T, U>(&self, uri: &str, body: &T) -> anyhow::Result<U>
    where
        T: Serialize,
        U: DeserializeOwned,
    {
        let response = self
            .client
            .post(format!("{BASE_URL}{uri}"))
            .header("Authorization", format!("Bearer {}", self.token))
            .json(body)
            .send()
            .await?
            .error_for_status()?;

        let slack_response: Response<U> = response.json().await?;

        match slack_response.content {
            ResponseContent::Error { error } => bail!("slack api error: {error}"),
            ResponseContent::Success(content) => Ok(content),
        }
    }

    pub async fn post_message(&self, message: PostMessageRequest<'_>) -> anyhow::Result<()> {
        let _response: PostMessageResponse = self.post("/chat.postMessage", &message).await?;

        Ok(())
    }

    pub async fn upload_file(&self, upload: UploadFileRequest<'_>) -> anyhow::Result<()> {
        let length = upload
            .path
            .metadata()
            .with_context(|| "cannot get file length")?
            .len();

        let req = GetUploadUrlExternalRequest {
            filename: upload.filename,
            length,
        };

        let res: GetUploadUrlExternalResponse =
            self.get("/files.getUploadURLExternal", &req).await?;

        let multipart_form = multipart::Form::new().file("file", upload.path).await?;

        self.client
            .post(&res.upload_url)
            .header("Authorization", format!("Bearer {}", self.token))
            .multipart(multipart_form)
            .send()
            .await?
            .error_for_status()?;

        let req = CompleteUploadExternalRequest {
            channel_id: upload.channel,
            files: vec![CompleteUploadExternalRequestFile { id: &res.file_id }],
        };

        let _res: CompleteUploadExternalResponse =
            self.post("/files.completeUploadExternal", &req).await?;

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct Response<T> {
    pub ok: bool,

    #[serde(flatten)]
    pub content: ResponseContent<T>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ResponseContent<T> {
    Error { error: String },
    Success(T),
}

#[derive(Debug, Serialize)]
pub struct PostMessageRequest<'a> {
    pub channel: &'a str,
    pub text: &'a str,
}

#[derive(Debug, Deserialize)]
struct PostMessageResponse {
    channel: String,
    ts: String,
}

#[derive(Debug)]
pub struct UploadFileRequest<'a> {
    pub channel: &'a str,
    pub filename: &'a str,
    pub path: &'a Path,
}

#[derive(Debug, Serialize)]
struct GetUploadUrlExternalRequest<'a> {
    filename: &'a str,
    length: u64,
}

#[derive(Debug, Deserialize)]
struct GetUploadUrlExternalResponse {
    upload_url: String,
    file_id: String,
}

#[derive(Debug, Serialize)]
struct CompleteUploadExternalRequest<'a> {
    channel_id: &'a str,
    files: Vec<CompleteUploadExternalRequestFile<'a>>,
}

#[derive(Debug, Serialize)]
struct CompleteUploadExternalRequestFile<'a> {
    id: &'a str,
}

#[derive(Debug, Deserialize)]
struct CompleteUploadExternalResponseFile {
    id: String,
    title: String,
}

#[derive(Debug, Deserialize)]
struct CompleteUploadExternalResponse {
    files: Vec<CompleteUploadExternalResponseFile>,
}
