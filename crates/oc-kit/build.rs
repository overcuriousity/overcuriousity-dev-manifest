use std::{env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=assets/css");
    let mut layers: Vec<PathBuf> = fs::read_dir("assets/css")?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                n.len() > 3
                    && n.as_bytes()[..2].iter().all(u8::is_ascii_digit)
                    && n.ends_with(".css")
            })
        })
        .collect();
    layers.sort();
    let mut out = String::new();
    for layer in layers {
        out.push_str(&fs::read_to_string(&layer)?);
        out.push('\n');
    }
    fs::write(PathBuf::from(env::var("OUT_DIR")?).join("oc.css"), out)?;
    Ok(())
}
