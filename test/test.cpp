#include<memory>
#include<iostream>
#include "cxx.h"
#include "mzdata_cxx.rs.h"

int main(int argc, char *argv[])
{
    auto fname = "batching_test.mzML";
    if (argc > 1)
    {
        fname = argv[1];
    }
    std::cout << "Reading " << fname << std::endl;

    auto reader = mzdata_cpp::open(fname);

    std::cout << "Reading spectrum?" << std::endl;
    // auto spec = reader->next();
    auto spec = reader->get_by_index(0);

    std::printf("Read MS%d spectrum. Spectrum is profile? %d\n", spec->ms_level(), spec->is_profile());
    std::cout << spec->id() << std::endl;

    std::vector<double> mzs;
    std::vector<float> intensities;
    spec->signal_into(mzs, intensities);
    std::cout << "Read " << mzs.size() << " data points" << std::endl;
    try {
        auto precursor = spec->precursor();
        std::cout << "Read precursor?" << std::endl;
        double prec_mz = 0;
        if (precursor->selected_mz(prec_mz))
        {
            std::printf("Selected ion m/z: %f\n", prec_mz);
        }
        auto iso = precursor->isolation_window();
    } catch (std::exception &err) {
        std::cout << "Err: " << err.what() << std::endl;
    }

    std::cout << "Done" << std::endl;
    return 0;
}