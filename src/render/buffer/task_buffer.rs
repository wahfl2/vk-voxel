use std::collections::VecDeque;

use vulkano::{memory::allocator::StandardMemoryAllocator, buffer::{BufferUsage, BufferContents, subbuffer::BufferWriteGuard, Subbuffer}};

use super::swap_buffer::{SwapBuffer, SwapDirtyPhase};

pub struct TaskBuffer<T, U> {
    queue: VecDeque<T>,
    after_free: VecDeque<T>,
    write_fn: fn(&mut VecDeque<T>, &mut BufferWriteGuard<U>),
    pub inner: SwapBuffer<U>,
    pub dirty: SwapDirtyPhase,
}

impl<T, U> TaskBuffer<T, U>
where
    T: Clone,
    U: BufferContents + Clone
{
    pub fn new(allocator: &StandardMemoryAllocator, write_fn: fn(&mut VecDeque<T>, &mut BufferWriteGuard<U>)) -> Self {
        let inner: SwapBuffer<U> = SwapBuffer::new_sized(BufferUsage::STORAGE_BUFFER, allocator);

        Self {
            queue: VecDeque::new(),
            after_free: VecDeque::new(),
            write_fn,
            inner,
            dirty: SwapDirtyPhase::Clean,
        }
    }

    pub fn write(&mut self, task: T) {
        self.queue.push_back(task);
        match self.dirty {
            SwapDirtyPhase::SwappedWaiting => self.dirty = SwapDirtyPhase::SwappedImmediate,
            SwapDirtyPhase::Clean => self.dirty = SwapDirtyPhase::Dirty,
            _ => ()
        }
    }

    pub fn get_buffer(&self) -> Subbuffer<U> {
        self.inner.current_buffer()
    }

    pub fn update(&mut self) -> bool {
        match self.dirty {
            SwapDirtyPhase::Dirty => {
                let free = self.inner.free_buffer();
                if let Ok(mut write) = free.write() {
                    // (there may be a better way to transfer the data instead of copying it)
                    self.after_free = self.queue.clone();
                    self.write_fn.call_once((&mut self.queue, &mut write));

                    self.inner.swap();
                    self.dirty = SwapDirtyPhase::SwappedWaiting;
                    return true
                };
            },
            SwapDirtyPhase::SwappedWaiting => {
                // Try to gain write access to the recently freed buffer
                // May not be instant due to frames in flight
                let free = self.inner.free_buffer();
                if let Ok(mut write) = free.write() {
                    // Write the new data that was written to the in-use buffer
                    self.write_fn.call_once((&mut self.after_free, &mut write));
                    self.dirty = SwapDirtyPhase::Clean;
                    return true
                };
            },
            SwapDirtyPhase::SwappedImmediate => {
                // Try to gain write access to the recently freed buffer
                // May not be instant due to frames in flight
                let free = self.inner.free_buffer();
                if let Ok(mut write) = free.write() {
                    // Write the new data that was written to the in-use buffer
                    self.write_fn.call_once((&mut self.after_free, &mut write));
                    self.after_free = self.queue.clone();

                    // Write the new data to the now-free buffer and swap immediately
                    self.write_fn.call_once((&mut self.queue, &mut write));
                    self.inner.swap();
                    self.dirty = SwapDirtyPhase::SwappedWaiting;
                    return true
                };
            },
            SwapDirtyPhase::Clean => (),
        }
        false
    }
}