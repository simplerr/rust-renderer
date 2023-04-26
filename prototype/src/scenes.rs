pub fn create_scene(
    renderer: &mut utopian::Renderer,
    camera: &mut utopian::Camera,
    device: &utopian::Device,
) {
    let sphere = utopian::gltf_loader::load_gltf(device, "utopian/data/models/sphere.gltf");

    renderer.add_model(
        device,
        sphere,
        glam::Mat4::from_translation(glam::Vec3::new(f32::MAX, f32::MAX, f32::MAX)),
    );

    // create_cornell_box_scene(renderer, camera, device);
    create_metal_rough_spheres(renderer, camera, device);
    // create_sponza_scene(renderer, camera, device);
    // create_cube_scene(renderer, camera, device);
}

pub fn create_metal_rough_spheres(
    renderer: &mut utopian::Renderer,
    camera: &mut utopian::Camera,
    device: &utopian::Device,
) {
    camera.set_position_target(
        glam::Vec3::new(0.0, 0.9, 2.0),
        glam::Vec3::new(0.0, 0.5, 0.0),
    );

    let spheres = utopian::gltf_loader::load_gltf(
        device,
        "prototype/data/models/MetalRoughSpheresNoTextures/glTF/MetalRoughSpheresNoTextures.gltf",
    );

    renderer.add_model(
        device,
        spheres,
        glam::Mat4::from_scale_rotation_translation(
            glam::Vec3::new(1000.0, 1000.0, 1000.0),
            glam::Quat::from_rotation_y(std::f32::consts::PI / 2.0),
            glam::Vec3::new(-10.0, 15.0, 2.5),
        ),
    );
}

pub fn create_cornell_box_scene(
    renderer: &mut utopian::Renderer,
    camera: &mut utopian::Camera,
    device: &utopian::Device,
) {
    camera.set_position_target(
        glam::Vec3::new(0.0, 0.9, 2.0),
        glam::Vec3::new(0.0, 0.5, 0.0),
    );

    let cornell_box =
        utopian::gltf_loader::load_gltf(device, "prototype/data/models/CornellBox-Original.gltf");

    let flight_helmet = utopian::gltf_loader::load_gltf(
        device,
        "prototype/data/models/FlightHelmet/glTF/FlightHelmet.gltf",
    );

    renderer.add_model(
        device,
        cornell_box,
        glam::Mat4::from_translation(glam::Vec3::new(0.0, 0.0, 0.0)),
    );

    let mut light = utopian::ModelLoader::load_cube(device);
    light.meshes[0].material.material_type = utopian::gltf_loader::MaterialType::DiffuseLight;

    renderer.add_model(
        device,
        light,
        glam::Mat4::from_scale_rotation_translation(
            glam::Vec3::new(0.50, 0.05, 0.35),
            glam::Quat::IDENTITY,
            glam::Vec3::new(0.0, 1.95, 0.0),
        ),
    );

    renderer.add_model(
        device,
        flight_helmet,
        glam::Mat4::from_translation(glam::Vec3::new(-0.33, 0.4, 0.3)),
    );
}

pub fn create_sponza_scene(
    renderer: &mut utopian::Renderer,
    camera: &mut utopian::Camera,
    device: &utopian::Device,
) {
    camera.set_position_target(
        glam::Vec3::new(-10.28, 2.10, -0.18),
        glam::Vec3::new(0.0, 0.5, 0.0),
    );

    let sponza =
        utopian::gltf_loader::load_gltf(device, "prototype/data/models/Sponza/glTF/Sponza.gltf");

    let mut metal_sphere =
        utopian::gltf_loader::load_gltf(device, "prototype/data/models/sphere.gltf");
    metal_sphere.meshes[0].material.material_type = utopian::gltf_loader::MaterialType::Metal;
    let mut dielectric_sphere =
        utopian::gltf_loader::load_gltf(device, "prototype/data/models/sphere.gltf");
    dielectric_sphere.meshes[0].material.material_type =
        utopian::gltf_loader::MaterialType::Dielectric;
    dielectric_sphere.meshes[0].material.material_property = 1.5;

    renderer.add_model(
        device,
        sponza,
        glam::Mat4::from_translation(glam::Vec3::new(0.0, 0.0, 0.0)),
    );

    let size = 0.6;
    renderer.add_model(
        device,
        metal_sphere,
        glam::Mat4::from_scale_rotation_translation(
            glam::Vec3::new(size, size, size),
            glam::Quat::IDENTITY,
            glam::Vec3::new(-3.0, 2.65, 0.7),
        ),
    );

    renderer.add_model(
        device,
        dielectric_sphere,
        glam::Mat4::from_scale_rotation_translation(
            glam::Vec3::new(size, size, size),
            glam::Quat::IDENTITY,
            glam::Vec3::new(-3.0, 0.65, 0.7),
        ),
    );
}

pub fn create_cube_scene(
    renderer: &mut utopian::Renderer,
    camera: &mut utopian::Camera,
    device: &utopian::Device,
) {
    camera.set_position_target(
        glam::Vec3::new(-2.5, 3.0, -2.5),
        glam::Vec3::new(10.0, 1.0, 10.0),
    );

    let model = utopian::model_loader::ModelLoader::load_cube(device);

    renderer.add_model(
        device,
        model,
        glam::Mat4::from_scale_rotation_translation(
            glam::Vec3::new(1000.0, 1.0, 1000.0),
            glam::Quat::IDENTITY,
            glam::Vec3::new(0.0, -5.0, 0.0),
        ),
    );

    for x in 0..10 {
        for z in 0..10 {
            let model = utopian::model_loader::ModelLoader::load_cube(device);

            renderer.add_model(
                device,
                model,
                glam::Mat4::from_scale_rotation_translation(
                    glam::Vec3::new(1.0, 1.0, 1.0),
                    glam::Quat::IDENTITY,
                    glam::Vec3::new(x as f32 * 2.0, 0.0, z as f32 * 2.0),
                ),
            );
        }
    }
}
