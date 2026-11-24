// StarPlay: liquid glass under the controls, computed by mpv (libplacebo) on the final image.
// Simple optics, no noise: a block of glass with rounded edges refracts the video outward
// near the edge (lens), a slight blur, a tint for legibility and a light rim.
// Panel, menu and mode switch (rounded rectangles) + three circles for the window buttons,
// placed by the application via glsl-shader-opts, in window pixels.

//!PARAM origin_x
//!TYPE DYNAMIC float
0.0
//!PARAM origin_y
//!TYPE DYNAMIC float
0.0
//!PARAM panel_x
//!TYPE DYNAMIC float
0.0
//!PARAM panel_y
//!TYPE DYNAMIC float
0.0
//!PARAM panel_w
//!TYPE DYNAMIC float
0.0
//!PARAM panel_h
//!TYPE DYNAMIC float
0.0
//!PARAM panel_r
//!TYPE DYNAMIC float
0.0
//!PARAM panel_a
//!TYPE DYNAMIC float
0.0
//!PARAM menu_x
//!TYPE DYNAMIC float
0.0
//!PARAM menu_y
//!TYPE DYNAMIC float
0.0
//!PARAM menu_w
//!TYPE DYNAMIC float
0.0
//!PARAM menu_h
//!TYPE DYNAMIC float
0.0
//!PARAM menu_r
//!TYPE DYNAMIC float
0.0
//!PARAM menu_a
//!TYPE DYNAMIC float
0.0
//!PARAM win_x
//!TYPE DYNAMIC float
0.0
//!PARAM win_y
//!TYPE DYNAMIC float
0.0
//!PARAM win_d
//!TYPE DYNAMIC float
0.0
//!PARAM win_step
//!TYPE DYNAMIC float
0.0
//!PARAM win_a
//!TYPE DYNAMIC float
0.0
//!PARAM mode_x
//!TYPE DYNAMIC float
0.0
//!PARAM mode_y
//!TYPE DYNAMIC float
0.0
//!PARAM mode_w
//!TYPE DYNAMIC float
0.0
//!PARAM mode_h
//!TYPE DYNAMIC float
0.0
//!PARAM mode_r
//!TYPE DYNAMIC float
0.0
//!PARAM mode_a
//!TYPE DYNAMIC float
0.0
//!PARAM scale
//!TYPE DYNAMIC float
1.0

//!HOOK OUTPUT
//!BIND HOOKED
//!DESC StarPlay liquid glass

float sd_box(vec2 p, vec4 rect, float r) {
    vec2 h = rect.zw * 0.5;
    vec2 q = abs(p - rect.xy - h) - h + vec2(r);
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}

// Image seen through the glass at pixel p (window coordinates).
vec3 through_glass(vec2 p, vec4 rect, float r) {
    float s = scale;
    float d = sd_box(p, rect, r);
    vec2 e = vec2(1.0, 0.0);
    vec2 n = normalize(vec2(sd_box(p + e.xy, rect, r) - sd_box(p - e.xy, rect, r),
                            sd_box(p + e.yx, rect, r) - sd_box(p - e.yx, rect, r)) + 1e-6);
    float edge = min(20.0 * s, 0.5 * min(rect.z, rect.w));
    float t = clamp(-d / edge, 0.0, 1.0);      // 0 at the edge, 1 at the core
    float bend = pow(1.0 - t, 2.5) * 1.3 * edge; // the rounded edge bends strongly, the core barely
    vec2 origin = vec2(origin_x, origin_y);
    // Circles (buttons): true magnifier, the image enlarges toward the center (×1.8); the edge always bends outward
    bool round_shape = 2.0 * r >= min(rect.z, rect.w) - 0.5;
    vec2 center = rect.xy + 0.5 * rect.zw;
    vec2 q = round_shape ? center + (p - center) * 0.55 : p;

    // Spiral blur around the refracted point, with slight R/B dispersion
    vec3 acc = vec3(0.0);
    const int N = 32;
    float radius = (round_shape ? 10.0 : 7.0) * s; // sharp blur in small circles
    for (int i = 0; i < N; i++) {
        float k = float(i) + 0.5;
        vec2 o = radius * sqrt(k / float(N)) * vec2(cos(k * 2.39996), sin(k * 2.39996));
        vec2 base = q - origin + o;
        acc.r += HOOKED_tex((base + n * bend * 1.08) * HOOKED_pt).r;
        acc.g += HOOKED_tex((base + n * bend) * HOOKED_pt).g;
        acc.b += HOOKED_tex((base + n * bend * 0.92) * HOOKED_pt).b;
    }
    vec3 c = acc / float(N);

    c = mix(c, vec3(0.0), 0.12);                 // very slight dark tint for icon legibility
    c += pow(1.0 - t, 4.0) * 0.05;               // the glass lightens slightly toward the edge
    // Rim: light coming from the top left, weaker at the bottom right
    float rim = smoothstep(1.6 * s, 0.0, -d);
    float light = 0.25 + 0.75 * pow(max(dot(n, normalize(vec2(-0.55, -0.85))), 0.0), 1.5)
                + 0.35 * pow(max(dot(n, normalize(vec2(0.55, 0.85))), 0.0), 3.0);
    c += rim * light * 0.55;
    return c;
}

vec3 apply_glass(vec3 col, vec2 p, vec4 rect, float r, float a) {
    if (a <= 0.0 || rect.z <= 0.0)
        return col;
    float d = sd_box(p, rect, r);
    if (d > 0.0)
        return col; // no shadow under the glass: only elements to be read have one (in the interface)
    float cover = clamp(0.5 - d, 0.0, 1.0);
    return mix(col, through_glass(p, rect, r), cover * a);
}

vec4 hook() {
    vec4 col = HOOKED_texOff(0);
    vec2 p = HOOKED_pos * HOOKED_size + vec2(origin_x, origin_y);
    col.rgb = apply_glass(col.rgb, p, vec4(panel_x, panel_y, panel_w, panel_h), panel_r, panel_a);
    col.rgb = apply_glass(col.rgb, p, vec4(menu_x, menu_y, menu_w, menu_h), menu_r, menu_a);
    col.rgb = apply_glass(col.rgb, p, vec4(mode_x, mode_y, mode_w, mode_h), mode_r, mode_a);
    for (int i = 0; i < 3; i++)
        col.rgb = apply_glass(col.rgb, p, vec4(win_x + float(i) * win_step, win_y, win_d, win_d), 0.5 * win_d, win_a);
    return col;
}
