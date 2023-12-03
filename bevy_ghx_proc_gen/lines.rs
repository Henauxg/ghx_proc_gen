use bevy::{
    asset::{Asset, Assets},
    ecs::system::{Commands, ResMut},
    math::Vec3,
    pbr::{Material, MaterialMeshBundle, MaterialPipeline, MaterialPipelineKey},
    reflect::TypePath,
    render::{
        color::Color,
        mesh::{Mesh, MeshVertexBufferLayout},
        render_resource::{
            AsBindGroup, PolygonMode, PrimitiveTopology, RenderPipelineDescriptor, ShaderRef,
            SpecializedMeshPipelineError,
        },
    },
    transform::components::Transform,
    utils::default,
};

// Built on top of https://bevyengine.org/examples/3D%20Rendering/lines/ (on bevy 0.12)

fn draw_debug_grid(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<LineMaterial>>,
    grid: &GridDefinition,
    grid_origin: Vec3,
    node_size: Vec3,
) {

    // // Spawn a list of lines with start and end points for each lines
    // commands.spawn(MaterialMeshBundle {
    //     mesh: meshes.add(Mesh::from(LineList {
    //         lines: vec![
    //             (Vec3::ZERO, Vec3::new(1.0, 1.0, 0.0)),
    //             (Vec3::new(1.0, 1.0, 0.0), Vec3::new(1.0, 0.0, 0.0)),
    //         ],
    //     })),
    //     transform: Transform::from_xyz(-1.5, 0.0, 0.0),
    //     material: materials.add(LineMaterial {
    //         color: Color::GREEN,
    //     }),
    //     ..default()
    // });

    // // Spawn a line strip that goes from point to point
    // commands.spawn(MaterialMeshBundle {
    //     mesh: meshes.add(Mesh::from(LineStrip {
    //         points: vec![
    //             Vec3::ZERO,
    //             Vec3::new(1.0, 1.0, 0.0),
    //             Vec3::new(1.0, 0.0, 0.0),
    //         ],
    //     })),
    //     transform: Transform::from_xyz(0.5, 0.0, 0.0),
    //     material: materials.add(LineMaterial { color: Color::BLUE }),
    //     ..default()
    // });
}

#[derive(Asset, TypePath, Default, AsBindGroup, Debug, Clone)]
struct LineMaterial {
    #[uniform(0)]
    color: Color,
}

impl Material for LineMaterial {
    fn fragment_shader() -> ShaderRef {
        "#import bevy_pbr::forward_io::VertexOutput

		struct LineMaterial {
			color: vec4<f32>,
		};
		
		@group(1) @binding(0) var<uniform> material: LineMaterial;
		
		@fragment
		fn fragment(
			mesh: VertexOutput,
		) -> @location(0) vec4<f32> {
			return material.color;
		}"
        .into()
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayout,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // This is the important part to tell bevy to render this material as a line between vertices
        descriptor.primitive.polygon_mode = PolygonMode::Line;
        Ok(())
    }
}

/// A list of lines with a start and end position
#[derive(Debug, Clone)]
pub struct LineList {
    pub lines: Vec<(Vec3, Vec3)>,
}

impl From<LineList> for Mesh {
    fn from(line: LineList) -> Self {
        let vertices: Vec<_> = line.lines.into_iter().flat_map(|(a, b)| [a, b]).collect();

        // This tells wgpu that the positions are list of lines
        // where every pair is a start and end point
        Mesh::new(PrimitiveTopology::LineList)
            // Add the vertices positions as an attribute
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
    }
}

/// A list of points that will have a line drawn between each consecutive points
#[derive(Debug, Clone)]
pub struct LineStrip {
    pub points: Vec<Vec3>,
}

impl From<LineStrip> for Mesh {
    fn from(line: LineStrip) -> Self {
        // This tells wgpu that the positions are a list of points
        // where a line will be drawn between each consecutive point
        Mesh::new(PrimitiveTopology::LineStrip)
            // Add the point positions as an attribute
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, line.points)
    }
}
