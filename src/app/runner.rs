use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::{AppConfig, app::state::State};

pub struct Runner {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    state: Option<State>,
    config: Option<AppConfig>,
}

impl Runner {
    pub fn new(config: AppConfig, _event_loop: &EventLoop<State>) -> Self {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(_event_loop.create_proxy());

        Self {
            config: Some(config),
            state: None,
            #[cfg(target_arch = "wasm32")]
            proxy,
        }
    }
}

impl ApplicationHandler<State> for Runner {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() || self.config.is_none() {
            return;
        }

        let config = self.config.take().expect("AppConfig is missing");

        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes()
            .with_title(config.window.title)
            .with_inner_size(PhysicalSize::new(config.window.width, config.window.height));

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;

            let window = web_sys::window().unwrap();
            let document = window.document().unwrap();

            let target_id = config.window.canvas_id.as_deref().unwrap_or("canvas");

            let canvas = document
                .get_element_by_id(target_id)
                .and_then(|el| el.dyn_into::<web_sys::HtmlCanvasElement>().ok())
                .unwrap_or_else(|| {
                    let canvas = document
                        .create_element("canvas")
                        .unwrap()
                        .dyn_into::<web_sys::HtmlCanvasElement>()
                        .unwrap();
                    canvas.set_id(target_id);
                    document.body().unwrap().append_child(&canvas).unwrap();
                    canvas
                });

            window_attributes = window_attributes.with_canvas(Some(canvas));
        }

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.state = Some(
                pollster::block_on(State::new(window, &config.compute, &config.render)).unwrap(),
            );
        }

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(
                        proxy
                            .send_event(
                                State::new(window, config.render)
                                    .await
                                    .expect("Unable to create State on canvas")
                            )
                            .is_ok()
                    );
                });
            }
        }
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
        #[cfg(target_arch = "wasm32")]
        {
            let size = event.window().inner_size();
            event.resize(size.width, size.height);
            event.window().request_redraw();
        }
        self.state = Some(event);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        if window_id != state.window().id() {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("{e}");
                        event_loop.exit();
                    }
                }
            }

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => event_loop.exit(),

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = &self.state {
            state.window().request_redraw();
        }
    }
}
