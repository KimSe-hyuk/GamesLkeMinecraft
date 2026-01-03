use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};

mod player;
mod world;
mod chunk;
mod ui; // [추가] ui 모듈 등록

use crate::world::{setup_game, VoxelWorld, ChunkManager, GameState, TextureMap}; 
use crate::chunk::{load_assets, check_assets_ready, update_chunks, rebuild_chunks};
use crate::player::{player_look, player_physics, player_interaction, highlight_block}; 
// setup_ui_once 대신 ui 모듈의 기능들을 가져옵니다.
use crate::ui::{setup_ui, update_inventory_input, Inventory};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .init_state::<GameState>()
        .insert_resource(VoxelWorld::default())
        .insert_resource(ChunkManager::default())
        .insert_resource(TextureMap::default())
        .insert_resource(Inventory::default()) // [추가] 인벤토리 리소스 초기화
        
        // [수정] setup_ui_once -> setup_ui 로 변경
        .add_systems(Startup, (setup_ui, grab_cursor)) 
        
        .add_systems(OnEnter(GameState::Loading), load_assets)
        .add_systems(Update, check_assets_ready.run_if(in_state(GameState::Loading)))
        .add_systems(OnEnter(GameState::InGame), setup_game)
        
        .add_systems(Update, (
            player_look, 
            player_physics, 
            player_interaction, 
            update_chunks, 
            rebuild_chunks,
            highlight_block,
            update_inventory_input // [추가] 키보드 1~5 입력 감지 시스템
        ).run_if(in_state(GameState::InGame)))
        
        .run();
}

fn grab_cursor(mut q_windows: Query<&mut Window, With<PrimaryWindow>>) {
    let mut primary_window = q_windows.single_mut();
    primary_window.cursor.grab_mode = CursorGrabMode::Locked;
    primary_window.cursor.visible = false;
}