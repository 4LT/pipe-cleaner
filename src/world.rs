use crate::visual;
use visual::geo;

pub struct World {
    ring_model: usize,
}

impl World {
    pub fn new(builder: &mut visual::ManagerBuilder) -> Self {
        let vertices = geo::circle_pts(20);
        let indices = geo::loop_indices(20);

        let ring_mesh = visual::Mesh {
            vertices,
            indices,
        };

        let ring_model = builder.register_class(ring_mesh);

        Self {
            ring_model,
        }
    }
}
