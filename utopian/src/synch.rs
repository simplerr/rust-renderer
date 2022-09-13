use ash::vk;

use crate::Device;
use crate::Image;

pub fn image_pipeline_barrier(
    device: &Device,
    command_buffer: vk::CommandBuffer,
    image: &Image,
    prev_access: vk_sync::AccessType,
    next_access: vk_sync::AccessType,
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
            discard_contents: false,
            src_queue_family_index: 0,
            dst_queue_family_index: 0,
            image: image.image, // Todo transition all images
            range: vk::ImageSubresourceRange::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .layer_count(1)
                .level_count(1)
                .build(),
        }],
    );

    next_access
}
