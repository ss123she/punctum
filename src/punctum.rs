use winit::event_loop::EventLoop;

use crate::runner::Runner;

pub fn run() -> anyhow::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    #[cfg(target_arch = "wasm32")]
    console_log::init_with_level(log::Level::Info).unwrap_throw();

    let event_loop = EventLoop::with_user_event().build()?;
    let mut runner = Runner::new(&event_loop);

    #[cfg(not(target_arch = "wasm32"))]
    event_loop.run_app(&mut runner)?;

    #[cfg(target_arch = "wasm32")]
    event_loop.spawn_app(runner);

    Ok(())
}
