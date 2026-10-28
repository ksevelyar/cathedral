import bpy
from mathutils import Vector

from box_mesh import build_box_mesh
from tower_dimensions import (
    DOOR_POCKET_DEPTH,
    DOOR_POCKET_OPENING_OVERLAP,
    DOOR_POCKET_PANEL_CLEARANCE,
    DOOR_POCKET_VERTICAL_OVERSHOOT,
    DOOR_TOP_WIDTH_FRACTION,
)

APERTURE_DEPTH = 4.0


def build_aperture_profile(clear_width, clear_height):
    aperture_vertical_overshoot = 0.02
    aperture_clear_height = clear_height + aperture_vertical_overshoot * 2.0
    aperture_top_width = clear_width * DOOR_TOP_WIDTH_FRACTION
    half_clear_width = clear_width * 0.5
    half_top_width = aperture_top_width * 0.5
    half_clear_height = aperture_clear_height * 0.5
    shoulder_height = half_clear_height - aperture_clear_height / 3.0
    return [
        (-half_clear_width, -half_clear_height),
        (half_clear_width, -half_clear_height),
        (half_clear_width, shoulder_height),
        (half_top_width, half_clear_height),
        (-half_top_width, half_clear_height),
        (-half_clear_width, shoulder_height),
    ]


def build_aperture_mesh(mesh_name, clear_width, clear_height, aperture_depth):
    aperture_profile = build_aperture_profile(clear_width, clear_height)
    half_aperture_depth = aperture_depth * 0.5
    aperture_vertices = [
        (horizontal_position, depth_position, vertical_position)
        for depth_position in (-half_aperture_depth, half_aperture_depth)
        for horizontal_position, vertical_position in aperture_profile
    ]
    aperture_profile_vertex_count = len(aperture_profile)
    aperture_faces = []
    for vertex_index in range(1, aperture_profile_vertex_count - 1):
        aperture_faces.append((0, vertex_index, vertex_index + 1))
        aperture_faces.append((
            aperture_profile_vertex_count,
            aperture_profile_vertex_count + vertex_index + 1,
            aperture_profile_vertex_count + vertex_index,
        ))
    for vertex_index in range(aperture_profile_vertex_count):
        next_vertex_index = (vertex_index + 1) % aperture_profile_vertex_count
        aperture_faces.append((
            vertex_index,
            aperture_profile_vertex_count + vertex_index,
            aperture_profile_vertex_count + next_vertex_index,
            next_vertex_index,
        ))
    aperture_mesh = bpy.data.meshes.new(mesh_name)
    aperture_mesh.from_pydata(aperture_vertices, [], aperture_faces)
    aperture_mesh.update()
    return aperture_mesh


def create_aperture_cutter(
    construction_collection,
    cutter_name,
    clear_width,
    clear_height,
    cutter_depth,
    cutter_center,
    cutter_rotation,
    architectural_role,
):
    aperture_mesh = build_aperture_mesh(
        f"{cutter_name} mesh",
        clear_width,
        clear_height,
        cutter_depth,
    )
    aperture_cutter = bpy.data.objects.new(cutter_name, aperture_mesh)
    construction_collection.objects.link(aperture_cutter)
    aperture_cutter.location = cutter_center
    aperture_cutter.rotation_mode = "QUATERNION"
    aperture_cutter.rotation_quaternion = cutter_rotation
    aperture_cutter.display_type = "WIRE"
    aperture_cutter.hide_render = True
    aperture_cutter["architectural_role"] = architectural_role
    return aperture_cutter


def calculate_pocket_dimensions(clear_width, clear_height):
    panel_width = clear_width * 0.5
    pocket_width = panel_width + DOOR_POCKET_PANEL_CLEARANCE
    pocket_center_offset = clear_width * 0.5 + pocket_width * 0.5 - DOOR_POCKET_OPENING_OVERLAP
    pocket_height = clear_height + DOOR_POCKET_VERTICAL_OVERSHOOT * 2.0
    return pocket_width, DOOR_POCKET_DEPTH, pocket_height, pocket_center_offset


def create_pocket_cutters(
    construction_collection,
    doorway_name,
    clear_width,
    clear_height,
    wall_center,
    wall_rotation,
    architectural_role,
):
    pocket_width, pocket_depth, pocket_height, pocket_center_offset = calculate_pocket_dimensions(
        clear_width,
        clear_height,
    )
    pocket_mesh = build_box_mesh(f"{doorway_name} door pocket mesh")
    wall_center_vector = Vector(wall_center)
    pocket_cutters = []
    for pocket_side, pocket_direction in (("left", -1.0), ("right", 1.0)):
        pocket_name = f"{doorway_name} door pocket {pocket_side}"
        pocket_cutter = bpy.data.objects.new(pocket_name, pocket_mesh)
        construction_collection.objects.link(pocket_cutter)
        pocket_local_offset = Vector((pocket_direction * pocket_center_offset, 0.0, 0.0))
        pocket_cutter.location = wall_center_vector + wall_rotation @ pocket_local_offset
        pocket_cutter.scale = (pocket_width, pocket_depth, pocket_height)
        pocket_cutter.rotation_mode = "QUATERNION"
        pocket_cutter.rotation_quaternion = wall_rotation
        pocket_cutter.display_type = "WIRE"
        pocket_cutter.hide_render = True
        pocket_cutter["architectural_role"] = architectural_role
        pocket_cutters.append(pocket_cutter)
    return pocket_cutters


def create_door_mount(mount_collection, mount_location, mount_rotation):
    door_mount = bpy.data.objects.new("Door mount", None)
    mount_collection.objects.link(door_mount)
    door_mount.empty_display_type = "PLAIN_AXES"
    door_mount.empty_display_size = 0.5
    door_mount.location = mount_location
    door_mount.rotation_mode = "QUATERNION"
    door_mount.rotation_quaternion = mount_rotation
    return door_mount
