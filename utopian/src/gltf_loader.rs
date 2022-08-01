use glam::{Mat3, Mat4, Vec2, Vec3, Vec4};

use crate::device::*;
use crate::primitive::*;
use crate::texture::*;

pub const DEFAULT_TEXTURE_MAP: u32 = u32::MAX;

#[derive(Copy, Clone, Debug)]
pub enum MaterialType {
    Lambertian = 0,
    Metal = 1,
    Dielectric = 2,
    DiffuseLight = 3,
}

// Note: indexes into the Model specific texture array,
// not bindless indexes.
#[derive(Debug)]
pub struct Material {
    pub diffuse_map: u32,
    pub normal_map: u32,
    pub metallic_roughness_map: u32,
    pub occlusion_map: u32,
    pub base_color_factor: Vec4,

    // Ray tracing properties
    pub material_type: MaterialType, // 0 = lambertian, 1 = metal, 2 = dielectric, 3 = diffuse light
    pub material_property: f32,      // metal = fuzz, dielectric = index of refraction
}

#[derive(Debug)]
pub struct Mesh {
    //pub primitive: Primitive,
    pub first_vertex: u32,
    pub first_index: u32,
    pub index_count: u32,
    pub material: Material,
    pub gpu_mesh: u32,
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub textures: Vec<Texture>,
    pub transforms: Vec<Mat4>,
    pub primitive: Primitive,
}

pub fn load_node(
    device: &Device,
    node: &gltf::Node,
    buffers: &[gltf::buffer::Data],
    parent_transform: Mat4,
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    transforms: &mut Vec<Mat4>,
    meshes: &mut Vec<Mesh>,
) {
    let node_transform =
        parent_transform * glam::Mat4::from_cols_array_2d(&node.transform().matrix());

    for child in node.children() {
        load_node(
            device,
            &child,
            buffers,
            node_transform,
            vertices,
            indices,
            transforms,
            meshes,
        );
    }

    if let Some(mesh) = node.mesh() {
        let primitives = mesh.primitives();

        for primitive in primitives {
            let reader = primitive.reader(|i| Some(&buffers[i.index()]));

            let new_indices: Vec<_> = reader.read_indices().unwrap().into_u32().collect();
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

            let mut new_vertices: Vec<Vertex> = vec![];

            for (i, _) in positions.iter().enumerate() {
                new_vertices.push(Vertex {
                    // pos: positions[i].extend(0.0),
                    // normal: normals[i].extend(0.0),
                    pos: node_transform * positions[i].extend(1.0),
                    //normal: (Mat3::from_mat4(Mat4::transpose(&Mat4::inverse(&node_transform))) * normals[i]).normalize().extend(0.0),
                    normal: node_transform * normals[i].extend(0.0).normalize(),
                    //tangent: (node_transform * tangents[i].truncate().extend(0.0)).normalize(),
                    uv: tex_coords[i],
                    tangent: tangents[i],
                    color: colors[i],
                    material_index: 0,
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

            let first_vertex = vertices.len() as u32;
            let first_index = indices.len() as u32;
            let index_count = new_indices.len() as u32;

            let mut new_indices = new_indices.into_iter().map(|x| x + first_vertex).collect();

            vertices.append(&mut new_vertices);
            indices.append(&mut new_indices);

            meshes.push(Mesh {
                first_vertex,
                first_index,
                index_count,
                //primitive: Primitive::new(device, indices, vertices),
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

            transforms.push(node_transform);
        }
    }
}

pub fn load_gltf(device: &Device, path: &str) -> Model {
    let (gltf, buffers, mut images) = match gltf::import(path) {
        Ok(result) => (result),
        Err(err) => panic!("Loading model {} failed with error: {}", path, err),
    };

    let mut vertices: Vec<Vertex> = vec![];
    let mut indices: Vec<u32> = vec![];
    let mut meshes: Vec<Mesh> = vec![];
    let mut textures: Vec<Texture> = vec![];
    let mut transforms: Vec<Mat4> = vec![];

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

        textures.push(texture);
    }

    for scene in gltf.scenes() {
        for node in scene.nodes() {
            load_node(
                device,
                &node,
                &buffers,
                Mat4::IDENTITY,
                &mut vertices,
                &mut indices,
                &mut transforms,
                &mut meshes,
            );
        }
    }

    // println!(
    //     "indices: {:?}, vertices: {:?}, meshes: {:#?}, transforms: {:?}, textures: {:?}",
    //     indices.len(),
    //     vertices.len(),
    //     meshes,
    //     transforms.len(),
    //     textures.len()
    // );

    Model {
        primitive: Primitive::new(device, indices, vertices),
        meshes,
        transforms,
        textures,
    }
}
