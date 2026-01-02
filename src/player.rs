use bevy::prelude::*;
use crate::world::*; // 여기서 ChunkManager를 가져옵니다.
use crate::chunk::{ChunkCoord, NeedsRemesh}; // ChunkManager는 여기서 뺐습니다.

#[derive(Component)]
pub struct Player {
    pub velocity: Vec3,
    pub on_ground: bool,
}

pub fn player_look(
    time: Res<Time>,
    mut mouse_motion_events: EventReader<bevy::input::mouse::MouseMotion>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    let mut delta = Vec2::ZERO;
    for event in mouse_motion_events.read() {
        delta += event.delta;
    }

    for mut transform in query.iter_mut() {
        transform.rotate_y(-delta.x * 0.003);
        transform.rotate_local_x(-delta.y * 0.003);
    }
}

pub fn player_physics(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut Player)>,
    voxel_world: Res<VoxelWorld>,
) {
    let dt = time.delta_seconds();

    for (mut transform, mut player) in query.iter_mut() {
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
        
        // 간단한 바닥 충돌 처리
        if transform.translation.y < 5.0 { 
             transform.translation.y = 5.0;
             player.velocity.y = 0.0;
             player.on_ground = true;
        }
    }
}

// === 플레이어 상호작용 (최적화 버전) ===

pub fn player_interaction(
    mut commands: Commands,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut voxel_world: ResMut<VoxelWorld>,
    // [수정] chunk_manager는 더 이상 필요 없어서 제거했습니다! (경고 해결)
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
                        println!("블록 파괴: {:?}", block_pos);
                        
                        // 1. 데이터만 바꿈 (공기로)
                        voxel_world.blocks.insert(block_pos, BLOCK_AIR);

                        // 2. 해당 청크 찾아서 "NeedsRemesh" 스티커 붙이기
                        let cx = (ix as f32 / CHUNK_SIZE as f32).floor() as i32;
                        let cz = (iz as f32 / CHUNK_SIZE as f32).floor() as i32;

                        for (entity, coord) in chunk_query.iter() {
                            if coord.x == cx && coord.z == cz {
                                // ★ 핵심 ★: 삭제하지 않고 컴포넌트만 추가!
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

// === UI 설정 ===
pub fn setup_ui_once(
    mut commands: Commands,
) {
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