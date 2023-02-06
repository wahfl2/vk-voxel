use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use ultraviolet::{Mat4, IVec2};
use vulkano::{memory::allocator::StandardMemoryAllocator, VulkanLibrary, swapchain::{self, Surface, Swapchain, SwapchainCreateInfo, SwapchainCreationError, AcquireError, SwapchainPresentInfo, ColorSpace, PresentMode}, command_buffer::{allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo}, PrimaryAutoCommandBuffer, AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents, DrawIndirectCommand}, device::{physical::{PhysicalDevice, PhysicalDeviceType}, Device, DeviceCreateInfo, QueueCreateInfo, Queue, DeviceExtensions, Features}, image::{view::{ImageView, ImageViewCreateInfo}, ImageUsage, SwapchainImage, AttachmentImage, ImageSubresourceRange}, instance::{Instance, InstanceCreateInfo}, pipeline::{GraphicsPipeline, graphics::{input_assembly::InputAssemblyState, viewport::{Viewport, ViewportState}, rasterization::{RasterizationState, CullMode, FrontFace}, depth_stencil::DepthStencilState}, Pipeline, PipelineBindPoint, StateMode, PipelineLayout, layout::PipelineLayoutCreateInfo}, render_pass::{RenderPass, Framebuffer, FramebufferCreateInfo, Subpass}, shader::ShaderModule, sync::{GpuFuture, FlushError, self, FenceSignalFuture}, buffer::{DeviceLocalBuffer, BufferUsage}, descriptor_set::{allocator::StandardDescriptorSetAllocator, PersistentDescriptorSet, WriteDescriptorSet, layout::{DescriptorSetLayout, DescriptorSetLayoutCreateInfo}}, sampler::{Sampler, SamplerCreateInfo, Filter, SamplerAddressMode}, format::Format};
use vulkano_win::VkSurfaceBuild;
use winit::{event_loop::EventLoop, window::WindowBuilder, dpi::PhysicalSize};

use crate::{event_handler::UserEvent, world::block_data::StaticBlockData};

use super::{buffer::allocator::VertexChunkBuffer, texture::TextureAtlas, shaders::{LoadFromPath, ShaderPair}, util::{GetWindow, RenderState}, mesh::{renderable::Renderable, chunk_render::ChunkRender}};

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

    pub viewport: Viewport,
    pub vertex_chunk_buffer: VertexChunkBuffer,
    pub indirect_buffer: Option<Arc<DeviceLocalBuffer<[DrawIndirectCommand]>>>,
    pub num_vertices: u32,
    pub cam_uniform: Option<Mat4>,
    pub texture_atlas: TextureAtlas,
    pub texture_sampler: Arc<Sampler>,
    pub atlas_descriptor_set: Option<Arc<PersistentDescriptorSet>>,
    pub vertex_descriptor_set: Option<Arc<PersistentDescriptorSet>>,

    pub upload_texture_atlas: bool,

    pub general_shader: ShaderPair,
    pub block_shader: ShaderPair,

    fences: Vec<Option<Arc<FenceSignalFuture<Box<dyn GpuFuture>>>>>,
    previous_fence_i: usize,
}

impl Renderer {
    pub fn new(event_loop: &EventLoop<UserEvent>) -> Self {
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
            
        let (vk_physical, queue_family_indices) = Self::select_physical_device(
            &vk_instance, 
            &vk_surface, 
            &device_extensions
        );

        let (vk_device, mut queues) = Device::new(
            vk_physical.clone(),
            DeviceCreateInfo {
                enabled_features: Features {
                    shader_storage_texel_buffer_array_dynamic_indexing: true,
                    multi_draw_indirect: true,
                    ..Default::default()
                },
                queue_create_infos: vec![
                    QueueCreateInfo {
                        queue_family_index: queue_family_indices.graphics,
                        ..Default::default()
                    }
                ],
                enabled_extensions: device_extensions,
                ..Default::default()
            },
        ).expect("failed to create device");

        let vk_graphics_queue = queues.next().unwrap();

        let vk_command_buffer_allocator = StandardCommandBufferAllocator::new(
            vk_device.clone(), 
            StandardCommandBufferAllocatorCreateInfo::default()
        );

        let vk_descriptor_set_allocator = StandardDescriptorSetAllocator::new(vk_device.clone());
        
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
                image_color_space: ColorSpace::SrgbNonLinear,
                present_mode: PresentMode::Immediate,
                ..Default::default()
            },
        ).unwrap();
        
        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: window.inner_size().into(),
            depth_range: 0.0..1.0,
        };

        let vk_render_pass = Self::get_render_pass(vk_device.clone(), &vk_swapchain);
        let vk_frame_buffers = Self::get_framebuffers(&vk_swapchain_images, dimensions, &vk_render_pass, &vk_memory_allocator);

        let general_shader = ShaderPair::load(vk_device.clone(), "shader");
        let block_shader = ShaderPair::load(vk_device.clone(), "specialized/block_shader");

        let pipelines = Self::get_pipelines(
            vk_device.clone(), 
            &block_shader,
            &general_shader,
            vk_render_pass.clone(),
            viewport.clone(),
        );

        let texture_atlas = Self::load_texture_folder_into_atlas("./resources");
        let texture_sampler = Sampler::new(
            vk_device.clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Nearest,
                min_filter: Filter::Nearest,
                address_mode: [SamplerAddressMode::ClampToEdge; 3],
                ..Default::default()
            },
        ).unwrap();

        let fences = vec![None; vk_swapchain_images.len()];

        Self {
            vk_lib,
            vk_instance,
            vk_surface,
            vk_physical,
            vk_device: vk_device.clone(),
            vk_graphics_queue,
            vk_command_buffer_allocator,
            vk_descriptor_set_allocator,
            vk_memory_allocator,
            vk_swapchain,
            vk_swapchain_images,
            vk_render_pass,
            vk_frame_buffers,
            pipelines,

            viewport,
            vertex_chunk_buffer: VertexChunkBuffer::new(vk_device),
            indirect_buffer: None,
            num_vertices: 0,
            cam_uniform: None,
            texture_atlas,
            texture_sampler,
            atlas_descriptor_set: None,
            vertex_descriptor_set: None,

            upload_texture_atlas: true,

            block_shader,
            general_shader,

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
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .filter_map(|p| {
                let mut graphics = None;
                for (i, q) in p.queue_family_properties().iter().enumerate() {
                    if q.queue_flags.graphics && p.surface_support(i as u32, surface).unwrap() {
                        graphics = Some(i);
                    }
                }

                Some((p, QueueFamilyIndices {
                    graphics: graphics? as u32,
                }))
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
        general_shader: &ShaderPair,
        render_pass: Arc<RenderPass>,
        viewport: Viewport,
    ) -> Pipelines {
        let block_quads = GraphicsPipeline::start()
            .rasterization_state(RasterizationState {
                cull_mode: StateMode::Fixed(CullMode::Back),
                front_face: StateMode::Fixed(FrontFace::CounterClockwise),
                ..Default::default()
            })
            .vertex_shader(block_shader.vertex.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([viewport.clone()]))
            .fragment_shader(block_shader.fragment.entry_point("main").unwrap(), ())
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap();

        let decorations  = GraphicsPipeline::start()
            .rasterization_state(RasterizationState {
                cull_mode: StateMode::Fixed(CullMode::None),
                ..Default::default()
            })
            .vertex_shader(general_shader.vertex.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([viewport]))
            .fragment_shader(general_shader.fragment.entry_point("main").unwrap(), ())
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .render_pass(Subpass::from(render_pass, 1).unwrap())
            .build(device)
            .unwrap();

        Pipelines { block_quads, decorations }
    }

    fn get_render_pass(device: Arc<Device>, swapchain: &Arc<Swapchain>) -> Arc<RenderPass> {
        vulkano::ordered_passes_renderpass!(
            device.clone(),
            attachments: {
                blocks: {
                    load: Clear,
                    store: Store,
                    format: swapchain.image_format(),
                    samples: 1,
                },
                decorations: {
                    load: Load,
                    store: Store,
                    format: swapchain.image_format(),
                    samples: 1,
                },
                depth: {
                    load: Clear,
                    store: DontCare,
                    format: Format::D16_UNORM,
                    samples: 1,
                }
            },
            passes: [
                {// Render blocks
                    color: [blocks],
                    depth_stencil: {depth},
                    input: []
                },

                {// Render decorations
                    color: [decorations],
                    depth_stencil: {depth},
                    input: [blocks]
                }
            ]
        ).unwrap()
    }

    fn get_framebuffers(
        images: &[Arc<SwapchainImage>], 
        dimensions: PhysicalSize<u32>,
        render_pass: &Arc<RenderPass>, 
        allocator: &StandardMemoryAllocator
    ) -> Vec<Arc<Framebuffer>> {
        let depth_buffer = ImageView::new_default(
            AttachmentImage::transient(allocator, dimensions.into(), Format::D16_UNORM).unwrap()
        ).unwrap();

        let image_fmt = Format::B8G8R8A8_UNORM;
        let range = ImageSubresourceRange::from_parameters(image_fmt, 1, 1);
        let intermediate_image = ImageView::new(
            AttachmentImage::transient(allocator, dimensions.into(), image_fmt).unwrap(),
            ImageViewCreateInfo {
                format: Some(image_fmt),
                subresource_range: range.clone(),
                usage: ImageUsage {
                    input_attachment: true,
                    color_attachment: true,
                    ..Default::default()
                },
                ..Default::default()
            }
        ).unwrap();

        images
            .iter()
            .map(|image| {
                let swapchain_view = ImageView::new_default(image.clone()).unwrap();

                Framebuffer::new(
                    render_pass.clone(),
                    FramebufferCreateInfo {
                        attachments: vec![
                            intermediate_image.clone(), 
                            swapchain_view, 
                            depth_buffer.clone()
                        ],
                        ..Default::default()
                    },
                ).unwrap()
            })
            .collect::<Vec<_>>()
    }

    fn load_texture_folder_into_atlas(folder_path: &str) -> TextureAtlas {
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
        self.vk_frame_buffers = Self::get_framebuffers(
            &new_images, 
            dimensions, 
            &self.vk_render_pass, 
            &self.vk_memory_allocator
        );
    }

    /// Recreates the graphics pipeline of this renderer
    pub fn recreate_pipeline(&mut self) {
        self.pipelines = Self::get_pipelines(
            self.vk_device.clone(), 
            &self.block_shader,
            &self.general_shader,
            self.vk_render_pass.clone(),
            self.viewport.clone(),
        );
    }

    pub fn upload_chunk(&mut self, pos: IVec2, chunk: impl ChunkRender, block_data: &StaticBlockData) {
        self.vertex_chunk_buffer.push_chunk(pos, chunk, &self.texture_atlas, block_data);
    }

    /// Get a command buffer that will upload `self`'s texture atlas to the GPU when executed.
    /// 
    /// The atlas is stored in `self`'s `PersistentDescriptorSet`
    pub fn get_upload_command_buffer(&mut self) -> Arc<PrimaryAutoCommandBuffer> {
        let mut builder = AutoCommandBufferBuilder::primary(
            &self.vk_command_buffer_allocator, 
            self.vk_graphics_queue.queue_family_index(), 
            CommandBufferUsage::OneTimeSubmit,
        ).unwrap();

        let texture = self.texture_atlas.get_texture(
            &self.vk_memory_allocator, 
            &mut builder
        );

        let layout = self.pipelines.block_quads.layout().set_layouts()[0].to_owned();
        let set = PersistentDescriptorSet::new(
            &self.vk_descriptor_set_allocator,
            layout, 
            [WriteDescriptorSet::image_view_sampler(
                0, 
                texture, 
                self.texture_sampler.clone(),
            )]
        ).unwrap();
        self.atlas_descriptor_set = Some(set);

        builder.build().unwrap().into()
    }

    /// Get a command buffer that will render the scene.
    pub fn get_render_command_buffer(&mut self, image_index: usize) -> Arc<PrimaryAutoCommandBuffer> {
        let just_swapped = self.vertex_chunk_buffer.update();
        // Invalidate vertex descriptor set
        if just_swapped { self.vertex_descriptor_set = None; }
        
        let v_buffer = self.vertex_chunk_buffer.get_buffer();

        let vert_desc_set = match &self.vertex_descriptor_set {
            Some(v) => v.clone(),
            None => {
                let layout = &self.pipelines.block_quads.layout().set_layouts()[1];
                let vertex_set = PersistentDescriptorSet::new(
                    &self.vk_descriptor_set_allocator, 
                    layout.clone(), 
                    [WriteDescriptorSet::buffer(
                        0, 
                        v_buffer,
                    )]
                ).unwrap();
                self.vertex_descriptor_set = Some(vertex_set.clone());
                vertex_set
            }
        };
        
        let mut builder = AutoCommandBufferBuilder::primary(
            &self.vk_command_buffer_allocator,
            self.vk_graphics_queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        ).unwrap();

        builder.bind_descriptor_sets(
            PipelineBindPoint::Graphics, 
            self.pipelines.block_quads.layout().to_owned(), 
            0, 
            vec![
                self.atlas_descriptor_set.as_ref().unwrap().to_owned(),
                vert_desc_set.to_owned()
            ],
        );

        const FACE_LIGHTING: FaceLighting = FaceLighting {
            positive: [0.6, 1.0, 0.8],
            negative: [0.6, 0.4, 0.8],
            _pad1: 0,
            _pad2: 0,
        };
        
        if let Some(mat) = self.cam_uniform.take() {
            let pc = PushConstants {
                camera: mat.into(),
                face_lighting: FACE_LIGHTING,
            };
            builder.push_constants(self.pipelines.block_quads.layout().clone(), 0, pc);
            builder.push_constants(self.pipelines.decorations.layout().clone(), 0, pc);
        }

        if just_swapped {
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
                    clear_values: vec![
                        // Color
                        Some([0.1, 0.1, 0.1, 1.0].into()),
                        // Load
                        None,
                        // Depth buffer
                        Some(1.0.into())
                    ],
                    ..RenderPassBeginInfo::framebuffer(self.vk_frame_buffers[image_index].clone())
                },
                SubpassContents::Inline,
            ).unwrap();

        if let Some(multi_buffer) = &self.indirect_buffer {
            builder.bind_pipeline_graphics(self.pipelines.block_quads.clone())
                .draw_indirect(multi_buffer.clone())
                .unwrap();
        }
            
        builder.end_render_pass().unwrap();

        Arc::new(builder.build().unwrap())
    }

    /// Get the command buffers to be executed on the GPU this frame.
    fn get_command_buffers(&mut self, image_index: usize) -> Vec<Arc<PrimaryAutoCommandBuffer>> {
        let mut ret = Vec::new();
        if self.upload_texture_atlas || self.atlas_descriptor_set.is_none() {
            self.upload_texture_atlas = false;
            ret.push(self.get_upload_command_buffer());
        }
        ret.push(self.get_render_command_buffer(image_index));
        ret
    }

    /// Renders the scene
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
            exec = exec.then_execute(self.vk_graphics_queue.clone(), command_buffer).unwrap().boxed();
        }
            
        // Present overwritten swapchain image
        let present_future = exec.then_swapchain_present(
            self.vk_graphics_queue.clone(),
            SwapchainPresentInfo::swapchain_image_index(self.vk_swapchain.clone(), image_i)
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
pub struct PushConstants {
    camera: [[f32; 4]; 4],
    face_lighting: FaceLighting,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct FaceLighting {
    positive: [f32; 3],
    _pad1: u32,
    negative: [f32; 3],
    _pad2: u32,
}

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
    block_quads: Arc<GraphicsPipeline>,
    decorations: Arc<GraphicsPipeline>,
}