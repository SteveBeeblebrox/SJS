// From deno:cli/cache/mod.rs
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
use deno_cache_dir::DenoCacheEnv;

use std::time::SystemTime;
use std::path::Path;

/// Permissions used to save a file in the disk caches.
pub const CACHE_PERM: u32 = 0o644;

#[derive(Debug, Clone)]
pub struct SJSCacheEnv;

impl DenoCacheEnv for SJSCacheEnv {
  fn read_file_bytes(&self, path: &Path) -> std::io::Result<Option<Vec<u8>>> {
    match std::fs::read(path) {
      Ok(s) => Ok(Some(s)),
      Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
      Err(err) => Err(err),
    }
  }

  fn atomic_write_file(
    &self,
    path: &Path,
    bytes: &[u8],
  ) -> std::io::Result<()> {
    atomic_write_file(path, bytes, CACHE_PERM)
  }

  fn modified(&self, path: &Path) -> std::io::Result<Option<SystemTime>> {
    match std::fs::metadata(path) {
      Ok(metadata) => Ok(Some(
        metadata.modified().unwrap_or_else(|_| SystemTime::now()),
      )),
      Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
      Err(err) => Err(err),
    }
  }

  fn is_file(&self, path: &Path) -> bool {
    path.is_file()
  }

  fn time_now(&self) -> SystemTime {
    SystemTime::now()
  }
}


// From deno:cli/util/fs.rs
use std::fmt::Write as _;
use std::io::Write as _;
use std::io::Error;
use std::io::ErrorKind;
use std::fs::OpenOptions;

/// Writes the file to the file system at a temporary path, then
/// renames it to the destination in a single sys call in order
/// to never leave the file system in a corrupted state.
///
/// This also handles creating the directory if a NotFound error
/// occurs.
pub fn atomic_write_file<T: AsRef<[u8]>>(
    file_path: &Path,
    data: T,
    mode: u32,
  ) -> std::io::Result<()> {
    fn atomic_write_file_raw(
      temp_file_path: &Path,
      file_path: &Path,
      data: &[u8],
      mode: u32,
    ) -> std::io::Result<()> {
      write_file(temp_file_path, data, mode)?;
      std::fs::rename(temp_file_path, file_path)?;
      Ok(())
    }
  
    fn inner(file_path: &Path, data: &[u8], mode: u32) -> std::io::Result<()> {
      let temp_file_path = {
        let rand: String = (0..4).fold(String::new(), |mut output, _| {
          let _ = write!(output, "{:02x}", rand::random::<u8>());
          output
        });
        let extension = format!("{rand}.tmp");
        file_path.with_extension(extension)
      };
  
      if let Err(write_err) =
        atomic_write_file_raw(&temp_file_path, file_path, data, mode)
      {
        if write_err.kind() == ErrorKind::NotFound {
          let parent_dir_path = file_path.parent().unwrap();
          match std::fs::create_dir_all(parent_dir_path) {
            Ok(()) => {
              return atomic_write_file_raw(
                &temp_file_path,
                file_path,
                data,
                mode,
              )
              .map_err(|err| add_file_context_to_err(file_path, err));
            }
            Err(create_err) => {
              if !parent_dir_path.exists() {
                return Err(Error::new(
                  create_err.kind(),
                  format!(
                    "{:#} (for '{}')\nCheck the permission of the directory.",
                    create_err,
                    parent_dir_path.display()
                  ),
                ));
              }
            }
          }
        }
        return Err(add_file_context_to_err(file_path, write_err));
      }
      Ok(())
    }
  
    inner(file_path, data.as_ref(), mode)
}

fn add_file_context_to_err(file_path: &Path, err: Error) -> Error {
    Error::new(
      err.kind(),
      format!("{:#} (for '{}')", err, file_path.display()),
    )
}

pub fn write_file<T: AsRef<[u8]>>(
    filename: &Path,
    data: T,
    mode: u32,
  ) -> std::io::Result<()> {
    write_file_2(filename, data, true, mode, true, false)
  }
  
  pub fn write_file_2<T: AsRef<[u8]>>(
    filename: &Path,
    data: T,
    update_mode: bool,
    mode: u32,
    is_create: bool,
    is_append: bool,
  ) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
      .read(false)
      .write(true)
      .append(is_append)
      .truncate(!is_append)
      .create(is_create)
      .open(filename)?;
  
    if update_mode {
      #[cfg(unix)]
      {
        use std::os::unix::fs::PermissionsExt;
        let mode = mode & 0o777;
        let permissions = PermissionsExt::from_mode(mode);
        file.set_permissions(permissions)?;
      }
      #[cfg(not(unix))]
      let _ = mode;
    }
  
    file.write_all(data.as_ref())
  }