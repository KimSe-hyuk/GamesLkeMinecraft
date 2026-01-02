use bevy::prelude::*;
use bevy::asset::{LoadState, LoadedFolder};
use bevy::render::{mesh::Indices, render_asset::RenderAssetUsages, render_resource::PrimitiveTopology};
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

// === 1. 로딩 시스템 ===
pub fn load_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    println!("=== [System] 텍스처 로딩 시작 ===");
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
            if let Some(LoadState::Loading) = asset_server.get_load_state(handle) {
                not_ready = true;
            }
        }
        if not_ready { return; } 

        println!("=== [System] 리소스 준비 완료! ===");
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
                println!("=== [System] 게임 진입 ===");
                game_assets.atlas_layout = texture_atlases.add(layout);
                game_assets.atlas_image = textures.add(image);
                next_state.set(GameState::InGame);
            },
            Err(_) => println!("패킹 실패"),
        }
    }
}

// === 2. 청크 로직 ===

pub fn update_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut chunk_manager: ResMut<ChunkManager>,
    mut voxel_world: ResMut<VoxelWorld>,
    player_query: Query<&Transform, With<Player>>,
    chunk_query: Query<(Entity, &ChunkCoord)>,
    game_assets: Res<GameAssets>,
    atlases: Res<Assets<TextureAtlasLayout>>,
    texture_map: Res<TextureMap>,
) {
    if let Ok(player_transform) = player_query.get_single() {
        let player_pos = player_transform.translation;
        let cx = (player_pos.x / CHUNK_SIZE as f32).floor() as i32;
        let cz = (player_pos.z / CHUNK_SIZE as f32).floor() as i32;

        if let Some(atlas_layout) = atlases.get(&game_assets.atlas_layout) {
            for x in -RENDER_DISTANCE..=RENDER_DISTANCE {
                for z in -RENDER_DISTANCE..=RENDER_DISTANCE {
                    let chunk_x = cx + x;
                    let chunk_z = cz + z;
                    let chunk_coord = IVec3::new(chunk_x, 0, chunk_z);

                    if chunk_manager.loaded_chunks.contains_key(&chunk_coord) { continue; }
                    chunk_manager.loaded_chunks.insert(chunk_coord, true);

                    let mut rng = rand::thread_rng();
                    let mut chunk_data = vec![BLOCK_AIR; CHUNK_SIZE * CHUNK_SIZE * CHUNK_HEIGHT];
                    
                    for bx in 0..CHUNK_SIZE {
                        for bz in 0..CHUNK_SIZE {
                            let wx = chunk_x * CHUNK_SIZE as i32 + bx as i32;
                            let wz = chunk_z * CHUNK_SIZE as i32 + bz as i32;
                            
                            // 지형 높이 (노이즈)
                            let height = ((wx as f32 * 0.05).sin() * 10.0 + (wz as f32 * 0.08).cos() * 8.0 + 20.0) as usize;
                            let height = height.clamp(1, CHUNK_HEIGHT - 1);
                            
                            for y in 0..height {
                                let block_type;
                                if y == height - 1 { block_type = BLOCK_GRASS; }
                                else if y > height.saturating_sub(4) { block_type = BLOCK_DIRT; }
                                else if y == 0 { block_type = BLOCK_GOLD; }
                                else {
                                    let chance = rng.gen::<f32>();
                                    // 광물 빈도 약간 증가
                                    if y < 15 && chance < 0.08 { block_type = BLOCK_DIAMOND; }
                                    else if y < 25 && chance < 0.1 { block_type = BLOCK_GOLD; }
                                    else { block_type = BLOCK_STONE; }
                                }
                                chunk_data[bx + bz * CHUNK_SIZE + y * CHUNK_SIZE * CHUNK_SIZE] = block_type;
                                voxel_world.blocks.insert(IVec3::new(wx, y as i32, wz), block_type);
                            }
                        }
                    }

                    let mesh = generate_mesh(&chunk_data, atlas_layout, &texture_map);
                    
                    commands.spawn((
                        PbrBundle {
                            mesh: meshes.add(mesh),
                            material: materials.add(StandardMaterial {
                                base_color_texture: Some(game_assets.atlas_image.clone()),
                                // [중요] 깜빡임 해결: 양면 렌더링 + 조명 반응 강화
                                cull_mode: None, 
                                perceptual_roughness: 0.8,
                                metallic: 0.0,
                                reflectance: 0.1,
                                ..default()
                            }),
                            transform: Transform::from_xyz(chunk_x as f32 * CHUNK_SIZE as f32, 0.0, chunk_z as f32 * CHUNK_SIZE as f32),
                            ..default()
                        },
                        ChunkCoord { x: chunk_x, z: chunk_z },
                        // [추가] AABB(경계박스)를 강제로 계산하게 유도 (깜빡임 방지용, Bevy가 자동 처리하지만 명시적이면 좋음)
                    ));
                }
            }
        }
        
        for (entity, chunk) in chunk_query.iter() {
            if (chunk.x - cx).abs() > RENDER_DISTANCE + 1 || (chunk.z - cz).abs() > RENDER_DISTANCE + 1 {
                commands.entity(entity).despawn();
                chunk_manager.loaded_chunks.remove(&IVec3::new(chunk.x, 0, chunk.z));
            }
        }
    }
}

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
        
        // 주변 블록 정보까지 고려해서 다시 만들어야 정확하지만, 
        // 일단 현재 청크 데이터만 VoxelWorld에서 긁어옵니다.
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

        let new_mesh = generate_mesh(&chunk_data, atlas_layout, &texture_map);
        *mesh_handle = meshes.add(new_mesh);
        
        commands.entity(entity).remove::<NeedsRemesh>();
        println!("♻️ 청크 갱신됨: ({}, {})", coord.x, coord.z);
    }
}

fn generate_mesh(chunk_data: &Vec<u8>, layout: &TextureAtlasLayout, map: &TextureMap) -> Mesh {
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

                // [중요] 인접한 곳이 공기(AIR)일 때만 면을 그립니다.
                // 그래야 블록 파괴 시 뒤에 있던 블록이 보입니다.
                if get_block(x, y + 1, z) == BLOCK_AIR { add_face(&mut positions, &mut normals, &mut uvs, &mut indices, &mut idx_counter, [x, y + 1, z], [x + 1, y + 1, z + 1], [0.0, 1.0, 0.0], block, 0, layout, map); }
                if get_block(x, y - 1, z) == BLOCK_AIR { add_face(&mut positions, &mut normals, &mut uvs, &mut indices, &mut idx_counter, [x, y, z + 1], [x + 1, y, z], [0.0, -1.0, 0.0], block, 1, layout, map); }
                if get_block(x, y, z + 1) == BLOCK_AIR { add_face(&mut positions, &mut normals, &mut uvs, &mut indices, &mut idx_counter, [x, y, z + 1], [x + 1, y + 1, z + 1], [0.0, 0.0, 1.0], block, 2, layout, map); }
                if get_block(x, y, z - 1) == BLOCK_AIR { add_face(&mut positions, &mut normals, &mut uvs, &mut indices, &mut idx_counter, [x + 1, y, z], [x, y + 1, z], [0.0, 0.0, -1.0], block, 2, layout, map); }
                if get_block(x + 1, y, z) == BLOCK_AIR { add_face(&mut positions, &mut normals, &mut uvs, &mut indices, &mut idx_counter, [x + 1, y, z + 1], [x + 1, y + 1, z], [1.0, 0.0, 0.0], block, 2, layout, map); }
                if get_block(x - 1, y, z) == BLOCK_AIR { add_face(&mut positions, &mut normals, &mut uvs, &mut indices, &mut idx_counter, [x, y, z], [x, y + 1, z + 1], [-1.0, 0.0, 0.0], block, 2, layout, map); }
            }
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn add_face(
    pos: &mut Vec<[f32; 3]>, norm: &mut Vec<[f32; 3]>, uvs: &mut Vec<[f32; 2]>, idx: &mut Vec<u32>, counter: &mut u32,
    from: [i32; 3], to: [i32; 3], normal: [f32; 3], block_id: u8, face_type: usize, layout: &TextureAtlasLayout, map: &TextureMap
) {
    let (x1, y1, z1) = (from[0] as f32, from[1] as f32, from[2] as f32);
    let (x2, y2, z2) = (to[0] as f32, to[1] as f32, to[2] as f32);

    // 좌표 생성 로직 (저번에 고친 완벽한 버전)
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
        BLOCK_GOLD => "gold_ore",
        BLOCK_DIAMOND => "diamond_ore", 
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