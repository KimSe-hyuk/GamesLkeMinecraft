use bevy::prelude::*;
use bevy::utils::{HashMap, HashSet};
use bevy::asset::LoadedFolder; // [추가] 이게 없어서 에러가 났습니다.

// === 상수 ===
pub const CHUNK_SIZE: usize = 16;
pub const CHUNK_HEIGHT: usize = 64;
pub const RENDER_DISTANCE: i32 = 6;
pub const MOVE_SPEED: f32 = 12.0;
pub const GRAVITY: f32 = -25.0;
pub const JUMP_FORCE: f32 = 10.0;

// === 블록 ID ===
pub const BLOCK_AIR: u8 = 0;
pub const BLOCK_DIRT: u8 = 1;
pub const BLOCK_STONE: u8 = 2;
pub const BLOCK_GRASS: u8 = 3;
pub const BLOCK_GOLD: u8 = 4;
pub const BLOCK_DIAMOND: u8 = 5;

// === 상태 ===
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    Loading,
    InGame,
}

// === 리소스 ===
#[derive(Resource, Default)]
pub struct GameAssets {
    pub folder_handle: Handle<LoadedFolder>, // 폴더 전체 핸들
    pub atlas_layout: Handle<TextureAtlasLayout>,
    pub atlas_image: Handle<Image>,
}

#[derive(Resource, Default)]
pub struct TextureMap {
    pub map: HashMap<String, usize>,
}

 

#[derive(Resource, Default)]
pub struct VoxelWorld {
    pub blocks: HashMap<IVec3, u8>,
}

#[derive(Resource, Default)]
pub struct ChunkManager {
    pub loaded_chunks: HashSet<IVec3>,
}