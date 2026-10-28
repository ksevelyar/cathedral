import sys
from pathlib import Path

import bpy
from mathutils import Quaternion

sys.path.insert(0, str(Path(__file__).resolve().parent))

from box_mesh import build_box_mesh
from doorway_architecture import (
    APERTURE_DEPTH,
    create_aperture_cutter,
    create_door_mount,
    create_pocket_cutters,
)
from tower_dimensions import ENTRANCE_CLEAR_HEIGHT, calculate_beveled_edge_width

CONCRETE_COLOR = (0.30, 0.30, 0.28, 1.0)
FLOOR_AND_CEILING_THICKNESS = 0.3
WALL_THICKNESS = 0.3
CORRIDOR_WIDTH = calculate_beveled_edge_width()
CORRIDOR_LENGTH = 5.0
ROOM_CLEAR_WIDTH = 15.0
ROOM_CLEAR_LENGTH = 15.0
CORRIDOR_EXTERIOR_WIDTH = CORRIDOR_WIDTH + WALL_THICKNESS * 2.0


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
    concrete_material = bpy.data.materials.new("Underground entrance concrete")
    concrete_material.diffuse_color = CONCRETE_COLOR
    concrete_material.use_nodes = True
    principled_shader = concrete_material.node_tree.nodes.get("Principled BSDF")
    principled_shader.inputs["Base Color"].default_value = CONCRETE_COLOR
    principled_shader.inputs["Roughness"].default_value = 0.92
    return concrete_material


def create_concrete_box(entrance_collection, concrete_material, object_name, center, dimensions):
    box_mesh = build_box_mesh(f"{object_name} mesh", dimensions)
    concrete_box = bpy.data.objects.new(object_name, box_mesh)
    entrance_collection.objects.link(concrete_box)
    concrete_box.location = center
    concrete_box.data.materials.append(concrete_material)
    return concrete_box


def create_connector(entrance_collection):
    connector = bpy.data.objects.new("Entrance connector", None)
    entrance_collection.objects.link(connector)
    connector.empty_display_type = "PLAIN_AXES"
    connector.empty_display_size = 0.5
    connector.location = (0.0, 0.0, 0.0)
    return connector


def create_corridor_entrance_wall(entrance_collection, construction_collection, concrete_material):
    doorway_wall_center = (0.0, -WALL_THICKNESS * 0.5, 0.0)
    doorway_rotation = Quaternion()
    corridor_entrance_wall = create_concrete_box(
        entrance_collection,
        concrete_material,
        "Corridor entrance wall",
        doorway_wall_center,
        (
            CORRIDOR_EXTERIOR_WIDTH,
            WALL_THICKNESS,
            ENTRANCE_CLEAR_HEIGHT + FLOOR_AND_CEILING_THICKNESS * 2.0,
        ),
    )
    aperture_cutter = create_aperture_cutter(
        construction_collection,
        "Corridor entrance opening",
        CORRIDOR_WIDTH,
        ENTRANCE_CLEAR_HEIGHT,
        APERTURE_DEPTH,
        doorway_wall_center,
        doorway_rotation,
        "underground_entrance_opening",
    )
    pocket_cutters = create_pocket_cutters(
        construction_collection,
        "Corridor",
        CORRIDOR_WIDTH,
        ENTRANCE_CLEAR_HEIGHT,
        doorway_wall_center,
        doorway_rotation,
        "underground_entrance_door_pocket",
    )
    create_door_mount(entrance_collection, doorway_wall_center, doorway_rotation)
    for opening_cutter in [aperture_cutter, *pocket_cutters]:
        opening_modifier = corridor_entrance_wall.modifiers.new(opening_cutter.name, "BOOLEAN")
        opening_modifier.operation = "DIFFERENCE"
        opening_modifier.solver = "EXACT"
        opening_modifier.object = opening_cutter


def create_corridor(entrance_collection, concrete_material):
    corridor_half_width = CORRIDOR_WIDTH * 0.5
    corridor_center_y = -CORRIDOR_LENGTH * 0.5
    corridor_floor_center_z = -ENTRANCE_CLEAR_HEIGHT * 0.5 - FLOOR_AND_CEILING_THICKNESS * 0.5
    corridor_ceiling_center_z = ENTRANCE_CLEAR_HEIGHT * 0.5 + FLOOR_AND_CEILING_THICKNESS * 0.5
    corridor_wall_center_x = corridor_half_width + WALL_THICKNESS * 0.5
    corridor_wall_center_z = 0.0

    create_concrete_box(
        entrance_collection,
        concrete_material,
        "Corridor floor",
        (0.0, corridor_center_y, corridor_floor_center_z),
        (CORRIDOR_WIDTH, CORRIDOR_LENGTH, FLOOR_AND_CEILING_THICKNESS),
    )
    create_concrete_box(
        entrance_collection,
        concrete_material,
        "Corridor ceiling",
        (0.0, corridor_center_y, corridor_ceiling_center_z),
        (CORRIDOR_EXTERIOR_WIDTH, CORRIDOR_LENGTH, FLOOR_AND_CEILING_THICKNESS),
    )
    for wall_name, wall_direction in (("Corridor wall left", -1.0), ("Corridor wall right", 1.0)):
        create_concrete_box(
            entrance_collection,
            concrete_material,
            wall_name,
            (wall_direction * corridor_wall_center_x, corridor_center_y, corridor_wall_center_z),
            (WALL_THICKNESS, CORRIDOR_LENGTH, ENTRANCE_CLEAR_HEIGHT),
        )


def create_room(entrance_collection, concrete_material):
    room_front_y = -CORRIDOR_LENGTH
    room_center_y = room_front_y - ROOM_CLEAR_LENGTH * 0.5
    room_half_width = ROOM_CLEAR_WIDTH * 0.5
    room_wall_center_x = room_half_width + WALL_THICKNESS * 0.5
    room_floor_center_z = -ENTRANCE_CLEAR_HEIGHT * 0.5 - FLOOR_AND_CEILING_THICKNESS * 0.5
    room_ceiling_center_z = ENTRANCE_CLEAR_HEIGHT * 0.5 + FLOOR_AND_CEILING_THICKNESS * 0.5
    room_side_wall_center_z = 0.0
    room_end_wall_center_y = room_front_y - WALL_THICKNESS * 0.5
    room_back_wall_center_y = room_front_y - ROOM_CLEAR_LENGTH - WALL_THICKNESS * 0.5
    passage_half_width = CORRIDOR_WIDTH * 0.5
    passage_side_wall_width = (ROOM_CLEAR_WIDTH - CORRIDOR_WIDTH) * 0.5
    passage_side_wall_center_x = passage_half_width + passage_side_wall_width * 0.5

    create_concrete_box(
        entrance_collection,
        concrete_material,
        "Room floor",
        (0.0, room_center_y, room_floor_center_z),
        (ROOM_CLEAR_WIDTH, ROOM_CLEAR_LENGTH, FLOOR_AND_CEILING_THICKNESS),
    )
    create_concrete_box(
        entrance_collection,
        concrete_material,
        "Room ceiling",
        (0.0, room_center_y, room_ceiling_center_z),
        (ROOM_CLEAR_WIDTH, ROOM_CLEAR_LENGTH, FLOOR_AND_CEILING_THICKNESS),
    )
    for wall_name, wall_direction in (("Room wall left", -1.0), ("Room wall right", 1.0)):
        create_concrete_box(
            entrance_collection,
            concrete_material,
            wall_name,
            (wall_direction * room_wall_center_x, room_center_y, room_side_wall_center_z),
            (WALL_THICKNESS, ROOM_CLEAR_LENGTH, ENTRANCE_CLEAR_HEIGHT),
        )
    create_concrete_box(
        entrance_collection,
        concrete_material,
        "Room back wall",
        (0.0, room_back_wall_center_y, room_side_wall_center_z),
        (ROOM_CLEAR_WIDTH, WALL_THICKNESS, ENTRANCE_CLEAR_HEIGHT),
    )
    for wall_name, wall_direction in (("Room front wall left", -1.0), ("Room front wall right", 1.0)):
        create_concrete_box(
            entrance_collection,
            concrete_material,
            wall_name,
            (wall_direction * passage_side_wall_center_x, room_end_wall_center_y, room_side_wall_center_z),
            (passage_side_wall_width, WALL_THICKNESS, ENTRANCE_CLEAR_HEIGHT),
        )


def save_scene_and_export_entrance(entrance_collection):
    model_directory = Path(__file__).resolve().parent
    blend_path = model_directory / "entrance-to-underground-level.blend"
    glb_path = model_directory / "entrance-to-underground-level.glb"
    model_directory.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(blend_path))
    bpy.ops.object.select_all(action="DESELECT")
    for entrance_object in entrance_collection.objects:
        entrance_object.select_set(True)
    bpy.context.view_layer.objects.active = entrance_collection.objects[0]
    bpy.ops.export_scene.gltf(
        filepath=str(glb_path),
        export_format="GLB",
        use_selection=True,
        export_apply=True,
        export_extras=True,
    )


def main():
    clear_scene()
    entrance_collection = create_collection("Underground entrance")
    construction_collection = create_collection("Underground entrance construction")
    concrete_material = create_concrete_material()
    create_connector(entrance_collection)
    create_corridor(entrance_collection, concrete_material)
    create_corridor_entrance_wall(entrance_collection, construction_collection, concrete_material)
    create_room(entrance_collection, concrete_material)
    save_scene_and_export_entrance(entrance_collection)


main()
