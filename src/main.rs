use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};

mod player;
mod world;
mod chunk;
mod ui;
mod physics;
mod interaction;

use crate::world::{setup_game, VoxelWorld, ChunkManager, GameState, TextureMap}; 
// [수정] update_chunks는 사라졌고, 세분화된 시스템을 가져옵니다.
use crate::chunk::{load_assets, check_assets_ready, rebuild_chunks, queue_chunks, process_chunks, despawn_chunks};
use crate::ui::{setup_ui, update_inventory_input, Inventory};
use crate::player::player_look; 
use crate::physics::player_physics; 
use crate::interaction::{player_interaction, highlight_block};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .init_state::<GameState>()
        .insert_resource(VoxelWorld::default())
        .insert_resource(ChunkManager::default())
        .insert_resource(TextureMap::default())
        .insert_resource(Inventory::default())
        .insert_resource(ClearColor(Color::srgba(0.5, 0.8, 1.0, 1.0))) 
        
        .add_systems(Startup, (setup_ui, grab_cursor)) 
        
        .add_systems(OnEnter(GameState::Loading), load_assets)
        .add_systems(Update, check_assets_ready.run_if(in_state(GameState::Loading)))
        .add_systems(OnEnter(GameState::InGame), setup_game)
        
        .add_systems(Update, (
            player_look, 
            player_physics,     
            player_interaction, 
            highlight_block,    
            update_inventory_input,
            // [수정] 비동기 청크 시스템 등록
            queue_chunks,   // 1. 작업 요청
            process_chunks, // 2. 작업 완료 및 생성
            despawn_chunks, // 3. 멀어진 청크 삭제
            rebuild_chunks,
        ).run_if(in_state(GameState::InGame)))
        
        .run();
}

fn grab_cursor(mut q_windows: Query<&mut Window, With<PrimaryWindow>>) {
    let mut primary_window = q_windows.single_mut();
    primary_window.cursor.grab_mode = CursorGrabMode::Locked;
    primary_window.cursor.visible = false;
}