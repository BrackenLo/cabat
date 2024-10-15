//====================================================================
// Uniforms

struct Camera {
    projection: mat4x4<f32>,
    position: vec3<f32>,
}

struct GlobalLightData {
    ambient_color: vec3<f32>,
    ambient_strength: f32,
}

struct LightData {
    light_count: i32,
}

struct Light {
    position: vec4<f32>,
    direction: vec4<f32>,
    diffuse_color: vec4<f32>,
    specular_color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;

@group(1) @binding(0) var<uniform> global_lighting: GlobalLightData;
@group(1) @binding(1) var<uniform> light_data: LightData;
@group(1) @binding(2) var<storage, read> light_array: array<Light>;

@group(2) @binding(0) var texture: texture_2d<f32>;
@group(2) @binding(1) var texture_sampler: sampler;


//====================================================================

struct VertexIn {
    // Vertex
    @location(0) vertex_position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal: vec3<f32>,

    // Instance
    @location(3) transform_1: vec4<f32>,
    @location(4) transform_2: vec4<f32>,
    @location(5) transform_3: vec4<f32>,
    @location(6) transform_4: vec4<f32>,

    @location(7) color: vec4<f32>,

    @location(8) normal_0: vec3<f32>,
    @location(9) normal_1: vec3<f32>,
    @location(10) normal_2: vec3<f32>,
}

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) color: vec4<f32>,
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

    let normal_matrix = mat3x3<f32>(
        in.normal_0,
        in.normal_1,
        in.normal_2,
    );

    let world_position = transform * vec4<f32>(in.vertex_position, 1.);

    out.clip_position =
        camera.projection
        * world_position;

    out.position = world_position.xyz;
    out.uv = in.uv;
    out.normal = normal_matrix * in.normal;
    out.color = in.color;

    return out;
}

//====================================================================

const DEFAULT_MATERIAL_SHININESS: f32 = 32.;

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {

    let ambient = vec3<f32>(global_lighting.ambient_strength * global_lighting.ambient_color);

    // let light_count = bitcast<i32>(arrayLength(&light_array));
    let light_count = light_data.light_count;

    var sum_diffuse = vec3<f32>();
    var sum_specular = vec3<f32>();

    for (var i = 0; i < light_count; i += 1) {

        // Calculate Diffuse Color
        let norm = normalize(in.normal);
        let light_dir = normalize(light_array[i].position.xyz - in.position);

        let diffuse_strength = max(dot(norm, light_dir), 0.0);
        sum_diffuse += light_array[i].diffuse_color.xyz * diffuse_strength;

        // Specular
        let view_dir = normalize(camera.position - in.position);
        let half_dir = normalize(view_dir + light_dir);
        let specular_strength = pow(max(dot(norm, half_dir), 0.0), DEFAULT_MATERIAL_SHININESS);
        sum_specular += light_array[i].specular_color.xyz * specular_strength;
    }

    let result = (
        ambient
        + sum_diffuse
        + sum_specular
    ) * textureSample(texture, texture_sampler, in.uv).xyz;
    
    return vec4(result, 1.0) * in.color;
}

//====================================================================

