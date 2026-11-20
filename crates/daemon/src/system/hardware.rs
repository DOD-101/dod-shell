use std::time::Duration;

use common::{Config, types::Percentage};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind};
use tokio::{
    spawn,
    sync::mpsc::{self, Receiver},
    task::spawn_blocking,
    time::interval,
};
use zbus::object_server::{InterfaceDerefMut, InterfaceRef};

use crate::{system::System, system_state::SystemState};

#[derive(Default, Debug)]
struct PollValue<T>(T);

impl<T: PartialEq> PollValue<T> {
    fn update<N: Into<T>>(&mut self, new: N) -> bool {
        let new = new.into();
        if self.0 == new {
            return false;
        }

        self.0 == new;

        true
    }
}
#[derive(Default)]
struct Updater<'a> {
    interface: InterfaceRef<System>,
    callbacks: Vec<Box<dyn Fn(InterfaceDerefMut<'a, System>)>>,
}

impl<'a> Updater<'a> {
    async fn update_all(&mut self) {
        let deref = tokio::time::timeout(Duration::from_millis(100), self.interface.get_mut())
            .await
            .inspect_err(|el| log::warn!("Failed to lock interface after {el} "))
            .unwrap();

        for u in self.callbacks.drain(.) {
            u(deref)
        }
    }
}

/// TODO: doc
#[allow(
    clippy::cast_precision_loss,
    reason = "Precision loss only occurs when calculating percentages, where we don't care since they are just for display."
)]
pub(super) fn hardware_thread(interface: InterfaceRef<System>, config_rx: Receiver<Config>) {
    let mut interval = interval(Duration::from_secs(1));

    let mut updater = Updater::default();

    let mut sys = sysinfo::System::default();

    let mut cpu: PollValue<Percentage> = PollValue::default();
    let mut memory: PollValue<Percentage> = PollValue::default();

    spawn(async move {
        let err;
        loop {
            interval.tick().await;

            sys.refresh_specifics(
                RefreshKind::nothing()
                    .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
                    .with_memory(MemoryRefreshKind::nothing().with_ram()),
            );

            if cpu.update(f64::from(sys.global_cpu_usage()) / 100.0) {
                let mut deref =
                    tokio::time::timeout(Duration::from_millis(100), interface.get_mut())
                        .await
                        .unwrap();

                updater
                    .0
                    .push(Box::new(|deref: InterfaceDerefMut<'_, System>| {
                        deref.cpu = cpu.0;
                    }));
            }

            if memory.update(sys.used_memory() as f64 / sys.total_memory() as f64)
                && let Err(e) = update_tx.send(HardwareUpdate::MemUsage(mem_usage.0)).await
            {
                err = e;
                break;
            }
        }
        log::error!("Failed to send HardwareUpdate: {err}");
    });

    update_rx
}

async fn interface_mut<'a>(interface: &'a InterfaceRef<System>) -> InterfaceDerefMut<'a, System> {
    tokio::time::timeout(Duration::from_millis(100), interface.get_mut())
        .await
        .inspect_err(|el| log::warn!("Failed to lock interface after {el} "))
}
