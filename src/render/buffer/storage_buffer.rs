use std::sync::Arc;

use vulkano::{command_buffer::{allocator::CommandBufferAllocator, AutoCommandBufferBuilder}, descriptor_set::{PersistentDescriptorSet, allocator::StandardDescriptorSetAllocator, layout::DescriptorSetLayout, WriteDescriptorSet}, memory::allocator::StandardMemoryAllocator, buffer::{DeviceLocalBuffer, BufferUsage, BufferContents}, pipeline::{PipelineBindPoint, PipelineLayout}};

pub struct StorageBuffer {
    pub layout: Arc<DescriptorSetLayout>,
    pub descriptor_set: Option<Arc<PersistentDescriptorSet>>,
}

impl StorageBuffer {
    pub fn new(layout: Arc<DescriptorSetLayout>) -> Self {
        Self { 
            layout,
            descriptor_set: None,
        }
    }

    pub fn update_data<D, T, L, A>(
        &mut self,
        memory_allocator: StandardMemoryAllocator,
        desc_set_allocator: StandardDescriptorSetAllocator,
        command_buffer_builder: &mut AutoCommandBufferBuilder<L, A>,
        data: D, 
    ) where 
        D: IntoIterator<Item = T>,
        [T]: BufferContents,
        D::IntoIter: ExactSizeIterator,
        A: CommandBufferAllocator,
    {
        self.descriptor_set = Some(PersistentDescriptorSet::new(
            &desc_set_allocator,
            self.layout.clone(),
            [WriteDescriptorSet::buffer(
                0,
                DeviceLocalBuffer::from_iter(
                    &memory_allocator, 
                    data, 
                    BufferUsage {
                        storage_buffer: true,
                        ..Default::default()
                    }, 
                    command_buffer_builder
                ).unwrap(), 
            )],
        ).unwrap());
    }
}

pub trait BindStorageBuffer {
    fn bind_storage_buffer(&mut self, pipeline_layout: Arc<PipelineLayout>, set_num: u32, storage_buffer: StorageBuffer) -> &mut Self;
}

impl<L, A> BindStorageBuffer for AutoCommandBufferBuilder<L, A>
where
    A: CommandBufferAllocator,
{
    fn bind_storage_buffer(&mut self, pipeline_layout: Arc<PipelineLayout>, set_num: u32, storage_buffer: StorageBuffer) -> &mut Self {
        self.bind_descriptor_sets(
            PipelineBindPoint::Graphics, 
            pipeline_layout, 
            set_num, 
            storage_buffer.descriptor_set.unwrap().clone(),
        )
    }
}