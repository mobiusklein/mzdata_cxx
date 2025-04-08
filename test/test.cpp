#include<memory>;
#include<iostream>;
#include "include/rust/cxx.h";
#include "include/mzdata_cxx/src/lib.rs.h";


int main() {
    mzdata_cpp::Spectrum* spec = nullptr;
    auto reader = mzdata_cpp::open("test/batching_test.mzML");
    if (reader->next(*spec)) {
        std::cout << spec->id() << std::endl;
    }
    return 0;
}