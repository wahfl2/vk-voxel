pub struct Server {
    pub world: hecs::World,
}

impl Server {
    pub fn new() -> Self {
        Self {
            world: hecs::World::new(),
        }
    }

    pub fn tick(&mut self) {
        
    }
}