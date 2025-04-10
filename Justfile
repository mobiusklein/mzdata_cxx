build_libraries:
    cargo b -r
    cp target/release/*mzdata_cxx* test/lib/
    cp -r target/cxxbridge/rust/* test/
    cp -r target/cxxbridge/mzdata_cxx/src/* test/