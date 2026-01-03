use bevy::prelude::*;
use bevy::asset::{LoadState, LoadedFolder};
use bevy::render::{mesh::Indices, render_asset::RenderAssetUsages, render_resource::PrimitiveTopology};
use bevy::tasks::{AsyncComputeTaskPool, Task}; // [핵심] 비동기 작업 도구
use futures_lite::future; // [핵심] 작업 결과 기다리는 도구
use crate::world::*;
use crate::player::Player;
use rand::Rng;

#[derive(Component)]
pub struct ChunkCoord {
    pub x: i32,
    pub z: i32,
}

#[derive(Component)]
pub struct NeedsRemesh;

// [신규] 비동기 작업을 관리하는 컴포넌트
// "이 엔티티는 지금 백그라운드에서 청크를 굽고 있어요"라는 표시표
#[derive(Component)]
pub struct ChunkTask(Task<ChunkResult>);

// 작업 결과물 (데이터 + 메쉬 정보)
struct ChunkResult {
    chunk_data: Vec<u8>,
    mesh_data: MeshData,
}

// 메쉬를 만들기 위한 원시 데이터 (Vertices, Indices 등)
struct MeshData {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

// === 1. 로딩 시스템 (기존과 동일) ===
pub fn load_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    let folder_handle: Handle<LoadedFolder> = asset_server.load_folder("textures/blocks");
    commands.insert_resource(GameAssets {
        folder_handle,
        atlas_layout: Handle::default(),
        atlas_image: Handle::default(),
    });
}

pub fn check_assets_ready(
    mut game_assets: ResMut<GameAssets>,
    mut next_state: ResMut<NextState<GameState>>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    mut textures: ResMut<Assets<Image>>,
    mut texture_map: ResMut<TextureMap>,
    loaded_folders: Res<Assets<LoadedFolder>>,
    asset_server: Res<AssetServer>,
) {
    if let Some(folder) = loaded_folders.get(&game_assets.folder_handle) {
        let mut not_ready = false;
        for handle in &folder.handles {
            if let Some(LoadState::Loading) = asset_server.get_load_state(handle) { not_ready = true; }
        }
        if not_ready { return; } 

        let mut builder = TextureAtlasBuilder::default();
        builder.padding(UVec2::ZERO);
        builder.max_size(UVec2::new(4096, 4096));

        let mut valid_textures: Vec<(String, Handle<Image>)> = Vec::new();
        for handle in &folder.handles {
            if let Some(LoadState::Failed(_)) = asset_server.get_load_state(handle) { continue; }
            let typed_handle = handle.clone().typed::<Image>();
            if let Some(path) = handle.path() {
                let file_name = path.path().file_stem().unwrap().to_str().unwrap().to_string();
                if textures.get(&typed_handle).is_some() {
                    valid_textures.push((file_name, typed_handle));
                }
            }
        }
        
        if valid_textures.is_empty() { return; }
        valid_textures.sort_by(|a, b| a.0.cmp(&b.0));

        for (index, (name, handle)) in valid_textures.iter().enumerate() {
            let texture = textures.get(handle).unwrap();
            builder.add_texture(Some(handle.id()), texture);
            texture_map.map.insert(name.clone(), index);
        }

        match builder.build() {
            Ok((layout, image)) => {
                game_assets.atlas_layout = texture_atlases.add(layout);
                game_assets.atlas_image = textures.add(image);
                next_state.set(GameState::InGame);
            },
            Err(_) => println!("패킹 실패"),
        }
    }
}

// === 2. [변경] 청크 생성 요청 (Queue Chunks) ===
// 메인 스레드: "이거 만들어주세요" 하고 작업만 던져놓고 바로 끝냄 (렉 없음)
pub fn queue_chunks(
    mut commands: Commands,
    mut chunk_manager: ResMut<ChunkManager>,
    player_query: Query<&Transform, With<Player>>,
    game_assets: Res<GameAssets>,
    atlases: Res<Assets<TextureAtlasLayout>>,
    texture_map: Res<TextureMap>,
) {
    let player_transform = match player_query.get_single() {
        Ok(t) => t,
        Err(_) => return,
    };

    let cx = (player_transform.translation.x / CHUNK_SIZE as f32).floor() as i32;
    let cz = (player_transform.translation.z / CHUNK_SIZE as f32).floor() as i32;

    if let Some(atlas_layout) = atlases.get(&game_assets.atlas_layout) {
        // TaskPool: 비동기 작업 관리자
        let thread_pool = AsyncComputeTaskPool::get();

        for x in -RENDER_DISTANCE..=RENDER_DISTANCE {
            for z in -RENDER_DISTANCE..=RENDER_DISTANCE {
                let chunk_x = cx + x;
                let chunk_z = cz + z;
                let chunk_coord = IVec3::new(chunk_x, 0, chunk_z);

                // 이미 로딩됐거나, 로딩 중이면 패스
                if chunk_manager.loaded_chunks.contains_key(&chunk_coord) { continue; }
                
                // "로딩 중"이라고 표시 (중복 요청 방지)
                chunk_manager.loaded_chunks.insert(chunk_coord, false); // false = 아직 로딩 안 끝남

                // [중요] 비동기 작업에 필요한 데이터 복사 (클론)
                // 백그라운드 스레드는 메인 메모리에 직접 접근 못하므로 복사해서 줘야 함
                let layout_clone = atlas_layout.clone();
                let map_clone = TextureMap { map: texture_map.map.clone() };

                // === 백그라운드 작업 시작 ===
                let task = thread_pool.spawn(async move {
                    // 1. 데이터 생성 (노이즈 계산)
                    let mut rng = rand::thread_rng();
                    let mut chunk_data = vec![BLOCK_AIR; CHUNK_SIZE * CHUNK_SIZE * CHUNK_HEIGHT];
                    
                    for bx in 0..CHUNK_SIZE {
                        for bz in 0..CHUNK_SIZE {
                            let wx = chunk_x * CHUNK_SIZE as i32 + bx as i32;
                            let wz = chunk_z * CHUNK_SIZE as i32 + bz as i32;
                            let height = ((wx as f32 * 0.05).sin() * 10.0 + (wz as f32 * 0.08).cos() * 8.0 + 20.0) as usize;
                            let height = height.clamp(1, CHUNK_HEIGHT - 1);
                            
                            for y in 0..height {
                                let block_type;
                                if y == height - 1 { block_type = BLOCK_GRASS; }
                                else if y > height.saturating_sub(4) { block_type = BLOCK_DIRT; }
                                else if y == 0 { block_type = BLOCK_GOLD; }
                                else {
                                    let chance = rng.gen::<f32>();
                                    if y < 15 && chance < 0.08 { block_type = BLOCK_DIAMOND; }
                                    else if y < 25 && chance < 0.1 { block_type = BLOCK_GOLD; }
                                    else { block_type = BLOCK_STONE; }
                                }
                                chunk_data[bx + bz * CHUNK_SIZE + y * CHUNK_SIZE * CHUNK_SIZE] = block_type;
                            }
                        }
                    }

                    // 2. 메쉬 데이터 계산 (Vertices, UVs 등)
                    // 여기서 무거운 계산을 다 끝냅니다.
                    let mesh_data = calculate_mesh_data(&chunk_data, &layout_clone, &map_clone);

                    ChunkResult { chunk_data, mesh_data }
                });

                // 작업을 수행하는 엔티티 생성 (나중에 결과 받으려고 만듦)
                commands.spawn((
                    ChunkTask(task),
                    ChunkCoord { x: chunk_x, z: chunk_z }
                ));
            }
        }
    }
}

// === 3. [신규] 청크 작업 완료 처리 (Process Chunks) ===
// 메인 스레드: "보조야, 다 된 거 있니?" 확인하고 있으면 화면에 띄움
pub fn process_chunks(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut ChunkTask, &ChunkCoord)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut chunk_manager: ResMut<ChunkManager>,
    mut voxel_world: ResMut<VoxelWorld>,
    game_assets: Res<GameAssets>,
) {
    for (entity, mut task, coord) in &mut tasks {
        // 작업이 끝났는지 확인 (future::block_on은 poll_once랑 비슷하게 즉시 확인)
        if let Some(result) = future::block_on(future::poll_once(&mut task.0)) {
            // === 작업 완료! ===
            
            // 1. VoxelWorld에 데이터 등록
            for bx in 0..CHUNK_SIZE {
                for bz in 0..CHUNK_SIZE {
                    for y in 0..CHUNK_HEIGHT {
                        let block = result.chunk_data[bx + bz * CHUNK_SIZE + y * CHUNK_SIZE * CHUNK_SIZE];
                        if block != BLOCK_AIR {
                            let wx = coord.x * CHUNK_SIZE as i32 + bx as i32;
                            let wz = coord.z * CHUNK_SIZE as i32 + bz as i32;
                            voxel_world.blocks.insert(IVec3::new(wx, y as i32, wz), block);
                        }
                    }
                }
            }

            // 2. 메쉬 생성 (데이터는 이미 보조가 다 만들어옴)
            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, result.mesh_data.positions);
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, result.mesh_data.normals);
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, result.mesh_data.uvs);
            mesh.insert_indices(Indices::U32(result.mesh_data.indices));

            // 3. 실제 게임 오브젝트 소환
            commands.spawn((
                PbrBundle {
                    mesh: meshes.add(mesh),
                    material: materials.add(StandardMaterial {
                        base_color_texture: Some(game_assets.atlas_image.clone()),
                        cull_mode: None,
                        perceptual_roughness: 0.8,
                        metallic: 0.0,
                        reflectance: 0.1,
                        ..default()
                    }),
                    transform: Transform::from_xyz(coord.x as f32 * CHUNK_SIZE as f32, 0.0, coord.z as f32 * CHUNK_SIZE as f32),
                    ..default()
                },
                ChunkCoord { x: coord.x, z: coord.z },
            ));

            // 4. 작업 엔티티 삭제 및 로딩 완료 표시
            commands.entity(entity).despawn();
            chunk_manager.loaded_chunks.insert(IVec3::new(coord.x, 0, coord.z), true); // true = 로딩 완료
        }
    }
}

// === 4. 청크 삭제 (Despawn) ===
pub fn despawn_chunks(
    mut commands: Commands,
    mut chunk_manager: ResMut<ChunkManager>,
    chunk_query: Query<(Entity, &ChunkCoord), Without<ChunkTask>>, // 작업 중인 건 건드리지 않음
    player_query: Query<&Transform, With<Player>>,
) {
    if let Ok(player_transform) = player_query.get_single() {
        let cx = (player_transform.translation.x / CHUNK_SIZE as f32).floor() as i32;
        let cz = (player_transform.translation.z / CHUNK_SIZE as f32).floor() as i32;

        for (entity, chunk) in chunk_query.iter() {
            if (chunk.x - cx).abs() > RENDER_DISTANCE + 2 || (chunk.z - cz).abs() > RENDER_DISTANCE + 2 {
                commands.entity(entity).despawn();
                chunk_manager.loaded_chunks.remove(&IVec3::new(chunk.x, 0, chunk.z));
            }
        }
    }
}

// === 5. 리빌딩 시스템 (기존과 동일하지만 함수 분리 활용) ===
pub fn rebuild_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut chunk_query: Query<(Entity, &ChunkCoord, &mut Handle<Mesh>), With<NeedsRemesh>>,
    voxel_world: Res<VoxelWorld>,
    game_assets: Res<GameAssets>,
    atlases: Res<Assets<TextureAtlasLayout>>,
    texture_map: Res<TextureMap>,
) {
    let atlas_layout = if let Some(l) = atlases.get(&game_assets.atlas_layout) { l } else { return; };

    for (entity, coord, mut mesh_handle) in chunk_query.iter_mut() {
        let mut chunk_data = vec![BLOCK_AIR; CHUNK_SIZE * CHUNK_SIZE * CHUNK_HEIGHT];
        
        for bx in 0..CHUNK_SIZE {
            for bz in 0..CHUNK_SIZE {
                for y in 0..CHUNK_HEIGHT {
                    let wx = coord.x * CHUNK_SIZE as i32 + bx as i32;
                    let wz = coord.z * CHUNK_SIZE as i32 + bz as i32;
                    if let Some(&block) = voxel_world.blocks.get(&IVec3::new(wx, y as i32, wz)) {
                        chunk_data[bx + bz * CHUNK_SIZE + y * CHUNK_SIZE * CHUNK_SIZE] = block;
                    }
                }
            }
        }

        let mesh_data = calculate_mesh_data(&chunk_data, atlas_layout, &texture_map);
        
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, mesh_data.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, mesh_data.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, mesh_data.uvs);
        mesh.insert_indices(Indices::U32(mesh_data.indices));

        *mesh_handle = meshes.add(mesh);
        commands.entity(entity).remove::<NeedsRemesh>();
    }
}

// === 6. 메쉬 데이터 계산 함수 (Bevy Mesh 의존성 제거) ===
// 이 함수는 백그라운드 스레드에서도 돌아갈 수 있게 순수 데이터만 리턴합니다.
fn calculate_mesh_data(chunk_data: &Vec<u8>, layout: &TextureAtlasLayout, map: &TextureMap) -> MeshData {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    let mut idx_counter = 0;

    let get_block = |x: i32, y: i32, z: i32| -> u8 {
        if x < 0 || x >= CHUNK_SIZE as i32 || z < 0 || z >= CHUNK_SIZE as i32 || y < 0 || y >= CHUNK_HEIGHT as i32 { return BLOCK_AIR; }
        chunk_data[x as usize + z as usize * CHUNK_SIZE + y as usize * CHUNK_SIZE * CHUNK_SIZE]
    };

    for x in 0..CHUNK_SIZE as i32 {
        for z in 0..CHUNK_SIZE as i32 {
            for y in 0..CHUNK_HEIGHT as i32 {
                let block = get_block(x, y, z);
                if block == BLOCK_AIR { continue; }

                if get_block(x, y + 1, z) == BLOCK_AIR { add_face(&mut positions, &mut normals, &mut uvs, &mut indices, &mut idx_counter, [x, y + 1, z], [x + 1, y + 1, z + 1], [0.0, 1.0, 0.0], block, 0, layout, map); }
                if get_block(x, y - 1, z) == BLOCK_AIR { add_face(&mut positions, &mut normals, &mut uvs, &mut indices, &mut idx_counter, [x, y, z + 1], [x + 1, y, z], [0.0, -1.0, 0.0], block, 1, layout, map); }
                if get_block(x, y, z + 1) == BLOCK_AIR { add_face(&mut positions, &mut normals, &mut uvs, &mut indices, &mut idx_counter, [x, y, z + 1], [x + 1, y + 1, z + 1], [0.0, 0.0, 1.0], block, 2, layout, map); }
                if get_block(x, y, z - 1) == BLOCK_AIR { add_face(&mut positions, &mut normals, &mut uvs, &mut indices, &mut idx_counter, [x + 1, y, z], [x, y + 1, z], [0.0, 0.0, -1.0], block, 2, layout, map); }
                if get_block(x + 1, y, z) == BLOCK_AIR { add_face(&mut positions, &mut normals, &mut uvs, &mut indices, &mut idx_counter, [x + 1, y, z + 1], [x + 1, y + 1, z], [1.0, 0.0, 0.0], block, 2, layout, map); }
                if get_block(x - 1, y, z) == BLOCK_AIR { add_face(&mut positions, &mut normals, &mut uvs, &mut indices, &mut idx_counter, [x, y, z], [x, y + 1, z + 1], [-1.0, 0.0, 0.0], block, 2, layout, map); }
            }
        }
    }

    MeshData { positions, normals, uvs, indices }
}

fn add_face(
    pos: &mut Vec<[f32; 3]>, norm: &mut Vec<[f32; 3]>, uvs: &mut Vec<[f32; 2]>, idx: &mut Vec<u32>, counter: &mut u32,
    from: [i32; 3], to: [i32; 3], normal: [f32; 3], block_id: u8, face_type: usize, layout: &TextureAtlasLayout, map: &TextureMap
) {
    let (x1, y1, z1) = (from[0] as f32, from[1] as f32, from[2] as f32);
    let (x2, y2, z2) = (to[0] as f32, to[1] as f32, to[2] as f32);

    if normal[1].abs() > 0.0 { 
        pos.push([x1, y1, z1]); pos.push([x2, y1, z1]); pos.push([x2, y1, z2]); pos.push([x1, y1, z2]);
    } else if normal[2].abs() > 0.0 { 
        pos.push([x1, y1, z1]); pos.push([x2, y1, z1]); pos.push([x2, y2, z1]); pos.push([x1, y2, z1]);
    } else { 
        pos.push([x1, y1, z1]); pos.push([x1, y1, z2]); pos.push([x1, y2, z2]); pos.push([x1, y2, z1]);
    }

    for _ in 0..4 { norm.push(normal); }

    let texture_name = match block_id {
        BLOCK_DIRT => "dirt",
        BLOCK_STONE => "stone",
        BLOCK_GOLD => "gold",
        BLOCK_DIAMOND => "diamond", 
        BLOCK_GRASS => match face_type {
            0 => "grass_top",
            1 => "dirt",
            _ => "grass_side",
        },
        _ => "dirt",
    };

    let texture_index = *map.map.get(texture_name).unwrap_or(&0);
    if let Some(rect) = layout.textures.get(texture_index) {
        let size = layout.size;
        uvs.push([rect.min.x as f32 / size.x as f32, rect.max.y as f32 / size.y as f32]);
        uvs.push([rect.max.x as f32 / size.x as f32, rect.max.y as f32 / size.y as f32]);
        uvs.push([rect.max.x as f32 / size.x as f32, rect.min.y as f32 / size.y as f32]);
        uvs.push([rect.min.x as f32 / size.x as f32, rect.min.y as f32 / size.y as f32]);
    } else {
        uvs.push([0.0, 0.0]); uvs.push([1.0, 0.0]); uvs.push([1.0, 1.0]); uvs.push([0.0, 1.0]);
    }

    idx.push(*counter); idx.push(*counter + 2); idx.push(*counter + 1);
    idx.push(*counter); idx.push(*counter + 3); idx.push(*counter + 2);
    *counter += 4;
}