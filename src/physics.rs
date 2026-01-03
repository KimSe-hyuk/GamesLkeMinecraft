use bevy::prelude::*;
use crate::world::*; // 여기에 ChunkManager가 포함되어 있습니다!
// use crate::chunk::ChunkManager;  <-- [삭제] 이 줄이 에러의 원인입니다! 지워주세요.
use crate::player::{Player, MainCamera}; 

// === 물리 엔진 시스템 ===
pub fn player_physics(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<(&mut Transform, &mut Player)>,
    mut camera_query: Query<&mut Transform, (With<MainCamera>, Without<Player>)>,
    chunk_manager: Res<ChunkManager>, // 이제 world에서 가져온 걸 잘 씁니다.
    voxel_world: Res<VoxelWorld>,
) {
    let dt = time.delta_seconds();

    for (mut transform, mut player) in player_query.iter_mut() {
        // 로딩 체크
        let cx = (transform.translation.x / CHUNK_SIZE as f32).floor() as i32;
        let cz = (transform.translation.z / CHUNK_SIZE as f32).floor() as i32;
        if !chunk_manager.loaded_chunks.contains_key(&IVec3::new(cx, 0, cz)) { return; }

        // 1. 입력 및 속도 설정
        let mut input_dir = Vec3::ZERO;
        let forward = transform.forward();
        let right = transform.right();
        let forward_flat = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
        let right_flat = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

        if keyboard_input.pressed(KeyCode::KeyW) { input_dir += forward_flat; }
        if keyboard_input.pressed(KeyCode::KeyS) { input_dir -= forward_flat; }
        if keyboard_input.pressed(KeyCode::KeyD) { input_dir += right_flat; }
        if keyboard_input.pressed(KeyCode::KeyA) { input_dir -= right_flat; }

        let is_moving = input_dir.length_squared() > 0.0;
        if is_moving { input_dir = input_dir.normalize(); }

        player.velocity.x = input_dir.x * MOVE_SPEED;
        player.velocity.z = input_dir.z * MOVE_SPEED;

        // 점프
        if keyboard_input.just_pressed(KeyCode::Space) && player.on_ground {
            player.velocity.y = JUMP_FORCE;
            player.on_ground = false;
        }

        // 중력
        player.velocity.y += GRAVITY * dt;

        // 2. AABB 충돌 처리
        let move_amount = player.velocity * dt;
        
        let next_x = transform.translation + Vec3::new(move_amount.x, 0.0, 0.0);
        if !aabb_collision(next_x, &voxel_world) { transform.translation.x = next_x.x; } 
        else { player.velocity.x = 0.0; }

        let next_z = transform.translation + Vec3::new(0.0, 0.0, move_amount.z);
        if !aabb_collision(next_z, &voxel_world) { transform.translation.z = next_z.z; } 
        else { player.velocity.z = 0.0; }

        let next_y = transform.translation + Vec3::new(0.0, move_amount.y, 0.0);
        if !aabb_collision(next_y, &voxel_world) {
            transform.translation.y = next_y.y;
            player.on_ground = false;
        } else {
            if player.velocity.y < 0.0 {
                player.on_ground = true;
                transform.translation.y = transform.translation.y.round();
            }
            player.velocity.y = 0.0;
        }

        // 추락 복귀
        if transform.translation.y < -50.0 { transform.translation.y = 100.0; player.velocity = Vec3::ZERO; }

        // 3. 카메라 흔들림 (View Bobbing)
        if let Ok(mut camera_transform) = camera_query.get_single_mut() {
            if player.on_ground && is_moving {
                let bob_speed = 10.0;
                let bob_amount = 0.05;
                let bob_offset = (time.elapsed_seconds() * bob_speed).sin() * bob_amount;
                camera_transform.translation.y = 1.7 + bob_offset;
            } else {
                let current_y = camera_transform.translation.y;
                camera_transform.translation.y = current_y + (1.7 - current_y) * dt * 5.0;
            }
        }
    }
}

// AABB 충돌 함수 (private)
fn aabb_collision(pos: Vec3, world: &VoxelWorld) -> bool {
    let half_width = 0.25;
    let height = 1.75;
    let min = pos - Vec3::new(half_width, 0.0, half_width);
    let max = pos + Vec3::new(half_width, height, half_width);
    let min_ix = min.x.floor() as i32;
    let max_ix = max.x.floor() as i32;
    let min_iy = min.y.floor() as i32;
    let max_iy = max.y.floor() as i32;
    let min_iz = min.z.floor() as i32;
    let max_iz = max.z.floor() as i32;

    for x in min_ix..=max_ix {
        for y in min_iy..=max_iy {
            for z in min_iz..=max_iz {
                if let Some(&block) = world.blocks.get(&IVec3::new(x, y, z)) {
                    if block != BLOCK_AIR { return true; }
                }
            }
        }
    }
    false
}