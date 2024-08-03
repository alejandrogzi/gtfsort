#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>


#define GTFSORT_ERROR_INVALID_INPUT 1

#define GTFSORT_ERROR_INVALID_OUTPUT 2

#define GTFSORT_ERROR_INVALID_PARAMETER -1

#define GTFSORT_ERROR_INVALID_THREADS 4

#define GTFSORT_ERROR_IO_ERROR 5

#define GTFSORT_ERROR_PARSE_ERROR 3

#define GTFSORT_PARSE_MODE_GFF 2

#define GTFSORT_PARSE_MODE_GFF3 2

#define GTFSORT_PARSE_MODE_GTF 1

typedef struct SortAnnotationsJobResultFFI {
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
} SortAnnotationsJobResultFFI;

typedef struct GtfSortErrorFFI {
  int32_t code;
  const char *message;
} GtfSortErrorFFI;

typedef enum SortAnnotationsRet_Tag {
  Ok,
  Err,
} SortAnnotationsRet_Tag;

typedef struct SortAnnotationsRet {
  SortAnnotationsRet_Tag tag;
  union {
    struct {
      struct SortAnnotationsJobResultFFI *ok;
    };
    struct {
      struct GtfSortErrorFFI *err;
    };
  };
} SortAnnotationsRet;

void gtfsort_free_sort_annotations_ret(struct SortAnnotationsRet ret);

void gtfsort_init_logger(const char *level);

struct SortAnnotationsRet *gtfsort_new_sort_annotations_ret(void);

bool gtfsort_sort_annotations(const char *input,
                              const char *output,
                              size_t threads,
                              struct SortAnnotationsRet *result_ptr);

bool gtfsort_sort_annotations_gtf_str(uint8_t mode,
                                      const char *input,
                                      const char *(*output)(void*, const char*, unsigned long),
                                      size_t threads,
                                      void *caller_data,
                                      struct SortAnnotationsRet *result_ptr);
