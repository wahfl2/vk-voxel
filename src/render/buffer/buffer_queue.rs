use vulkano::buffer::Subbuffer;

use crate::render::vertex::VertexRaw;

pub struct BufferQueue {
    pub tasks: Vec<BufferQueueTask>,
}

impl BufferQueue {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    pub fn push_data(&mut self, start_index: u32, data: Vec<VertexRaw>) {
        let task = WriteTask::new(start_index, data);
        self.tasks.push(BufferQueueTask::Write(task));
    }

    pub fn flush(&mut self) -> Vec<BufferQueueTask> {
        let ret = self.tasks.clone();
        self.tasks.clear();
        ret
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

#[derive(Clone)]
pub enum BufferQueueTask {
    Write(WriteTask),
    Transfer(TransferTask),
}

#[derive(Clone)]
pub struct WriteTask {
    pub start_idx: u32,
    pub data: Vec<VertexRaw>,
}

impl WriteTask {
    pub fn new(start_idx: u32, data: Vec<VertexRaw>) -> Self {
        Self {
            start_idx,
            data,
        }
    }
}

#[derive(Clone)]
pub struct TransferTask {
    pub src_buf: Subbuffer<[VertexRaw]>,
    pub dst_buf: Subbuffer<[VertexRaw]>,
}

impl TransferTask {
    pub fn new(src_buf: Subbuffer<[VertexRaw]>, dst_buf: Subbuffer<[VertexRaw]>) -> Self {
        Self {
            src_buf,
            dst_buf,
        }
    }
}