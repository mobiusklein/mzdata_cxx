#include<memory>
#include<iostream>
#include<cxx.h>
#include<lib.rs.h>

int main() {
    mzdata_cpp::Spectrum* spec = nullptr;
    auto reader = mzdata_cpp::open("test/batching_test.mzML");
    if (reader->next(*spec)) {
        std::cout << spec->id() << std::endl;
    }
    return 0;
}