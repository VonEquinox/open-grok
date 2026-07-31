use crate::app::actions::Action;
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand};

pub struct RewindCommand;

impl SlashCommand for RewindCommand {
    fn name(&self) -> &str {
        "rewind"
    }

    fn aliases(&self) -> &[&str] {
        &["undo"]
    }

    fn description(&self) -> &str {
        "Rewind to a previous turn"
    }

    fn session_scoped(&self) -> bool {
        true
    }

    fn usage(&self) -> &str {
        "/rewind"
    }

    fn run(&self, _ctx: &mut CommandExecCtx, _args: &str) -> CommandResult {
        CommandResult::Action(Action::RewindShowPicker)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slash::command::SlashCommand;

    #[test]
    fn name_is_rewind() {
        assert_eq!(RewindCommand.name(), "rewind");
    }

    /// `/undo` is a plain alias: same picker, same session scope.
    #[test]
    fn aliases_include_undo() {
        assert_eq!(RewindCommand.aliases(), &["undo"]);
    }
}
