use bevy::prelude::*;
use crate::world::*;
use crate::chunk::{ChunkCoord, NeedsRemesh};

#[derive(Component)]
pub struct Player {
    pub velocity: Vec3,
    pub on_ground: bool,
    pub pitch: f32,
}

// === 1. 카메라/마우스 회전 ===
pub fn player_look(
    _time: Res<Time>,
    mut mouse_motion_events: EventReader<bevy::input::mouse::MouseMotion>,
    mut query: Query<(&mut Transform, &mut Player)>,
) {
    let mut delta = Vec2::ZERO;
    for event in mouse_motion_events.read() {
        delta += event.delta;
    }

    for (mut transform, mut player) in query.iter_mut() {
        transform.rotate_y(-delta.x * 0.003);
        player.pitch -= delta.y * 0.003;
        player.pitch = player.pitch.clamp(-1.55, 1.55);

        let yaw_rotation = Quat::from_axis_angle(Vec3::Y, transform.rotation.to_euler(EulerRot::YXZ).0);
        let pitch_rotation = Quat::from_axis_angle(Vec3::X, player.pitch);
        transform.rotation = yaw_rotation * pitch_rotation;
    }
}

// === 2. 물리 엔진 (추락 방지 기능 추가!) ===
pub fn player_physics(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut Player)>,
    // [중요] 청크 매니저를 가져와서 땅이 있는지 확인해야 함
    chunk_manager: Res<ChunkManager>, 
) {
    let dt = time.delta_seconds();

    for (mut transform, mut player) in query.iter_mut() {
        // [핵심 수정] 내 위치에 청크가 로딩되었는지 확인
        let cx = (transform.translation.x / CHUNK_SIZE as f32).floor() as i32;
        let cz = (transform.translation.z / CHUNK_SIZE as f32).floor() as i32;
        let current_chunk_coord = IVec3::new(cx, 0, cz);

        // 만약 내 발밑 청크가 아직 안 만들어졌다면?
        // -> 움직이지 말고, 중력도 받지 말고 공중에 떠 있어라! (로딩 대기)
        if !chunk_manager.loaded_chunks.contains_key(&current_chunk_coord) {
            // 아직 로딩 중이면 함수 종료 (떨어지지 않음)
            return; 
        }

        // --- 로딩이 다 된 후에야 아래 코드가 실행됨 ---

        let mut input_dir = Vec3::ZERO;
        let forward = transform.forward();
        let right = transform.right();
        let forward_flat = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
        let right_flat = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

        if keyboard_input.pressed(KeyCode::KeyW) { input_dir += forward_flat; }
        if keyboard_input.pressed(KeyCode::KeyS) { input_dir -= forward_flat; }
        if keyboard_input.pressed(KeyCode::KeyD) { input_dir += right_flat; }
        if keyboard_input.pressed(KeyCode::KeyA) { input_dir -= right_flat; }

        if input_dir.length_squared() > 0.0 {
            input_dir = input_dir.normalize();
        }

        player.velocity.x = input_dir.x * MOVE_SPEED;
        player.velocity.z = input_dir.z * MOVE_SPEED;

        if keyboard_input.just_pressed(KeyCode::Space) && player.on_ground {
            player.velocity.y = JUMP_FORCE;
            player.on_ground = false;
        }

        player.velocity.y += GRAVITY * dt;
        let move_delta = player.velocity * dt;
        transform.translation += move_delta;
        
        // [임시 충돌] 바닥(y=25) 밑으로 떨어지면 강제로 올림 (지형 평균 높이 고려)
        if transform.translation.y < 25.0 { 
             transform.translation.y = 25.0;
             player.velocity.y = 0.0;
             player.on_ground = true;
        }
    }
}

// === 3. 상호작용 (그대로 유지) ===
pub fn player_interaction(
    mut commands: Commands,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut voxel_world: ResMut<VoxelWorld>,
    player_query: Query<&Transform, With<Player>>,
    chunk_query: Query<(Entity, &ChunkCoord)>, 
) {
    if let Ok(transform) = player_query.get_single() {
        if mouse_input.just_pressed(MouseButton::Left) {
            let ray_start = transform.translation;
            let ray_dir = transform.forward();

            for i in 1..8 {
                let pos = ray_start + ray_dir * i as f32;
                let ix = pos.x.round() as i32;
                let iy = pos.y.round() as i32;
                let iz = pos.z.round() as i32;
                let block_pos = IVec3::new(ix, iy, iz);

                if let Some(&block_id) = voxel_world.blocks.get(&block_pos) {
                    if block_id != BLOCK_AIR {
                        println!("🔨 파괴: {:?}", block_pos);
                        voxel_world.blocks.insert(block_pos, BLOCK_AIR);

                        let cx = (ix as f32 / CHUNK_SIZE as f32).floor() as i32;
                        let cz = (iz as f32 / CHUNK_SIZE as f32).floor() as i32;

                        for (entity, coord) in chunk_query.iter() {
                            if coord.x == cx && coord.z == cz {
                                commands.entity(entity).insert(NeedsRemesh);
                                break;
                            }
                        }
                        break;
                    }
                }
            }
        }
    }
}

// === 4. UI 설정 (카메라 경고 완벽 해결) ===
pub fn setup_ui_once(mut commands: Commands) {
    // [핵심 수정] UI 카메라는 순서(Order)를 1로 설정해서 3D 카메라(Order 0)보다 나중에 그립니다.
    // 이러면 "Camera order ambiguities" 경고가 싹 사라집니다.
    commands.spawn(Camera2dBundle {
        camera: Camera {
            order: 1, 
            ..default()
        },
        ..default()
    });

    commands.spawn(
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        }
    ).with_children(|parent| {
        parent.spawn(NodeBundle {
            style: Style {
                width: Val::Px(10.0),
                height: Val::Px(10.0),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            background_color: Color::WHITE.into(),
            ..default()
        });
    });
}