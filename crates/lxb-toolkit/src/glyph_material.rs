pub const SHAPE_MARK: &str = "lxb:shape";

pub const SDF_RANGE: f32 = 0.125;

pub const SDF_SUPERSAMPLE: u32 = 4;

pub const CELL: u32 = 128;

pub const DEPTH_SHARE: f32 = 0.075;

pub const LAMP: [f32; 3] = [-0.4915, -0.7078, 0.5069];

pub const SHADOW: f32 = 0.30;

pub const SIMPLE_TINT: f32 = 0.22;
pub const SIMPLE_ALPHA: f32 = 0.88;
pub const SIMPLE_STAIN: f32 = 0.15;

pub const GRADIENT_ARM: f32 = 1.5;

pub const COVERAGE_FEATHER: f32 = 0.75;

pub const MIN_DEPTH: f32 = 0.5;

pub const RIDGE_BEGIN: f32 = 0.20;
pub const RIDGE_END: f32 = 0.80;

pub const SHADOW_OFFSET: f32 = 0.22;

pub fn decode_sdf(stored: f32) -> f32 {
    (stored - 0.5) * 2.0 * SDF_RANGE
}

pub fn depth_for(size: f32) -> f32 {
    size * DEPTH_SHARE
}

pub fn is_shape(source: &[u8]) -> bool {
    source
        .windows(SHAPE_MARK.len())
        .any(|window| window == SHAPE_MARK.as_bytes())
}

pub fn distance_field_alpha(inside: &[bool], size: u32) -> Option<Vec<u8>> {
    if size == 0 {
        return None;
    }
    let fine = size.checked_mul(SDF_SUPERSAMPLE)?;
    let fine_len = (fine as usize).checked_mul(fine as usize)?;
    if inside.len() != fine_len {
        return None;
    }

    let out = euclidean_distance(inside, fine, false);
    let within = euclidean_distance(inside, fine, true);

    let block = SDF_SUPERSAMPLE as usize;
    let cell = size as usize;
    let cell_len = cell.checked_mul(cell)?;
    let mut alpha = vec![255u8; cell_len];
    for y in 0..cell {
        for x in 0..cell {
            let mut sum = 0.0f32;
            for dy in 0..block {
                for dx in 0..block {
                    let i = (y * block + dy) * fine as usize + x * block + dx;
                    sum += out[i] - within[i];
                }
            }
            let fine_px = sum / (block * block) as f32;
            let cell_fraction = fine_px / fine as f32;
            let stored = 0.5 + cell_fraction / (2.0 * SDF_RANGE);
            alpha[y * cell + x] = (stored.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
    Some(alpha)
}

pub fn distance_field_alpha_from_coverage(coverage: &[u8], size: u32) -> Option<Vec<u8>> {
    let fine = size.checked_mul(SDF_SUPERSAMPLE)?;
    let fine_len = (fine as usize).checked_mul(fine as usize)?;
    if coverage.len() != fine_len {
        return None;
    }
    let inside: Vec<bool> = coverage.iter().map(|&alpha| alpha >= 128).collect();
    distance_field_alpha(&inside, size)
}

pub fn distance_field_rgba(inside: &[bool], size: u32) -> Option<Vec<u8>> {
    let alpha = distance_field_alpha(inside, size)?;
    let rgba_len = alpha.len().checked_mul(4)?;
    let mut rgba = vec![255u8; rgba_len];
    for (pixel, stored) in rgba.chunks_exact_mut(4).zip(alpha) {
        pixel[3] = stored;
    }
    Some(rgba)
}

fn euclidean_distance(inside: &[bool], size: u32, seed_outside: bool) -> Vec<f32> {
    let n = size as usize;
    let far = f32::MAX / 4.0;
    let mut grid: Vec<f32> = inside
        .iter()
        .map(
            |&is_inside| {
                if is_inside == seed_outside {
                    far
                } else {
                    0.0
                }
            },
        )
        .collect();

    let mut line = vec![0.0f32; n];
    for x in 0..n {
        for y in 0..n {
            line[y] = grid[y * n + x];
        }
        let done = envelope(&line);
        for y in 0..n {
            grid[y * n + x] = done[y];
        }
    }
    for y in 0..n {
        let done = envelope(&grid[y * n..(y + 1) * n]);
        grid[y * n..(y + 1) * n].copy_from_slice(&done);
    }
    grid.iter()
        .map(|distance| distance.max(0.0).sqrt())
        .collect()
}

fn envelope(f: &[f32]) -> Vec<f32> {
    let n = f.len();
    let mut out = vec![0.0f32; n];
    if n == 0 {
        return out;
    }
    let mut vertex = vec![0usize; n];
    let mut cross = vec![0.0f32; n + 1];
    let mut k = 0usize;
    cross[0] = f32::MIN;
    cross[1] = f32::MAX;
    let squared = |value: usize| (value * value) as f32;

    for q in 1..n {
        loop {
            let intersection = ((f[q] + squared(q)) - (f[vertex[k]] + squared(vertex[k])))
                / (2.0 * q as f32 - 2.0 * vertex[k] as f32);
            if intersection <= cross[k] && k > 0 {
                k -= 1;
            } else {
                k += 1;
                vertex[k] = q;
                cross[k] = intersection;
                cross[k + 1] = f32::MAX;
                break;
            }
        }
    }

    k = 0;
    for (q, slot) in out.iter_mut().enumerate() {
        while cross[k + 1] < q as f32 {
            k += 1;
        }
        *slot = (q as f32 - vertex[k] as f32).powi(2) + f[vertex[k]];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_field_and_the_material_are_the_shells() {
        assert_eq!(decode_sdf(0.0), -SDF_RANGE);
        assert_eq!(decode_sdf(0.5), 0.0);
        assert_eq!(decode_sdf(1.0), SDF_RANGE);
        assert!((depth_for(100.0) - 7.5).abs() < 1e-5);
        assert_eq!(CELL, 128);
        assert_eq!(SDF_SUPERSAMPLE, 4);
    }

    #[test]
    fn a_shape_says_so_in_its_source() {
        assert!(is_shape(b"<!-- lxb:shape -->"));
        assert!(!is_shape(b"<svg/>"));
    }

    #[test]
    fn the_shells_boolean_and_coverage_paths_are_the_same_transform() {
        let size = 4;
        let fine = size * SDF_SUPERSAMPLE;
        let inside: Vec<bool> = (0..fine)
            .flat_map(|y| (0..fine).map(move |x| (4..12).contains(&x) && (4..12).contains(&y)))
            .collect();
        let coverage: Vec<u8> = inside
            .iter()
            .map(|&is_inside| if is_inside { 128 } else { 127 })
            .collect();

        let from_inside = distance_field_alpha(&inside, size).unwrap();
        let from_coverage = distance_field_alpha_from_coverage(&coverage, size).unwrap();
        assert_eq!(from_inside, from_coverage);
        assert_eq!(from_inside.len(), (size * size) as usize);

        let at = |x: usize, y: usize| decode_sdf(from_inside[y * size as usize + x] as f32 / 255.0);
        assert!(at(1, 1) < 0.0, "the shape is the negative side");
        assert!(at(0, 0) > 0.0, "the air is the positive side");
    }

    #[test]
    fn rgba_is_white_with_the_measurement_in_alpha() {
        let size = 2;
        let fine = size * SDF_SUPERSAMPLE;
        let inside: Vec<bool> = (0..fine)
            .flat_map(|_| (0..fine).map(move |x| x < fine / 2))
            .collect();
        let alpha = distance_field_alpha(&inside, size).unwrap();
        let rgba = distance_field_rgba(&inside, size).unwrap();
        assert_eq!(rgba.len(), alpha.len() * 4);
        for (pixel, stored) in rgba.chunks_exact(4).zip(alpha) {
            assert_eq!(&pixel[..3], &[255, 255, 255]);
            assert_eq!(pixel[3], stored);
        }
    }

    #[test]
    fn the_native_atlas_cell_is_a_128_texel_field() {
        let fine = CELL * SDF_SUPERSAMPLE;
        let inside: Vec<bool> = (0..fine)
            .flat_map(|y| {
                (0..fine).map(move |x| {
                    (fine / 4..fine * 3 / 4).contains(&x) && (fine / 4..fine * 3 / 4).contains(&y)
                })
            })
            .collect();
        let alpha = distance_field_alpha(&inside, CELL).unwrap();
        assert_eq!(alpha.len(), (CELL * CELL) as usize);
        assert!(alpha[(CELL * CELL / 2 + CELL / 2) as usize] < 128);
        assert!(alpha[0] > 128);
    }

    #[test]
    fn bad_raster_dimensions_are_refused() {
        assert!(distance_field_alpha(&[], 0).is_none());
        assert!(distance_field_alpha(&[false; 15], 1).is_none());
        assert!(distance_field_alpha_from_coverage(&[0; 15], 1).is_none());
        assert!(distance_field_rgba(&[false; 15], 1).is_none());
        assert!(distance_field_alpha(&[], u32::MAX).is_none());
    }
}
