//! User-selected local Markdown folders, independent of Warp Drive and cloud services.
mod catalog;

use std::io::Write as _;
use std::path::{Path, PathBuf};

use futures_lite::FutureExt;
use ordered_float::OrderedFloat;
use settings::macros::define_settings_group;
use settings::{Setting, SupportedPlatforms, SyncToCloud};
use warpui::{AppContext, Element, SingletonEntity};

use crate::appearance::Appearance;
use crate::search::command_palette::files::search_item::FileSearchItem;
use crate::search::command_palette::mixer::CommandPaletteItemAction;
use crate::search::data_source::{Query, QueryResult};
use crate::search::item::SearchItem;
use crate::search::mixer::{AsyncDataSource, BoxFuture, DataSourceRunErrorWrapper};
use crate::search::result_renderer::ItemHighlightState;

define_settings_group!(LocalRunbookSettings, settings: [
    folders: LocalRunbookFolders {
        type: Vec<String>,
        default: Vec::new(),
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Never,
        surface: settings::SettingSurfaces::GUI,
        private: false,
        toml_path: "runbooks.folders",
        description: "Local folders containing Markdown runbooks. Absolute paths and ~/ paths are supported. Subfolders are searched; nested symlinks are skipped. Nothing is uploaded.",
    },
]);

/// Configured runbook folders, expanded. Relative entries are ignored, matching discovery.
pub fn configured_folders(app: &AppContext) -> Vec<PathBuf> {
    LocalRunbookSettings::as_ref(app)
        .folders
        .value()
        .iter()
        .map(|path| PathBuf::from(shellexpand::tilde(path).into_owned()))
        .filter(|path| path.is_absolute())
        .collect()
}

const NEW_RUNBOOK_TEMPLATE: &str = "# Untitled runbook\n\
\n\
Describe what this runbook is for. Only fenced shell blocks get a Run button.\n\
\n\
## Check the working tree\n\
\n\
```sh\n\
git status --short\n\
```\n";

/// Creates `untitled-runbook.md` (or `untitled-runbook-N.md`) in `folder` from a
/// template without overwriting existing files, and returns its path.
pub fn create_runbook(folder: &Path) -> std::io::Result<PathBuf> {
    if !folder.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("runbook folder {} does not exist", folder.display()),
        ));
    }
    for n in 1..1000 {
        let name = if n == 1 {
            "untitled-runbook.md".to_owned()
        } else {
            format!("untitled-runbook-{n}.md")
        };
        let path = folder.join(name);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                file.write_all(NEW_RUNBOOK_TEMPLATE.as_bytes())?;
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "too many untitled runbooks; rename some first",
    ))
}

pub struct RunbookDataSource;
impl warpui::Entity for RunbookDataSource {
    type Event = ();
}
impl AsyncDataSource for RunbookDataSource {
    type Action = CommandPaletteItemAction;
    fn run_query(
        &self,
        query: &Query,
        app: &AppContext,
    ) -> BoxFuture<'static, Result<Vec<QueryResult<Self::Action>>, DataSourceRunErrorWrapper>> {
        let roots = configured_folders(app);
        let query = query.text.clone();
        async move {
            Ok(blocking::unblock(move || catalog::discover(&roots))
                .await
                .into_iter()
                .filter_map(|path| {
                    let text = path.to_string_lossy();
                    let matched = if query.is_empty() {
                        fuzzy_match::FuzzyMatchResult::no_match()
                    } else {
                        fuzzy_match::match_indices_case_insensitive(&text, &query)?
                    };
                    Some(QueryResult::from(RunbookItem(FileSearchItem {
                        path,
                        project_directory: String::new(),
                        match_result: matched,
                        line_and_column_arg: None,
                        is_directory: false,
                    })))
                })
                .take(250)
                .collect())
        }
        .boxed()
    }
}

#[derive(Debug)]
struct RunbookItem(FileSearchItem);
impl SearchItem for RunbookItem {
    type Action = CommandPaletteItemAction;
    fn render_icon(&self, state: ItemHighlightState, appearance: &Appearance) -> Box<dyn Element> {
        self.0.render_icon(state, appearance)
    }
    fn render_item(&self, state: ItemHighlightState, app: &AppContext) -> Box<dyn Element> {
        self.0.render_item(state, app)
    }
    fn score(&self) -> OrderedFloat<f64> {
        self.0.score()
    }
    fn accept_result(&self) -> Self::Action {
        CommandPaletteItemAction::OpenLocalRunbook {
            path: self.0.path.clone(),
        }
    }
    fn execute_result(&self) -> Self::Action {
        self.accept_result()
    }
    fn accessibility_label(&self) -> String {
        format!("Local runbook: {}", self.0.path.display())
    }
    fn accessibility_help_message(&self) -> Option<String> {
        Some("Open Markdown runbook; commands run only when you choose Run".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_runbook_never_overwrites() {
        let root = std::env::temp_dir().join(format!("warpeer-new-runbook-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let first = create_runbook(&root).unwrap();
        std::fs::write(&first, "mine").unwrap();
        let second = create_runbook(&root).unwrap();
        assert_eq!(first.file_name().unwrap(), "untitled-runbook.md");
        assert_eq!(second.file_name().unwrap(), "untitled-runbook-2.md");
        assert_eq!(std::fs::read_to_string(&first).unwrap(), "mine");
        assert!(
            std::fs::read_to_string(&second)
                .unwrap()
                .contains("```sh\n")
        );
        assert!(create_runbook(&root.join("missing")).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
