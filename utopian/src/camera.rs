use dolly::prelude::*;
use glam::{Mat3, Mat4, Quat, Vec3};

use crate::Input;

pub struct Camera {
    camera_rig: CameraRig,
    fov_degrees: f32,
    aspect_ratio: f32,
    z_near: f32,
    z_far: f32,
    speed: f32,
}

impl Camera {
    pub fn new(
        pos: Vec3,
        target: Vec3,
        fov_degrees: f32,
        aspect_ratio: f32,
        z_near: f32,
        z_far: f32,
        speed: f32,
    ) -> Camera {
        let rotation = Self::get_lookat_rotation(pos, target);

        let camera_rig = CameraRig::builder()
            .with(Position::new(pos))
            .with(YawPitch::new().rotation_quat(rotation))
            .with(Smooth::new_position_rotation(1.0, 1.0))
            .build();

        Camera {
            camera_rig,
            fov_degrees,
            aspect_ratio,
            z_near,
            z_far,
            speed,
        }
    }

    pub fn get_lookat_rotation(pos: Vec3, target: Vec3) -> Quat {
        // Rotation calculation from
        // https://github.com/h3r2tic/dolly/blob/main/src/drivers/look_at.rs
        
        (target - pos)
            .try_normalize()
            .and_then(|forward| {
                let right = forward.cross(Vec3::Y).try_normalize()?;
                let up = right.cross(forward);
                Some(Quat::from_mat3(&Mat3::from_cols(right, up, -forward)))
            })
            .unwrap_or_default()
    }

    pub fn update(&mut self, input: &Input) -> bool {
        let transform = self.camera_rig.final_transform;

        let mut movement = Vec3::new(0.0, 0.0, 0.0);
        if input.key_down(winit::event::VirtualKeyCode::W) {
            movement += self.speed * transform.forward();
        }
        if input.key_down(winit::event::VirtualKeyCode::S) {
            movement -= self.speed * transform.forward();
        }
        if input.key_down(winit::event::VirtualKeyCode::A) {
            movement -= self.speed * transform.right();
        }
        if input.key_down(winit::event::VirtualKeyCode::D) {
            movement += self.speed * transform.right();
        }

        self.camera_rig.driver_mut::<Position>().translate(movement);

        let mut view_changed = false;
        if input.right_mouse_down {
            self.camera_rig
                .driver_mut::<YawPitch>()
                .rotate_yaw_pitch(-0.3 * input.mouse_delta.x, -0.3 * input.mouse_delta.y);
            view_changed = true;
        }

        // Todo: proper frame delta time
        self.camera_rig.update(1.0);

        movement != Vec3::new(0.0, 0.0, 0.0) || view_changed
    }

    pub fn get_view(&self) -> Mat4 {
        let transform = self.camera_rig.final_transform;

        glam::Mat4::look_at_rh(
            transform.position,
            transform.position + transform.forward(),
            transform.up(),
        )
    }

    pub fn get_projection(&self) -> Mat4 {
        glam::Mat4::perspective_rh(
            f32::to_radians(self.fov_degrees),
            self.aspect_ratio,
            self.z_near,
            self.z_far,
        )
    }

    pub fn get_position(&self) -> Vec3 {
        self.camera_rig.final_transform.position
    }

    pub fn get_forward(&self) -> Vec3 {
        self.camera_rig.final_transform.forward()
    }

    pub fn set_position_target(&mut self, position: Vec3, target: Vec3) {
        self.camera_rig.driver_mut::<Position>().position = position;

        let rotation = Self::get_lookat_rotation(position, target);
        self.camera_rig
            .driver_mut::<YawPitch>()
            .set_rotation_quat(rotation);
    }

    pub(crate) fn get_near_plane(&self) -> f32 {
        self.z_near
    }

    pub(crate) fn get_far_plane(&self) -> f32 {
        self.z_far
    }
}
