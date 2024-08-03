#pragma once

#include <cstdarg>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>


static const int32_t GTFSORT_ERROR_INVALID_INPUT = 1;

static const int32_t GTFSORT_ERROR_INVALID_OUTPUT = 2;

static const int32_t GTFSORT_ERROR_INVALID_PARAMETER = -1;

static const int32_t GTFSORT_ERROR_INVALID_THREADS = 4;

static const int32_t GTFSORT_ERROR_IO_ERROR = 5;

static const int32_t GTFSORT_ERROR_PARSE_ERROR = 3;

static const uint8_t GTFSORT_PARSE_MODE_GFF = 2;

static const uint8_t GTFSORT_PARSE_MODE_GFF3 = 2;

static const uint8_t GTFSORT_PARSE_MODE_GTF = 1;

struct SortAnnotationsJobResultFFI {
  const char *input;
  const char *output;
  size_t threads;
  bool input_mmaped;
  bool output_mmaped;
  double parsing_secs;
  double indexing_secs;
  double writing_secs;
  double start_mem_mb;
  double end_mem_mb;

  SortAnnotationsJobResultFFI(const char *const& input,
                              const char *const& output,
                              size_t const& threads,
                              bool const& input_mmaped,
                              bool const& output_mmaped,
                              double const& parsing_secs,
                              double const& indexing_secs,
                              double const& writing_secs,
                              double const& start_mem_mb,
                              double const& end_mem_mb)
    : input(input),
      output(output),
      threads(threads),
      input_mmaped(input_mmaped),
      output_mmaped(output_mmaped),
      parsing_secs(parsing_secs),
      indexing_secs(indexing_secs),
      writing_secs(writing_secs),
      start_mem_mb(start_mem_mb),
      end_mem_mb(end_mem_mb)
  {}

};

struct GtfSortErrorFFI {
  int32_t code;
  const char *message;

  GtfSortErrorFFI(int32_t const& code,
                  const char *const& message)
    : code(code),
      message(message)
  {}

};

struct SortAnnotationsRet {
  enum class Tag {
    Ok,
    Err,
  };

  struct Ok_Body {
    SortAnnotationsJobResultFFI *_0;

    Ok_Body(SortAnnotationsJobResultFFI *const& _0)
      : _0(_0)
    {}

  };

  struct Err_Body {
    GtfSortErrorFFI *_0;

    Err_Body(GtfSortErrorFFI *const& _0)
      : _0(_0)
    {}

  };

  Tag tag;
  union {
    Ok_Body ok;
    Err_Body err;
  };
};


extern "C" {

void gtfsort_free_sort_annotations_ret(SortAnnotationsRet ret);

void gtfsort_init_logger(const char *level);

SortAnnotationsRet *gtfsort_new_sort_annotations_ret();

bool gtfsort_sort_annotations(const char *input,
                              const char *output,
                              size_t threads,
                              SortAnnotationsRet *result_ptr);

bool gtfsort_sort_annotations_gtf_str(uint8_t mode,
                                      const char *input,
                                      const char *(*output)(void*, const char*, unsigned long),
                                      size_t threads,
                                      void *caller_data,
                                      SortAnnotationsRet *result_ptr);

} // extern "C"
