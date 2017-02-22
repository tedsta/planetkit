use specs;

pub struct Collision {
    pub globe_entity: Option<specs::Entity>,
}

impl Collision {
    pub fn new(globe_entity: Option<specs::Entity>) -> Collision {
        Collision {
            globe_entity: globe_entity,
        }
    }
}

impl specs::Component for Collision {
    type Storage = specs::HashMapStorage<Collision>;
}
