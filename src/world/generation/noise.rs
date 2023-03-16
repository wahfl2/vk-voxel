use noise::{SuperSimplex, NoiseFn};
use ultraviolet::{Vec3, Vec2};

pub struct ScaleNoise3D {
    scale: Vec3,
    noise: SuperSimplex,
}

impl ScaleNoise3D {
    pub fn new(scale: Vec3, seed: u32) -> Self {
        Self {
            scale,
            noise: SuperSimplex::new(seed)
        }
    }

    pub fn get(&self, pos: Vec3) -> f64 {
        let scaled = pos * self.scale;
        self.noise.get([scaled.x as f64, scaled.y as f64, scaled.z as f64])
    }
}

pub struct ScaleNoise2D {
    scale: Vec2,
    noise: SuperSimplex,
}

impl ScaleNoise2D {
    pub fn new(scale: Vec2, seed: u32) -> Self {
        Self {
            scale,
            noise: SuperSimplex::new(seed)
        }
    }

    pub fn get(&self, pos: Vec2) -> f64 {
        let scaled = pos * self.scale;
        self.noise.get([scaled.x as f64, scaled.y as f64]) * 0.5 + 0.5
    }

    pub fn sample(&self, pos: Vec2, octaves: u32) -> f64 {
        let mut ret = 0.0;
        let persistence = 0.5;
        let mut amp = 1.0;
        let mut max_amp = 0.0;
        let mut scaled = pos;

        for _ in 0..octaves {
            ret += self.get(scaled) * amp;
            max_amp += amp;
            amp *= persistence;
            scaled *= 2.0;
        }

        ret / max_amp
    }
}

