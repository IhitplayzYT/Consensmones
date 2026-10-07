pub mod Models{
    use std::{collections::HashMap, sync::{LazyLock, RwLock}};
    use crate::random::Random::Rng;

    type NodeId = u64;

    #[derive(Clone, Copy, Debug)]
    pub struct Config{
        pub diff_r: f64,
        pub alpha: f64, // self-reinforcement
        pub beta: f64, // inhibition between competing propositions
        pub decay: f64, 
        pub accept_c: f64, // Conc to select decision
        pub drop_c: f64, // Conc when decision dropped
        pub t_stale: u64, // Neighbor and their effects are forgotten after this many steps/ticks
    }

    impl Config{
        pub fn new(diff_r:f64,alpha:f64,beta:f64,decay:f64,accept_c:f64,drop_c:f64,t_stale:u64) -> Self{
            Self { diff_r, alpha, beta, decay, accept_c, drop_c, t_stale}    
        }
    }


    impl Default for Config{
        fn default() -> Self {
            Self { diff_r: 0.5, alpha: 0.3, beta: 0.4, decay: 0.05, accept_c: 0.85, drop_c: 0.5, t_stale: 5 }
        }
    }

    pub struct Cell{
        pub id: u64,
        pub age:usize,
        pub max_age: usize,
        pub alive: bool,
        pub neighbors: Vec<usize>,
        pub mix: HashMap<NodeId,f64>,  // Influence/Conc of a vote from NodeId node
        pub heard: HashMap<NodeId,Heard_from_Cell>, // Last heard data from neighbors
        pub ans: Option<NodeId>
    }

    impl Cell{
        pub fn new(id:NodeId,max_age:usize) -> Self{
            let mut mix = HashMap::new();
            mix.insert(id, 1.0); 
            Self { id, age: 0, max_age, alive: true, neighbors: Vec::new(), mix, heard: HashMap::new(), ans: None}
        }
    }


    pub struct Network{
        pub cells: Vec<Cell>,
        pub rng: Rng,
        pub tick: u64,
        pub config: Config
    }


    impl Network{

        pub fn init_ring(n:usize,seed:u64,enable_death: bool,config: Config) -> Self{
            assert!(n > 2);
            let mut ret = Network{cells:vec![],rng:Rng::seed(seed),tick:0,config};
            let mut cells:Vec<Cell> = (0..n).map(|_| {
                let max_age = if enable_death { (ret.rng.uniform() * 30.0) as usize + 5 } else { usize::MAX };
                Cell::new(ret.rng.next_u64(), max_age)
            }).collect();
            for i in 0..n {
                cells[i].neighbors = vec![(i + n - 1) % n, (i + 1) % n];
            }
            ret.cells.append(&mut cells);
            ret
        }

        pub fn init_mesh(n:usize,seed:u64,enable_death: bool,config: Config) -> Self{
            assert!(n > 2);
            let mut ret = Network{cells:vec![],rng:Rng::seed(seed),tick:0,config};
            let mut cells:Vec<Cell> = (0..n).map(|_| {
                let max_age = if enable_death { (ret.rng.uniform() * 30.0) as usize + 5 } else { usize::MAX };
                Cell::new(ret.rng.next_u64(), max_age)
            }).collect();
            let rg = (0..n).collect::<Vec<_>>();
            for i in 0..n {
                cells[i].neighbors = if i == 0 {rg[1..].to_vec()}else if i == n-1 {rg[..n-1].to_vec()} else{let mut k = rg[..i].to_vec();k.append(&mut rg[i+1..].to_vec());k};
            }
            ret.cells.append(&mut cells);   
            ret
        }


        pub fn init_snowflake(n:usize,depth:usize,seed:u64,enable_death: bool,config: Config) -> Self{
            assert!(n > 2);
            let mut ret = Network{cells:vec![],rng:Rng::seed(seed),tick:0,config};
            let tot = n * (1 - n.pow(depth as u32))/(1-n); // Becomes sum of GM ie=i.e n + n^2 + n^3
            
            let mut cells:Vec<Cell> = (0..tot).map(|_| {
                let max_age = if enable_death { (ret.rng.uniform() * 30.0) as usize + 5 } else { usize::MAX };
                Cell::new(ret.rng.next_u64(), max_age)
            }).collect();
            let mut st = 0_usize;
            let mut c = 0;
            for i in 0..tot {
                let mut k:Vec<usize> = (st..st+n).collect();
                k.remove(c);
                c += 1;
                cells[i].neighbors = k;
                if c == n{
                    c = 0;
                    st += n
                }
            }
            ret.cells.append(&mut cells);   
            ret           
        }

        pub fn kill(&mut self,idx:usize) {
            self.cells[idx].alive = false;
        }

        pub fn step(&mut self){
            let conf = self.config;
            let tick = self.tick;
           let (mut mixes,mut ids,mut is_alive) = (Vec::new(),Vec::new(),Vec::new());
            self.cells.iter().for_each(|x| {
                mixes.push(x.mix.clone());
                ids.push(x.id);
                is_alive.push(x.alive)
            });
            for i in 0..self.cells.len(){
                if !is_alive[i] {continue;}
                let cell = &mut self.cells[i];
                for j in &cell.neighbors{
                    if is_alive[*j]{ cell.heard.insert(ids[*j], Heard_from_Cell { last_tick: tick, mix: mixes[*j].clone()}); }   
                }

                // Forget neighbors if more then t_stale ticks happen
                cell.heard.retain(|_,h| { tick - h.last_tick <= conf.t_stale});

                let new_mix = {
                    let cur: Vec<&HashMap<NodeId, f64>> = cell.heard.values().map(|h| &h.mix).collect();
                    let mut node_ids: Vec<NodeId> = cell.mix.keys().copied().collect();
                    for b in &cur { 
                        node_ids.extend(b.keys().copied()); 
                    }
                    // Keep sorted list of unique node ids
                    node_ids.sort_unstable();
                    node_ids.dedup(); 

                    let tot:f64 = cell.mix.values().sum();
                    let mut out = HashMap::new();
                    for i in node_ids{
                        let c = cell.mix.get(&i).unwrap_or(&0.0);
                        let mean = if cur.is_empty() { *c } else { cur.iter().map(|b| b.get(&i).copied().unwrap_or(0.0)).sum::<f64>() / cur.len() as f64 };
                        let other = tot - c;
                        let v = ((1.0 -conf.diff_r)* c + (conf.diff_r * mean) + (conf.alpha * c * (1.0 - c)) - (conf.beta * c * other) - (conf.decay * c)).clamp(0.0, 1.0);
                        if v > 0.001 {
                            out.insert(i, v);
                        }
                    }
                    out
                };

                cell.mix = new_mix;

                let best = cell.mix.iter().max_by(|a,b| a.1.partial_cmp(b.1).unwrap()).map(|(k,v)| (*k,*v));
                if let Some(ans) = cell.ans{
                    let cur_c = *cell.mix.get(&ans).unwrap_or(&0.0);
                    if cur_c < conf.drop_c{
                        cell.ans = match best{
                            Some((k,v)) if v > conf.accept_c => Some(k),
                            _ => None
                        };

                    }
                } else if let Some((k,v)) = best{
                    if v > conf.accept_c { cell.ans = Some(k); }
                }

                cell.age += 1;
                if cell.age >= cell.max_age{ cell.alive = false;}
            }
            self.tick += 1;
        }

    }

    // The last state we heard from a cell
    pub struct Heard_from_Cell{
        pub last_tick:u64,
        pub mix: HashMap<NodeId,f64>
    }

    pub struct Locality{
        pub candidate: NodeId,
        pub signal_map: HashMap<NodeId,SignalParams>,
        pub local_consensus: SignalParams
    }

    impl Locality{
        pub fn new(neig: &Vec<NodeId>) -> Self{
            assert!(neig.len() != 0);
            let signals = neig.iter().map(|x| {MAP.read().unwrap().get(x).expect("Node not found in MAP").signal.clone()}).collect::<Vec<SignalParams>>();
            let candidate = neig[0];
            let signal_map  = neig.into_iter().map(|x| *x).zip(signals.into_iter()).collect();
            Self { candidate, signal_map , local_consensus: SignalParams::new() }
        }

        pub fn from(id: NodeId) -> Self{
            let mut signal_map = HashMap::new();
            let sig = MAP.read().unwrap().get(&id).expect("Node not found in MAP").signal.clone();
            signal_map.insert(id, sig.clone());
            Self { candidate: id, signal_map, local_consensus: sig}
        }

        pub fn add(&mut self, id: NodeId){
            let sig = MAP.read().unwrap().get(&id).expect("Node not found in MAP").signal.clone();
            self.signal_map.insert(id, sig);
        }

        pub fn step(&mut self){
            self.signal_map.iter_mut().for_each(|(id,sig)| {
                let nsig = MAP.write().unwrap().get_mut(id).expect("Node doesn't exist in MAP").diffuse();
                *sig = nsig;
            });

            // TODO: Update consensus
        }

    }


    #[derive(Debug,Clone)]
    pub struct SignalParams{
        pub proposal: f64,
        pub agreement: f64,
        pub conflict: f64,
        pub freshness: f64,
        pub failure: f64,
    }

    impl SignalParams{
        pub fn new() -> Self{
            Self { proposal: 1.0, agreement: 0.0, conflict: 0.0, freshness: 1.0, failure: 0.0 }
        }
    }


    pub struct Consensus{
        pub localities: Vec<Locality>,
        pub cohesion_coupling: Vec<Vec<SignalParams>>
    }




}