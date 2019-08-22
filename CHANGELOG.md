### 0.4.5

Released 2019/08/22.

* Drop `unreachable` dependency, now that `core::hints::unreachable_unchecked`
  is stable in Rust 1.27.

### 0.4.4

Released 2019/04/15.

* Add support for running on stable Rust 1.33 and newer.

### 0.4.3

Released 2019/02/18.

* Add support for building on stable Rust on Windows and Unix.
* `wasm32` intrinsics are now invoked using `core::arch` rather than LLVM.
* Use `SRWLOCK` for windows implementation.

### 0.4.2

Released 2018/07/16.

* Updated again for changes to Rust's standard allocator API.

### 0.4.1

Released 2018/06/15.

* Updated for changes to Rust's standard allocator API.

### 0.4.0

Released 2018/05/01.

* Added support for allocating out of a static array heap. This enables using
  `wee_alloc` in embdedded and bare-metal environments.

* Added @ZackPierce to the `wee_alloc` team \o/

### 0.3.0

Released 2018/04/24.

* Almost 10x faster replaying Real World(tm) allocation traces.

* Updated for the latest allocator trait changes from the Rust standard library.

### 0.2.0

Released 2018/03/06.

* Added support for allocations with arbitrary alignments.

* Updated to work with rustc's LLVM 6 upgrade and the change of intrinsic link
  names.

* Added windows support.

* Added @pepyakin and @DrGoldfire to the `wee_alloc` team \o/

### 0.1.0

Released 2018/02/02.

* Initial release!
