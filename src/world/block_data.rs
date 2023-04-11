use ahash::HashMap;

use crate::render::{texture::TextureAtlas, mesh::{cube::UnitCube, quad::QuadUV, model::Model}};

/// Static block data, should be initialized at startup and probably left alone.
pub struct StaticBlockData {
    inner: Vec<InitBlockData>,
    ids: HashMap<String, u32>,
}

impl StaticBlockData {
    pub fn empty() -> Self {
        Self { 
            inner: Default::default(),
            ids: HashMap::default(),
        }
    }

    pub fn init(&mut self, atlas: &TextureAtlas) {
        self.add(InitBlockData::air());
        self.add(InitBlockData::new_block(
            "stone", 
            Some(UnitCube::new([
                atlas.get_handle("stone").unwrap(),
            ].to_vec()).unwrap()),
            BlockType::Full,
        ));
        self.add(InitBlockData::new_block(
            "dirt", 
            Some(UnitCube::new([
                atlas.get_handle("dirt").unwrap(),
            ].to_vec()).unwrap()),
            BlockType::Full,
        ));
        self.add(InitBlockData::new_block(
            "grass_block", 
            Some(UnitCube::new([
                atlas.get_handle("grass_block_top").unwrap(),
                atlas.get_handle("grass_block_side").unwrap(),
                atlas.get_handle("dirt").unwrap(),
            ].to_vec()).unwrap()),
            BlockType::Full,
        ));
        self.add(InitBlockData::new_block(
            "leaves", 
            Some(UnitCube::new([
                atlas.get_handle("leaves").unwrap(),
            ].to_vec()).unwrap()),
            BlockType::Transparent,
        ));
        self.add(InitBlockData::new_block(
            "log", 
            Some(UnitCube::new([
                atlas.get_handle("log_top").unwrap(),
                atlas.get_handle("log_side").unwrap(),
                atlas.get_handle("log_top").unwrap(),
            ].to_vec()).unwrap()),
            BlockType::Full,
        ));

        self.add(InitBlockData::new_plant("grass", atlas.get_uv(atlas.get_handle("grass").unwrap())));
    }

    pub fn add(&mut self, data: InitBlockData) -> BlockHandle {
        let idx = self.inner.len() as u32;
        self.inner.push(data.clone());
        self.ids.insert(data.id, idx);
        BlockHandle::new(idx)
    }

    pub fn get(&self, handle: &BlockHandle) -> &InitBlockData {
        self.inner.get(handle.inner as usize).unwrap()
    }

    pub fn get_handle(&self, id: &str) -> Option<BlockHandle> {
        let idx = self.ids.get(id)?;
        Some(BlockHandle::new(*idx))
    }
}

#[repr(u32)]
pub enum Blocks {
    Air,
    Stone,
    Dirt,
    Grass,
    Leaves,
    Log,
}

impl Blocks {
    pub fn handle(self) -> BlockHandle {
        BlockHandle::new(self as u32)
    }
}

/// Represents the readable ID of this block as well as its model.
#[derive(Debug, Clone)]
pub struct InitBlockData {
    pub id: String,
    pub model: ModelType,
    pub block_type: BlockType,
}

impl InitBlockData {
    pub fn air() -> Self {
        Self {
            id: "air".to_string(),
            model: ModelType::None,
            block_type: BlockType::None
        }
    }

    pub fn new_block(id: &str, model: Option<UnitCube>, block_type: BlockType) -> Self {
        Self { id: id.to_string(), model: model.into(), block_type }
    }

    pub fn new_plant(id: &str, uv: QuadUV) -> Self {
        let plant_model = Model::create_plant_model(uv);
        Self { id: id.to_string(), model: ModelType::Plant(plant_model), block_type: BlockType::Transparent }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub enum BlockType {
    Full,
    Transparent,
    None,
}

#[derive(Clone, Debug)]
pub enum ModelType {
    FullBlock(UnitCube),
    Plant(Model),
    None,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BlockHandle {
    inner: u32
}

impl BlockHandle {
    fn new(inner: u32) -> Self {
        Self { inner }
    }

    pub fn new_unchecked(inner: u32) -> Self {
        Self { inner }
    }
}

impl From<Option<UnitCube>> for ModelType {
    fn from(value: Option<UnitCube>) -> Self {
        match value {
            Some(m) => Self::FullBlock(m),
            None => Self::None,
        }
    }
}