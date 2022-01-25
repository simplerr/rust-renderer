use ash::vk;

use crate::buffer::*;
use crate::device::*;
use crate::shader::*;
use crate::texture::*;

pub struct DescriptorSet {
    pub handle: vk::DescriptorSet,
    binding_map: BindingMap,
}

impl DescriptorSet {
    pub fn new(
        device: &Device,
        layout: vk::DescriptorSetLayout,
        binding_map: BindingMap,
    ) -> DescriptorSet {
        let descriptor_pool_sizes = binding_map
            .iter()
            .map(|(_, val)| {
                let descriptor_type = match val.info.ty {
                    rspirv_reflect::DescriptorType::COMBINED_IMAGE_SAMPLER => {
                        vk::DescriptorType::COMBINED_IMAGE_SAMPLER
                    }
                    rspirv_reflect::DescriptorType::SAMPLED_IMAGE => {
                        vk::DescriptorType::SAMPLED_IMAGE
                    }
                    rspirv_reflect::DescriptorType::STORAGE_IMAGE => {
                        vk::DescriptorType::STORAGE_IMAGE
                    }
                    rspirv_reflect::DescriptorType::UNIFORM_BUFFER => {
                        vk::DescriptorType::UNIFORM_BUFFER
                    }
                    rspirv_reflect::DescriptorType::STORAGE_BUFFER => {
                        vk::DescriptorType::STORAGE_BUFFER
                    }
                    _ => unimplemented!(),
                };

                vk::DescriptorPoolSize::builder()
                    .ty(descriptor_type)
                    .descriptor_count(1) // Todo: val.info.binding_count)
                    .build()
            })
            .collect::<Vec<_>>();

        let descriptor_pool_info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&descriptor_pool_sizes)
            .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET
                   | vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND)
            .max_sets(descriptor_pool_sizes.len() as u32);

        let descriptor_pool = unsafe {
            device
                .handle
                .create_descriptor_pool(&descriptor_pool_info, None)
                .expect("Error creating descriptor pool")
        };

        let descriptor_alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&[layout])
            .build();

        let descriptor_sets = unsafe {
            device
                .handle
                .allocate_descriptor_sets(&descriptor_alloc_info)
                .expect("Error allocating descriptor sets")
        };

        DescriptorSet {
            handle: descriptor_sets[0],
            binding_map,
        }
    }

    pub fn write_uniform_buffer(&self, device: &Device, name: String, buffer: &Buffer) {
        let buffer_info = vk::DescriptorBufferInfo::builder()
            .offset(0)
            .range(buffer.size)
            .buffer(buffer.buffer)
            .build();

        let binding = match self.binding_map.get(&name) {
            Some(binding) => binding,
            None => panic!("No descriptor binding found with name: \"{}\"", name),
        };

        let descriptor_writes = vk::WriteDescriptorSet::builder()
            .dst_set(self.handle)
            .dst_binding(binding.binding)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER) // todo
            .buffer_info(&[buffer_info])
            .build();

        unsafe {
            device
                .handle
                .update_descriptor_sets(&[descriptor_writes], &[])
        };
    }

    pub fn write_combined_image(&self, device: &Device, name: String, texture: &Texture) {
        let binding = match self.binding_map.get(&name) {
            Some(binding) => binding,
            None => panic!("No descriptor binding found with name: \"{}\"", name),
        };

        let descriptor_writes = vk::WriteDescriptorSet::builder()
            .dst_set(self.handle)
            .dst_binding(binding.binding)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&[texture.descriptor_info])
            .build();

        unsafe {
            device
                .handle
                .update_descriptor_sets(&[descriptor_writes], &[])
        };
    }
}
