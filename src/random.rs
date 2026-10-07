pub mod Random{
// Xoshiro256** PRNG

    pub struct Rng {
        s: [u64; 4],
    }

    impl Rng {
        pub fn seed(seed: u64) -> Self {
            let mut z = seed;
            let mut next = || {
                z = z.wrapping_add(0x9E3779B97F4A7C15);
                let mut x = z;
                x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
                x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
                x ^ (x >> 31)
            };
            Rng { s: [next(), next(), next(), next()] }
        }


        pub fn next_u64(&mut self) -> u64 {
            let r = self.s[0].wrapping_add(self.s[3]).rotate_left(23).wrapping_add(self.s[0]);
            let t = self.s[1] << 17;
            self.s[2] ^= self.s[0];
            self.s[3] ^= self.s[1];
            self.s[1] ^= self.s[2];
            self.s[0] ^= self.s[3];
            self.s[2] ^= t;
            self.s[3] = self.s[3].rotate_left(45);
            r
        }

        pub fn uniform(&mut self) -> f64 {
            (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
        }


    }
}