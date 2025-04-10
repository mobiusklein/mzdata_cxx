#include<memory>
#include<iostream>
#include<cxx.h>
#include<lib.rs.h>

int main() {
    mzdata_cpp::Spectrum* spec = nullptr;
    auto reader = mzdata_cpp::open("test/batching_test.mzML");
    std::cout << "Reading spectrum?" << std::endl;
    if (reader->next(*spec)) {
        std::cout << "Read spectrum" << std::endl;
        std::cout << spec->id() << std::endl;
    }
    std::cout << "Done" << std::endl;
    return 0;
}