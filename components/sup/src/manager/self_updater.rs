//! Encapsulates logic required for updating the Habitat Supervisor
//! itself.

use crate::util;
use habitat_common::command::package::install::InstallSource;
use habitat_core::{package::{PackageIdent,
                             PackageInstall},
                   ChannelIdent};
use rand::Rng;
use std::{borrow::Borrow,
          time::Duration};
use tokio::{self,
            sync::oneshot::{self,
                            error::TryRecvError,
                            Receiver,
                            Sender},
            time as tokiotime};

pub const SUP_PKG_IDENT: &str = "core/hab-sup";

pub struct SelfUpdater {
    rx:             Receiver<PackageInstall>,
    current:        PackageIdent,
    update_url:     String,
    update_channel: ChannelIdent,
    period:         Duration,
}

/// The subset of data from `SelfUpdater` needed to spawn the updater task.
struct Runner {
    current:        PackageIdent,
    update_url:     String,
    update_channel: ChannelIdent,
    period:         Duration,
}

impl<T: Borrow<SelfUpdater>> From<T> for Runner {
    fn from(other: T) -> Self {
        let other = other.borrow();
        Self { current:        other.current.clone(),
               update_url:     other.update_url.clone(),
               update_channel: other.update_channel.clone(),
               period:         other.period, }
    }
}

impl SelfUpdater {
    pub fn new(current: &PackageIdent,
               update_url: String,
               update_channel: ChannelIdent,
               period: Duration)
               -> Self {
        let runner = Runner { current: current.clone(),
                              update_url: update_url.clone(),
                              update_channel: update_channel.clone(),
                              period };
        let rx = Self::init(runner);
        SelfUpdater { rx,
                      current: current.clone(),
                      update_url,
                      update_channel,
                      period }
    }

    /// Spawn a new Supervisor updater task.
    fn init(runner: Runner) -> Receiver<PackageInstall> {
        let (tx, rx) = oneshot::channel();
        tokio::spawn(Self::run(tx, runner));
        rx
    }

    async fn run(tx: Sender<PackageInstall>, runner: Runner) {
        // SUP_PKG_IDENT will always parse as a valid PackageIdent,
        // and thus a valid InstallSource
        let install_source: InstallSource = SUP_PKG_IDENT.parse().unwrap();
        let Runner { current,
                     update_url,
                     update_channel,
                     period, } = runner;
        let splay =
            Duration::from_secs_f64(rand::thread_rng().gen_range(0.0, period.as_secs_f64()));
        debug!("Starting self updater with current package {} in {}s",
               current,
               splay.as_secs_f64());
        tokiotime::delay_for(splay).await;
        loop {
            match util::pkg::install_no_ui(&update_url, &install_source, &update_channel).await {
                Ok(package) => {
                    if &current < package.ident() {
                        debug!("Self updater installing newer Supervisor, {}",
                               package.ident());
                        tx.send(package).expect("Main thread has gone away!");
                        break;
                    } else {
                        debug!("Supervisor package found is not newer than ours");
                    }
                }
                Err(err) => {
                    warn!("Self updater failed to get latest, {}", err);
                }
            }
            trace!("Self updater delaying for {}s", period.as_secs_f64());
            tokiotime::delay_for(period).await;
        }
    }

    pub async fn updated(&mut self) -> Option<PackageInstall> {
        match self.rx.try_recv() {
            Ok(package) => Some(package),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Closed) => {
                debug!("Self updater has died, restarting...");
                self.rx = Self::init(self.into());
                None
            }
        }
    }
}
