use bevy::prelude::*;
use crate::world::*;
use crate::chunk::{ChunkCoord, NeedsRemesh};

#[derive(Component)]
pub struct Player {
    pub velocity: Vec3,
    pub on_ground: bool,
    pub pitch: f32, // 위아래 회전 (고개)
    pub yaw: f32,   // 좌우 회전 (몸통)
}

// === 1. 마우스 시점 변환 (위아래 고정 해결) ===
pub fn player_look(
    mut mouse_motion_events: EventReader<bevy::input::mouse::MouseMotion>,
    mut query: Query<(&mut Transform, &mut Player)>,
) {
    let mut delta = Vec2::ZERO;
    for event in mouse_motion_events.read() {
        delta += event.delta;
    }

    for (mut transform, mut player) in query.iter_mut() {
        let sensitivity = 0.003; // 마우스 감도

        // 값 누적
        player.yaw -= delta.x * sensitivity;
        player.pitch -= delta.y * sensitivity;

        // 고개 각도 제한 (-89도 ~ +89도)
        player.pitch = player.pitch.clamp(-1.55, 1.55);

        // 회전 적용 (쿼터니언 연산: Y축 회전 후 X축 회전)
        transform.rotation = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.0);
    }
}

// === 2. 물리 엔진 (벽 통과 방지 + AABB 충돌) ===
pub fn player_physics(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut Player)>,
    chunk_manager: Res<ChunkManager>,
    voxel_world: Res<VoxelWorld>,
) {
    let dt = time.delta_seconds();

    for (mut transform, mut player) in query.iter_mut() {
        // 내 발밑 청크가 로딩 안 됐으면 멈춤 (추락 방지)
        let cx = (transform.translation.x / CHUNK_SIZE as f32).floor() as i32;
        let cz = (transform.translation.z / CHUNK_SIZE as f32).floor() as i32;
        if !chunk_manager.loaded_chunks.contains_key(&IVec3::new(cx, 0, cz)) { return; }

        // --- 입력 처리 ---
        let mut input_dir = Vec3::ZERO;
        // 시선 방향 기준으로 수평 이동 벡터 계산
        let yaw_rot = Quat::from_rotation_y(player.yaw);
        let forward = yaw_rot * Vec3::NEG_Z; 
        let right = yaw_rot * Vec3::X;

        if keyboard_input.pressed(KeyCode::KeyW) { input_dir += forward; }
        if keyboard_input.pressed(KeyCode::KeyS) { input_dir -= forward; }
        if keyboard_input.pressed(KeyCode::KeyD) { input_dir += right; }
        if keyboard_input.pressed(KeyCode::KeyA) { input_dir -= right; }

        if input_dir.length_squared() > 0.0 {
            input_dir = input_dir.normalize();
        }

        // --- 속도 설정 ---
        player.velocity.x = input_dir.x * MOVE_SPEED;
        player.velocity.z = input_dir.z * MOVE_SPEED;

        if keyboard_input.just_pressed(KeyCode::Space) && player.on_ground {
            player.velocity.y = JUMP_FORCE;
            player.on_ground = false;
        }

        player.velocity.y += GRAVITY * dt;

        // --- [핵심] 충돌 처리 (Collision Detection) ---
        let move_delta = player.velocity * dt;
        
        // 1. X축 이동 시도
        let next_x = transform.translation + Vec3::new(move_delta.x, 0.0, 0.0);
        if !check_collision(next_x, &voxel_world) {
            transform.translation.x = next_x.x;
        } else {
            player.velocity.x = 0.0;
        }

        // 2. Z축 이동 시도
        let next_z = transform.translation + Vec3::new(0.0, 0.0, move_delta.z);
        if !check_collision(next_z, &voxel_world) {
            transform.translation.z = next_z.z;
        } else {
            player.velocity.z = 0.0;
        }

        // 3. Y축 이동 시도
        let next_y = transform.translation + Vec3::new(0.0, move_delta.y, 0.0);
        if !check_collision(next_y, &voxel_world) {
            transform.translation.y = next_y.y;
            player.on_ground = false;
        } else {
            if player.velocity.y < 0.0 {
                player.on_ground = true; // 바닥 착지
            }
            player.velocity.y = 0.0;
        }

        // [안전장치] 버그로 추락 시 복귀
        if transform.translation.y < -50.0 {
            transform.translation.y = 100.0;
            player.velocity = Vec3::ZERO;
        }
    }
}

// 충돌 감지 함수
fn check_collision(pos: Vec3, world: &VoxelWorld) -> bool {
    let padding = 0.3; // 몸통 두께
    let height = 1.8;  // 키

    let check_points = [
        pos, // 발
        pos + Vec3::new(padding, 0.0, 0.0),
        pos + Vec3::new(-padding, 0.0, 0.0),
        pos + Vec3::new(0.0, 0.0, padding),
        pos + Vec3::new(0.0, 0.0, -padding),
        // 머리
        pos + Vec3::new(0.0, height, 0.0),
        pos + Vec3::new(padding, height, 0.0),
        pos + Vec3::new(-padding, height, 0.0),
        pos + Vec3::new(0.0, height, padding),
        pos + Vec3::new(0.0, height, -padding),
    ];

    for p in check_points {
        let ix = p.x.floor() as i32;
        let iy = p.y.floor() as i32;
        let iz = p.z.floor() as i32;
        
        if let Some(&block) = world.blocks.get(&IVec3::new(ix, iy, iz)) {
            if block != BLOCK_AIR {
                return true; // 충돌 발생
            }
        }
    }
    false
}

// === 3. 블록 상호작용 (정밀 에임) ===
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

            // 0.1 단위로 6.0 거리까지 검사
            for i in 0..60 {
                let dist = i as f32 * 0.1;
                let pos = ray_start + ray_dir * dist;
                
                let ix = pos.x.floor() as i32;
                let iy = pos.y.floor() as i32;
                let iz = pos.z.floor() as i32;
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

// === 4. UI 설정 (경고 해결됨!) ===
pub fn setup_ui_once(mut commands: Commands) {
    commands.spawn(Camera2dBundle {
        camera: Camera { order: 1, ..default() },
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
        // 십자선 가로
        parent.spawn(NodeBundle {
            style: Style {
                width: Val::Px(16.0),
                height: Val::Px(2.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            // [수정] Color::rgba -> Color::srgba 로 변경!
            background_color: Color::srgba(1.0, 1.0, 1.0, 0.8).into(),
            ..default()
        });
        // 십자선 세로
        parent.spawn(NodeBundle {
            style: Style {
                width: Val::Px(2.0),
                height: Val::Px(16.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            // [수정] Color::rgba -> Color::srgba 로 변경!
            background_color: Color::srgba(1.0, 1.0, 1.0, 0.8).into(),
            ..default()
        });
    });
}