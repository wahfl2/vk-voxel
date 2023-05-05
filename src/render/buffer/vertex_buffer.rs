use ultraviolet::{IVec2, IVec3};
use vulkano::{buffer::BufferUsage, memory::allocator::StandardMemoryAllocator};

use crate::{render::brick::{brickmap::{Brickmap, BrickmapPointer}, brickgrid::{BrickgridBuffer, BrickgridBufferTask, BRICKGRID_SIZE}}, world::{chunk::{Chunk, CHUNK_HEIGHT}, section::Section, block_data::{BlockTexture, StaticBlockData, ModelType}}, util::util::{InsertVec2, VecModPos}};

use super::allocator::HeapBuffer;

pub struct ChunkVertexBuffer {
    pub brickmap_buffer: HeapBuffer<Brickmap>,
    pub texture_pointer_buffer: HeapBuffer<u32>,
    pub brickgrid_buffer: BrickgridBuffer,
}

const BM_BUFFER_USAGE: BufferUsage = BufferUsage::STORAGE_BUFFER;

impl ChunkVertexBuffer {
    pub fn new(allocator: &StandardMemoryAllocator) -> Self {
        Self {
            brickmap_buffer: HeapBuffer::new(
                allocator, BM_BUFFER_USAGE, 
                1_000_000, 
                0
            ),

            texture_pointer_buffer: HeapBuffer::new(
                allocator, BM_BUFFER_USAGE, 
                128_000_000, 
                0
            ),

            brickgrid_buffer: BrickgridBuffer::new(allocator),
        }
    }

    pub fn update(&mut self) -> (bool, bool, bool) {
        let bg = self.brickgrid_buffer.update();
        let tp = self.texture_pointer_buffer.update();
        let bm = self.brickmap_buffer.update();

        (bg, tp, bm)
    }

    pub fn insert_chunk(&mut self, chunk: &Chunk, block_data: &StaticBlockData) {
        for (i, section) in chunk.sections.iter().enumerate() {
            let section_pos = chunk.pos.insert_y(i as i32);
            self.insert_section(section_pos, section, block_data);
        }
    }

    pub fn remove_chunk(&mut self, chunk_pos: IVec2) {
        for i in 0..CHUNK_HEIGHT as i32 {
            let section_pos = chunk_pos.insert_y(i);
            if self.brickmap_buffer.allocations.contains_key(&section_pos) {
                self.remove_section(section_pos);
            }
        }
    }

    pub fn insert_section(
        &mut self, 
        section_pos: IVec3, 
        section: &Section,
        block_data: &StaticBlockData,
    ) {
        if self.has_section(section_pos) {
            self.remove_section(section_pos);
        }

        let mut brickmap = section.brickmap;
        let ptr = if brickmap.is_empty() {
            BrickmapPointer::Empty
        } else {
            let block_textures = section.blocks.iter().filter_map(|b| {
                match &block_data.get(b).model {
                    ModelType::FullBlock(_) => Some(b.inner()),
                    _ => None,
                }
            }).collect::<Vec<_>>();
            brickmap.textures_offset = self.texture_pointer_buffer.insert(section_pos, &block_textures).front;

            let allocation = self.brickmap_buffer.insert(section_pos, &[brickmap]);
            BrickmapPointer::Brickmap(allocation.front)
        };

        let raw_ptr = ptr.to_raw();
        let m_pos = section_pos.mod_pos(BRICKGRID_SIZE.into());

        self.brickgrid_buffer.write(BrickgridBufferTask::One { pos: m_pos, section: raw_ptr });
    }

    pub fn remove_section(&mut self, section_pos: IVec3) {
        self.brickmap_buffer.remove(section_pos);
        self.texture_pointer_buffer.remove(section_pos);

        let raw_ptr = BrickmapPointer::Empty.to_raw();
        let m_pos = section_pos.mod_pos(BRICKGRID_SIZE.into());
        self.brickgrid_buffer.write(BrickgridBufferTask::One { pos: m_pos, section: raw_ptr });
    }

    pub fn has_section(&self, section_pos: IVec3) -> bool {
        self.brickmap_buffer.allocations.get(&section_pos).is_some()
    }
}