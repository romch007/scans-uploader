mod slack;

use anyhow::{anyhow, Context};
use notify::{
    event::{AccessKind, AccessMode},
    EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    sync::mpsc,
};

fn main() {
    tracing_subscriber::fmt::init();

    let dir_mapping = env::var("DIR_MAPPING").expect("DIR_MAPPING not provided");
    let dir_mapping: HashMap<String, String> =
        serde_json::from_str(&dir_mapping).expect("invalid mapping");

    tracing::info!("mappings:");
    for (dir, channel_id) in &dir_mapping {
        tracing::info!("  {dir} -> {channel_id}");
    }

    let watch_dir: PathBuf = env::var_os("WATCH_DIR")
        .expect("WATCH_DIR not provided")
        .into();

    let watch_dir = fs::canonicalize(watch_dir).expect("cannot canonicalize path");

    let (tx, rx) = mpsc::channel();

    let mut watcher = notify::recommended_watcher(tx).expect("cannot create watcher");

    watcher
        .watch(&watch_dir, RecursiveMode::Recursive)
        .expect("cannot watch directory");

    tracing::info!(
        "watching {} using {:?}",
        watch_dir.display(),
        RecommendedWatcher::kind()
    );

    let slack_token = env::var("SLACK_OAUTH_TOKEN").expect("SLACK_OAUTH_TOKEN not provided");

    let slack_client = slack::Client::new(slack_token);

    for res in rx {
        if let Err(error) = handle_event(res, &watch_dir, &slack_client, &dir_mapping) {
            tracing::error!("{error:?}");
        }
    }
}

fn handle_event(
    event: Result<notify::Event, notify::Error>,
    watch_dir: &Path,
    slack_client: &slack::Client,
    dir_mapping: &HashMap<String, String>,
) -> anyhow::Result<()> {
    let event = event?;

    // check if the event is a close event on a writable file
    if matches!(
        event.kind,
        EventKind::Access(AccessKind::Close(AccessMode::Write))
    ) {
        let full_path = event.paths.first().ok_or(anyhow!("no path in fs event"))?;

        let relative_path = pathdiff::diff_paths(full_path, watch_dir)
            .ok_or(anyhow!("cannot get relative path of modified file"))?;

        let parent_directory = relative_path
            .parent()
            .ok_or(anyhow!("no parent folder to modified file"))?
            .to_str()
            .ok_or(anyhow!("invalid utf-8 parent folder name"))?;

        let filename = relative_path
            .file_name()
            .ok_or(anyhow!("modified file has no filename"))?
            .to_str()
            .ok_or(anyhow!("invalid utf-8 filename"))?;

        tracing::debug!("{relative_path:?} was modified, parent folder is '{parent_directory}'");

        let channel = dir_mapping.get(parent_directory).ok_or(anyhow!(
            "cannot find channel mapping for directory '{parent_directory}'"
        ))?;

        tracing::debug!("found channel mapping to {channel}");

        let upload_request = slack::UploadFileRequest {
            channel,
            filename,
            path: full_path,
        };

        slack_client
            .upload_file(upload_request)
            .with_context(|| "error when uploading file")?;

        tracing::debug!("file uploaded!");
    }

    Ok(())
}
