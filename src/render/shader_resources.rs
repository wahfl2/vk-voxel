use std::sync::Arc;

use vulkano::{pipeline::PipelineLayout, command_buffer::{AutoCommandBufferBuilder, allocator::CommandBufferAllocator}};

use super::buffer::{uniform_buffer::{UniformBuffer, BindUniformBuffer}, storage_buffer::{StorageBuffer, BindStorageBuffer}};

pub struct ShaderResources {
    pub view: UniformBuffer,
    pub face_lighting: UniformBuffer,
}

impl ShaderResources {
    pub fn new(view: UniformBuffer, face_lighting: UniformBuffer) -> Self {
        Self { 
            view,
            face_lighting,
        }
    }
}

pub enum ShaderResource {
    Uniform(UniformBuffer),
    Storage(StorageBuffer),
}

impl From<UniformBuffer> for ShaderResource {
    fn from(value: UniformBuffer) -> Self {
        Self::Uniform(value)
    }
}

impl From<StorageBuffer> for ShaderResource {
    fn from(value: StorageBuffer) -> Self {
        Self::Storage(value)
    }
}

pub trait BindResources {
    fn bind_resources(&mut self, pipeline_layout: &Arc<PipelineLayout>, resources: &ShaderResources) -> &mut Self;
}

impl<L, A> BindResources for AutoCommandBufferBuilder<L, A>
where
    A: CommandBufferAllocator,
{
    fn bind_resources(&mut self, pipeline_layout: &Arc<PipelineLayout>, resources: &ShaderResources) -> &mut Self {
        self.bind_uniform_buffer(pipeline_layout, &resources.view);
        self.bind_uniform_buffer(pipeline_layout, &resources.face_lighting)
    }
}