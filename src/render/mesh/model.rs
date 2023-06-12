use std::f32::consts::SQRT_2;

use ultraviolet::Vec3;

use crate::render::vertex::{Vertex, VertexRaw};

use super::quad::QuadUV;

#[derive(Debug, Clone)]
pub struct Model {
    pub vertices: Vec<Vertex>,
}

const HALF_SQRT_2: f32 = SQRT_2 * 0.5;
const QUARTER_SQRT_2: f32 = SQRT_2 * 0.25;
const QUAD_INDICES: [usize; 6] = [0, 1, 2, 0, 2, 3];
const PLANT_INDICES: [usize; 12] = [0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7];

const DEFAULT_PLANT_VERT_POSITIONS: [Vec3; 8] = [
    Vec3::new(-QUARTER_SQRT_2, -0.5, -QUARTER_SQRT_2),
    Vec3::new(QUARTER_SQRT_2, -0.5, QUARTER_SQRT_2),
    Vec3::new(QUARTER_SQRT_2, 0.5, QUARTER_SQRT_2),
    Vec3::new(-QUARTER_SQRT_2, 0.5, -QUARTER_SQRT_2),
    Vec3::new(QUARTER_SQRT_2, -0.5, -QUARTER_SQRT_2),
    Vec3::new(-QUARTER_SQRT_2, -0.5, QUARTER_SQRT_2),
    Vec3::new(-QUARTER_SQRT_2, 0.5, QUARTER_SQRT_2),
    Vec3::new(QUARTER_SQRT_2, 0.5, -QUARTER_SQRT_2),
];
const DEFAULT_PLANT_VERT_NORMALS: [Vec3; 8] = [
    Vec3::new(HALF_SQRT_2, 0.0, -HALF_SQRT_2),
    Vec3::new(HALF_SQRT_2, 0.0, -HALF_SQRT_2),
    Vec3::new(HALF_SQRT_2, 0.0, -HALF_SQRT_2),
    Vec3::new(HALF_SQRT_2, 0.0, -HALF_SQRT_2),
    Vec3::new(HALF_SQRT_2, 0.0, HALF_SQRT_2),
    Vec3::new(HALF_SQRT_2, 0.0, HALF_SQRT_2),
    Vec3::new(HALF_SQRT_2, 0.0, HALF_SQRT_2),
    Vec3::new(HALF_SQRT_2, 0.0, HALF_SQRT_2),
];

// https://rust-lang.github.io/rust-clippy/master/index.html#/new_without_default
impl Default for Model {
    fn default() -> Self {
        Model::new()
    }
}

impl Model {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
        }
    }

    pub fn push_quad(&mut self, quad: [Vertex; 4]) {
        self.vertices
            .extend(QUAD_INDICES.iter().map(|i| quad[*i].clone()));
    }

    pub fn translate(&mut self, translation: Vec3) {
        self.vertices
            .iter_mut()
            .for_each(|v| v.position += translation);
    }

    pub fn with_translation(&self, translation: Vec3) -> Self {
        let vertices = self
            .vertices
            .iter()
            .map(|v| Vertex {
                position: v.position + translation,
                normal: v.normal,
                tex_coords: v.tex_coords,
            })
            .collect();
        Self { vertices }
    }

    pub fn get_raw_vertices(&self) -> impl Iterator<Item = VertexRaw> + '_ {
        self.vertices.iter().map(|v| VertexRaw::from(v.clone()))
    }

    pub fn create_plant_model(uv: QuadUV) -> Self {
        let min_max = uv.tex_coords();
        let binding = [min_max, min_max];
        let mm = binding.flatten();

        let vertices = PLANT_INDICES
            .iter()
            .map(|i| Vertex {
                position: DEFAULT_PLANT_VERT_POSITIONS[*i],
                normal: DEFAULT_PLANT_VERT_NORMALS[*i],
                tex_coords: mm[*i],
            })
            .collect();

        Self { vertices }
    }
}
