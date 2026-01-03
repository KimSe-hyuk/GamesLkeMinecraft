use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};

mod player;
mod world;
mod chunk;

use crate::world::{setup_game, VoxelWorld, ChunkManager, GameState, TextureMap}; 
use crate::chunk::{load_assets, check_assets_ready, update_chunks, rebuild_chunks};
// [수정] highlight_block 추가
use crate::player::{setup_ui_once, player_look, player_physics, player_interaction, highlight_block}; 

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .init_state::<GameState>()
        .insert_resource(VoxelWorld::default())
        .insert_resource(ChunkManager::default())
        .insert_resource(TextureMap::default())
        .add_systems(Startup, (setup_ui_once, grab_cursor))
        .add_systems(OnEnter(GameState::Loading), load_assets)
        .add_systems(Update, check_assets_ready.run_if(in_state(GameState::Loading)))
        
        .add_systems(OnEnter(GameState::InGame), setup_game)
        
        .add_systems(Update, (
            player_look, 
            player_physics, 
            player_interaction, 
            update_chunks, 
            rebuild_chunks,
            highlight_block // <--- [여기] 시스템 등록!
        ).run_if(in_state(GameState::InGame)))
        
        .run();
}

fn grab_cursor(mut q_windows: Query<&mut Window, With<PrimaryWindow>>) {
    let mut primary_window = q_windows.single_mut();
    primary_window.cursor.grab_mode = CursorGrabMode::Locked;
    primary_window.cursor.visible = false;
}