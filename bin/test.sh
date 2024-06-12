#!/usr/bin/env bash
set -euo pipefail

# Test functionality of cmprss by comparing it with the official tools

CACHE_DIR="${PRJ_ROOT}/.cache"

# Create a tmp directory and cd into it
tmpdir() {
  mkdir -p "$CACHE_DIR"
  local dir=$(mktemp --directory --tmpdir="$CACHE_DIR")
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
  local size1=$(stat -c %s "$file1")
  local size2=$(stat -c %s "$file2")
  if [ $((size1 - size2)) -gt $max_diff ]; then
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
  gzip -$1 -c file >gzip_file.gz
  cmprss gzip --level $1 file cmprss_file.gz
  # Compare the two archives
  # The archives may have slight variations (versioning or whatever) so we
  # only compare the sizes to make sure they are similar
  compare_size gzip_file.gz cmprss_file.gz
  # Decompress the 4 variations
  echo "Decompressing"
  gzip -c -d gzip_file.gz >gzip_gzip
  gzip -c -d cmprss_file.gz >cmprss_gzip
  cmprss gzip --extract cmprss_file.gz cmprss_cmprss
  cmprss gzip --extract gzip_file.gz gzip_cmprss
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
  xz -$1 --stdout file >xz_file.xz
  cmprss xz --level $1 file cmprss_file.xz --progress=off
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
  bzip2 -$1 --stdout file >bzip2_file.bz2
  cmprss bzip2 --level $1 file cmprss_file.bz2 --progress=off
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

# Run all the tests if no arguments are given
if [ $# -eq 0 ]; then
  set -- gzip xz bzip2
fi

# Run the tests given on the command line
for test in "$@"; do
  test_"$test"
done
