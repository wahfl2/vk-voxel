use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use ultraviolet::{IVec2, IVec3, UVec3};
use vulkano::{buffer::{BufferUsage, Subbuffer, Buffer, BufferCreateInfo}, device::Device, memory::allocator::{StandardMemoryAllocator, AllocationCreateInfo, MemoryUsage}};

use crate::{render::{brick::{brickmap::{Brickmap, BrickmapPointer}, brickgrid::{Brickgrid, BrickgridBuffer, BrickgridBufferTask, BRICKGRID_SIZE}}, util::CreateInfoConvenience}, world::{chunk::{Chunk, CHUNK_HEIGHT}, section::Section}, util::util::{InsertVec2, VecModPos}};

use super::allocator::HeapBuffer;

pub struct ChunkVertexBuffer {
    pub brickmap_buffer: HeapBuffer<Brickmap>,
    pub brickgrid_buffer: BrickgridBuffer,
}

const BM_BUFFER_USAGE: BufferUsage = BufferUsage::STORAGE_BUFFER;
const BG_BUFFER_USAGE: BufferUsage = BufferUsage::STORAGE_BUFFER.union(BufferUsage::TRANSFER_DST);

impl ChunkVertexBuffer {
    pub fn new(allocator: &StandardMemoryAllocator) -> Self {
        Self {
            brickmap_buffer: HeapBuffer::new(
                allocator, BM_BUFFER_USAGE, 
                1_000_000, 
                0
            ),

            brickgrid_buffer: BrickgridBuffer::new(allocator),
        }
    }

    pub fn update(&mut self) -> (bool, bool) {
        let bg = self.brickgrid_buffer.update();
        if bg {
            println!("Brickgrid swapped");
        }
        let bm = self.brickmap_buffer.update();
        if bm {
            println!("Brickmap swapped");
        }

        (bg, bm)
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

        let brickmap = section.brickmap;
        let ptr = if brickmap.is_empty() {
            BrickmapPointer::Empty
        } else {
            let allocation = self.brickmap_buffer.insert(section_pos, &[brickmap]);
            BrickmapPointer::Brickmap(allocation.front)
        };

        let raw_ptr = ptr.to_raw();
        let m_pos = section_pos.mod_pos(BRICKGRID_SIZE.into());

        // println!("Wrote {:#034b} to ({}, {}, {})", raw_ptr.pointer, m_pos.x, m_pos.y, m_pos.z);

        self.brickgrid_buffer.write(BrickgridBufferTask::One { pos: m_pos, section: raw_ptr });
    }

    pub fn remove_section(&mut self, section_pos: IVec3) {
        self.brickmap_buffer.remove(section_pos);
    }

    pub fn has_section(&self, section_pos: IVec3) -> bool {
        self.brickmap_buffer.allocations.get(&section_pos).is_some()
    }
}