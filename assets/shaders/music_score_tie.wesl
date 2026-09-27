// SPDX-License-Identifier: MIT

// A tied-note connector for the shared music-notation staff
// (`music_score`), drawn as a real curved arc rather than the flat
// rectangle a plain `bevy_ui` `Node`/`BackgroundColor` is limited to —
// see `music_score::tie_material`'s own module doc comment.
//
// uv.x spans the node horizontally (0 = left end, 1 = right end). The arc's
// two ends sit on the edge nearest the noteheads and it bows away from
// them: from the top edge down for a tie below the notes, and — with
// `params.z` set — from the bottom edge up for one above them.

import bevy_ui_render::ui_vertex_output::UiVertexOutput;

// x = arc depth (fraction of the node's own height the middle dips to),
// y = line thickness (fraction of the node's own height),
// z = 1 for a tie above the notes (the arc bows upward), 0 below.
@group(1) @binding(0) var<uniform> color: vec4<f32>;
@group(1) @binding(1) var<uniform> params: vec4<f32>;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // Mirrored for a tie above the notes, so its ends sit on the bottom edge.
    let uv = vec2<f32>(in.uv.x, select(in.uv.y, 1.0 - in.uv.y, params.z > 0.5));
    let depth = params.x;
    let thickness = params.y;

    // A parabola pinned to 0 at uv.x = 0/1, peaking at `depth` at the
    // midpoint — cheap, and plenty smooth-looking at this scale; no need
    // for a true cubic bezier for a mark this small.
    let curve_y = depth * 4.0 * uv.x * (1.0 - uv.x);
    let dist = abs(uv.y - curve_y);
    let aa = fwidth(uv.y) + 0.01;
    let alpha = 1.0 - smoothstep(thickness - aa, thickness + aa, dist);

    return vec4<f32>(color.rgb, alpha * color.a);
}
