use std::sync::Arc;

use vulkano::buffer::CpuAccessibleBuffer;

use crate::render::vertex::VertexRaw;

pub struct SwappingBuffer {
    current: u8,
    pub buffer_1: Arc<CpuAccessibleBuffer<[VertexRaw]>>,
    pub buffer_2: Arc<CpuAccessibleBuffer<[VertexRaw]>>,
    pub dirty: bool,
    pub data: Vec<Option<VertexRaw>>,
}

impl SwappingBuffer {
    pub fn swap(&mut self) {
        self.current = 3 - self.current;
    }

    pub fn get_buffer(&self) -> Arc<CpuAccessibleBuffer<[VertexRaw]>> {
        match self.current {
            1 => self.buffer_1.clone(),
            2 => self.buffer_2.clone(),
            n => panic!("Invalid buffer: {}", n),
        }
    }

    
}