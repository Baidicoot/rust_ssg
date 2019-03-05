extern crate rust_ssg;
extern crate geml;

use std::path::{PathBuf};

fn main() {
    rust_ssg::run(PathBuf::from("main.site")).unwrap();
}
