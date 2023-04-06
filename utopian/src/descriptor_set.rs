use ash::vk;

use crate::buffer::*;
use crate::device::*;
use crate::image::*;
use crate::shader::*;
use crate::texture::*;

pub struct DescriptorSet {
    pub handle: vk::DescriptorSet,
    pub pool: vk::DescriptorPool,
    binding_map: BindingMap,
}

pub enum DescriptorIdentifier {
    Name(String),
    Index(u32),
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
                    rspirv_reflect::DescriptorType::ACCELERATION_STRUCTURE_KHR => {
                        vk::DescriptorType::ACCELERATION_STRUCTURE_KHR
                    }
                    _ => unimplemented!(),
                };

                vk::DescriptorPoolSize::builder()
                    .ty(descriptor_type)
                    .descriptor_count(1) // Todo: val.info.binding_count)
                    .build()
            })
            .collect::<Vec<_>>();

        // Todo: Every descriptor should not have its own pool
        let descriptor_pool = {
            puffin::profile_scope!("create_descriptor_pool");
            let descriptor_pool_info = vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(&descriptor_pool_sizes)
                .flags(
                    vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET
                        | vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND,
                )
                .max_sets(descriptor_pool_sizes.len() as u32);

            let descriptor_pool = unsafe {
                device
                    .handle
                    .create_descriptor_pool(&descriptor_pool_info, None)
                    .expect("Error creating descriptor pool")
            };
            descriptor_pool
        };

        let descriptor_sets = {
            puffin::profile_scope!("allocate_descriptor_set");
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
            descriptor_sets
        };

        DescriptorSet {
            handle: descriptor_sets[0],
            pool: descriptor_pool,
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

    pub fn write_storage_buffer(
        &self,
        device: &Device,
        name: DescriptorIdentifier,
        buffer: &Buffer,
    ) {
        let binding = match name {
            DescriptorIdentifier::Name(name) => match self.binding_map.get(&name) {
                Some(binding) => binding.binding,
                None => panic!("No descriptor binding found with name: \"{}\"", name),
            },
            DescriptorIdentifier::Index(index) => index,
        };

        let buffer_info = vk::DescriptorBufferInfo::builder()
            .offset(0)
            .range(buffer.size)
            .buffer(buffer.buffer)
            .build();

        let descriptor_writes = vk::WriteDescriptorSet::builder()
            .dst_set(self.handle)
            .dst_binding(binding)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER) // todo
            .buffer_info(&[buffer_info])
            .build();

        unsafe {
            device
                .handle
                .update_descriptor_sets(&[descriptor_writes], &[])
        };
    }

    pub fn write_combined_image(
        &self,
        device: &Device,
        name: DescriptorIdentifier,
        texture: &Texture,
    ) {
        let binding = match name {
            DescriptorIdentifier::Name(name) => match self.binding_map.get(&name) {
                Some(binding) => binding.binding,
                None => panic!("No descriptor binding found with name: \"{}\"", name),
            },
            DescriptorIdentifier::Index(index) => index,
        };

        let descriptor_writes = vk::WriteDescriptorSet::builder()
            .dst_set(self.handle)
            .dst_binding(binding)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&[texture.descriptor_info])
            .build();

        unsafe {
            device
                .handle
                .update_descriptor_sets(&[descriptor_writes], &[])
        };
    }

    pub fn write_storage_image(&self, device: &Device, name: DescriptorIdentifier, image: &Image) {
        let binding = match name {
            DescriptorIdentifier::Name(name) => match self.binding_map.get(&name) {
                Some(binding) => binding.binding,
                None => panic!("No descriptor binding found with name: \"{}\"", name),
            },
            DescriptorIdentifier::Index(index) => index,
        };

        let descriptor_info = vk::DescriptorImageInfo {
            image_layout: vk::ImageLayout::GENERAL,
            image_view: image.image_view,
            sampler: vk::Sampler::null(),
        };

        let descriptor_writes = vk::WriteDescriptorSet::builder()
            .dst_set(self.handle)
            .dst_binding(binding)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .image_info(&[descriptor_info])
            .build();

        unsafe {
            device
                .handle
                .update_descriptor_sets(&[descriptor_writes], &[])
        };
    }

    pub fn write_acceleration_structure(
        &self,
        device: &Device,
        name: String,
        acceleration_structure: vk::AccelerationStructureKHR,
    ) {
        let binding = match self.binding_map.get(&name) {
            Some(binding) => binding,
            None => panic!("No descriptor binding found with name: \"{}\"", name),
        };

        let mut descriptor_info = vk::WriteDescriptorSetAccelerationStructureKHR::builder()
            .acceleration_structures(std::slice::from_ref(&acceleration_structure))
            .build();

        let mut descriptor_writes = vk::WriteDescriptorSet::builder()
            .dst_set(self.handle)
            .dst_binding(binding.binding)
            .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
            .push_next(&mut descriptor_info)
            .build();
        descriptor_writes.descriptor_count = 1; // Not set for acceleration structures

        unsafe {
            device
                .handle
                .update_descriptor_sets(&[descriptor_writes], &[])
        };
    }

    // Note: this is unlike the other functions above an associate function
    // specifically used in Renderer for writing the mesh and material buffers.
    // Either this should be a metho and the bindless descriptor set shall be
    // an object of this type or all function above should be associate functions
    // as well. TBD.
    pub fn write_raw_storage_buffer(
        device: &Device,
        descriptor_set: vk::DescriptorSet,
        binding: u32,
        buffer: &Buffer,
    ) {
        let buffer_info = vk::DescriptorBufferInfo::builder()
            .buffer(buffer.buffer)
            .range(buffer.size)
            .build();

        let descriptor_write = vk::WriteDescriptorSet::builder()
            .dst_set(descriptor_set)
            .dst_binding(binding)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(std::slice::from_ref(&buffer_info))
            .build();

        unsafe {
            device
                .handle
                .update_descriptor_sets(std::slice::from_ref(&descriptor_write), &[])
        };
    }

    pub fn get_set_index(&self) -> u32 {
        self.binding_map
            .iter()
            .next()
            .expect("Empty DescriptorSet")
            .1
            .set
    }
}
