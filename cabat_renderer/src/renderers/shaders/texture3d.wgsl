//====================================================================
// Uniforms

struct Camera {
    projection: mat4x4<f32>,
    position: vec3<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;

@group(1) @binding(0) var texture: texture_2d<f32>;
@group(1) @binding(1) var texture_sampler: sampler;


//====================================================================

struct VertexIn {
    // Vertex
    @location(0) vertex_position: vec2<f32>,
    @location(1) uv: vec2<f32>,

    // Instance
    @location(2) size: vec2<f32>,
    @location(3) transform_1: vec4<f32>,
    @location(4) transform_2: vec4<f32>,
    @location(5) transform_3: vec4<f32>,
    @location(6) transform_4: vec4<f32>,
    @location(7) color: vec4<f32>,
}

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

//====================================================================

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    
    let transform = mat4x4<f32>(
        in.transform_1,
        in.transform_2,
        in.transform_3,
        in.transform_4,
    );

    let vertex_pos = in.vertex_position * in.size;

    out.clip_position =
        camera.projection
        * transform
        * vec4<f32>(vertex_pos, 1., 1.);

    out.uv = in.uv;
    out.color = in.color;

    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let tex_color = textureSample(texture, texture_sampler, in.uv);
    
    return tex_color * in.color;
}

//====================================================================

