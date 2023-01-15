use std::mem::size_of;
use std::rc::{Rc, Weak};
use std::ops::Range;
use std::cell::RefCell;

const IDENTITY: [f32; 12] = [
    1f32, 0f32, 0f32, 0f32,
    0f32, 1f32, 0f32, 0f32,
    0f32, 0f32, 1f32, 0f32,
];

#[derive(Clone, Copy)]
pub struct Mesh<'a> {
    pub indices: &'a [u32],
    pub vertices: &'a [[f32; 3]],
}

pub struct Instance<P> {
    pub position: P,
    pub color: [f32; 3],
}

pub trait Attributes {
    /// Get transform matrix from model space to _something_
    fn transform(&self) -> [f32; 12] {
        IDENTITY
    }

    /// Color, RGB 0-1
    fn color(&self) -> [f32; 3];
}

type InternalInstRef = Weak<RefCell<dyn Attributes>>;

pub type ExternalInstRef<P> = Rc<RefCell<Instance<P>>>;

#[derive(Clone, Copy)]
pub struct WorldPosition(pub [f32; 3]);

impl Attributes for Instance<WorldPosition> {
    fn transform(&self) -> [f32; 12] {
        let WorldPosition([x, y, z]) = self.position;

        [
            1f32, 0f32, 0f32, x,
            0f32, 1f32, 0f32, y,
            0f32, 0f32, 1f32, z,
        ]
    }

    fn color(&self) -> [f32; 3] {
        self.color
    }
}

#[derive(Clone, Copy)]
pub struct PipePosition {
    pub angle: f32,
    pub depth: f32,
}

impl Attributes for Instance<PipePosition> {
    /// Convert position into model-to-pipe-space matrix
    fn transform(&self) -> [f32; 12] {
        let pos = self.position;
        let (sin, cos) = pos.angle.sin_cos();

        [
            1f32, 0f32, 0f32, cos,
            0f32, 1f32, 0f32, sin,
            0f32, 0f32, 1f32, pos.depth,
        ]
    }

    fn color(&self) -> [f32; 3] {
        self.color
    }
}

pub struct Class {
    index_range: Range<u32>,
    vertex_range: Range<u32>,
    instances: Vec<InternalInstRef>,
}

impl Class {
    fn new(
        mesh: Mesh<'_>,
        index_start: u32,
        index_buffer: &wgpu::Buffer,
        vertex_start: u32,
        vertex_buffer: &wgpu::Buffer,
    ) -> (Self, u64, u64) {
        let index_bytes: Vec<u8> = mesh.indices.iter()
            .flat_map(|idx| { (idx + vertex_start).to_ne_bytes() })
            .collect();

        let vertex_bytes: Vec<u8> = mesh.vertices.iter()
            .flat_map(|arr| {
                arr.iter().flat_map(|f| { f.to_ne_bytes() })
            })
            .collect();

        let index_offset = index_start as u64 * size_of::<u32>() as u64;
        let vertex_offset = vertex_start as u64 * size_of::<[f32; 3]>() as u64;

        index_buffer.slice(
            index_offset..(index_offset + index_bytes.len() as u64)
        ).get_mapped_range_mut().copy_from_slice(&index_bytes);

        vertex_buffer.slice(
            vertex_offset..(vertex_offset + vertex_bytes.len() as u64)
        ).get_mapped_range_mut().copy_from_slice(&vertex_bytes);

        let index_end = index_start + mesh.indices.len() as u32;
        let vertex_end = vertex_start + mesh.vertices.len() as u32;

        (
            Self {
                index_range: index_start..index_end,
                vertex_range: vertex_start..vertex_end,
                instances: Vec::new(),
            },
            index_bytes.len() as u64,
            vertex_bytes.len() as u64,
        )
    }

    fn clean_up(&mut self) -> u32 {
        let old_count = self.instances.len();

        self.instances.retain(|weak_ref| {
            weak_ref.upgrade().is_some()
        });

        (old_count - self.instances.len()) as u32
    }
}

pub struct ManagerBuilder<'m> {
    meshes: Vec<Mesh<'m>>,
}

impl<'m> ManagerBuilder<'m> {
    pub fn new() -> Self {
        Self { meshes: Vec::new() }
    }

    pub fn register_class(&mut self, mesh: Mesh<'m>) -> u64 {
        let class_idx = self.meshes.len();
        self.meshes.push(mesh);
        class_idx as u64
    }

    pub fn build(
        self,
        max_instances: u32,
        device: &wgpu::Device,
    ) -> Manager {
        Manager::new(&self.meshes[..], max_instances, device)
    }
}

pub struct Manager {
    index_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    inst_buffer: wgpu::Buffer,
    instance_ct: u32,
    max_instances: u32,
    classes: Vec<Class>,
}

impl Manager {
    fn new<'m>(
        meshes: &[Mesh<'m>],
        max_instances: u32,
        device: &wgpu::Device,
    ) -> Self {
        let (idx_ct_sum, vert_ct_sum) = meshes.iter()
            .fold(
                (0u64, 0u64),
                |
                    (idx_ct_sum, vert_ct_sum),
                    &mesh
                | {(
                    idx_ct_sum + mesh.indices.len() as u64,
                    vert_ct_sum + mesh.vertices.len() as u64,
                )},
            );

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("index_buffer"),
            size: idx_ct_sum * size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::INDEX,
            mapped_at_creation: true,
        });

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vertex_buffer"),
            size: vert_ct_sum * size_of::<[f32; 3]>() as u64,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: true,
        });

        let inst_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instance_buffer"),
            size: max_instances as u64 * size_of::<[f32; 16]>() as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        println!("Total Index Ct: {}", idx_ct_sum);
        println!("Total Vertex Ct: {}", vert_ct_sum);
        println!("Total Instance Max: {}", max_instances);

        let mut index_start = 0u32;
        let mut vertex_start = 0u32;

        let mut classes = Vec::new();

        for &mesh in meshes {
            println!("Index Start: {}", index_start);
            println!("Vertex Start: {}", vertex_start);

            let (class, index_byte_ct, vertex_byte_ct) = Class::new(
                mesh,
                index_start,
                &index_buffer,
                vertex_start,
                &vertex_buffer,
            );

            println!("Allocated {} Index Bytes", index_byte_ct);
            println!("Allocated {} Vertex Bytes", vertex_byte_ct);

            index_start = class.index_range.end;
            vertex_start = class.vertex_range.end;

            classes.push(class);
        };

        index_buffer.unmap();
        vertex_buffer.unmap();

        Self {
            index_buffer,
            vertex_buffer,
            inst_buffer,
            instance_ct: 0u32,
            max_instances,
            classes,
        }
    }

    pub fn create_instance<P: 'static>(
        &mut self,
        class_idx: u64,
        position: P,
        color: [f32; 3],
    )  -> Rc<RefCell<Instance<P>>>
    where Instance<P>: Attributes {
        let class = &mut self.classes[class_idx as usize];

        if self.instance_ct >= self.max_instances {
            panic!("Hit max instances of {}", self.max_instances);
        }

        self.instance_ct+= 1u32;

        let inst = Rc::new(RefCell::new(Instance { position, color }));
        let clone = Rc::downgrade(&inst);
        class.instances.push(clone);
        inst
    }

    pub fn update(
        &mut self,
        queue: &wgpu::Queue,
    ) {
        for class in self.classes.iter_mut() {
            self.instance_ct-= class.clean_up();
        }

        let mut offset = 0u64;

        for class in &self.classes {
            for rc in class.instances.iter().map(|weak_ref| {
                weak_ref.upgrade().unwrap()
            }) {
                let instance = rc.borrow();
                let transform = instance.transform();
                let color = instance.color();

                let transform_bytes = transform.iter()
                    .flat_map(|f| { f.to_ne_bytes() });

                let color_bytes = color.iter()
                    .flat_map(|f| { f.to_ne_bytes() });

                let pad = [0u8, 0u8, 0u8, 0u8];

                let bytes: Vec<u8> = transform_bytes.chain(
                    color_bytes
                ).chain(
                    pad
                ).collect();

                queue.write_buffer(
                    self.instances(),
                    offset,
                    &bytes
                );

                offset+= bytes.len() as u64;
            }
        }
    }

    pub fn ranges(&self) -> impl Iterator<Item=(Range<u32>, Range<u32>)> +'_ {
        let mut instance_start = 0u32;
        self.classes.iter().map(move |class| {
            let instance_end = instance_start + class.instances.len() as u32;
            let ranges = (
                class.index_range.clone(),
                (instance_start..instance_end)
            );
            instance_start = instance_end;
            ranges
        })
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
