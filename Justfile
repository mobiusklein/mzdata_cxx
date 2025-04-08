build_libraries:
    cargo b -r
    cp target/release/*mzdata_cxx* test/lib/
    cp -r target/cxxbridge/* test/include