#!/usr/bin/env bash
set -euo pipefail

# Test functionality of cmprss by comparing it with the official tools

CACHE_DIR="${PRJ_ROOT}/.cache"

# Create a tmp directory and cd into it
tmpdir() {
  mkdir -p "$CACHE_DIR"
  local dir
  dir=$(mktemp --directory --tmpdir="$CACHE_DIR")
  cd "$dir"
}

# Compare the two files/directories and exit with an error if they are different
compare() {
  local file1="$1"
  local file2="$2"
  if ! diff -qr "$file1" "$file2" >/dev/null; then
    echo "Diff Detected: $file1 $file2"
    exit 1
  fi
}

# Compare the two archive sizes to check if they are similar
# TODO: This doesn't work anywhere in the file because we're compressing random data
# So the algos will always fail to compress things very well
compare_size() {
  local file1="$1"
  local file2="$2"
  # Use $3 as the max difference or 100 bytes
  local max_diff=${3:-100}
  local size1
  local size2
  size1=$(stat -c %s "$file1")
  size2=$(stat -c %s "$file2")
  if [ $((size1 - size2)) -gt "$max_diff" ]; then
    echo "Size difference too large: $file1:$size1 $file2:$size2"
    exit 1
  fi
}

# Create a random file with the given size
random_file() {
  local size="$1"
  local file="$2"
  dd if=/dev/urandom of="$file" bs=1 count="$size" 2>/dev/null
}

# Create a random directory with the given size
random_dir() {
  local size="$1"
  local dir="$2"
  mkdir -p "$dir"
  for i in $(seq 1 "$size"); do
    random_file 128 "$dir/$i"
  done
}

# Run cmprss using cargo to test the current version
cmprss() {
  cargo run --release --quiet -- "$1" --ignore-pipes "${@:2}"
}

# Test gzip using the provided compression level
test_gzip_level() {
  tmpdir
  echo "Testing gzip level $1 in $PWD"
  echo "Creating random data"
  random_file 1000000 file
  echo "Compressing with gzip and cmprss"
  gzip -"$1" -c file >gzip_file.gz
  cmprss gzip --level "$1" file cmprss_file.gz --progress=off
  # Compare the two archives
  # The archives may have slight variations (versioning or whatever) so we
  # only compare the sizes to make sure they are similar
  compare_size gzip_file.gz cmprss_file.gz
  # Decompress the 4 variations
  echo "Decompressing"
  gzip -c -d gzip_file.gz >gzip_gzip
  gzip -c -d cmprss_file.gz >cmprss_gzip
  cmprss gzip --extract cmprss_file.gz cmprss_cmprss --progress=off
  cmprss gzip --extract gzip_file.gz gzip_cmprss --progress=off
  echo "Comparing the decompressed files"
  compare file gzip_gzip
  compare file gzip_cmprss
  compare file cmprss_cmprss
  compare file cmprss_gzip
  echo "No errors detected"
}

test_gzip() {
  test_gzip_level 1
  test_gzip_level 6 # Default
  test_gzip_level 9
}

# Test xz using the provided compression level
test_xz_level() {
  tmpdir
  echo "Testing xz level $1 in $PWD"
  echo "Creating random data"
  random_file 1000000 file
  echo "Compressing with xz and cmprss"
  xz -"$1" --stdout file >xz_file.xz
  cmprss xz --level "$1" file cmprss_file.xz --progress=off
  # Compare the two archives
  # The archives may have slight variations (versioning or whatever) so we
  # only compare the sizes to make sure they are similar
  compare_size xz_file.xz cmprss_file.xz
  # Decompress the 4 variations
  echo "Decompressing"
  xz --stdout --decompress xz_file.xz >xz_xz
  xz --stdout --decompress cmprss_file.xz >xz_cmprss
  cmprss xz --extract cmprss_file.xz cmprss_cmprss --progress=off
  cmprss xz --extract xz_file.xz cmprss_xz --progress=off
  echo "Comparing the decompressed files"
  compare file xz_xz
  compare file xz_cmprss
  compare file cmprss_cmprss
  compare file cmprss_xz
  echo "No errors detected"
}

test_xz() {
  test_xz_level 0 # No compression
  test_xz_level 1
  test_xz_level 6 # Default
  test_xz_level 9
}

# Test bzip2 using the provided compression level
test_bzip2_level() {
  tmpdir
  echo "Testing bzip2 level $1 in $PWD"
  echo "Creating random data"
  random_file 1000000 file
  echo "Compressing with bzip2 and cmprss"
  bzip2 -"$1" --stdout file >bzip2_file.bz2
  cmprss bzip2 --level "$1" file cmprss_file.bz2 --progress=off
  # Compare the two archives
  # The archives may have slight variations (versioning or whatever) so we
  # only compare the sizes to make sure they are similar
  compare_size bzip2_file.bz2 cmprss_file.bz2
  # Decompress the 4 variations
  echo "Decompressing"
  bzip2 --stdout --decompress bzip2_file.bz2 >bzip2_bzip2
  bzip2 --stdout --decompress cmprss_file.bz2 >cmprss_bzip2
  cmprss bzip2 --extract cmprss_file.bz2 cmprss_cmprss --progress=off
  cmprss bzip2 --extract bzip2_file.bz2 bzip2_cmprss --progress=off
  echo "Comparing the decompressed files"
  compare file bzip2_bzip2
  compare file bzip2_cmprss
  compare file cmprss_cmprss
  compare file cmprss_bzip2
  echo "No errors detected"
}

test_bzip2() {
  test_bzip2_level 1
  test_bzip2_level 6
  test_bzip2_level 9 # Default
}

# Test zstd using the provided compression level
test_zstd_level() {
  tmpdir
  echo "Testing zstd level $1 in $PWD"
  echo "Creating random data"
  random_file 1000000 file
  echo "Compressing with zstd and cmprss"
  zstd -"$1" -c file >zstd_file.zst
  cmprss zstd --level "$1" file cmprss_file.zst --progress=off
  # Compare the two archives
  # The archives may have slight variations (versioning or whatever) so we
  # only compare the sizes to make sure they are similar
  compare_size zstd_file.zst cmprss_file.zst
  # Decompress the 4 variations
  echo "Decompressing"
  zstd -d -c zstd_file.zst >zstd_zstd
  zstd -d -c cmprss_file.zst >cmprss_zstd
  cmprss zstd --extract cmprss_file.zst cmprss_cmprss --progress=off
  cmprss zstd --extract zstd_file.zst zstd_cmprss --progress=off
  echo "Comparing the decompressed files"
  compare file zstd_zstd
  compare file zstd_cmprss
  compare file cmprss_cmprss
  compare file cmprss_zstd
  echo "No errors detected"
}

test_zstd() {
  test_zstd_level 1 # Fast compression
  test_zstd_level 3
  test_zstd_level 6 # Default
  test_zstd_level 9 # High compression
}

# Test lz4 compression
test_lz4() {
  tmpdir
  echo "Testing lz4 in $PWD"
  echo "Creating random data"
  random_file 1000000 file
  echo "Compressing with lz4 and cmprss"
  lz4 -c file >lz4_file.lz4
  cmprss lz4 file cmprss_file.lz4 --progress=off
  # Compare the two archives
  # The archives may have slight variations (versioning or whatever) so we
  # only compare the sizes to make sure they are similar
  compare_size lz4_file.lz4 cmprss_file.lz4
  # Decompress the 4 variations
  echo "Decompressing"
  lz4 -d -c lz4_file.lz4 >lz4_lz4
  lz4 -d -c cmprss_file.lz4 >cmprss_lz4
  cmprss lz4 --extract cmprss_file.lz4 cmprss_cmprss --progress=off
  cmprss lz4 --extract lz4_file.lz4 lz4_cmprss --progress=off
  echo "Comparing the decompressed files"
  compare file lz4_lz4
  compare file lz4_cmprss
  compare file cmprss_cmprss
  compare file cmprss_lz4
  echo "No errors detected"
}

# Test lzma (legacy LZMA1) at the given level against the lzma CLI from xz-utils
test_lzma_level() {
  tmpdir
  echo "Testing lzma level $1 in $PWD"
  echo "Creating random data"
  random_file 1000000 file
  echo "Compressing with lzma and cmprss"
  lzma --stdout -"$1" file >lzma_file.lzma
  cmprss lzma --level "$1" file cmprss_file.lzma --progress=off
  compare_size lzma_file.lzma cmprss_file.lzma
  echo "Decompressing"
  lzma --stdout --decompress lzma_file.lzma >lzma_lzma
  lzma --stdout --decompress cmprss_file.lzma >cmprss_lzma
  cmprss lzma --extract cmprss_file.lzma cmprss_cmprss --progress=off
  cmprss lzma --extract lzma_file.lzma lzma_cmprss --progress=off
  echo "Comparing the decompressed files"
  compare file lzma_lzma
  compare file cmprss_lzma
  compare file cmprss_cmprss
  compare file lzma_cmprss
  echo "No errors detected"
}

test_lzma() {
  test_lzma_level 1
  test_lzma_level 6 # Default
  test_lzma_level 9
}

# Test brotli at the given quality against the brotli CLI
test_brotli_level() {
  tmpdir
  echo "Testing brotli quality $1 in $PWD"
  echo "Creating random data"
  random_file 1000000 file
  echo "Compressing with brotli and cmprss"
  brotli --quality="$1" --stdout file >brotli_file.br
  cmprss brotli --level "$1" file cmprss_file.br --progress=off
  compare_size brotli_file.br cmprss_file.br
  echo "Decompressing"
  brotli --decompress --stdout brotli_file.br >brotli_brotli
  brotli --decompress --stdout cmprss_file.br >cmprss_brotli
  cmprss brotli --extract cmprss_file.br cmprss_cmprss --progress=off
  cmprss brotli --extract brotli_file.br brotli_cmprss --progress=off
  echo "Comparing the decompressed files"
  compare file brotli_brotli
  compare file cmprss_brotli
  compare file cmprss_cmprss
  compare file brotli_cmprss
  echo "No errors detected"
}

test_brotli() {
  test_brotli_level 1
  test_brotli_level 6 # Default
  test_brotli_level 11
}

# Test tar archive interop with the tar CLI. Tar has no progress bar, so no
# --progress flag is passed.
test_tar() {
  tmpdir
  echo "Testing tar in $PWD"
  echo "Creating random data"
  random_dir 10 indir
  echo "Creating tar archives with each tool"
  tar -cf tar_archive.tar indir
  cmprss tar indir cmprss_archive.tar
  echo "Extracting each archive with the opposite tool"
  mkdir -p tar_from_cmprss
  tar -xf cmprss_archive.tar -C tar_from_cmprss
  mkdir -p cmprss_from_tar
  cmprss tar --extract tar_archive.tar cmprss_from_tar
  echo "Comparing the extracted contents"
  compare indir tar_from_cmprss/indir
  compare indir cmprss_from_tar/indir
  echo "No errors detected"
}

# Test zip archive interop with the zip/unzip CLIs. Zip has no progress bar,
# so no --progress flag is passed.
test_zip() {
  tmpdir
  echo "Testing zip in $PWD"
  echo "Creating random data"
  random_dir 10 indir
  echo "Creating zip archives with each tool"
  (cd "$(dirname indir)" && zip -qr zip_archive.zip "$(basename indir)")
  cmprss zip indir cmprss_archive.zip
  echo "Extracting each archive with the opposite tool"
  mkdir -p zip_from_cmprss
  (cd zip_from_cmprss && unzip -q ../cmprss_archive.zip)
  mkdir -p cmprss_from_zip
  cmprss zip --extract zip_archive.zip cmprss_from_zip
  echo "Comparing the extracted contents"
  compare indir zip_from_cmprss/indir
  compare indir cmprss_from_zip/indir
  echo "No errors detected"
}

# Shared helper for tar.<codec> pipeline interop. Takes the compound extension
# (tar.gz/tar.xz/tar.zst/tar.bz2) and the corresponding tar short flag
# (-z/-J/--zstd/-j). Verifies that cmprss produces archives the tar CLI can
# read, and vice versa. Pipelines are invoked without a subcommand so
# --progress isn't accepted; the archive is written to a file, not stdout,
# which is fine for tests.
test_tar_pipeline() {
  local ext="$1"
  local tar_flag="$2"
  tmpdir
  echo "Testing $ext pipeline in $PWD"
  echo "Creating random data"
  random_dir 10 indir
  echo "Creating $ext archives with each tool"
  tar "$tar_flag" -cf tar_archive."$ext" indir
  cmprss indir cmprss_archive."$ext"
  echo "Extracting each archive with the opposite tool"
  mkdir -p tar_from_cmprss
  tar "$tar_flag" -xf cmprss_archive."$ext" -C tar_from_cmprss
  mkdir -p cmprss_from_tar
  cmprss --extract tar_archive."$ext" cmprss_from_tar
  echo "Comparing the extracted contents"
  compare indir tar_from_cmprss/indir
  compare indir cmprss_from_tar/indir
  echo "No errors detected"
}

test_tar_gz() { test_tar_pipeline tar.gz -z; }
test_tar_xz() { test_tar_pipeline tar.xz -J; }
test_tar_bz2() { test_tar_pipeline tar.bz2 -j; }
test_tar_zst() { test_tar_pipeline tar.zst --zstd; }

# Run all the tests if no arguments are given
if [ $# -eq 0 ]; then
  set -- gzip xz bzip2 zstd lz4 lzma brotli tar zip tar_gz tar_xz tar_bz2 tar_zst
fi

# Run the tests given on the command line
for test in "$@"; do
  test_"$test"
done
