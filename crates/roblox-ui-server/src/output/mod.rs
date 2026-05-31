use async_channel::{unbounded, Receiver, Sender};

use roblox_ui_project::Config;

mod structs;

use structs::*;

#[derive(Debug)]
pub struct OutputProcessor {
    _config: Config,
    notification_tx: Sender<OutputMessage>,
    notification_rx: Option<Receiver<OutputMessage>>,
}

impl OutputProcessor {
    pub fn new(config: Config) -> Self {
        let (notification_tx, notification_rx) = unbounded();
        Self {
            _config: config,
            notification_tx,
            notification_rx: Some(notification_rx),
        }
    }

    pub fn take_notification_receiver(&mut self) -> Option<Receiver<OutputMessage>> {
        self.notification_rx.take()
    }

    fn notify(&self, notification: OutputMessage) {
        // NOTE: Not having any listeners is fine and is the only error case
        self.notification_tx.try_send(notification).ok();
    }
}
