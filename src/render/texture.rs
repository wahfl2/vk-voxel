use std::{ffi::OsString, fs::File, path::PathBuf, sync::Arc};

use ahash::HashMap;
use glob::glob;
use guillotiere::{
    euclid::{Box2D, UnknownUnit},
    SimpleAtlasAllocator,
};
use png::{ColorType, Transformations};
use ultraviolet::UVec2;
use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer},
    format::Format,
    image::{view::ImageView, ImmutableImage, MipmapsCount},
    memory::allocator::StandardMemoryAllocator,
};

use super::{
    mesh::quad::TexelTexture,
    util::{BoxToUV, VecConvenience},
};

pub struct TextureAtlas {
    /// Map that matches file names to the index of the texture
    pub name_index_map: HashMap<String, usize>,
    pub data: ImageData,
    allocations: Vec<Box2D<i32, UnknownUnit>>,
    pub uvs: Vec<TexelTexture>,
}

impl TextureAtlas {
    pub fn from_folder(folder_path: &str) -> Self {
        let mut paths = Vec::new();
        let glob = glob(format!("{}/**/*.png", folder_path).as_str()).unwrap();
        for path in glob {
            paths.push(path.unwrap());
        }
        Self::from_images(paths)
    }

    pub fn from_images(paths: Vec<PathBuf>) -> Self {
        let mut images = Vec::new();
        let mut name_index_map = HashMap::default();
        let mut total_image_area = 0;
        for (index, path) in paths.into_iter().enumerate() {
            let file_name = path.file_stem().unwrap().to_str().unwrap().to_owned();
            name_index_map.insert(file_name, index);

            let image = ImageData::new_file(path.into());
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
                None => panic!("Ran out of space in the altas!"),
            };

            let image_row_len = image.dimensions.x as usize;
            for row in 0..image.dimensions.y as usize {
                let image_row_start = row * image_row_len;
                let image_row_end = (row + 1) * image_row_len;

                let alloc_min_idx = alloc.min.y as usize * atlas_row_len + alloc.min.x as usize;
                let atlas_row_start = row * atlas_row_len + alloc_min_idx;
                let atlas_row_end = atlas_row_start + image_row_len;

                atlas_data[(atlas_row_start * 4)..(atlas_row_end * 4)]
                    .copy_from_slice(&image.data[(image_row_start * 4)..(image_row_end * 4)]);
            }

            allocations.push(alloc);
            uvs.push(alloc.to_quad_uv());
        }

        Self {
            name_index_map,
            data: ImageData::new(atlas_data, atlas_size),
            allocations,
            uvs,
        }
    }

    pub fn get_handle(&self, file_name: &str) -> Option<TextureHandle> {
        if let Some(idx) = self.name_index_map.get(file_name) {
            return Some(TextureHandle {
                inner_index: *idx as u32,
            });
        }
        None
    }

    pub fn get_texture(
        &self,
        allocator: &StandardMemoryAllocator,
        cbb: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ) -> Arc<ImageView<ImmutableImage>> {
        let image = ImmutableImage::from_iter(
            allocator,
            self.data.data.clone(),
            self.data.dimensions.to_image_dimensions(),
            MipmapsCount::One,
            Format::R8G8B8A8_UNORM,
            cbb,
        )
        .unwrap();

        ImageView::new_default(image).unwrap()
    }

    pub fn get_uv(&self, handle: TextureHandle) -> TexelTexture {
        let alloc = self.allocations[handle.inner_index as usize];
        alloc.to_quad_uv()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TextureHandle {
    inner_index: u32,
}

impl TextureHandle {
    pub fn index(&self) -> u32 {
        self.inner_index
    }
}

pub struct ImageData {
    pub data: Vec<u8>,
    pub dimensions: UVec2,
}

impl ImageData {
    pub fn new_file(path: OsString) -> Self {
        // `path` implements `Copy`, so there is no need to clone.
        let mut decoder = png::Decoder::new(File::open(path).unwrap());
        decoder.set_transformations(Transformations::normalize_to_color8());
        let mut reader = decoder.read_info().unwrap();

        let mut buf = vec![0; reader.output_buffer_size()];
        reader.next_frame(&mut buf).unwrap();

        let info = reader.info().to_owned();
        let dimensions = UVec2::new(info.width, info.height);

        let transformed_color_type = match info.color_type {
            ColorType::Indexed => ColorType::Rgb,
            c => c,
        };

        let data = match transformed_color_type {
            ColorType::Grayscale => buf
                .into_iter()
                .flat_map(|gray| [gray, gray, gray, 255])
                .collect(),

            ColorType::GrayscaleAlpha => buf
                .chunks(2)
                .flat_map(|chunk| {
                    let (gray, alpha) = (chunk[0], chunk[1]);
                    [gray, gray, gray, alpha]
                })
                .collect(),

            ColorType::Rgb => buf
                .chunks(3)
                .flat_map(|chunk| {
                    let (r, g, b) = (chunk[0], chunk[1], chunk[2]);
                    [r, g, b, 255]
                })
                .collect(),

            ColorType::Rgba => buf,

            ColorType::Indexed => unreachable!(),
        };

        Self { data, dimensions }
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
