use crate::*;
use ash::vk;
use glam::Vec4;

pub const MAX_NUM_GPU_MATERIALS: usize = 1024;
pub const MAX_NUM_GPU_MESHES: usize = 1024;

/// All shaders share these common descriptor set indexes
/// Every custom shader descriptor set needs to be starting from index 3
pub const DESCRIPTOR_SET_INDEX_BINDLESS: u32 = 0;
pub const DESCRIPTOR_SET_INDEX_VIEW: u32 = 1;
pub const DESCRIPTOR_SET_INDEX_INPUT_TEXTURES: u32 = 2;

pub struct ModelInstance {
    pub model: Model,
    pub transform: glam::Mat4,
}

pub struct Renderer {
    pub bindless_descriptor_set_layout: vk::DescriptorSetLayout,
    pub bindless_descriptor_set: vk::DescriptorSet,
    pub instances: Vec<ModelInstance>,
    pub gpu_materials_buffer: Buffer,
    pub gpu_meshes_buffer: Buffer,
    gpu_materials: Vec<GpuMaterial>,
    gpu_meshes: Vec<GpuMesh>,
    default_diffuse_map_index: u32,
    default_normal_map_index: u32,
    default_occlusion_map_index: u32,
    default_metallic_roughness_map_index: u32,
    next_bindless_image_index: u32,
    next_bindless_vertex_buffer_index: u32,
    next_bindless_index_buffer_index: u32,

    // This should probably be somewhere else
    pub need_environment_map_update: bool,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Copy)]
#[repr(C)]
pub struct ViewUniformData {
    pub view: glam::Mat4,
    pub projection: glam::Mat4,
    pub inverse_view: glam::Mat4,
    pub inverse_projection: glam::Mat4,
    pub eye_pos: glam::Vec3,
    pub samples_per_frame: u32,
    pub sun_dir: glam::Vec3,
    pub total_samples: u32,
    pub num_bounces: u32,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub pad1: u32,

    // render settings
    pub shadows_enabled: u32,
    pub ssao_enabled: u32,
    pub fxaa_enabled: u32,
    pub cubemap_enabled: u32,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct GpuMaterial {
    diffuse_map: u32,
    normal_map: u32,
    metallic_roughness_map: u32,
    occlusion_map: u32,
    base_color_factor: Vec4,

    // Ray tracing properties
    // x = type (0 = lambertian, 1 = metal, 2 = dielectric, 3 = diffuse light)
    // y = metal -> fuzz, dielectric -> index of refractions
    raytrace_properties: Vec4,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct GpuMesh {
    vertex_buffer: u32,
    index_buffer: u32,
    material: u32,
}

impl Renderer {
    pub fn new(device: &Device) -> Renderer {
        let bindless_descriptor_set_layout = create_bindless_descriptor_set_layout(device);
        let bindless_descriptor_set =
            create_bindless_descriptor_set(device, bindless_descriptor_set_layout);

        let gpu_materials_buffer = Buffer::new::<u8>(
            device,
            None,
            (MAX_NUM_GPU_MATERIALS * std::mem::size_of::<GpuMaterial>()) as u64,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            gpu_allocator::MemoryLocation::CpuToGpu,
        );

        let gpu_meshes_buffer = Buffer::new::<u8>(
            device,
            None,
            (MAX_NUM_GPU_MESHES * std::mem::size_of::<GpuMesh>()) as u64,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            gpu_allocator::MemoryLocation::CpuToGpu,
        );

        DescriptorSet::write_storage_buffer(
            device,
            bindless_descriptor_set,
            3,
            &gpu_materials_buffer,
        );
        DescriptorSet::write_storage_buffer(device, bindless_descriptor_set, 4, &gpu_meshes_buffer);

        Renderer {
            bindless_descriptor_set_layout,
            bindless_descriptor_set,
            instances: vec![],
            gpu_materials: vec![],
            gpu_meshes: vec![],
            gpu_meshes_buffer,
            gpu_materials_buffer,
            next_bindless_image_index: 0,
            next_bindless_vertex_buffer_index: 0,
            next_bindless_index_buffer_index: 0,
            default_diffuse_map_index: 0,
            default_normal_map_index: 0,
            default_occlusion_map_index: 0,
            default_metallic_roughness_map_index: 0,
            need_environment_map_update: true,
        }
    }

    pub fn initialize(&mut self, device: &Device) {
        let default_diffuse_map =
            Texture::load(device, "utopian/data/textures/defaults/checker.jpg");
        let default_normal_map =
            Texture::load(device, "utopian/data/textures/defaults/flat_normal_map.png");
        let default_occlusion_map =
            Texture::load(device, "utopian/data/textures/defaults/white_texture.png");
        let default_metallic_roughness_map =
            Texture::load(device, "utopian/data/textures/defaults/white_texture.png");

        self.default_diffuse_map_index = self.add_bindless_texture(device, &default_diffuse_map);
        self.default_normal_map_index = self.add_bindless_texture(device, &default_normal_map);
        self.default_occlusion_map_index =
            self.add_bindless_texture(device, &default_occlusion_map);
        self.default_metallic_roughness_map_index =
            self.add_bindless_texture(device, &default_metallic_roughness_map);
    }

    pub fn add_model(&mut self, device: &Device, mut model: Model, transform: glam::Mat4) {
        // Add the images from the new model to the bindless descriptor set and
        // also update the mappings for each primitive to be indexes corresponding
        // to the ordering in the bindless descriptor set texture array.
        for mesh in &mut model.meshes {
            let diffuse_bindless_index = match mesh.material.diffuse_map {
                DEFAULT_TEXTURE_MAP => self.default_diffuse_map_index,
                _ => self.add_bindless_texture(
                    device,
                    &model.textures[mesh.material.diffuse_map as usize],
                ),
            };

            let normal_bindless_index = match mesh.material.normal_map {
                DEFAULT_TEXTURE_MAP => self.default_normal_map_index,
                _ => self.add_bindless_texture(
                    device,
                    &model.textures[mesh.material.normal_map as usize],
                ),
            };

            let metallic_roughness_bindless_index = match mesh.material.metallic_roughness_map {
                DEFAULT_TEXTURE_MAP => self.default_metallic_roughness_map_index,
                _ => self.add_bindless_texture(
                    device,
                    &model.textures[mesh.material.metallic_roughness_map as usize],
                ),
            };

            let occlusion_bindless_index = match mesh.material.occlusion_map {
                DEFAULT_TEXTURE_MAP => self.default_occlusion_map_index,
                _ => self.add_bindless_texture(
                    device,
                    &model.textures[mesh.material.occlusion_map as usize],
                ),
            };

            let vertex_buffer_bindless_idx =
                self.add_bindless_vertex_buffer(device, &mesh.primitive.vertex_buffer);
            let index_buffer_bindless_idx =
                self.add_bindless_index_buffer(device, &mesh.primitive.index_buffer);

            let material_index = self.add_material(GpuMaterial {
                diffuse_map: diffuse_bindless_index,
                normal_map: normal_bindless_index,
                metallic_roughness_map: metallic_roughness_bindless_index,
                occlusion_map: occlusion_bindless_index,
                base_color_factor: mesh.material.base_color_factor,
                raytrace_properties: Vec4::new(
                    mesh.material.material_type as u32 as f32,
                    mesh.material.material_property,
                    0.0,
                    0.0,
                ),
            });

            let mesh_index = self.add_mesh(GpuMesh {
                vertex_buffer: vertex_buffer_bindless_idx,
                index_buffer: index_buffer_bindless_idx,
                material: material_index,
            });

            mesh.gpu_mesh = mesh_index;
        }

        // println!("{:?}", self.gpu_meshes);
        // println!("{:?}", self.gpu_materials);

        self.gpu_meshes_buffer
            .update_memory(device, self.gpu_meshes.as_slice());
        self.gpu_materials_buffer
            .update_memory(device, self.gpu_materials.as_slice());

        self.instances.push(ModelInstance { model, transform });
    }

    fn add_bindless_texture(&mut self, device: &Device, texture: &Texture) -> u32 {
        let new_image_index = self.next_bindless_image_index;

        let descriptor_write = vk::WriteDescriptorSet::builder()
            .dst_set(self.bindless_descriptor_set)
            .dst_binding(0)
            .dst_array_element(new_image_index)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(std::slice::from_ref(&texture.descriptor_info))
            .build();

        unsafe {
            device
                .handle
                .update_descriptor_sets(std::slice::from_ref(&descriptor_write), &[])
        };

        self.next_bindless_image_index += 1;

        new_image_index
    }

    fn add_bindless_vertex_buffer(&mut self, device: &Device, buffer: &Buffer) -> u32 {
        let new_buffer_index = self.next_bindless_vertex_buffer_index;

        let buffer_info = vk::DescriptorBufferInfo::builder()
            .buffer(buffer.buffer)
            .range(buffer.size)
            .build();

        let descriptor_write = vk::WriteDescriptorSet::builder()
            .dst_set(self.bindless_descriptor_set)
            .dst_binding(1)
            .dst_array_element(new_buffer_index)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(std::slice::from_ref(&buffer_info))
            .build();

        unsafe {
            device
                .handle
                .update_descriptor_sets(std::slice::from_ref(&descriptor_write), &[])
        };

        self.next_bindless_vertex_buffer_index += 1;

        new_buffer_index
    }

    fn add_bindless_index_buffer(&mut self, device: &Device, buffer: &Buffer) -> u32 {
        let new_buffer_index = self.next_bindless_index_buffer_index;

        let buffer_info = vk::DescriptorBufferInfo::builder()
            .buffer(buffer.buffer)
            .range(buffer.size)
            .build();

        let descriptor_write = vk::WriteDescriptorSet::builder()
            .dst_set(self.bindless_descriptor_set)
            .dst_binding(2)
            .dst_array_element(new_buffer_index)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(std::slice::from_ref(&buffer_info))
            .build();

        unsafe {
            device
                .handle
                .update_descriptor_sets(std::slice::from_ref(&descriptor_write), &[])
        };

        self.next_bindless_index_buffer_index += 1;

        new_buffer_index
    }

    fn add_material(&mut self, gpu_material: GpuMaterial) -> u32 {
        let material_index = self.gpu_materials.len() as u32;
        self.gpu_materials.push(gpu_material);

        material_index
    }

    fn add_mesh(&mut self, gpu_mesh: GpuMesh) -> u32 {
        let gpu_index = self.gpu_meshes.len() as u32;
        self.gpu_meshes.push(gpu_mesh);

        gpu_index
    }

    pub fn draw_meshes(
        &self,
        device: &Device,
        command_buffer: vk::CommandBuffer,
        pipeline_layout: vk::PipelineLayout,
    ) {
        #[allow(dead_code)]
        struct PushConstants {
            world: glam::Mat4,
            color: glam::Vec4,
            mesh_index: u32,
            pad: [u32; 3],
        }

        unsafe {
            for instance in &self.instances {
                for (i, mesh) in instance.model.meshes.iter().enumerate() {
                    let push_data = PushConstants {
                        world: instance.transform * instance.model.transforms[i],
                        color: glam::Vec4::new(1.0, 0.5, 0.2, 1.0),
                        mesh_index: mesh.gpu_mesh,
                        pad: [0; 3],
                    };

                    device.handle.cmd_push_constants(
                        command_buffer,
                        pipeline_layout,
                        vk::ShaderStageFlags::ALL,
                        0,
                        std::slice::from_raw_parts(
                            &push_data as *const _ as *const u8,
                            std::mem::size_of_val(&push_data),
                        ),
                    );

                    device.handle.cmd_bind_vertex_buffers(
                        command_buffer,
                        0,
                        &[mesh.primitive.vertex_buffer.buffer],
                        &[0],
                    );
                    device.handle.cmd_bind_index_buffer(
                        command_buffer,
                        mesh.primitive.index_buffer.buffer,
                        0,
                        vk::IndexType::UINT32,
                    );
                    device.handle.cmd_draw_indexed(
                        command_buffer,
                        mesh.primitive.indices.len() as u32,
                        1,
                        0,
                        0,
                        1,
                    );
                }
            }
        }
    }
}
