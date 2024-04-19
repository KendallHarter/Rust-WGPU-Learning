// Vertex shader

struct VertexOutput {
   @builtin(position) clip_position: vec4<f32>,
   @location(0) tex_loc: vec2f,
};

// We can't use const in the actual shader for some reason :c
const screen_width: f32 = 240.0;
const screen_height: f32 = 160.0;
// screen is from -1.0 to 1.0, each tile is 8 pixels
const tile_width: f32 = 2.0 / (screen_width / 8.0);
const tile_height: f32 = 2.0 / (screen_height / 8.0);

// This has to have a 16 byte aligned type so just use vec4u
// We only use the lowest byte tho
@group(1) @binding(0)
var<uniform> tiles: array<vec4<u32>, 1024>;

@vertex
fn vs_main(
   @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
   let tile_no = u32(in_vertex_index / 6);
   let tile_x = u32(tile_no % (256 / 8));
   let tile_y = u32(tile_no / (256 / 8));
   let triangle_index = in_vertex_index % 6;
   var out: VertexOutput;
   var x: f32;
   var y: f32;
   // This could easily be better but whatever
   if triangle_index == 0 {
      x = -1.0 + tile_width * f32(tile_x);
      y = 1.0 - tile_height * f32(tile_y);
      out.tex_loc = vec2<f32>(f32(tiles[tile_no].x) / 1024.0, 0);
   }
   else if triangle_index == 1 {
      x = -1.0 + tile_width * f32(tile_x + 1);
      y = 1.0 - tile_height * f32(tile_y);
      out.tex_loc = vec2<f32>(f32(tiles[tile_no].x + 1) / 1024.0, 0);
   }
   else if triangle_index == 2 {
      x = -1.0 + tile_width * f32(tile_x);
      y = 1.0 - tile_height * f32(tile_y + 1);
      out.tex_loc = vec2<f32>(f32(tiles[tile_no].x) / 1024.0, 1.0);
   }
   else if triangle_index == 3 {
      x = -1.0 + tile_width * f32(tile_x + 1);
      y = 1.0 - tile_height * f32(tile_y);
      out.tex_loc = vec2<f32>(f32(tiles[tile_no].x + 1) / 1024.0, 0.0);
   }
   else if triangle_index == 4 {
      x = -1.0 + tile_width * f32(tile_x + 1);
      y = 1.0 - tile_height * f32(tile_y + 1);
      out.tex_loc = vec2<f32>(f32(tiles[tile_no].x + 1) / 1024.0, 1.0);
   }
   else if triangle_index == 5 {
      x = -1.0 + tile_width * f32(tile_x);
      y = 1.0 - tile_height * f32(tile_y + 1);
      out.tex_loc = vec2<f32>(f32(tiles[tile_no].x) / 1024.0, 1.0);
   }
   out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
   return out;
}

@group(0) @binding(0)
var tile_texture: texture_2d<f32>;
@group(0) @binding(1)
var tile_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
   return textureSample(tile_texture, tile_sampler, in.tex_loc);
}
