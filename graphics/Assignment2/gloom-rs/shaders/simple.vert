#version 430 core

in layout(location = 0) vec3 position;
in layout(location = 1) vec4 color;
uniform layout(location = 2) mat4 matrix;
//uniform layout(location = 5) mat4 test;

out layout(location=0) vec4 outVertexColor;

void main()
{
    outVertexColor = color;

    mat4 mat;
    mat[0] = vec4(1, 0, 0, 0);
    mat[1] = vec4(0, 1, 0, 0);
    mat[2] = vec4(0, 0, 1, 0);
    mat[3] = vec4(0, 0, 0, 1);

    gl_Position = matrix * vec4(position, 1.0f);
}