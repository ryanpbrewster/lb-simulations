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
    let mut clients: Vec<Client> = {
        let mut acc = Vec::new();
        acc.push(Client::new('a', backends.clone()));
        acc.push(Client::new('b', backends.clone()));
        acc.push(Client::new('c', backends.clone()));
        // If there were a Zone D without any backends, clients in zones A..C won't even
        // know it exists. That screws up their calculations and the overall
        // distribution is skewed slightly. Uncomment this to see the skewed output.
        // acc.push(Client::new('d', backends.clone()));
        acc
    };

    let mut tally = vec![0; backends.len()];
    let mut in_zone = 0;
    let mut total = 0;
    for client in &mut clients {
        for _ in 0..args.iterations {
            let b = client.sample() as usize;
            tally[b] += 1;
            if backends[b].zone == client.zone {
                in_zone += 1;
            }
            total += 1;
        }
    }

    for (backend, count) in backends.iter().zip(tally) {
        println!(
            "[{zone}] {frac:.05}",
            zone = backend.zone,
            frac = count as f64 / (total / backends.len()) as f64
        );
    }
    println!(
        "% in-zone = {fraction}",
        fraction = in_zone as f64 / total as f64
    );
}

#[derive(Parser)]
struct Args {
    #[arg(long, default_value_t = 1_000)]
    iterations: u64,
}

#[derive(Clone)]
struct Client {
    zone: char,
    backends: Vec<(f64, Backend)>,
    prng: SmallRng,
}
impl Client {
    fn new(zone: char, backends: Vec<Backend>) -> Self {
        let capacities = {
            let mut acc: HashMap<char, f64> = HashMap::new();
            for b in &backends {
                *acc.entry(b.zone).or_default() += 1.0;
            }
            acc
        };
        let avg = backends.len() as f64 / capacities.len() as f64;
        let bz = capacities.get(&zone).copied().unwrap_or_default();
        let total_surplus: f64 = capacities
            .values()
            .copied()
            .map(|cap| if cap > avg { cap - avg } else { 0.0 })
            .sum();
        let backends = backends
            .into_iter()
            .map(|b| {
                let zone_cap = capacities[&b.zone];
                let zone_weight = if b.zone == zone {
                    if bz >= avg {
                        1.0
                    } else {
                        bz / avg
                    }
                } else if bz >= avg || zone_cap <= avg {
                    0.0
                } else {
                    (1.0 - bz / avg) * (zone_cap - avg) / total_surplus
                };

                (zone_weight / zone_cap, b)
            })
            .collect();
        Self {
            zone,
            backends,
            prng: SmallRng::seed_from_u64(42),
        }
    }
    fn sample(&mut self) -> u32 {
        let mut cur = 0;
        let mut total_weight = 0.0;
        for (weight, b) in &self.backends {
            total_weight += weight;
            if self.prng.gen::<f64>() < weight / total_weight {
                cur = b.id;
            }
        }
        cur
    }
}

#[derive(Default, Clone, Debug)]
struct Backend {
    id: u32,
    zone: char,
}
