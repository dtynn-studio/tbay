use std::process::Command;

use dingtalk_sdk::{Client, WebhookService};

use crate::{
    config::{Notify, NotifyCmdArgs},
    prelude::*,
};

pub enum Notifier {
    No,
    Cmd(CmdNotifier),
    DingTalk(DingTalkNotifier),
}

impl Notifier {
    pub fn new(cfg: Notify) -> Result<Self> {
        match cfg {
            Notify::No => Ok(Self::No),
            Notify::Cmd(args) => Ok(Self::Cmd(CmdNotifier { args })),
            Notify::DingTalk(args) => {
                let client = Client::builder().build().context(DingTalkCtx)?;
                let webhook = client.webhook(args.token, args.secret);

                Ok(Self::DingTalk(DingTalkNotifier {
                    webhook: Arc::new(webhook),
                }))
            }
        }
    }

    pub fn process(&self, title: &str, lines: &[String]) -> Result<()> {
        match self {
            Self::No => Ok(()),
            Self::Cmd(n) => n.process(title, lines),
            Self::DingTalk(n) => n.process(title, lines),
        }
    }
}

pub struct CmdNotifier {
    args: NotifyCmdArgs,
}

impl CmdNotifier {
    fn process(&self, title: &str, lines: &[String]) -> Result<()> {
        if lines.is_empty() {
            return Ok(());
        }

        let args = self.args.args.iter().map(|s| match s.as_str() {
            "$title" => title.to_owned(),
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

pub struct DingTalkNotifier {
    webhook: Arc<WebhookService>,
}

impl DingTalkNotifier {
    fn process(&self, title: &str, lines: &[String]) -> Result<()> {
        let content = format!("{title}\n{}", lines.join("\n"));
        let wh = self.webhook.clone();
        tokio::spawn(async move {
            if let Err(_e) =
                wh.send_text_message(&content, None, None, None).await
            {
            }
        });

        Ok(())
    }
}
