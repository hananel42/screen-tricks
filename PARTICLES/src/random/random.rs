extern "system" {
    fn GetTickCount() -> u32;
}
pub struct Random {
    seed: u32,
}
const REC_U32_MAX: f32 = 1.0 / u32::MAX as f32; // יחושב בזמן קומפילציה
fn hash_u32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    x
}
impl Random {
    pub fn new() -> Random {
        Random { seed: unsafe { GetTickCount() } }
    }

    #[inline(always)]
    pub fn positive_jitter(&mut self, amount: f32) -> f32 {
        self.seed = hash_u32(self.seed);
        (self.seed as f32) * REC_U32_MAX * amount
    }

    #[inline(always)]
    pub fn jitter(&mut self, amount: f32) -> f32 {
        (self.positive_jitter(2.0) - 1.0) * amount
    }

    #[inline(always)]
    pub fn range(&mut self, min: f32, max: f32) -> f32 {
        self.positive_jitter(max - min) + min
    }
    pub fn integer(&mut self,max:u32) -> u32 {
       self.seed = hash_u32(self.seed);
        self.seed % (max+1) 
    }
    #[inline(always)]
    pub fn choose<'a, F>(&mut self, f: &'a [F]) -> &'a F {
        self.seed = hash_u32(self.seed);
        &f[(self.seed as usize) % f.len()]
    }

}

