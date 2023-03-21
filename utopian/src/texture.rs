use ash::vk;

use crate::buffer::*;
use crate::device::*;
use crate::image::*;

pub struct Texture {
    pub image: Image,
    pub sampler: vk::Sampler,
    pub descriptor_info: vk::DescriptorImageInfo,
    pub debug_name: String,
}

impl Texture {
    pub fn load(device: &Device, path: &str) -> Texture {
        let image = match image::open(path) {
            Ok(image) => image,
            Err(_err) => panic!("Unable to load \"{}\"", path),
        };

        let image = image.to_rgba8();
        let (width, height) = (image.width(), image.height());
        let image_data = image.into_raw();

        let mut texture = Texture::create(
            device,
            Some(&image_data),
            ImageDesc::new_2d(width, height, vk::Format::R8G8B8A8_UNORM),
        );

        texture.set_debug_name(device, path);

        texture
    }

    pub fn create(device: &Device, pixels: Option<&[u8]>, image_desc: ImageDesc) -> Texture {
        let image = Image::new_from_desc(device, image_desc);

        device.execute_and_submit(|device, cb| {
            image.transition_layout(device, cb, vk::ImageLayout::TRANSFER_DST_OPTIMAL);

            if let Some(pixels) = pixels {
                let staging_buffer = Buffer::new(
                    device,
                    Some(pixels),
                    std::mem::size_of_val(pixels) as u64,
                    vk::BufferUsageFlags::TRANSFER_SRC,
                    gpu_allocator::MemoryLocation::CpuToGpu,
                );
                staging_buffer.copy_to_image(device, cb, &image);
            }

            if Image::is_depth_image_fmt(image.desc.format) {
                image.transition_layout(
                    device,
                    cb,
                    vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
                );
            } else {
                image.transition_layout(device, cb, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
            }
        });

        let sampler_info = vk::SamplerCreateInfo {
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            address_mode_u: vk::SamplerAddressMode::MIRRORED_REPEAT,
            address_mode_v: vk::SamplerAddressMode::MIRRORED_REPEAT,
            address_mode_w: vk::SamplerAddressMode::MIRRORED_REPEAT,
            max_anisotropy: 1.0,
            border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
            compare_op: vk::CompareOp::NEVER,
            min_lod: 0.0,
            max_lod: image_desc.mip_levels as f32,
            ..Default::default()
        };

        let sampler = unsafe {
            device
                .handle
                .create_sampler(&sampler_info, None)
                .expect("Unable to create sampler")
        };

        let descriptor_info = vk::DescriptorImageInfo {
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            image_view: image.image_view,
            sampler,
        };

        Texture {
            image,
            sampler,
            descriptor_info,
            debug_name: String::from("unnamed_texture"),
        }
    }

    pub fn set_debug_name(&mut self, device: &Device, debug_name: &str) {
        self.debug_name = String::from(debug_name);
        let name = std::ffi::CString::new(debug_name).unwrap();
        let name_info = vk::DebugUtilsObjectNameInfoEXT::builder()
            .object_handle(vk::Handle::as_raw(self.image.image))
            .object_name(&name)
            .object_type(vk::ObjectType::IMAGE)
            .build();
        unsafe {
            device
                .debug_utils
                .debug_utils_set_object_name(device.handle.handle(), &name_info)
                .expect("Error setting debug name for image")
        };
    }
}
