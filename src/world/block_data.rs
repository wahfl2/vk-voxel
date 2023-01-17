use rustc_data_structures::stable_map::FxHashMap;

pub struct InternalBlockIds {
    num_to_id: FxHashMap<u32, usize>,
    id_to_num: FxHashMap<String, usize>,
    pairs: Vec<(u32, String)>,
}

impl InternalBlockIds {
    pub fn new() -> Self {
        Self {
            num_to_id: FxHashMap::default(),
            id_to_num: FxHashMap::default(),
            pairs: Vec::new(),
        }
    }

    pub fn get_id_of(&self, num: u32) -> Option<&str> {
        if let Some(idx) = self.num_to_id.get(&num) {
            let (_, id) = &self.pairs[*idx];
            return Some(id.as_str())
        }
        None
    }

    pub fn get_num_of(&self, id: &str) -> Option<u32> {
        if let Some(idx) = self.id_to_num.get(id) {
            let (num, _) = self.pairs[*idx];
            return Some(num)
        }
        None
    }
}