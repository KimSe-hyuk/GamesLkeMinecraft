use bevy::prelude::*;
use crate::world::*; // 블록 ID 상수 가져오기

// === 리소스: 현재 선택된 블록 ===
#[derive(Resource)]
pub struct Inventory {
    pub current_block: u8,
}

impl Default for Inventory {
    fn default() -> Self {
        Self { current_block: BLOCK_STONE } // 기본은 돌
    }
}

// UI 마커 컴포넌트
#[derive(Component)]
pub struct HotbarSlot {
    pub index: usize,
}

// === 1. UI 설정 (핫바 그리기) ===
pub fn setup_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    // 2D 카메라
    commands.spawn(Camera2dBundle {
        camera: Camera { order: 1, ..default() },
        ..default()
    });

    // 1. 크로스헤어 (중앙 십자선)
    commands.spawn(NodeBundle {
        style: Style {
            width: Val::Percent(100.0), height: Val::Percent(100.0),
            justify_content: JustifyContent::Center, align_items: AlignItems::Center,
            position_type: PositionType::Absolute, ..default()
        }, ..default()
    }).with_children(|parent| {
        parent.spawn(NodeBundle { style: Style { width: Val::Px(16.0), height: Val::Px(2.0), position_type: PositionType::Absolute, ..default() }, background_color: Color::srgba(1.0, 1.0, 1.0, 0.8).into(), ..default() });
        parent.spawn(NodeBundle { style: Style { width: Val::Px(2.0), height: Val::Px(16.0), position_type: PositionType::Absolute, ..default() }, background_color: Color::srgba(1.0, 1.0, 1.0, 0.8).into(), ..default() });
    });

    // 2. 하단 핫바
    commands.spawn(NodeBundle {
        style: Style {
            width: Val::Percent(100.0), height: Val::Percent(100.0),
            justify_content: JustifyContent::Center, align_items: AlignItems::FlexEnd,
            padding: UiRect::bottom(Val::Px(20.0)),
            ..default()
        }, ..default()
    }).with_children(|parent| {
        // 슬롯 5개 생성
        for i in 1..=5 {
            // [수정] Color::RED / Color::BLACK 대신 srgba 사용
            let border_color = if i == 1 { 
                Color::srgba(1.0, 0.0, 0.0, 1.0) // 빨강
            } else { 
                Color::srgba(0.0, 0.0, 0.0, 1.0) // 검정
            };

            parent.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(50.0), height: Val::Px(50.0),
                    margin: UiRect::all(Val::Px(5.0)),
                    border: UiRect::all(Val::Px(4.0)),
                    justify_content: JustifyContent::Center, align_items: AlignItems::Center,
                    ..default()
                },
                border_color: border_color.into(),
                background_color: Color::srgba(0.2, 0.2, 0.2, 0.8).into(), // 반투명 검정 배경
                ..default()
            })
            .insert(HotbarSlot { index: i })
            .with_children(|slot| {
                // 숫자 텍스트
                slot.spawn(TextBundle::from_section(
                    i.to_string(),
                    TextStyle { 
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"), 
                        font_size: 30.0, 
                        // [수정] Color::WHITE -> srgba
                        color: Color::srgba(1.0, 1.0, 1.0, 1.0) 
                    }
                ));
            });
        }
    });
}

// === 2. 인벤토리 입력 처리 (1~5 키) ===
pub fn update_inventory_input(
    keyboard: Res<ButtonInput<KeyCode>>, 
    mut inventory: ResMut<Inventory>,
    mut query: Query<(&mut BorderColor, &HotbarSlot)>,
) {
    let mut selected_idx = 0;

    if keyboard.just_pressed(KeyCode::Digit1) { selected_idx = 1; inventory.current_block = BLOCK_STONE; }
    if keyboard.just_pressed(KeyCode::Digit2) { selected_idx = 2; inventory.current_block = BLOCK_DIRT; }
    if keyboard.just_pressed(KeyCode::Digit3) { selected_idx = 3; inventory.current_block = BLOCK_GRASS; }
    if keyboard.just_pressed(KeyCode::Digit4) { selected_idx = 4; inventory.current_block = BLOCK_GOLD; }
    if keyboard.just_pressed(KeyCode::Digit5) { selected_idx = 5; inventory.current_block = BLOCK_DIAMOND; }

    // 키를 눌렀다면 UI 업데이트
    if selected_idx > 0 {
        for (mut border, slot) in query.iter_mut() {
            if slot.index == selected_idx {
                // [수정] Color::RED -> srgba
                *border = Color::srgba(1.0, 0.0, 0.0, 1.0).into(); 
            } else {
                // [수정] Color::BLACK -> srgba
                *border = Color::srgba(0.0, 0.0, 0.0, 1.0).into();
            }
        }
    }
}