use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess},
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage,
        RenderingAttachmentInfo, RenderingInfo,
    },
    device::{
        physical::PhysicalDeviceType, Device, DeviceCreateInfo, DeviceExtensions, Features, Queue,
        QueueCreateInfo,
    },
    image::{view::ImageView, ImageAccess, ImageUsage, SwapchainImage},
    impl_vertex,
    instance::Instance,
    memory::allocator::StandardMemoryAllocator,
    pipeline::{
        graphics::{
            input_assembly::InputAssemblyState,
            render_pass::PipelineRenderingCreateInfo,
            vertex_input::BuffersDefinition,
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline,
    },
    render_pass::{LoadOp, StoreOp},
    swapchain::{
        acquire_next_image, AcquireError, Surface, Swapchain, SwapchainCreateInfo,
        SwapchainCreationError, SwapchainPresentInfo,
    },
    sync::{self, FlushError, GpuFuture},
    Version,
};

use winit::window::Window;

struct Color {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
struct Vertex {
    position: [f32; 2],
}
impl_vertex!(Vertex, position);

pub struct Graphics {
    surface: Arc<Surface>,
    device: Arc<Device>,
    swapchain: Arc<Swapchain>,
    viewport: Viewport,
    queue: Arc<Queue>,
    attachment_image_views: Vec<Arc<ImageView<SwapchainImage>>>,
    command_buffer_allocator: StandardCommandBufferAllocator,
    pipeline: Arc<GraphicsPipeline>,
    vertex_buffer: Arc<CpuAccessibleBuffer<[Vertex]>>,
    recreate_swapchain: bool,
    clear_color: Color,
}

fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage>],
    viewport: &mut Viewport,
) -> Vec<Arc<ImageView<SwapchainImage>>> {
    let dimensions = images[0].dimensions().width_height();
    viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];

    images
        .iter()
        .map(|image| ImageView::new_default(image.clone()).unwrap())
        .collect::<Vec<_>>()
}

impl Graphics {
    pub fn new(instance: Arc<Instance>, surface: Arc<Surface>) -> Graphics {
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };

        let (physical_device, queue_family_index) = instance
            .enumerate_physical_devices()
            .unwrap()
            .filter(|p| p.api_version() >= Version::V1_3)
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(i, q)| {
                        q.queue_flags.graphics
                            && p.surface_support(i as u32, &surface).unwrap_or(false)
                    })
                    .map(|i| (p, i as u32))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                PhysicalDeviceType::Other => 4,
                _ => 5,
            })
            .expect("no suitable physical device found");

        println!(
            "Using device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
        );

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                enabled_extensions: device_extensions,

                enabled_features: Features {
                    dynamic_rendering: true,
                    ..Features::empty()
                },

                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],

                ..Default::default()
            },
        )
        .unwrap();

        let queue = queues.next().unwrap();

        let (swapchain, images) = {
            let surface_capabilities = device
                .physical_device()
                .surface_capabilities(&surface, Default::default())
                .unwrap();

            let image_format = Some(
                device
                    .physical_device()
                    .surface_formats(&surface, Default::default())
                    .unwrap()[0]
                    .0,
            );

            let window = surface.object().unwrap().downcast_ref::<Window>().unwrap();

            Swapchain::new(
                device.clone(),
                surface.clone(),
                SwapchainCreateInfo {
                    min_image_count: surface_capabilities.min_image_count,
                    image_format,
                    image_extent: window.inner_size().into(),
                    image_usage: ImageUsage {
                        color_attachment: true,
                        ..ImageUsage::empty()
                    },
                    composite_alpha: surface_capabilities
                        .supported_composite_alpha
                        .iter()
                        .next()
                        .unwrap(),
                    ..Default::default()
                },
            )
            .unwrap()
        };

        let memory_allocator = StandardMemoryAllocator::new_default(device.clone());

        let vertices = [
            Vertex {
                position: [-0.5, -0.25],
            },
            Vertex {
                position: [0.0, 0.5],
            },
            Vertex {
                position: [0.25, -0.1],
            },
        ];
        let vertex_buffer = CpuAccessibleBuffer::from_iter(
            &memory_allocator,
            BufferUsage {
                vertex_buffer: true,
                ..BufferUsage::empty()
            },
            false,
            vertices,
        )
        .unwrap();

        mod vs {
            vulkano_shaders::shader! {
                ty: "vertex",
                src: "
                    #version 450
                    layout(location = 0) in vec2 position;
                    void main() {
                        gl_Position = vec4(position, 0.0, 1.0);
                    }
                "
            }
        }

        mod fs {
            vulkano_shaders::shader! {
                ty: "fragment",
                src: "
                    #version 450
                    layout(location = 0) out vec4 f_color;
                    void main() {
                        f_color = vec4(1.0, 1.0, 1.0, 1.0);
                    }
                "
            }
        }

        let vs = vs::load(device.clone()).unwrap();
        let fs = fs::load(device.clone()).unwrap();

        let pipeline = GraphicsPipeline::start()
            .render_pass(PipelineRenderingCreateInfo {
                color_attachment_formats: vec![Some(swapchain.image_format())],
                ..Default::default()
            })
            .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
            .input_assembly_state(InputAssemblyState::new())
            .vertex_shader(vs.entry_point("main").unwrap(), ())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(fs.entry_point("main").unwrap(), ())
            .build(device.clone())
            .unwrap();

        let mut viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [0.0, 0.0],
            depth_range: 0.0..1.0,
        };

        let attachment_image_views = window_size_dependent_setup(&images, &mut viewport);

        let command_buffer_allocator =
            StandardCommandBufferAllocator::new(device.clone(), Default::default());

        let clear_color = Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        };

        Graphics {
            surface,
            device,
            swapchain,
            viewport,
            attachment_image_views,
            command_buffer_allocator,
            queue,
            pipeline,
            vertex_buffer,
            recreate_swapchain: false,
            clear_color,
        }
    }

    pub fn request_swapchain_recreation(&mut self) {
        self.recreate_swapchain = true;
    }

    pub fn set_clear_color(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.clear_color.r = r;
        self.clear_color.g = g;
        self.clear_color.b = b;
        self.clear_color.a = a;
    }

    pub fn present(&mut self) {
        if self.recreate_swapchain {
            let window = self
                .surface
                .object()
                .unwrap()
                .downcast_ref::<Window>()
                .unwrap();

            let (new_swapchain, new_images) = match self.swapchain.recreate(SwapchainCreateInfo {
                image_extent: window.inner_size().into(),
                ..self.swapchain.create_info()
            }) {
                Ok(r) => r,
                Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
                Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
            };

            self.swapchain = new_swapchain;
            self.attachment_image_views =
                window_size_dependent_setup(&new_images, &mut self.viewport);
            self.recreate_swapchain = false;
        }

        let (image_index, suboptimal, acquire_future) =
            match acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    return;
                }
                Err(e) => panic!("Failed to acquire next image: {:?}", e),
            };

        if suboptimal {
            self.recreate_swapchain = true;
        }

        let mut builder = AutoCommandBufferBuilder::primary(
            &self.command_buffer_allocator,
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        builder
            .begin_rendering(RenderingInfo {
                color_attachments: vec![Some(RenderingAttachmentInfo {
                    load_op: LoadOp::Clear,
                    store_op: StoreOp::Store,
                    clear_value: Some(
                        [
                            self.clear_color.r,
                            self.clear_color.g,
                            self.clear_color.b,
                            self.clear_color.a,
                        ]
                        .into(),
                    ),
                    ..RenderingAttachmentInfo::image_view(
                        self.attachment_image_views[image_index as usize].clone(),
                    )
                })],
                ..Default::default()
            })
            .unwrap()
            .set_viewport(0, [self.viewport.clone()])
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .draw(self.vertex_buffer.len() as u32, 1, 0, 0)
            .unwrap()
            .end_rendering()
            .unwrap();

        let command_buffer = builder.build().unwrap();

        let mut previous_frame_end = Some(sync::now(self.device.clone()).boxed());

        let future = previous_frame_end
            .take()
            .unwrap()
            .join(acquire_future)
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_index),
            )
            .then_signal_fence_and_flush();

        match future {
            Ok(_) => {}
            Err(FlushError::OutOfDate) => {
                self.recreate_swapchain = true;
            }
            Err(e) => panic!("failed to flush future: {:?}", e),
        }
    }
}
