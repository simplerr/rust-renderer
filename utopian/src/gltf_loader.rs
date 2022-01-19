use glam::{Vec2, Vec3, Vec4};

use crate::device::*;
use crate::primitive::*;

pub fn load_gltf(device: &Device, path: &str) -> Primitive {
    let mut indices: Vec<u32> = vec![];
    let mut positions: Vec<Vec3> = vec![];
    let mut normals: Vec<Vec3> = vec![];
    let mut vertices: Vec<Vertex> = vec![];

    let (gltf, buffers, mut _images) = match gltf::import(path) {
        Ok(result) => (result),
        Err(err) => panic!("Loading model {} failed with error: {}", path, err),
    };

    for scene in gltf.scenes() {
        for node in scene.nodes() {
            if let Some(mesh) = node.mesh() {
                let primitives = mesh.primitives();

                for primitive in primitives {
                    let reader = primitive.reader(|i| Some(&buffers[i.index()]));

                    let prim_indices: Vec<_> = reader.read_indices().unwrap().into_u32().collect();
                    let prim_positions: Vec<_> =
                        reader.read_positions().unwrap().map(Vec3::from).collect();
                    let prim_normals: Vec<_> =
                        reader.read_normals().unwrap().map(Vec3::from).collect();

                    indices.extend(prim_indices);
                    positions.extend(prim_positions);
                    normals.extend(prim_normals);
                }
            }
        }
    }

    for (i, _) in positions.iter().enumerate() {
        vertices.push(Vertex {
            pos: positions[i],
            normal: normals[i],
            uv: Vec2::new(0.0, 0.0),
            color: Vec4::new(1.0, 1.0, 1.0, 1.0),
        });
    }

    Primitive::new(device, indices, vertices)
}
