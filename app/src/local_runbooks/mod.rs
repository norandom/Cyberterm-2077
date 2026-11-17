//! User-selected local Markdown folders, independent of Warp Drive and cloud services.
mod catalog;

use std::path::PathBuf;

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
        let roots: Vec<PathBuf> = LocalRunbookSettings::as_ref(app)
            .folders
            .value()
            .iter()
            .map(|path| PathBuf::from(shellexpand::tilde(path).into_owned()))
            .collect();
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
