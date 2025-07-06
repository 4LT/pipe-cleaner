use std::cell::RefCell;
use std::mem::size_of;
use std::ops::Range;
use std::rc::Rc;

pub mod geo;
mod renderer;

pub use renderer::Renderer;

const IDENTITY: [f32; 12] = [
    1f32, 0f32, 0f32, 0f32, 0f32, 1f32, 0f32, 0f32, 0f32, 0f32, 1f32, 0f32,
];

#[derive(Clone)]
pub struct BaseMesh {
    pub indices: Box<[u32]>,
    pub vertices: Box<[[f32; 3]]>,
}

#[derive(Clone, Copy)]
pub struct ThickMeshVertex {
    pub this_position: [f32; 3],
    pub other_position: [f32; 3],
}

#[derive(Clone)]
pub struct ThickMesh {
    pub indices: Box<[u32]>,
    pub vertices: Box<[ThickMeshVertex]>,
}

impl BaseMesh {
    pub fn thicken(self) -> ThickMesh {
        let mut new_indices = Vec::with_capacity(self.indices.len() * 3);

        let mut new_vertices = Vec::with_capacity(self.indices.len() * 2);
        let segments = self.indices.chunks(2);

        for segment in segments {
            let vertex_template = ThickMeshVertex {
                this_position: self.vertices[segment[0] as usize],
                other_position: self.vertices[segment[1] as usize],
            };

            new_vertices.push(vertex_template);
            new_vertices.push(vertex_template);
            new_vertices.push(vertex_template);
            new_vertices.push(vertex_template);

            let idx1 = (new_vertices.len() - 4) as u32;

            new_indices.push(idx1);
            new_indices.push(idx1 + 1);
            new_indices.push(idx1 + 2);
            new_indices.push(idx1 + 2);
            new_indices.push(idx1 + 3);
            new_indices.push(idx1);
        }

        ThickMesh {
            indices: new_indices.into(),
            vertices: new_vertices.into(),
        }
    }
}

pub type Attributes = [u8; 64];

pub type TransformMatrix = [f32; 12];
pub type Color = [f32; 3];

pub trait Instance {
    /// Get transform matrix from model space to world space
    fn transform(&self) -> TransformMatrix {
        IDENTITY
    }

    /// Color, RGB 0-1
    fn color(&self) -> Color;

    fn model(&self) -> usize;

    fn attributes(&self) -> Attributes {
        let mut bytes: Attributes = [0; size_of::<Attributes>()];

        for (idx, float) in self.transform().iter().enumerate() {
            let start = idx * 4;
            let end = start + 4;
            bytes[start..end].copy_from_slice(&float.to_ne_bytes());
        }

        for (idx, float) in self.color().iter().enumerate() {
            let start = size_of::<TransformMatrix>() + idx * 4;
            let end = start + 4;
            bytes[start..end].copy_from_slice(&float.to_ne_bytes());
        }

        bytes
    }
}

impl<T: Instance> Instance for Rc<RefCell<T>> {
    fn transform(&self) -> TransformMatrix {
        RefCell::borrow(self).transform()
    }

    fn color(&self) -> Color {
        RefCell::borrow(self).color()
    }

    fn model(&self) -> usize {
        RefCell::borrow(self).model()
    }

    fn attributes(&self) -> Attributes {
        RefCell::borrow(self).attributes()
    }
}

#[derive(Clone, Copy)]
pub struct WorldPosition(pub [f32; 3]);

pub struct Model {
    index_range: Range<u32>,
    vertex_range: Range<u32>,
}

impl Model {
    fn new(
        mesh: ThickMesh,
        index_start: u32,
        index_buffer: &wgpu::Buffer,
        vertex_start: u32,
        vertex_buffer: &wgpu::Buffer,
    ) -> (Self, u64, u64) {
        let index_bytes: Vec<u8> = mesh
            .indices
            .iter()
            .flat_map(|idx| (idx + vertex_start).to_ne_bytes())
            .collect();

        let vertex_bytes: Vec<u8> = mesh
            .vertices
            .iter()
            .flat_map(|v| {
                v.this_position.iter().flat_map(|f| f.to_ne_bytes()).chain(
                    v.other_position.iter().flat_map(|f| f.to_ne_bytes()),
                )
            })
            .collect();

        let index_offset = index_start as u64 * size_of::<u32>() as u64;
        let vertex_offset =
            vertex_start as u64 * size_of::<ThickMeshVertex>() as u64;

        index_buffer
            .slice(index_offset..(index_offset + index_bytes.len() as u64))
            .get_mapped_range_mut()
            .copy_from_slice(&index_bytes);

        vertex_buffer
            .slice(vertex_offset..(vertex_offset + vertex_bytes.len() as u64))
            .get_mapped_range_mut()
            .copy_from_slice(&vertex_bytes);

        let index_end = index_start + mesh.indices.len() as u32;
        let vertex_end = vertex_start + mesh.vertices.len() as u32;

        (
            Self {
                index_range: index_start..index_end,
                vertex_range: vertex_start..vertex_end,
            },
            index_bytes.len() as u64,
            vertex_bytes.len() as u64,
        )
    }
}

pub struct ManagerBuilder {
    meshes: Vec<ThickMesh>,
}

impl ManagerBuilder {
    pub fn new() -> Self {
        Self { meshes: Vec::new() }
    }

    pub fn register_model(&mut self, mesh: ThickMesh) -> usize {
        let model_idx = self.meshes.len();
        self.meshes.push(mesh);
        model_idx
    }

    pub fn build(self, max_instances: u32, device: &wgpu::Device) -> Manager {
        Manager::new(self.meshes.into(), max_instances, device)
    }
}

pub struct Manager {
    index_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    inst_buffer: wgpu::Buffer,
    models: Vec<Model>,
}

impl Manager {
    fn new(
        meshes: Box<[ThickMesh]>,
        max_instances: u32,
        device: &wgpu::Device,
    ) -> Self {
        let (idx_ct_sum, vert_ct_sum) = meshes.iter().fold(
            (0u64, 0u64),
            |(idx_ct_sum, vert_ct_sum), mesh| {
                (
                    idx_ct_sum + mesh.indices.len() as u64,
                    vert_ct_sum + mesh.vertices.len() as u64,
                )
            },
        );

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("index_buffer"),
            size: idx_ct_sum * size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::INDEX,
            mapped_at_creation: true,
        });

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vertex_buffer"),
            size: vert_ct_sum * size_of::<ThickMeshVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: true,
        });

        let inst_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instance_buffer"),
            size: max_instances as u64 * size_of::<[f32; 16]>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        println!("Total Index Ct: {}", idx_ct_sum);
        println!("Total Vertex Ct: {}", vert_ct_sum);
        println!("Total Instance Max: {}", max_instances);

        let mut index_start = 0u32;
        let mut vertex_start = 0u32;

        let mut models = Vec::new();

        for mesh in meshes {
            println!("Index Start: {}", index_start);
            println!("Vertex Start: {}", vertex_start);

            let (model, index_byte_ct, vertex_byte_ct) = Model::new(
                mesh,
                index_start,
                &index_buffer,
                vertex_start,
                &vertex_buffer,
            );

            println!("Allocated {} Index Bytes", index_byte_ct);
            println!("Allocated {} Vertex Bytes", vertex_byte_ct);

            index_start = model.index_range.end;
            vertex_start = model.vertex_range.end;

            models.push(model);
        }

        index_buffer.unmap();
        vertex_buffer.unmap();

        Self {
            index_buffer,
            vertex_buffer,
            inst_buffer,
            models,
        }
    }

    pub fn update<'a>(
        &'_ mut self,
        queue: &'a wgpu::Queue,
        instances: impl Iterator<Item = &'a dyn Instance>,
    ) -> Vec<(Range<u32>, Range<u32>)> {
        let mut attributes = vec![Vec::<Attributes>::new(); self.models.len()];
        for inst in instances.into_iter() {
            attributes[inst.model()].push(inst.attributes());
        }

        let mut offset = 0u64;
        for i in 0..self.models.len() {
            let bytes = attributes[i].as_flattened();
            queue.write_buffer(self.instances(), offset, &*bytes);
            offset += bytes.len() as u64;
        }

        let instance_counts = attributes.iter().map(|v| v.len() as u32);

        self.ranges(instance_counts)
    }

    fn ranges(
        &self,
        instance_counts: impl Iterator<Item = u32>,
    ) -> Vec<(Range<u32>, Range<u32>)> {
        let mut instance_start = 0u32;

        self.models
            .iter()
            .zip(instance_counts)
            .map(move |(model, instance_count)| {
                let instance_end = instance_start + instance_count;
                let ranges =
                    (model.index_range.clone(), (instance_start..instance_end));
                instance_start = instance_end;
                ranges
            })
            .collect()
    }

    pub fn indices(&self) -> &wgpu::Buffer {
        &self.index_buffer
    }

    pub fn vertices(&self) -> &wgpu::Buffer {
        &self.vertex_buffer
    }

    pub fn instances(&self) -> &wgpu::Buffer {
        &self.inst_buffer
    }
}
