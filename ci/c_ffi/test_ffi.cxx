#include <iostream>
#include <fstream>
#include <sstream>
#include "gtfsort.hxx"

#define BAIL_IF(x, v) \
    do                \
    {                 \
        if (x)        \
            return v; \
    } while (0)

#define PANIC_IF(x, msg)                                \
    do                                                  \
    {                                                   \
        if (x)                                          \
        {                                               \
            std::cerr << "Panic: " << msg << std::endl; \
            abort();                                    \
        }                                               \
    } while (0)

struct SortAnnotationRetWrapper
{
    SortAnnotationsRet *ret;

    SortAnnotationRetWrapper()
    {
        PANIC_IF(
            nullptr == (ret = gtfsort_new_sort_annotations_ret()),
            "Failed to allocate SortAnnotationsRet");
    }

    ~SortAnnotationRetWrapper()
    {
        gtfsort_free_sort_annotations_ret(*ret);
    }
};

std::ostream &operator<<(std::ostream &os, const SortAnnotationRetWrapper &ret)
{
    if (ret.ret->tag == SortAnnotationsRet::Tag::Ok)
    {
        const auto &result = *ret.ret->ok._0;
        os << "Ok: input=" << result.input << ", output=" << result.output << ", threads=" << result.threads
           << ", input_mmaped=" << result.input_mmaped << ", output_mmaped=" << result.output_mmaped
           << ", parsing_secs=" << result.parsing_secs << ", indexing_secs=" << result.indexing_secs
           << ", writing_secs=" << result.writing_secs << ", start_mem_mb=" << result.start_mem_mb
           << ", end_mem_mb=" << result.end_mem_mb;
    }
    else
    {
        const auto &err = *ret.ret->err._0;
        os << "Err: code=" << err.code << ", message=" << err.message;
    }

    return os;
}

bool cmp_files(const char *file1, const char *file2)
{
    std::ifstream f1(file1);
    std::ifstream f2(file2);

    PANIC_IF(!f1.good(), "Failed to open file1");
    PANIC_IF(!f2.good(), "Failed to open file2");

    BAIL_IF(f1.tellg() != f2.tellg(), false);

    f1.seekg(0, std::ios::beg);
    f2.seekg(0, std::ios::beg);

    return std::equal(std::istreambuf_iterator<char>(f1.rdbuf()),
                      std::istreambuf_iterator<char>(),
                      std::istreambuf_iterator<char>(f2.rdbuf()));
}

int main(int argc, char **argv)
{
    if (argc < 4)
    {
        fprintf(stderr, "Usage: %s <input> <output> <output2>\n", argv[0]);
        return 1;
    }

    const auto input = argv[1];
    const auto output = argv[2];
    const auto output2 = argv[3];

    gtfsort_init_logger("info");

    SortAnnotationRetWrapper ret;

    std::cout << "Sorting annotations from " << input << " to " << output << std::endl;

    PANIC_IF(!gtfsort_sort_annotations(input, output, 4, ret.ret), "Failed to sort annotations");
    PANIC_IF(ret.ret->tag != SortAnnotationsRet::Tag::Ok, "Failed to sort annotations, but somehow no error was returned");

    std::cout << "File process result: " << ret << std::endl
              << ret << std::endl;

    const auto &result = *ret.ret->ok._0;
    PANIC_IF(result.threads != 4, "Expected 4 threads");

    PANIC_IF(!result.input_mmaped, "Expected input to be mmaped");
    PANIC_IF(!result.output_mmaped, "Expected output to be mmaped");

    PANIC_IF(result.parsing_secs <= 0.0, "Expected parsing time to be greater than 0");
    PANIC_IF(result.indexing_secs <= 0.0, "Expected indexing time to be greater than 0");
    PANIC_IF(result.writing_secs <= 0.0, "Expected writing time to be greater than 0");

    PANIC_IF(!(result.start_mem_mb > 0.0), std::string("Expected start memory to be greater than 0, got ") + std::to_string(result.start_mem_mb));
    PANIC_IF(!(result.end_mem_mb > 0.0), std::string("Expected end memory to be greater than 0, got ") + std::to_string(result.end_mem_mb));

    std::cout << "Sorting annotations from string to " << output2 << std::endl;

    std::ifstream input_file(input);
    PANIC_IF(!input_file.good(), "Failed to open input file");
    std::ostringstream input_stream;
    input_stream << input_file.rdbuf();
    const auto input_str = input_stream.str();

    std::ofstream output_file(output2, std::ios::trunc);
    PANIC_IF(!output_file.good(), "Failed to open output file");

    PANIC_IF(!gtfsort_sort_annotations_gtf_str(GTFSORT_PARSE_MODE_GFF3, input_str.c_str(), [](void *data, const char *buf, size_t len)
                                               { static_cast<std::ofstream*>(data)->write(buf, len); return (const char*)nullptr ; }, 3, static_cast<void *>(&output_file), ret.ret),
             "Failed to sort annotations");
    PANIC_IF(ret.ret->tag != SortAnnotationsRet::Tag::Ok, "Failed to sort annotations, but somehow no error was returned");

    std::cout << "String process result: " << ret << std::endl
              << ret << std::endl;

    const auto &result2 = *ret.ret->ok._0;
    PANIC_IF(result2.threads != 3, "Expected 3 threads");

    PANIC_IF(result2.input_mmaped, "Expected input to be not mmaped");
    PANIC_IF(result2.output_mmaped, "Expected output to be not mmaped");

    PANIC_IF(result2.parsing_secs <= 0.0, "Expected parsing time to be greater than 0");
    PANIC_IF(result2.indexing_secs <= 0.0, "Expected indexing time to be greater than 0");
    PANIC_IF(result2.writing_secs <= 0.0, "Expected writing time to be greater than 0");

    PANIC_IF(!(result2.start_mem_mb > 0.0), std::string("Expected start memory to be greater than 0, got ") + std::to_string(result2.start_mem_mb));
    PANIC_IF(!(result2.end_mem_mb > 0.0), std::string("Expected end memory to be greater than 0, got ") + std::to_string(result2.end_mem_mb));

    input_file.close();
    PANIC_IF(input_file.fail(), "Failed to close input file");
    output_file.close();
    PANIC_IF(output_file.fail(), "Failed to close output file");

    PANIC_IF(!cmp_files(output, output2), "Files are not the same");

    return 0;
}