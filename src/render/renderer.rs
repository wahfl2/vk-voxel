use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use ultraviolet::Mat4;
use vulkano::{memory::allocator::FastMemoryAllocator, VulkanLibrary, swapchain::{self, Surface, Swapchain, SwapchainCreateInfo, SwapchainCreationError, AcquireError, SwapchainPresentInfo}, command_buffer::{allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo}, PrimaryAutoCommandBuffer, AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents, CopyBufferInfoTyped, DrawIndirectCommand}, device::{physical::{PhysicalDevice, PhysicalDeviceType}, Device, DeviceCreateInfo, QueueCreateInfo, Queue, DeviceExtensions}, image::{view::ImageView, ImageUsage, SwapchainImage}, instance::{Instance, InstanceCreateInfo}, pipeline::{GraphicsPipeline, graphics::{input_assembly::InputAssemblyState, vertex_input::BuffersDefinition, viewport::{Viewport, ViewportState}}, Pipeline}, render_pass::{RenderPass, Framebuffer, FramebufferCreateInfo, Subpass}, shader::{ShaderModule}, sync::{GpuFuture, FlushError, self, FenceSignalFuture}, buffer::{DeviceLocalBuffer, BufferUsage, CpuAccessibleBuffer}, descriptor_set::{PersistentDescriptorSet, allocator::StandardDescriptorSetAllocator, WriteDescriptorSet}};
use vulkano_win::VkSurfaceBuild;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

use crate::event_handler::InputHandlerEvent;

use super::buffer::allocator::VertexChunkBuffer;
use super::buffer::buffer_queue::BufferQueueTask;
use super::shader_module::LoadFromPath;
use super::util::{GetWindow, RenderState};
use super::vertex::VertexRaw;

pub struct Renderer {
    pub vk_lib: Arc<VulkanLibrary>,
    pub vk_instance: Arc<Instance>,
    pub vk_surface: Arc<Surface>,
    pub vk_physical: Arc<PhysicalDevice>,
    pub vk_device: Arc<Device>,
    pub vk_queue: Arc<Queue>,
    pub vk_command_buffer_allocator: StandardCommandBufferAllocator,
    pub vk_descriptor_set_allocator: StandardDescriptorSetAllocator,
    pub vk_memory_allocator: FastMemoryAllocator,
    pub vk_swapchain: Arc<Swapchain>,
    pub vk_swapchain_images: Vec<Arc<SwapchainImage>>,
    pub vk_render_pass: Arc<RenderPass>,
    pub vk_frame_buffers: Vec<Arc<Framebuffer>>,
    pub vk_pipeline: Arc<GraphicsPipeline>,
    pub vk_descriptor_set: Arc<PersistentDescriptorSet>,

    pub viewport: Viewport,
    pub vertex_chunk_buffer: VertexChunkBuffer,
    pub indirect_buffer: Option<Arc<DeviceLocalBuffer<[DrawIndirectCommand]>>>,
    pub num_vertices: u32,
    pub cam_uniform: Option<Mat4>,

    pub vertex_shader: Arc<ShaderModule>,
    pub fragment_shader: Arc<ShaderModule>,

    fences: Vec<Option<Arc<FenceSignalFuture<Box<dyn GpuFuture>>>>>,
    previous_fence_i: usize,
}

impl Renderer {
    pub fn new(event_loop: &EventLoop<InputHandlerEvent>) -> Self {
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

        let vk_descriptor_set_allocator = StandardDescriptorSetAllocator::new(vk_device.clone());
        
        let vk_memory_allocator = FastMemoryAllocator::new_default(vk_device.clone());

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

        let data_buffer = CpuAccessibleBuffer::from_iter(
            &vk_memory_allocator,
            BufferUsage { storage_buffer: true, ..Default::default() },
            false,
            0..65536u32,
        ).unwrap();

        let vk_descriptor_set = PersistentDescriptorSet::new(
            &vk_descriptor_set_allocator,
            vk_pipeline.layout().set_layouts()[0].clone(),
            [WriteDescriptorSet::buffer(0, data_buffer.clone())]
        ).unwrap();

        let fences = vec![None; vk_swapchain_images.len()];

        Self {
            vk_lib,
            vk_instance,
            vk_surface,
            vk_physical,
            vk_device: vk_device.clone(),
            vk_queue,
            vk_command_buffer_allocator,
            vk_descriptor_set_allocator,
            vk_memory_allocator,
            vk_swapchain,
            vk_swapchain_images,
            vk_render_pass,
            vk_frame_buffers,
            vk_pipeline,
            vk_descriptor_set,

            viewport,
            vertex_chunk_buffer: VertexChunkBuffer::new(vk_device),
            indirect_buffer: None,
            num_vertices: 0,
            cam_uniform: None,

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

    pub fn get_command_buffer(&mut self, image_index: usize) -> Arc<PrimaryAutoCommandBuffer> {
        let v_buffer = self.vertex_chunk_buffer.get_buffer();
        
        let mut builder = AutoCommandBufferBuilder::primary(
            &self.vk_command_buffer_allocator,
            self.vk_queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        ).unwrap();

        if let Some(mat) = self.cam_uniform {
            let pc = PushConstants {
                camera: mat.into(),
            };
            builder.push_constants(self.vk_pipeline.layout().clone(), 0, pc);
        }
        
        let mut update_indirect_buffer = false;
        for task in self.vertex_chunk_buffer.queue.flush().into_iter() {
            update_indirect_buffer = true;
            match task {
                BufferQueueTask::Write(write) => {
                    builder.update_buffer(
                        write.data.into_boxed_slice(), 
                        v_buffer.clone(), 
                        write.start_idx.into()
                    ).unwrap();
                },
                BufferQueueTask::Transfer(transfer) => {
                    let copy = 
                        CopyBufferInfoTyped::buffers(transfer.src_buf, transfer.dst_buf);

                    builder.copy_buffer(copy).unwrap();
                },
            }
        }
        
        if update_indirect_buffer {
            let data = self.vertex_chunk_buffer.get_indirect_commands();
            if data.len() > 0 {
                self.indirect_buffer = Some(DeviceLocalBuffer::from_iter(
                    &self.vk_memory_allocator, 
                    data, 
                    BufferUsage {
                        indirect_buffer: true,
                        ..Default::default()
                    }, 
                    &mut builder
                ).unwrap());
            } else {
                self.indirect_buffer = None;
            }
        }

        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some([0.1, 0.1, 0.1, 1.0].into())],
                    ..RenderPassBeginInfo::framebuffer(self.vk_frame_buffers[image_index].clone())
                },
                SubpassContents::Inline,
            ).unwrap();

        if let Some(multi_buffer) = &self.indirect_buffer {
            builder.bind_pipeline_graphics(self.vk_pipeline.clone())
                .bind_vertex_buffers(0, v_buffer)
                .draw_indirect(multi_buffer.clone())
                .unwrap();
        }
            
        builder.end_render_pass().unwrap();

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
            Some(fence) => Box::new(fence),
        };

        // Wait for the previous future
        let future = previous_future
            // Wait to acquire swapchain image
            .join(acquire_future)
            // Execute command buffer 
            .then_execute(self.vk_queue.clone(), command_buffer)
            .unwrap()
            // Present overwritten swapchain image
            .then_swapchain_present(
                self.vk_queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.vk_swapchain.clone(), image_i)
            ).boxed() // Box it into a dyn GpuFuture for easier handling
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

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct PushConstants {
    camera: [[f32; 4]; 4],
}