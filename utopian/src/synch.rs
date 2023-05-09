use ash::vk;

use crate::Device;
use crate::Image;

pub fn global_pipeline_barrier(
    device: &Device,
    command_buffer: vk::CommandBuffer,
    prev_access: vk_sync::AccessType,
    next_access: vk_sync::AccessType,
) -> vk_sync::AccessType {
    vk_sync::cmd::pipeline_barrier(
        &device.handle,
        command_buffer,
        Some(vk_sync::GlobalBarrier {
            previous_accesses: &[prev_access],
            next_accesses: &[next_access],
        }),
        &[],
        &[],
    );

    next_access
}

pub fn image_pipeline_barrier(
    device: &Device,
    command_buffer: vk::CommandBuffer,
    image: &Image,
    prev_access: vk_sync::AccessType,
    next_access: vk_sync::AccessType,
    discard_contents: bool,
) -> vk_sync::AccessType {
    vk_sync::cmd::pipeline_barrier(
        &device.handle,
        command_buffer,
        None,
        &[],
        &[vk_sync::ImageBarrier {
            previous_accesses: &[prev_access],
            next_accesses: &[next_access],
            previous_layout: vk_sync::ImageLayout::Optimal,
            next_layout: vk_sync::ImageLayout::Optimal,
            discard_contents,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image: image.image, // Todo transition all images
            range: vk::ImageSubresourceRange::builder()
                .aspect_mask(image.desc.aspect_flags)
                .layer_count(vk::REMAINING_ARRAY_LAYERS)
                .level_count(vk::REMAINING_MIP_LEVELS)
                .build(),
        }],
    );

    next_access
}
