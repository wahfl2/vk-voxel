use std::sync::Arc;

use vulkano::{descriptor_set::{layout::DescriptorSetLayout, PersistentDescriptorSet, allocator::StandardDescriptorSetAllocator, WriteDescriptorSet}, memory::allocator::StandardMemoryAllocator, command_buffer::{AutoCommandBufferBuilder, allocator::CommandBufferAllocator}, buffer::{BufferContents, DeviceLocalBuffer, BufferUsage}, pipeline::{PipelineLayout, PipelineBindPoint}};

pub struct UniformBuffer {
    pub set: u32,
    pub layout: Arc<DescriptorSetLayout>,
    pub descriptor_set: Option<Arc<PersistentDescriptorSet>>,
}

impl UniformBuffer {
    pub fn new(set: u32, layout: Arc<DescriptorSetLayout>) -> Self {
        Self { 
            set,
            layout,
            descriptor_set: None,
        }
    }

    pub fn update_data<D, L, A>(
        &mut self,
        memory_allocator: &StandardMemoryAllocator,
        desc_set_allocator: &StandardDescriptorSetAllocator,
        command_buffer_builder: &mut AutoCommandBufferBuilder<L, A>,
        data: D, 
    ) where 
        D: BufferContents,
        A: CommandBufferAllocator,
    {
        self.descriptor_set = Some(PersistentDescriptorSet::new(
            desc_set_allocator,
            self.layout.clone(),
            [WriteDescriptorSet::buffer(
                0,
                DeviceLocalBuffer::from_data(
                    memory_allocator, 
                    data, 
                    BufferUsage {
                        storage_buffer: true,
                        uniform_buffer: true,
                        ..Default::default()
                    }, 
                    command_buffer_builder
                ).unwrap(), 
            )],
        ).unwrap());
    }
}

pub trait BindUniformBuffer {
    fn bind_uniform_buffer(&mut self, pipeline_layout: &Arc<PipelineLayout>, uniform_buffer: &UniformBuffer) -> &mut Self;
}

impl<L, A> BindUniformBuffer for AutoCommandBufferBuilder<L, A>
where
    A: CommandBufferAllocator,
{
    fn bind_uniform_buffer(
        &mut self, 
        pipeline_layout: &Arc<PipelineLayout>,
        uniform_buffer: &UniformBuffer
    ) -> &mut Self {
        if uniform_buffer.descriptor_set.is_none() {
            panic!("Tried to bind an empty uniform buffer, set = {:?}", uniform_buffer.set);
        }

        self.bind_descriptor_sets(
            PipelineBindPoint::Graphics, 
            pipeline_layout.clone(),
            uniform_buffer.set,
            uniform_buffer.descriptor_set.clone().unwrap(),
        )
    }
}