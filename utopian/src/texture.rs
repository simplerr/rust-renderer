use ash::vk;

use crate::buffer::*;
use crate::device::*;
use crate::image::*;

pub struct Texture {
    pub image: Image,
    pub sampler: vk::Sampler,
    pub descriptor_info: vk::DescriptorImageInfo,
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

        Texture::create(device, &image_data, width, height)
    }

    pub fn create(device: &Device, pixels: &[u8], width: u32, height: u32) -> Texture {
        let staging_buffer = Buffer::new(
            device,
            pixels,
            std::mem::size_of_val(pixels) as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
        );

        let image = Image::new(
            device,
            width,
            height,
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            vk::ImageAspectFlags::COLOR,
        );

        device.execute_and_submit(|device, cb| {
            image.transition_layout(device, cb, vk::ImageLayout::TRANSFER_DST_OPTIMAL);

            staging_buffer.copy_to_image(device, cb, &image);

            image.transition_layout(device, cb, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
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
        }
    }
}
