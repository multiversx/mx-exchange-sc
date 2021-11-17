for f in $(find . -name "*.wasm"); do
    ./wasm2wat $f -o tmp.wat
    ./wat2wasm tmp.wat -o $f
done
