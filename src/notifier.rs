use std::process::Command;

use crate::{config::NotifyCmdArgs, prelude::*};

pub enum Notifier {
    No,
    Cmd(CmdNotifier),
}

impl Notifier {
    pub fn process(&self, title: String, lines: &[String]) -> Result<()> {
        if let Self::Cmd(nofi) = self {
            nofi.process(title, lines)
        } else {
            Ok(())
        }
    }
}

pub struct CmdNotifier {
    args: NotifyCmdArgs,
}

impl CmdNotifier {
    fn process(&self, title: String, lines: &[String]) -> Result<()> {
        if lines.is_empty() {
            return Ok(());
        }

        let args = self.args.args.iter().map(|s| match s.as_str() {
            "$title" => title.clone(),
            "$body" => lines.join("\n"),
            _ => s.to_owned(),
        });

        let mut child = Command::new(&self.args.bin)
            .args(args)
            .spawn()
            .context(ExecuteCtx)?;

        // TODO: handle exit status
        let _exit = child.wait();

        Ok(())
    }
}
