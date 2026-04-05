use serde_json::Value;
use std::path::Path;

pub fn detect(project_root: &Path) -> Option<String> {
    let pkg_path = project_root.join("package.json");
    if let Ok(content) = std::fs::read_to_string(&pkg_path) {
        if let Ok(pkg) = serde_json::from_str::<Value>(&content) {
            let deps = pkg.get("dependencies").and_then(Value::as_object);
            let dev = pkg.get("devDependencies").and_then(Value::as_object);
            let all_keys: Vec<&str> = deps
                .iter()
                .chain(dev.iter())
                .flat_map(|m| m.keys().map(String::as_str))
                .collect();

            if all_keys.contains(&"next") { return Some("Next.js".into()); }
            if all_keys.contains(&"nuxt") { return Some("Nuxt".into()); }
            if all_keys.contains(&"@sveltejs/kit") { return Some("SvelteKit".into()); }
            if all_keys.contains(&"vite") { return Some("Vite".into()); }
            if all_keys.contains(&"react-scripts") { return Some("Create React App".into()); }
            if all_keys.contains(&"@angular/core") { return Some("Angular".into()); }
            if all_keys.contains(&"express") { return Some("Express".into()); }
            if all_keys.contains(&"fastify") { return Some("Fastify".into()); }
            if all_keys.contains(&"@nestjs/core") { return Some("NestJS".into()); }
            if all_keys.contains(&"gatsby") { return Some("Gatsby".into()); }
            if all_keys.contains(&"remix") { return Some("Remix".into()); }
            if all_keys.contains(&"astro") { return Some("Astro".into()); }
            if all_keys.contains(&"@remix-run/node") { return Some("Remix".into()); }
            if all_keys.contains(&"hapi") || all_keys.contains(&"@hapi/hapi") { return Some("Hapi".into()); }
            return Some("Node.js".into());
        }
    }

    if project_root.join("Cargo.toml").exists() { return Some("Rust".into()); }
    if project_root.join("go.mod").exists() { return Some("Go".into()); }
    if project_root.join("manage.py").exists() { return Some("Django".into()); }
    if project_root.join("requirements.txt").exists() { return Some("Python".into()); }
    if project_root.join("mix.exs").exists() { return Some("Phoenix".into()); }
    if project_root.join("Gemfile").exists() { return Some("Ruby/Rails".into()); }
    if project_root.join("pom.xml").exists() || project_root.join("build.gradle").exists() {
        return Some("Java".into());
    }
    None
}
