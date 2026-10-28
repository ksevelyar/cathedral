from pathlib import Path
import sys

import bpy

sys.path.insert(0, str(Path(__file__).resolve().parent))

from tower_dimensions import (
    DOOR_PANEL_CENTER_CHAMFER_DEPTH_FRACTION,
    DOOR_PANEL_CENTER_CHAMFER_WIDTH,
    DOOR_PANEL_THICKNESS,
    DOOR_TOP_WIDTH_FRACTION,
    ENTRANCE_CLEAR_HEIGHT,
    calculate_beveled_edge_width,
)

DOOR_FRAME_THICKNESS = 0.28


def clear_scene():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    for collection in tuple(bpy.data.collections):
        bpy.data.collections.remove(collection)
    for mesh_data in tuple(bpy.data.meshes):
        bpy.data.meshes.remove(mesh_data)
    for material_data in tuple(bpy.data.materials):
        bpy.data.materials.remove(material_data)


def create_collection(collection_name):
    collection = bpy.data.collections.new(collection_name)
    bpy.context.scene.collection.children.link(collection)
    return collection


def create_concrete_material():
    concrete_color = (0.30, 0.30, 0.28, 1.0)
    concrete_material = bpy.data.materials.new("Tower concrete")
    concrete_material.diffuse_color = concrete_color
    concrete_material.use_nodes = True
    principled_shader = concrete_material.node_tree.nodes.get("Principled BSDF")
    principled_shader.inputs["Base Color"].default_value = concrete_color
    principled_shader.inputs["Roughness"].default_value = 0.92
    return concrete_material


def build_prism_mesh(mesh_name, profiles, depth):
    vertices = []
    faces = []
    for profile in profiles:
        first_vertex_index = len(vertices)
        profile_vertex_count = len(profile)
        for depth_coordinate in (-depth * 0.5, depth * 0.5):
            vertices.extend((horizontal_coordinate, depth_coordinate, vertical_coordinate) for horizontal_coordinate, vertical_coordinate in profile)
        faces.append(tuple(first_vertex_index + vertex_index for vertex_index in range(profile_vertex_count)))
        faces.append(tuple(first_vertex_index + profile_vertex_count + vertex_index for vertex_index in reversed(range(profile_vertex_count))))
        for vertex_index in range(profile_vertex_count):
            next_vertex_index = (vertex_index + 1) % profile_vertex_count
            faces.append((
                first_vertex_index + vertex_index,
                first_vertex_index + next_vertex_index,
                first_vertex_index + profile_vertex_count + next_vertex_index,
                first_vertex_index + profile_vertex_count + vertex_index,
            ))
    prism_mesh = bpy.data.meshes.new(mesh_name)
    prism_mesh.from_pydata(vertices, [], faces)
    prism_mesh.update()
    return prism_mesh


def create_concrete_object(door_collection, concrete_material, object_name, profiles, depth):
    concrete_object = bpy.data.objects.new(object_name, build_prism_mesh(f"{object_name} mesh", profiles, depth))
    door_collection.objects.link(concrete_object)
    concrete_object.data.materials.append(concrete_material)
    return concrete_object


def build_door_panel_mesh(
    mesh_name,
    panel_profile,
    panel_depth,
    center_edge_horizontal_position,
    center_edge_horizontal_direction,
):
    half_panel_depth = panel_depth * 0.5
    center_chamfer_depth = panel_depth * DOOR_PANEL_CENTER_CHAMFER_DEPTH_FRACTION
    front_center_chamfer_depth_position = -half_panel_depth + center_chamfer_depth
    rear_center_chamfer_depth_position = half_panel_depth - center_chamfer_depth
    center_edge_profile_indices = [
        profile_index
        for profile_index, (horizontal_coordinate, _) in enumerate(panel_profile)
        if horizontal_coordinate == center_edge_horizontal_position
    ]
    center_edge_lower_profile_index = min(
        center_edge_profile_indices,
        key=lambda profile_index: panel_profile[profile_index][1],
    )
    center_edge_upper_profile_index = max(
        center_edge_profile_indices,
        key=lambda profile_index: panel_profile[profile_index][1],
    )
    center_edge_lower_height = panel_profile[center_edge_lower_profile_index][1]
    center_edge_upper_height = panel_profile[center_edge_upper_profile_index][1]
    front_panel_profile = [
        (
            horizontal_coordinate - center_edge_horizontal_direction * DOOR_PANEL_CENTER_CHAMFER_WIDTH
            if horizontal_coordinate == center_edge_horizontal_position
            else horizontal_coordinate,
            vertical_coordinate,
        )
        for horizontal_coordinate, vertical_coordinate in panel_profile
    ]
    panel_vertices = [
        (horizontal_coordinate, -half_panel_depth, vertical_coordinate)
        for horizontal_coordinate, vertical_coordinate in front_panel_profile
    ]
    panel_vertices.extend(
        (horizontal_coordinate, half_panel_depth, vertical_coordinate)
        for horizontal_coordinate, vertical_coordinate in front_panel_profile
    )
    panel_profile_vertex_count = len(panel_profile)
    rear_panel_profile_vertex_offset = panel_profile_vertex_count
    front_center_edge_lower_vertex_index = center_edge_lower_profile_index
    front_center_edge_upper_vertex_index = center_edge_upper_profile_index
    rear_center_edge_lower_vertex_index = rear_panel_profile_vertex_offset + center_edge_lower_profile_index
    rear_center_edge_upper_vertex_index = rear_panel_profile_vertex_offset + center_edge_upper_profile_index
    front_structural_lower_vertex_index = len(panel_vertices)
    panel_vertices.extend([
        (center_edge_horizontal_position, front_center_chamfer_depth_position, center_edge_lower_height),
        (center_edge_horizontal_position, front_center_chamfer_depth_position, center_edge_upper_height),
        (center_edge_horizontal_position, rear_center_chamfer_depth_position, center_edge_lower_height),
        (center_edge_horizontal_position, rear_center_chamfer_depth_position, center_edge_upper_height),
    ])
    front_structural_upper_vertex_index = front_structural_lower_vertex_index + 1
    rear_structural_lower_vertex_index = front_structural_lower_vertex_index + 2
    rear_structural_upper_vertex_index = front_structural_lower_vertex_index + 3
    panel_faces = [
        tuple(range(panel_profile_vertex_count)),
        tuple(reversed(range(rear_panel_profile_vertex_offset, panel_profile_vertex_count * 2))),
    ]
    for profile_index, profile_point in enumerate(panel_profile):
        next_profile_index = (profile_index + 1) % panel_profile_vertex_count
        next_profile_point = panel_profile[next_profile_index]
        if profile_point[0] == center_edge_horizontal_position and next_profile_point[0] == center_edge_horizontal_position:
            continue
        panel_faces.append((
            profile_index,
            next_profile_index,
            rear_panel_profile_vertex_offset + next_profile_index,
            rear_panel_profile_vertex_offset + profile_index,
        ))
    front_center_chamfer_face = (
        front_center_edge_lower_vertex_index,
        front_structural_lower_vertex_index,
        front_structural_upper_vertex_index,
        front_center_edge_upper_vertex_index,
    )
    rear_center_chamfer_face = (
        rear_center_edge_lower_vertex_index,
        rear_center_edge_upper_vertex_index,
        rear_structural_upper_vertex_index,
        rear_structural_lower_vertex_index,
    )
    center_structural_face = (
        front_structural_lower_vertex_index,
        rear_structural_lower_vertex_index,
        rear_structural_upper_vertex_index,
        front_structural_upper_vertex_index,
    )
    top_center_cap_face = (
        front_center_edge_upper_vertex_index,
        front_structural_upper_vertex_index,
        rear_structural_upper_vertex_index,
        rear_center_edge_upper_vertex_index,
    )
    bottom_center_cap_face = (
        front_center_edge_lower_vertex_index,
        rear_center_edge_lower_vertex_index,
        rear_structural_lower_vertex_index,
        front_structural_lower_vertex_index,
    )
    if center_edge_horizontal_direction < 0.0:
        front_center_chamfer_face = tuple(reversed(front_center_chamfer_face))
        rear_center_chamfer_face = tuple(reversed(rear_center_chamfer_face))
        center_structural_face = tuple(reversed(center_structural_face))
        top_center_cap_face = tuple(reversed(top_center_cap_face))
        bottom_center_cap_face = tuple(reversed(bottom_center_cap_face))
    panel_faces.extend((
        front_center_chamfer_face,
        rear_center_chamfer_face,
        center_structural_face,
        top_center_cap_face,
        bottom_center_cap_face,
    ))
    door_panel_mesh = bpy.data.meshes.new(mesh_name)
    door_panel_mesh.from_pydata(panel_vertices, [], panel_faces)
    door_panel_mesh.update()
    return door_panel_mesh


def create_door_panel(
    door_collection,
    concrete_material,
    panel_name,
    door_panel_profile,
    door_panel_width,
    center_edge_horizontal_direction,
):
    center_edge_horizontal_position = center_edge_horizontal_direction * door_panel_width * 0.5
    door_panel_mesh = build_door_panel_mesh(
        f"{panel_name} mesh",
        door_panel_profile,
        DOOR_PANEL_THICKNESS,
        center_edge_horizontal_position,
        center_edge_horizontal_direction,
    )
    door_panel = bpy.data.objects.new(panel_name, door_panel_mesh)
    door_collection.objects.link(door_panel)
    door_panel.data.materials.append(concrete_material)
    return door_panel


def create_door_connector(door_collection):
    door_connector = bpy.data.objects.new("Door connector", None)
    door_collection.objects.link(door_connector)
    door_connector.empty_display_type = "PLAIN_AXES"
    door_connector.empty_display_size = 0.5


def build_frame_profiles(clear_width, top_width):
    half_clear_width = clear_width * 0.5
    half_top_width = top_width * 0.5
    half_clear_height = ENTRANCE_CLEAR_HEIGHT * 0.5
    shoulder_height = half_clear_height - ENTRANCE_CLEAR_HEIGHT / 3.0
    left_frame_profile = [
        (-half_clear_width - DOOR_FRAME_THICKNESS, -half_clear_height),
        (-half_clear_width, -half_clear_height),
        (-half_clear_width, shoulder_height),
        (-half_top_width, half_clear_height),
        (-half_top_width - DOOR_FRAME_THICKNESS, half_clear_height),
        (-half_clear_width - DOOR_FRAME_THICKNESS, shoulder_height),
    ]
    right_frame_profile = [
        (half_clear_width, -half_clear_height),
        (half_clear_width + DOOR_FRAME_THICKNESS, -half_clear_height),
        (half_clear_width + DOOR_FRAME_THICKNESS, shoulder_height),
        (half_top_width + DOOR_FRAME_THICKNESS, half_clear_height),
        (half_top_width, half_clear_height),
        (half_clear_width, shoulder_height),
    ]
    top_frame_profile = [
        (-half_top_width - DOOR_FRAME_THICKNESS, half_clear_height),
        (half_top_width + DOOR_FRAME_THICKNESS, half_clear_height),
        (half_top_width + DOOR_FRAME_THICKNESS, half_clear_height + DOOR_FRAME_THICKNESS),
        (-half_top_width - DOOR_FRAME_THICKNESS, half_clear_height + DOOR_FRAME_THICKNESS),
    ]
    return [left_frame_profile, right_frame_profile, top_frame_profile]


def build_left_panel_profile(panel_width, top_width, closed_panel_center_offset):
    half_panel_width = panel_width * 0.5
    half_top_width = top_width * 0.5
    half_clear_height = ENTRANCE_CLEAR_HEIGHT * 0.5
    shoulder_height = half_clear_height - ENTRANCE_CLEAR_HEIGHT / 3.0
    left_panel_top_outer_x = -half_top_width + closed_panel_center_offset
    return [
        (-half_panel_width, -half_clear_height),
        (half_panel_width, -half_clear_height),
        (half_panel_width, half_clear_height),
        (left_panel_top_outer_x, half_clear_height),
        (-half_panel_width, shoulder_height),
    ]


def create_door_panels(door_collection, concrete_material, clear_width, top_width):
    panel_width = clear_width * 0.5
    closed_panel_center_offset = panel_width * 0.5
    open_panel_center_offset = clear_width * 0.5 + panel_width * 0.5 + DOOR_FRAME_THICKNESS
    left_panel_profile = build_left_panel_profile(panel_width, top_width, closed_panel_center_offset)
    right_panel_profile = [(-horizontal_coordinate, vertical_coordinate) for horizontal_coordinate, vertical_coordinate in reversed(left_panel_profile)]
    left_panel_center_edge_direction = 1.0
    right_panel_center_edge_direction = -1.0
    left_panel = create_door_panel(
        door_collection,
        concrete_material,
        "Door panel left",
        left_panel_profile,
        panel_width,
        left_panel_center_edge_direction,
    )
    left_panel.location.x = -closed_panel_center_offset
    right_panel = create_door_panel(
        door_collection,
        concrete_material,
        "Door panel right",
        right_panel_profile,
        panel_width,
        right_panel_center_edge_direction,
    )
    right_panel.location.x = closed_panel_center_offset
    create_open_marker(door_collection, "Door panel left open", -open_panel_center_offset)
    create_open_marker(door_collection, "Door panel right open", open_panel_center_offset)


def create_open_marker(door_collection, marker_name, horizontal_coordinate):
    open_marker = bpy.data.objects.new(marker_name, None)
    door_collection.objects.link(open_marker)
    open_marker.empty_display_type = "PLAIN_AXES"
    open_marker.empty_display_size = 0.25
    open_marker.location.x = horizontal_coordinate


def save_scene_and_export_door(door_collection):
    model_directory = Path(__file__).resolve().parent
    blend_path = model_directory / "door.blend"
    glb_path = model_directory / "door.glb"
    bpy.ops.wm.save_as_mainfile(filepath=str(blend_path))
    bpy.ops.object.select_all(action="DESELECT")
    for door_object in door_collection.objects:
        door_object.select_set(True)
    bpy.context.view_layer.objects.active = door_collection.objects[0]
    bpy.ops.export_scene.gltf(
        filepath=str(glb_path),
        export_format="GLB",
        use_selection=True,
        export_apply=True,
        export_extras=True,
    )


def main():
    clear_scene()
    door_collection = create_collection("Door")
    concrete_material = create_concrete_material()
    door_clear_width = calculate_beveled_edge_width()
    top_width = door_clear_width * DOOR_TOP_WIDTH_FRACTION
    create_door_connector(door_collection)
    create_concrete_object(
        door_collection,
        concrete_material,
        "Door frame",
        build_frame_profiles(door_clear_width, top_width),
        DOOR_PANEL_THICKNESS,
    )
    create_door_panels(door_collection, concrete_material, door_clear_width, top_width)
    save_scene_and_export_door(door_collection)


main()
