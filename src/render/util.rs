use std::{sync::Arc, time::SystemTime};

use bytemuck::{Pod, Zeroable};
use guillotiere::euclid::{Box2D, Size2D, UnknownUnit};
use ultraviolet::UVec2;
use vulkano::{
    buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        AutoCommandBufferBuilder, CopyBufferInfo, CopyBufferInfoTyped, PrimaryAutoCommandBuffer,
    },
    image::ImageDimensions,
    memory::allocator::{AllocationCreateInfo, MemoryUsage, StandardMemoryAllocator},
    swapchain::Surface,
};
use winit::window::Window;

use super::mesh::quad::TexelTexture;

pub trait GetWindow {
    fn get_window(&self) -> Option<Arc<Window>>;
}

impl GetWindow for Arc<Surface> {
    fn get_window(&self) -> Option<Arc<Window>> {
        match self.object().unwrap().clone().downcast::<Window>() {
            Ok(w) => Some(w),
            Err(_) => None,
        }
    }
}

pub enum RenderState {
    Ok,
    Suboptimal,
    OutOfDate,
}

pub trait BoxToUV {
    fn to_quad_uv(self) -> TexelTexture;
}

impl BoxToUV for Box2D<i32, UnknownUnit> {
    fn to_quad_uv(self) -> TexelTexture {
        let size = self.max - self.min;
        let ret = TexelTexture::new(
            [self.min.x as u16, self.min.y as u16],
            [size.x as u16, size.y as u16],
        );
        println!("texel texture: {:?}", ret);
        ret
    }
}

pub trait VecConvenience {
    type Value;
    fn splat(v: Self::Value) -> Self;
    fn to_size_2d(self) -> Size2D<i32, UnknownUnit>;
    fn to_image_dimensions(self) -> ImageDimensions;
}

impl VecConvenience for UVec2 {
    type Value = u32;
    fn splat(v: Self::Value) -> Self {
        Self::new(v, v)
    }

    fn to_size_2d(self) -> Size2D<i32, UnknownUnit> {
        Size2D::new(self.x as i32, self.y as i32)
    }

    fn to_image_dimensions(self) -> ImageDimensions {
        ImageDimensions::Dim2d {
            width: self.x,
            height: self.y,
            array_layers: 1,
        }
    }
}

pub trait Reversed {
    fn reversed(self) -> Self;
}

impl<T, const N: usize> Reversed for [T; N] {
    fn reversed(mut self) -> Self {
        self.reverse();
        self
    }
}

pub trait CreateInfoConvenience {
    type UsageType;
    fn usage(usage: Self::UsageType) -> Self;
}

impl CreateInfoConvenience for BufferCreateInfo {
    type UsageType = BufferUsage;

    fn usage(usage: Self::UsageType) -> Self {
        Self {
            usage,
            ..Default::default()
        }
    }
}

impl CreateInfoConvenience for AllocationCreateInfo {
    type UsageType = MemoryUsage;

    fn usage(usage: Self::UsageType) -> Self {
        Self {
            usage,
            ..Default::default()
        }
    }
}

pub fn make_device_only_buffer_slice<T, I>(
    allocator: &StandardMemoryAllocator,
    cbb: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    usage: BufferUsage,
    data: I,
) -> Subbuffer<[T]>
where
    T: BufferContents + Clone,
    I: IntoIterator<Item = T>,
    I::IntoIter: ExactSizeIterator,
{
    let iter = data.into_iter();
    let len = iter.len();
    let staging = Buffer::from_iter(
        allocator,
        BufferCreateInfo::usage(BufferUsage::TRANSFER_SRC),
        AllocationCreateInfo::usage(MemoryUsage::Upload),
        iter,
    )
    .unwrap();

    let ret = Buffer::new_slice(
        allocator,
        BufferCreateInfo::usage(usage.union(BufferUsage::TRANSFER_DST)),
        AllocationCreateInfo::usage(MemoryUsage::DeviceOnly),
        len as u64,
    )
    .unwrap();

    cbb.copy_buffer(CopyBufferInfoTyped::buffers(staging, ret.clone()))
        .unwrap();
    ret
}

pub fn make_device_only_buffer_sized<T>(
    allocator: &StandardMemoryAllocator,
    cbb: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    usage: BufferUsage,
    data: T,
) -> Subbuffer<T>
where
    T: Send + Sync + Pod,
{
    let staging = Buffer::from_data(
        allocator,
        BufferCreateInfo::usage(BufferUsage::TRANSFER_SRC),
        AllocationCreateInfo::usage(MemoryUsage::Upload),
        data,
    )
    .unwrap();

    let ret = Buffer::new_sized(
        allocator,
        BufferCreateInfo::usage(usage.union(BufferUsage::TRANSFER_DST)),
        AllocationCreateInfo::usage(MemoryUsage::DeviceOnly),
    )
    .unwrap();

    cbb.copy_buffer(CopyBufferInfo::buffers(staging, ret.clone()))
        .unwrap();
    ret
}

pub fn make_download_buffer_sized<T>(
    allocator: &StandardMemoryAllocator,
    cbb: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    usage: BufferUsage,
    data: T,
) -> Subbuffer<T>
where
    T: Send + Sync + Pod,
{
    let staging = Buffer::from_data(
        allocator,
        BufferCreateInfo::usage(BufferUsage::TRANSFER_SRC),
        AllocationCreateInfo::usage(MemoryUsage::Upload),
        data,
    )
    .unwrap();

    let ret = Buffer::new_sized(
        allocator,
        BufferCreateInfo::usage(usage.union(BufferUsage::TRANSFER_DST)),
        AllocationCreateInfo::usage(MemoryUsage::Download),
    )
    .unwrap();

    cbb.copy_buffer(CopyBufferInfo::buffers(staging, ret.clone()))
        .unwrap();
    ret
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ProgramInfo {
    pub frame_number: u32,
    pub start: u32,
}

// https://rust-lang.github.io/rust-clippy/master/index.html#/new_without_default
impl Default for ProgramInfo {
    fn default() -> Self {
        ProgramInfo::new()
    }
}

impl ProgramInfo {
    pub fn new() -> Self {
        Self {
            frame_number: 0,
            start: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos(),
        }
    }
}
