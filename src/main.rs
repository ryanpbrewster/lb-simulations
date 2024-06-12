use std::collections::HashMap;

use clap::Parser;
use rand::{rngs::SmallRng, Rng, SeedableRng};

/*
Sample output from
$ cargo run --release -- --iterations=1000000

[a] 601164
[b] 600333
[b] 598227
[b] 600936
[b] 598203
[b] 599478
[c] 599397
[c] 598491
[c] 600216
[c] 600138
[c] 601620
[c] 600462
[c] 597711
[c] 602604
[c] 601020

*/

fn main() {
    let args = Args::parse();

    let backends: Vec<Backend> = {
        let mut acc = Vec::new();
        for _ in 0..1 {
            acc.push(Backend {
                id: acc.len() as u32,
                zone: 'a',
            });
        }
        for _ in 0..5 {
            acc.push(Backend {
                id: acc.len() as u32,
                zone: 'b',
            });
        }
        for _ in 0..9 {
            acc.push(Backend {
                id: acc.len() as u32,
                zone: 'c',
            });
        }
        acc
    };
    let clients: Vec<Client> = {
        let mut acc = Vec::new();
        for _ in 0..3 {
            acc.push(Client::new('a', backends.clone()));
        }
        for _ in 0..3 {
            acc.push(Client::new('b', backends.clone()));
        }
        for _ in 0..3 {
            acc.push(Client::new('c', backends.clone()));
        }
        acc
    };

    let mut tally = vec![0; backends.len()];
    for mut client in clients {
        for _ in 0..args.iterations {
            tally[client.sample() as usize] += 1;
        }
    }

    for (backend, count) in backends.iter().zip(tally) {
        println!("[{zone}] {count}", zone = backend.zone, count = count);
    }
}

#[derive(Parser)]
struct Args {
    #[arg(long, default_value_t = 1_000)]
    iterations: u64,
}

#[derive(Clone)]
struct Client {
    zone: char,
    backends: Vec<Backend>,
    rho: f64,
    prng: SmallRng,
}
impl Client {
    fn new(zone: char, backends: Vec<Backend>) -> Self {
        let capacities = {
            let mut acc: HashMap<char, u32> = HashMap::new();
            for b in &backends {
                *acc.entry(b.zone).or_default() += 1;
            }
            acc
        };
        let biggest_zone = *capacities.values().max().expect("empty capacities");
        let in_zone = capacities.get(&zone).copied().unwrap_or_default();
        let rho = (biggest_zone - in_zone) as f64 / backends.len() as f64;
        Self {
            zone,
            backends,
            rho,
            prng: SmallRng::seed_from_u64(42),
        }
    }
    fn sample(&mut self) -> u32 {
        let mut cur = 0;
        let mut total_weight = 0.0;
        for b in &self.backends {
            let weight = if b.zone == self.zone {
                1.0
            } else {
                1.0 - 1.0 / (1.0 + self.rho)
            };
            total_weight += weight;
            if self.prng.gen::<f64>() < weight / total_weight {
                cur = b.id;
            }
        }
        cur
    }
}

#[derive(Default, Clone)]
struct Backend {
    id: u32,
    zone: char,
}
