// From https://internals.rust-lang.org/t/path-to-lexical-absolute/14940
// Based off of work by chrisd 2021
use std::path::{Path,PathBuf, Component};

pub trait ToAbsolutePath {
    fn absolute(&self) -> std::io::Result<PathBuf>;
}

impl ToAbsolutePath for PathBuf {
    fn absolute(&self) -> std::io::Result<PathBuf> {
        let mut absolute = if self.is_absolute() {
            PathBuf::new()
        } else {
            std::env::current_dir()?
        };
        for component in self.components() {
            match component {
                Component::CurDir => {},
                Component::ParentDir => { absolute.pop(); },
                component @ _ => absolute.push(component.as_os_str()),
            }
        }
        Ok(absolute)
    }
}

impl ToAbsolutePath for Path {
    fn absolute(&self) -> std::io::Result<PathBuf> {
        let mut absolute = if self.is_absolute() {
            PathBuf::new()
        } else {
            std::env::current_dir()?
        };
        for component in self.components() {
            match component {
                Component::CurDir => {},
                Component::ParentDir => { absolute.pop(); },
                component @ _ => absolute.push(component.as_os_str()),
            }
        }
        Ok(absolute)
    }
}