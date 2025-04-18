#include<memory>
#include<iostream>
#include "cxx.h"
#include "lib.rs.h"

int main() {
    auto reader = mzdata_cpp::open("batching_test.mzML");
    std::cout << "Reading spectrum?" << std::endl;
    auto spec = reader->next();

    std::printf("Read MS%d spectrum\n", spec->ms_level());
    std::cout << spec->id() << std::endl;

    std::vector<double> mzs;
    std::vector<float> intensities;
    spec->signal_into(mzs, intensities);
    std::cout << "Read " << mzs.size() << " data points" << std::endl;
    auto precursor = spec->precursor();

    double prec_mz;
    if (precursor->selected_mz(prec_mz)) {
        std::printf("Selected ion m/z: %f", prec_mz);
    }
    auto iso = precursor->isolation_window();
    std::cout << "Done" << std::endl;
    return 0;
}