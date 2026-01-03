use bevy::prelude::*;

#[derive(Component)]
pub struct Player {
    pub velocity: Vec3,
    pub on_ground: bool,
}

#[derive(Component)]
pub struct MainCamera {
    pub pitch: f32,
}

// === 마우스 시점 변환 (Camera Look) ===
pub fn player_look(
    mut mouse_motion_events: EventReader<bevy::input::mouse::MouseMotion>,
    mut player_query: Query<&mut Transform, With<Player>>,
    mut camera_query: Query<(&mut Transform, &mut MainCamera), Without<Player>>,
) {
    let mut delta = Vec2::ZERO;
    for event in mouse_motion_events.read() {
        delta += event.delta;
    }
    let sensitivity = 0.002;

    if let Ok(mut player_transform) = player_query.get_single_mut() {
        player_transform.rotate_y(-delta.x * sensitivity);
    }

    if let Ok((mut camera_transform, mut camera_data)) = camera_query.get_single_mut() {
        camera_data.pitch -= delta.y * sensitivity;
        camera_data.pitch = camera_data.pitch.clamp(-1.55, 1.55);
        camera_transform.rotation = Quat::from_rotation_x(camera_data.pitch);
    }
}