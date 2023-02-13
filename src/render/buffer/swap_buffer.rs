use std::sync::Arc;

use vulkano::{buffer::{CpuAccessibleBuffer, cpu_access::WriteLock, BufferUsage, BufferContents}, memory::allocator::StandardMemoryAllocator};

pub struct SwappingBuffer<T> 
    where [T]: BufferContents 
{
    current: u8,
    queue: Vec<SwapBufferQueueTask<T>>,
    after_free: Vec<SwapBufferQueueTask<T>>,
    pub buffer_1: Arc<CpuAccessibleBuffer<[T]>>,
    pub buffer_2: Arc<CpuAccessibleBuffer<[T]>>,
    pub dirty: SwapDirtyPhase,
}

#[derive(Clone, Debug)]
pub struct SwapBufferQueueTask<T> {
    pub start_idx: usize,
    pub data: Vec<T>,
}

impl<T> SwapBufferQueueTask<T> {
    pub fn new(start_idx: usize, data: Vec<T>) -> Self {
        Self { start_idx, data }
    }
}

pub enum SwapDirtyPhase {
    /// Swap ASAP
    Dirty,
    /// Swapped, no new data yet; Write after free but don't swap again.
    SwappedWaiting,
    /// Swapped, new data in queue; Write after free and swap again immediately.
    SwappedImmediate,
    /// All good!
    Clean,
}

impl<T> SwappingBuffer<T>
where 
    T: Clone + Copy,
    [T]: BufferContents
{
    pub fn new(size: usize, usage: BufferUsage, allocator: &StandardMemoryAllocator) -> Self {
        Self {
            current: 1,
            queue: Vec::new(),
            after_free: Vec::new(),
            buffer_1: Self::create_buffer(size, usage, allocator),
            buffer_2: Self::create_buffer(size, usage, allocator),
            dirty: SwapDirtyPhase::Clean,
        }
    }

    fn create_buffer(size: usize, usage: BufferUsage, allocator: &StandardMemoryAllocator) -> Arc<CpuAccessibleBuffer<[T]>> {
        unsafe {
            CpuAccessibleBuffer::uninitialized_array(
                allocator, 
                size as u64, 
                usage, 
                false
            ).unwrap()
        }
    }

    fn swap(&mut self) {
        self.current = 3 - self.current;
    }

    pub fn get_current_buffer(&self) -> Arc<CpuAccessibleBuffer<[T]>> {
        match self.current {
            1 => self.buffer_1.clone(),
            2 => self.buffer_2.clone(),
            n => panic!("Invalid buffer: {}", n),
        }
    }

    fn get_free_buffer(&self) -> Arc<CpuAccessibleBuffer<[T]>> {
        match self.current {
            2 => self.buffer_1.clone(),
            1 => self.buffer_2.clone(),
            n => panic!("Invalid buffer: {}", n),
        }
    }

    pub fn write(&mut self, start_idx: usize, vertices: &[T]) {
        self.queue.push(SwapBufferQueueTask::new(start_idx, vertices.to_owned().to_vec()));
        match self.dirty {
            SwapDirtyPhase::SwappedWaiting => self.dirty = SwapDirtyPhase::SwappedImmediate,
            SwapDirtyPhase::Clean => self.dirty = SwapDirtyPhase::Dirty,
            _ => ()
        }
    }

    pub fn update(&mut self) -> bool {
        match self.dirty {
            SwapDirtyPhase::Dirty => {
                let free = self.get_free_buffer();
                if let Ok(mut write) = free.write() {
                    // (there may be a better way to transfer the data instead of copying it)
                    self.after_free = self.queue.clone();
                    Self::write_queue_buffer(&mut self.queue, &mut write);

                    self.swap();
                    self.dirty = SwapDirtyPhase::SwappedWaiting;
                    return true
                };
            },
            SwapDirtyPhase::SwappedWaiting => {
                // Try to gain write access to the recently freed buffer
                // May not be instant due to frames in flight
                let free = self.get_free_buffer();
                if let Ok(mut write) = free.write() {
                    // Write the new data that was written to the in-use buffer
                    Self::write_queue_buffer(&mut self.after_free, &mut write);
                    self.dirty = SwapDirtyPhase::Clean;
                    return true
                };
            },
            SwapDirtyPhase::SwappedImmediate => {
                // Try to gain write access to the recently freed buffer
                // May not be instant due to frames in flight
                let free = self.get_free_buffer();
                if let Ok(mut write) = free.write() {
                    // Write the new data that was written to the in-use buffer
                    Self::write_queue_buffer(&mut self.after_free, &mut write);
                    self.after_free = self.queue.clone();

                    // Write the new data to the now-free buffer and swap immediately
                    Self::write_queue_buffer(&mut self.queue, &mut write);
                    self.swap();
                    self.dirty = SwapDirtyPhase::SwappedWaiting;
                    return true
                };
            },
            SwapDirtyPhase::Clean => (),
        }
        false
    }

    /// Writes a queue to a write lock and clears the queue.
    fn write_queue_buffer(queue: &mut Vec<SwapBufferQueueTask<T>>, write_lock: &mut WriteLock<[T]>) {
        queue.iter().for_each(|task| {
            let end = task.start_idx + task.data.len();
            write_lock[task.start_idx..end].copy_from_slice(&task.data);
        });
        queue.clear();
    }
}