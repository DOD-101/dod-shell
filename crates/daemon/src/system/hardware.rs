use std::time::Duration;

use common::{Config, types::Percentage};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind};
use tokio::{spawn, sync::mpsc::Receiver, task::spawn_blocking, time::interval};
use zbus::object_server::{InterfaceDerefMut, InterfaceRef};

use crate::system::System;

#[derive(Default, Debug)]
struct PollValue<T>(T);

impl<T: PartialEq> PollValue<T> {
    fn update<N: Into<T>>(&mut self, new: N) -> bool {
        let new = new.into();
        if self.0 == new {
            return false;
        }

        self.0 = new;

        true
    }
}
struct Updater {
    interface: InterfaceRef<System>,
    callbacks: Vec<Box<dyn for<'r, 'd> Fn(&'r mut InterfaceDerefMut<'d, System>) + Send>>,
}

impl Updater {
    fn new(interface: InterfaceRef<System>, capacity: usize) -> Self {
        Self {
            interface,
            callbacks: Vec::with_capacity(capacity),
        }
    }

    async fn update_all(&mut self) {
        let mut deref = tokio::time::timeout(Duration::from_millis(100), self.interface.get_mut())
            .await
            .inspect_err(|el| log::warn!("Failed to lock interface after {el} "))
            .unwrap();

        for u in self.callbacks.drain(0..) {
            u(&mut deref);
        }
    }
}

/// TODO: doc
#[allow(
    clippy::cast_precision_loss,
    reason = "Precision loss only occurs when calculating percentages, where we don't care since they are just for display."
)]
pub(super) fn hardware_thread(interface: InterfaceRef<System>, _config_rx: Receiver<Config>) {
    let mut interval = interval(Duration::from_secs(1));

    let mut updater = Updater::new(interface, 1);

    let mut sys = sysinfo::System::default();

    let mut cpu: PollValue<Percentage> = PollValue::default();
    let mut memory: PollValue<Percentage> = PollValue::default();

    spawn(async move {
        loop {
            interval.tick().await;

            sys = spawn_blocking(move || {
                sys.refresh_specifics(
                    RefreshKind::nothing()
                        .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
                        .with_memory(MemoryRefreshKind::nothing().with_ram()),
                );
                sys
            })
            .await
            .unwrap();

            if cpu.update(f64::from(sys.global_cpu_usage()) / 100.0) {
                updater.callbacks.push(Box::new(
                    move |deref: &mut InterfaceDerefMut<'_, System>| {
                        deref.cpu = cpu.0;
                    },
                ));
            }

            if memory.update(sys.used_memory() as f64 / sys.total_memory() as f64) {
                updater.callbacks.push(Box::new(
                    move |deref: &mut InterfaceDerefMut<'_, System>| {
                        deref.memory = memory.0;
                    },
                ));
            }

            updater.update_all().await;
        }
    });
}
