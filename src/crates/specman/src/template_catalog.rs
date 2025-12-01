use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

use crate::error::SpecmanError;
use crate::scratchpad::{ScratchPadProfile, ScratchPadProfileKind};
use crate::template::{
    TemplateDescriptor, TemplateLocator, TemplateProvenance, TemplateScenario, TemplateTier,
};
use crate::workspace::WorkspacePaths;

const EMBEDDED_SPEC: &str = include_str!("../../../../templates/spec/spec.md");
const EMBEDDED_IMPL: &str = include_str!("../../../../templates/impl/impl.md");
const EMBEDDED_SCRATCH: &str = include_str!("../../../../templates/scratch/scratch.md");

/// Canonical template catalog implementation backed by workspace overrides,
/// pointer files, remote caches, and embedded defaults.
pub struct TemplateCatalog {
    workspace: WorkspacePaths,
}

/// Result of resolving a template with provenance metadata for persistence.
#[derive(Clone, Debug)]
pub struct ResolvedTemplate {
    pub descriptor: TemplateDescriptor,
    pub provenance: TemplateProvenance,
}

impl TemplateCatalog {
    pub fn new(workspace: WorkspacePaths) -> Self {
        Self { workspace }
    }

    /// Resolves a template descriptor for the given scenario following the
    /// override → pointer → embedded order mandated by SpecMan Core.
    pub fn resolve(&self, scenario: TemplateScenario) -> Result<ResolvedTemplate, SpecmanError> {
        if let Some(resolved) = self.try_workspace_override(&scenario)? {
            return Ok(resolved);
        }

        if let Some(resolved) = self.try_pointer(&scenario)? {
            return Ok(resolved);
        }

        self.embedded_default(&scenario)
    }

    /// Convenience helper for describing scratch pad profiles with catalog
    /// managed templates and provenance metadata.
    pub fn scratch_profile(
        &self,
        kind: ScratchPadProfileKind,
    ) -> Result<ScratchPadProfile, SpecmanError> {
        let scenario = TemplateScenario::WorkType(kind.slug().to_string());
        let resolved = self.resolve(scenario)?;
        Ok(ScratchPadProfile {
            kind,
            name: String::new(),
            template: resolved.descriptor,
            provenance: Some(resolved.provenance),
            configuration: Default::default(),
        })
    }

    fn try_workspace_override(
        &self,
        scenario: &TemplateScenario,
    ) -> Result<Option<ResolvedTemplate>, SpecmanError> {
        for candidate in self.override_candidates(scenario) {
            if candidate.is_file() {
                return Ok(Some(self.resolved_from_path(
                    scenario,
                    candidate,
                    TemplateTier::WorkspaceOverride,
                    None,
                    None,
                    None,
                    None,
                )));
            }
        }
        Ok(None)
    }

    fn try_pointer(
        &self,
        scenario: &TemplateScenario,
    ) -> Result<Option<ResolvedTemplate>, SpecmanError> {
        let pointer_name = pointer_name(scenario);
        let pointer_path = self
            .workspace
            .dot_specman()
            .join("templates")
            .join(pointer_name);
        if !pointer_path.is_file() {
            return Ok(None);
        }

        let contents = fs::read_to_string(&pointer_path).map_err(|err| {
            SpecmanError::Template(format!(
                "failed to read template pointer {}: {err}",
                pointer_path.display()
            ))
        })?;
        let trimmed = contents.trim();
        if trimmed.is_empty() {
            return Err(SpecmanError::Template(format!(
                "template pointer {} has no content",
                pointer_path.display()
            )));
        }

        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            let url = Url::parse(trimmed).map_err(|err| {
                SpecmanError::Template(format!("invalid template pointer URL {}: {err}", trimmed))
            })?;
            let cache = TemplateCache::new(&self.workspace);
            match cache.fetch_url(&url) {
                Ok(hit) => {
                    let cache_path = workspace_relative(self.workspace.root(), &hit.path);
                    return Ok(Some(self.resolved_from_path(
                        scenario,
                        hit.path,
                        TemplateTier::PointerUrl,
                        Some(pointer_name.to_string()),
                        Some(url.to_string()),
                        Some(cache_path),
                        hit.last_modified,
                    )));
                }
                Err(_err) => {
                    // Spec requires falling back to embedded defaults when the remote
                    // pointer cannot be refreshed and no cache exists.
                    return Ok(None);
                }
            }
        }

        let file_path = self.resolve_pointer_path(trimmed, pointer_name)?;
        Ok(Some(self.resolved_from_path(
            scenario,
            file_path,
            TemplateTier::PointerFile,
            Some(pointer_name.to_string()),
            None,
            None,
            None,
        )))
    }

    fn embedded_default(
        &self,
        scenario: &TemplateScenario,
    ) -> Result<ResolvedTemplate, SpecmanError> {
        let (key, body) = match scenario {
            TemplateScenario::Specification => ("spec", EMBEDDED_SPEC),
            TemplateScenario::Implementation => ("impl", EMBEDDED_IMPL),
            TemplateScenario::ScratchPad | TemplateScenario::WorkType(_) => {
                ("scratch", EMBEDDED_SCRATCH)
            }
        };

        let cache = TemplateCache::new(&self.workspace);
        let path = cache.write_embedded(key, body)?;
        let cache_path = workspace_relative(self.workspace.root(), &path);
        Ok(self.resolved_from_path(
            scenario,
            path,
            TemplateTier::EmbeddedDefault,
            None,
            Some(format!("embedded://{key}")),
            Some(cache_path),
            None,
        ))
    }

    fn override_candidates(&self, scenario: &TemplateScenario) -> Vec<PathBuf> {
        let base = self.workspace.dot_specman().join("templates");
        match scenario {
            TemplateScenario::Specification => vec![base.join("spec.md")],
            TemplateScenario::Implementation => vec![base.join("impl.md")],
            TemplateScenario::ScratchPad => vec![base.join("scratch.md")],
            TemplateScenario::WorkType(kind) => {
                let slug = sanitize_key(kind);
                vec![
                    base.join("scratch").join(format!("{slug}.md")),
                    base.join(format!("scratch-{slug}.md")),
                    base.join("scratch.md"),
                ]
            }
        }
    }

    fn resolve_pointer_path(&self, raw: &str, pointer_name: &str) -> Result<PathBuf, SpecmanError> {
        let candidate = PathBuf::from(raw);
        let resolved = if candidate.is_absolute() {
            candidate
        } else {
            self.workspace.root().join(candidate)
        };

        if !resolved.starts_with(self.workspace.root()) {
            return Err(SpecmanError::Template(format!(
                "pointer {} resolved outside the workspace: {}",
                pointer_name,
                resolved.display()
            )));
        }

        if !resolved.is_file() {
            return Err(SpecmanError::Template(format!(
                "pointer {} references missing file: {}",
                pointer_name,
                resolved.display()
            )));
        }

        Ok(resolved)
    }

    fn resolved_from_path(
        &self,
        scenario: &TemplateScenario,
        path: PathBuf,
        tier: TemplateTier,
        pointer: Option<String>,
        locator_override: Option<String>,
        cache_override: Option<String>,
        last_modified: Option<String>,
    ) -> ResolvedTemplate {
        let locator = TemplateLocator::FilePath(path.clone());
        let provenance = TemplateProvenance {
            tier,
            locator: locator_override
                .unwrap_or_else(|| workspace_relative(self.workspace.root(), &path)),
            pointer,
            cache_path: cache_override,
            last_modified,
        };
        ResolvedTemplate {
            descriptor: TemplateDescriptor {
                locator,
                scenario: scenario.clone(),
                required_tokens: Vec::new(),
            },
            provenance,
        }
    }
}

fn pointer_name(scenario: &TemplateScenario) -> &'static str {
    match scenario {
        TemplateScenario::Specification => "SPEC",
        TemplateScenario::Implementation => "IMPL",
        TemplateScenario::ScratchPad | TemplateScenario::WorkType(_) => "SCRATCH",
    }
}

fn sanitize_key(raw: &str) -> String {
    raw.chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect::<String>()
        .to_lowercase()
}

fn workspace_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}

struct TemplateCache {
    root: PathBuf,
}

impl TemplateCache {
    fn new(workspace: &WorkspacePaths) -> Self {
        Self {
            root: workspace.dot_specman().join("cache").join("templates"),
        }
    }

    fn ensure_root(&self) -> Result<(), SpecmanError> {
        fs::create_dir_all(&self.root).map_err(SpecmanError::from)
    }

    fn write_embedded(&self, key: &str, contents: &str) -> Result<PathBuf, SpecmanError> {
        self.ensure_root()?;
        let path = self.root.join(format!("embedded-{key}.md"));
        fs::write(&path, contents)?;
        Ok(path)
    }

    fn fetch_url(&self, url: &Url) -> Result<CacheHit, SpecmanError> {
        self.ensure_root()?;
        let key = hash_url(url);
        let path = self.root.join(format!("url-{key}.md"));
        let meta_path = self.root.join(format!("url-{key}.json"));

        match ureq::get(url.as_str()).call() {
            Ok(response) => {
                if response.status() >= 400 {
                    return Err(SpecmanError::Template(format!(
                        "failed to download template {}; status {}",
                        url,
                        response.status()
                    )));
                }
                let last_modified = response
                    .header("Last-Modified")
                    .map(|value| value.to_string());
                let body = response
                    .into_string()
                    .map_err(|err| SpecmanError::Template(err.to_string()))?;
                fs::write(&path, body)?;
                let metadata = TemplateCacheMetadata {
                    locator: url.to_string(),
                    last_modified: last_modified.clone(),
                };
                fs::write(&meta_path, serde_json::to_string_pretty(&metadata)?)?;
                Ok(CacheHit {
                    path,
                    last_modified,
                })
            }
            Err(err) => {
                if path.is_file() {
                    let metadata = read_metadata(&meta_path)?;
                    return Ok(CacheHit {
                        path,
                        last_modified: metadata.and_then(|m| m.last_modified),
                    });
                }
                Err(SpecmanError::Template(format!(
                    "failed to download template {}: {}",
                    url, err
                )))
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
struct TemplateCacheMetadata {
    locator: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_modified: Option<String>,
}

struct CacheHit {
    path: PathBuf,
    last_modified: Option<String>,
}

fn read_metadata(path: &Path) -> Result<Option<TemplateCacheMetadata>, SpecmanError> {
    if !path.is_file() {
        return Ok(None);
    }
    let content = fs::read_to_string(path)?;
    let metadata = serde_json::from_str(&content).map_err(|err| {
        SpecmanError::Serialization(format!("invalid template cache metadata: {err}"))
    })?;
    Ok(Some(metadata))
}

fn hash_url(url: &Url) -> String {
    let mut hasher = Sha256::new();
    hasher.update(url.as_str().as_bytes());
    let digest = hasher.finalize();
    hex::encode(digest)
}
