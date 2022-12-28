use std::sync::Arc;

use vulkano::VulkanLibrary;
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::image::view::ImageView;
use vulkano::image::{ImageUsage, SwapchainImage};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::device::{Device, DeviceCreateInfo, QueueCreateInfo, Queue, DeviceExtensions};
use vulkano::render_pass::{RenderPass, Framebuffer, FramebufferCreateInfo};
use vulkano::swapchain::{Surface, Swapchain, SwapchainCreateInfo};
use vulkano_win::VkSurfaceBuild;
use winit::event_loop::EventLoop;
use winit::window::{WindowBuilder, Window};

pub struct Renderer {
    pub vk_lib: Arc<VulkanLibrary>,
    pub vk_instance: Arc<Instance>,
    pub vk_surface: Arc<Surface>,
    pub vk_physical: Arc<PhysicalDevice>,
    pub vk_device: Arc<Device>,
    pub vk_queue: Arc<Queue>,
    pub vk_swapchain: Arc<Swapchain>,
    pub vk_swapchain_images: Vec<Arc<SwapchainImage>>,
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

        let capabilities = vk_physical
            .surface_capabilities(&vk_surface, Default::default())
            .expect("failed to get surface capabilities");

        let window = vk_surface.object().unwrap().clone().downcast::<Window>().unwrap();
        
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

        Self {
            vk_lib,
            vk_instance,
            vk_surface,
            vk_physical,
            vk_device,
            vk_queue,
            vk_swapchain,
            vk_swapchain_images,
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

    pub fn get_render_pass(&self) -> Arc<RenderPass> {
        vulkano::single_pass_renderpass!(
            self.vk_device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: self.vk_swapchain.image_format(),
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        ).unwrap()
    }

    pub fn get_framebuffers(&self, render_pass: &Arc<RenderPass>) -> Vec<Arc<Framebuffer>> {
        self.vk_swapchain_images
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
}