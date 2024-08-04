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

/**
 * Frees the [SortAnnotationsRet].
 *
 * # Safety
 * ret must be a valid pointer to a [SortAnnotationsRet] that is allocated by [gtfsort_new_sort_annotations_ret].
 */
void gtfsort_free_sort_annotations_ret(struct SortAnnotationsRet *ret);

/**
 * Initializes the logger with the given log level.
 * The log level must be one of the following: trace, debug, info, warn, error.
 *
 * # Safety
 * level must be a valid C string.
 */
void gtfsort_init_logger(const char *level);

/**
 * Allocates a new [SortAnnotationsRet] on the Rust heap.
 *
 * # Safety
 * The caller is responsible for freeing the allocated memory using [gtfsort_free_sort_annotations_ret].
 * Do not free the memory using any other method.
 */
struct SortAnnotationsRet *gtfsort_new_sort_annotations_ret(void);

/**
 * Sorts the annotations in the given GTF or GFF3 file and writes the result to the output file.
 *
 * `result_ptr` is a pointer to a [SortAnnotationsRet] that will be set to the result of the operation.
 * if you don't need the result, you can pass a null pointer.
 *
 * The return value is true if the operation was successful, false otherwise.
 *
 * # Safety
 * input and output must be valid C strings that point to valid file paths.
 */
bool gtfsort_sort_annotations(const char *input,
                              const char *output,
                              size_t threads,
                              struct SortAnnotationsRet *result_ptr);

/**
 * Sorts the annotations in the given GTF or GFF3 string and writes the result chunk by chunk to the output callback.
 *
 * The mode must be one of the following:
 * - [GTFSORT_PARSE_MODE_GTF]
 * - [GTFSORT_PARSE_MODE_GFF3]
 * - [GTFSORT_PARSE_MODE_GFF]
 *
 * output is a callback function that will be called with the following arguments:
 * - caller_data: a pointer to the caller data
 * - output: a pointer to the output bytes
 * - len: the length of the output bytes
 *
 * The callback function should return a null pointer in case of success, or an error message in case of failure.
 *
 * caller_data is a pointer to the caller data that will be passed to the output callback.
 *
 * result_ptr is a pointer to a SortAnnotationsRet that will be set to the result of the operation.
 * if you don't need the result, you can pass a null pointer.
 *
 * the return value is true if the operation was successful, false otherwise.
 *
 * # Safety
 *
 * input must be a valid C string.
 *
 * The caller is responsible for freeing the error message in output callback.
 *
 */
bool gtfsort_sort_annotations_gtf_str(uint8_t mode,
                                      const char *input,
                                      const char *(*output)(void*, const char*, unsigned long),
                                      size_t threads,
                                      void *caller_data,
                                      struct SortAnnotationsRet *result_ptr);
