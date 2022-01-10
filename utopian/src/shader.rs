use ash::util::*;
use ash::vk;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fs;
use std::io::Cursor;

use rspirv_reflect;
use shaderc;

type DescriptorSetMap = BTreeMap<u32, BTreeMap<u32, rspirv_reflect::DescriptorInfo>>;

#[derive(Debug, Clone, Copy)]
pub struct Binding {
    pub set: u32,
    pub binding: u32,
}

pub struct Reflection {
    pub descriptor_set_reflections: DescriptorSetMap,
    pub push_constant_reflections: Vec<rspirv_reflect::PushConstantInfo>,
    pub binding_mappings: HashMap<String, Binding>,
}

impl Reflection {
    pub fn new(shader_stages: &[&[u8]]) -> Reflection {
        let mut descriptor_sets_combined: DescriptorSetMap = BTreeMap::new();
        let mut push_constant_ranges: Vec<rspirv_reflect::PushConstantInfo> = vec![];

        // Combine reflection information from all shader stages
        for shader_stage in shader_stages {
            let stage_reflection = rspirv_reflect::Reflection::new_from_spirv(shader_stage)
                .expect("Shader reflection failed!");

            let descriptor_sets = stage_reflection.get_descriptor_sets().unwrap();

            for (set, descriptor_set) in descriptor_sets {
                if let Some(existing_descriptor_set) = descriptor_sets_combined.get_mut(&set) {
                    for (binding, descriptor) in descriptor_set {
                        if let Some(existing_descriptor) = existing_descriptor_set.get(&binding) {
                            assert!(
                                descriptor == *existing_descriptor,
                                "Set: {} binding: {} inconsistent between shader stages",
                                set,
                                binding
                            );
                        } else {
                            existing_descriptor_set.insert(binding, descriptor);
                            println!("Set: {} binding: {} does not exist, adding!", set, binding);
                        }
                    }
                } else {
                    descriptor_sets_combined.insert(set, descriptor_set);
                }
            }

            if let Some(push_constant_reflection) =
                stage_reflection.get_push_constant_range().unwrap()
            {
                push_constant_ranges.push(push_constant_reflection);
            }
        }

        // Retrieve binding and set mappings
        let binding_mappings: HashMap<String, Binding> = descriptor_sets_combined
            .iter()
            .filter_map(|(set_key, set_val)| {
                let bindings: HashMap<String, Binding> = set_val
                    .iter()
                    .filter_map(|(binding_key, binding_val)| {
                        Some((
                            binding_val.name.clone(),
                            Binding {
                                set: *set_key,
                                binding: *binding_key,
                            },
                        ))
                    })
                    .collect();

                Some(bindings)
            })
            .flatten()
            .collect();

        Reflection {
            descriptor_set_reflections: descriptor_sets_combined,
            push_constant_reflections: push_constant_ranges,
            binding_mappings,
        }
    }

    pub fn get_binding(&self, name: &str) -> Binding {
        match self.binding_mappings.get(name) {
            Some(binding) => *binding,
            None => panic!("Binding with \"{}\" name not available", name),
        }
    }
}

pub fn compile_glsl_shader(path: &str) -> shaderc::CompilationArtifact {
    let source = &fs::read_to_string(path).expect("Error reading shader file")[..];

    let shader_kind = if path.ends_with(".vert") {
        shaderc::ShaderKind::Vertex
    } else if path.ends_with(".frag") {
        shaderc::ShaderKind::Fragment
    } else {
        panic!("Unsupported shader extension");
    };

    let mut compiler = shaderc::Compiler::new().unwrap();
    let mut options = shaderc::CompileOptions::new().unwrap();
    options.add_macro_definition("EP", Some("main"));
    let binary_result = compiler
        .compile_into_spirv(source, shader_kind, "shader.glsl", "main", Some(&options))
        .unwrap();

    assert_eq!(Some(&0x07230203), binary_result.as_binary().first());

    let text_result = compiler
        .compile_into_spirv_assembly(source, shader_kind, "shader.glsl", "main", Some(&options))
        .unwrap();

    assert!(text_result.as_text().starts_with("; SPIR-V\n"));

    //println!("{}", text_result.as_text());

    binary_result
}

pub fn create_layouts_from_reflection(
    device: &ash::Device,
    reflection: &Reflection,
) -> (
    vk::PipelineLayout,
    Vec<vk::DescriptorSetLayout>,
    Vec<vk::PushConstantRange>,
) {
    let descriptor_sets_layouts: Vec<vk::DescriptorSetLayout> = reflection
        .descriptor_set_reflections
        .iter()
        .map(|(_slot, descriptor_set)| {
            let descriptor_set_layout_bindings: Vec<vk::DescriptorSetLayoutBinding> =
                descriptor_set
                    .iter()
                    .map(|(binding, descriptor_info)| {
                        println!("{:?}", descriptor_info.name);
                        let descriptor_type = match descriptor_info.ty {
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
                            _ => panic!("Unsupported descriptor type"),
                        };

                        let descriptor_set_layout_binding =
                            vk::DescriptorSetLayoutBinding::builder()
                                .binding(*binding)
                                .descriptor_type(descriptor_type)
                                .descriptor_count(1) // descriptor_info.binding_count
                                .stage_flags(vk::ShaderStageFlags::ALL)
                                .build();

                        descriptor_set_layout_binding
                    })
                    .collect();

            let descriptor_sets_layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
                .bindings(&descriptor_set_layout_bindings)
                .build();

            let descriptor_set_layout = unsafe {
                device
                    .create_descriptor_set_layout(&descriptor_sets_layout_info, None)
                    .expect("Error creating descriptor set layout")
            };

            descriptor_set_layout
        })
        .collect();

    let mut push_constant_ranges: Vec<vk::PushConstantRange> = vec![];

    for push_constant_reflection in &reflection.push_constant_reflections {
        push_constant_ranges.push(
            vk::PushConstantRange::builder()
                .size(push_constant_reflection.size)
                .offset(push_constant_reflection.offset)
                .stage_flags(vk::ShaderStageFlags::ALL)
                .build(),
        );
    }

    let pipeline_layout_create_info: vk::PipelineLayoutCreateInfoBuilder;

    if push_constant_ranges.len() > 0 {
        pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&descriptor_sets_layouts)
            .push_constant_ranges(&push_constant_ranges);
    } else {
        pipeline_layout_create_info =
            vk::PipelineLayoutCreateInfo::builder().set_layouts(&descriptor_sets_layouts);
    }

    let pipeline_layout = unsafe {
        device
            .create_pipeline_layout(&pipeline_layout_create_info, None)
            .expect("Error creating pipeline layout")
    };

    (
        pipeline_layout,
        descriptor_sets_layouts,
        push_constant_ranges,
    )
}

pub fn create_shader_module(mut spv_file: Cursor<&[u8]>, device: &ash::Device) -> vk::ShaderModule {
    let shader_code = read_spv(&mut spv_file).expect("Failed to read shader spv file");
    let shader_info = vk::ShaderModuleCreateInfo::builder().code(&shader_code);

    let shader_module = unsafe {
        device
            .create_shader_module(&shader_info, None)
            .expect("Error creating shader module")
    };

    shader_module
}
