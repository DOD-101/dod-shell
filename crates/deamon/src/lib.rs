pub mod system_state;

// #[derive(Default, Debug)]
// struct State {
//     state: u32,
// }
//
// #[interface(
//     name = "dod.shell.Deamon1",
//     proxy(
//         gen_blocking = false,
//         default_path = "/dod/shell/Deamon",
//         default_service = "dod.shell.Deamon"
//     )
// )]
// impl State {
//     async fn greet(&self, name: &str) -> String {
//         format!("Hello {name}!")
//     }
//
//     #[zbus(property)]
//     async fn state_state(&self) -> u32 {
//         self.state
//     }
//
//     #[zbus(property)]
//     async fn set_state_state(&mut self, state: u32) {
//         self.state = state;
//     }
// }
