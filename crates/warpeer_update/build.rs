fn main() {
    println!("cargo:rerun-if-env-changed=WARPEER_UPDATE_PUBLIC_KEY");
    println!("cargo:rerun-if-env-changed=GIT_RELEASE_TAG");
}
