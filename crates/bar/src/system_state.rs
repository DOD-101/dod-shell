use std::time::Duration;

use hyprland::shared::HyprDataActive;

use relm4::SharedState;
use sysinfo::System;
use time::OffsetDateTime;

pub static SYSTEM_STATE: SharedState<SystemState> = SharedState::new();

pub fn init_update_loop() {
    relm4::spawn_blocking(|| {
        loop {
            SYSTEM_STATE.write().update();

            std::thread::sleep(Duration::from_millis(500));
        }
    });
}

// Struct  containing all of the system state
//
// Provides the [SystemState::update] method for updating all of the state.
#[derive(Debug)]
pub struct SystemState {
    sys: System,
    data: SystemStateData,
}

#[derive(Debug, Clone)]
pub struct SystemStateData {
    pub total_mem: u64,
    pub cpu_usage: f32,
    pub mem_usage: f32,
    pub used_mem: u64,
    pub time: OffsetDateTime,
    pub workspace: i32,
}

impl Default for SystemState {
    fn default() -> Self {
        let sys = System::new_all();
        let mut state = Self {
            sys,
            data: SystemStateData {
                total_mem: 0,
                cpu_usage: 0.0,
                mem_usage: 0.0,
                used_mem: 0,
                time: OffsetDateTime::UNIX_EPOCH,
                workspace: 0,
            },
        };

        state.data.total_mem = state.sys.total_memory();
        state.update();

        state
    }
}

impl SystemState {
    #[allow(clippy::cast_precision_loss)]
    pub fn update(&mut self) {
        self.sys.refresh_all();
        self.data.cpu_usage = self.sys.global_cpu_usage();
        self.data.used_mem = self.sys.used_memory();
        self.data.mem_usage = self.data.used_mem as f32 / self.data.total_mem as f32;
        self.data.time = OffsetDateTime::now_local().expect("Failed to get time offset.");
        self.data.workspace = hyprland::data::Workspace::get_active().unwrap().id;

        log::trace!("State updated");
    }

    pub fn get_data(&self) -> &SystemStateData {
        &self.data
    }
}
