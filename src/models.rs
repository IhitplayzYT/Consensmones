pub mod Models{
    use std::{collections::HashMap, sync::{LazyLock, Mutex, RwLock}};

use crate::random::Random::Rng;


    type NodeId = u64;
    static RNG: LazyLock<RwLock<Rng>> = LazyLock::new(|| RwLock::new(Rng::seed(0)));
    static MAP: LazyLock<RwLock<HashMap<NodeId,Node>>> = LazyLock::new(|| RwLock::new(HashMap::new()));

    pub struct Node{
        pub id: u64,
        pub age:usize,
        pub max_age: usize,
        pub neighbors: Vec<NodeId>,
        pub signal: SignalParams,
        pub locality: Locality,
    }

    impl Node{
        pub fn new(max_age: Option<usize>) -> Self{
            let mut handle = RNG.write().unwrap();
            let id = handle.next_u64();
            Self { id , age: 0, max_age: max_age.unwrap_or((handle.uniform() * 30.0) as usize),signal:SignalParams::new() ,neighbors: vec![], locality:  Locality { candidate: id, signal_map: HashMap::new(),local_consensus:SignalParams::new()}}
        }

        pub fn diffuse(&mut self) -> SignalParams{
            // TODO:
            SignalParams::new()
        }
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