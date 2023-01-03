use std::sync::Arc;

use vulkano::memory::allocator::{StandardMemoryAllocator, PoolAllocator};
use vulkano::{VulkanLibrary, swapchain};
use vulkano::buffer::{CpuAccessibleBuffer, TypedBufferAccess, BufferAccess, BufferUsage};
use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo};
use vulkano::command_buffer::{PrimaryAutoCommandBuffer, AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents};
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::image::view::ImageView;
use vulkano::image::{ImageUsage, SwapchainImage};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::device::{Device, DeviceCreateInfo, QueueCreateInfo, Queue, DeviceExtensions};
use vulkano::pipeline::{GraphicsPipeline, ComputePipeline};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::render_pass::{RenderPass, Framebuffer, FramebufferCreateInfo, Subpass};
use vulkano::shader::ShaderModule;
use vulkano::swapchain::{Surface, Swapchain, SwapchainCreateInfo, SwapchainCreationError, AcquireError, SwapchainPresentInfo};
use vulkano::sync::{GpuFuture, FlushError};
use vulkano::sync;
use vulkano_win::VkSurfaceBuild;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

use super::shader_module::LoadFromPath;
use super::util::{GetWindow, RenderState, ExecuteFence};
use super::vertex::VertexRaw;

pub struct Renderer {
    pub vk_lib: Arc<VulkanLibrary>,
    pub vk_instance: Arc<Instance>,
    pub vk_surface: Arc<Surface>,
    pub vk_physical: Arc<PhysicalDevice>,
    pub vk_device: Arc<Device>,
    pub vk_queue: Arc<Queue>,
    pub vk_command_buffer_allocator: StandardCommandBufferAllocator,
    pub vk_memory_allocator: StandardMemoryAllocator,
    pub vk_swapchain: Arc<Swapchain>,
    pub vk_swapchain_images: Vec<Arc<SwapchainImage>>,
    pub vk_render_pass: Arc<RenderPass>,
    pub vk_frame_buffers: Vec<Arc<Framebuffer>>,
    pub vk_pipeline: Arc<GraphicsPipeline>,

    pub viewport: Viewport,
    vertex_buffer: Option<Arc<CpuAccessibleBuffer<[VertexRaw]>>>,
    pub num_vertices: u32,

    pub vertex_shader: Arc<ShaderModule>,
    pub fragment_shader: Arc<ShaderModule>,

    fences: Vec<Option<Arc<ExecuteFence>>>,
    previous_fence_i: usize,
}

impl Renderer {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let vk_lib = VulkanLibrary::new().expect("no local Vulkan library/DLL");

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };

        let vk_instance = Instance::new(
            vk_lib.clone(), 
            InstanceCreateInfo {
                enabled_extensions: vulkano_win::required_extensions(&vk_lib),
                ..Default::default()
            },
        ).expect("failed to create instance");

        let vk_surface = WindowBuilder::new()
            .build_vk_surface(event_loop, vk_instance.clone())
            .unwrap();
            
        let (vk_physical, queue_family_index) = Self::select_physical_device(
            &vk_instance, 
            &vk_surface, 
            &device_extensions
        );

        let (vk_device, mut queues) = Device::new(
            vk_physical.clone(),
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                enabled_extensions: device_extensions,
                ..Default::default()
            },
        ).expect("failed to create device");

        let vk_queue = queues.next().unwrap();

        let vk_command_buffer_allocator = StandardCommandBufferAllocator::new(
            vk_device.clone(), 
            StandardCommandBufferAllocatorCreateInfo::default()
        );
        
        let vk_memory_allocator = StandardMemoryAllocator::new_default(vk_device.clone());

        let capabilities = vk_physical
            .surface_capabilities(&vk_surface, Default::default())
            .expect("failed to get surface capabilities");

        let window = vk_surface.get_window().unwrap();
        
        let dimensions = window.inner_size();
        let composite_alpha = capabilities.supported_composite_alpha.iter().next().unwrap();
        let image_format = Some(
            vk_physical
                .surface_formats(&vk_surface, Default::default())
                .unwrap()[0]
                .0,
        );

        let (vk_swapchain, vk_swapchain_images) = Swapchain::new(
            vk_device.clone(),
            vk_surface.clone(),
            SwapchainCreateInfo {
                min_image_count: capabilities.min_image_count + 1,
                image_format,
                image_extent: dimensions.into(),
                image_usage: ImageUsage {
                    color_attachment: true,
                    ..Default::default()
                },
                composite_alpha,
                ..Default::default()
            },
        ).unwrap();
        
        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: window.inner_size().into(),
            depth_range: 0.0..1.0,
        };

        let vk_render_pass = Self::get_render_pass(vk_device.clone(), &vk_swapchain);
        let vk_frame_buffers = Self::get_framebuffers(&vk_swapchain_images, &vk_render_pass);

        let vertex_shader = ShaderModule::load(vk_device.clone(), "shader.vert");
        let fragment_shader = ShaderModule::load(vk_device.clone(), "shader.frag");

        let vk_pipeline = Self::get_pipeline(
            vk_device.clone(), 
            vertex_shader.clone(),
            fragment_shader.clone(),
            vk_render_pass.clone(),
            viewport.clone(),
        );

        let fences = vec![None; vk_swapchain_images.len()];

        Self {
            vk_lib,
            vk_instance,
            vk_surface,
            vk_physical,
            vk_device,
            vk_queue,
            vk_command_buffer_allocator,
            vk_memory_allocator,
            vk_swapchain,
            vk_swapchain_images,
            vk_render_pass,
            vk_frame_buffers,
            vk_pipeline,

            viewport,
            vertex_buffer: None,
            num_vertices: 0,

            vertex_shader,
            fragment_shader,

            fences,
            previous_fence_i: 0,
        }
    }

    fn select_physical_device(
        instance: &Arc<Instance>,
        surface: &Arc<Surface>,
        device_extensions: &DeviceExtensions,
    ) -> (Arc<PhysicalDevice>, u32) {
        instance
            .enumerate_physical_devices()
            .expect("could not enumerate devices")
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    // Find the first first queue family that is suitable.
                    // If none is found, `None` is returned to `filter_map`,
                    // which disqualifies this physical device.
                    .position(|(i, q)| {
                        q.queue_flags.graphics && p.surface_support(i as u32, &surface).unwrap_or(false)
                    })
                    .map(|q| (p, q as u32))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                _ => 4,
            })
            .expect("no device available")
    }

    fn get_pipeline(
        device: Arc<Device>,
        vs: Arc<ShaderModule>,
        fs: Arc<ShaderModule>,
        render_pass: Arc<RenderPass>,
        viewport: Viewport,
    ) -> Arc<GraphicsPipeline> {
        GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<VertexRaw>())
            .vertex_shader(vs.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([viewport]))
            .fragment_shader(fs.entry_point("main").unwrap(), ())
            .render_pass(Subpass::from(render_pass, 0).unwrap())
            .build(device)
            .unwrap()
    }

    fn get_render_pass(device: Arc<Device>, swapchain: &Arc<Swapchain>) -> Arc<RenderPass> {
        vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.image_format(),
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        ).unwrap()
    }

    fn get_framebuffers(images: &[Arc<SwapchainImage>], render_pass: &Arc<RenderPass>) -> Vec<Arc<Framebuffer>> {
        images
            .iter()
            .map(|image| {
                let view = ImageView::new_default(image.clone()).unwrap();
                Framebuffer::new(
                    render_pass.clone(),
                    FramebufferCreateInfo {
                        attachments: vec![view],
                        ..Default::default()
                    },
                )
                .unwrap()
            })
            .collect::<Vec<_>>()
    }

    /// Recreates the swapchain and frame buffers of this renderer.<br>
    /// Also sets the internal viewport dimensions to the dimensions of the surface.
    pub fn recreate_swapchain(&mut self) {
        let dimensions = self.vk_surface.get_window().unwrap().inner_size();
        self.viewport.dimensions = dimensions.into();

        let (new_swapchain, new_images) = match self.vk_swapchain.recreate(SwapchainCreateInfo {
            image_extent: dimensions.into(),
            ..self.vk_swapchain.create_info()
        }) {
            Ok(r) => r,
            Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
            Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
        };
        
        self.vk_swapchain = new_swapchain;
        self.vk_frame_buffers = Self::get_framebuffers(&new_images, &self.vk_render_pass);
    }

    /// Recreates the graphics pipeline of this renderer
    pub fn recreate_pipeline(&mut self) {
        self.vk_pipeline = Self::get_pipeline(
            self.vk_device.clone(), 
            self.vertex_shader.clone(),
            self.fragment_shader.clone(),
            self.vk_render_pass.clone(),
            self.viewport.clone(),
        );
    }

    pub fn overwrite_vbuffer(&mut self, vertices: &[VertexRaw]) {
        let mut usage = BufferUsage::empty();
        usage.vertex_buffer = true;

        self.vertex_buffer = Some(CpuAccessibleBuffer::from_iter(
            &self.vk_memory_allocator,
            usage, 
            false, 
            vertices.to_owned()
        ).unwrap());
    }

    // TODO: Move the growable buffer to its own struct
    pub fn add_vertices(&mut self, vertices: &[VertexRaw]) {
        let mut buffer_usage = BufferUsage::empty();
        buffer_usage.vertex_buffer = true;

        match &mut self.vertex_buffer {
            None => {
                // Create a large buffer to avoid constant recreation
                let mut buffer_len = 1000;
                while buffer_len < vertices.len() {
                    buffer_len *= 2;
                }
                let v_slice_len = vertices.len();

                let mut vec = vec![VertexRaw::default(); buffer_len];
                vec[..v_slice_len].copy_from_slice(vertices);
                self.num_vertices = v_slice_len as u32;

                self.vertex_buffer = Some(CpuAccessibleBuffer::from_iter(
                    &self.vk_memory_allocator,
                    buffer_usage,
                    false,
                    vec
                ).unwrap());
            },
            Some(buffer) => {
                let len = buffer.len();
                let total = self.num_vertices as u64 + vertices.len() as u64;
                if total > len {
                    let mut buffer_len = len * 2;
                    while buffer_len < total {
                        buffer_len *= 2;
                    }

                    let mut vec = Vec::with_capacity(buffer_len as usize);
                    vec.extend_from_slice(&buffer.read().unwrap());
                    vec.extend_from_slice(vertices);
                    vec.resize_with(buffer_len as usize, Default::default);

                    self.vertex_buffer = Some(CpuAccessibleBuffer::from_iter(
                        &self.vk_memory_allocator,
                        buffer_usage,
                        false,
                        vec
                    ).unwrap());
                } else {
                    match &mut buffer.write() {
                        Ok(write) => {
                            let num = self.num_vertices as usize;
                            write[num..(num + vertices.len())].copy_from_slice(vertices);
                            self.num_vertices += vertices.len() as u32;
                        },
                        Err(e) => { panic!("Could not gain write access to buffer. {}", e) }
                    }
                }
            }
        }
    }

    pub fn get_command_buffer(&self, image_index: usize) -> Arc<PrimaryAutoCommandBuffer> {
        let mut builder = AutoCommandBufferBuilder::primary(
            &self.vk_command_buffer_allocator,
            self.vk_queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,  // don't forget to write the correct buffer usage
        )
        .unwrap();

        let v_buffer = match &self.vertex_buffer {
            Some(v) => v,
            None => panic!("No vertex buffer bound to this renderer.")
        };

        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some([0.1, 0.1, 0.1, 1.0].into())],
                    ..RenderPassBeginInfo::framebuffer(self.vk_frame_buffers[image_index].clone())
                },
                SubpassContents::Inline,
            )
            .unwrap()
            .bind_pipeline_graphics(self.vk_pipeline.clone())
            .bind_vertex_buffers(0, v_buffer.to_owned())
            .draw(self.num_vertices, 1, 0, 0)
            .unwrap()
            .end_render_pass()
            .unwrap();

        Arc::new(builder.build().unwrap())
    }

    pub fn render(&mut self) -> RenderState {
        let mut state = RenderState::Ok;

        let (image_i, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.vk_swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    return RenderState::OutOfDate;
                }
                Err(e) => panic!("Failed to acquire next image: {:?}", e),
            };
        if suboptimal { state = RenderState::Suboptimal; }

        let command_buffer = self.get_command_buffer(image_i as usize);

        // Overwrite the oldest fence and take control for drawing
        if let Some(image_fence) = &mut self.fences[image_i as usize] {
            image_fence.cleanup_finished();
        }

        // Get the previous future
        let previous_future = match self.fences[self.previous_fence_i].clone() {
            None => sync::now(self.vk_device.clone()).boxed(),
            Some(fence) => fence.boxed(),
        };

        // Wait for the previous future to finish, and then start rendering the next frame.
        let future = previous_future
            .join(acquire_future)
            .then_execute(self.vk_queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(
                self.vk_queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.vk_swapchain.clone(), image_i)
            )
            .then_signal_fence_and_flush();

        self.fences[image_i as usize] = match future {
            Ok(value) => Some(Arc::new(value)),
            Err(FlushError::OutOfDate) => {
                state = RenderState::OutOfDate;
                None
            }
            Err(e) => {
                println!("Failed to flush future: {:?}", e);
                None
            }
        };

        self.previous_fence_i = image_i as usize;
        
        state
    }
}