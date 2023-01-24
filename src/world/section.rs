use std::ops::{Index, IndexMut, Mul};

use ultraviolet::{UVec3, Vec3};

use crate::{render::{mesh::{renderable::Renderable, quad::TexturedSquare}, texture::TextureAtlas, vertex::VertexRaw}, util::{Facing, Axis, Sign}};

use super::{block_access::BlockAccess, block_data::{BlockHandle, StaticBlockData, BlockType}};

pub struct Section {
    blocks: [[[BlockHandle; 16]; 16]; 16],
    mesh: Vec<VertexRaw>,
}

impl BlockAccess for Section {
    fn get_block(&self, pos: UVec3) -> BlockHandle {
        self.blocks[pos.x as usize][pos.y as usize][pos.z as usize]
    }

    fn set_block(&mut self, pos: UVec3, block: BlockHandle) {
        self.blocks[pos.x as usize][pos.y as usize][pos.z as usize] = block;
    }
}

impl Section {
    pub fn empty() -> Self {
        Self {
            blocks: [[[BlockHandle::default(); 16]; 16]; 16],
            mesh: Vec::new(),
        }
    }

    pub fn flat_iter(&self) -> impl Iterator<Item = (UVec3, &BlockHandle)> {
        self.blocks.iter().enumerate()
            .flat_map(|(x, b)| { b.iter().enumerate()
                .flat_map(move |(y, b)| { b.iter().enumerate()
                    .map(move |(z, b)| { (UVec3::new(x as u32, y as u32, z as u32), b) }) }
                ) }
            )
    }

    pub fn get_column(&self, x: usize, z: usize) -> [&BlockHandle; 16] {
        let x_plane = self.blocks.get(x).unwrap();
        let column: Vec<&BlockHandle> = x_plane.iter().map(
            |f| { f.get(z).unwrap() }
        ).collect();

        column[..16].try_into().unwrap()
    }

    pub fn column_iter_mut(&mut self, x: usize, z: usize) -> impl Iterator<Item = &mut BlockHandle> {
        let x_plane = self.blocks.get_mut(x).unwrap();
        x_plane.iter_mut().map(move |p| { p.get_mut(z).unwrap() })
    }

    pub fn rebuild_mesh(&mut self, offset: Vec3, atlas: &TextureAtlas, block_data: &StaticBlockData) {
        // Iterate through every single block with the pos attached
        let iter = self.blocks.iter().enumerate()
            .flat_map(|(x, b)| { b.iter().enumerate()
                .flat_map(move |(y, b)| { b.iter().enumerate()
                    .filter_map(move |(z, b)| { 
                        if block_data.get(b).block_type == BlockType::None { return None } 
                        Some((UsizeVec3::new(x, y, z), b))
                    })
                })
            });

        let mut quads = Vec::new();
        for (pos, handle) in iter {
            let mut faces_op: Option<[TexturedSquare; 6]> = None;
            let block_offset = pos.into_vec3() + offset;
            for (i, neighbor) in self.get_neighbors(pos).into_iter().enumerate() {
                if let Neighbor::Block { handle, pos } = neighbor {
                    if block_data.get(&handle).block_type == BlockType::Full {
                        continue;
                    }
                }

                let face_idx = i;
                let mut face = match faces_op.as_ref() {
                    Some(faces) => faces[face_idx].clone(),
                    None => {
                        let faces = block_data.get(handle).model.unwrap().get_faces();
                        let ret = faces[face_idx].clone();
                        faces_op = Some(faces);
                        ret
                    }
                };

                face.center += block_offset;
                quads.push(face);
            }
        }
        self.mesh = quads.get_vertices(atlas, block_data);
    }

    fn get_neighbors(&self, pos: UsizeVec3) -> [Neighbor; 6] {
        let mut neighbors = [Neighbor::Boundary; 6];

        if pos.x < 15 { neighbors[0] = self.neighbor_block((pos.x + 1, pos.y, pos.z).into()); }
        if pos.x > 0  { neighbors[1] = self.neighbor_block((pos.x - 1, pos.y, pos.z).into()); }
        if pos.y < 15 { neighbors[2] = self.neighbor_block((pos.x, pos.y + 1, pos.z).into()); }
        if pos.y > 0  { neighbors[3] = self.neighbor_block((pos.x, pos.y - 1, pos.z).into()); }
        if pos.z < 15 { neighbors[4] = self.neighbor_block((pos.x, pos.y, pos.z + 1).into()); }
        if pos.z > 0  { neighbors[5] = self.neighbor_block((pos.x, pos.y, pos.z - 1).into()); }
        
        neighbors
    }

    fn neighbor_block(&self, pos: UsizeVec3) -> Neighbor {
        Neighbor::new_block(self.blocks[pos], pos)
    }
}

#[derive(Copy, Clone, Debug)]
struct UsizeVec3 {
    x: usize,
    y: usize,
    z: usize,
}

impl UsizeVec3 {
    fn new(x: usize, y: usize, z: usize) -> Self {
        Self { x, y, z }
    }
    
    fn into_vec3(self) -> Vec3 {
        Vec3::new(self.x as f32, self.y as f32, self.z as f32)
    }
}

impl<T, const N: usize, const M: usize, const P: usize> Index<UsizeVec3> for [[[T; N]; M]; P] {
    type Output = T;

    fn index(&self, index: UsizeVec3) -> &Self::Output {
        &self[index.x][index.y][index.z]
    }
}

impl<T, const N: usize, const M: usize, const P: usize> IndexMut<UsizeVec3> for [[[T; N]; M]; P] {
    fn index_mut(&mut self, index: UsizeVec3) -> &mut Self::Output {
        &mut self[index.x][index.y][index.z]
    }
}

impl Mul<usize> for UsizeVec3 {
    type Output = UsizeVec3;

    fn mul(self, rhs: usize) -> Self::Output {
        (self.x * rhs, self.y * rhs, self.z * rhs).into()
    }
}

impl From<(usize, usize, usize)> for UsizeVec3 {
    fn from(value: (usize, usize, usize)) -> Self {
        Self::new(value.0, value.1, value.2)
    }
}

#[derive(Copy, Clone, Debug)]
enum Neighbor {
    Boundary,
    Block {
        handle: BlockHandle,
        pos: UsizeVec3,
    },
}

impl Neighbor {
    fn new_block(handle: BlockHandle, pos: UsizeVec3) -> Self {
        Self::Block { handle, pos }
    }
}

impl Renderable for Section {
    fn get_vertices(&self, atlas: &TextureAtlas, block_data: &StaticBlockData) -> Vec<VertexRaw> {
        self.mesh.clone()
    }
}