use bevy::prelude::*;
use crate::world::*;
use crate::chunk::{ChunkCoord, NeedsRemesh};
use crate::player::MainCamera;
use crate::ui::Inventory;

// === 블록 하이라이트 시스템 ===
pub fn highlight_block(
    mut gizmos: Gizmos,
    voxel_world: Res<VoxelWorld>,
    camera_query: Query<&GlobalTransform, With<MainCamera>>,
) {
    if let Ok(cam_tf) = camera_query.get_single() {
        if let Some((hit_pos, _)) = raycast_voxel(cam_tf.translation(), *cam_tf.forward(), 6.0, &voxel_world) {
            gizmos.cuboid(
                Transform::from_translation(hit_pos.as_vec3() + 0.5).with_scale(Vec3::ONE * 1.01),
                Color::BLACK
            );
        }
    }
}

// === 블록 설치 및 파괴 시스템 ===
pub fn player_interaction(
    mut commands: Commands,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut voxel_world: ResMut<VoxelWorld>,
    camera_query: Query<&GlobalTransform, With<MainCamera>>,
    chunk_query: Query<(Entity, &ChunkCoord)>,
    inventory: Res<Inventory>, 
) {
    if let Ok(cam_tf) = camera_query.get_single() {
        if let Some((hit_pos, prev_pos)) = raycast_voxel(cam_tf.translation(), *cam_tf.forward(), 6.0, &voxel_world) {
            
            // 좌클릭: 파괴
            if mouse_input.just_pressed(MouseButton::Left) {
                voxel_world.blocks.insert(hit_pos, BLOCK_AIR);
                refresh_chunk(hit_pos, &mut commands, &chunk_query);
            }
            
            // 우클릭: 설치
            if mouse_input.just_pressed(MouseButton::Right) {
                let player_dist = cam_tf.translation().distance(prev_pos.as_vec3() + 0.5);
                if player_dist > 1.5 { 
                    voxel_world.blocks.insert(prev_pos, inventory.current_block);
                    refresh_chunk(prev_pos, &mut commands, &chunk_query);
                }
            }
        }
    }
}

// 청크 갱신 헬퍼
fn refresh_chunk(pos: IVec3, commands: &mut Commands, chunk_query: &Query<(Entity, &ChunkCoord)>) {
    let cx = (pos.x as f32 / CHUNK_SIZE as f32).floor() as i32;
    let cz = (pos.z as f32 / CHUNK_SIZE as f32).floor() as i32;
    for (entity, coord) in chunk_query.iter() {
        if coord.x == cx && coord.z == cz {
            commands.entity(entity).insert(NeedsRemesh);
        }
    }
}

// DDA 알고리즘 (Raycasting)
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