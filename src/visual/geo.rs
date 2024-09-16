pub fn circle_pts(vert_count: i32) -> Vec<[f32; 3]> {
    (0..vert_count)
        .map(|i| {
            let angle =
                (i as f32) / (vert_count as f32) * std::f32::consts::TAU;
            let (sin, cos) = angle.sin_cos();
            [cos, sin, 0f32]
        })
        .collect()
}

pub fn loop_indices(vert_count: u32) -> Vec<u32> {
    (0u32..vert_count)
        .flat_map(|idx| [idx, (idx + 1) % vert_count])
        .collect()
}

pub fn cube_pts() -> Vec<[f32; 3]> {
    vec![
        [-0.1f32, -0.1f32, -0.1f32], // 0
        [-0.1f32, -0.1f32, 0.1f32],  // 1
        [-0.1f32, 0.1f32, -0.1f32],  // 2
        [-0.1f32, 0.1f32, 0.1f32],   // 3
        [0.1f32, -0.1f32, -0.1f32],  // 4
        [0.1f32, -0.1f32, 0.1f32],   // 5
        [0.1f32, 0.1f32, -0.1f32],   // 6
        [0.1f32, 0.1f32, 0.1f32],    // 7
    ]
}

pub fn cube_indices() -> Vec<u32> {
    vec![
        0, 1, 0, 2, 0, 4, 1, 3, 1, 5, 2, 3, 2, 6, 3, 7, 4, 5, 4, 6, 5, 7, 6, 7,
    ]
}
