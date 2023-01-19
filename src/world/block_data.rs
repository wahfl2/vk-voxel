use rustc_data_structures::stable_map::FxHashMap;

pub struct StaticBlockData {
    // Could probably replace with a Vec; this is easier though
    inner: FxHashMap<String, InitBlockData>,
}

impl StaticBlockData {
    pub fn empty() -> Self {
        Self { inner: Default::default() }
    }

    pub fn init(&mut self) {
        self.inner = FxHashMap::from_iter([
            ("grass_block".into(), [0, 0, 0, 0, 0, 0].into())
        ].into_iter());
    }
}

/// Only represents face textures right now, will have more in the future.
pub struct InitBlockData {
    pub face_texures: [usize; 6],
}

impl From<[usize; 6]> for InitBlockData {
    fn from(value: [usize; 6]) -> Self {
        Self { face_texures: value }
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