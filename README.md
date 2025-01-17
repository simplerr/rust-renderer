# rust-renderer

![Image](utopian/data/printscreens/printscreen3.jpg)
![Image](utopian/data/printscreens/printscreen4.jpg)
_Example renders using the path tracing mode_

Developed to learn Rust and to have a framework for experimenting with modern rendering techniques

## Features
+ GPU path tracer based on [Ray Tracing in One Weekend](https://raytracing.github.io/books/RayTracingInOneWeekend.html)
+ ReSTIR to improve sampling of analytical point lights
+ Basic PBR rendering
+ Bindless GPU resources
+ Experimental high level render graph
+ Vulkan using [ash](https://github.com/MaikKlein/ash)
+ SSAO, FXAA, IBL, shadow mapping
+ Raytraced shadows and reflections
+ Live shader recompilation
+ Shader reflection
+ Marching cubes demo

## Building

You need [Rust](https://www.rust-lang.org/tools/install) for building

Build and run the project with `cargo run --release`

**Note**: requires a GPU with raytracing support, e.g Nvidia RTX series

## Controls

- `1`, `2` - toggle between path tracing and rasterization
- `W/A/S/D` - move the camera
- `MOUSE + RMB` - rotate the camera
- `Q` - toggle profiling window
- `SPACE` - pause profiler
