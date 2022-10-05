// Uncomment these following global attributes to silence most warnings of "low" interest:
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(unreachable_code)]
#![allow(unused_mut)]
#![allow(unused_unsafe)]
#![allow(unused_variables)]
#![allow(unused_assignments)]

extern crate nalgebra_glm as glm;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::{mem, os::raw::c_void, ptr};

mod shader;
mod util;
mod mesh;
mod scene_graph;
mod toolbox;


use glutin::event::{
    DeviceEvent,
    ElementState::{Pressed, Released},
    Event, KeyboardInput,
    VirtualKeyCode::{self, *},
    WindowEvent,
};
use glutin::event_loop::ControlFlow;

// initial window size
const INITIAL_SCREEN_W: u32 = 800;
const INITIAL_SCREEN_H: u32 = 600;


// == // Helper functions to make interacting with OpenGL a little bit prettier. You *WILL* need these! // == //

// Get the size of an arbitrary array of numbers measured in bytes
// Example usage:  pointer_to_array(my_array)
fn byte_size_of_array<T>(val: &[T]) -> isize {
    std::mem::size_of_val(&val[..]) as isize
}

// Get the OpenGL-compatible pointer to an arbitrary array of numbers
// Example usage:  pointer_to_array(my_array)
fn pointer_to_array<T>(val: &[T]) -> *const c_void {
    &val[0] as *const T as *const c_void
}

// Get the size of the given type in bytes
// Example usage:  size_of::<u64>()
fn size_of<T>() -> i32 {
    mem::size_of::<T>() as i32
}

// Get an offset in bytes for n units of type T, represented as a relative pointer
// Example usage:  offset::<u64>(4)
fn offset<T>(n: u32) -> *const c_void {
    (n * mem::size_of::<T>() as u32) as *const T as *const c_void
}

// Get a null pointer (equivalent to an offset of 0)
// ptr::null()

// == // Generate your VAO here
unsafe fn create_vao(vertices: &Vec<f32>, indices: &Vec<u32>, colors: &Vec<f32>, normals: &Vec<f32> ) -> u32 {

    let mut vao_id = 0;
    gl::GenVertexArrays(1, &mut vao_id);
    gl::BindVertexArray(vao_id);

    let mut buffer_id = 0;
    gl::GenBuffers(1, &mut buffer_id);
    gl::BindBuffer(gl::ARRAY_BUFFER, buffer_id);

    gl::BufferData(
        gl::ARRAY_BUFFER,
        byte_size_of_array(vertices),
        pointer_to_array(vertices),
        gl::STATIC_DRAW,
    );

    let mut index_buffer_id = 0;
    gl::GenBuffers(1, &mut index_buffer_id);
    gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, index_buffer_id);

    gl::BufferData(
        gl::ELEMENT_ARRAY_BUFFER,
        byte_size_of_array(indices),
        pointer_to_array(indices),
        gl::STATIC_DRAW,
    );

    let vertices_index = 0;
    gl::EnableVertexAttribArray(vertices_index);
    gl::VertexAttribPointer(
        vertices_index,
        3, //[x,y,z],
        gl::FLOAT,
        gl::FALSE,
        0, // same type for all values
        ptr::null()
    );

    let mut buffer_color_id = 0;
    gl::GenBuffers(1, &mut buffer_color_id);
    gl::BindBuffer(gl::ARRAY_BUFFER, buffer_color_id);
    gl::BufferData(
        gl::ARRAY_BUFFER,
        byte_size_of_array(colors),
        pointer_to_array(colors),
        gl::STATIC_DRAW
    );

    let color_index = 1;
    gl::EnableVertexAttribArray(color_index);
    gl::VertexAttribPointer(
        color_index,
        4,
        gl::FLOAT, 
        gl::FALSE, 
        0, 
        ptr::null()
    );

    /* Vertex Buffer Object for normals */
    let mut buffer_normal_id = 0;
    gl::GenBuffers(1, &mut buffer_normal_id);
    gl::BindBuffer(gl::ARRAY_BUFFER, buffer_normal_id);
    gl::BufferData(
        gl::ARRAY_BUFFER,
        byte_size_of_array(normals),
        pointer_to_array(normals),
        gl::STATIC_DRAW,
    );
    let normals_index = 2;
    gl::EnableVertexAttribArray(normals_index);
    // The VertexAttribPointer give the Vertex shader info about the data
    gl::VertexAttribPointer(
        normals_index,
        3, // 3 floats -> XYZ
        gl::FLOAT,
        gl::FALSE,   // Whether OpenGL should normalize the values in the buffer
        0, // All floats, so OpenGL fixes this. Specify a value != 0 if there are multiple types (e.g. float, integers) in one entry
        ptr::null(), // Array buffer offset
    );


    return vao_id;
}

unsafe fn create_vao_from_mesh(mesh: &mesh::Mesh) -> u32 {
    return create_vao(&mesh.vertices, &mesh.indices, &mesh.colors, &mesh.normals);
}

unsafe fn draw_scene(root: &scene_graph::SceneNode, view_projection_matrix: &glm::Mat4) {
    // Check if node is drawable, set uniforms, draw
    if root.index_count > 0 {
        gl::UniformMatrix4fv(5, 1, 0, (view_projection_matrix * root.current_transformation_matrix).as_ptr());
        gl::UniformMatrix4fv(6, 1, 0, (root.current_transformation_matrix).as_ptr());
        gl::BindVertexArray(root.vao_id);
        gl::DrawElements(gl::TRIANGLES, root.index_count, gl::UNSIGNED_INT, ptr::null());
    }

    // Recurse
    for &child in &root.children {
        draw_scene(&*child, view_projection_matrix);
    }
}


unsafe fn update_node_transformations(root: &mut scene_graph::SceneNode, transformation_so_far: &glm::Mat4) {
    // Construct the correct transformation matrix
    let origin = glm::mat4(
        1.0, 0.0, 0.0, root.reference_point[0],
        0.0, 1.0, 0.0, root.reference_point[1],
        0.0, 0.0, 1.0, root.reference_point[2],
        0.0, 0.0, 0.0, 1.0,
    );

    let rotate_x = glm::mat4(
        1.0, 0.0, 0.0, 0.0,
        0.0, root.rotation[0].cos(), -root.rotation[0].sin(), 0.0,
        0.0, root.rotation[0].sin(), root.rotation[0].cos(), 0.0,
        0.0, 0.0, 0.0, 1.0,
    );

    let rotate_y = glm::mat4(
        root.rotation[1].cos(), 0.0, root.rotation[1].sin(), 0.0,
        0.0, 1.0, 0.0, 0.0,
        -root.rotation[1].sin(), 0.0, root.rotation[1].cos(), 0.0,
        0.0, 0.0, 0.0, 1.0,
    );

    let rotate_z = glm::mat4(
        root.rotation[2].cos(), -root.rotation[2].sin(), 0.0, 0.0,
        root.rotation[2].sin(), root.rotation[2].cos(), 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    );

    let inverse_origin = glm::mat4(
        1.0, 0.0, 0.0, -root.reference_point[0],
        0.0, 1.0, 0.0, -root.reference_point[1],
        0.0, 0.0, 1.0, -root.reference_point[2],
        0.0, 0.0, 0.0, 1.0,
    );

    let translation = glm::mat4(
        1.0, 0.0, 0.0, root.position[0],
        0.0, 1.0, 0.0, root.position[1],
        0.0, 0.0, 1.0, root.position[2],
        0.0, 0.0, 0.0, 1.0,
    );
    // Update the node's transformation matrix
    root.current_transformation_matrix = transformation_so_far * translation * origin * rotate_x * rotate_y * rotate_z * inverse_origin;

    // Recurse
    for &child in &root.children {
        update_node_transformations(&mut *child,
        &root.current_transformation_matrix);
    }
}

fn main() {
    // Set up the necessary objects to deal with windows and event handling
    let el = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new()
        .with_title("Gloom-rs")
        .with_resizable(false)
        .with_inner_size(glutin::dpi::LogicalSize::new(
            INITIAL_SCREEN_W,
            INITIAL_SCREEN_H,
        ));
    let cb = glutin::ContextBuilder::new().with_vsync(true);
    let windowed_context = cb.build_windowed(wb, &el).unwrap();
    // Uncomment these if you want to use the mouse for controls, but want it to be confined to the screen and/or invisible.
    // windowed_context.window().set_cursor_grab(true).expect("failed to grab cursor");
    // windowed_context.window().set_cursor_visible(false);

    // Set up a shared vector for keeping track of currently pressed keys
    let arc_pressed_keys = Arc::new(Mutex::new(Vec::<VirtualKeyCode>::with_capacity(10)));
    // Make a reference of this vector to send to the render thread
    let pressed_keys = Arc::clone(&arc_pressed_keys);

    // Set up shared tuple for tracking mouse movement between frames
    let arc_mouse_delta = Arc::new(Mutex::new((0f32, 0f32)));
    // Make a reference of this tuple to send to the render thread
    let mouse_delta = Arc::clone(&arc_mouse_delta);

    // Set up shared tuple for tracking changes to the window size
    let arc_window_size = Arc::new(Mutex::new((INITIAL_SCREEN_W, INITIAL_SCREEN_H, false)));
    // Make a reference of this tuple to send to the render thread
    let window_size = Arc::clone(&arc_window_size);

    // Spawn a separate thread for rendering, so event handling doesn't block rendering
    let render_thread = thread::spawn(move || {
        // Acquire the OpenGL Context and load the function pointers.
        // This has to be done inside of the rendering thread, because
        // an active OpenGL context cannot safely traverse a thread boundary
        let context = unsafe {
            let c = windowed_context.make_current().unwrap();
            gl::load_with(|symbol| c.get_proc_address(symbol) as *const _);
            c
        };

        //let mut window_aspect_ratio = INITIAL_SCREEN_W as f32 / INITIAL_SCREEN_H as f32;

        // Set up openGL
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthFunc(gl::LESS);
            gl::Enable(gl::CULL_FACE);
            gl::Disable(gl::MULTISAMPLE);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
            gl::DebugMessageCallback(Some(util::debug_callback), ptr::null());

            // Print some diagnostics
            println!(
                "{}: {}",
                util::get_gl_string(gl::VENDOR),
                util::get_gl_string(gl::RENDERER)
            );
            println!("OpenGL\t: {}", util::get_gl_string(gl::VERSION));
            println!(
                "GLSL\t: {}",
                util::get_gl_string(gl::SHADING_LANGUAGE_VERSION)
            );
        }

        
        // in different z
        /*
        let vertices: Vec<f32> = vec![
            -0.0, 0.6, 0.2, // 0 top center
            0.3, 0.0, 0.2, // 1 left center
            -0.3, 0.0, 0.2, // 2 right center

            0.6 ,0.0, 0.4, // 3 bottom right
            0.9 , 0.6, 0.4, // 4 right right
            0.3, 0.6, 0.4, // 5 left right

            -0.6 ,0.0, 0.0, // 6 bottom left
            -0.9 , 0.6, 0.0, // 7 right left
            -0.3, 0.6, 0.0, // 8 left left

            -0.45, 0.0, -0.2, // 9 top lower left
            -0.75, -0.6, -0.2, // 10 left lower left
            -0.15, -0.6, -0.2, // 11 rigth lower left


            0.45, 0.0, -0.4, // 12 top lower right
            0.75, -0.6, -0.4, // 13 right lower right
            0.15, -0.6, -0.4, // 14 left lower right

        ];
    

        let indices: Vec<u32> = vec![
            2, 1, 0, 
            3, 4, 5,
            6, 8, 7,
            9, 10, 11,
            12, 14, 13,
        ];
        
        let colors: Vec<f32> = vec![
            1.0, 0.0, 1.0, 0.4, //      Center 0
            0.0, 1.0, 1.0, 0.4, //      Left center 1
            1.0, 1.0, 0.0, 0.4, //      Right center 2
            0.0, 1.0, 1.0, 0.4, //      Bottom most left 3
            1.0, 0.0, 1.0, 0.4, //      Bottom left 4
            1.0, 1.0, 0.0, 0.4, //      Bottom a bit left 5
            0.0, 1.0, 1.0, 0.4, //      Bottom a bit right 6
            1.0, 0.0, 1.0, 0.4, //      Bottom right 7
            1.0, 1.0, 0.0, 0.4, //      Bottom most right 8
            1.0, 0.0, 1.0, 0.4, //      Center 0
            0.0, 1.0, 1.0, 0.4, //      Left center 1
            1.0, 1.0, 0.0, 0.4, //      Right center 2
            0.0, 1.0, 1.0, 0.4, //      Bottom most left 3
            1.0, 0.0, 1.0, 0.4, //      Bottom left 4
            1.0, 1.0, 0.0, 0.4, //      Bottom a bit left 5
        ];
        

        
        let vertices: Vec<f32> = vec![
            0.0, 0.0, 0.0, //       Center 0
            -0.5, 0.0, -0.5, //     Left center 1
            0.5, 0.0, 0.5, //       Right center 2
            -0.75, -0.5, -0.5, //   Bottom most left 3
            -0.25, -0.5, 0.0, //    Bottom left 4
            -0.10, -0.5, 0.5, //    Bottom a bit left 5
            0.10, -0.5, -0.5, //    Bottom a bit right 6
            0.25, -0.5, 0.0, //     Bottom right 7
            0.75, -0.5, 0.5, //     Bottom most right 8
        ];

        let indices: Vec<u32> = vec![
            1, 3, 6, // Bottom left triangle
            0, 4, 7, // Bottom center triangle
            2, 5, 8, // Bottom right triangle
        ];
        
        // One RGBA value per vertex in vertices i.e. 4 values here per 3 values in vertices
        // The triangle-indexes given by indicies are equal here
        let colors: Vec<f32> = vec![
            1.0, 1.0, 1.0, 0.4,
            1.0, 0.0, 1.0, 0.4,
            1.0, 0.0, 1.0, 0.4,

            1.0, 1.0, 1.0, 0.4,
            0.0, 1.0, 1.0, 0.4,
            0.0, 1.0, 1.0, 0.4,
            
            1.0, 1.0, 1.0, 0.4,
            1.0, 1.0, 0.0, 0.4,
            1.0, 1.0, 0.0, 0.4,
            
            
            1.0, 1.0, 1.0, 0.4,
            0.0, 1.0, 1.0, 0.4,
            0.0, 1.0, 1.0, 0.4,
            
            0.0, 0.0, 0.0, 0.4,
            1.0, 1.0, 1.0, 0.4,
            1.0, 1.0, 1.0, 0.4,
        ];
         */
        /* 
        let vertices: Vec<f32> = vec![
            -0.0, 0.6, 0.0, // 0 top center
            0.3, 0.0, 0.0, // 1 left center
            -0.3, 0.0, 0.0, // 2 right center

            0.6 ,0.0, 0.0, // 3 bottom right
            0.9 , 0.6, 0.0, // 4 right right
            0.3, 0.6, 0.0, // 5 left right

            -0.6 ,0.0, 0.0, // 6 bottom left
            -0.9 , 0.6, 0.0, // 7 right left
            -0.3, 0.6, 0.0, // 8 left left
        ];

        let indices: Vec<u32> = vec![
            2, 1, 0, 
            3, 4, 5,
            6, 8, 7,
        ];

        let colors: Vec<f32> = vec![
            1.0, 1.0, 1.0, 0.4,
            1.0, 0.0, 1.0, 0.4,
            1.0, 0.0, 1.0, 0.4,

            1.0, 1.0, 1.0, 0.4,
            0.0, 1.0, 1.0, 0.4,
            0.0, 1.0, 1.0, 0.4,
            
            1.0, 1.0, 1.0, 0.4,
            1.0, 1.0, 0.0, 0.4,
            1.0, 1.0, 0.0, 0.4,
        ];
        */
        /*
        let vertices: Vec<f32> = vec![
            -0.5, -0.5, 0.6,
            0.5, 0.5, 0.6,
            0.0, 0.7, 0.6,

            0.0, 0.0, -0.5,
            0.8, 0.8, -0.5,
            0.2, 0.7, -0.5,

            -0.8, -0.8, -0.3,
            0.4, 0.4, -0.3,
            -0.4, 0.7, -0.3,
        ];
        let indices: Vec<u32> = vec![
            0, 1, 2,
            6, 7, 8,
            
            3, 4, 5,
        ];
        let colors: Vec<f32> = vec![
            0.0, 1.0, 0.0, 0.5,
            0.0, 1.0, 0.0, 0.5,
            0.0, 1.0, 0.0, 0.5,

            0.0, 0.0, 1.0, 0.5,
            0.0, 0.0, 1.0, 0.5,
            0.0, 0.0, 1.0, 0.5,

            1.0, 0.0, 0.0, 0.5,
            1.0, 0.0, 0.0, 0.5,
            1.0, 0.0, 0.0, 0.5,
        ];
         */

        let terrain = mesh::Terrain::load("./resources/lunarsurface.obj");
        let terrain_vao = unsafe { create_vao_from_mesh(&terrain) };

        let mut root_node = scene_graph::SceneNode::new();
        let mut terrain_node = scene_graph::SceneNode::from_vao(terrain_vao, terrain.index_count);

        root_node.add_child(&terrain_node);

        let translate_z_index: glm::Mat4 = glm::mat4(
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            // First scale the z-axis so that [-1, 1] -> [-49.5, 49.5]. Then we translate the z-axis -50.5 to [-100, -1].
            0.0, 0.0, 1.0, -1.0, //
            0.0, 0.0, 0.0, 1.0, //
        );

        let perspective: glm::Mat4 = glm::perspective(
            (INITIAL_SCREEN_W as f32) / (INITIAL_SCREEN_H as f32), // Aspect ratio = width/height
            (60.0 * 3.14) / 180.0,                 // 60 degress FOV, but the function uses radians
            1.0,                                   //
            1000.0,                                 //
        );


        // == // Set up your VAO around here
        //let vao_id = unsafe { create_vao(&vertices, &indices, &colors) };

        // == // Set up your shaders here

        let simple_shader = unsafe {
            shader::ShaderBuilder::new()
                .attach_file("./shaders/simple.vert")
                .attach_file("./shaders/simple.frag")
                .link()
                .activate();
        };

        // Used to demonstrate keyboard handling for exercise 2.
        //let mut _arbitrary_number = 0.0; // feel free to remove

        let mut x = 0.0;
        let mut y = 0.0;
        let mut z = -2.0;
        let mut yaw = 0.0; // Left-right rotation (parallell to the floor)
        let mut pitch = 0.0; // Up-down rotation

        let mut eta: Vec<f32> = vec![
            0.0, 0.0, -2.0, 0.0, 0.0
        ];

        // The main rendering loop
        let first_frame_time = std::time::Instant::now();
        let mut prevous_frame_time = first_frame_time;
        loop {
            // Compute time passed since the previous frame and since the start of the program
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(first_frame_time).as_secs_f32();
            let delta_time = now.duration_since(prevous_frame_time).as_secs_f32();
            prevous_frame_time = now;

            let z_speed = 0.8;
            let x_speed = 2.0;
            let y_speed = 2.0;

            if let Ok(keys) = pressed_keys.lock() {
                for key in keys.iter() {
                    match key {
                        VirtualKeyCode::A => {
                            x += delta_time * x_speed;
                        }
                        VirtualKeyCode::D => {
                            x -= delta_time * x_speed;
                        }
                        VirtualKeyCode::W => {
                            z += delta_time * z_speed;
                        }
                        VirtualKeyCode::S => {
                            z -= delta_time * z_speed;
                        }
                        VirtualKeyCode::LShift => {
                            y -= delta_time * y_speed;
                        }
                        VirtualKeyCode::LControl => {
                            y += delta_time * y_speed;
                        }
                        VirtualKeyCode::Up => {
                            pitch += delta_time;
                        }
                        VirtualKeyCode::Down => {
                            pitch -= delta_time;
                        }
                        VirtualKeyCode::Right => {
                            yaw += delta_time;
                        }
                        VirtualKeyCode::Left => {
                            yaw -= delta_time;
                        }
                        _ => {}
                    }
                }
            }

            // Handle resize events
            /*if let Ok(mut new_size) = window_size.lock() && new_size.2 {
                context.resize(glutin::dpi::PhysicalSize::new(new_size.0, new_size.1));
                window_aspect_ratio = new_size.0 as f32 / new_size.1 as f32;
                (*new_ size).2 = false;
                println!("Resized");
                unsafe { gl::Viewport(0, 0, new_size.0 as i32, new_size.1 as i32); }
            } 
            */

        
            // Handle mouse movement. delta contains the x and y movement of the mouse since last frame in pixels
            if let Ok(mut delta) = mouse_delta.lock() {
                // == // Optionally access the acumulated mouse movement between
                // == // frames here with `delta.0` and `delta.1`

                *delta = (0.0, 0.0); // reset when done
            }

            /* 
            let mut shader_matrix: glm::Mat4 = perspective * translate_z_index; // First we apply the perspective on the z-index translation matrix

            // Perform the camera transformation before rendering
            shader_matrix = glm::translate(&shader_matrix, &glm::vec3(x, 0.0, 0.0));
            shader_matrix = glm::translate(&shader_matrix, &glm::vec3(0.0, y, 0.0));
            shader_matrix = glm::translate(&shader_matrix, &glm::vec3(0.0, 0.0, z));

            //shader_matrix = glm::rotation(x, &glm::vec3(1.0, 0.0, 0.0));
            //shader_matrix = glm::rotation(y, &glm::vec3(0.0, 1.0, 0.0));
            //shader_matrix = glm::rotation(z, &glm::vec3(0.0, 0.0, 1.0));


            //shader_matrix = glm::rotation(pitch, &glm::vec3(1.0, 0.0, 0.0));

            //shader_matrix = rotate_y * rotate_x * shader_matrix;

            shader_matrix = glm::rotate(&shader_matrix, yaw.sin(), &glm::vec3(0.0, 1.0, 0.0));
            shader_matrix = glm::rotate(&shader_matrix, pitch.sin(), &glm::vec3(1.0, 0.0, 0.0));
            */



            // == // Please compute camera transforms here (exercise 2 & 3)

            let matrix = glm::mat4(
                1.0, 0.0, 0.0, x,
                0.0, 1.0, 0.0, y,
                0.0, 0.0, 1.0, z,
                0.0, 0.0, 0.0, 1.0
            );

            let pitch_rotation: glm::Mat4 = glm::mat4(
                    1.0, 0.0, 0.0, 0.0, 
                    0.0, pitch.cos(), -pitch.sin(), 0.0, 
                    0.0, pitch.sin(), pitch.cos(), 0.0, 
                    0.0, 0.0, 0.0, 1.0,
            );

            let yaw_rotation: glm::Mat4 = glm::mat4(
                    yaw.cos(), 0.0, yaw.sin(), 0.0, 
                    0.0, 1.0, 0.0, 0.0, 
                    -yaw.sin(), 0.0, yaw.cos(), 0.0, 
                    0.0, 0.0, 0.0, 1.0,
                );

            let shader_matrix = perspective * yaw_rotation * pitch_rotation * matrix;

            let mut view_projection_matrix: glm::Mat4 = perspective;

            // Perform the camera transformation before rendering
            view_projection_matrix = glm::rotate_y(&view_projection_matrix, yaw);
            view_projection_matrix = glm::rotate_x(&view_projection_matrix, pitch);
            //view_projection_matrix = glm::rotate_z(&view_projection_matrix, roll);
            view_projection_matrix = glm::translate(&view_projection_matrix, &glm::vec3(x, y, z));

            unsafe {
                // Clear the color and depth buffers
                gl::ClearColor(0.035, 0.046, 0.078, 1.0); // night sky, full opacity
                gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

                // == // Issue the necessary gl:: commands to draw your scene here

                draw_scene(&root_node, &view_projection_matrix)

                //gl::BindVertexArray(vao_id);

                //gl::UniformMatrix4fv(2, 1, gl::FALSE, shader_matrix.as_ptr()); // layout (location = 2), pass 1 matrix

                /*gl::DrawElements(
                    gl::TRIANGLES,
                    indices.len() as i32,
                    gl::UNSIGNED_INT,
                    ptr::null(),
                );*/
            }

            // Display the new color buffer on the display
            context.swap_buffers().unwrap(); // we use "double buffering" to avoid artifacts
        }
    });

    // == //
    // == // From here on down there are only internals.
    // == //

    // Keep track of the health of the rendering thread
    let render_thread_healthy = Arc::new(RwLock::new(true));
    let render_thread_watchdog = Arc::clone(&render_thread_healthy);
    thread::spawn(move || {
        if !render_thread.join().is_ok() {
            if let Ok(mut health) = render_thread_watchdog.write() {
                println!("Render thread panicked!");
                *health = false;
            }
        }
    });

    // Start the event loop -- This is where window events are initially handled
    el.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        // Terminate program if render thread panics
        if let Ok(health) = render_thread_healthy.read() {
            if *health == false {
                *control_flow = ControlFlow::Exit;
            }
        }

        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(physical_size),
                ..
            } => {
                println!(
                    "New window size! width: {}, height: {}",
                    physical_size.width, physical_size.height
                );
                if let Ok(mut new_size) = arc_window_size.lock() {
                    *new_size = (physical_size.width, physical_size.height, true);
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            // Keep track of currently pressed keys to send to the rendering thread
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: key_state,
                                virtual_keycode: Some(keycode),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                if let Ok(mut keys) = arc_pressed_keys.lock() {
                    match key_state {
                        Released => {
                            if keys.contains(&keycode) {
                                let i = keys.iter().position(|&k| k == keycode).unwrap();
                                keys.remove(i);
                            }
                        }
                        Pressed => {
                            if !keys.contains(&keycode) {
                                keys.push(keycode);
                            }
                        }
                    }
                }

                // Handle Escape and Q keys separately
                match keycode {
                    Escape => {
                        *control_flow = ControlFlow::Exit;
                    }
                    Q => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => {}
                }
            }
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                // Accumulate mouse movement
                if let Ok(mut position) = arc_mouse_delta.lock() {
                    *position = (position.0 + delta.0 as f32, position.1 + delta.1 as f32);
                }
            }
            _ => {}
        }
    });
}
 

