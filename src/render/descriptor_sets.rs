use std::sync::Arc;

use vulkano::{image::{view::ImageView, ImmutableImage, AttachmentImage}, sampler::Sampler, buffer::{Subbuffer, Buffer, BufferCreateInfo, BufferUsage}, descriptor_set::allocator::StandardDescriptorSetAllocator, pipeline::{Pipeline, PipelineBindPoint, GraphicsPipeline}, memory::allocator::{StandardMemoryAllocator, AllocationCreateInfo, MemoryUsage}, command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer}};

use super::{buffer::{upload::UploadDescriptorSet}, mesh::quad::BlockQuad, renderer::{FaceLighting, Pipelines, View}, texture::TextureAtlas, util::CreateInfoConvenience, brick::{brickmap::Brickmap, brickgrid::{Brickgrid, self}}};

pub type ImageViewSampler = (Arc<ImageView<ImmutableImage>>, Arc<Sampler>);

pub struct DescriptorSets {
    pub atlas: UploadDescriptorSet<ImageViewSampler>,
    pub atlas_map: UploadDescriptorSet<Subbuffer<[[f32; 4]]>>,

    pub view: UploadDescriptorSet<Subbuffer<View>>,
    pub face_lighting: UploadDescriptorSet<Subbuffer<FaceLighting>>,

    pub brickmap: UploadDescriptorSet<Subbuffer<[Brickmap]>>,
    pub brickgrid: UploadDescriptorSet<Subbuffer<Brickgrid>>,
}

impl DescriptorSets {
    pub fn new(
        memory_allocator: &StandardMemoryAllocator,
        descriptor_set_allocator: &StandardDescriptorSetAllocator,
        cbb: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,

        pipelines: &Pipelines,
        texture_atlas: &TextureAtlas,
        sampler: Arc<Sampler>,
        view: View,
        face_lighting: FaceLighting,
    ) -> Self {
        let raytracing_layouts = pipelines.raytracing.layout().set_layouts();

        let atlas = UploadDescriptorSet::new(
            descriptor_set_allocator,
            raytracing_layouts[0].clone(), 0,
            (texture_atlas.get_texture(memory_allocator, cbb), sampler)
        );
        
        let atlas_map_storage_buffer = super::util::make_device_only_buffer_slice(
            memory_allocator, cbb, 
            BufferUsage::STORAGE_BUFFER, 
            texture_atlas.uvs.iter().map(|uv| { uv.to_raw() })
        );
        
        let atlas_map = UploadDescriptorSet::new(
            descriptor_set_allocator,
            raytracing_layouts[1].clone(), 0,
            atlas_map_storage_buffer
        );

        let view_storage_buffer = super::util::make_device_only_buffer_sized(
            memory_allocator, cbb, 
            BufferUsage::STORAGE_BUFFER | BufferUsage::UNIFORM_BUFFER, 
            View::default()
        );

        let view = UploadDescriptorSet::new(
            descriptor_set_allocator,
            raytracing_layouts[2].clone(), 0,
            view_storage_buffer
        );

        let face_lighting_storage_buffer = super::util::make_device_only_buffer_sized(
            memory_allocator, cbb, 
            BufferUsage::STORAGE_BUFFER | BufferUsage::UNIFORM_BUFFER, 
            face_lighting
        );

        let face_lighting = UploadDescriptorSet::new(
            descriptor_set_allocator,
            raytracing_layouts[3].clone(), 0,
            face_lighting_storage_buffer
        );

        let brickmap = UploadDescriptorSet::new(
            descriptor_set_allocator,
            raytracing_layouts[4].clone(), 0,
            Buffer::new_slice(
                memory_allocator, 
                BufferCreateInfo::usage(BufferUsage::STORAGE_BUFFER), 
                AllocationCreateInfo::usage(MemoryUsage::Upload), 
                1
            ).unwrap()
        );

        let brickgrid = UploadDescriptorSet::new(
            descriptor_set_allocator,
            raytracing_layouts[5].clone(), 0,
            Buffer::new_sized(
                memory_allocator, 
                BufferCreateInfo::usage(BufferUsage::STORAGE_BUFFER), 
                AllocationCreateInfo::usage(MemoryUsage::Upload), 
            ).unwrap()
        );

        Self {
            atlas,
            atlas_map,
            view,
            face_lighting,
            brickmap,
            brickgrid,
        }
    }

    pub fn bind_raytracing(
        &self, 
        cbb: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        raytracing_pipeline: Arc<GraphicsPipeline>,
    ) {
        cbb.bind_descriptor_sets(
            PipelineBindPoint::Graphics, 
            raytracing_pipeline.layout().clone(), 
            0, 
            vec![
                self.atlas.set.clone(),
                self.atlas_map.set.clone(),
                self.view.set.clone(),
                self.face_lighting.set.clone(),
                self.brickmap.set.clone(),
                self.brickgrid.set.clone(),
            ]
        );
    }
}