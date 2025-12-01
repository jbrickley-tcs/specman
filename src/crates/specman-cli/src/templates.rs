use specman::{ResolvedTemplate, TemplateCatalog as LibraryTemplateCatalog, TemplateScenario};
use specman::workspace::WorkspacePaths;

use crate::error::CliError;

#[derive(Clone, Copy)]
pub enum TemplateKind {
    Specification,
    Implementation,
    Scratch,
}

pub struct TemplateCatalog {
    catalog: LibraryTemplateCatalog,
}

impl TemplateCatalog {
    pub fn new(workspace: WorkspacePaths) -> Self {
        Self {
            catalog: LibraryTemplateCatalog::new(workspace),
        }
    }

    pub fn descriptor(&self, kind: TemplateKind) -> Result<ResolvedTemplate, CliError> {
        let scenario = match kind {
            TemplateKind::Specification => TemplateScenario::Specification,
            TemplateKind::Implementation => TemplateScenario::Implementation,
            TemplateKind::Scratch => TemplateScenario::ScratchPad,
        };
        self.catalog.resolve(scenario).map_err(CliError::from)
    }

}
