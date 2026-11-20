pub mod models {
    use crate::random::Rng;
    use std::collections::HashMap;
    type NodeId = u64;
    
    #[derive(Clone, Copy, Debug)]
    pub struct Config {
        pub diff_r: f64, // diffusion rate 
        pub alpha: f64, // self reinforce
        pub beta: f64,  // inhibition between competing votes 
        pub decay: f64,  // Decay of vote strength 
        pub accept_c: f64, // Conc to select decision
        pub drop_c: f64,   // Conc when decision dropped
        pub t_stale: u64,  // Neighbor and their effects are forgotten after this many steps/ticks
    }
    impl Config { pub fn new(diff_r: f64,alpha: f64,beta: f64,decay: f64,accept_c: f64,drop_c: f64,t_stale: u64) -> Self { Self {diff_r,alpha,beta,decay,accept_c,drop_c,t_stale} } }
    impl Default for Config { fn default() -> Self { Self {diff_r: 0.5,alpha: 0.3,beta: 0.4,decay: 0.05,accept_c: 0.85,drop_c: 0.5,t_stale: 5} } }

    pub struct Cell {
        pub id: u64,
        pub age: usize,
        pub max_age: usize,
        pub alive: bool,
        pub neighbors: Vec<usize>,
        pub mix: HashMap<NodeId, f64>, // Influence/Conc of a vote from NodeId node
        pub heard: HashMap<NodeId, HeardFromCell>, // Last heard data from neighbors
        pub ans: Option<NodeId>,
    }

    impl Cell {
        pub fn new(id: NodeId, proposal: NodeId, max_age: usize) -> Self {
            let mut mix = HashMap::new();
            mix.insert(proposal, 1.0);
            Self {id,age: 0,max_age,alive: true,neighbors: Vec::new(),mix,heard: HashMap::new(),ans: None}
        }
    }

    pub fn initial_cells(n: usize, rng: &mut Rng, enable_death: bool) -> Vec<Cell> { (0..n).map(|id| { Cell::new(id as u64, (rng.uniform() * (((n as f64).sqrt().ceil() as usize).max(2)) as f64) as u64, if enable_death { (rng.uniform() * 30.0) as usize + 5 } else { usize::MAX }) }).collect() }

    pub struct Network {
        pub cells: Vec<Cell>,
        pub rng: Rng,
        pub tick: u64,
        pub config: Config,
    }

    impl Network {
        pub fn init_ring(n: usize, seed: u64, enable_death: bool, config: Config) -> Self {
            assert!(n > 2);
            let mut ret = Network {cells: vec![],rng: Rng::seed(seed),tick: 0,config};
            let mut cells = initial_cells(n, &mut ret.rng, enable_death);
            for i in 0..n { cells[i].neighbors = vec![(i + n - 1) % n, (i + 1) % n]; }
            ret.cells.append(&mut cells);
            ret
        }

        pub fn init_mesh(n: usize, seed: u64, enable_death: bool, config: Config) -> Self {
            assert!(n > 2);
            let mut ret = Network {cells: vec![],rng: Rng::seed(seed),tick: 0,config};
            let mut cells = initial_cells(n, &mut ret.rng, enable_death);
            let rg = (0..n).collect::<Vec<_>>();
            for i in 0..n {
                cells[i].neighbors = if i == 0 { rg[1..].to_vec() } else if i == n - 1 { rg[..n - 1].to_vec() } else {
                    let mut k = rg[..i].to_vec();
                    k.append(&mut rg[i + 1..].to_vec());
                    k
                };
            }
            ret.cells.append(&mut cells);
            ret
        }

        pub fn init_snowflake(n: usize,depth: usize,seed: u64,enable_death: bool,config: Config) -> Self {
            assert!(n > 2);
            let mut ret = Network {cells: vec![],rng: Rng::seed(seed),tick: 0,config};
            let mut level_sizes = vec![1_usize];
            for _ in 0..depth {
                let next = level_sizes.last().unwrap().checked_mul(n).expect("snowflake network is too large");
                level_sizes.push(next);
            }
            let tot = level_sizes.iter().try_fold(0_usize, |sum, size| sum.checked_add(*size)).expect("snowflake network is too large");
            let mut cells = initial_cells(tot, &mut ret.rng, enable_death);
            let mut parent_start = 0;
            let mut child_start = 1;
            for parent_count in level_sizes.iter().take(depth) {
                for parent in parent_start..parent_start + parent_count {
                    let first_child = child_start + (parent - parent_start) * n;
                    for child in first_child..first_child + n {
                        cells[parent].neighbors.push(child);
                        cells[child].neighbors.push(parent);
                    }
                }
                parent_start += parent_count;
                child_start += parent_count * n;
            }
            ret.cells.append(&mut cells);
            ret
        }

        pub fn step(&mut self) {
            let conf = self.config;
            let tick = self.tick;
            let (mut mixes, mut ids, mut is_alive) = (Vec::new(), Vec::new(), Vec::new());
            self.cells.iter().for_each(|x| {
                mixes.push(x.mix.clone());
                ids.push(x.id);
                is_alive.push(x.alive)
            });
            for i in 0..self.cells.len() {
                if !is_alive[i] { continue; }
                let cell = &mut self.cells[i];
                for j in &cell.neighbors { if is_alive[*j] { cell.heard.insert(ids[*j],HeardFromCell {last_tick: tick,mix: mixes[*j].clone()}); } }
                // Forget neighbors if more then t_stale ticks happen
                cell.heard.retain(|_, h| tick - h.last_tick <= conf.t_stale);
                let new_mix = {
                    let cur: Vec<&HashMap<NodeId, f64>> = cell.heard.values().map(|h| &h.mix).collect();
                    let mut node_ids: Vec<NodeId> = cell.mix.keys().copied().collect();
                    for b in &cur { node_ids.extend(b.keys().copied()); }
                    node_ids.sort_unstable();
                    node_ids.dedup();
                    let tot: f64 = cell.mix.values().sum();
                    let mut out = HashMap::new();
                    for i in node_ids {
                        let c = cell.mix.get(&i).unwrap_or(&0.0);
                        let mean = if cur.is_empty() { *c} else { cur.iter().map(|b| b.get(&i).copied().unwrap_or(0.0)).sum::<f64>() / cur.len() as f64 };
                        let other = tot - c;
                        let v = ((1.0 - conf.diff_r) * c+ (conf.diff_r * mean)+ (conf.alpha * c * (1.0 - c))- (conf.beta * c * other)- (conf.decay * c)).clamp(0.0, 1.0);
                        if v > 0.001 { out.insert(i, v); }
                    }
                    out
                };
                cell.mix = new_mix;
                let best = cell.mix.iter().max_by(|a, b| a.1.total_cmp(b.1).then_with(|| b.0.cmp(a.0))).map(|(k, v)| (*k, *v));
                if let Some(ans) = cell.ans {
                    let cur_c = *cell.mix.get(&ans).unwrap_or(&0.0);
                    if cur_c < conf.drop_c {
                        cell.ans = match best {
                            Some((k, v)) if v > conf.accept_c => Some(k),
                            _ => None,
                        };
                    }
                } else if let Some((k, v)) = best {
                    if v > conf.accept_c { cell.ans = Some(k); }
                }
                cell.age += 1;
                if cell.age >= cell.max_age { cell.alive = false; }
            }
            self.tick += 1;
        }

        /// Returns the proposal shared by every living, decided cell.
        /// A fully failed network and a network with undecided cells have no consensus.
        pub fn consensus(&self) -> Option<NodeId> {
            let mut alive = self.cells.iter().filter(|cell| cell.alive);
            let proposal = alive.next()?.ans?;
            alive.all(|cell| cell.ans == Some(proposal)).then_some(proposal)
        }

        pub fn stats(&self) -> NetworkStats {
            let alive = self.cells.iter().filter(|cell| cell.alive).count();
            let mut proposals = HashMap::new();
            for cell in self.cells.iter().filter(|cell| cell.alive) {
                if let Some(proposal) = cell.ans { *proposals.entry(proposal).or_insert(0) += 1; }
            }
            let decided = proposals.values().sum();
            NetworkStats {total: self.cells.len(),alive,decided,proposals}
        }
    }

    #[derive(Clone, Debug)]
    // The most recent state heard from a cell.
    pub struct HeardFromCell {
        pub last_tick: u64,
        pub mix: HashMap<NodeId, f64>,
    }

    #[derive(Debug)]
    pub struct NetworkStats {
        pub total: usize,
        pub alive: usize,
        pub decided: usize,
        pub proposals: HashMap<NodeId, usize>,
    }
}
