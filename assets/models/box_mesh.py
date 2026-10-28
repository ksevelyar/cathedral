import bpy


def build_box_mesh(mesh_name, dimensions=(1.0, 1.0, 1.0)):
    half_width, half_length, half_height = (dimension * 0.5 for dimension in dimensions)
    box_vertices = [
        (-half_width, -half_length, -half_height),
        (half_width, -half_length, -half_height),
        (half_width, half_length, -half_height),
        (-half_width, half_length, -half_height),
        (-half_width, -half_length, half_height),
        (half_width, -half_length, half_height),
        (half_width, half_length, half_height),
        (-half_width, half_length, half_height),
    ]
    box_faces = [
        (0, 3, 2, 1),
        (4, 5, 6, 7),
        (0, 1, 5, 4),
        (1, 2, 6, 5),
        (2, 3, 7, 6),
        (3, 0, 4, 7),
    ]
    box_mesh = bpy.data.meshes.new(mesh_name)
    box_mesh.from_pydata(box_vertices, [], box_faces)
    box_mesh.update()
    return box_mesh
