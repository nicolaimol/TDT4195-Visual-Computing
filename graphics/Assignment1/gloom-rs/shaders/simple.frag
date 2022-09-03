#version 430 core

out vec4 color;

void main()
{

    vec4 white = vec4(1.0f, 1.0f, 1.0f, 1.0f);
    vec4 black = vec4(0.0f, 0.0f, 0.0f, 0.0f);


    if ((int(floor(gl_FragCoord.xy.y)) % 20 < 10 &&  int(floor(gl_FragCoord.xy.x)) % 20 >= 10 )  ||
     (int(floor(gl_FragCoord.xy.y)) % 20 >= 10 &&  int(floor(gl_FragCoord.xy.x)) % 20 < 10 )){
        color = black;
    } else {
        color = white;
    }
}

