use crate::model::person::Person;
use std::collections::HashMap;

pub struct ServerState {
    pub persons: HashMap<u32, Person>,
    pub next_id: u32,
}

impl ServerState {
    pub fn new() -> Self {
        ServerState {
            persons: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn get_person(&self, id: u32) -> Option<&Person> {
        self.persons.get(&id)
    }

    pub fn add_person(&mut self, name: String, age: u32) -> u32 {
        let id = self.next_id;
        self.persons.insert(id, Person { id, name, age });
        self.next_id += 1;
        id
    }

    pub fn update_person(&mut self, id: u32, name: String, age: u32) -> bool {
        if self.persons.contains_key(&id) {
            self.persons.insert(id, Person { id, name, age });
            true
        } else {
            false
        }
    }
}
