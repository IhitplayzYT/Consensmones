pub mod helper {
    #[derive(Debug, Clone)]
    pub struct Cli {
        pub nodes: usize,
        pub ticks: u64,
        pub seed: u64,
        pub topo: String,
        pub depth: usize,
        pub diff: f64,
        pub reinf: f64,
        pub inhibit: f64,
        pub decay: f64,
        pub accept: f64,
        pub drop: f64,
        pub stale: u64,
        pub death: bool,
        pub debug: bool,
        pub help: bool,
    }

    impl Default for Cli { fn default() -> Self { Self {nodes: 20,ticks: 100,seed: 1,topo: "ring".to_owned(),depth: 2,diff: 0.5,reinf: 0.3,inhibit: 0.4,decay: 0.05,accept: 0.85,drop: 0.5,stale: 5,death: false,debug: false,help: false} } }

    impl Cli {
        pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
            let mut cli = Self::default();
            let mut args = args.into_iter();
            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "-h" | "--help" => cli.help = true,
                    "-d" | "--debug" => cli.debug = true,
                    "--death" => cli.death = true,
                    "--nodes" => {
                        cli.nodes = parse_value("--nodes", args.next())?;
                        if cli.nodes < 3 { return Err("--nodes must be at least 3".to_owned()); }
                    }
                    "--ticks" => cli.ticks = parse_value("--ticks", args.next())?,
                    "--seed" => cli.seed = parse_value("--seed", args.next())?,
                    "-diff"| "--diffusion" => cli.diff = parse_value("--diffusion", args.next())?,
                    "-reinforce"| "--reinforcement" =>  cli.reinf = parse_value("--reinforcement", args.next())? ,
                    "--inhibition" => cli.inhibit = parse_value("--inhibition", args.next())?,
                    "--decay" => cli.decay = parse_value("--decay", args.next())?,
                    "--accept" => cli.accept = parse_value("--accept", args.next())?,
                    "--drop" => cli.drop = parse_value("--drop", args.next())?,
                    "--stale" => cli.stale = parse_value("--stale", args.next())?,
                    "--depth" => cli.depth = parse_value("--depth", args.next())?,
                    "-topo" | "--topology" => {
                        cli.topo = args.next().ok_or("--topology requires a value")?;
                        if !["ring", "mesh", "snowflake"].contains(&cli.topo.as_str()) { return Err("--topology must be 'ring', 'mesh', or 'snowflake'".to_owned()); }
                    }
                    _ => return Err(format!("unknown argument: {arg}")),
                }
            }
            for (name, value) in [("--diffusion", cli.diff),("--accept", cli.accept),("--drop", cli.drop)] { if !(0.0..=1.0).contains(&value) { return Err(format!("{name} must be between 0 and 1")); } }
            for (name, value) in [("--reinforcement", cli.reinf),("--inhibition", cli.inhibit),("--decay", cli.decay)] { if !value.is_finite() || value < 0.0 { return Err(format!("{name} must be a finite non-negative number")); } }
            if cli.drop >= cli.accept { return Err("--drop must be lower than --accept".to_owned()); }
            Ok(cli)
        }

        pub fn usage() -> &'static str {
            "Usage: Consensmones [OPTIONS]\n\n\
             Simulate consensus through diffusion of proposal concentrations.\n\n\
             Options:\n\
             --nodes <N>       Number of cells (default: 20; minimum: 3)\n\
             --ticks <N>       Maximum simulation steps (default: 100)\n\
             --seed <N>        Random seed (default: 1)\n\
             --topology <T>    Network: ring, mesh, or snowflake (default: ring)\n\
             --depth <N>       Snowflake tree depth (default: 2)\n\
             --diffusion <X>   Neighbor diffusion rate in [0, 1] (default: 0.5)\n\
             --reinforcement <X> Positive feedback (default: 0.3)\n\
             --inhibition <X>  Competition suppression (default: 0.4)\n\
             --decay <X>       Signal decay rate (default: 0.05)\n\
             --accept <X>      Decision threshold in [0, 1] (default: 0.85)\n\
             --drop <X>        Withdrawal threshold in [0, accept) (default: 0.5)\n\
             --stale <N>       Ticks before cached neighbor data expires (default: 5)\n\
             --death           Enable cell aging and failure\n\
             -d, --debug       Print parsed configuration\n\
             -h, --help        Show this help"
        }
    }

    fn parse_value<T: std::str::FromStr>(name: &str, value: Option<String>) -> Result<T, String> {
        let value = value.ok_or_else(|| format!("{name} requires a value"))?;
        value.parse().map_err(|_| format!("invalid value for {name}: {value}"))
    }
}
