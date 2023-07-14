use std::sync::Arc;

use bytemuck::Zeroable;
use vulkano::{image::{view::ImageView, ImmutableImage}, sampler::Sampler, buffer::{Subbuffer, Buffer, BufferCreateInfo, BufferUsage}, descriptor_set::allocator::StandardDescriptorSetAllocator, pipeline::{PipelineBindPoint, PipelineLayout}, memory::allocator::{StandardMemoryAllocator, AllocationCreateInfo, MemoryUsage}, command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer}};

use crate::world::block_data::{StaticBlockData, BlockTexture, ModelType};

use super::{buffer::upload::UploadDescriptorSet, renderer::{Pipelines, View}, texture::TextureAtlas, util::{CreateInfoConvenience, ProgramInfo}, brick::{brickmap::Brickmap, brickgrid::Brickgrid}, mesh::quad::TexelTexture};

pub type ImageViewSampler = (Arc<ImageView<ImmutableImage>>, Arc<Sampler>);

pub struct DescriptorSets {
    pub atlas: UploadDescriptorSet<ImageViewSampler>,
    pub atlas_map: UploadDescriptorSet<Subbuffer<[TexelTexture]>>,
    pub block_texture_map: UploadDescriptorSet<Subbuffer<[BlockTexture]>>,

    pub view: UploadDescriptorSet<Subbuffer<View>>,
    pub program_info: UploadDescriptorSet<Subbuffer<ProgramInfo>>,

    pub brickmap: UploadDescriptorSet<Subbuffer<[Brickmap]>>,
    pub brickgrid: UploadDescriptorSet<Subbuffer<Brickgrid>>,
    pub texture_buffer: UploadDescriptorSet<Subbuffer<[u32]>>,
}

impl DescriptorSets {
    pub fn new(
        memory_allocator: &StandardMemoryAllocator,
        descriptor_set_allocator: &StandardDescriptorSetAllocator,
        cbb: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,

        pipelines: &Pipelines,
        texture_atlas: &TextureAtlas,
        block_data: &StaticBlockData,
        sampler: Arc<Sampler>,
    ) -> Self {
        let raytracing_layouts = pipelines.layout.set_layouts();

        let atlas = UploadDescriptorSet::new(
            descriptor_set_allocator,
            raytracing_layouts[0].clone(), 0,
            (texture_atlas.get_texture(memory_allocator, cbb), sampler)
        );
        
        let atlas_map_storage_buffer = super::util::make_device_only_buffer_slice(
            memory_allocator, cbb, 
            BufferUsage::STORAGE_BUFFER, 
            texture_atlas.uvs.clone()
        );
        
        let atlas_map = UploadDescriptorSet::new(
            descriptor_set_allocator,
            raytracing_layouts[1].clone(), 0,
            atlas_map_storage_buffer
        );

        let block_texture_storage_buffer = super::util::make_device_only_buffer_slice(
            memory_allocator, cbb, 
            BufferUsage::STORAGE_BUFFER, 
            block_data.block_data().iter().map(|b| {
                match &b.model {
                    ModelType::FullBlock(m) => BlockTexture::from(m.clone()),
                    _ => BlockTexture::zeroed(),
                }
            })
        );

        let block_texture_map = UploadDescriptorSet::new(
            descriptor_set_allocator,
            raytracing_layouts[7].clone(), 0,
            block_texture_storage_buffer
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

        let program_info_storage_buffer = super::util::make_device_only_buffer_sized(
            memory_allocator, cbb, 
            BufferUsage::STORAGE_BUFFER | BufferUsage::UNIFORM_BUFFER, 
            ProgramInfo::new(),
        );

        let program_info = UploadDescriptorSet::new(
            descriptor_set_allocator,
            raytracing_layouts[3].clone(), 0,
            program_info_storage_buffer
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

        let texture_buffer = UploadDescriptorSet::new(
            descriptor_set_allocator,
            raytracing_layouts[6].clone(), 0,
            Buffer::new_slice(
                memory_allocator,
                BufferCreateInfo::usage(BufferUsage::STORAGE_BUFFER), 
                AllocationCreateInfo::usage(MemoryUsage::Upload),
                1,
            ).unwrap()
        );

        Self {
            atlas,
            atlas_map,
            block_texture_map,
            program_info,
            view,
            brickmap,
            brickgrid,
            texture_buffer,
        }
    }

    pub fn bind_raytracing(
        &self, 
        cbb: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        pipeline_layout: Arc<PipelineLayout>,
    ) {
        cbb.bind_descriptor_sets(
            PipelineBindPoint::Graphics, 
            pipeline_layout.clone(), 
            0, 
            vec![
                self.atlas.set.clone(),
                self.atlas_map.set.clone(),
                self.view.set.clone(),
                self.program_info.set.clone(),
                self.brickmap.set.clone(),
                self.brickgrid.set.clone(),
                self.texture_buffer.set.clone(),
                self.block_texture_map.set.clone(),
            ]
        );
    }
}