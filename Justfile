build_libraries:
    cargo b -r

make:
    cmake build -B build -S .