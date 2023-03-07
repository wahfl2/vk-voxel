use derive_more::{Deref, DerefMut};
use hecs::{Entity, ComponentError};

/// Entities with this component are a parent of other entities, specified in this component's tuple
#[derive(Deref, DerefMut)]
pub struct Children(Vec<Entity>);

/// Entities with this component are a child of the entity in this component's tuple
#[derive(Deref, DerefMut)]
pub struct Parent(Entity);

pub trait Hierarchy {
    fn add_child(&mut self, parent: Entity, child: Entity);
    fn add_children(&mut self, parent: Entity, children: Vec<Entity>);

    fn set_parent(&mut self, child: Entity, parent: Entity);
}

impl Hierarchy for hecs::World {
    fn add_child(&mut self, parent: Entity, child: Entity) {
        let get_result = self.get::<&mut Children>(parent);

        if let Err(ComponentError::NoSuchEntity) = get_result {
            panic!("Error adding child; parent was despawned.");
        }

        if let Ok(mut p) = get_result {
            p.push(child);
            return
        }

        drop(get_result);
        self.insert_one(parent, Children(vec![child])).unwrap();
    }

    fn add_children(&mut self, parent: Entity, mut children: Vec<Entity>) {
        let get_result = self.get::<&mut Children>(parent);

        if let Err(ComponentError::NoSuchEntity) = get_result {
            panic!("Error adding children; parent was despawned.");
        }

        if let Ok(mut p) = get_result {
            p.append(&mut children);
            return
        }

        drop(get_result);
        self.insert_one(parent, Children(children)).unwrap();
    }

    fn set_parent(&mut self, child: Entity, parent: Entity) {
        let get_result = self.get::<&mut Parent>(child);

        if let Err(ComponentError::NoSuchEntity) = get_result {
            panic!("Error setting parent; child was despawned.");
        }

        if let Ok(mut p) = get_result {
            **p = parent;
            return
        }

        drop(get_result);
        self.insert_one(child, Parent(parent)).unwrap();
    }
}