use ash::vk;

use crate::device::*;
use crate::RenderPass;
use crate::Renderer;

pub struct Graph {
    pub passes: Vec<RenderPass>,
}

impl Graph {
    pub fn add_pass(&mut self, pass: RenderPass) {
        self.passes.push(pass);
    }

    pub fn render_passes(
        &self,
        device: &Device,
        command_buffer: vk::CommandBuffer,
        renderer: &Renderer,
    ) {
        for pass in &self.passes {
            if let Some(render_func) = &pass.render_func {
                render_func(device, command_buffer, renderer, pass);
            }
        }
    }
}
