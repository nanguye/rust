// aux-build:empty-plugin.rs

#![plugin(empty_plugin)]
//~^ ERROR The specified module could not be found. (os error 126)

fn main() {}
