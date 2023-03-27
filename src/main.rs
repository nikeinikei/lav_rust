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
mod timer;
mod vulkan_backend;

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

    let backend = vulkan_backend::VulkanBackend::new(instance.clone(), surface.clone());

    let graphics = Arc::new(Mutex::new(graphics::Graphics::new(backend)));
    let timer = Arc::new(Mutex::new(timer::Timer::new()));

    let lua = rlua::Lua::new();
    lua.context(|ctx| {
        let lav = ctx.create_table().unwrap();

        let graphics_clone = graphics.clone();
        let graphics_set_clear_color = ctx
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

        let graphics_clone = graphics.clone();
        let graphics_rectangle = ctx
            .create_function_mut(move |_, (x, y, w, h)| {
                graphics_clone.lock().unwrap().rectangle(x, y, w, h);

                Ok(())
            })
            .unwrap();

        let graphics_clone = graphics.clone();
        let graphics_origin = ctx
            .create_function_mut(move |_, ()| {
                graphics_clone.lock().unwrap().origin();

                Ok(())
            })
            .unwrap();

        let graphics_clone = graphics.clone();
        let graphics_translate = ctx
            .create_function_mut(move |_, (x, y)| {
                graphics_clone.lock().unwrap().translate(x, y);

                Ok(())
            })
            .unwrap();

        let graphics_clone = graphics.clone();
        let graphics_rotate = ctx
            .create_function_mut(move |_, r| {
                graphics_clone.lock().unwrap().rotate(r);

                Ok(())
            })
            .unwrap();

        let graphics_clone = graphics.clone();
        let graphics_set_color = ctx
            .create_function_mut(move |_, (r, g, b, a)| {
                graphics_clone
                    .lock()
                    .unwrap()
                    .set_color(graphics::Color { r, g, b, a });

                Ok(())
            })
            .unwrap();

        let graphics_mod = ctx.create_table().unwrap();

        graphics_mod
            .set("setClearColor", graphics_set_clear_color)
            .unwrap();
        graphics_mod.set("present", graphics_present).unwrap();
        graphics_mod.set("rectangle", graphics_rectangle).unwrap();
        graphics_mod.set("origin", graphics_origin).unwrap();
        graphics_mod.set("translate", graphics_translate).unwrap();
        graphics_mod.set("rotate", graphics_rotate).unwrap();
        graphics_mod.set("setColor", graphics_set_color).unwrap();

        lav.set("graphics", graphics_mod).unwrap();

        let timer_clone = timer.clone();
        let timer_step = ctx.create_function_mut(move |_, ()| {
            timer_clone.lock().unwrap().step();

            Ok(())
        }).unwrap();

        let timer_clone = timer.clone();
        let timer_get_fps = ctx.create_function(move |_, ()| {
            let fps = timer_clone.lock().unwrap().get_fps();

            Ok(fps)
        }).unwrap();

        let timer_clone = timer.clone();
        let timer_get_time = ctx.create_function(move |_, ()| {
            let time = timer_clone.lock().unwrap().get_time();

            Ok(time)
        }).unwrap();

        let timer_clone = timer.clone();
        let timer_get_delta = ctx.create_function(move |_, ()| {
            let delta = timer_clone.lock().unwrap().get_delta();

            Ok(delta)
        }).unwrap();

        let timer_clone = timer.clone();
        let timer_sleep = ctx.create_function(move |_, duration| {
            timer_clone.lock().unwrap().sleep(duration);

            Ok(())
        }).unwrap();

        let timer_mod = ctx.create_table().unwrap();

        timer_mod.set("step", timer_step).unwrap();
        timer_mod.set("getFPS", timer_get_fps).unwrap();
        timer_mod.set("getTime", timer_get_time).unwrap();
        timer_mod.set("getDelta", timer_get_delta).unwrap();
        timer_mod.set("sleep", timer_sleep).unwrap();

        lav.set("timer", timer_mod).unwrap();

        ctx.globals().set("lav", lav).unwrap();
    });

    lua.context(|ctx| {
        let contents = fs::read_to_string("main.lua").expect("main.lua does not exist.");
        ctx.load(&contents).exec().unwrap();

        if let Ok(lav) = ctx.globals().get::<&str, Table>("lav") {
            if let Ok(load) = lav.get::<&str, Function>("load") {
                load.call::<_, ()>(()).unwrap();
            }
        }
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

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
                        if let Ok(draw) = lav.get::<&str, Function>("update") {
                            timer.lock().unwrap().step();
                            draw.call::<f64, ()>(timer.lock().unwrap().get_delta()).unwrap();
                        }
                        if let Ok(draw) = lav.get::<&str, Function>("draw") {
                            draw.call::<(), ()>(()).unwrap();
                        }
                    }
                });
            }
            _ => ()
        }
    });
}

fn main() {
    run_lav();
}
