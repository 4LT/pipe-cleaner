struct CameraUniforms {
    world2screen: mat3x4<f32>,
    scale: vec3<f32>,
}

struct Instance {
    @location(1) col0: vec4<f32>,
    @location(2) col1: vec4<f32>,
    @location(3) col2: vec4<f32>,
    @location(4) color: vec3<f32>,
}

struct VertexToPixel {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
}

struct FragOut {
    @builtin(frag_depth) depth: f32,
    @location(0) color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> cam: CameraUniforms;

const FOG = vec4<f32>(0.0, 0.0, 0.0, 0.99);
const POST_MULTIPLY = 1.2;

@vertex
fn vs_main(@location(0) pos: vec3<f32>, inst: Instance) -> VertexToPixel {
    let mdl2world = mat3x4<f32>(
        inst.col0,
        inst.col1,
        inst.col2,
    );
    let world_pos = vec4<f32>(pos, 1.0) * mdl2world;
    let screen_pos = vec4<f32>(world_pos, 1.0) * cam.world2screen;
    let scale = cam.scale * vec3<f32>(1.0, 1.0, screen_pos.z);
    let out_pos = vec4<f32>(screen_pos * scale, screen_pos.z);

    var frag_in: VertexToPixel;
    frag_in.pos = out_pos;
    frag_in.color
        = vec4<f32>(inst.color, length(screen_pos) * cam.scale.z);

    return frag_in;
}

@fragment
fn fs_main(vert_out: VertexToPixel) -> FragOut {
    let pos = vert_out.pos;
    let color = vert_out.color.xyz;
    let fog_power = vert_out.color.w;
    let fog_blend = 1.0 - pow(1.0 - FOG.w, fog_power);
    let color_out = mix(color, FOG.xyz, fog_blend) * POST_MULTIPLY;

    var frag: FragOut;
    frag.depth = pos.z;
    frag.color = vec4<f32>(color_out, 1.0);

    return frag;
}
