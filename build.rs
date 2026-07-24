use std::path::Path;
fn main() {
    let dist = Path::new("frontend/dist/index.html");
    if !dist.exists() {
        println!("cargo:warning=frontend/dist/index.html 不存在。请先运行：cd frontend && pnpm build");
    }
    println!("cargo:rerun-if-changed=frontend/dist/index.html");
}
