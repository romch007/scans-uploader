mod uploader;

use anyhow::anyhow;
use notify::{
    event::{AccessKind, AccessMode},
    EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::mpsc,
};

fn main() {
    tracing_subscriber::fmt::init();

    let watch_dir: PathBuf = env::var_os("WATCH_DIR")
        .expect("WATCH_DIR not provided")
        .into();

    let watch_dir = fs::canonicalize(watch_dir).expect("cannot canonicalize path");

    let (fs_event_tx, fs_event_rx) = mpsc::channel();

    let mut watcher = notify::recommended_watcher(fs_event_tx).expect("cannot create watcher");

    watcher
        .watch(&watch_dir, RecursiveMode::Recursive)
        .expect("cannot watch directory");

    tracing::info!(
        "watching {} using {:?}",
        watch_dir.display(),
        RecommendedWatcher::kind()
    );

    let discord_webhook_url = env::var("WEBHOOK_URL").expect("no WEBHOOK_URL");

    let uploader = uploader::Discord::new(discord_webhook_url);

    for res in fs_event_rx {
        if let Err(error) = handle_event(res, &watch_dir, uploader.clone()) {
            tracing::error!("{error:?}");
        }
    }
}

fn handle_event(
    event: Result<notify::Event, notify::Error>,
    watch_dir: &Path,
    uploader: uploader::Discord,
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

        uploader.upload(filename, full_path)?;

        tracing::debug!("file uploaded!");
    }

    Ok(())
}
