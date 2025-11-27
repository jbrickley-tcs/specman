use clap::{Arg, ArgAction, ArgMatches, Command};
use serde::Serialize;

use crate::error::{CliError, ExitStatus};

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DependencyView {
    Downstream,
    Upstream,
    All,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyScope {
    Specification,
    Implementation,
    ScratchPad,
}

pub fn with_direction_flags(command: Command) -> Command {
    command
        .arg(
            Arg::new("downstream")
                .long("downstream")
                .action(ArgAction::SetTrue)
                .help("Render the downstream dependency tree (default)."),
        )
        .arg(
            Arg::new("upstream")
                .long("upstream")
                .action(ArgAction::SetTrue)
                .help("Render the upstream dependency tree."),
        )
        .arg(
            Arg::new("all")
                .long("all")
                .action(ArgAction::SetTrue)
                .help("Render both upstream and downstream trees."),
        )
}

pub fn parse_view(matches: &ArgMatches) -> Result<DependencyView, CliError> {
    let selected = [
        (DependencyView::Downstream, matches.get_flag("downstream")),
        (DependencyView::Upstream, matches.get_flag("upstream")),
        (DependencyView::All, matches.get_flag("all")),
    ];

    let mut chosen = None;
    for (view, flag) in selected {
        if flag {
            if chosen.is_some() {
                return Err(CliError::new(
                    "use only one of --downstream, --upstream, or --all",
                    ExitStatus::Usage,
                ));
            }
            chosen = Some(view);
        }
    }

    Ok(chosen.unwrap_or(DependencyView::Downstream))
}

pub fn scope_label(scope: DependencyScope) -> &'static str {
    match scope {
        DependencyScope::Specification => "specification",
        DependencyScope::Implementation => "implementation",
        DependencyScope::ScratchPad => "scratch pad",
    }
}

pub fn view_label(view: DependencyView) -> &'static str {
    match view {
        DependencyView::Downstream => "downstream",
        DependencyView::Upstream => "upstream",
        DependencyView::All => "complete",
    }
}
