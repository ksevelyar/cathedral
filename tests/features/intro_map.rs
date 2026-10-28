use avian3d::prelude::{Collider, RigidBody};
use bevy::prelude::*;
use terminal::app::build_test_app;
use terminal::maps::{MapLoadingDependency, SandField, SandFieldSurface, build_intro};
use terminal::player::Player;
use terminal::state::GameState;

const ENTRANCE_ALIGNMENT_TOLERANCE: f32 = 0.01;
const TOWER_WALL_THICKNESS: f32 = 0.5;

#[test]
fn underground_entrances_align_at_each_tower_connector() {
    let mut app = build_test_app(build_intro());
    let tower_root = find_named_entity(&mut app, "Intro tower");

    for level_number in 1..=4 {
        let level_marker_name = format!("Level -{level_number}");
        let entrance_root_name = format!("Entrance to Level -{level_number}");
        let level_marker = find_named_descendant(&mut app, tower_root, &level_marker_name);
        let entrance_root = find_named_entity(&mut app, &entrance_root_name);
        let entrance_connector = find_named_descendant(&mut app, entrance_root, "Entrance connector");
        let level_marker_transform = *app
            .world()
            .get::<GlobalTransform>(level_marker)
            .expect("tower level marker should have a global transform");
        let entrance_connector_transform = *app
            .world()
            .get::<GlobalTransform>(entrance_connector)
            .expect("entrance connector should have a global transform");
        let (level_marker_scale, level_marker_rotation, level_marker_position) =
            level_marker_transform.to_scale_rotation_translation();
        let (entrance_connector_scale, entrance_connector_rotation, entrance_connector_position) =
            entrance_connector_transform.to_scale_rotation_translation();

        assert!(
            level_marker_position.distance(entrance_connector_position) <= ENTRANCE_ALIGNMENT_TOLERANCE,
            "{entrance_root_name} connector position should match {level_marker_name}"
        );
        assert!(
            level_marker_rotation.angle_between(entrance_connector_rotation) <= ENTRANCE_ALIGNMENT_TOLERANCE,
            "{entrance_root_name} connector rotation should match {level_marker_name}"
        );
        assert!(
            level_marker_scale.distance(entrance_connector_scale) <= ENTRANCE_ALIGNMENT_TOLERANCE,
            "{entrance_root_name} connector scale should match {level_marker_name}"
        );
    }
}

#[test]
fn underground_passage_width_and_height_match_each_beveled_edge() {
    let mut app = build_test_app(build_intro());
    let tower_root = find_named_entity(&mut app, "Intro tower");

    for level_number in 1..=4 {
        let level_marker_name = format!("Level -{level_number}");
        let entrance_root_name = format!("Entrance to Level -{level_number}");
        let level_marker = find_named_descendant(&mut app, tower_root, &level_marker_name);
        let entrance_root = find_named_entity(&mut app, &entrance_root_name);
        let entrance_connector = find_named_descendant(&mut app, entrance_root, "Entrance connector");
        let connector_transform = *app
            .world()
            .get::<GlobalTransform>(entrance_connector)
            .expect("entrance connector should have a global transform");
        let measured_beveled_edge_length = calculate_beveled_edge_length(&mut app, tower_root, connector_transform);
        let left_wall = find_named_descendant_with_collider(&mut app, entrance_root, "Corridor wall left");
        let right_wall = find_named_descendant_with_collider(&mut app, entrance_root, "Corridor wall right");
        let corridor_floor = find_named_descendant_with_collider(&mut app, entrance_root, "Corridor floor");
        let corridor_ceiling = find_named_descendant_with_collider(&mut app, entrance_root, "Corridor ceiling");
        let left_wall_vertices = find_collider_vertices_in_connector_space(&app, left_wall, connector_transform);
        let right_wall_vertices = find_collider_vertices_in_connector_space(&app, right_wall, connector_transform);
        let corridor_floor_vertices =
            find_collider_vertices_in_connector_space(&app, corridor_floor, connector_transform);
        let corridor_ceiling_vertices =
            find_collider_vertices_in_connector_space(&app, corridor_ceiling, connector_transform);
        let left_wall_inner_edge = left_wall_vertices
            .iter()
            .map(|vertex| vertex.x)
            .reduce(f32::max)
            .expect("left corridor wall should have vertices");
        let right_wall_inner_edge = right_wall_vertices
            .iter()
            .map(|vertex| vertex.x)
            .reduce(f32::min)
            .expect("right corridor wall should have vertices");
        let corridor_floor_top = corridor_floor_vertices
            .iter()
            .map(|vertex| vertex.y)
            .reduce(f32::max)
            .expect("corridor floor should have vertices");
        let corridor_ceiling_bottom = corridor_ceiling_vertices
            .iter()
            .map(|vertex| vertex.y)
            .reduce(f32::min)
            .expect("corridor ceiling should have vertices");
        let passage_width = right_wall_inner_edge - left_wall_inner_edge;
        let passage_height = corridor_ceiling_bottom - corridor_floor_top;
        let level_marker_transform = *app
            .world()
            .get::<GlobalTransform>(level_marker)
            .expect("tower level marker should have a global transform");
        let connector_matches_beveled_face = level_marker_transform
            .translation()
            .distance(connector_transform.translation())
            <= ENTRANCE_ALIGNMENT_TOLERANCE;
        let tower_root_transform = *app
            .world()
            .get::<GlobalTransform>(tower_root)
            .expect("tower root should have a global transform");
        let tower_center_in_connector_space = connector_transform
            .affine()
            .inverse()
            .transform_point3(tower_root_transform.translation());
        let level_marker_faces_outward = tower_center_in_connector_space.z < -ENTRANCE_ALIGNMENT_TOLERANCE;

        assert!(
            (passage_width - measured_beveled_edge_length).abs() <= ENTRANCE_ALIGNMENT_TOLERANCE,
            "{entrance_root_name} clear width should equal its measured bevel segment: passage={passage_width}, bevel={measured_beveled_edge_length}"
        );
        assert!(
            (passage_height - 6.0).abs() <= ENTRANCE_ALIGNMENT_TOLERANCE,
            "{entrance_root_name} clear height should be 6 m: passage={passage_height}"
        );
        assert!(
            connector_matches_beveled_face && level_marker_faces_outward,
            "{entrance_root_name} connector should define and face outward from the beveled-face plane"
        );
    }
}

#[test]
fn every_underground_corridor_collider_stays_outside_its_beveled_face() {
    let mut app = build_test_app(build_intro());

    for level_number in 1..=4 {
        let entrance_root_name = format!("Entrance to Level -{level_number}");
        let entrance_root = find_named_entity(&mut app, &entrance_root_name);
        let entrance_connector = find_named_descendant(&mut app, entrance_root, "Entrance connector");
        let connector_transform = *app
            .world()
            .get::<GlobalTransform>(entrance_connector)
            .expect("entrance connector should have a global transform");

        for corridor_component_name in [
            "Corridor floor",
            "Corridor ceiling",
            "Corridor wall left",
            "Corridor wall right",
        ] {
            let corridor_component =
                find_named_descendant_with_collider(&mut app, entrance_root, corridor_component_name);
            let corridor_component_vertices =
                find_collider_vertices_in_connector_space(&app, corridor_component, connector_transform);
            let corridor_component_minimum_depth = corridor_component_vertices
                .iter()
                .map(|vertex| vertex.z)
                .reduce(f32::min)
                .expect("corridor component should have vertices");
            let corridor_component_maximum_depth = corridor_component_vertices
                .iter()
                .map(|vertex| vertex.z)
                .reduce(f32::max)
                .expect("corridor component should have vertices");
            let corridor_component_touches_face = corridor_component_vertices
                .iter()
                .any(|vertex| vertex.z.abs() <= ENTRANCE_ALIGNMENT_TOLERANCE);

            assert!(
                corridor_component_minimum_depth >= -ENTRANCE_ALIGNMENT_TOLERANCE,
                "{entrance_root_name} {corridor_component_name} must not enter the tower"
            );
            assert!(
                corridor_component_touches_face && corridor_component_maximum_depth > ENTRANCE_ALIGNMENT_TOLERANCE,
                "{entrance_root_name} {corridor_component_name} should start at the beveled face and extend outward"
            );
        }
    }
}

#[test]
fn underground_entrance_keeps_both_adjacent_straight_faces_intact() {
    let mut app = build_test_app(build_intro());
    let tower_root = find_named_entity(&mut app, "Intro tower");
    let tower_shell = find_named_descendant_with_collider(&mut app, tower_root, "Tower shell");

    for level_number in 1..=4 {
        let entrance_root_name = format!("Entrance to Level -{level_number}");
        let entrance_root = find_named_entity(&mut app, &entrance_root_name);
        let entrance_connector = find_named_descendant(&mut app, entrance_root, "Entrance connector");
        let connector_transform = *app
            .world()
            .get::<GlobalTransform>(entrance_connector)
            .expect("entrance connector should have a global transform");
        let measured_beveled_edge_length = calculate_beveled_edge_length(&mut app, tower_root, connector_transform);
        let tower_shell_triangles = find_collider_triangles_in_connector_space(&app, tower_shell, connector_transform);

        for straight_face_direction in [-1.0_f32, 1.0] {
            for straight_face_depth in [-0.51_f32, -0.56, -0.62, -0.69, -0.72] {
                let straight_face_local_x = if straight_face_direction > 0.0 {
                    measured_beveled_edge_length * 0.5 - TOWER_WALL_THICKNESS * 2.0_f32.sqrt() - straight_face_depth
                } else {
                    -measured_beveled_edge_length * 0.5 + TOWER_WALL_THICKNESS * 2.0_f32.sqrt() + straight_face_depth
                };

                for entrance_height in [-1.9_f32, -1.0, 0.0, 1.0, 1.9] {
                    let straight_face_point = Vec3::new(straight_face_local_x, entrance_height, straight_face_depth);
                    assert!(
                        collider_surface_contains_point(&tower_shell_triangles, straight_face_point),
                        "{entrance_root_name} adjacent straight face should remain closed at {straight_face_point:?}"
                    );
                }
            }
        }

        for entrance_height in [-1.9_f32, 0.0, 1.9] {
            for passage_width_fraction in [0.02_f32, 0.25, 0.5, 0.75, 0.98] {
                let passage_width_position = measured_beveled_edge_length * (passage_width_fraction - 0.5);
                let opening_plane_point = Vec3::new(passage_width_position, entrance_height, 0.0);

                assert!(
                    !collider_surface_contains_point(&tower_shell_triangles, opening_plane_point),
                    "{entrance_root_name} should clear only the beveled-face passage at {opening_plane_point:?}"
                );
            }
        }
    }
}

#[test]
fn underground_passage_floor_connects_landing_to_corridor_without_an_extra_slab() {
    let mut app = build_test_app(build_intro());
    let tower_root = find_named_entity(&mut app, "Intro tower");
    let tower_shell = find_named_descendant_with_collider(&mut app, tower_root, "Tower shell");
    let inner_beveled_edge_endpoint_inset = TOWER_WALL_THICKNESS * (2.0_f32.sqrt() - 1.0);
    let connection_depth_samples = [-1.0_f32, -0.75, -0.5, -0.25, 0.0, 0.25];
    let passage_width_sample_count = 11;

    for level_number in 1..=4 {
        let entrance_root_name = format!("Entrance to Level -{level_number}");
        let entrance_root = find_named_entity(&mut app, &entrance_root_name);
        let entrance_connector = find_named_descendant(&mut app, entrance_root, "Entrance connector");
        let connector_transform = *app
            .world()
            .get::<GlobalTransform>(entrance_connector)
            .expect("entrance connector should have a global transform");
        let landing_name = format!("Corner landing Lower flight {level_number}");
        let landing = find_named_descendant_with_collider(&mut app, tower_root, &landing_name);
        let corridor_floor = find_named_descendant_with_collider(&mut app, entrance_root, "Corridor floor");
        let tower_shell_triangles = find_collider_triangles_in_connector_space(&app, tower_shell, connector_transform);
        let landing_triangles = find_collider_triangles_in_connector_space(&app, landing, connector_transform);
        let corridor_floor_triangles =
            find_collider_triangles_in_connector_space(&app, corridor_floor, connector_transform);
        let measured_beveled_edge_length = calculate_beveled_edge_length(&mut app, tower_root, connector_transform);
        let corridor_floor_vertices =
            find_collider_vertices_in_connector_space(&app, corridor_floor, connector_transform);
        let landing_vertices = find_collider_vertices_in_connector_space(&app, landing, connector_transform);
        let corridor_floor_top = corridor_floor_vertices
            .iter()
            .map(|vertex| vertex.y)
            .reduce(f32::max)
            .expect("corridor floor should have vertices");
        let corridor_floor_minimum_width_position = corridor_floor_vertices
            .iter()
            .map(|vertex| vertex.x)
            .reduce(f32::min)
            .expect("corridor floor should have vertices");
        let corridor_floor_maximum_width_position = corridor_floor_vertices
            .iter()
            .map(|vertex| vertex.x)
            .reduce(f32::max)
            .expect("corridor floor should have vertices");
        let corridor_floor_connection_width =
            corridor_floor_maximum_width_position - corridor_floor_minimum_width_position;
        let landing_top = landing_vertices
            .iter()
            .map(|vertex| vertex.y)
            .reduce(f32::max)
            .expect("tower landing should have vertices");
        let connection_floor_surfaces = [&tower_shell_triangles, &landing_triangles, &corridor_floor_triangles];
        let landing_connector_floor_name = format!("Level -{level_number} landing connector floor");

        assert!(
            (corridor_floor_top - landing_top).abs() <= ENTRANCE_ALIGNMENT_TOLERANCE,
            "{entrance_root_name} landing and corridor floor should meet at one height: landing={landing_top}, corridor={corridor_floor_top}"
        );
        assert!(
            (corridor_floor_connection_width - measured_beveled_edge_length).abs() <= ENTRANCE_ALIGNMENT_TOLERANCE,
            "{entrance_root_name} corridor floor should meet the beveled face without extending past its endpoints"
        );
        assert!(
            find_named_descendant_or_none(&mut app, tower_root, &landing_connector_floor_name).is_none(),
            "{entrance_root_name} should not have an extra landing connector slab"
        );

        for connection_depth in connection_depth_samples {
            let inner_beveled_edge_length = measured_beveled_edge_length - inner_beveled_edge_endpoint_inset * 2.0;
            let passage_width_at_connection_depth = if (-TOWER_WALL_THICKNESS..0.0).contains(&connection_depth) {
                let wall_depth_fraction = (connection_depth + TOWER_WALL_THICKNESS) / TOWER_WALL_THICKNESS;
                inner_beveled_edge_length
                    + (measured_beveled_edge_length - inner_beveled_edge_length) * wall_depth_fraction
            } else {
                measured_beveled_edge_length
            };

            for width_sample_index in 0..passage_width_sample_count {
                let passage_width_fraction = width_sample_index as f32 / (passage_width_sample_count - 1) as f32;
                let passage_width_position = -passage_width_at_connection_depth * 0.5
                    + passage_width_at_connection_depth * passage_width_fraction;
                let connection_floor_point = Vec3::new(passage_width_position, corridor_floor_top, connection_depth);
                let connection_floor_is_covered = connection_floor_surfaces.iter().any(|surface_triangles| {
                    collider_surface_contains_point(surface_triangles, connection_floor_point)
                });

                assert!(
                    connection_floor_is_covered,
                    "{entrance_root_name} passage floor should continuously cover its clear width at {connection_floor_point:?}"
                );
            }
        }
    }
}

fn find_named_entity(app: &mut App, expected_name: &str) -> Entity {
    app.world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find(|(_, name)| name.as_str() == expected_name)
        .map_or_else(
            || panic!("entity named {expected_name} should exist"),
            |(entity, _)| entity,
        )
}

fn find_named_descendant(app: &mut App, root: Entity, expected_name: &str) -> Entity {
    find_named_descendant_or_none(app, root, expected_name)
        .unwrap_or_else(|| panic!("descendant named {expected_name} should exist under {root:?}"))
}

fn find_named_descendant_or_none(app: &mut App, root: Entity, expected_name: &str) -> Option<Entity> {
    let world = app.world_mut();
    let mut entities_to_visit = world
        .get::<Children>(root)
        .map_or_else(Vec::new, |children| children.iter().collect::<Vec<_>>());

    while let Some(entity) = entities_to_visit.pop() {
        if world
            .get::<Name>(entity)
            .is_some_and(|name| name.as_str() == expected_name)
        {
            return Some(entity);
        }
        if let Some(children) = world.get::<Children>(entity) {
            entities_to_visit.extend(children.iter());
        }
    }

    None
}

fn find_named_descendant_with_collider(app: &mut App, root: Entity, expected_name: &str) -> Entity {
    let named_entity = find_named_descendant(app, root, expected_name);
    let world = app.world_mut();
    let mut entities_to_visit = vec![named_entity];

    while let Some(entity) = entities_to_visit.pop() {
        if world.get::<Collider>(entity).is_some() {
            return entity;
        }
        if let Some(children) = world.get::<Children>(entity) {
            entities_to_visit.extend(children.iter());
        }
    }

    panic!("{expected_name} should contain a collider");
}

fn find_collider_vertices_in_connector_space(
    app: &App,
    collider_entity: Entity,
    connector_transform: GlobalTransform,
) -> Vec<Vec3> {
    let collider = app
        .world()
        .get::<Collider>(collider_entity)
        .expect("named component should have a collider");
    let collider_mesh = collider
        .shape()
        .as_trimesh()
        .expect("named collider should be a triangle mesh");
    let collider_global_transform = *app
        .world()
        .get::<GlobalTransform>(collider_entity)
        .expect("named collider should have a global transform");
    let connector_inverse = connector_transform.affine().inverse();
    collider_mesh
        .vertices()
        .iter()
        .map(|collider_vertex| {
            let collider_local_vertex = Vec3::new(collider_vertex.x, collider_vertex.y, collider_vertex.z);
            let collider_world_vertex = collider_global_transform
                .affine()
                .transform_point3(collider_local_vertex);
            connector_inverse.transform_point3(collider_world_vertex)
        })
        .collect()
}

fn find_collider_triangles_in_connector_space(
    app: &App,
    collider_entity: Entity,
    connector_transform: GlobalTransform,
) -> Vec<[Vec3; 3]> {
    let collider = app
        .world()
        .get::<Collider>(collider_entity)
        .expect("named component should have a collider");
    let collider_mesh = collider
        .shape()
        .as_trimesh()
        .expect("named collider should be a triangle mesh");
    let collider_global_transform = *app
        .world()
        .get::<GlobalTransform>(collider_entity)
        .expect("named collider should have a global transform");
    let connector_inverse = connector_transform.affine().inverse();
    collider_mesh
        .indices()
        .iter()
        .map(|triangle_indices| {
            triangle_indices.map(|vertex_index| {
                let collider_vertex = collider_mesh.vertices()[vertex_index as usize];
                let collider_local_vertex = Vec3::new(collider_vertex.x, collider_vertex.y, collider_vertex.z);
                let collider_world_vertex = collider_global_transform
                    .affine()
                    .transform_point3(collider_local_vertex);
                connector_inverse.transform_point3(collider_world_vertex)
            })
        })
        .collect()
}

fn calculate_beveled_edge_length(app: &mut App, tower_root: Entity, connector_transform: GlobalTransform) -> f32 {
    let connector_inverse = connector_transform.affine().inverse();
    let beveled_edge_vertices = (0..8)
        .filter_map(|vertex_index| {
            let vertex_name = format!("Terrain cutout vertex {vertex_index:02}");
            let vertex_entity = find_named_descendant(app, tower_root, &vertex_name);
            let vertex_transform = *app
                .world()
                .get::<GlobalTransform>(vertex_entity)
                .expect("terrain cutout vertex should have a global transform");
            let connector_space_vertex = connector_inverse.transform_point3(vertex_transform.translation());
            (connector_space_vertex.z.abs() <= ENTRANCE_ALIGNMENT_TOLERANCE)
                .then_some(Vec2::new(connector_space_vertex.x, connector_space_vertex.z))
        })
        .collect::<Vec<_>>();

    assert_eq!(
        beveled_edge_vertices.len(),
        2,
        "the beveled-face plane should contain exactly two tower profile vertices"
    );
    beveled_edge_vertices[0].distance(beveled_edge_vertices[1])
}

fn collider_surface_contains_point(surface_triangles: &[[Vec3; 3]], point: Vec3) -> bool {
    let surface_tolerance = ENTRANCE_ALIGNMENT_TOLERANCE;
    surface_triangles.iter().any(|triangle| {
        let first_edge = triangle[1] - triangle[0];
        let second_edge = triangle[2] - triangle[0];
        let point_offset = point - triangle[0];
        let triangle_normal = first_edge.cross(second_edge);
        let triangle_area_squared = triangle_normal.length_squared();
        let point_distance_from_plane = point_offset.dot(triangle_normal).abs() / triangle_area_squared.sqrt();
        let longest_triangle_edge = first_edge.length().max(second_edge.length());
        let barycentric_tolerance = surface_tolerance / longest_triangle_edge;
        let first_edge_squared = first_edge.length_squared();
        let first_second_edge_dot = first_edge.dot(second_edge);
        let second_edge_squared = second_edge.length_squared();
        let first_point_edge_dot = point_offset.dot(first_edge);
        let second_point_edge_dot = point_offset.dot(second_edge);
        let barycentric_denominator = first_edge_squared * second_edge_squared - first_second_edge_dot.powi(2);
        let first_barycentric_coordinate = (second_edge_squared * first_point_edge_dot
            - first_second_edge_dot * second_point_edge_dot)
            / barycentric_denominator;
        let second_barycentric_coordinate = (first_edge_squared * second_point_edge_dot
            - first_second_edge_dot * first_point_edge_dot)
            / barycentric_denominator;

        point_distance_from_plane <= surface_tolerance
            && first_barycentric_coordinate >= -barycentric_tolerance
            && second_barycentric_coordinate >= -barycentric_tolerance
            && first_barycentric_coordinate + second_barycentric_coordinate <= 1.0 + barycentric_tolerance
    })
}

#[test]
fn terrain_samples_remain_finite_and_smooth_at_two_meter_intervals() {
    for grid_x in (-300..=300).step_by(10) {
        for grid_z in (-300..=300).step_by(10) {
            let horizontal_position = Vec2::new(grid_x as f32, grid_z as f32);
            let terrain_height = SandField::calculate_height_at(horizontal_position);
            let eastward_terrain_height = SandField::calculate_height_at(horizontal_position + Vec2::X * 2.0);
            let northward_terrain_height = SandField::calculate_height_at(horizontal_position + Vec2::Y * 2.0);
            assert!(terrain_height.is_finite());
            assert!((eastward_terrain_height - terrain_height).abs() < 1.5);
            assert!((northward_terrain_height - terrain_height).abs() < 1.5);
        }
    }
}

#[test]
fn intro_map_enters_playing_with_sandfield_and_player_above_terrain() {
    let mut app = build_test_app(build_intro());

    assert_eq!(app.world().resource::<State<GameState>>().get(), &GameState::Playing);

    let remaining_map_loading_dependencies = app
        .world_mut()
        .query_filtered::<Entity, With<MapLoadingDependency>>()
        .iter(app.world())
        .count();
    assert_eq!(remaining_map_loading_dependencies, 0);

    let static_sandfield_count = app
        .world_mut()
        .query_filtered::<&RigidBody, (With<SandFieldSurface>, With<Collider>)>()
        .iter(app.world())
        .filter(|rigid_body| **rigid_body == RigidBody::Static)
        .count();
    assert_eq!(static_sandfield_count, 1);

    let player_position = app
        .world_mut()
        .query_filtered::<&Transform, With<Player>>()
        .single(app.world())
        .expect("intro should spawn one player")
        .translation;
    let terrain_height = SandField::calculate_height_at(player_position.xz());
    assert!(
        player_position.y > terrain_height,
        "player should start above the terrain at y={}, terrain y={terrain_height}",
        player_position.y
    );
}
