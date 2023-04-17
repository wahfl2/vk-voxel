use std::sync::Arc;

use ultraviolet::{IVec2, IVec3};
use vulkano::{buffer::BufferUsage, device::Device};

use crate::{render::{vertex::VertexRaw, mesh::{quad::BlockQuad, chunk_render::ChunkRender}, texture::TextureAtlas}, world::{block_data::StaticBlockData, chunk::{Chunk, CHUNK_HEIGHT}, section::Section}, util::util::InsertVec2};

use super::allocator::HeapBuffer;

pub struct ChunkVertexBuffer {
    pub block_quad_buffer: HeapBuffer<BlockQuad>,
    pub deco_buffer: HeapBuffer<VertexRaw>,
}

const BQ_BUFFER_USAGE: BufferUsage = BufferUsage::STORAGE_BUFFER.union(BufferUsage::STORAGE_TEXEL_BUFFER);
const DECO_BUFFER_USAGE: BufferUsage = BufferUsage::VERTEX_BUFFER;

impl ChunkVertexBuffer {
    pub fn new(device: Arc<Device>) -> Self {
        Self {
            block_quad_buffer: HeapBuffer::new(device.clone(), BQ_BUFFER_USAGE, 6),
            deco_buffer: HeapBuffer::new(device, DECO_BUFFER_USAGE, 1),
        }
    }

    pub fn update(&mut self) -> (bool, bool) {
        let ret = (
            self.block_quad_buffer.update(),
            self.deco_buffer.update(),
        );

        ret
    }

    pub fn insert_chunk(&mut self, chunk: &Chunk) {
        for (i, section) in chunk.sections.iter().enumerate() {
            let section_pos = chunk.pos.insert_y(i as i32);
            self.insert_section(section_pos, section);
        }
    }

    pub fn remove_chunk(&mut self, chunk_pos: IVec2) {
        for i in 0..CHUNK_HEIGHT as i32 {
            let section_pos = chunk_pos.insert_y(i);
            self.remove_section(section_pos);
        }
    }

    pub fn insert_section(
        &mut self, 
        section_pos: IVec3, 
        section: &Section,
    ) {
        if self.has_section(section_pos) {
            self.remove_section(section_pos);
        }

        let render_section = &section.render;

        if render_section.deco_vertices.len() % 3 != 0 {
            panic!("Number of vertices in the decorations were not a multiple of 3")
        }

        self.block_quad_buffer.insert(section_pos, &render_section.block_quads);
        self.deco_buffer.insert(section_pos, &render_section.deco_vertices);
    }

    pub fn remove_section(&mut self, section_pos: IVec3) {
        self.block_quad_buffer.remove(section_pos);
        self.deco_buffer.remove(section_pos);
    }

    pub fn has_section(&self, section_pos: IVec3) -> bool {
        self.block_quad_buffer.allocations.get(&section_pos).is_some()
    }
}