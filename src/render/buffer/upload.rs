use std::{sync::Arc, ops::Range};

use vulkano::{descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet, WriteDescriptorSetElements, layout::DescriptorSetLayout, allocator::{DescriptorSetAllocator, StandardDescriptorSetAlloc}}, buffer::{Buffer, Subbuffer}, image::{view::ImageView, ImmutableImage, ImageViewAbstract}, sampler::Sampler, DeviceSize, command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer, CopyBufferInfo}, pipeline::{PipelineBindPoint, PipelineLayout}};

pub struct UploadDescriptorSet<T, A = StandardDescriptorSetAlloc> 
where 
    T: DescriptorUploadable 
{
    pub set: Arc<PersistentDescriptorSet<A>>,
    pub layout: Arc<DescriptorSetLayout>,
    pub binding: u32,
    pub data: T,
}

impl<T, A> UploadDescriptorSet<T, A>
where
    T: DescriptorUploadable
{
    pub fn new<Al>(allocator: &Al, layout: Arc<DescriptorSetLayout>, binding: u32, data: T) -> Self
    where 
        Al: DescriptorSetAllocator<Alloc = A> + ?Sized,
    {
        Self {
            set: PersistentDescriptorSet::new(allocator, layout.clone(), data.write(binding)).unwrap(),
            layout,
            binding,
            data,
        }
    }

    pub fn replace<Al>(&mut self, allocator: &Al, new_data: T)
    where 
        Al: DescriptorSetAllocator<Alloc = A> + ?Sized,
    {
        self.set = PersistentDescriptorSet::new(
            allocator, 
            self.layout.clone(), 
            new_data.write(self.binding)
        ).unwrap();

        self.data = new_data;
    }
}

impl<U, A> UploadDescriptorSet<Subbuffer<U>, A>
{
    pub fn copy_data(&mut self, cbb: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, src_buffer: Subbuffer<U>) {
        cbb.copy_buffer(CopyBufferInfo::buffers(src_buffer, self.data.clone())).unwrap();
    }
}

pub trait DescriptorUploadable {
    fn write(&self, binding: u32) -> Vec<WriteDescriptorSet>;
}

impl<T> DescriptorUploadable for Subbuffer<T>
where 
    T: ?Sized
{
    fn write(&self, binding: u32) -> Vec<WriteDescriptorSet> {
        vec![WriteDescriptorSet::buffer(binding, self.clone())]
    }
}

impl<I> DescriptorUploadable for (Arc<I>, Arc<Sampler>)
where I: ImageViewAbstract + 'static
{
    fn write(&self, binding: u32) -> Vec<WriteDescriptorSet> {
        let (image_view, sampler) = self.clone();
        vec![WriteDescriptorSet::image_view_sampler(binding, image_view, sampler)]
    }
}

impl<I: ImageViewAbstract + 'static> DescriptorUploadable for Arc<I>
{
    fn write(&self, binding: u32) -> Vec<WriteDescriptorSet> {
        vec![WriteDescriptorSet::image_view(binding, self.clone())]
    }
}

impl<D, const N: usize> DescriptorUploadable for [D; N]
where D: DescriptorUploadable
{
    fn write(&self, binding: u32) -> Vec<WriteDescriptorSet> {
        let mut ret = Vec::new();
        for (i, w) in self.iter().enumerate() {
            ret.append(&mut w.write(binding + i as u32))
        }
        ret
    }
}

pub trait UploadSet {
    fn descriptor_set(&self) -> Arc<PersistentDescriptorSet>;
}

impl<T> UploadSet for UploadDescriptorSet<T>
where 
    T: DescriptorUploadable
{
    fn descriptor_set(&self) -> Arc<PersistentDescriptorSet> {
        self.set.clone()
    }
}

pub trait BindUploadDescriptorSet {
    fn bind_upload_set(
        &mut self, 
        pipeline_bind_point: PipelineBindPoint, 
        pipeline_layout: Arc<PipelineLayout>, 
        set_num: u32,
        upload_set: impl UploadSet,
    ) -> &mut Self;
}

impl BindUploadDescriptorSet for AutoCommandBufferBuilder<PrimaryAutoCommandBuffer> {
    fn bind_upload_set(
        &mut self, 
        pipeline_bind_point: PipelineBindPoint, 
        pipeline_layout: Arc<PipelineLayout>, 
        set_num: u32,
        upload_set: impl UploadSet,
    ) -> &mut Self {
        self.bind_descriptor_sets(
            pipeline_bind_point, 
            pipeline_layout, 
            set_num, 
            upload_set.descriptor_set()
        )
    }
}