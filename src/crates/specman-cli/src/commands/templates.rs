use clap::{Arg, ArgMatches, Command};
use serde::Serialize;
use specman::{ResolvedTemplate, TemplateDescriptor, TemplateProvenance};

use crate::commands::CommandResult;
use crate::context::CliSession;
use crate::error::{CliError, ExitStatus};
use crate::templates::TemplateKind;

/// Defines the `specman template` command tree with `set` and `remove` subcommands.
pub fn command() -> Command {
    Command::new("template")
        .about("Manage template pointers for spec, impl, or scratch artifacts")
        .subcommand_required(true)
        .subcommand(
            Command::new("set")
                .about("Set or update the pointer file for a template kind")
                .arg(kind_arg())
                .arg(
                    Arg::new("locator")
                        .long("locator")
                        .value_name("LOCATOR")
                        .help("Workspace-relative path or HTTPS URL for the new pointer target")
                        .required(true),
                ),
        )
        .subcommand(
            Command::new("remove")
                .about("Remove the pointer file for a template kind and fall back to overrides/defaults")
                .arg(kind_arg()),
        )
}

/// Dispatches `template` subcommands to the correct handler.
pub fn run(session: &CliSession, matches: &ArgMatches) -> Result<CommandResult, CliError> {
    match matches.subcommand() {
        Some(("set", sub)) => set_pointer(session, sub),
        Some(("remove", sub)) => remove_pointer(session, sub),
        _ => Err(CliError::new(
            "missing template subcommand",
            ExitStatus::Usage,
        )),
    }
}

fn set_pointer(session: &CliSession, matches: &ArgMatches) -> Result<CommandResult, CliError> {
    let kind = resolve_kind(matches)?;
    let locator = matches
        .get_one::<String>("locator")
        .ok_or_else(|| CliError::new("--locator is required", ExitStatus::Usage))?;
    let resolved = session.templates.set_pointer(kind, locator)?;
    Ok(CommandResult::TemplatePointer {
        report: PointerReport::new(PointerAction::Set, kind, resolved),
    })
}

fn remove_pointer(session: &CliSession, matches: &ArgMatches) -> Result<CommandResult, CliError> {
    let kind = resolve_kind(matches)?;
    let resolved = session.templates.remove_pointer(kind)?;
    Ok(CommandResult::TemplatePointer {
        report: PointerReport::new(PointerAction::Remove, kind, resolved),
    })
}

fn resolve_kind(matches: &ArgMatches) -> Result<TemplateKind, CliError> {
    let raw = matches
        .get_one::<String>("kind")
        .ok_or_else(|| CliError::new("--kind is required", ExitStatus::Usage))?;
    Ok(match raw.as_str() {
        "spec" | "specification" => TemplateKind::Specification,
        "impl" | "implementation" => TemplateKind::Implementation,
        "scratch" => TemplateKind::Scratch,
        other => {
            return Err(CliError::new(
                format!("unsupported template kind: {other}"),
                ExitStatus::Usage,
            ));
        }
    })
}

fn kind_arg() -> Arg {
    Arg::new("kind")
        .long("kind")
        .value_name("KIND")
        .help("Template kind to mutate: spec, impl, or scratch")
        .required(true)
        .value_parser(["spec", "impl", "scratch"])
}

#[derive(Debug, Serialize)]
pub struct PointerReport {
    pub action: PointerAction,
    pub kind: TemplateKind,
    pub descriptor: TemplateDescriptor,
    pub provenance: TemplateProvenance,
}

impl PointerReport {
    fn new(action: PointerAction, kind: TemplateKind, resolved: ResolvedTemplate) -> Self {
        Self {
            action,
            kind,
            descriptor: resolved.descriptor,
            provenance: resolved.provenance,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PointerAction {
    Set,
    Remove,
}
