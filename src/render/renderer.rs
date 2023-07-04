use std::sync::{Arc, Mutex};

use bytemuck::{Pod, Zeroable};
use ultraviolet::Mat4;
use vulkano::{
    buffer::{BufferUsage, Subbuffer},
    command_buffer::{
        allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo},
        AutoCommandBufferBuilder, CommandBufferUsage, DrawIndirectCommand,
        PrimaryAutoCommandBuffer, RenderPassBeginInfo, SubpassContents,
    },
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator,
        layout::{
            DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateInfo,
            DescriptorType,
        },
    },
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, Features, Queue, QueueCreateInfo, QueueFlags,
    },
    image::{view::ImageView, ImageUsage, SwapchainImage},
    instance::{Instance, InstanceCreateInfo},
    memory::allocator::StandardMemoryAllocator,
    pipeline::{
        graphics::{
            color_blend::ColorBlendState,
            vertex_input::Vertex,
            viewport::{Viewport, ViewportState},
        },
        layout::PipelineLayoutCreateInfo,
        GraphicsPipeline, PipelineLayout,
    },
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
    sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo},
    shader::ShaderStages,
    swapchain::{
        self, AcquireError, ColorSpace, PresentMode, Surface, Swapchain, SwapchainCreateInfo,
        SwapchainCreationError, SwapchainPresentInfo,
    },
    sync::{self, future::FenceSignalFuture, FlushError, GpuFuture},
    VulkanLibrary,
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    dpi::PhysicalSize,
    event_loop::EventLoop,
    window::{CursorGrabMode, WindowBuilder},
};

use crate::{
    event_handler::UserEvent,
    world::{block_data::StaticBlockData, world_blocks::WorldBlocks},
};

use super::{
    buffer::vertex_buffer::ChunkVertexBuffer,
    descriptor_sets::DescriptorSets,
    shaders::ShaderPair,
    texture::TextureAtlas,
    util::{GetWindow, ProgramInfo, RenderState},
    vertex::Vertex2D,
};

pub struct Renderer {
    pub vk_lib: Arc<VulkanLibrary>,
    pub vk_instance: Arc<Instance>,
    pub vk_surface: Arc<Surface>,
    pub vk_physical: Arc<PhysicalDevice>,
    pub vk_device: Arc<Device>,
    pub vk_graphics_queue: Arc<Queue>,
    pub vk_command_buffer_allocator: StandardCommandBufferAllocator,
    pub vk_descriptor_set_allocator: StandardDescriptorSetAllocator,
    pub vk_memory_allocator: StandardMemoryAllocator,
    pub vk_swapchain: Arc<Swapchain>,
    pub vk_swapchain_images: Vec<Arc<SwapchainImage>>,
    pub vk_render_pass: Arc<RenderPass>,
    pub vk_frame_buffers: Vec<Arc<Framebuffer>>,
    pub pipelines: Pipelines,

    pub view: View,
    pub viewport: Viewport,
    pub vertex_buffer: ChunkVertexBuffer,
    pub fullscreen_quad: Option<Subbuffer<[Vertex2D; 3]>>,
    pub indirect_buffer: Option<Subbuffer<[DrawIndirectCommand]>>,
    pub num_vertices: u32,
    pub cam_uniform: Option<Mat4>,
    pub texture_atlas: TextureAtlas,
    pub texture_sampler: Arc<Sampler>,
    pub program_info: ProgramInfo,
    pub descriptor_sets: DescriptorSets,

    pub upload_texture_atlas: bool,

    pub block_shader: ShaderPair,

    // Complex type could be simplified using the `type` keyword.
    fences: Vec<Option<Arc<FenceSignalFuture<Box<dyn GpuFuture>>>>>,
    previous_fence_i: usize,
}

impl Renderer {
    pub fn new(
        event_loop: &EventLoop<UserEvent>,
        texture_atlas: TextureAtlas,
        block_data: &StaticBlockData,
    ) -> Self {
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
        )
        .expect("failed to create instance");

        let vk_surface = WindowBuilder::new()
            .with_inner_size(PhysicalSize::new(1600, 900))
            .with_title("VK Voxel")
            .build_vk_surface(event_loop, vk_instance.clone())
            .unwrap();

        let window = vk_surface.get_window().unwrap();
        window.set_cursor_grab(CursorGrabMode::Confined).unwrap();
        window.set_cursor_visible(false);

        let (vk_physical, queue_family_indices) =
            Self::select_physical_device(&vk_instance, &vk_surface, &device_extensions);

        let (vk_device, mut queues) = Device::new(
            vk_physical.clone(),
            DeviceCreateInfo {
                enabled_features: Features {
                    shader_storage_texel_buffer_array_dynamic_indexing: true,
                    multi_draw_indirect: true,
                    ..Default::default()
                },
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index: queue_family_indices.graphics,
                    ..Default::default()
                }],
                enabled_extensions: device_extensions,
                ..Default::default()
            },
        ).expect("failed to create device");

        let vk_graphics_queue = queues.next().unwrap();

        let vk_command_buffer_allocator = StandardCommandBufferAllocator::new(
            vk_device.clone(),
            StandardCommandBufferAllocatorCreateInfo::default(),
        );

        let vk_descriptor_set_allocator = StandardDescriptorSetAllocator::new(vk_device.clone());

        let vk_memory_allocator = StandardMemoryAllocator::new_default(vk_device.clone());

        let capabilities = vk_physical
            .surface_capabilities(&vk_surface, Default::default())
            .expect("failed to get surface capabilities");

        let window = vk_surface.get_window().unwrap();

        let dimensions = window.inner_size();
        let composite_alpha = capabilities
            .supported_composite_alpha
            .into_iter()
            .next()
            .unwrap();
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
                image_usage: ImageUsage::COLOR_ATTACHMENT,
                composite_alpha,
                image_color_space: ColorSpace::SrgbNonLinear,
                present_mode: PresentMode::Immediate,
                ..Default::default()
            },
        )
        .unwrap();

        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: window.inner_size().into(),
            depth_range: 0.0..1.0,
        };

        let vertex_buffer = ChunkVertexBuffer::new(&vk_memory_allocator);

        let vk_render_pass = Self::get_render_pass(vk_device.clone(), &vk_swapchain);

        let vk_frame_buffers = Self::get_framebuffers(&vk_swapchain_images, &vk_render_pass);

        let block_shader = ShaderPair::load(vk_device.clone(), "specialized/block_shader");

        let pipelines = Self::get_pipelines(
            vk_device.clone(),
            &block_shader,
            vk_render_pass.clone(),
            viewport.clone(),
        );

        let texture_sampler = Sampler::new(
            vk_device.clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Nearest,
                min_filter: Filter::Nearest,
                address_mode: [SamplerAddressMode::ClampToEdge; 3],
                ..Default::default()
            },
        )
        .unwrap();

        let program_info = ProgramInfo::new();

        let mut cbb = AutoCommandBufferBuilder::primary(
            &vk_command_buffer_allocator,
            vk_graphics_queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        let view = View {
            resolution: [window.inner_size().width, window.inner_size().height],
            ..Default::default()
        };

        let descriptor_sets = DescriptorSets::new(
            &vk_memory_allocator,
            &vk_descriptor_set_allocator,
            &mut cbb,
            &pipelines,
            &texture_atlas,
            block_data,
            texture_sampler.clone(),
        );

        let future = sync::now(vk_device.clone())
            .then_execute(vk_graphics_queue.clone(), cbb.build().unwrap())
            .unwrap()
            .then_signal_fence_and_flush()
            .unwrap();

        future.wait(None).unwrap();

        let fences = vec![None; vk_swapchain_images.len()];

        Self {
            vk_lib,
            vk_instance,
            vk_surface,
            vk_physical,
            vk_device,
            vk_graphics_queue,
            vk_command_buffer_allocator,
            vk_descriptor_set_allocator,
            vk_memory_allocator,
            vk_swapchain,
            vk_swapchain_images,
            vk_render_pass,
            vk_frame_buffers,
            pipelines,

            view,
            viewport,
            vertex_buffer,
            fullscreen_quad: None,
            indirect_buffer: None,
            num_vertices: 0,
            cam_uniform: None,
            texture_atlas,
            texture_sampler,
            program_info,
            descriptor_sets,

            upload_texture_atlas: true,

            block_shader,

            fences,
            previous_fence_i: 0,
        }
    }

    /// Select the best available phyisical device.
    ///
    /// Returns the device and queue family index.
    fn select_physical_device(
        instance: &Arc<Instance>,
        surface: &Arc<Surface>,
        device_extensions: &DeviceExtensions,
    ) -> (Arc<PhysicalDevice>, QueueFamilyIndices) {
        instance
            .enumerate_physical_devices()
            .expect("could not enumerate devices")
            .filter(|p| p.supported_extensions().contains(device_extensions)) // Reference created here, but complier immediately dereferences it either way.
            .filter_map(|p| {
                let mut graphics = None;
                for (i, q) in p.queue_family_properties().iter().enumerate() {
                    if q.queue_flags.contains(QueueFlags::GRAPHICS)
                        && p.surface_support(i as u32, surface).unwrap()
                    {
                        graphics = Some(i);
                    }
                }

                Some((
                    p,
                    QueueFamilyIndices {
                        graphics: graphics? as u32,
                    },
                ))
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

    /// Get the graphics pipeline
    fn get_pipelines(
        device: Arc<Device>,
        block_shader: &ShaderPair,
        render_pass: Arc<RenderPass>,
        viewport: Viewport,
    ) -> Pipelines {
        fn create_fragment_layout_type(
            device: Arc<Device>,
            descriptor_type: DescriptorType,
        ) -> Arc<DescriptorSetLayout> {
            DescriptorSetLayout::new(
                device,
                DescriptorSetLayoutCreateInfo {
                    bindings: [(
                        0,
                        DescriptorSetLayoutBinding {
                            stages: ShaderStages::FRAGMENT,
                            ..DescriptorSetLayoutBinding::descriptor_type(descriptor_type)
                        },
                    )]
                    .into(),
                    ..Default::default()
                },
            )
            .unwrap()
        }

        let set_layouts = vec![
            create_fragment_layout_type(device.clone(), DescriptorType::CombinedImageSampler), // 0
            create_fragment_layout_type(device.clone(), DescriptorType::StorageBuffer),        // 1
            create_fragment_layout_type(device.clone(), DescriptorType::UniformBuffer),        // 2
            create_fragment_layout_type(device.clone(), DescriptorType::UniformBuffer),        // 3
            create_fragment_layout_type(device.clone(), DescriptorType::StorageBuffer),        // 4
            create_fragment_layout_type(device.clone(), DescriptorType::StorageBuffer),        // 5
            create_fragment_layout_type(device.clone(), DescriptorType::StorageBuffer),        // 6
            create_fragment_layout_type(device.clone(), DescriptorType::StorageBuffer),        // 7
        ];

        let layout = PipelineLayout::new(
            device.clone(),
            PipelineLayoutCreateInfo {
                set_layouts,
                ..Default::default()
            },
        )
        .unwrap();

        let raytracing = GraphicsPipeline::start()
            .vertex_shader(block_shader.vertex.entry_point("main").unwrap(), ())
            .fragment_shader(block_shader.fragment.entry_point("main").unwrap(), ())
            .color_blend_state(ColorBlendState::new(1))
            .vertex_input_state(Vertex2D::per_vertex())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([
                viewport, // Redundant clone.
            ]))
            .render_pass(Subpass::from(render_pass, 0).unwrap()) // Redundant clone for `render_pass`
            .with_pipeline_layout(device, layout.clone())
            .unwrap();

        Pipelines { raytracing, layout }
    }

    fn get_render_pass(device: Arc<Device>, swapchain: &Arc<Swapchain>) -> Arc<RenderPass> {
        vulkano::single_pass_renderpass!(
            device, // Redundant clone
            attachments: {
                blocks: {
                    load: Clear,
                    store: Store,
                    format: swapchain.image_format(),
                    samples: 1,
                },
            },
            pass: {
                color: [blocks],
                depth_stencil: {},
            }
        )
        .unwrap()
    }

    fn get_framebuffers(
        images: &[Arc<SwapchainImage>],
        render_pass: &Arc<RenderPass>,
    ) -> Vec<Arc<Framebuffer>> {
        images
            .iter()
            .map(|image| {
                let swapchain_view = ImageView::new_default(image.clone()).unwrap();

                Framebuffer::new(
                    render_pass.clone(),
                    FramebufferCreateInfo {
                        attachments: vec![swapchain_view],
                        ..Default::default()
                    },
                )
                .unwrap()
            })
            .collect::<Vec<_>>()
    }

    pub fn load_texture_folder_into_atlas(folder_path: &str) -> TextureAtlas {
        TextureAtlas::from_folder(folder_path)
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
        self.pipelines = Self::get_pipelines(
            self.vk_device.clone(),
            &self.block_shader,
            self.vk_render_pass.clone(),
            self.viewport.clone(),
        );
    }

    /// Get a command buffer that will upload `self`'s texture atlas to the GPU when executed.
    ///
    /// The atlas is stored in `self`'s `PersistentDescriptorSet`
    pub fn get_upload_command_buffer(&mut self) -> Arc<PrimaryAutoCommandBuffer> {
        let mut builder = AutoCommandBufferBuilder::primary(
            &self.vk_command_buffer_allocator,
            self.vk_graphics_queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        let texture = self
            .texture_atlas
            .get_texture(&self.vk_memory_allocator, &mut builder);

        self.descriptor_sets.atlas.replace(
            &self.vk_descriptor_set_allocator,
            (texture, self.texture_sampler.clone()),
        );

        self.descriptor_sets.atlas_map.replace(
            &self.vk_descriptor_set_allocator,
            super::util::make_device_only_buffer_slice(
                &self.vk_memory_allocator,
                &mut builder,
                BufferUsage::STORAGE_BUFFER,
                self.texture_atlas.uvs.clone(),
            ),
        );

        builder.build().unwrap().into()
    }

    /// Get a command buffer that will render the scene.
    pub fn get_render_command_buffer(
        &mut self,
        image_index: usize,
    ) -> Arc<PrimaryAutoCommandBuffer> {
        let mut builder = AutoCommandBufferBuilder::primary(
            &self.vk_command_buffer_allocator,
            self.vk_graphics_queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        let (brickgrid_swapped, texture_pointer_swapped, brickmaps_swapped) =
            self.vertex_buffer.update();
        if brickmaps_swapped {
            self.descriptor_sets.brickmap.replace(
                &self.vk_descriptor_set_allocator,
                self.vertex_buffer.brickmap_buffer.get_buffer(),
            );
        }

        if texture_pointer_swapped {
            self.descriptor_sets.texture_buffer.replace(
                &self.vk_descriptor_set_allocator,
                self.vertex_buffer.texture_pointer_buffer.get_buffer(),
            );
        }

        if brickgrid_swapped {
            self.descriptor_sets.brickgrid.replace(
                &self.vk_descriptor_set_allocator,
                self.vertex_buffer.brickgrid_buffer.get_buffer(),
            );
        }

        self.program_info.frame_number += 1;
        self.descriptor_sets.program_info.replace(
            &self.vk_descriptor_set_allocator,
            super::util::make_device_only_buffer_sized(
                &self.vk_memory_allocator,
                &mut builder,
                BufferUsage::STORAGE_BUFFER.union(BufferUsage::UNIFORM_BUFFER),
                self.program_info,
            ),
        );

        if let Some(mat) = self.cam_uniform.take() {
            self.view.camera = mat.as_array().to_owned();

            self.descriptor_sets.view.replace(
                &self.vk_descriptor_set_allocator,
                super::util::make_device_only_buffer_sized(
                    &self.vk_memory_allocator,
                    &mut builder,
                    BufferUsage::STORAGE_BUFFER | BufferUsage::UNIFORM_BUFFER,
                    self.view,
                ),
            );
        }

        // Redundant `if let` useage.
        if self.fullscreen_quad.is_none() {
            self.fullscreen_quad = Some(super::util::make_device_only_buffer_sized(
                &self.vk_memory_allocator,
                &mut builder,
                BufferUsage::VERTEX_BUFFER,
                [
                    Vertex2D {
                        position: [-1.0, -1.0],
                    },
                    Vertex2D {
                        position: [3.0, -1.0],
                    },
                    Vertex2D {
                        position: [-1.0, 3.0],
                    },
                ],
            ));
        }

        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![
                        // Color
                        Some([0.1, 0.1, 0.1, 1.0].into()),
                        Some([0.0, 0.0, 0.0, 0.0].into()),
                        Some([0.0, 0.0, 0.0, 0.0].into()),
                        Some(1.0.into()),
                        Some(1.0.into()),
                    ],
                    ..RenderPassBeginInfo::framebuffer(self.vk_frame_buffers[image_index].clone())
                },
                SubpassContents::Inline,
            )
            .unwrap();

        // Render blocks
        builder.bind_pipeline_graphics(self.pipelines.raytracing.clone());
        self.descriptor_sets.bind_raytracing(&mut builder, self.pipelines.layout.clone());
        builder
            .bind_vertex_buffers(0, self.fullscreen_quad.clone().unwrap())
            .draw(3, 1, 0, 0)
            .unwrap();

        builder.end_render_pass().unwrap();

        Arc::new(builder.build().unwrap())
    }

    fn update_vertex_buffers(
        &mut self,
        world_blocks: Arc<Mutex<WorldBlocks>>,
        block_data: &StaticBlockData,
    ) {
        let mut lock = world_blocks.lock().unwrap();
        let updated_chunks = lock.updated_chunks.drain(..).collect::<Vec<_>>();
        for chunk_pos in updated_chunks {
            if let Some(chunk) = lock.loaded_chunks.get(&chunk_pos) {
                self.vertex_buffer.insert_chunk(chunk, block_data);
            } else {
                self.vertex_buffer.remove_chunk(chunk_pos);
            }
        }
    }

    /// Get the command buffers to be executed on the GPU this frame.
    fn get_command_buffers(&mut self, image_index: usize) -> Vec<Arc<PrimaryAutoCommandBuffer>> {
        let mut ret = Vec::new();
        if self.upload_texture_atlas {
            self.upload_texture_atlas = false;
            ret.push(self.get_upload_command_buffer());
        }
        ret.push(self.get_render_command_buffer(image_index));
        ret
    }

    /// Renders the scene
    pub fn render(
        &mut self,
        world_blocks: Arc<Mutex<WorldBlocks>>,
        block_data: &StaticBlockData,
    ) -> RenderState {
        self.update_vertex_buffers(world_blocks, block_data);

        let mut state = RenderState::Ok;

        let (image_i, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.vk_swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    return RenderState::OutOfDate;
                }
                Err(e) => panic!("Failed to acquire next image: {:?}", e),
            };
        if suboptimal {
            state = RenderState::Suboptimal;
        }

        let command_buffers = self.get_command_buffers(image_i as usize);

        // Overwrite the oldest fence and take control for drawing
        if let Some(image_fence) = &mut self.fences[image_i as usize] {
            image_fence.cleanup_finished();
        }

        // Get the previous future
        let previous_future = match self.fences[self.previous_fence_i].clone() {
            None => sync::now(self.vk_device.clone()).boxed(),
            Some(fence) => fence.boxed(),
        };

        // Wait for the previous future as well as the swapchain image acquire
        let join = previous_future.join(acquire_future);

        let mut exec = join.boxed();
        for command_buffer in command_buffers.into_iter() {
            // Execute command buffers in order
            exec = exec
                .then_execute(self.vk_graphics_queue.clone(), command_buffer)
                .unwrap()
                .boxed();
        }

        // Present overwritten swapchain image
        let present_future = exec
            .then_swapchain_present(
                self.vk_graphics_queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.vk_swapchain.clone(), image_i),
            )
            .boxed() // Box it into a dyn GpuFuture for easier handling
            .then_signal_fence_and_flush();

        self.fences[image_i as usize] = match present_future {
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
pub struct View {
    camera: [f32; 16],
    resolution: [u32; 2],
    fov: f32,
}

impl Default for View {
    fn default() -> Self {
        Self {
            camera: Mat4::identity().as_array().to_owned(),
            resolution: [0; 2],
            fov: 90.0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct FaceLighting {
    positive: [f32; 3],
    _pad1: u32,
    negative: [f32; 3],
    _pad2: u32,
}

const BLOCK_FACE_LIGHTING: FaceLighting = FaceLighting {
    positive: [0.6, 1.0, 0.8],
    negative: [0.6, 0.4, 0.8],
    _pad1: 0,
    _pad2: 0,
};

impl Default for FaceLighting {
    fn default() -> Self {
        Self {
            positive: [1.0, 1.0, 1.0],
            negative: [1.0, 1.0, 1.0],
            _pad1: Default::default(),
            _pad2: Default::default(),
        }
    }
}

struct QueueFamilyIndices {
    graphics: u32,
}

pub struct Pipelines {
    pub raytracing: Arc<GraphicsPipeline>,
    pub layout: Arc<PipelineLayout>,
}
