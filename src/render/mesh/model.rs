use std::f32::consts::{FRAC_1_SQRT_2, SQRT_2};

use once_cell::sync::Lazy;
use ultraviolet::Vec3;

use crate::render::vertex::{Vertex, VertexRaw};

use super::quad::QuadUV;

#[derive(Debug, Clone)]
pub struct Model {
    pub vertices: Vec<Vertex>
}

const HALF_SQRT_2: f32 = SQRT_2 * 0.5;
const QUAD_INDICES: [usize; 6] = [0, 1, 2, 0, 2, 3];
const DEFAULT_PLANT_VERT_POSITIONS: [Vec3; 8] = [
    Vec3::new(-HALF_SQRT_2, -0.5, -HALF_SQRT_2),
    Vec3::new( HALF_SQRT_2, -0.5,  HALF_SQRT_2),
    Vec3::new( HALF_SQRT_2,  0.5,  HALF_SQRT_2),
    Vec3::new(-HALF_SQRT_2,  0.5, -HALF_SQRT_2),

    Vec3::new( HALF_SQRT_2, -0.5, -HALF_SQRT_2),
    Vec3::new(-HALF_SQRT_2, -0.5,  HALF_SQRT_2),
    Vec3::new(-HALF_SQRT_2,  0.5,  HALF_SQRT_2),
    Vec3::new( HALF_SQRT_2,  0.5, -HALF_SQRT_2),
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

impl Model {
    pub fn new() -> Self {
        Self { vertices: Vec::new() }
    }

    pub fn push_quad(&mut self, quad: [Vertex; 4]) {
        self.vertices.extend(QUAD_INDICES.iter().map(|i| { quad[*i].clone() }));
    }

    pub fn translate(&mut self, translation: Vec3) {
        self.vertices.iter_mut().for_each(|v| { v.position += translation });
    }

    pub fn with_translation(&self, translation: Vec3) -> Self {
        let vertices = self.vertices.iter().map(|v| {
            Vertex {
                position: v.position + translation,
                normal: v.normal,
                tex_coords: v.tex_coords,
            }
        }).collect();
        Self { vertices }
    }

    pub fn get_raw_vertices(&self) -> Vec<VertexRaw> {
        self.vertices.iter().map(|v| { VertexRaw::from(v.clone()) }).collect()
    }

    pub fn create_plant_model(uv: QuadUV) -> Self {
        let min_max = uv.tex_coords();
        let vertices = DEFAULT_PLANT_VERT_POSITIONS.iter()
            .zip(DEFAULT_PLANT_VERT_NORMALS.iter())
            .zip([min_max, min_max].flatten().into_iter())
            .map(|((pos, normal), tex_coord)| {
                Vertex {
                    position: *pos,
                    normal: *normal,
                    tex_coords: *tex_coord,
                }
            }).collect();

        Self { vertices }
    }
}