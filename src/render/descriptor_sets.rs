use std::sync::Arc;

use vulkano::{image::{view::ImageView, ImmutableImage, AttachmentImage}, sampler::Sampler, buffer::{Subbuffer, Buffer, BufferCreateInfo, BufferUsage}, descriptor_set::allocator::StandardDescriptorSetAllocator, pipeline::{Pipeline, PipelineBindPoint, GraphicsPipeline}, memory::allocator::{StandardMemoryAllocator, AllocationCreateInfo, MemoryUsage}, command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer}};

use super::{buffer::upload::{UploadDescriptorSet, UploadDescriptorSetArray}, mesh::quad::BlockQuad, renderer::{FaceLighting, Pipelines, View}, texture::TextureAtlas, util::CreateInfoConvenience};


pub type ImageViewSampler = (Arc<ImageView<ImmutableImage>>, Arc<Sampler>);

pub struct DescriptorSets {
    pub atlas: UploadDescriptorSet<ImageViewSampler>,
    pub atlas_map: UploadDescriptorSet<Subbuffer<[[f32; 4]]>>,

    pub block_quads: UploadDescriptorSet<Subbuffer<[BlockQuad]>>,
    pub attachments: UploadDescriptorSetArray<Arc<ImageView<AttachmentImage>>, 4>,

    pub view: UploadDescriptorSet<Subbuffer<View>>,
    pub face_lighting: UploadDescriptorSet<Subbuffer<FaceLighting>>,
}

impl DescriptorSets {
    pub fn new(
        memory_allocator: &StandardMemoryAllocator,
        descriptor_set_allocator: &StandardDescriptorSetAllocator,
        cbb: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,

        pipelines: &Pipelines,
        texture_atlas: &TextureAtlas,
        sampler: Arc<Sampler>,
        attachment_images: [Arc<ImageView<AttachmentImage>>; 4],
        face_lighting: FaceLighting,
    ) -> Self {
        let block_layouts = pipelines.block_quads.layout().set_layouts();
        let final_layouts = pipelines.fin.layout().set_layouts();

        let atlas = UploadDescriptorSet::new(
            descriptor_set_allocator,
            block_layouts[0].clone(), 0,
            (texture_atlas.get_texture(memory_allocator, cbb), sampler)
        );
        
        let atlas_map_storage_buffer = super::util::make_device_only_buffer_slice(
            memory_allocator, cbb, 
            BufferUsage::STORAGE_BUFFER, 
            texture_atlas.uvs.iter().map(|uv| { uv.to_raw() })
        );
        
        let atlas_map = UploadDescriptorSet::new(
            descriptor_set_allocator,
            block_layouts[4].clone(), 0,
            atlas_map_storage_buffer
        );

        let block_quads = UploadDescriptorSet::new(
            descriptor_set_allocator,
            block_layouts[3].clone(), 0,
            Buffer::new_slice(
                memory_allocator, 
                BufferCreateInfo::usage(BufferUsage::STORAGE_BUFFER), 
                AllocationCreateInfo::usage(MemoryUsage::DeviceOnly),
                1
            ).unwrap()
        );

        let attachments = UploadDescriptorSetArray::new(
            descriptor_set_allocator, 
            final_layouts[0].clone(), 
            0, 
            attachment_images,
        );

        let view_storage_buffer = super::util::make_device_only_buffer_sized(
            memory_allocator, cbb, 
            BufferUsage::STORAGE_BUFFER | BufferUsage::UNIFORM_BUFFER, 
            View::default()
        );

        let view = UploadDescriptorSet::new(
            descriptor_set_allocator,
            block_layouts[1].clone(),
            0,
            view_storage_buffer
        );

        let face_lighting_storage_buffer = super::util::make_device_only_buffer_sized(
            memory_allocator, cbb, 
            BufferUsage::STORAGE_BUFFER | BufferUsage::UNIFORM_BUFFER, 
            face_lighting
        );

        let face_lighting = UploadDescriptorSet::new(
            descriptor_set_allocator,
            block_layouts[2].clone(),
            0,
            face_lighting_storage_buffer
        );

        Self {
            atlas,
            atlas_map,
            block_quads,
            attachments,
            view,
            face_lighting
        }
    }

    pub fn bind_block(
        &self, 
        cbb: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        block_pipeline: Arc<GraphicsPipeline>,
    ) {
        cbb.bind_descriptor_sets(
            PipelineBindPoint::Graphics, 
            block_pipeline.layout().clone(), 
            0, 
            vec![
                self.atlas.set.clone(),
                self.view.set.clone(),
                self.face_lighting.set.clone(),
                self.block_quads.set.clone(),
                self.atlas_map.set.clone(),
            ]
        );
    }

    pub fn bind_deco(
        &self, 
        cbb: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        block_pipeline: Arc<GraphicsPipeline>,
    ) {
        cbb.bind_descriptor_sets(
            PipelineBindPoint::Graphics, 
            block_pipeline.layout().clone(), 
            0, 
            vec![
                self.atlas.set.clone(),
                self.view.set.clone(),
                self.face_lighting.set.clone(),
            ]
        );
    }

    pub fn bind_final(
        &self, 
        cbb: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        block_pipeline: Arc<GraphicsPipeline>,
    ) {
        cbb.bind_descriptor_sets(
            PipelineBindPoint::Graphics, 
            block_pipeline.layout().clone(), 
            0, 
            self.attachments.set.clone()
        );
    }
}