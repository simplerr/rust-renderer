use glam::{Mat4, Vec2, Vec3, Vec4};

use crate::device::*;
use crate::primitive::*;
use crate::texture::*;

pub const DEFAULT_TEXTURE_MAP: u32 = u32::MAX;

#[derive(Copy, Clone)]
pub enum MaterialType {
    Lambertian = 0,
    Metal = 1,
    Dielectric = 2,
}

// Note: indexes into the Model specific texture array,
// not bindless indexes.
pub struct Material {
    pub diffuse_map: u32,
    pub normal_map: u32,
    pub metallic_roughness_map: u32,
    pub occlusion_map: u32,
    pub base_color_factor: Vec4,

    // Ray tracing properties
    pub material_type: MaterialType, // 0 = lambertian, 1 = metal, 2 = dielectric
    pub material_property: f32, // metal = fuzz, dielectric = index of refraction
}

pub struct Mesh {
    pub primitive: Primitive,
    pub material: Material,
    pub gpu_mesh: u32,
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
            let tex_coords = if let Some(tex_coords) = reader.read_tex_coords(0) {
                tex_coords.into_f32().map(Vec2::from).collect()
            } else {
                vec![Vec2::new(0.0, 0.0); positions.len()]
            };

            let tangents = if let Some(tangents) = reader.read_tangents() {
                tangents.map(Vec4::from).collect()
            } else {
                vec![Vec4::new(0.0, 0.0, 0.0, 0.0); positions.len()]
            };

            let colors: Vec<_> = if let Some(colors) = reader.read_colors(0) {
                colors.into_rgba_f32().map(Vec4::from).collect()
            } else {
                vec![Vec4::new(1.0, 1.0, 1.0, 1.0); positions.len()]
            };

            let mut vertices: Vec<Vertex> = vec![];

            for (i, _) in positions.iter().enumerate() {
                vertices.push(Vertex {
                    pos: positions[i].extend(0.0),
                    normal: normals[i].extend(0.0),
                    uv: tex_coords[i],
                    tangent: tangents[i],
                    color: colors[i],
                });
            }

            let material = primitive.material();
            let pbr = material.pbr_metallic_roughness();

            let diffuse_index = pbr
                .base_color_texture()
                .map_or(DEFAULT_TEXTURE_MAP, |texture| {
                    texture.texture().index() as u32
                });

            let normal_index = material
                .normal_texture()
                .map_or(DEFAULT_TEXTURE_MAP, |texture| {
                    texture.texture().index() as u32
                });

            let metallic_roughness_index = pbr
                .metallic_roughness_texture()
                .map_or(DEFAULT_TEXTURE_MAP, |texture| {
                    texture.texture().index() as u32
                });

            let occlusion_index = material
                .occlusion_texture()
                .map_or(DEFAULT_TEXTURE_MAP, |texture| {
                    texture.texture().index() as u32
                });

            let base_color_factor = pbr.base_color_factor();

            model.meshes.push(Mesh {
                primitive: Primitive::new(device, indices, vertices),
                material: Material {
                    diffuse_map: diffuse_index,
                    normal_map: normal_index,
                    metallic_roughness_map: metallic_roughness_index,
                    occlusion_map: occlusion_index,
                    base_color_factor: Vec4::from(base_color_factor),
                    material_type: MaterialType::Lambertian,
                    material_property: 0.0,
                },
                gpu_mesh: 0,
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

    for scene in gltf.scenes() {
        for node in scene.nodes() {
            load_node(device, &node, &mut model, &buffers, Mat4::IDENTITY);
        }
    }

    model
}
