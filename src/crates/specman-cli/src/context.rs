use std::path::PathBuf;

use specman::dependency_tree::FilesystemDependencyMapper;
use specman::persistence::WorkspacePersistence;
use specman::template::MarkdownTemplateEngine;
use specman::workspace::{FilesystemWorkspaceLocator, WorkspaceLocator, WorkspacePaths};

use crate::error::CliError;
use crate::templates::TemplateCatalog;
use crate::util::Verbosity;

pub struct CliSession {
    pub workspace_paths: WorkspacePaths,
    pub dependency_mapper: FilesystemDependencyMapper<FilesystemWorkspaceLocator>,
    pub persistence: WorkspacePersistence<FilesystemWorkspaceLocator>,
    pub template_engine: MarkdownTemplateEngine,
    pub templates: TemplateCatalog,
    pub verbosity: Verbosity,
}

impl CliSession {
    pub fn bootstrap(
        workspace_override: Option<String>,
        verbosity: Verbosity,
    ) -> Result<Self, CliError> {
        let locator = match workspace_override {
            Some(path) => {
                let locator = FilesystemWorkspaceLocator::new(PathBuf::from(path));
                locator.workspace()?;
                locator
            }
            None => FilesystemWorkspaceLocator::from_current_dir()?,
        };

        let workspace_paths = locator.workspace()?;
        let start = workspace_paths.root().to_path_buf();
        let dependency_mapper =
            FilesystemDependencyMapper::new(FilesystemWorkspaceLocator::new(start.clone()));
        let persistence = WorkspacePersistence::new(FilesystemWorkspaceLocator::new(start));
        let template_engine = MarkdownTemplateEngine::default();
        let templates = TemplateCatalog::new(workspace_paths.clone());

        Ok(Self {
            workspace_paths,
            dependency_mapper,
            persistence,
            template_engine,
            templates,
            verbosity,
        })
    }
}
