use std::fs;
use std::path::PathBuf;

use clap::Command;
use serde::Serialize;
use specman::dependency_tree::{ArtifactId, ArtifactKind, DependencyMapping};

use crate::commands::CommandResult;
use crate::context::CliSession;
use crate::error::CliError;
use crate::util;

#[derive(Clone, Debug, Serialize)]
pub struct StatusReport {
    pub name: String,
    pub kind: String,
    pub path: String,
    pub ok: bool,
    pub message: Option<String>,
}

pub fn command() -> Command {
    Command::new("status").about("Validate specifications and implementations")
}

pub fn run(session: &CliSession, _matches: &clap::ArgMatches) -> Result<CommandResult, CliError> {
    let mut reports = Vec::new();
    let mut healthy = true;
    for artifact in collect_artifacts(session) {
        let path = artifact_path(&session.workspace_paths, &artifact);
        match session.dependency_mapper.dependency_tree(&artifact) {
            Ok(_) => reports.push(StatusReport {
                name: artifact.name.clone(),
                kind: artifact_kind(&artifact),
                path,
                ok: true,
                message: None,
            }),
            Err(err) => {
                healthy = false;
                reports.push(StatusReport {
                    name: artifact.name.clone(),
                    kind: artifact_kind(&artifact),
                    path,
                    ok: false,
                    message: Some(err.to_string()),
                });
            }
        }
    }
    reports.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(CommandResult::Status { reports, healthy })
}

fn collect_artifacts(session: &CliSession) -> Vec<ArtifactId> {
    let mut artifacts = Vec::new();
    artifacts.extend(read_dir_artifacts(
        session.workspace_paths.spec_dir(),
        ArtifactKind::Specification,
    ));
    artifacts.extend(read_dir_artifacts(
        session.workspace_paths.impl_dir(),
        ArtifactKind::Implementation,
    ));
    artifacts
}

fn read_dir_artifacts(dir: PathBuf, kind: ArtifactKind) -> Vec<ArtifactId> {
    let mut result = Vec::new();
    if dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    if let Some(name) = entry.file_name().to_str() {
                        result.push(ArtifactId {
                            kind,
                            name: name.to_string(),
                        });
                    }
                }
            }
        }
    }
    result
}

fn artifact_path(paths: &specman::workspace::WorkspacePaths, artifact: &ArtifactId) -> String {
    let rel = match artifact.kind {
        ArtifactKind::Specification => paths.spec_dir().join(&artifact.name).join("spec.md"),
        ArtifactKind::Implementation => paths.impl_dir().join(&artifact.name).join("impl.md"),
        ArtifactKind::ScratchPad => paths
            .scratchpad_dir()
            .join(&artifact.name)
            .join("scratch.md"),
    };
    util::workspace_relative(paths.root(), &rel)
}

fn artifact_kind(artifact: &ArtifactId) -> String {
    match artifact.kind {
        ArtifactKind::Specification => "spec".into(),
        ArtifactKind::Implementation => "impl".into(),
        ArtifactKind::ScratchPad => "scratch".into(),
    }
}
