use crate::model::person::Person;
use mysql::{Pool, params, prelude::*};

pub struct ServerState {
    pub pool: Pool,
}

impl ServerState {
    pub fn new(pool: Pool) -> Self {
        ServerState { pool }
    }

    pub fn get_person(&self, id: u32) -> Option<Person> {
        let mut conn = self.pool.get_conn().unwrap();
        let person: Option<(u32, String, String, String, String, String, String, u32)> = conn
            .exec_first(
                "SELECT * FROM person WHERE id = :id",
                params! { "id" => id },
            )
            .unwrap();

        person.map(
            |(id, name, email, phone, address, city, state, version)| Person {
                id,
                name,
                email,
                phone,
                address,
                city,
                state,
                version,
            },
        )
    }

    pub fn add_person(&mut self, name: String, age: u32) -> u32 {
        let mut conn = self.pool.get_conn().unwrap();
        conn.exec_drop(
            "INSERT INTO person (name, age) VALUES (:name, :age)",
            params! { "name" => &name, "age" => age },
        )
        .unwrap();
        conn.last_insert_id() as u32
    }

    pub fn update_person(&mut self, id: u32, name: String, age: u32) -> bool {
        let mut conn = self.pool.get_conn().unwrap();
        conn.exec_drop(
            "UPDATE persons SET name = :name, age = :age WHERE id = :id",
            mysql::params! { "name" => &name, "age" => age, "id" => id },
        )
        .unwrap();
        conn.affected_rows() > 0
    }
}
