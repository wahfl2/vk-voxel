use vulkano::buffer::BufferContents;

#[repr(C)]
#[derive(BufferContents)]
pub struct Surfel {
    pub accumulated: [f32; 3],
    pub radius: f32,
    pub normal: [u16; 3],
    pub offset: [u16; 3],
}