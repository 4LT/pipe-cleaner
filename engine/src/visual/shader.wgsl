struct CameraUniforms {
    world2screen: mat3x4<f32>,
    scale: vec3<f32>,
    pixel_height: f32,
}

struct Instance {
    @location(2) col0: vec4<f32>,
    @location(3) col1: vec4<f32>,
    @location(4) col2: vec4<f32>,
    @location(5) color: vec3<f32>,
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

const FOG = vec4<f32>(0.0, 0.0, 0.0, 0.95);
const POST_MULTIPLY = 1.1;
const LINE_WIDTH = 3.0;

fn transform_position(
    mdl2world: mat3x4<f32>,
    position: vec3<f32>,
) -> vec3<f32> {
    let world_pos = vec4<f32>(position, 1.0) * mdl2world;
    return vec4<f32>(world_pos, 1.0) * cam.world2screen;
}

fn scale_position(view_pos: vec3<f32>) -> vec4<f32> {
    let scale = cam.scale * vec3<f32>(1.0, 1.0, view_pos.z);
    return vec4<f32>(view_pos * scale, view_pos.z);
}

fn project(position: vec4<f32>) -> vec3<f32> {
    return vec3<f32>(position.xyz /  position.w);
}

@vertex
fn vs_main(
    @builtin(vertex_index) idx: u32,
    @location(0) pos1: vec3<f32>,
    @location(1) pos2: vec3<f32>,
    inst: Instance,
) -> VertexToPixel {
    let mdl2world = mat3x4<f32>(
        inst.col0,
        inst.col1,
        inst.col2,
    );

    let view_pos1 = transform_position(mdl2world, pos1);
    let view_pos2 = transform_position(mdl2world, pos2);

    let pos1_pre_proj = scale_position(view_pos1);
    let pos2_pre_proj = scale_position(view_pos2);

    let pos1_post_proj = project(pos1_pre_proj);
    let pos2_post_proj = project(pos2_pre_proj);

    let w_h_ratio = cam.scale.y / cam.scale.x;

    let pixel_dim = vec2<f32>(w_h_ratio, 1.0)
        * cam.pixel_height;

    let screen_pos1 = pos1_post_proj.xy / 2.0 * pixel_dim;
    let screen_pos2 = pos2_post_proj.xy / 2.0 * pixel_dim;

    let delta_norm = normalize(screen_pos2 - screen_pos1);

    let offset = delta_norm.yx * vec2<f32>(-1.0, 1.0)
        / pixel_dim * LINE_WIDTH;

    let quad_vert_idx = idx % 4;

    var quad_vert_pos: vec3<f32>;
    var fog_dist: f32;

    if quad_vert_idx == 0 {
        quad_vert_pos = pos1_post_proj + vec3<f32>(offset, 0.0);
        fog_dist = length(view_pos1);
    }

    if quad_vert_idx == 1 {
        quad_vert_pos  = pos1_post_proj + vec3<f32>(-offset, 0.0);
        fog_dist = length(view_pos1);
    }

    if quad_vert_idx == 2 {
        quad_vert_pos = pos2_post_proj + vec3<f32>(-offset, 0.0);
        fog_dist = length(view_pos2);
    }

    if quad_vert_idx == 3 {
        quad_vert_pos = pos2_post_proj + vec3<f32>(offset, 0.0);
        fog_dist = length(view_pos2);
    }

    var frag_in: VertexToPixel;
    frag_in.pos = vec4<f32>(quad_vert_pos, 1.0);
    frag_in.color
        = vec4<f32>(inst.color, fog_dist * cam.scale.z);

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
