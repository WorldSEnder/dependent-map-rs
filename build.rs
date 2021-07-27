#[rustversion::nightly]
fn main() {
    println!("cargo:rustc-cfg=feature=\"unstable_features\"");
}

#[rustversion::not(nightly)]
fn main() {}
