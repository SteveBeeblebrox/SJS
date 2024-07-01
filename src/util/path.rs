// ToAbsolutePath from https://internals.rust-lang.org/t/path-to-lexical-absolute/14940
// Based off of work by chrisd 2021
use std::path::{Path, PathBuf, Component};
use std::ffi::OsStr;
use std::ops::Deref;

mod sealed {
    pub trait Sealed {}
}

impl<T: Deref<Target=Path>> sealed::Sealed for T {}

pub trait ToAbsolutePath : sealed::Sealed {
    fn absolute(&self) -> std::io::Result<PathBuf>;
}

impl<T: Deref<Target=Path>> ToAbsolutePath for T {
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

pub trait GetSubExtension : sealed::Sealed {
    fn sub_extension(&self) -> Option<&OsStr>;
}

impl<T: Deref<Target=Path>> GetSubExtension for T {
    fn sub_extension(&self) -> Option<&OsStr> {
        return Path::new(self.file_stem()?).extension();
    }
}