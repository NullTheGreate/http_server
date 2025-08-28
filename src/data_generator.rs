use crate::model::person::Person;
use std::sync::mpsc::Sender;

pub struct DataGenerator;

impl DataGenerator {
    pub fn new() -> Self {
        DataGenerator
    }

    pub fn generate(&self, count: u32, start_id: u32, tx: Sender<Vec<Person>>) {
        let mut persons = Vec::with_capacity(count as usize);
        for i in 0..count {
            persons.push(Person {
                id: 0,
                name: format!("name {}", start_id + i),
                email: format!("email{}@example.com", start_id + i),
                phone: format!("{}", start_id + i),
                address: format!("address {}", start_id + i),
                city: format!("city {}", start_id + i),
                state: format!("state {}", start_id + i),
                version: 0,
            }); // ID will be set by DB
        }
        let send = tx.send(persons);
        if let Err(e) = send {
            eprintln!("Failed to send generated data: {}", e);
        }
    }
}
