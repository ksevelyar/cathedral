import math
import sys
from pathlib import Path

import bpy
from mathutils import Matrix, Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))

from stairwell import build_stair_railing_mesh, create_stair_railing_segment

TOWER_HEIGHT = 54.0
TOWER_BASE_WIDTH = 36.0
TOWER_TOP_WIDTH = 28.0
TOWER_BASE_CORNER_CUT = 3.6
WALL_THICKNESS = 0.5
ENTRANCE_RADIUS = 2.0
ENTRANCE_CENTER_HEIGHT = 2.0
ENTRANCE_OPENING_DEPTH = 4.0
ENTRANCE_SEGMENT_COUNT = 32
TOP_FLOOR_CLEARANCE = 8.0
TOP_FLOOR_OPENING_SIZE = 7.0
STAIRCASE_TOTAL_HEIGHT = TOWER_HEIGHT - TOP_FLOOR_CLEARANCE
STAIR_STOREY_HEIGHT = STAIRCASE_TOTAL_HEIGHT * 0.5
STAIR_RISER_COUNT_PER_STOREY = 116
STAIR_RISER_COUNT_PER_FLIGHT = STAIR_RISER_COUNT_PER_STOREY // 2
STAIR_RISER_HEIGHT = STAIR_STOREY_HEIGHT / STAIR_RISER_COUNT_PER_STOREY
STAIR_TREAD_DEPTH = 0.28
STAIR_FLIGHT_LENGTH = STAIR_TREAD_DEPTH * STAIR_RISER_COUNT_PER_FLIGHT
STAIR_WELL_WIDTH = STAIR_FLIGHT_LENGTH
WALL_CLEARANCE = 0.05
STAIR_RAILING_THICKNESS = 0.5
STAIR_RAILING_HEIGHT = 2.1
STAIR_SLAB_THICKNESS = 0.6
LIGHT_SPHERE_RADIUS = 0.55
LIGHT_ROD_RADIUS = 0.075
LIGHT_ROD_LENGTH = 1.8
LIGHT_ENERGY = 150.0


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


def build_beveled_square_profile(width, corner_cut, height):
    half_width = width * 0.5
    long_edge_half_width = half_width - corner_cut
    return [
        (-long_edge_half_width, -half_width, height),
        (long_edge_half_width, -half_width, height),
        (half_width, -long_edge_half_width, height),
        (half_width, long_edge_half_width, height),
        (long_edge_half_width, half_width, height),
        (-long_edge_half_width, half_width, height),
        (-half_width, long_edge_half_width, height),
        (-half_width, -long_edge_half_width, height),
    ]


def build_tower_shell_mesh():
    top_corner_cut = TOWER_BASE_CORNER_CUT * TOWER_TOP_WIDTH / TOWER_BASE_WIDTH
    basement_profile = build_beveled_square_profile(TOWER_BASE_WIDTH, TOWER_BASE_CORNER_CUT, -STAIRCASE_TOTAL_HEIGHT)
    base_profile = build_beveled_square_profile(TOWER_BASE_WIDTH, TOWER_BASE_CORNER_CUT, 0.0)
    top_profile = build_beveled_square_profile(TOWER_TOP_WIDTH, top_corner_cut, TOWER_HEIGHT)
    vertices = basement_profile + base_profile + top_profile
    faces = [tuple(range(16, 24))]
    for profile_index in range(2):
        lower_profile_vertex_index = profile_index * 8
        upper_profile_vertex_index = lower_profile_vertex_index + 8
        for face_index in range(8):
            next_face_index = (face_index + 1) % 8
            faces.append((
                lower_profile_vertex_index + face_index,
                lower_profile_vertex_index + next_face_index,
                upper_profile_vertex_index + next_face_index,
                upper_profile_vertex_index + face_index,
            ))
    tower_mesh = bpy.data.meshes.new("Tower shell mesh")
    tower_mesh.from_pydata(vertices, [], faces)
    tower_mesh.update()
    return tower_mesh


def append_box_mesh_geometry(vertices, faces, minimum_corner, maximum_corner):
    start_vertex_index = len(vertices)
    minimum_x, minimum_y, minimum_z = minimum_corner
    maximum_x, maximum_y, maximum_z = maximum_corner
    vertices.extend([
        (minimum_x, minimum_y, minimum_z),
        (maximum_x, minimum_y, minimum_z),
        (maximum_x, maximum_y, minimum_z),
        (minimum_x, maximum_y, minimum_z),
        (minimum_x, minimum_y, maximum_z),
        (maximum_x, minimum_y, maximum_z),
        (maximum_x, maximum_y, maximum_z),
        (minimum_x, maximum_y, maximum_z),
    ])
    faces.extend([
        (start_vertex_index, start_vertex_index + 3, start_vertex_index + 2, start_vertex_index + 1),
        (start_vertex_index + 4, start_vertex_index + 5, start_vertex_index + 6, start_vertex_index + 7),
        (start_vertex_index, start_vertex_index + 1, start_vertex_index + 5, start_vertex_index + 4),
        (start_vertex_index + 1, start_vertex_index + 2, start_vertex_index + 6, start_vertex_index + 5),
        (start_vertex_index + 2, start_vertex_index + 3, start_vertex_index + 7, start_vertex_index + 6),
        (start_vertex_index + 3, start_vertex_index, start_vertex_index + 4, start_vertex_index + 7),
    ])


def calculate_tower_inner_half_width(height):
    tower_height = max(height, 0.0)
    tower_width = TOWER_BASE_WIDTH + (TOWER_TOP_WIDTH - TOWER_BASE_WIDTH) * tower_height / TOWER_HEIGHT
    return tower_width * 0.5 - WALL_THICKNESS


def append_stair_step_mesh_geometry(vertices, faces, step_start, step_end, flight_length, outer_wall_start_y, outer_wall_end_y, step_height, stair_direction):
    stair_slope = stair_direction * STAIR_RISER_COUNT_PER_FLIGHT * STAIR_RISER_HEIGHT / flight_length
    start_underside_height = step_start * stair_slope - STAIR_SLAB_THICKNESS
    end_underside_height = step_end * stair_slope - STAIR_SLAB_THICKNESS
    inner_well_y = -STAIR_WELL_WIDTH * 0.5
    start_vertex_index = len(vertices)
    vertices.extend([
        (step_start, outer_wall_start_y, start_underside_height),
        (step_end, outer_wall_end_y, end_underside_height),
        (step_end, inner_well_y, end_underside_height),
        (step_start, inner_well_y, start_underside_height),
        (step_start, outer_wall_start_y, step_height),
        (step_end, outer_wall_end_y, step_height),
        (step_end, inner_well_y, step_height),
        (step_start, inner_well_y, step_height),
    ])
    faces.extend([
        (start_vertex_index, start_vertex_index + 3, start_vertex_index + 2, start_vertex_index + 1),
        (start_vertex_index + 4, start_vertex_index + 5, start_vertex_index + 6, start_vertex_index + 7),
        (start_vertex_index, start_vertex_index + 1, start_vertex_index + 5, start_vertex_index + 4),
        (start_vertex_index + 1, start_vertex_index + 2, start_vertex_index + 6, start_vertex_index + 5),
        (start_vertex_index + 2, start_vertex_index + 3, start_vertex_index + 7, start_vertex_index + 6),
        (start_vertex_index + 3, start_vertex_index, start_vertex_index + 4, start_vertex_index + 7),
    ])


def build_stair_flight_mesh(flight_base_height, flight_length, descends):
    vertices = []
    faces = []
    stair_direction = -1.0 if descends else 1.0
    stair_tread_depth = flight_length / STAIR_RISER_COUNT_PER_FLIGHT
    for step_index in range(STAIR_RISER_COUNT_PER_FLIGHT):
        step_start = step_index * stair_tread_depth
        step_end = step_start + stair_tread_depth
        step_height = stair_direction * (step_index + 1) * STAIR_RISER_HEIGHT
        step_start_height = stair_direction * step_index * STAIR_RISER_HEIGHT
        outer_wall_start_y = -calculate_tower_inner_half_width(flight_base_height + step_start_height) + WALL_CLEARANCE
        outer_wall_end_y = -calculate_tower_inner_half_width(flight_base_height + step_height) + WALL_CLEARANCE
        append_stair_step_mesh_geometry(
            vertices,
            faces,
            step_start,
            step_end,
            flight_length,
            outer_wall_start_y,
            outer_wall_end_y,
            step_height,
            stair_direction,
        )
    stair_flight_mesh = bpy.data.meshes.new("Tapered concrete stair flight mesh")
    stair_flight_mesh.from_pydata(vertices, [], faces)
    stair_flight_mesh.update()
    return stair_flight_mesh


def build_unit_cube_mesh(mesh_name):
    vertices = []
    faces = []
    half_unit_length = 0.5
    append_box_mesh_geometry(
        vertices,
        faces,
        (-half_unit_length, -half_unit_length, -half_unit_length),
        (half_unit_length, half_unit_length, half_unit_length),
    )
    unit_cube_mesh = bpy.data.meshes.new(mesh_name)
    unit_cube_mesh.from_pydata(vertices, [], faces)
    unit_cube_mesh.update()
    return unit_cube_mesh


def create_concrete_material():
    concrete_material = bpy.data.materials.new("Tower concrete")
    concrete_material.diffuse_color = (0.30, 0.30, 0.28, 1.0)
    concrete_material.use_nodes = True
    principled_shader = concrete_material.node_tree.nodes.get("Principled BSDF")
    principled_shader.inputs["Base Color"].default_value = (0.30, 0.30, 0.28, 1.0)
    principled_shader.inputs["Roughness"].default_value = 0.92
    return concrete_material


def create_light_material():
    light_material = bpy.data.materials.new("Cold stair light")
    light_material.diffuse_color = (0.62, 0.78, 1.0, 1.0)
    light_material.use_nodes = True
    principled_shader = light_material.node_tree.nodes.get("Principled BSDF")
    principled_shader.inputs["Base Color"].default_value = (0.62, 0.78, 1.0, 1.0)
    principled_shader.inputs["Emission Color"].default_value = (0.62, 0.78, 1.0, 1.0)
    principled_shader.inputs["Emission Strength"].default_value = 1.25
    principled_shader.inputs["Roughness"].default_value = 0.35
    return light_material


def create_metal_material():
    metal_material = bpy.data.materials.new("Brushed stair light metal")
    metal_material.diffuse_color = (0.24, 0.27, 0.31, 1.0)
    metal_material.use_nodes = True
    principled_shader = metal_material.node_tree.nodes.get("Principled BSDF")
    principled_shader.inputs["Base Color"].default_value = (0.24, 0.27, 0.31, 1.0)
    principled_shader.inputs["Metallic"].default_value = 0.85
    principled_shader.inputs["Roughness"].default_value = 0.28
    return metal_material


def create_box_object(collection, object_name, mesh_data, material, location, scale):
    box_object = bpy.data.objects.new(object_name, mesh_data)
    collection.objects.link(box_object)
    box_object.location = location
    box_object.scale = scale
    if material is not None:
        box_object.data.materials.append(material)
    return box_object


def create_tower_shell(tower_collection, concrete_material):
    tower_shell = bpy.data.objects.new("Tower shell", build_tower_shell_mesh())
    tower_collection.objects.link(tower_shell)
    tower_shell.data.materials.append(concrete_material)
    tower_shell.rotation_euler[2] = math.pi
    tower_shell["architectural_role"] = "tower_shell"
    wall_thickness_modifier = tower_shell.modifiers.new("Wall thickness", "SOLIDIFY")
    wall_thickness_modifier.thickness = WALL_THICKNESS
    wall_thickness_modifier.offset = -1.0
    wall_thickness_modifier.use_even_offset = True
    wall_thickness_modifier.use_quality_normals = True
    return tower_shell


def calculate_entrance_wall_normal():
    tower_taper_per_height = (TOWER_BASE_WIDTH - TOWER_TOP_WIDTH) * 0.5 / TOWER_HEIGHT
    return Vector((0.0, -1.0, tower_taper_per_height)).normalized()


def create_entrance_opening(construction_collection):
    entrance_wall_normal = calculate_entrance_wall_normal()
    entrance_center_y = -TOWER_BASE_WIDTH * 0.5 + (TOWER_BASE_WIDTH - TOWER_TOP_WIDTH) * 0.5 * ENTRANCE_CENTER_HEIGHT / TOWER_HEIGHT
    entrance_center = Vector((0.0, entrance_center_y, ENTRANCE_CENTER_HEIGHT))
    opening_center = entrance_center - entrance_wall_normal * (ENTRANCE_OPENING_DEPTH * 0.5 - WALL_THICKNESS)
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=ENTRANCE_SEGMENT_COUNT,
        radius=ENTRANCE_RADIUS,
        depth=ENTRANCE_OPENING_DEPTH,
        location=opening_center,
    )
    entrance_opening = bpy.context.active_object
    entrance_opening.name = "Tower entrance opening"
    for collection in tuple(entrance_opening.users_collection):
        collection.objects.unlink(entrance_opening)
    construction_collection.objects.link(entrance_opening)
    entrance_opening.rotation_mode = "QUATERNION"
    entrance_opening.rotation_quaternion = Vector((0.0, 0.0, 1.0)).rotation_difference(entrance_wall_normal)
    entrance_opening.display_type = "WIRE"
    entrance_opening.hide_render = True
    entrance_opening["architectural_role"] = "tower_entrance_opening"
    return entrance_opening


def add_opening_modifier(tower_shell, modifier_name, opening_object):
    opening_modifier = tower_shell.modifiers.new(modifier_name, "BOOLEAN")
    opening_modifier.operation = "DIFFERENCE"
    opening_modifier.solver = "EXACT"
    opening_modifier.object = opening_object


def create_stair_flight(stair_collection, stair_railing_mesh, concrete_material, flight_name, flight_base_height, flight_length, flight_location, flight_rotation, descends):
    stair_flight_mesh = build_stair_flight_mesh(flight_base_height, flight_length, descends)
    stair_flight = bpy.data.objects.new(f"{flight_name} concrete stair", stair_flight_mesh)
    stair_collection.objects.link(stair_flight)
    stair_flight.location = flight_location
    stair_flight.rotation_euler[2] = flight_rotation
    stair_flight.data.materials.append(concrete_material)
    stair_flight["architectural_role"] = "reusable_stair_flight"
    create_stair_railing_segment(
        stair_collection,
        stair_railing_mesh,
        concrete_material,
        flight_name,
        flight_location,
        flight_rotation,
    )
    return stair_flight


def clip_stair_railing_at_floor(construction_collection, unit_cube_mesh, stair_collection, flight_name, floor_height):
    stair_railing = stair_collection.objects.get(f"{flight_name} concrete railing")
    railing_clipper = create_box_object(
        construction_collection,
        f"{flight_name} railing below-floor clipper",
        unit_cube_mesh,
        None,
        (0.0, 0.0, floor_height - 50.0),
        (100.0, 100.0, 100.0),
    )
    railing_clipper.display_type = "WIRE"
    railing_clipper.hide_render = True
    railing_clipper["architectural_role"] = "stair_railing_floor_clipper"
    clipping_modifier = stair_railing.modifiers.new("Hangar floor clipping", "BOOLEAN")
    clipping_modifier.operation = "INTERSECT"
    clipping_modifier.solver = "EXACT"
    clipping_modifier.object = railing_clipper


def create_corner_parapet(stair_collection, unit_cube_mesh, concrete_material, landing_name, landing_height, landing_x_direction, landing_y_direction):
    well_half_width = STAIR_WELL_WIDTH * 0.5
    parapet_half_thickness = STAIR_RAILING_THICKNESS * 0.5
    parapet_base_height = landing_height - STAIR_SLAB_THICKNESS
    corner_parapet = create_box_object(
        stair_collection,
        f"{landing_name} parapet",
        unit_cube_mesh,
        concrete_material,
        (
            landing_x_direction * (well_half_width + parapet_half_thickness),
            landing_y_direction * (well_half_width + parapet_half_thickness),
            parapet_base_height + STAIR_RAILING_HEIGHT * 0.5,
        ),
        (STAIR_RAILING_THICKNESS, STAIR_RAILING_THICKNESS, STAIR_RAILING_HEIGHT),
    )
    corner_parapet["architectural_role"] = "stair_corner_parapet"
    return corner_parapet


def create_corner_landing(stair_collection, unit_cube_mesh, concrete_material, landing_number, landing_height, landing_x_direction, landing_y_direction):
    tower_inner_half_width = calculate_tower_inner_half_width(landing_height) - WALL_CLEARANCE
    well_half_width = STAIR_WELL_WIDTH * 0.5
    landing_span = tower_inner_half_width - well_half_width
    corner_landing = create_box_object(
        stair_collection,
        f"Corner landing {landing_number}",
        unit_cube_mesh,
        concrete_material,
        (
            landing_x_direction * (tower_inner_half_width + well_half_width) * 0.5,
            landing_y_direction * (tower_inner_half_width + well_half_width) * 0.5,
            landing_height - STAIR_SLAB_THICKNESS * 0.5,
        ),
        (landing_span, landing_span, STAIR_SLAB_THICKNESS),
    )
    corner_landing["architectural_role"] = "stair_corner_landing"
    return corner_landing


def build_tower_floor_mesh(floor_height, mesh_name):
    tower_width = TOWER_BASE_WIDTH + (TOWER_TOP_WIDTH - TOWER_BASE_WIDTH) * floor_height / TOWER_HEIGHT
    floor_width = tower_width - (WALL_THICKNESS + WALL_CLEARANCE) * 2.0
    floor_corner_cut = TOWER_BASE_CORNER_CUT * floor_width / TOWER_BASE_WIDTH
    bottom_profile = build_beveled_square_profile(floor_width, floor_corner_cut, -STAIR_SLAB_THICKNESS)
    top_profile = build_beveled_square_profile(floor_width, floor_corner_cut, 0.0)
    vertices = bottom_profile + top_profile
    faces = [tuple(range(7, -1, -1)), tuple(range(8, 16))]
    for face_index in range(8):
        next_face_index = (face_index + 1) % 8
        faces.append((face_index, next_face_index, next_face_index + 8, face_index + 8))
    tower_floor_mesh = bpy.data.meshes.new(mesh_name)
    tower_floor_mesh.from_pydata(vertices, [], faces)
    tower_floor_mesh.update()
    return tower_floor_mesh


def create_stair_opening_cutters(construction_collection, unit_cube_mesh, floor_name, floor_height, flight_location, flight_rotation, landing_height, landing_x_direction, landing_y_direction):
    tower_inner_half_width = calculate_tower_inner_half_width(landing_height) - WALL_CLEARANCE
    well_half_width = STAIR_WELL_WIDTH * 0.5
    landing_span = tower_inner_half_width - well_half_width
    opening_depth = STAIR_SLAB_THICKNESS * 3.0
    flight_center = Vector((STAIR_FLIGHT_LENGTH * 0.5, -(tower_inner_half_width + well_half_width) * 0.5))
    flight_rotation_matrix = Matrix.Rotation(flight_rotation, 2)
    rotated_flight_center = flight_rotation_matrix @ flight_center + Vector(flight_location[:2])
    flight_opening = create_box_object(
        construction_collection,
        f"{floor_name} flight opening cutter",
        unit_cube_mesh,
        None,
        (rotated_flight_center.x, rotated_flight_center.y, floor_height),
        (STAIR_FLIGHT_LENGTH, landing_span, opening_depth),
    )
    flight_opening.rotation_euler[2] = flight_rotation
    landing_center = (
        landing_x_direction * (tower_inner_half_width + well_half_width) * 0.5,
        landing_y_direction * (tower_inner_half_width + well_half_width) * 0.5,
        floor_height,
    )
    landing_opening = create_box_object(
        construction_collection,
        f"{floor_name} landing opening cutter",
        unit_cube_mesh,
        None,
        landing_center,
        (landing_span, landing_span, opening_depth),
    )
    for stair_opening in (flight_opening, landing_opening):
        stair_opening.display_type = "WIRE"
        stair_opening.hide_render = True
        stair_opening["architectural_role"] = "stair_opening"
    return flight_opening, landing_opening


def add_stair_openings(floor, floor_name, stair_openings):
    for opening_number, stair_opening in enumerate(stair_openings, start=1):
        add_opening_modifier(floor, f"{floor_name} stair opening {opening_number}", stair_opening)


def create_stair_core(stair_collection, construction_collection, concrete_material):
    stair_rise = STAIR_RISER_COUNT_PER_FLIGHT * STAIR_RISER_HEIGHT
    stair_railing_mesh = build_stair_railing_mesh(STAIR_FLIGHT_LENGTH, stair_rise, STAIR_WELL_WIDTH, STAIR_RAILING_THICKNESS, -STAIR_SLAB_THICKNESS, STAIR_RAILING_HEIGHT)
    lower_stair_railing_mesh = build_stair_railing_mesh(STAIR_FLIGHT_LENGTH, -stair_rise, STAIR_WELL_WIDTH, STAIR_RAILING_THICKNESS, -STAIR_SLAB_THICKNESS, STAIR_RAILING_HEIGHT)
    unit_cube_mesh = build_unit_cube_mesh("Reusable architectural cube mesh")
    stair_half_length = STAIR_FLIGHT_LENGTH * 0.5
    flight_rise = STAIR_STOREY_HEIGHT * 0.5
    top_floor_height = STAIRCASE_TOTAL_HEIGHT
    upper_flight_specifications = [
        (1, 0.0, STAIR_FLIGHT_LENGTH, (0.0, -stair_half_length, 0.0), math.pi * 0.5, flight_rise, 1.0, 1.0),
        (2, flight_rise, STAIR_FLIGHT_LENGTH, (stair_half_length, 0.0, flight_rise), math.pi, flight_rise * 2.0, -1.0, 1.0),
        (3, flight_rise * 2.0, STAIR_FLIGHT_LENGTH, (0.0, stair_half_length, flight_rise * 2.0), -math.pi * 0.5, flight_rise * 3.0, -1.0, -1.0),
        (4, flight_rise * 3.0, STAIR_FLIGHT_LENGTH, (-stair_half_length, 0.0, flight_rise * 3.0), 0.0, top_floor_height, 1.0, -1.0),
    ]
    for flight_number, flight_base_height, flight_length, flight_location, flight_rotation, landing_height, landing_x_direction, landing_y_direction in upper_flight_specifications:
        flight_name = f"Flight {flight_number}"
        create_stair_flight(stair_collection, stair_railing_mesh, concrete_material, flight_name, flight_base_height, flight_length, flight_location, flight_rotation, False)
        create_corner_landing(stair_collection, unit_cube_mesh, concrete_material, flight_name, landing_height, landing_x_direction, landing_y_direction)
        if flight_number < 4:
            create_corner_parapet(stair_collection, unit_cube_mesh, concrete_material, f"Corner landing {flight_name}", landing_height, landing_x_direction, landing_y_direction)
    clip_stair_railing_at_floor(construction_collection, unit_cube_mesh, stair_collection, "Flight 4", top_floor_height)

    top_hangar_floor = bpy.data.objects.new("Top hangar floor", build_tower_floor_mesh(top_floor_height, "Top hangar floor mesh"))
    stair_collection.objects.link(top_hangar_floor)
    top_hangar_floor.location.z = top_floor_height
    top_hangar_floor.data.materials.append(concrete_material)
    top_hangar_floor["architectural_role"] = "top_hangar_floor"
    upper_last_flight = upper_flight_specifications[-1]
    top_floor_openings = create_stair_opening_cutters(construction_collection, unit_cube_mesh, "Top hangar", top_floor_height, upper_last_flight[3], upper_last_flight[4], upper_last_flight[5], upper_last_flight[6], upper_last_flight[7])
    add_stair_openings(top_hangar_floor, "Top hangar", top_floor_openings)

    lower_flight_specifications = [
        (1, 0.0, (0.0, stair_half_length, 0.0), -math.pi * 0.5, -flight_rise, -1.0, -1.0),
        (2, -flight_rise, (-stair_half_length, 0.0, -flight_rise), 0.0, -flight_rise * 2.0, 1.0, -1.0),
        (3, -flight_rise * 2.0, (0.0, -stair_half_length, -flight_rise * 2.0), math.pi * 0.5, -flight_rise * 3.0, 1.0, 1.0),
        (4, -flight_rise * 3.0, (stair_half_length, 0.0, -flight_rise * 3.0), math.pi, -top_floor_height, -1.0, 1.0),
    ]
    for flight_number, flight_base_height, flight_location, flight_rotation, landing_height, landing_x_direction, landing_y_direction in lower_flight_specifications:
        flight_name = f"Lower flight {flight_number}"
        create_stair_flight(stair_collection, lower_stair_railing_mesh, concrete_material, flight_name, flight_base_height, STAIR_FLIGHT_LENGTH, flight_location, flight_rotation, True)
        create_corner_landing(stair_collection, unit_cube_mesh, concrete_material, flight_name, landing_height, landing_x_direction, landing_y_direction)
        create_corner_parapet(stair_collection, unit_cube_mesh, concrete_material, f"Corner landing {flight_name}", landing_height, landing_x_direction, landing_y_direction)
    clip_stair_railing_at_floor(construction_collection, unit_cube_mesh, stair_collection, "Lower flight 1", 0.0)

    entrance_floor = bpy.data.objects.new("Entrance floor", build_tower_floor_mesh(0.0, "Entrance floor mesh"))
    stair_collection.objects.link(entrance_floor)
    entrance_floor.data.materials.append(concrete_material)
    entrance_floor["architectural_role"] = "entrance_floor"
    lower_first_flight = lower_flight_specifications[0]
    entrance_floor_openings = create_stair_opening_cutters(
        construction_collection,
        unit_cube_mesh,
        "Entrance floor",
        0.0,
        lower_first_flight[2],
        lower_first_flight[3],
        lower_first_flight[4],
        lower_first_flight[5],
        lower_first_flight[6],
    )
    add_stair_openings(entrance_floor, "Entrance", entrance_floor_openings)


def create_stair_lights(lighting_collection, light_material, metal_material):
    stair_half_length = STAIR_FLIGHT_LENGTH * 0.5
    flight_rise = STAIR_STOREY_HEIGHT * 0.5
    parapet_joint_specifications = [
        (flight_rise, 1.0, 1.0),
        (flight_rise * 2.0, -1.0, 1.0),
        (flight_rise * 3.0, -1.0, -1.0),
        (-flight_rise, -1.0, -1.0),
        (-flight_rise * 2.0, 1.0, -1.0),
        (-flight_rise * 3.0, 1.0, 1.0),
        (-flight_rise * 4.0, -1.0, 1.0),
    ]
    bpy.ops.mesh.primitive_uv_sphere_add(segments=24, ring_count=16, radius=LIGHT_SPHERE_RADIUS)
    sphere_source = bpy.context.object
    sphere_mesh = sphere_source.data
    sphere_mesh.materials.append(light_material)
    bpy.data.objects.remove(sphere_source, do_unlink=True)
    bpy.ops.mesh.primitive_cylinder_add(vertices=16, radius=LIGHT_ROD_RADIUS, depth=LIGHT_ROD_LENGTH)
    rod_source = bpy.context.object
    rod_mesh = rod_source.data
    rod_mesh.materials.append(metal_material)
    bpy.data.objects.remove(rod_source, do_unlink=True)
    parapet_half_thickness = STAIR_RAILING_THICKNESS * 0.5
    for light_number, (joint_height, joint_x_direction, joint_y_direction) in enumerate(parapet_joint_specifications, start=1):
        joint_position_x = joint_x_direction * (stair_half_length + parapet_half_thickness)
        joint_position_y = joint_y_direction * (stair_half_length + parapet_half_thickness)
        parapet_top_height = joint_height + STAIR_RAILING_HEIGHT - STAIR_SLAB_THICKNESS
        rod_object = bpy.data.objects.new(f"Stair light rod {light_number:02d}", rod_mesh)
        lighting_collection.objects.link(rod_object)
        rod_object.location = (joint_position_x, joint_position_y, parapet_top_height + LIGHT_ROD_LENGTH * 0.5)
        rod_object["architectural_role"] = "stair_light_rod"
        sphere_center_height = parapet_top_height + LIGHT_ROD_LENGTH + LIGHT_SPHERE_RADIUS * 0.65
        light_sphere = bpy.data.objects.new(f"Stair light sphere {light_number:02d}", sphere_mesh)
        lighting_collection.objects.link(light_sphere)
        light_sphere.location = (joint_position_x, joint_position_y, sphere_center_height)
        light_sphere["architectural_role"] = "stair_light_fixture"
        point_light_data = bpy.data.lights.new(f"Stair light {light_number:02d}", "POINT")
        point_light_data.color = (0.62, 0.78, 1.0)
        point_light_data.energy = LIGHT_ENERGY
        point_light_data.shadow_soft_size = 2.0
        point_light = bpy.data.objects.new(f"Stair light {light_number:02d}", point_light_data)
        lighting_collection.objects.link(point_light)
        point_light.location = light_sphere.location
        point_light["architectural_role"] = "stair_point_light"


def save_scene_and_export_tower(tower_collection, stair_collection, lighting_collection):
    model_directory = Path(__file__).resolve().parent
    blend_path = model_directory / "intro-tower.blend"
    glb_path = model_directory / "intro-tower.glb"
    blend_path.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(blend_path))
    bpy.ops.object.select_all(action="DESELECT")
    for export_collection in (tower_collection, stair_collection, lighting_collection):
        for export_object in export_collection.objects:
            export_object.select_set(True)
    bpy.context.view_layer.objects.active = tower_collection.objects[0]
    bpy.ops.export_scene.gltf(
        filepath=str(glb_path),
        export_format="GLB",
        use_selection=True,
        export_apply=True,
        export_extras=True,
        export_lights=True,
    )


def main():
    clear_scene()
    tower_collection = create_collection("Tower")
    stair_collection = create_collection("Stair core")
    lighting_collection = create_collection("Tower lighting")
    construction_collection = create_collection("Tower construction")
    concrete_material = create_concrete_material()
    light_material = create_light_material()
    metal_material = create_metal_material()
    tower_shell = create_tower_shell(tower_collection, concrete_material)
    entrance_opening = create_entrance_opening(construction_collection)
    add_opening_modifier(tower_shell, "Tower entrance opening", entrance_opening)
    create_stair_core(stair_collection, construction_collection, concrete_material)
    create_stair_lights(lighting_collection, light_material, metal_material)
    save_scene_and_export_tower(tower_collection, stair_collection, lighting_collection)


main()
