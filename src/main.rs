use std::{
    fs,
    sync::{Arc, Mutex},
};

use rlua::{Function, Table};
use vulkano::{
    instance::{Instance, InstanceCreateInfo},
    VulkanLibrary,
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod graphics;
mod gfx;

fn run_lav() {
    let event_loop = EventLoop::new();

    let library = VulkanLibrary::new().unwrap();
    let required_extensions = vulkano_win::required_extensions(&library);

    let instance = Instance::new(
        library,
        InstanceCreateInfo {
            enabled_extensions: required_extensions,
            enumerate_portability: true,
            ..Default::default()
        },
    )
    .unwrap();

    let surface = WindowBuilder::new()
        .build_vk_surface(&event_loop, instance.clone())
        .unwrap();

    let graphics = Arc::new(Mutex::new(graphics::Graphics::new(
        instance.clone(),
        surface.clone(),
    )));

    let lua = rlua::Lua::new();
    lua.context(|ctx| {
        let lav = ctx.create_table().unwrap();

        let graphics_mod = ctx.create_table().unwrap();

        let graphics_clone = graphics.clone();
        let graphics_set_clear_color_function = ctx
            .create_function_mut(move |_, (r, g, b, a)| {
                graphics_clone.lock().unwrap().set_clear_color(r, g, b, a);

                Ok(())
            })
            .unwrap();

        let graphics_clone = graphics.clone();
        let graphics_present = ctx
            .create_function_mut(move |_, ()| {
                graphics_clone.lock().unwrap().present();

                Ok(())
            })
            .unwrap();

        graphics_mod
            .set("setClearColor", graphics_set_clear_color_function)
            .unwrap();
        graphics_mod.set("present", graphics_present).unwrap();

        lav.set("graphics", graphics_mod).unwrap();

        ctx.globals().set("lav", lav).unwrap();
    });

    lua.context(|ctx| {
        let contents = fs::read_to_string("main.lua").expect("main lua does not exist.");
        ctx.load(&contents).exec().unwrap();

        if let Ok(lav) = ctx.globals().get::<&str, Table>("lav") {
            if let Ok(draw) = lav.get::<&str, Function>("load") {
                draw.call::<_, ()>(()).unwrap();
            }
        }
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                graphics.lock().unwrap().request_swapchain_recreation();
            }
            Event::RedrawEventsCleared => {
                lua.context(|ctx| {
                    if let Ok(lav) = ctx.globals().get::<&str, Table>("lav") {
                        if let Ok(draw) = lav.get::<&str, Function>("draw") {
                            draw.call::<_, ()>(()).unwrap();
                        }
                    }
                });
            }
            _ => (),
        }
    });
}

fn main() {
    run_lav();
}
