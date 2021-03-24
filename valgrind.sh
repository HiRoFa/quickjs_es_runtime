cargo clean
cargo test
#find ./target/debug/deps/quickjs_runtime-* -maxdepth 1 -type f -executable | xargs valgrind --leak-check=full --error-exitcode=1
test_exe=$(find ./target/debug/deps/quickjs_runtime-* -maxdepth 1 -type f -executable)
echo "exe = ${test_exe}"
test_output=$($test_exe)
#echo "test_output = ${test_output}"
echo "${test_output}" | while read test_line; do
  if [[ $test_line == test* ]] && [[ $test_line == *ok ]]  ;
  then
      test_line=${test_line#test }
      test_line=${test_line% ... ok}
      echo "testing: ${test_line}"
      valgrind --leak-check=full --error-exitcode=1 $test_exe $test_line
      echo "done: ${test_line}"
  fi
done