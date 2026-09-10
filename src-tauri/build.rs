cat << 'EOF' > src-tauri/build.rs
fn main() {
    tauri_build::build()
}
EOF
