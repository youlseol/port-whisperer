use crate::types::DockerInfo;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::process::Command;

static DOCKER_PORT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:[\d.]+|:::|\*):(\d+)->").unwrap());

/// Shell out to `docker ps` once and return a port→DockerInfo map.
/// Returns empty map if docker is not available or returns no containers.
pub fn get_docker_port_map() -> HashMap<u16, DockerInfo> {
    let output = Command::new("docker")
        .args(["ps", "--format", "{{.Ports}}\t{{.Names}}\t{{.Image}}"])
        .output();

    let mut map = HashMap::new();
    let out = match output {
        Ok(o) if o.status.success() => o,
        _ => return map,
    };

    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        let parts: Vec<&str> = line.splitn(3, '\t').collect();
        if parts.len() < 3 {
            continue;
        }
        let ports_str = parts[0];
        let name = parts[1].to_string();
        let image = parts[2].to_string();

        for cap in DOCKER_PORT_RE.captures_iter(ports_str) {
            if let Ok(port) = cap[1].parse::<u16>() {
                map.insert(
                    port,
                    DockerInfo {
                        name: name.clone(),
                        image: image.clone(),
                    },
                );
            }
        }
    }
    map
}
