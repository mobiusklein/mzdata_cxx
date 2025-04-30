fn main() {
    cxx_build::bridge("src/mzdata_cxx.rs")
        .std("c++20")
        .compile("mzdata_cxx");

    println!("cargo:rerun-if-changed=src/lib.rs");
}
