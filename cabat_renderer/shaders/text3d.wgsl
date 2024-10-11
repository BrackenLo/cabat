//====================================================================
// Uniforms

struct Camera {
    projection: mat4x4<f32>,
    position: vec3<f32>,
}

struct Instance {
    transform: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;

@group(1) @binding(0) var atlas_texture: texture_2d<f32>;
@group(1) @binding(1) var atlas_texture_sampler: sampler;

@group(2) @binding(0) var<uniform> instance: Instance;


//====================================================================

struct VertexIn {
    // Vertex
    @builtin(vertex_index) index: u32,
    // Instance
    @location(0) glyph_pos: vec2<f32>,
    @location(1) glyph_size: vec2<f32>,
    @location(2) uv_start: vec2<f32>,
    @location(3) uv_end: vec2<f32>,
    @location(4) color: u32,
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

    var vertex_pos: vec2<f32>;
    
    switch (in.index) {
        // 0 = Top Left
        case 0u: {
            vertex_pos = vec2<f32>(-0.5, 0.5);
            out.uv = in.uv_start;
            break;
        }
        // 1 = Top Right
        case 2u: {
            vertex_pos = vec2<f32>(0.5, 0.5);
            out.uv = vec2<f32>(in.uv_end.x, in.uv_start.y);
            break;
        }
        // Bottom Left
        case 1u: {
            vertex_pos = vec2<f32>(-0.5, -0.5);
            out.uv = vec2<f32>(in.uv_start.x, in.uv_end.y);
            break;
        }
        // Bottom Right
        case 3u: {
            vertex_pos = vec2<f32>(0.5, -0.5);
            out.uv = in.uv_end;
            break;
        }
        default: {}
    }
    
    vertex_pos = vertex_pos * in.glyph_size + in.glyph_pos;

    out.clip_position =
        camera.projection
        * instance.transform
        * vec4<f32>(vertex_pos, 1., 1.);

    out.color = vec4<f32>(
        f32((in.color & 0x00ff0000u) >> 16u) / 255.,
        f32((in.color & 0x0000ff00u) >> 8u) / 255.,
        f32(in.color & 0x00ff0000u) / 255.,
        f32((in.color & 0xff000000u) >> 24u) / 255.,
    );

    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let tex_color = textureSample(atlas_texture, atlas_texture_sampler, in.uv);
    
    return vec4<f32>(in.color.xyz, in.color.w * tex_color.x);
}

//====================================================================

