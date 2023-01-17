use std::{fs::File, sync::Arc, ffi::OsString};

use glob::glob;
use guillotiere::{SimpleAtlasAllocator, euclid::{Box2D, UnknownUnit}};
use ultraviolet::UVec2;
use vulkano::{memory::allocator::FastMemoryAllocator, command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer}, image::{ImmutableImage, MipmapsCount, view::ImageView}, format::Format};

use super::{util::{VecConvenience, BoxToUV}, mesh::quad::QuadUV};

pub struct TextureAtlas {
    pub data: ImageData,
    pub allocations: Vec<Box2D<i32, UnknownUnit>>,
    pub uvs: Vec<QuadUV>,
}

impl TextureAtlas {
    pub fn from_folder(folder_path: &str) -> Self {
        let mut paths = Vec::new();
        let glob = glob(format!("{}/**/*.png", folder_path).as_str()).unwrap();
        let mut num_images = 0;
        for path in glob {
            num_images += 1;
            paths.push(path.unwrap().into_os_string());
        }
        println!("{:?}", num_images);
        Self::from_images(paths)
    }

    pub fn from_images(paths: Vec<OsString>) -> Self {
        let mut images = Vec::new();
        let mut total_image_area = 0;
        for path in paths.into_iter() {
            let image = ImageData::new_file(path);
            total_image_area += image.dimensions.x * image.dimensions.y;
            images.push(image);
        }

        // Double the total area should be enough... right?
        let atlas_size = UVec2::splat(((total_image_area * 2) as f32).sqrt() as u32);
        let mut allocator = SimpleAtlasAllocator::new(atlas_size.to_size_2d());

        let mut atlas_data = vec![0u8; (atlas_size.x * atlas_size.y * 4) as usize];
        let atlas_row_len = atlas_size.x as usize;
        let mut allocations = Vec::new();
        let mut uvs = Vec::new();
        for image in images.iter() {
            let alloc = match allocator.allocate(image.dimensions.to_size_2d()) {
                Some(a) => a,
                None => panic!("Ran out of space in the altas!")
            };

            let image_row_len = image.dimensions.x as usize;
            for row in 0..image.dimensions.y as usize {
                let image_row_start = row * image_row_len;
                let image_row_end = (row + 1) * image_row_len;

                let alloc_min_idx = alloc.min.y as usize * atlas_row_len + alloc.min.x as usize;
                let atlas_row_start = row * atlas_row_len + alloc_min_idx;
                let atlas_row_end = atlas_row_start + image_row_len;
                
                atlas_data[(atlas_row_start * 4)..(atlas_row_end * 4)].copy_from_slice(
                    &image.data[(image_row_start * 4)..(image_row_end * 4)]
                );
            }

            allocations.push(alloc);
            uvs.push(alloc.to_quad_uv(atlas_size));
        }

        Self {
            data: ImageData::new(atlas_data, atlas_size),
            allocations,
            uvs,
        }
    }

    pub fn get_texture(
        &self,
        allocator: &FastMemoryAllocator,
        cbb: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ) -> Arc<ImageView<ImmutableImage>> {
        let image = ImmutableImage::from_iter(
            allocator,
            self.data.data.clone(),
            self.data.dimensions.to_image_dimensions(),
            MipmapsCount::One,
            Format::R8G8B8A8_SRGB,
            cbb,
        ).unwrap();

        ImageView::new_default(image).unwrap()
    }

    pub fn get_uv(&self, texture_idx: usize) -> QuadUV {
        let alloc = self.allocations[texture_idx];
        alloc.to_quad_uv(self.data.dimensions)
    }
}

pub struct ImageData {
    pub data: Vec<u8>,
    pub dimensions: UVec2,
}

impl ImageData {
    pub fn new_file(path: OsString) -> Self {
        let decoder = png::Decoder::new(File::open(path).unwrap());
        let mut reader = decoder.read_info().unwrap();

        let info = reader.info();
        let dimensions = UVec2::new(info.width, info.height);

        let mut buf = vec![0; reader.output_buffer_size()];
        let bpp = info.bytes_per_pixel();
        reader.next_frame(&mut buf).unwrap();

        match bpp {
            4 => Self { data: buf, dimensions },
            3 => {
                let data: Vec<u8> = buf.chunks(3).map(|chunk| {
                    [chunk[0], chunk[1], chunk[2], 255]
                }).flatten().collect();
                Self { data, dimensions }
            },
            p => panic!("Unsupported bytes per pixel: {p}")
        }
    }

    pub fn new(data: Vec<u8>, dimensions: UVec2) -> Self {
        Self { data, dimensions }
    }
}

#[test]
fn glob_test() {
    for path in glob("./*").unwrap() {
        println!("{:?}", path.unwrap().display());
    }
}