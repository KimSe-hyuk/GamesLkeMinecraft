use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy::asset::LoadedFolder;
use crate::player::{Player, MainCamera};

// ... (상수 정의들은 그대로 유지) ...
pub const CHUNK_SIZE: usize = 16;
pub const CHUNK_HEIGHT: usize = 64;
pub const RENDER_DISTANCE: i32 = 6;

pub const BLOCK_AIR: u8 = 0;
pub const BLOCK_DIRT: u8 = 1;
pub const BLOCK_GRASS: u8 = 2;
pub const BLOCK_STONE: u8 = 3;
pub const BLOCK_GOLD: u8 = 4;
pub const BLOCK_DIAMOND: u8 = 5;

pub const MOVE_SPEED: f32 = 8.0; 
pub const JUMP_FORCE: f32 = 10.0;
pub const GRAVITY: f32 = -25.0;

// ... (리소스 struct들 그대로 유지) ...
#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState { #[default] Loading, InGame, }

#[derive(Resource, Default)]
pub struct GameAssets {
    pub folder_handle: Handle<LoadedFolder>,
    pub atlas_layout: Handle<TextureAtlasLayout>,
    pub atlas_image: Handle<Image>,
}

#[derive(Resource, Default)]
pub struct VoxelWorld { pub blocks: HashMap<IVec3, u8>, }

#[derive(Resource, Default)]
pub struct ChunkManager { pub loaded_chunks: HashMap<IVec3, bool>, }

#[derive(Resource, Default)]
pub struct TextureMap { pub map: HashMap<String, usize>, }

// === [업그레이드] Setup Game 함수 ===
pub fn setup_game(mut commands: Commands) {
    // 1. 하늘색 배경 설정 (ClearColor)
    // 약간 연한 하늘색으로 설정합니다.
    commands.insert_resource(ClearColor(Color::srgba(0.5, 0.8, 1.0, 1.0)));

    // 2. 플레이어 소환 (이전과 동일)
    commands.spawn((
        SpatialBundle {
            transform: Transform::from_xyz(0.0, 100.0, 0.0),
            ..default()
        },
        Player { velocity: Vec3::ZERO, on_ground: false },
    ))
    .with_children(|parent| {
        parent.spawn((
            Camera3dBundle {
                transform: Transform::from_xyz(0.0, 1.7, 0.0), 
                ..default()
            },
            // [추가] 안개 효과 (Fog)
            // 멀리 있는 청크가 잘린 단면이 보이지 않고 부드럽게 사라지게 함
            FogSettings {
                color: Color::srgba(0.5, 0.8, 1.0, 1.0), // 배경색과 맞춤
                falloff: FogFalloff::Linear {
                    start: ((RENDER_DISTANCE - 2) * CHUNK_SIZE as i32) as f32, // 안개 시작 거리
                    end: ((RENDER_DISTANCE) * CHUNK_SIZE as i32) as f32,       // 완전 안 보이는 거리
                },
                ..default()
            },
            MainCamera { pitch: 0.0 },
        ));
    });

    // 3. 조명 (그대로)
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            illuminance: 15000.0,
            ..default()
        },
        transform: Transform::from_xyz(50.0, 100.0, 50.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}