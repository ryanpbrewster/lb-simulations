#![allow(dead_code)]

use std::collections::BTreeMap;

use rand::{rngs::SmallRng, Rng, SeedableRng};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct BackendId(u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Zone(u8);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Subset(u8);

#[derive(Clone, Debug)]
struct Backend {
    id: BackendId,
    zone: Zone,
    subset: Subset,
    capacity: f64,
}

#[derive(Clone)]
struct Client {
    zone: Zone,
    // How this client should modify the backend weights in any given zone.
    zonal_multiplier: BTreeMap<Zone, f64>,
    backends: Vec<Backend>,
    prng: SmallRng,
}
impl Client {
    fn new(zone: Zone, backends: Vec<Backend>) -> Self {
        let mut total_capacity = 0.0;
        let per_zone_capacity = {
            let mut acc: BTreeMap<Zone, f64> = BTreeMap::new();
            for b in &backends {
                total_capacity += b.capacity;
                *acc.entry(b.zone).or_default() += b.capacity;
            }
            acc
        };
        let num_zones = per_zone_capacity.len() as f64;
        let avg_capacity = total_capacity / num_zones;
        let my_zone_capacity = per_zone_capacity.get(&zone).copied().unwrap_or_default();
        let surplus_capacity: f64 = per_zone_capacity
            .values()
            .copied()
            .map(|cap| {
                if cap > avg_capacity {
                    cap - avg_capacity
                } else {
                    0.0
                }
            })
            .sum();
        let zone_weights = if my_zone_capacity >= avg_capacity {
            // If we are from an over-capacity zone, stay entirely in-zone.
            [(zone, 1.0)].into_iter().collect()
        } else {
            // If we are from an under-capacity zone, we can't send _all_
            // traffic in-zone or we'll overload our backends.  So we need to
            // send some traffic in-zone and some cross-zone.
            let in_zone = my_zone_capacity / avg_capacity;
            let cross_zone = 1.0 - in_zone;
            per_zone_capacity
                .into_iter()
                .map(|(z, zone_cap)| {
                    let zone_weight = if z == zone {
                        in_zone
                    } else if zone_cap <= avg_capacity {
                        // If the target zone is under-capacity, don't send any traffic.
                        0.0
                    } else {
                        // Send cross-zone traffic proportional to how much of the surplus capacity
                        // is present in that zone.
                        cross_zone * (zone_cap - avg_capacity) / surplus_capacity
                    };
                    (z, zone_weight / zone_cap)
                })
                .collect()
        };
        Self {
            zone,
            zonal_multiplier: zone_weights,
            backends,
            prng: SmallRng::seed_from_u64(42),
        }
    }
    fn sample(&mut self, p: fn(&Backend) -> bool) -> Option<BackendId> {
        let mut cur: Option<BackendId> = None;
        let mut total_weight = 0.0;
        for b in &self.backends {
            if !p(b) {
                continue;
            }
            let Some(&lambda) = self.zonal_multiplier.get(&b.zone) else {
                continue;
            };
            let weight = lambda * b.capacity;
            total_weight += weight;
            if self.prng.gen::<f64>() < weight / total_weight {
                cur = Some(b.id);
            }
        }
        cur
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn naive() {
        /*
        Sample output from
        [a] 0.99992
        [b] 1.00138
        [b] 0.99883
        [b] 1.00172
        [b] 0.99911
        [b] 0.99896
        [c] 1.00092
        [c] 0.99619
        [c] 1.00335
        [c] 1.00154
        [c] 0.99987
        [c] 1.00022
        [c] 0.99613
        [c] 1.00119
        [c] 1.00066
        % in-zone = 0.7333283333333334
        */

        let iterations = 100_000;
        let backends: BTreeMap<BackendId, Backend> = [(b'a', 1), (b'b', 5), (b'c', 9)]
            .into_iter()
            .flat_map(|(zone, count)| std::iter::repeat(Zone(zone)).take(count))
            .enumerate()
            .map(|(idx, zone)| {
                let id = BackendId(idx as u32);
                (
                    id,
                    Backend {
                        id,
                        zone,
                        capacity: 1.0,
                        subset: Subset(0),
                    },
                )
            })
            .collect();

        // If there were a Zone D without any backends, clients in zones A..C won't even
        // know it exists. That screws up their calculations and the overall
        // distribution is skewed slightly. Uncomment this to see the skewed output.
        let mut clients: Vec<Client> = [b'a', b'b', b'c']
            .into_iter()
            .map(|zone| {
                let zone = Zone(zone);
                Client::new(zone, backends.values().cloned().collect())
            })
            .collect();

        let mut tally: BTreeMap<BackendId, u32> = BTreeMap::new();
        let mut in_zone = 0;
        let mut total = 0;
        for client in &mut clients {
            for _ in 0..iterations {
                let b = client.sample(|_| true).unwrap();
                *tally.entry(b).or_default() += 1;
                if backends[&b].zone == client.zone {
                    in_zone += 1;
                }
                total += 1;
            }
        }

        let avg = total as f64 / backends.len() as f64;
        let min_load = tally.values().min().copied().unwrap() as f64 / avg;
        let max_load = tally.values().max().copied().unwrap() as f64 / avg;

        assert!(0.95 <= min_load, "min load = {min_load}");
        assert!(max_load <= 1.05, "max load = {max_load}");

        let in_zone_frac = in_zone as f64 / total as f64;
        assert!(in_zone_frac > 0.70, "in_zone = {in_zone_frac}");
    }
}
