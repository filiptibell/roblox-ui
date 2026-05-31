use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use roblox_ui_project::Config;

mod structs;

use structs::*;

#[derive(Debug)]
pub struct OutputProcessor {
    _config: Config,
    notification_tx: UnboundedSender<OutputMessage>,
    notification_rx: Option<UnboundedReceiver<OutputMessage>>,
}

impl OutputProcessor {
    pub fn new(config: Config) -> Self {
        let (notification_tx, notification_rx) = unbounded_channel();
        Self {
            _config: config,
            notification_tx,
            notification_rx: Some(notification_rx),
        }
    }

    pub fn take_notification_receiver(&mut self) -> Option<UnboundedReceiver<OutputMessage>> {
        self.notification_rx.take()
    }

    fn notify(&self, notification: OutputMessage) {
        // NOTE: Not having any listeners is fine and is the only error case
        self.notification_tx.send(notification).ok();
    }
}
