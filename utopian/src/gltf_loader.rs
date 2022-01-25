use glam::{Mat4, Vec2, Vec3, Vec4};

use crate::device::*;
use crate::primitive::*;
use crate::texture::*;

pub struct Model {
    pub primitives: Vec<Primitive>,
    pub transforms: Vec<Mat4>,
    pub textures: Vec<Texture>,
    pub primitive_to_diffuse_idx: Vec<u32>,
    pub primitive_to_normal_idx: Vec<u32>,
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

            model
                .primitives
                .push(Primitive::new(device, indices, vertices));
            model.transforms.push(node_transform);

            let material = primitive.material();
            let pbr = material.pbr_metallic_roughness();
            let diffuse = pbr.base_color_texture().unwrap();
            let diffuse = diffuse.texture();
            let image_index = diffuse.source().index();
            model.primitive_to_diffuse_idx.push(image_index as u32);

            if let Some(texture) = material.normal_texture() {
                model
                    .primitive_to_normal_idx
                    .push(texture.texture().index() as u32);
            } else {
                model.primitive_to_normal_idx.push(0);
            }
        }
    }
}

pub fn load_gltf(device: &Device, path: &str) -> Model {
    let (gltf, buffers, mut images) = match gltf::import(path) {
        Ok(result) => (result),
        Err(err) => panic!("Loading model {} failed with error: {}", path, err),
    };

    let mut model = Model {
        primitives: vec![],
        transforms: vec![],
        textures: vec![],
        primitive_to_diffuse_idx: vec![],
        primitive_to_normal_idx: vec![],
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
