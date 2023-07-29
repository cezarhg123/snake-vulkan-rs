#version 460

layout(location = 0) out vec4 out_color;

layout(binding = 0) uniform UBO {
    vec3 color;
};

void main() {
    out_color = vec4(color, 1.0);
}
