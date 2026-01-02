use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy::asset::LoadedFolder;
use crate::player::Player;

// === 상수 정의 ===
pub const CHUNK_SIZE: usize = 16;
pub const CHUNK_HEIGHT: usize = 64;
pub const RENDER_DISTANCE: i32 = 6; // 시야 거리 약간 증가

pub const BLOCK_AIR: u8 = 0;
pub const BLOCK_DIRT: u8 = 1;
pub const BLOCK_GRASS: u8 = 2;
pub const BLOCK_STONE: u8 = 3;
pub const BLOCK_GOLD: u8 = 4;
pub const BLOCK_DIAMOND: u8 = 5;

pub const MOVE_SPEED: f32 = 8.0; // 이동 속도 약간 증가
pub const JUMP_FORCE: f32 = 10.0;
pub const GRAVITY: f32 = -25.0;

// === 리소스 정의 ===
#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    Loading,
    InGame,
}

#[derive(Resource, Default)]
pub struct GameAssets {
    pub folder_handle: Handle<LoadedFolder>,
    pub atlas_layout: Handle<TextureAtlasLayout>,
    pub atlas_image: Handle<Image>,
}

#[derive(Resource, Default)]
pub struct VoxelWorld {
    pub blocks: HashMap<IVec3, u8>,
}

#[derive(Resource, Default)]
pub struct ChunkManager {
    pub loaded_chunks: HashMap<IVec3, bool>, 
}

#[derive(Resource, Default)]
pub struct TextureMap {
    pub map: HashMap<String, usize>,
}

// === Setup Game 함수 ===
pub fn setup_game(mut commands: Commands) {
   // 1. 플레이어 소환
    // 높이를 100.0으로 넉넉하게 잡습니다. 
    // chunk가 로딩될 때까지 공중부양(player_physics 수정본 덕분)하다가 로딩되면 착지합니다.
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 100.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        Player {
            velocity: Vec3::ZERO,
            on_ground: false,
            pitch: 0.0,
        },
    ));

    // 2. 조명 (그림자 품질 향상)
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            illuminance: 15000.0, // 밝기 증가
            ..default()
        },
        // 해의 위치를 비스듬하게 해서 입체감 살리기
        transform: Transform::from_xyz(50.0, 100.0, 50.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}