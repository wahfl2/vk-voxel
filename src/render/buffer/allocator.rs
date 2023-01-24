use std::{sync::Arc, collections::hash_map::Iter, time::Instant};

use rustc_data_structures::stable_map::FxHashMap;
use smallvec::{smallvec, SmallVec};
use ultraviolet::IVec2;
use vulkano::{buffer::{BufferUsage, CpuAccessibleBuffer}, device::Device, memory::allocator::{FastMemoryAllocator, StandardMemoryAllocator}, command_buffer::{DrawIndirectCommand, AutoCommandBufferBuilder, PrimaryAutoCommandBuffer, CopyBufferInfo, BufferCopy, CopyBufferInfoTyped}, NonExhaustive};

use crate::{render::{vertex::VertexRaw, mesh::renderable::Renderable, texture::TextureAtlas}, world::block_data::StaticBlockData};

use super::buffer_queue::{BufferQueue, BufferQueueTask};

pub struct VertexChunkBuffer {
    inner_vertex: Arc<CpuAccessibleBuffer<[VertexRaw]>>,
    queue_buf: Arc<CpuAccessibleBuffer<[VertexRaw]>>,
    inner_vertex_size: u64,
    allocator: Arc<StandardMemoryAllocator>,
    chunk_allocator: ChunkBufferAllocator,
    allocations: FxHashMap<(i32, i32), ChunkBufferAllocation>,
    pub queue: BufferQueue,
}

impl VertexChunkBuffer {
    const INITIAL_SIZE: u64 = 50_000;

    pub fn new(device: Arc<Device>) -> Self {
        let allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let (inner_vertex, queue_buf) = 
            Self::create_buffers(allocator.clone(), Self::INITIAL_SIZE);

        VertexChunkBuffer {
            inner_vertex,
            queue_buf,
            inner_vertex_size: Self::INITIAL_SIZE,
            allocator,
            chunk_allocator: ChunkBufferAllocator::new(),
            allocations: FxHashMap::default(),
            queue: BufferQueue::new()
        }
    }

    pub fn push_chunk_vertices(
        &mut self, 
        chunk_pos: IVec2, 
        chunk: impl Renderable, 
        atlas: &TextureAtlas, 
        block_data: &StaticBlockData
    ) {
        let verts: Vec<VertexRaw> = chunk.get_vertices(atlas, block_data);
        let size = verts.len() as u32;
        let allocation = self.chunk_allocator.allocate(size);
        self.allocations.insert(chunk_pos.into(), allocation);
        while (allocation.back as u64) >= self.inner_vertex_size {
            self.grow_inner_vertex();
        }

        let mut write = self.queue_buf.write().expect("Epic queue write fail.");
        write[(allocation.front as usize)..(allocation.back as usize)]
            .copy_from_slice(&verts);
        
        self.queue.push_data(allocation.front, verts);
    }

    pub fn remove_chunk(&mut self, chunk_pos: IVec2) {
        if let Some(allocation) = self.allocations.get(&chunk_pos.into()) {
            self.chunk_allocator.deallocate(allocation);
            // No need to push to queue, corresponding memory will not be read
        } else {
            panic!("Tried to remove chunk that was not allocated. Chunk pos: {:?}", chunk_pos);
        }
    }

    pub fn get_buffer(&self) -> Arc<CpuAccessibleBuffer<[VertexRaw]>> {
        self.inner_vertex.clone()
    }

    pub fn get_indirect_commands(&self) -> Vec<DrawIndirectCommand> {
        let mut ret = Vec::with_capacity(self.allocations.len());
        for alloc in self.allocations.values() {
            ret.push(DrawIndirectCommand {
                vertex_count: alloc.back - alloc.front,
                instance_count: 1,
                first_vertex: alloc.front,
                first_instance: 0,
            });
        }
        ret
    }

    /// Should only be executed in a context where you know it's in sync.
    pub fn execute_queue(&mut self, command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) {
        let mut copy_regions: SmallVec<[_; 1]> = smallvec![];
        for task in self.queue.flush() {
            match task {
                BufferQueueTask::Write(write) => {
                    let offset = write.start_idx as u64;
                    let size = write.data.len() as u64;

                    copy_regions.push(BufferCopy {
                        src_offset: offset,
                        dst_offset: offset,
                        size,
                        ..Default::default()
                    });
                },
                BufferQueueTask::Transfer(transfer) => {
                    let copy = 
                        CopyBufferInfoTyped::buffers(transfer.src_buf, transfer.dst_buf);

                    command_buffer_builder.copy_buffer(copy).unwrap();
                },
            }
        }

        if !copy_regions.is_empty() {
            let mut copy_buffer_info = CopyBufferInfoTyped::buffers(
                self.queue_buf.clone(), 
                self.inner_vertex.clone(),
            );
            copy_buffer_info.regions = copy_regions;
            command_buffer_builder.copy_buffer(copy_buffer_info).expect("EPIC FAIL");
        }
    }

    // TODO: move growable buffer to its own struct maybe
    fn create_buffers(
        allocator: Arc<StandardMemoryAllocator>, 
        size: u64
    ) -> (Arc<CpuAccessibleBuffer<[VertexRaw]>>, Arc<CpuAccessibleBuffer<[VertexRaw]>>) {
        (
            unsafe {
                CpuAccessibleBuffer::uninitialized_array(
                    &allocator, 
                    size, 
                    BufferUsage {
                        vertex_buffer: true,
                        transfer_dst: true,
                        ..Default::default()
                    }, 
                    false
                ).unwrap()
            },
            
            unsafe {
                CpuAccessibleBuffer::uninitialized_array(
                    &allocator, 
                    size, 
                    BufferUsage {
                        transfer_src: true,
                        ..Default::default()
                    }, 
                    false
                ).unwrap()
            },
        )
    }

    /// 
    fn grow_inner_vertex(&mut self) {
        let old_v_buffer = self.inner_vertex.clone();
        let old_q_buffer = self.queue_buf.clone();

        println!("Creating new buffers...");
        let buffers = Self::create_buffers(self.allocator.clone(), self.inner_vertex_size * 2);
        self.inner_vertex = buffers.0;
        self.queue_buf = buffers.1;

        print!("Writing old data... ");
        let prev = Instant::now();
        let write = &mut self.inner_vertex.write().unwrap();
        write[0..self.inner_vertex_size as usize].copy_from_slice(&old_v_buffer.read().unwrap());
        let write = &mut self.queue_buf.write().unwrap();
        write[0..self.inner_vertex_size as usize].copy_from_slice(&old_q_buffer.read().unwrap());
        println!("{}ms", (Instant::now() - prev).as_millis());

        self.inner_vertex_size *= 2;
        println!("New size: {}", self.inner_vertex_size);
    }
}

/// Describes an allocation in the chunk vertex buffer. Has no real spot in the memory backing it.
#[derive(Copy, Clone, Debug)]
pub struct ChunkBufferAllocation {
    pub front: u32,
    pub back: u32,
}

impl ChunkBufferAllocation {
    fn new(front: u32, back: u32) -> Self {
        Self { front, back }
    }

    fn new_size(front: u32, size: u32) -> Self {
        Self::new(front, front + size)
    }
}

struct ChunkBufferAllocator {
    /// Represents all free sections
    dual_map: DualHashMap<u32>,
    top: u32,
}

impl ChunkBufferAllocator {
    fn new() -> Self {
        Self {
            dual_map: DualHashMap::new(),
            top: 0,
        }
    }

    fn allocate(&mut self, size: u32) -> ChunkBufferAllocation {
        if let Some((front, back, free_size)) = Self::find_free(&self.dual_map, size) {
            self.dual_map.remove_front(&front);
            if free_size != size { self.dual_map.insert(front + size, back); }
            ChunkBufferAllocation::new_size(front, size)
        } else {
            let old_top = self.top;
            self.top += size;
            ChunkBufferAllocation::new_size(old_top, size)
        }
    }

    /// Deallocates a section and marks it as free.<br>
    /// <b>DOES NOT CHECK IF THIS ALLOCATION IS VALID</b>. *Use an allocation generated by* `.allocate()`
    fn deallocate(&mut self, alloc: &ChunkBufferAllocation) {
        let mut joined_alloc = alloc.clone();

        if let Some(back) = self.dual_map.remove_front(&alloc.back) {
            joined_alloc.back = back.to_owned();
        }

        if let Some(front) = self.dual_map.remove_back(&alloc.front) {
            joined_alloc.front = front.to_owned();
        }

        self.dual_map.insert(joined_alloc.front, joined_alloc.back);
    }

    fn find_free(map: &DualHashMap<u32>, needed_size: u32) -> Option<(u32, u32, u32)> {
        for (front, back) in map.iter() {
            let free_size = back - front;
            if needed_size <= free_size {
                return Some((front.to_owned(), back.to_owned(), free_size))
            }
        }
        None
    }
}

struct DualHashMap<T> {
    front_back: FxHashMap<T, T>,
    back_front: FxHashMap<T, T>,
}

impl<T> DualHashMap<T> 
where
    T: std::hash::Hash + std::cmp::Eq + Clone
{
    fn new() -> Self {
        Self {
            front_back: FxHashMap::default(),
            back_front: FxHashMap::default(),
        }
    }

    fn insert(&mut self, front: T, back: T) {
        self.front_back.insert(front.clone(), back.clone());
        self.back_front.insert(back, front);
    }

    fn _get_back(&self, front: &T) -> Option<&T> {
        self.front_back.get(front)
    }

    fn _get_front(&self, back: &T) -> Option<&T> {
        self.back_front.get(back)
    }

    /// Removes the pair with the specified front, returning the back if it existed.
    fn remove_front(&mut self, front: &T) -> Option<T> {
        match self.front_back.remove(front) {
            Some(back) => {
                self.back_front.remove(&back);
                return Some(back)
            },
            None => return None
        }        
    }

    /// Removes the pair with the specified back, returning the front if it existed.
    fn remove_back(&mut self, back: &T) -> Option<T> {
        match self.back_front.remove(back) {
            Some(front) => {
                self.front_back.remove(&front);
                return Some(front)
            },
            None => return None
        }   
    }

    fn iter(&self) -> Iter<'_, T, T> {
        self.front_back.iter()
    }
}