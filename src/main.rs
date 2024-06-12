use std::{collections::HashMap, marker::PhantomData};

use clap::Parser;
use rand::{rngs::SmallRng, Rng, SeedableRng};

fn main() {
    let args = Args::parse();

    let backends: Vec<Backend> = {
        let mut acc = Vec::new();
        for _ in 0 .. 1 {
            acc.push(Backend { id: acc.len() as u32, zone: 'a' });
        }
        for _ in 0 .. 5 {
            acc.push(Backend { id: acc.len() as u32, zone: 'b' });
        }
        for _ in 0 .. 9 {
            acc.push(Backend { id: acc.len() as u32, zone: 'c' });
        }
        acc
    };
    let clients: Vec<Client> = {
        let mut acc = Vec::new();
        for _ in 0 .. 3 {
            acc.push(Client::new('a', backends.clone()));
        }
        for _ in 0 .. 3 {
            acc.push(Client::new('b', backends.clone()));
        }
        for _ in 0 .. 3 {
            acc.push(Client::new('c', backends.clone()));
        }
        acc
    };

    let mut tally = vec![0; backends.len()];
    for mut client in clients {
        for _ in 0 .. args.iterations {
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
    affinity: f64,
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
        let highest = *capacities.values().max().expect("empty capacities");
        let chi = capacities.get(&zone).copied().unwrap_or_default() as f64 / highest as f64;
        eprintln!("{zone} --> {chi} [{capacities:?}]");
        Self {
            zone,
            backends,
            affinity: chi,
            prng: SmallRng::seed_from_u64(42),
        }
    }
    fn sample(&mut self) -> u32 {
        let mut cur = 0;
        let mut total_weight = 0.0;
        for b in &self.backends {
            let mut weight = 1.0;
            if b.zone != self.zone {
                weight *= 1.0 - self.affinity;
            }
            eprintln!("[{client_az} --> {backend_az}] {weight}", client_az = self.zone, backend_az = b.zone, weight = weight);
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
