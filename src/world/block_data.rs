use rustc_data_structures::stable_map::FxHashMap;

use crate::render::{texture::TextureAtlas, mesh::cube::UnitCube};

/// Static block data, should be initialized at startup and probably left alone.
pub struct StaticBlockData {
    inner: Vec<InitBlockData>,
    ids: FxHashMap<String, u32>,
}

impl StaticBlockData {
    pub fn empty() -> Self {
        Self { 
            inner: Default::default(),
            ids: FxHashMap::default(),
        }
    }

    pub fn init(&mut self, atlas: &TextureAtlas) {
        self.add(InitBlockData::new("air", None, BlockType::None));
        self.add(InitBlockData::new(
            "stone", 
            Some(UnitCube::from_textures([
                atlas.get_handle("stone").unwrap(),
            ].to_vec())),
            BlockType::Full,
        ));
        self.add(InitBlockData::new(
            "dirt", 
            Some(UnitCube::from_textures([
                atlas.get_handle("dirt").unwrap(),
            ].to_vec())),
            BlockType::Full,
        ));
        self.add(InitBlockData::new(
            "grass_block", 
            Some(UnitCube::from_textures([
                atlas.get_handle("grass_block_top").unwrap(),
                atlas.get_handle("grass_block_side").unwrap(),
                atlas.get_handle("dirt").unwrap(),
            ].to_vec())),
            BlockType::Full,
        ));
        self.add(InitBlockData::new(
            "leaves", 
            Some(UnitCube::from_textures([
                atlas.get_handle("leaves").unwrap(),
            ].to_vec())),
            BlockType::Transparent,
        ));
        self.add(InitBlockData::new(
            "log", 
            Some(UnitCube::from_textures([
                atlas.get_handle("log_top").unwrap(),
                atlas.get_handle("log_side").unwrap(),
                atlas.get_handle("log_top").unwrap(),
            ].to_vec())),
            BlockType::Full,
        ));
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

/// Represents the readable ID of this block as well as its model.
#[derive(Debug, Clone)]
pub struct InitBlockData {
    pub id: String,
    pub model: Option<UnitCube>,
    pub block_type: BlockType,
}

impl InitBlockData {
    pub fn new(id: &str, model: Option<UnitCube>, block_type: BlockType) -> Self {
        Self { id: id.to_string(), model, block_type }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub enum BlockType {
    None,
    Transparent,
    Full,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BlockHandle {
    inner: u32
}

impl BlockHandle {
    fn new(inner: u32) -> Self {
        Self { inner }
    }

    #[deprecated = "Should be replaced and unused ASAP"]
    pub fn new_unsafe(inner: u32) -> Self {
        Self { inner }
    }
}

// Unused, may use later if bottlenecked?

// struct InternalBlockIds {
//     num_to_id: FxHashMap<u32, usize>,
//     id_to_num: FxHashMap<String, usize>,
//     pairs: Vec<(u32, String)>,
//     pub size: usize,
// }

// impl InternalBlockIds {
//     pub fn new() -> Self {
//         Self {
//             num_to_id: FxHashMap::default(),
//             id_to_num: FxHashMap::default(),
//             pairs: Vec::new(),
//             size: 0,
//         }
//     }

//     pub fn get_id_of(&self, num: u32) -> Option<&str> {
//         if let Some(idx) = self.num_to_id.get(&num) {
//             let (_, id) = &self.pairs[*idx];
//             return Some(id.as_str())
//         }
//         None
//     }

//     pub fn get_num_of(&self, id: &str) -> Option<u32> {
//         if let Some(idx) = self.id_to_num.get(id) {
//             let (num, _) = self.pairs[*idx];
//             return Some(num)
//         }
//         None
//     }

//     /// Adds an ID to the internal ids and returns a u32 by which you can reference it.
//     pub fn add(&mut self, id: &str) -> u32 {
//         let pair = (self.size as u32, id.to_owned());
//         self.pairs.push(pair.clone());
//         self.num_to_id.insert(pair.0, self.size);
//         if let Some(_) = self.id_to_num.insert(pair.1, self.size) {
//             panic!("Attempted to add '{id}' more than once.");
//         }

//         self.size += 1;
//         pair.0
//     }
// }