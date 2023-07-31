use std::{
    ffi::OsStr,
    fs::{self},
    path::Path,
    sync::Arc,
};

use shaderc::{CompileOptions, OptimizationLevel, ShaderKind, IncludeType, IncludeCallbackResult, ResolvedInclude};
use vulkano::{device::Device, shader::ShaderModule};

pub trait LoadFromPath {
    fn load(device: Arc<Device>, path: &str) -> Arc<Self>;
}

impl LoadFromPath for ShaderModule {
    fn load(device: Arc<Device>, path: &str) -> Arc<Self> {
        let compiler = shaderc::Compiler::new().unwrap();

        let shader_path = format!("./src/shaders/{}", path);
        let src = match fs::read_to_string(shader_path.clone()) {
            Ok(s) => s,
            Err(e) => panic!("Error reading shader file '{}': {}", path, e),
        };

        let extension = Path::new(path).extension().and_then(OsStr::to_str).unwrap();
        let shader_kind = match_shader_ext(extension);
        let mut compile_options = CompileOptions::new().unwrap();

        compile_options.set_include_callback(include_callback);
        compile_options.set_generate_debug_info();
        compile_options.set_optimization_level(OptimizationLevel::Performance);

        // let shader_pre = match compiler.compile_into_spirv_assembly(
        //     &src,
        //     shader_kind,
        //     path,
        //     "main",
        //     Some(&compile_options),
        // ) {
        //     Ok(b) => b,
        //     Err(e) => panic!("Error compiling shader file '{}': {}", path, e),
        // };

        // let mut asm_out = File::create(shader_path + ".spv").unwrap();
        // asm_out.write_all(shader_pre.as_text().as_bytes()).unwrap();

        let shader_binary = match compiler.compile_into_spirv(
            &src,
            shader_kind,
            path,
            "main",
            Some(&compile_options),
        ) {
            Ok(b) => b,
            Err(e) => panic!("Error compiling shader file '{}': {}", path, e),
        };

        unsafe { ShaderModule::from_words(device, shader_binary.as_binary()).unwrap() }
    }
}

fn include_callback(name: &str, _include_type: IncludeType, source_name: &str, _depth: usize) -> IncludeCallbackResult {
    let source_path_str = format!("./src/shaders/{}", source_name);
    let source_path = Path::new(&source_path_str);

    let include_path = source_path.parent().unwrap().join(name);

    let result = fs::read_to_string(include_path);
    let include_file = match result {
        Ok(file) => file,
        Err(e) => return Err(e.to_string()),
    };

    Ok(ResolvedInclude {
        resolved_name: name.to_string(),
        content: include_file,
    })
}

fn match_shader_ext(ext: &str) -> ShaderKind {
    match ext {
        "vert" => ShaderKind::Vertex,
        "frag" => ShaderKind::Fragment,
        "comp" => ShaderKind::Compute,
        e => panic!("Unsupported shader extension: {}", e),
    }
}

pub struct ShaderPair {
    pub vertex: Arc<ShaderModule>,
    pub fragment: Arc<ShaderModule>,
}

impl ShaderPair {
    pub fn new(vertex: Arc<ShaderModule>, fragment: Arc<ShaderModule>) -> Self {
        Self { vertex, fragment }
    }

    pub fn load(device: Arc<Device>, path: &str) -> Self {
        Self {
            vertex: ShaderModule::load(device.clone(), &format!("{path}.vert")),
            fragment: ShaderModule::load(device, &format!("{path}.frag")), // Redundant clone for `device`
        }
    }
}
