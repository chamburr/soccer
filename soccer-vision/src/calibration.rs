pub const ACC_OFFSET: [f32; 3] = [0., 0., 0.];
pub const ACC_MISALIGNMENT: [[f32; 3]; 3] = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];

// CEN
// pub const HARD_IRON_OFFSET: [f32; 3] = [-23.020, -164.870, 114.170];
// pub const SOFT_IRON_MATRIX: [[f32; 3]; 3] = [
//     [3.755, 0.086, 0.140],
//     [0.086, 3.322, -0.052],
//     [0.140, -0.052, 3.519],
// ];

// HAN
pub const HARD_IRON_OFFSET: [f32; 3] = [-47.940, -135.490, 267.130];
pub const SOFT_IRON_MATRIX: [[f32; 3]; 3] = [
    [3.008, -0.178, -0.265],
    [-0.178, 3.095, 0.076],
    [-0.265, 0.076, 3.031],
];
