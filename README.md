# Unity Unpacker Lib
A library that allows the user to extract unitypackage files. Unity packages are essentially gzip/tar files. This library should reduce boilerplate code to unpack unity packages.

# Unit tests
The unit tests cannot be run in parallel, so run tests with test-threads=1 argument:
```
cargo test -- --test-threads=1
```
