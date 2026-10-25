use glam::Vec4;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct Spectrum {
    pub bands: [Vec4; 16],
}

impl Spectrum {
    #[inline(always)]
    pub fn band(&self, i: u32) -> f32 {
        let v = self.bands[(i / 4).min(15) as usize];
        let k = i % 4;
        if k == 0 {
            v.x
        } else if k == 1 {
            v.y
        } else if k == 2 {
            v.z
        } else {
            v.w
        }
    }

    #[inline(always)]
    pub fn at(&self, x: f32) -> f32 {
        let f = x.clamp(0.0, 1.0) * 63.0;
        let i = f as u32;
        let a = self.band(i);
        let b = self.band((i + 1).min(63));
        a + (b - a) * (f - i as f32)
    }

    #[inline(always)]
    pub fn bass(&self) -> f32 {
        let mut s = 0.0;
        let mut i = 0;

        while i < 8 {
            s += self.band(i);
            i += 1;
        }
        s / 8.0
    }

    #[inline(always)]
    pub fn level(&self) -> f32 {
        let mut s = 0.0;
        let mut i = 0;
        while i < 16 {
            let v = self.bands[i];
            s += v.x + v.y + v.z + v.w;
            i += 1;
        }

        s / 64.0
    }
}
