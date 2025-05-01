mod uploader;

use color_eyre::eyre::{eyre, Context, ContextCompat};
use notify::{
    event::{AccessKind, AccessMode},
    EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::mpsc,
};

fn main() -> color_eyre::Result<()> {
    tracing_subscriber::fmt::init();
    color_eyre::install()?;

    let ignore_dotfiles = match env::var("IGNORE_DOTFILES") {
        Ok(val) => val
            .parse::<bool>()
            .wrap_err("invalid IGNORE_DOTFILES env variable")?,
        Err(env::VarError::NotPresent) => true,
        Err(e) => return Err(e).wrap_err("failed to read IGNORE_DOTFILES env variable"),
    };

    let watch_dir: PathBuf = env::var_os("WATCH_DIR")
        .wrap_err("WATCH_DIR not provided")?
        .into();

    let watch_dir = fs::canonicalize(watch_dir).wrap_err("cannot canonicalize path")?;

    let (fs_event_tx, fs_event_rx) = mpsc::channel();

    let mut watcher = notify::recommended_watcher(fs_event_tx).wrap_err("cannot create watcher")?;

    watcher
        .watch(&watch_dir, RecursiveMode::Recursive)
        .wrap_err("cannot watch directory")?;

    tracing::info!(
        "watching {} using {:?}",
        watch_dir.display(),
        RecommendedWatcher::kind()
    );

    let discord_webhook_url = env::var("WEBHOOK_URL").wrap_err("no WEBHOOK_URL")?;

    let uploader = uploader::Discord::new(discord_webhook_url);

    for res in fs_event_rx {
        if let Err(error) = handle_event(res, &watch_dir, ignore_dotfiles, uploader.clone()) {
            tracing::error!("error while handling event: {error:?}");
        }
    }

    Ok(())
}

fn handle_event(
    event: Result<notify::Event, notify::Error>,
    watch_dir: &Path,
    ignore_dotfiles: bool,
    uploader: uploader::Discord,
) -> color_eyre::Result<()> {
    let event = event.wrap_err("error in event")?;

    // check if the event is a close event on a writable file
    if matches!(
        event.kind,
        EventKind::Access(AccessKind::Close(AccessMode::Write))
    ) {
        let full_path = event.paths.first().ok_or(eyre!("no path in fs event"))?;

        let relative_path = pathdiff::diff_paths(full_path, watch_dir)
            .ok_or(eyre!("cannot get relative path of modified file"))?;

        let parent_directory = relative_path
            .parent()
            .ok_or(eyre!("no parent folder to modified file"))?
            .to_str()
            .ok_or(eyre!("invalid utf-8 parent folder name"))?;

        let filename = relative_path
            .file_name()
            .ok_or(eyre!("modified file has no filename"))?
            .to_str()
            .ok_or(eyre!("invalid utf-8 filename"))?;

        tracing::debug!("{relative_path:?} was modified, parent folder is '{parent_directory}'");

        if ignore_dotfiles && filename.starts_with('.') {
            tracing::debug!("file is a dotfile, ignoring");
        } else {
            uploader
                .upload(parent_directory, filename, full_path)
                .wrap_err_with(|| format!("could not upload file '{}'", full_path.display()))?;

            tracing::debug!("file uploaded!");
        }
    }

    Ok(())
}
