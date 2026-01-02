use bevy::prelude::*;

mod world;
mod chunk;
mod player;

use world::*;
use chunk::*;
use player::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .init_state::<GameState>()
        
        // 리소스 초기화
        .init_resource::<VoxelWorld>()
        .init_resource::<ChunkManager>()
        .init_resource::<GameAssets>() // 폴더 핸들 저장소
        .init_resource::<TextureMap>() // 이름표 저장소

        // [1] 로딩 상태
        .add_systems(OnEnter(GameState::Loading), load_assets)
        .add_systems(Update, check_assets_ready.run_if(in_state(GameState::Loading)))

        // [2] 게임 상태
        .add_systems(OnEnter(GameState::InGame), setup_game)
        .add_systems(Update, (
            update_chunks,
            player_look,
            player_physics,
            player_interaction,
            setup_ui_once,
            rebuild_chunks,
        ).run_if(in_state(GameState::InGame)))
        
        .run();
}