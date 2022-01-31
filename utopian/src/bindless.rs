use crate::device::*;
use ash::vk;

pub const MAX_BINDLESS_DESCRIPTOR_COUNT: usize = 512 * 1024;
pub const BINDLESS_DESCRIPTOR_INDEX: u32 = 0;

pub fn create_bindless_descriptor_set_layout(device: &Device) -> vk::DescriptorSetLayout {
    let descriptor_set_layout_binding = vk::DescriptorSetLayoutBinding::builder()
        .binding(0)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .descriptor_count(MAX_BINDLESS_DESCRIPTOR_COUNT as u32)
        .stage_flags(vk::ShaderStageFlags::ALL)
        .build();

    let binding_flags: Vec<vk::DescriptorBindingFlags> = vec![
        vk::DescriptorBindingFlags::PARTIALLY_BOUND
            | vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT,
    ];

    let mut binding_flags_create_info =
        vk::DescriptorSetLayoutBindingFlagsCreateInfo::builder().binding_flags(&binding_flags);

    let descriptor_sets_layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
        .bindings(&[descriptor_set_layout_binding])
        .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
        .push_next(&mut binding_flags_create_info)
        .build();

    let descriptor_set_layout = unsafe {
        device
            .handle
            .create_descriptor_set_layout(&descriptor_sets_layout_info, None)
            .expect("Error creating descriptor set layout")
    };

    descriptor_set_layout
}

pub fn create_bindless_descriptor_set(
    device: &Device,
    layout: vk::DescriptorSetLayout,
) -> vk::DescriptorSet {
    let descriptor_sizes = [vk::DescriptorPoolSize {
        ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
        descriptor_count: MAX_BINDLESS_DESCRIPTOR_COUNT as u32,
    }];

    let descriptor_pool_info = vk::DescriptorPoolCreateInfo::builder()
        .pool_sizes(&descriptor_sizes)
        .flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND)
        .max_sets(1);

    let descriptor_pool = unsafe {
        device
            .handle
            .create_descriptor_pool(&descriptor_pool_info, None)
            .expect("Error allocating bindless descriptor pool")
    };

    let variable_descriptor_count = MAX_BINDLESS_DESCRIPTOR_COUNT as u32;
    let mut variable_descriptor_count_allocate_info =
        vk::DescriptorSetVariableDescriptorCountAllocateInfo::builder()
            .descriptor_counts(std::slice::from_ref(&variable_descriptor_count))
            .build();

    let descriptor_set = unsafe {
        device
            .handle
            .allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::builder()
                    .descriptor_pool(descriptor_pool)
                    .set_layouts(std::slice::from_ref(&layout))
                    .push_next(&mut variable_descriptor_count_allocate_info)
                    .build(),
            )
            .expect("Error allocating bindless descriptor pool")[0]
    };

    descriptor_set
}
