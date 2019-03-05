#![feature(duration_as_u128)]

#[macro_use] extern crate lazy_static;
extern crate regex;
extern crate geml;

use std::fs;
use std::process;
use std::path::{Path, PathBuf};
use geml::{Geml, GemlFile, Result as GemlResult, GemlError::*};
use regex::{RegexBuilder, Captures, Regex};
use std::time::Instant;

fn reg(s: &str) -> regex::Regex {
    RegexBuilder::new(s)
        .multi_line(true)
        .build()
        .unwrap()
}

fn get_or_pathbuf(cap: &Captures, index: usize) -> PathBuf {
    match cap.get(index) {
        Some(x) => PathBuf::from(x.as_str()),
        None => PathBuf::new(),
    }
}

fn get_and_concat(cap: &Captures, index: usize, suffix: &PathBuf) -> PathBuf {
    let mut mutated_path = suffix.to_owned();
    mutated_path.push(get_or_pathbuf(cap, index));
    mutated_path
}

pub fn run_prg(prg: String, input_root: PathBuf, output_root: PathBuf) -> GemlResult<()> {
    lazy_static!{
        static ref COMPILE_WITH: Regex = reg(r"\s*compile (\S+) with (\S+) to (\S+)\s*");

        static ref COMPILE_DIR: Regex = reg(r"\s*compile dir (\S+) with (\S+) to (\S+)\s*");

        static ref COPY_TO: Regex = reg(r"\s*copy (\S+) to (\S+)\s*");

        static ref COPY_DIR: Regex = reg(r"\s*copy dir (\S+) to (\S+)\s*");
    }

    let program: Vec<String> = prg.lines()
        .map(|x| {
            x.trim().to_string()
        })
        .collect();

    for ins in program {
        match COMPILE_WITH.captures(&ins) {
            Some(x) => {
                compile_single_file(get_and_concat(&x, 1, &input_root), get_and_concat(&x, 2, &input_root), get_and_concat(&x, 3, &output_root))?;
            },
            None => {},
        }

        match COMPILE_DIR.captures(&ins) {
            Some(x) => {
                compile_dir(get_and_concat(&x, 1, &input_root), get_and_concat(&x, 2, &input_root), get_and_concat(&x, 3, &output_root))?;
            },
            None => {},
        }

         match COPY_DIR.captures(&ins) {
            Some(x) => {
                copy_dir(get_and_concat(&x, 1, &input_root), get_and_concat(&x, 2, &output_root))?;
            },
            None => {},
        }

        match COPY_TO.captures(&ins) {
            Some(x) => {
                let copy_start = Instant::now();

                let from_path = get_and_concat(&x, 1, &input_root);
                let to_path = get_and_concat(&x, 2, &output_root);

                fs::copy(&from_path, &to_path)?;

                println!("Copied '{}' to '{}' in {}ms.", from_path.to_str().unwrap(), to_path.to_str().unwrap(), copy_start.elapsed().as_millis());
            }
            None => {},
        }
    }

    Ok(())
}

pub fn run(prg_path: PathBuf) -> GemlResult<()> {
    let program_start = Instant::now();

    let geml_file = GemlFile::from_path(&prg_path)?;
    let deserialized_geml = match geml_file.gemls.get(0) {
        Some(x) => x,
        None => return Err(ParseError("No valid GEML defined.")),
    };

    let input_root = PathBuf::from(deserialized_geml.tags.get("input_root").unwrap_or(&String::from("")));
    let output_root = PathBuf::from(deserialized_geml.tags.get("output_root").unwrap_or(&String::from("")));

    run_prg(deserialized_geml.value.to_owned(), input_root, output_root)?;

    println!("Ran '{}' in {}ms.", prg_path.to_str().unwrap(), program_start.elapsed().as_millis());

    Ok(())
}

pub fn compile_dir(content_path: PathBuf, template_path: PathBuf, output_path: PathBuf) -> GemlResult<()> {
    for entry in fs::read_dir(&content_path)? {
        let entry_path = entry?.path();
        if entry_path.is_file() {
            let mut rel_output = output_path.clone();
            rel_output.push(entry_path.file_name().unwrap());
            rel_output.set_extension(&template_path.extension().unwrap());
            compile_single_file(entry_path, template_path.to_owned(), rel_output)?;
        }
    }

    Ok(())
}

pub fn copy_dir(content_path: PathBuf, output_path: PathBuf) -> GemlResult<()> {
    for entry in fs::read_dir(&content_path)? {
        let entry_path = entry?.path();
        if entry_path.is_file() {
            let mut rel_output = output_path.clone();
            rel_output.push(entry_path.file_name().unwrap());
            copy_file(entry_path.to_owned(), rel_output)?;
        }
    }

    Ok(())
}

pub fn copy_file(from_path: PathBuf, to_path: PathBuf) -> GemlResult<()> {
    let copy_start = Instant::now();

    fs::copy(&from_path, &to_path)?;

    println!("Copied '{}' to '{}' in {}ms.", from_path.to_str().unwrap(), to_path.to_str().unwrap(), copy_start.elapsed().as_millis());

    Ok(())
}

pub fn compile_single_file(content_path: PathBuf, template_path: PathBuf, output_path: PathBuf) -> GemlResult<()> {
    let compilation_start = Instant::now();

    let mut working_file: String = String::from_utf8_lossy(&fs::read(&template_path)?).to_string();

    let deserialized_geml = GemlFile::from_path(&content_path)?.parse()?;

    for single_geml in deserialized_geml.gemls.iter() {
        let replacer = reg(&format!(r"\${}\$", single_geml.key));

        working_file = replacer.replace_all(&working_file, &|c: &Captures| {
            &single_geml.value
        }).to_string();
    }

    fs::write(&output_path, &working_file)?;

    println!("Compiled '{}' to '{}' in {}ms.", content_path.to_str().unwrap(), output_path.to_str().unwrap(), compilation_start.elapsed().as_millis());
    Ok(())
}