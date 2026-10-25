use glam::UVec2;

const GLYPH_W: u32 = 3;
const GLYPH_H: u32 = 5;
const ADVANCE: u32 = GLYPH_W + 1;

const FONT: [u32; 44] = [
    0b111_101_101_101_111, // 0
    0b010_110_010_010_111, // 1
    0b111_001_111_100_111, // 2
    0b111_001_111_001_111, // 3
    0b101_101_111_001_001, // 4
    0b111_100_111_001_111, // 5
    0b111_100_111_101_111, // 6
    0b111_001_001_001_001, // 7
    0b111_101_111_101_111, // 8
    0b111_101_111_001_111, // 9
    0b010_101_111_101_101, // A
    0b110_101_110_101_110, // B
    0b011_100_100_100_011, // C
    0b110_101_101_101_110, // D
    0b111_100_110_100_111, // E
    0b111_100_110_100_100, // F
    0b011_100_101_101_011, // G
    0b101_101_111_101_101, // H
    0b111_010_010_010_111, // I
    0b001_001_001_101_010, // J
    0b101_101_110_101_101, // K
    0b100_100_100_100_111, // L
    0b101_111_111_101_101, // M
    0b110_101_101_101_101, // N
    0b010_101_101_101_010, // O
    0b110_101_110_100_100, // P
    0b010_101_101_110_011, // Q
    0b110_101_110_101_101, // R
    0b011_100_010_001_110, // S
    0b111_010_010_010_010, // T
    0b101_101_101_101_111, // U
    0b101_101_101_101_010, // V
    0b101_101_111_111_101, // W
    0b101_101_010_101_101, // X
    0b101_101_010_010_010, // Y
    0b111_001_010_100_111, // Z
    0b000_000_000_000_000, // Space
    0b000_000_000_000_010, // .
    0b000_010_000_010_000, // :
    0b000_000_111_000_000, // -
    0b010_010_010_000_010, // !
    0b001_001_010_100_100, // /
    0b111_001_010_000_010, // ?
    0b000_010_111_010_000, // +
];

fn glyph_index(c: u32) -> u32 {
    let c = if c >= 97 && c <= 122 { c - 32 } else { c };
    if c >= 48 && c <= 57 {
        c - 48
    } else if c >= 65 && c <= 90 {
        c - 55
    } else if c == 32 {
        36
    } else if c == 46 {
        37
    } else if c == 58 {
        38
    } else if c == 45 {
        39
    } else if c == 33 {
        40
    } else if c == 47 {
        41
    } else if c == 43 {
        43
    } else {
        42
    }
}

fn glyph_bit(index: u32, gx: u32, gy: u32) -> f32 {
    let bit = 14 - (gy * GLYPH_W + gx);
    ((FONT[index as usize] >> bit) & 1) as f32
}

pub const fn codes<const N: usize>(s: &str) -> [u32; N] {
    let b = s.as_bytes();
    let mut out = [32u32; N];
    let mut i = 0;
    while i < N && i < b.len() {
        out[i] = b[i] as u32;
        i += 1;
    }
    out
}

fn locate(px: UVec2, origin: UVec2, scale: u32) -> Option<(u32, u32, u32)> {
    if px.x < origin.x || px.y < origin.y {
        return None;
    }
    let l = px - origin;
    if l.y >= GLYPH_H * scale {
        return None;
    }
    let cell = ADVANCE * scale;
    let ci = l.x / cell;
    let gx = (l.x % cell) / scale;
    if gx >= GLYPH_W {
        return None;
    }
    Some((ci, gx, l.y / scale))
}

pub fn text_mask<const N: usize>(px: UVec2, origin: UVec2, scale: u32, text: &[u32; N]) -> f32 {
    match locate(px, origin, scale) {
        Some((ci, gx, gy)) => {
            if ci >= N as u32 {
                0.0
            } else {
                glyph_bit(glyph_index(text[ci as usize]), gx, gy)
            }
        }
        None => 0.0,
    }
}

pub fn number_mask(px: UVec2, origin: UVec2, scale: u32, value: u32) -> f32 {
    match locate(px, origin, scale) {
        Some((ci, gx, gy)) => {
            let mut count = 1;
            let mut v = value;
            while v >= 10 {
                v /= 10;
                count += 1;
            }
            if ci >= count {
                return 0.0;
            }
            let mut pos = count - 1 - ci;
            let mut n = value;
            while pos > 0 {
                n /= 10;
                pos -= 1;
            }
            glyph_bit(n % 10, gx, gy)
        }
        None => 0.0,
    }
}

pub fn fixed_mask(px: UVec2, origin: UVec2, scale: u32, value: f32, decimals: u32) -> f32 {
    match locate(px, origin, scale) {
        Some((ci, gx, gy)) => {
            let mut p = 1u32;
            let mut d = 0;
            while d < decimals {
                p *= 10;
                d += 1;
            }
            let scaled = (value.max(0.0) * p as f32 + 0.5) as u32;
            let int_part = scaled / p;
            let frac = scaled % p;

            let mut count = 1;
            let mut v = int_part;
            while v >= 10 {
                v /= 10;
                count += 1;
            }

            if ci >= count + 1 + decimals {
                return 0.0;
            }
            if ci < count {
                let mut pos = count - 1 - ci;
                let mut n = int_part;
                while pos > 0 {
                    n /= 10;
                    pos -= 1;
                }
                glyph_bit(n % 10, gx, gy)
            } else if ci == count {
                glyph_bit(37, gx, gy) // '.'
            } else {
                let mut pos = decimals - 1 - (ci - count - 1);
                let mut n = frac;
                while pos > 0 {
                    n /= 10;
                    pos -= 1;
                }
                glyph_bit(n % 10, gx, gy)
            }
        }
        None => 0.0,
    }
}
