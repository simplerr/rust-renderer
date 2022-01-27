use glam::{Mat4, Vec2, Vec3, Vec4};

use crate::device::*;
use crate::primitive::*;
use crate::texture::*;

pub struct Material {
    pub diffuse_map: u32,
    pub normal_map: u32,
}

pub struct Mesh {
    pub primitive: Primitive,
    pub material: Material,
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub textures: Vec<Texture>,
    pub transforms: Vec<Mat4>,
}

pub fn load_node(
    device: &Device,
    node: &gltf::Node,
    model: &mut Model,
    buffers: &Vec<gltf::buffer::Data>,
    parent_transform: Mat4,
) {
    let node_transform =
        parent_transform * glam::Mat4::from_cols_array_2d(&node.transform().matrix());

    for child in node.children() {
        load_node(device, &child, model, buffers, node_transform);
    }

    if let Some(mesh) = node.mesh() {
        let primitives = mesh.primitives();

        for primitive in primitives {
            let reader = primitive.reader(|i| Some(&buffers[i.index()]));

            let indices: Vec<_> = reader.read_indices().unwrap().into_u32().collect();
            let positions: Vec<_> = reader.read_positions().unwrap().map(Vec3::from).collect();
            let normals: Vec<_> = reader.read_normals().unwrap().map(Vec3::from).collect();
            let tex_coords: Vec<_> = reader
                .read_tex_coords(0)
                .unwrap()
                .into_f32()
                .map(Vec2::from)
                .collect();

            let mut vertices: Vec<Vertex> = vec![];

            for (i, _) in positions.iter().enumerate() {
                vertices.push(Vertex {
                    pos: positions[i],
                    normal: normals[i],
                    uv: tex_coords[i],
                    color: Vec4::new(1.0, 1.0, 1.0, 1.0),
                });
            }

            let material = primitive.material();
            let pbr = material.pbr_metallic_roughness();
            let diffuse = pbr.base_color_texture().unwrap();
            let diffuse = diffuse.texture();
            let diffuse_index = diffuse.source().index() as u32;

            let normal_index = if let Some(texture) = material.normal_texture() {
                texture.texture().index() as u32
            } else {
                0
            };

            model.meshes.push(Mesh {
                primitive: Primitive::new(device, indices, vertices),
                material: Material {
                    diffuse_map: diffuse_index,
                    normal_map: normal_index,
                },
            });

            model.transforms.push(node_transform);
        }
    }
}

pub fn load_gltf(device: &Device, path: &str) -> Model {
    let (gltf, buffers, mut images) = match gltf::import(path) {
        Ok(result) => (result),
        Err(err) => panic!("Loading model {} failed with error: {}", path, err),
    };

    let mut model = Model {
        meshes: vec![],
        transforms: vec![],
        textures: vec![],
    };

    for image in &mut images {
        // Convert images from rgb8 to rgba8
        if image.format == gltf::image::Format::R8G8B8 {
            let dynamic_image = image::DynamicImage::ImageRgb8(
                image::RgbImage::from_raw(
                    image.width,
                    image.height,
                    std::mem::take(&mut image.pixels),
                )
                .unwrap(),
            );

            let rgba8_image = dynamic_image.to_rgba8();
            image.format = gltf::image::Format::R8G8B8A8;
            image.pixels = rgba8_image.into_raw();
        }

        if image.format != gltf::image::Format::R8G8B8A8 {
            panic!("Unsupported image format!");
        }

        let texture = Texture::create(device, &image.pixels, image.width, image.height);

        model.textures.push(texture);
    }

    for (i, material) in gltf.materials().enumerate() {
        let pbr = material.pbr_metallic_roughness();
        let diffuse = pbr.base_color_texture().unwrap();
        let diffuse = diffuse.texture();
        let image_index = diffuse.source().index();
        let image = &images[image_index];

        println!(
            "image_index: {}, format: {:#?}, widht: {}, height: {}",
            image_index, image.format, image.width, image.height
        );
    }

    for scene in gltf.scenes() {
        for node in scene.nodes() {
            load_node(device, &node, &mut model, &buffers, Mat4::IDENTITY);
        }
    }

    model
}
