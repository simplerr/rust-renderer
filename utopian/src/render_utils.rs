use ash::vk;

// Flip along Y axis https://www.saschawillems.de/blog/2019/03/29/flipping-the-vulkan-viewport/
pub fn viewport(width: u32, height: u32) -> vk::Viewport {
    vk::Viewport {
        x: 0.0,
        y: height as f32,
        width: width as f32,
        height: -(height as f32),
        min_depth: 0.0,
        max_depth: 1.0,
    }
}
