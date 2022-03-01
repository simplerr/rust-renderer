use crate::*;
use ash::vk;

pub struct ModelInstance {
    pub model: Model,
    pub transform: glam::Mat4,
}

pub struct Renderer {
    pub bindless_descriptor_set_layout: vk::DescriptorSetLayout,
    pub bindless_descriptor_set: vk::DescriptorSet,
    pub instances: Vec<ModelInstance>,
    default_diffuse_map_index: u32,
    default_normal_map_index: u32,
    default_occlusion_map_index: u32,
    default_metallic_roughness_map_index: u32,
    next_bindless_image_index: u32,
    next_bindless_vertex_buffer_index: u32,
    next_bindless_index_buffer_index: u32,
}

impl Renderer {
    pub fn new(device: &Device) -> Renderer {
        let bindless_descriptor_set_layout = create_bindless_descriptor_set_layout(&device);
        let bindless_descriptor_set =
            create_bindless_descriptor_set(&device, bindless_descriptor_set_layout);

        Renderer {
            bindless_descriptor_set_layout,
            bindless_descriptor_set,
            next_bindless_image_index: 0,
            next_bindless_vertex_buffer_index: 0,
            next_bindless_index_buffer_index: 0,
            instances: vec![],
            default_diffuse_map_index: 0,
            default_normal_map_index: 0,
            default_occlusion_map_index: 0,
            default_metallic_roughness_map_index: 0,
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

        self.default_diffuse_map_index = self.add_bindless_texture(&device, &default_diffuse_map);
        self.default_normal_map_index = self.add_bindless_texture(&device, &default_normal_map);
        self.default_occlusion_map_index =
            self.add_bindless_texture(&device, &default_occlusion_map);
        self.default_metallic_roughness_map_index =
            self.add_bindless_texture(&device, &default_metallic_roughness_map);
    }

    pub fn add_model(&mut self, device: &Device, mut model: Model, transform: glam::Mat4) {
        // Add the images from the new model to the bindless descriptor set and
        // also update the mappings for each primitive to be indexes corresponding
        // to the ordering in the bindless descriptor set texture array.
        // Note: After this remapping the indexes no longer corresponds to the
        // images in model.textures[].
        for mesh in &mut model.meshes {
            let diffuse_bindless_index = match mesh.material.diffuse_map {
                DEFAULT_TEXTURE_MAP => self.default_diffuse_map_index,
                _ => self.add_bindless_texture(
                    &device,
                    &model.textures[mesh.material.diffuse_map as usize],
                ),
            };

            let normal_bindless_index = match mesh.material.normal_map {
                DEFAULT_TEXTURE_MAP => self.default_normal_map_index,
                _ => self.add_bindless_texture(
                    &device,
                    &model.textures[mesh.material.normal_map as usize],
                ),
            };

            let metallic_roughness_bindless_index = match mesh.material.metallic_roughness_map {
                DEFAULT_TEXTURE_MAP => self.default_metallic_roughness_map_index,
                _ => self.add_bindless_texture(
                    &device,
                    &model.textures[mesh.material.metallic_roughness_map as usize],
                ),
            };

            let occlusion_bindless_index = match mesh.material.occlusion_map {
                DEFAULT_TEXTURE_MAP => self.default_occlusion_map_index,
                _ => self.add_bindless_texture(
                    &device,
                    &model.textures[mesh.material.occlusion_map as usize],
                ),
            };

            mesh.material.diffuse_map = diffuse_bindless_index;
            mesh.material.normal_map = normal_bindless_index;
            mesh.material.metallic_roughness_map = metallic_roughness_bindless_index;
            mesh.material.occlusion_map = occlusion_bindless_index;

            mesh.vertex_buffer_bindless_idx =
                self.add_bindless_vertex_buffer(device, &mesh.primitive.vertex_buffer);
            mesh.index_buffer_bindless_idx =
                self.add_bindless_index_buffer(device, &mesh.primitive.index_buffer);
        }

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
}
