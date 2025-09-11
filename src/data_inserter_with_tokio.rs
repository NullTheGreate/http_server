use crate::model::person::{self, Person};
use mysql::{Pool, TxOpts, params, prelude::*};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::task;

pub struct DataInserterWithTokio {
    pool: Pool,
}

impl DataInserterWithTokio {
    pub fn new(pool: Pool) -> Self {
        DataInserterWithTokio { pool }
    }

    pub async fn populate(&self, count: u32) -> mysql::Result<Duration> {
        let start_time = Instant::now();
        const BATCH_SIZE: u32 = 1000;
        const GENERATOR_THREADS: u32 = 4;
        const INSERTER_THREADS: u32 = 2;

        let (tx, rx): (Sender<Vec<Person>>, Receiver<Vec<Person>>) = std::sync::mpsc::channel();
        let rx: Arc<Mutex<Receiver<Vec<Person>>>> = Arc::new(Mutex::new(rx));
        let mut generator_handles = vec![];
        let chunk_size =
            count / GENERATOR_THREADS + if count % GENERATOR_THREADS > 0 { 1 } else { 0 };
        for i in 0..GENERATOR_THREADS {
            let start_id = i * chunk_size;
            let generate_count = if i == GENERATOR_THREADS - 1 {
                if count - (i * chunk_size) > chunk_size {
                    chunk_size
                } else {
                    count - (i * chunk_size)
                }
            } else {
                chunk_size
            };
            if generate_count == 0 {
                continue;
            }
            let tx = tx.clone();
            let generator = crate::data_generator::DataGenerator::new();
            generator_handles.push(task::spawn_blocking(move || {
                generator.generate(generate_count, start_id, tx);
                // () // Explicitly return () to clarify closure return type
            }));
        }

        let pool = self.pool.clone();
        let mut inserter_handles = vec![];
        for _ in 0..INSERTER_THREADS {
            let rx = Arc::clone(&rx);
            let pool = pool.clone();
            inserter_handles.push(task::spawn_blocking(move || {
                let mut conn = match pool.get_conn() {
                    Ok(conn) => conn,
                    Err(e) => {
                        eprintln!("Failed to get database connection: {}", e);
                        return;
                    }
                };
                let tx_opts = TxOpts::default();
                let mut tx = match conn.start_transaction(tx_opts) {
                    Ok(tx) => tx,
                    Err(e) => {
                        eprintln!("Failed to start transaction: {}", e);
                        return;
                    }
                };

                loop {
                    let persons = {
                        let rx = match rx.lock() {
                            Ok(rx) => rx,
                            Err(e) => {
                                eprintln!("Failed to lock receiver: {}", e);
                                return;
                            }
                        };
                        match rx.recv() {
                            Ok(persons) => persons,
                            Err(_) => break, // Channel closed
                        }
                    };

                    for person in persons.chunks(BATCH_SIZE as usize)  {
                        let params: Vec<_> = person.iter().map(|p| {
                        params! {
                                "name" => &p.name,
                                "email" => &p.email,
                                "phone" => &p.phone,
                                "address" => &p.address,
                                "city" => &p.city,
                                "state" => &p.state,
                                "version" => p.version,
                            }
                        }).collect();
                        if let Err(e) = tx.exec_batch(
                        "INSERT INTO person (name, email, phone, address, city, state, version) VALUES (:name, :email, :phone, :address, :city, :state, :version)",
                        params.iter(),
                    ) {
                        eprintln!("Failed to execute batch insert: {}", e);
                        return;
                    }
                    }

                    

                    
                }

                if let Err(e) = tx.commit() {
                    eprintln!("Failed to commit transaction: {}", e);
                }
            }));
        }

        // Wait for generators to finish
        for handle in generator_handles {
            if let Err(e) = handle.await {
                eprintln!("Generator task failed: {:?}", e);
            }
        }

        // Drop the original sender to close the channel
        drop(tx);

        // Wait for inserters to finish
        for handle in inserter_handles {
            if let Err(e) = handle.await {
                eprintln!("Inserter task failed: {:?}", e);
            }
        }

        Ok(start_time.elapsed())
    }
}
