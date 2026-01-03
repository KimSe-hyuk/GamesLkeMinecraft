use bevy::prelude::*;
use crate::world::*;
use crate::chunk::{ChunkCoord, NeedsRemesh};
use crate::ui::Inventory;
#[derive(Component)]
pub struct Player {
    pub velocity: Vec3,
    pub on_ground: bool,
}

#[derive(Component)]
pub struct MainCamera {
    pub pitch: f32,
}

// === 1. 시점 변환 ===
pub fn player_look(
    mut mouse_motion_events: EventReader<bevy::input::mouse::MouseMotion>,
    mut player_query: Query<&mut Transform, With<Player>>,
    mut camera_query: Query<(&mut Transform, &mut MainCamera), Without<Player>>,
) {
    let mut delta = Vec2::ZERO;
    for event in mouse_motion_events.read() { delta += event.delta; }
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

// === 2. 물리 엔진 + 1인칭 흔들림 ===
pub fn player_physics(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<(&mut Transform, &mut Player)>,
    mut camera_query: Query<&mut Transform, (With<MainCamera>, Without<Player>)>,
    chunk_manager: Res<ChunkManager>,
    voxel_world: Res<VoxelWorld>,
) {
    let dt = time.delta_seconds();

    for (mut transform, mut player) in player_query.iter_mut() {
        let cx = (transform.translation.x / CHUNK_SIZE as f32).floor() as i32;
        let cz = (transform.translation.z / CHUNK_SIZE as f32).floor() as i32;
        if !chunk_manager.loaded_chunks.contains_key(&IVec3::new(cx, 0, cz)) { return; }

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

        if keyboard_input.just_pressed(KeyCode::Space) && player.on_ground {
            player.velocity.y = JUMP_FORCE;
            player.on_ground = false;
        }

        player.velocity.y += GRAVITY * dt;

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

        if transform.translation.y < -50.0 { transform.translation.y = 100.0; player.velocity = Vec3::ZERO; }

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

// === DDA 알고리즘 ===
fn raycast_voxel(start: Vec3, dir: Vec3, range: f32, world: &VoxelWorld) -> Option<(IVec3, IVec3)> {
    let mut t = 0.0;
    let mut curr_voxel = IVec3::new(start.x.floor() as i32, start.y.floor() as i32, start.z.floor() as i32);
    let step = IVec3::new(if dir.x > 0.0 { 1 } else { -1 }, if dir.y > 0.0 { 1 } else { -1 }, if dir.z > 0.0 { 1 } else { -1 });
    let delta_dist = Vec3::new((1.0 / dir.x).abs().max(1e-30), (1.0 / dir.y).abs().max(1e-30), (1.0 / dir.z).abs().max(1e-30));
    let mut side_dist = Vec3::new(
        (if dir.x > 0.0 { (curr_voxel.x as f32 + 1.0) - start.x } else { start.x - curr_voxel.x as f32 }) * delta_dist.x,
        (if dir.y > 0.0 { (curr_voxel.y as f32 + 1.0) - start.y } else { start.y - curr_voxel.y as f32 }) * delta_dist.y,
        (if dir.z > 0.0 { (curr_voxel.z as f32 + 1.0) - start.z } else { start.z - curr_voxel.z as f32 }) * delta_dist.z,
    );
    let mut last_voxel = curr_voxel;

    while t < range {
        if let Some(&block) = world.blocks.get(&curr_voxel) {
            if block != BLOCK_AIR { return Some((curr_voxel, last_voxel)); }
        }
        last_voxel = curr_voxel;
        if side_dist.x < side_dist.y {
            if side_dist.x < side_dist.z { side_dist.x += delta_dist.x; curr_voxel.x += step.x; t = side_dist.x; } 
            else { side_dist.z += delta_dist.z; curr_voxel.z += step.z; t = side_dist.z; }
        } else {
            if side_dist.y < side_dist.z { side_dist.y += delta_dist.y; curr_voxel.y += step.y; t = side_dist.y; } 
            else { side_dist.z += delta_dist.z; curr_voxel.z += step.z; t = side_dist.z; }
        }
    }
    None
}

// === 에임 하이라이트 ===
pub fn highlight_block(
    mut gizmos: Gizmos,
    voxel_world: Res<VoxelWorld>,
    camera_query: Query<&GlobalTransform, With<MainCamera>>,
) {
    if let Ok(cam_tf) = camera_query.get_single() {
        // [수정] *cam_tf.forward() 로 변경하여 Dir3 -> Vec3 변환
        if let Some((hit_pos, _)) = raycast_voxel(cam_tf.translation(), *cam_tf.forward(), 6.0, &voxel_world) {
            gizmos.cuboid(
                Transform::from_translation(hit_pos.as_vec3() + 0.5).with_scale(Vec3::ONE * 1.01),
                Color::BLACK
            );
        }
    }
}
// === [수정] 블록 상호작용 ===
pub fn player_interaction(
    mut commands: Commands,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut voxel_world: ResMut<VoxelWorld>,
    camera_query: Query<&GlobalTransform, With<MainCamera>>,
    chunk_query: Query<(Entity, &ChunkCoord)>,
    inventory: Res<Inventory>, // [추가] 현재 선택된 블록 정보 가져오기
) {
    if let Ok(cam_tf) = camera_query.get_single() {
        if let Some((hit_pos, prev_pos)) = raycast_voxel(cam_tf.translation(), *cam_tf.forward(), 6.0, &voxel_world) {
            
            // 좌클릭: 파괴
            if mouse_input.just_pressed(MouseButton::Left) {
                // ... (파괴 로직 그대로) ...
                println!("🔨 파괴: {:?}", hit_pos);
                voxel_world.blocks.insert(hit_pos, BLOCK_AIR);
                refresh_chunk(hit_pos, &mut commands, &chunk_query);
            }
            
            // 우클릭: 설치
            if mouse_input.just_pressed(MouseButton::Right) {
                let player_dist = cam_tf.translation().distance(prev_pos.as_vec3() + 0.5);
                if player_dist > 1.5 { 
                    println!("🧱 설치: {:?} (타입: {})", prev_pos, inventory.current_block);
                    
                    // [핵심 수정] 무조건 STONE이 아니라, 인벤토리에서 선택한 블록을 설치!
                    voxel_world.blocks.insert(prev_pos, inventory.current_block); 
                    
                    refresh_chunk(prev_pos, &mut commands, &chunk_query);
                }
            }
        }
    }
}

fn refresh_chunk(pos: IVec3, commands: &mut Commands, chunk_query: &Query<(Entity, &ChunkCoord)>) {
    let cx = (pos.x as f32 / CHUNK_SIZE as f32).floor() as i32;
    let cz = (pos.z as f32 / CHUNK_SIZE as f32).floor() as i32;
    
    for (entity, coord) in chunk_query.iter() {
        if coord.x == cx && coord.z == cz {
            commands.entity(entity).insert(NeedsRemesh);
        }
    }
}

 