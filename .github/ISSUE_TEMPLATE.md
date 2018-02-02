<!--
  -- Thanks for filing an issue! Please fill out the appropriate template below.
  -- We appreciate it :)
  -->

<!---------------------------- BUG REPORTS ------------------------------------>

<!--
  -- When creating a bug report, make sure that you have enabled the
  -- `extra_assertions` feature, and have `RUST_BACKTRACE=1` set, as these
  -- things will give much better diagnostics about what is going wrong.
  -->

### Summary

Include a sentence or two summarizing the bug.

### Steps to Reproduce

* First clone this repository that uses `wee_alloc`: ..............
* `cd $REPO`
* `cargo run`

### Actual Results

```
Insert relevant panic messages and/or backtraces here.
```

### Expected Results

What did you expect to happen instead of what actually happened?

<!---------------------------- FEATURE REQUESTS ------------------------------->

### Summary

Include a sentence or two summary of the requested feature.

### Motivation

How does this further `wee_alloc`'s goal of being the best allocator for
`wasm32-unknown-unknown`, with a very small `.wasm` code size footprint?

### Details

* What bits of code would need to change? Which modules?

* What are the trade offs?

* Are you willing to implement this yourself? If you had a mentor? Are you
  willing to mentor someone else?
