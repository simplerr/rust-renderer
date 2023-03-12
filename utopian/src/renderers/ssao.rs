pub fn setup_ssao_pass(
   device: &crate::Device,
   graph: &mut crate::Graph,
   base: &crate::VulkanBase,
   gbuffer_position: crate::TextureId,
   gbuffer_normal: crate::TextureId,
   ssao_output: crate::TextureId,
) {
   let pipeline_handle = graph.create_pipeline(crate::PipelineDesc {
       vertex_path: "utopian/shaders/common/fullscreen.vert",
       fragment_path: "utopian/shaders/ssao/ssao.frag",
       vertex_input_binding_descriptions: vec![],
       vertex_input_attribute_descriptions: vec![],
       color_attachment_formats: vec![graph.resources.textures[ssao_output]
           .texture
           .image
           .format()],
       depth_stencil_attachment_format: base.depth_image.format(),
   });

   let radius = 0.1;
   let bias = 0.0;

   let radius_bias = glam::Vec4::new(radius, bias, 0.0, 0.0);

   graph
       .add_pass(String::from("ssao_pass"), pipeline_handle)
       .read(gbuffer_position)
       .read(gbuffer_normal)
       .write(ssao_output)
       .uniforms("settings_ubo", &(radius_bias))
       .render(
           move |device, command_buffer, _renderer, _pass, _resources| unsafe {
               device.handle.cmd_draw(command_buffer, 3, 1, 0, 0);
           },
       )
       .build(&device, graph);

   // It is common to also have a blur pass for SSAO which can be added here.
   // The SSAO effect looks decent without it, but it should be added here in the future.
}
