# Punctum

A minimal framework for creating and launching compute shaders on pure Rust (based on `wgpu` and `rust-gpu`). Allows to write GPU code right in Rust project.

## Benefits
* **Shaders in Rust:** Use macros `#[gpu_kernel]` to compile Rust functions right in SPIR-V.
* **Automatic Bundler:** Internal `punctum-bundler` automatically scans code, collects dependencies and compiling shaders on `build.rs` stage.
* **Simple API:** Builder pattern. Quick creation of Uniform/Storage Buffers.
* **(`punctum-common`):** Contains SDF-primitives, noise generators, text renderer, HUD and tools to work with audio for both CPU and GPU.

## Structure
* `punctum` - Core (`wgpu` context and `winit` window context).
* `punctum-macros` - proc macros (`#[gpu_kernel]`, `include_spv!`).
* `punctum-bundler` - util for creating shaders shadow crate and compiling in SPIR-V.
* `punctum-common` - common library (math, sdf, audio, text, canvas).
* `shader-app` - example of an app. Audio visualizer that captures system audio with `cpal`.

## Quick Start

1. **Add dependencies:**
   ```bash
   cargo add punctum
   cargo add --build punctum-bundler
   ```

2. **Add `build.rs`:**
   ```rust
   fn main() -> Result<(), Box<dyn std::error::Error>> {
       punctum_bundler::run(&punctum_bundler::Config::for_current_package())
   }
   ```

3. **Paste the [Example](#example) into `src/main.rs` and run:**
   ```bash
   cargo run
   ```

## Example

```rust
use punctum::gpu_kernel;
use punctum::spirv_std;
use punctum::anyhow;

#[gpu_kernel(16, 16, 1)]
pub fn grad(
    #[spirv(global_invocation_id)] id: spirv_std::glam::UVec3,
    #[spirv(descriptor_set = 0, binding = 0)] output: &spirv_std::image::StorageImage2d,
) {
    let coords = id.truncate();
    let dims: spirv_std::glam::UVec2 = output.query_size();

    if coords.x < dims.x && coords.y < dims.y {
        let color = spirv_std::glam::vec4(
            coords.x as f32 / dims.x as f32,
            coords.y as f32 / dims.y as f32,
            0.5,
            1.0,
        );

        unsafe {
            output.write(coords, color);
        }
    }
}

fn main() -> anyhow::Result<()> {
    let spv = punctum::include_spv!();

    punctum::builder()
        .with_title("Awesome Engine!")
        .with_kernel(grad)
        .with_spv(spv)
        .with_size(900, 800)
        .run()
}
```

Make sure that your `build.rs` looks like:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    punctum_bundler::run(&punctum_bundler::Config::for_current_package())
}
```

## Run examples

Base example with gradient:
```bash
cargo run --example simple
```

Audio visualizer:
```bash
cargo run -p shader-app
```
