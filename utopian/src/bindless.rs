use crate::device::*;
use ash::vk;

// RTX 3070 device limit maxDescriptorSetUpdateAfterBindStorageBuffers is 512x512
// so leave 1024 to be used for non-bindless descriptors
pub const MAX_BINDLESS_DESCRIPTOR_COUNT: usize = 512 * 510;

pub fn create_bindless_descriptor_set_layout(device: &Device) -> vk::DescriptorSetLayout {
    let descriptor_set_layout_binding = vec![
        // Textures
        vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
            .descriptor_count(MAX_BINDLESS_DESCRIPTOR_COUNT as u32)
            .stage_flags(vk::ShaderStageFlags::ALL)
            .build(),
        // Vertex buffers
        vk::DescriptorSetLayoutBinding::builder()
            .binding(1)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(MAX_BINDLESS_DESCRIPTOR_COUNT as u32)
            .stage_flags(vk::ShaderStageFlags::ALL)
            .build(),
        // Index buffers
        vk::DescriptorSetLayoutBinding::builder()
            .binding(2)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(MAX_BINDLESS_DESCRIPTOR_COUNT as u32)
            .stage_flags(vk::ShaderStageFlags::ALL)
            .build(),
        // Materials (not bindless)
        vk::DescriptorSetLayoutBinding::builder()
            .binding(3)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(MAX_BINDLESS_DESCRIPTOR_COUNT as u32) // Hack: actually 1
            .stage_flags(vk::ShaderStageFlags::ALL)
            .build(),
        // Meshes (not bindless)
        vk::DescriptorSetLayoutBinding::builder()
            .binding(4)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(MAX_BINDLESS_DESCRIPTOR_COUNT as u32) // Hack: actually 1
            .stage_flags(vk::ShaderStageFlags::ALL)
            .build(),
    ];

    let binding_flags: Vec<vk::DescriptorBindingFlags> = vec![
        vk::DescriptorBindingFlags::PARTIALLY_BOUND,
        vk::DescriptorBindingFlags::PARTIALLY_BOUND,
        vk::DescriptorBindingFlags::PARTIALLY_BOUND,
        vk::DescriptorBindingFlags::PARTIALLY_BOUND,
        vk::DescriptorBindingFlags::PARTIALLY_BOUND
            | vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT,
    ];

    let mut binding_flags_create_info =
        vk::DescriptorSetLayoutBindingFlagsCreateInfo::builder().binding_flags(&binding_flags);

    let descriptor_sets_layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
        .bindings(&descriptor_set_layout_binding)
        .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
        .push_next(&mut binding_flags_create_info)
        .build();

    unsafe {
        device
            .handle
            .create_descriptor_set_layout(&descriptor_sets_layout_info, None)
            .expect("Error creating descriptor set layout")
    }
}

pub fn create_bindless_descriptor_set(
    device: &Device,
    layout: vk::DescriptorSetLayout,
) -> vk::DescriptorSet {
    let descriptor_sizes = [vk::DescriptorPoolSize {
        ty: vk::DescriptorType::SAMPLED_IMAGE,
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
