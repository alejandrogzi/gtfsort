library(R.utils)
library(digest)

TEST_URL <- "https://ftp.ebi.ac.uk/pub/databases/gencode/Gencode_mouse/release_M35/gencode.vM35.chr_patch_hapl_scaff.basic.annotation.gff3.gz"

test_that("sorting file works", {
  tmp_gzipped <- tempfile(fileext = ".gff3.gz")
  tmp_out <- tempfile(fileext = ".gff3")
  download.file(TEST_URL, tmp_gzipped)
  tmp_unzipped <- tempfile(fileext = ".gff3")
  gunzip(tmp_gzipped, tmp_unzipped)

  res <- sort_annotations(tmp_unzipped, tmp_out, 8)
  expect_true(res$success)

  expect_true(res$input_mmaped)
  expect_true(res$output_mmaped)
  expect_true(res$parsing_secs > 0)
  expect_true(res$indexing_secs > 0)
  expect_true(res$writing_secs > 0)
  expect_true(res$start_mem_mb > 0)
  expect_true(res$end_mem_mb > 0)

  expect_true(file.exists(tmp_out))

  file.remove(tmp_out)
  file.remove(tmp_unzipped)
})
