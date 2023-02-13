use std::sync::Arc;

use ultraviolet::IVec2;
use vulkano::{buffer::BufferUsage, device::Device, command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer, DrawIndirectCommand}, memory::allocator::StandardMemoryAllocator};

use crate::{render::{vertex::VertexRaw, mesh::{quad::BlockQuad, chunk_render::{RenderSection, ChunkRender}}, texture::TextureAtlas}, world::{block_data::StaticBlockData, chunk::Chunk}};

use super::allocator::HeapBuffer;

pub struct ChunkVertexBuffer {
    pub block_quad_buffer: HeapBuffer<BlockQuad>,
    pub deco_buffer: HeapBuffer<VertexRaw>,
}

const BQ_BUFFER_USAGE: BufferUsage = BufferUsage {
    storage_buffer: true,
    storage_texel_buffer: true,
    ..BufferUsage::empty()
};

const DECO_BUFFER_USAGE: BufferUsage = BufferUsage {
    vertex_buffer: true,
    ..BufferUsage::empty()
};

impl ChunkVertexBuffer {
    pub fn new(device: Arc<Device>) -> Self {
        Self {
            block_quad_buffer: HeapBuffer::new(device.clone(), BQ_BUFFER_USAGE),
            deco_buffer: HeapBuffer::new(device, DECO_BUFFER_USAGE),
        }
    }

    pub fn update(&mut self, allocator: &StandardMemoryAllocator, builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) -> (bool, bool) {
        (
            self.block_quad_buffer.update(allocator, builder),
            self.deco_buffer.update(allocator, builder),
        )
    }

    pub fn insert_chunk(
        &mut self, 
        chunk_pos: IVec2, 
        chunk: &Chunk, 
        atlas: &TextureAtlas, 
        block_data: &StaticBlockData
    ) {
        let render_chunk = chunk.get_render_section(atlas, block_data);

        if render_chunk.deco_vertices.len() % 3 != 0 {
            panic!("Number of vertices in the decorations were not a multiple of 3")
        }

        self.block_quad_buffer.insert(chunk_pos, &render_chunk.block_quads, atlas, block_data);
        self.deco_buffer.insert(chunk_pos, &render_chunk.deco_vertices, atlas, block_data);
    }

    pub fn remove_chunk(&mut self, chunk_pos: IVec2) {
        self.block_quad_buffer.remove(chunk_pos);
        self.deco_buffer.remove(chunk_pos);
    }

    pub fn reinsert_chunk(
        &mut self, 
        chunk_pos: IVec2, 
        chunk: &Chunk, 
        atlas: &TextureAtlas, 
        block_data: &StaticBlockData
    ) {
        self.remove_chunk(chunk_pos);
        self.insert_chunk(chunk_pos, chunk, atlas, block_data);
    }

    pub fn get_block_quad_indirect_commands(&self) -> Vec<DrawIndirectCommand> {
        self.block_quad_buffer.allocations.values().map(|alloc| {
            DrawIndirectCommand {
                vertex_count: (alloc.back - alloc.front) * 6,
                instance_count: 1,
                first_vertex: alloc.front * 6,
                first_instance: 0,
            }
        }).collect()
    }
}