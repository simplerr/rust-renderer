use glam::{Mat4, Vec2, Vec4};

use crate::device::*;
use crate::gltf_loader::*;
use crate::primitive::*;
use crate::Model;

pub struct ModelLoader {}

pub fn add_triangle(indices: &mut Vec<u32>, v1: u32, v2: u32, v3: u32) {
    indices.push(v1);
    indices.push(v2);
    indices.push(v3);
}

pub fn add_vertex(
    vertices: &mut Vec<Vertex>,
    x: f32,
    y: f32,
    z: f32,
    nx: f32,
    ny: f32,
    nz: f32,
    u: f32,
    v: f32,
) {
    vertices.push(Vertex {
        pos: Vec4::new(x, y, z, 0.0),
        normal: Vec4::new(nx, ny, nz, 0.0),
        uv: Vec2::new(u, v),
        color: Vec4::new(1.0, 1.0, 1.0, 1.0),
        tangent: Vec4::new(0.0, 0.0, 0.0, 0.0),
    });
}

impl ModelLoader {
    pub fn load_triangle(device: &Device) -> Model {
        let indices = vec![0, 1, 2];

        let mut vertices = vec![];
        add_vertex(&mut vertices, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0);
        add_vertex(&mut vertices, -1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0);
        add_vertex(&mut vertices, 1.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0);

        let model = Model {
            meshes: vec![Mesh {
                primitive: Primitive::new(device, indices, vertices),
                material: Material {
                    diffuse_map: DEFAULT_TEXTURE_MAP,
                    normal_map: DEFAULT_TEXTURE_MAP,
                    metallic_roughness_map: DEFAULT_TEXTURE_MAP,
                    occlusion_map: DEFAULT_TEXTURE_MAP,
                },
                gpu_mesh: 0,
            }],
            transforms: vec![Mat4::IDENTITY],
            textures: vec![],
        };

        model
    }
    pub fn load_cube(device: &Device) -> Model {
        let mut model = Model {
            meshes: vec![],
            transforms: vec![],
            textures: vec![],
        };

        let mut indices = vec![];
        let mut vertices = vec![];

        // Front
        add_triangle(&mut indices, 2, 0, 1);
        add_triangle(&mut indices, 0, 2, 3);

        // Back
        add_triangle(&mut indices, 4, 6, 5);
        add_triangle(&mut indices, 6, 4, 7);

        // Top
        add_triangle(&mut indices, 10, 8, 9);
        add_triangle(&mut indices, 8, 10, 11);

        // Bottom
        add_triangle(&mut indices, 12, 14, 13);
        add_triangle(&mut indices, 14, 12, 15);

        // Let
        add_triangle(&mut indices, 16, 18, 17);
        add_triangle(&mut indices, 18, 16, 19);

        // Right
        add_triangle(&mut indices, 22, 20, 21);
        add_triangle(&mut indices, 20, 22, 23);

        // Front
        add_vertex(&mut vertices, -0.5, -0.5, 0.5, 0.0, 0.0, 1.0, 0.0, 1.0);
        add_vertex(&mut vertices, 0.5, -0.5, 0.5, 0.0, 0.0, 1.0, 1.0, 1.0);
        add_vertex(&mut vertices, 0.5, 0.5, 0.5, 0.0, 0.0, 1.0, 1.0, 0.0);
        add_vertex(&mut vertices, -0.5, 0.5, 0.5, 0.0, 0.0, 1.0, 0.0, 0.0);

        // Back
        add_vertex(&mut vertices, -0.5, -0.5, -0.5, 0.0, 0.0, -1.0, 0.0, 1.0);
        add_vertex(&mut vertices, 0.5, -0.5, -0.5, 0.0, 0.0, -1.0, 1.0, 1.0);
        add_vertex(&mut vertices, 0.5, 0.5, -0.5, 0.0, 0.0, -1.0, 1.0, 0.0);
        add_vertex(&mut vertices, -0.5, 0.5, -0.5, 0.0, 0.0, -1.0, 0.0, 0.0);

        // Top
        add_vertex(&mut vertices, -0.5, -0.5, -0.5, 0.0, -1.0, 0.0, 0.0, 1.0);
        add_vertex(&mut vertices, 0.5, -0.5, -0.5, 0.0, -1.0, 0.0, 1.0, 1.0);
        add_vertex(&mut vertices, 0.5, -0.5, 0.5, 0.0, -1.0, 0.0, 1.0, 0.0);
        add_vertex(&mut vertices, -0.5, -0.5, 0.5, 0.0, -1.0, 0.0, 0.0, 0.0);

        // Bottom
        add_vertex(&mut vertices, -0.5, 0.5, -0.5, 0.0, 1.0, 0.0, 0.0, 1.0);
        add_vertex(&mut vertices, 0.5, 0.5, -0.5, 0.0, 1.0, 0.0, 1.0, 1.0);
        add_vertex(&mut vertices, 0.5, 0.5, 0.5, 0.0, 1.0, 0.0, 1.0, 0.0);
        add_vertex(&mut vertices, -0.5, 0.5, 0.5, 0.0, 1.0, 0.0, 0.0, 0.0);

        // Let
        add_vertex(&mut vertices, -0.5, -0.5, -0.5, -1.0, 0.0, 0.0, 0.0, 1.0);
        add_vertex(&mut vertices, -0.5, 0.5, -0.5, -1.0, 0.0, 0.0, 1.0, 1.0);
        add_vertex(&mut vertices, -0.5, 0.5, 0.5, -1.0, 0.0, 0.0, 1.0, 0.0);
        add_vertex(&mut vertices, -0.5, -0.5, 0.5, -1.0, 0.0, 0.0, 0.0, 0.0);

        // Right
        add_vertex(&mut vertices, 0.5, -0.5, -0.5, 1.0, 0.0, 0.0, 0.0, 1.0);
        add_vertex(&mut vertices, 0.5, 0.5, -0.5, 1.0, 0.0, 0.0, 1.0, 1.0);
        add_vertex(&mut vertices, 0.5, 0.5, 0.5, 1.0, 0.0, 0.0, 1.0, 0.0);
        add_vertex(&mut vertices, 0.5, -0.5, 0.5, 1.0, 0.0, 0.0, 0.0, 0.0);

        model.meshes.push(Mesh {
            primitive: Primitive::new(device, indices, vertices),
            material: Material {
                diffuse_map: DEFAULT_TEXTURE_MAP,
                normal_map: DEFAULT_TEXTURE_MAP,
                metallic_roughness_map: DEFAULT_TEXTURE_MAP,
                occlusion_map: DEFAULT_TEXTURE_MAP,
            },
            gpu_mesh: 0,
        });
        model.transforms.push(Mat4::IDENTITY);

        model
    }
}
