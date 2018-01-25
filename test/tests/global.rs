// Adopted from
// https://github.com/alexcrichton/dlmalloc-rs/blob/master/tests/global.rs

#![feature(global_allocator)]

extern crate wee_alloc;

use std::collections::HashMap;
use std::thread;

#[global_allocator]
static A: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[test]
fn foo() {
    println!("hello");
}

#[test]
fn map() {
    let mut m = HashMap::new();
    m.insert(1, 2);
    m.insert(5, 3);
    drop(m);
}

#[test]
fn strings() {
    format!("foo, bar, {}", "baz");
}

#[test]
fn threads() {
    assert!(thread::spawn(|| panic!()).join().is_err());
}
