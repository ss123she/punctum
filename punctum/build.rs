fn main() -> Result<(), Box<dyn std::error::Error>> {
    punctum_bundler::run(&punctum_bundler::Config {
        scan_dirs: vec!["src".to_string(), "examples".to_string()],
        shadow_subdir: "punctum_shaders".to_string(),
        env_var: "COMPUTE_SPV".to_string(),
        skip_roots: vec!["punctum_macros".to_string()],
    })
}
