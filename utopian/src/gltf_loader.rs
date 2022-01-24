use glam::{Vec2, Vec3, Vec4, Mat4};

use crate::device::*;
use crate::primitive::*;

pub struct Model {
    pub primitives: Vec<Primitive>,
    pub transforms: Vec<Mat4>,
}

pub fn load_node(
    device: &Device,
    node: &gltf::Node,
    model: &mut Model,
    buffers: &Vec<gltf::buffer::Data>,
    parent_transform: Mat4,
) {
    let node_transform = parent_transform * glam::Mat4::from_cols_array_2d(&node.transform().matrix());

    for child in node.children() {
        load_node(device, &child, model, buffers, node_transform);
    }

    if let Some(mesh) = node.mesh() {
        let primitives = mesh.primitives();

        for primitive in primitives {
            println!("Loading primtive!");
            let reader = primitive.reader(|i| Some(&buffers[i.index()]));

            let indices: Vec<_> = reader.read_indices().unwrap().into_u32().collect();
            let positions: Vec<_> =
                reader.read_positions().unwrap().map(Vec3::from).collect();
            let normals: Vec<_> =
                reader.read_normals().unwrap().map(Vec3::from).collect();

            let mut vertices: Vec<Vertex> = vec![];

            for (i, _) in positions.iter().enumerate() {
                vertices.push(Vertex {
                    pos: positions[i],
                    normal: normals[i],
                    uv: Vec2::new(0.0, 0.0),
                    color: Vec4::new(1.0, 1.0, 1.0, 1.0),
                });
            }

            model.primitives.push(Primitive::new(device, indices, vertices));
            model.transforms.push(node_transform);
        }
    }
}

pub fn load_gltf(device: &Device, path: &str) -> Model {
    let (gltf, buffers, mut _images) = match gltf::import(path) {
        Ok(result) => (result),
        Err(err) => panic!("Loading model {} failed with error: {}", path, err),
    };

    let mut model = Model {
        primitives: vec![],
        transforms: vec![],
    };

    for scene in gltf.scenes() {
        for node in scene.nodes() {
            load_node(device, &node, &mut model, &buffers, Mat4::IDENTITY);
        }
    }

    model
}
