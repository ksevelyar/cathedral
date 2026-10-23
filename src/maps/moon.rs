use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::mesh::Mesh3d;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rand::{RngExt, SeedableRng, rngs::StdRng};

use super::{MapObject, SpawnContext};

pub(super) struct Moon {
    direction: Vec3,
}

struct LunarCrater {
    direction: Vec3,
    radius: f32,
    depth: f32,
}

impl Moon {
    pub(super) fn create(direction: Vec3) -> Self {
        Self { direction }
    }
}

impl MapObject for Moon {
    fn spawn(&self, context: &mut SpawnContext, _transform: Transform) {
        let moon_direction = self.direction.normalize();
        let moon_distance = 300.0;
        let moon_position = moon_direction * moon_distance;
        let moon_color = Color::srgb(0.72, 0.82, 1.0);
        let moon_emissive_luminance = 18.0;
        let moon_radius = 6.0;
        let moon_mesh = Sphere::new(moon_radius)
            .mesh()
            .ico(5)
            .expect("moon sphere mesh should be valid");
        let moon_texture = context.images.add(create_lunar_surface_texture());
        let moon_material = context.materials.add(StandardMaterial {
            base_color: moon_color,
            base_color_texture: Some(moon_texture.clone()),
            emissive: moon_color.to_linear() * moon_emissive_luminance,
            emissive_texture: Some(moon_texture),
            emissive_exposure_weight: 0.0,
            unlit: true,
            ..default()
        });
        let moonlight_color = Color::srgb(0.56, 0.68, 1.0);
        let moonlight_illuminance = 900.0;
        let moonlight_transform = Transform::from_translation(moon_position).looking_at(Vec3::ZERO, Vec3::Y);

        context.commands.spawn((
            DirectionalLight {
                color: moonlight_color,
                illuminance: moonlight_illuminance,
                shadow_maps_enabled: true,
                ..default()
            },
            moonlight_transform,
        ));
        context.commands.spawn((
            Mesh3d(context.meshes.add(moon_mesh)),
            MeshMaterial3d(moon_material),
            NotShadowCaster,
            NotShadowReceiver,
            Transform::from_translation(moon_position),
        ));
    }
}

fn create_lunar_surface_texture() -> Image {
    let texture_width = 512;
    let texture_height = 256;
    let crater_count = 110;
    let mut random_number_generator = StdRng::seed_from_u64(0x4d4f_4f4e);
    let lunar_craters = (0..crater_count)
        .map(|_| {
            let vertical_direction = random_number_generator.random_range(-1.0_f32..1.0_f32);
            let horizontal_angle = random_number_generator.random_range(0.0..std::f32::consts::TAU);
            let horizontal_radius = (1.0 - vertical_direction * vertical_direction).sqrt();
            LunarCrater {
                direction: Vec3::new(
                    horizontal_radius * horizontal_angle.cos(),
                    vertical_direction,
                    horizontal_radius * horizontal_angle.sin(),
                ),
                radius: random_number_generator.random_range(0.018..0.075),
                depth: random_number_generator.random_range(0.08..0.24),
            }
        })
        .collect::<Vec<_>>();
    let mut pixels = Vec::with_capacity((texture_width * texture_height * 4) as usize);

    for vertical_pixel in 0..texture_height {
        let latitude = (0.5 - (vertical_pixel as f32 + 0.5) / texture_height as f32) * std::f32::consts::PI;
        let latitude_radius = latitude.cos();
        for horizontal_pixel in 0..texture_width {
            let longitude = (horizontal_pixel as f32 + 0.5) / texture_width as f32 * std::f32::consts::TAU;
            let surface_direction = Vec3::new(
                latitude_radius * longitude.cos(),
                latitude.sin(),
                latitude_radius * longitude.sin(),
            );
            let maria_variation = (surface_direction.x * 8.0 + surface_direction.z * 3.0).sin() * 0.035
                + (surface_direction.z * 11.0 - surface_direction.y * 5.0).cos() * 0.025
                + (surface_direction.x * 19.0 + surface_direction.y * 13.0).sin() * 0.012;
            let crater_variation = lunar_craters.iter().fold(0.0_f32, |surface_variation, crater| {
                let cosine_distance = surface_direction.dot(crater.direction);
                let cosine_radius = crater.radius.cos();
                if cosine_distance <= cosine_radius {
                    return surface_variation;
                }
                let crater_distance = (2.0 * (1.0 - cosine_distance)).sqrt();
                let normalized_distance = crater_distance / crater.radius;
                let crater_bowl = (1.0 - normalized_distance * normalized_distance).max(0.0);
                let crater_rim = (1.0 - ((normalized_distance - 0.88) / 0.22).abs()).max(0.0);
                surface_variation - crater.depth * crater_bowl + crater_rim * crater.depth * 0.45
            });
            let surface_brightness = (0.82 + maria_variation + crater_variation).clamp(0.48, 1.0);
            let surface_value = (surface_brightness * 255.0) as u8;
            pixels.extend([surface_value, surface_value, surface_value, 255]);
        }
    }

    let mut lunar_surface_texture = Image::new(
        Extent3d {
            width: texture_width,
            height: texture_height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    lunar_surface_texture.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::ClampToEdge,
        ..ImageSamplerDescriptor::linear()
    });
    lunar_surface_texture
}
