// SPDX-License-Identifier: MIT

// A 3D note: one flat ribbon lying on its lane, as long as the note lasts.
// Its front edge is the attack — it reaches the hit line on time — and a
// bright cap of fixed world length marks it, so even the shortest note has
// a crisp edge and two notes back to back on one hole read as two attacks.
//
// The technique is drawn along the ribbon in *world* units, so it scrolls
// with the note: a vibrato's crests (and a wah's open/close bulges) are
// spaced so they cross the hit line at exactly the charted rate — the
// ribbon itself is the pulse to follow. A bend leans the ribbon's core to
// one side over its first stretch, further for a deeper bend.
//
// uv.y = 1 is the front edge (nearest the player), uv.y = 0 the back;
// uv.x runs across the lane.

import bevy_pbr::render::forward_io::VertexOutput;

// technique: x = mode (0 plain, 1 bend, 2 vibrato, 3 wah, 4 pitch-up shift),
//            y = oscillation cycles per world unit (vibrato/wah),
//            z = lean across the lane at full bend, signed (-0.5..0.5),
//            w = 1 when a vibrato rides on a bend
// shape:     x = ribbon length (world units), y = cap length (world units),
//            z = animation time (seconds), w = per-note phase
// hold:      x = pitch sounding now (1/0), y = hold integrity so far (0..1),
//            z = technique status (1 confirmed, -1 not heard, 0 n/a), w = 1
//            once hit — all zero until then
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> color: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> technique: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var<uniform> shape: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var<uniform> hold: vec4<f32>;

const TAU: f32 = 6.2831853;

fn ease(t: f32) -> f32 {
    let x = clamp(t, 0.0, 1.0);
    return x * x * (3.0 - 2.0 * x);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let len = max(shape.x, 0.001);
    let d = (1.0 - uv.y) * len; // world distance back from the front edge
    let t = shape.z;
    let phase = shape.w;

    let mode = technique.x;
    let cyc = technique.y;
    let lean = technique.z;
    let is_bend = f32(mode > 0.5 && mode < 1.5);
    let is_vib = f32(mode > 1.5 && mode < 2.5) + technique.w;
    let is_wah = f32(mode > 2.5 && mode < 3.5);
    let is_up = f32(mode > 3.5);

    // Core line across the lane: a bend or pitch-up shift leans it over the
    // first world unit, a vibrato rides a sine on top. A wah pinches the
    // ribbon's width instead.
    let wave = sin(d * cyc * TAU);
    let shift = (is_bend + is_up) * lean * ease(d / 1.0);
    let centre = 0.5 + shift + clamp(is_vib, 0.0, 1.0) * 0.16 * wave;
    let open = 0.5 + 0.5 * cos(d * cyc * TAU);
    let width = mix(1.0, 0.35 + 0.65 * open, is_wah);

    let from_centre = abs(uv.x - 0.5);
    let body_half = 0.5 * width;
    let aa = fwidth(uv.x) + 0.002;
    let body = 1.0 - smoothstep(body_half - aa, body_half + aa, from_centre);

    // A bright core stripe tracing the technique; plain notes keep a soft,
    // wide one so they don't read as flat slabs.
    let core_half = mix(0.22, 0.07, f32(mode > 0.5));
    let core_dist = abs(uv.x - centre);
    let core = 1.0 - smoothstep(core_half - aa, core_half + aa, core_dist);

    // The cap: a solid bar at the front edge, the attack.
    let cap = f32(d < shape.y);

    // Hold state, once hit (hold is all zero before): gold while the
    // expected pitch is sounding, paler for hold lost so far, dim grey the
    // moment it drops; a confirmed sustained technique shimmers the gold
    // and one not heard yet flattens it. Arithmetic masks, not branches:
    // `fwidth` above needs uniform control flow.
    let held = f32(hold.x > 0.5);
    let hit = f32(hold.w > 0.5);
    let confirmed = f32(hold.z > 0.5);
    let unheard = f32(hold.z < -0.5);
    let shimmer = 0.5 + 0.5 * sin(d * 3.0 * TAU - t * 9.0 + phase);
    let gold = vec3<f32>(1.0, 0.86, 0.35);
    let gold_lift = (0.45 + 0.35 * hold.y) + 0.2 * shimmer * confirmed - 0.3 * unheard;
    let held_rgb = mix(color.rgb, gold, clamp(gold_lift, 0.0, 1.0));
    let dropped_rgb = mix(color.rgb, vec3<f32>(0.42, 0.42, 0.48), 0.75);
    let base_rgb = mix(color.rgb, mix(dropped_rgb, held_rgb, held), hit);

    let core_rgb = mix(base_rgb, vec3<f32>(1.0, 1.0, 1.0), 0.45);
    let cap_rgb = mix(mix(base_rgb, vec3<f32>(1.0, 1.0, 1.0), 0.6), gold, hit);

    var rgb = mix(base_rgb * 0.8, core_rgb, core);
    rgb = mix(rgb, cap_rgb, cap);

    let body_alpha = mix(0.55, 0.95, core);
    let hold_alpha = mix(1.0, mix(0.5, 1.0, held), hit);
    let alpha = max(body * body_alpha, cap) * hold_alpha;

    return vec4<f32>(rgb, alpha * color.a);
}
