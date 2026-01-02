use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};

mod player;
mod world;
mod chunk;

// [수정] 끝부분에 있던 GameAssets를 지웠습니다.
use crate::world::{setup_game, VoxelWorld, ChunkManager, GameState, TextureMap}; 
use crate::chunk::{load_assets, check_assets_ready, update_chunks, rebuild_chunks};
use crate::player::{setup_ui_once, player_look, player_physics, player_interaction};

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
            rebuild_chunks
        ).run_if(in_state(GameState::InGame)))
        
        .run();
}

fn grab_cursor(mut q_windows: Query<&mut Window, With<PrimaryWindow>>) {
    let mut primary_window = q_windows.single_mut();
    primary_window.cursor.grab_mode = CursorGrabMode::Locked;
    primary_window.cursor.visible = false;
}