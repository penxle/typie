fn main() {
  println!("cargo:rerun-if-changed=src/wrapper.h");
  println!("cargo:rerun-if-changed=src/wrapper.cpp");

  let woff2 = glob::glob("vendor/woff2/src/*.cc").unwrap();
  let brotli = glob::glob("vendor/brotli/c/**/*.c").unwrap();

  let mut build = cc::Build::new();

  build
    .cpp(true)
    .warnings(false)
    .flag_if_supported("-Wno-deprecated")
    .include("vendor/woff2/include")
    .include("vendor/brotli/c/include");

  for file in woff2 {
    build.file(file.unwrap());
  }

  for file in brotli {
    build.file(file.unwrap());
  }

  build.compile("woff2");

  cxx_build::bridge("src/lib.rs")
    .file("src/wrapper.cpp")
    .include("src")
    .include("vendor/woff2/include")
    .compile("woff2-bridge");

  napi_build::setup();
}
