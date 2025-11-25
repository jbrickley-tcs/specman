use std::fs;
use std::path::{Path, PathBuf};

use specman::template::{TemplateDescriptor, TemplateLocator, TemplateScenario};
use specman::workspace::WorkspacePaths;

use crate::error::{CliError, ExitStatus};

#[derive(Clone, Copy)]
pub enum TemplateKind {
    Specification,
    Implementation,
    Scratch,
}

pub struct TemplateCatalog {
    workspace: WorkspacePaths,
}

impl TemplateCatalog {
    pub fn new(workspace: WorkspacePaths) -> Self {
        Self { workspace }
    }

    pub fn descriptor(&self, kind: TemplateKind) -> Result<TemplateDescriptor, CliError> {
        let (pointer_name, default_path, scenario) = match kind {
            TemplateKind::Specification => (
                "SPEC",
                "templates/spec/spec.md",
                TemplateScenario::Specification,
            ),
            TemplateKind::Implementation => (
                "IMPL",
                "templates/impl/impl.md",
                TemplateScenario::Implementation,
            ),
            TemplateKind::Scratch => (
                "SCRATCH",
                "templates/scratch/scratch.md",
                TemplateScenario::ScratchPad,
            ),
        };

        let locator = self.resolve_locator(pointer_name, default_path)?;
        Ok(TemplateDescriptor {
            locator,
            scenario,
            required_tokens: Vec::new(),
        })
    }

    fn resolve_locator(
        &self,
        pointer_name: &str,
        default_rel: &str,
    ) -> Result<TemplateLocator, CliError> {
        let pointer_path = self
            .workspace
            .dot_specman()
            .join("templates")
            .join(pointer_name);
        if pointer_path.is_file() {
            let contents = fs::read_to_string(&pointer_path)?;
            self.parse_locator(contents.trim(), pointer_path.as_path())
        } else {
            let default_path = self.workspace.root().join(default_rel);
            if default_path.is_file() {
                Ok(TemplateLocator::FilePath(default_path))
            } else {
                Err(CliError::new(
                    format!(
                        "template not found: {} (ensure {} exists)",
                        default_path.display(),
                        pointer_name
                    ),
                    ExitStatus::Config,
                ))
            }
        }
    }

    fn parse_locator(&self, raw: &str, pointer: &Path) -> Result<TemplateLocator, CliError> {
        if raw.is_empty() {
            return Err(CliError::new(
                format!("template pointer {} has no content", pointer.display()),
                ExitStatus::Config,
            ));
        }

        if raw.starts_with("https://") {
            return Ok(TemplateLocator::Url(raw.to_string()));
        }

        let candidate = PathBuf::from(raw);
        let resolved = if candidate.is_absolute() {
            candidate
        } else {
            self.workspace.root().join(candidate)
        };

        if !resolved.starts_with(self.workspace.root()) {
            return Err(CliError::new(
                format!(
                    "template pointer {} resolved outside the workspace: {}",
                    pointer.display(),
                    resolved.display()
                ),
                ExitStatus::Usage,
            ));
        }

        if !resolved.is_file() {
            return Err(CliError::new(
                format!("template file not found: {}", resolved.display()),
                ExitStatus::Config,
            ));
        }

        Ok(TemplateLocator::FilePath(resolved))
    }
}
