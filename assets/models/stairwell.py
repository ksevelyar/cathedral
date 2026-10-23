import bpy


def build_stair_railing_mesh(flight_length, flight_rise, well_width, railing_thickness, railing_base_height, railing_height):
    railing_outer_y = -well_width * 0.5
    railing_inner_y = railing_outer_y - railing_thickness
    railing_top_height = railing_base_height + railing_height
    railing_sections = [
        (0.0, railing_base_height, railing_top_height),
        (flight_length, railing_base_height + flight_rise, railing_top_height + flight_rise),
    ]
    vertices = []
    for section_x, section_base_height, section_top_height in railing_sections:
        vertices.extend([
            (section_x, railing_inner_y, section_base_height),
            (section_x, railing_outer_y, section_base_height),
            (section_x, railing_outer_y, section_top_height),
            (section_x, railing_inner_y, section_top_height),
        ])
    faces = [
        (0, 3, 2, 1),
        (0, 1, 5, 4),
        (1, 2, 6, 5),
        (2, 3, 7, 6),
        (3, 0, 4, 7),
        (4, 5, 6, 7),
    ]
    railing_mesh = bpy.data.meshes.new("Mitered concrete stair railing mesh")
    railing_mesh.from_pydata(vertices, [], faces)
    railing_mesh.update()
    return railing_mesh


def create_stair_railing_segment(stair_collection, railing_mesh, concrete_material, flight_name, flight_location, flight_rotation):
    railing_object = bpy.data.objects.new(f"{flight_name} concrete railing", railing_mesh)
    stair_collection.objects.link(railing_object)
    railing_object.location = flight_location
    railing_object.rotation_euler[2] = flight_rotation
    railing_object.data.materials.append(concrete_material)
    railing_object["architectural_role"] = "concrete_stair_railing"
