use std::fs;
use std::io::{self, Write};
use std::path::Path;

pub(super) fn write(path: &Path, code: &[u8]) -> io::Result<()> {
    let permissions = match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() || !meta.is_file() => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "output must be a regular file, not a symlink or directory",
            ));
        }
        Ok(meta) => Some(meta.permissions()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => return Err(err),
    };
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut staged = tempfile::Builder::new()
        .prefix(".rqb-")
        .tempfile_in(parent)?;
    staged.write_all(code)?;
    if let Some(permissions) = permissions {
        staged.as_file().set_permissions(permissions)?;
    }
    // Same-directory replacement preserves the old file on write failure.
    // This is not a file/directory fsync or a crash-durability guarantee.
    staged.persist(path).map_err(|err| err.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_and_replaces_complete_output() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("schema.rs");
        write(&path, b"old").unwrap();
        write(&path, b"new complete content").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new complete content");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
        assert!(write(dir.path(), b"wrong target").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn preserves_mode_and_rejects_symlinks() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("schema.rs");
        fs::write(&path, b"old").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        write(&path, b"new").unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o640
        );
        let link = dir.path().join("link.rs");
        symlink(&path, &link).unwrap();
        assert!(write(&link, b"wrong target").is_err());
        assert_eq!(fs::read(&path).unwrap(), b"new");
        let dangling = dir.path().join("dangling.rs");
        symlink(dir.path().join("missing.rs"), &dangling).unwrap();
        assert!(write(&dangling, b"wrong target").is_err());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 3);
    }

    #[cfg(unix)]
    #[test]
    fn partial_write_failure_preserves_old_file() {
        const CHILD_PATH: &str = "RQB_TEST_OUTPUT_FAILURE_PATH";
        const CHILD_VERIFIED: i32 = 42;
        if let Some(path) = std::env::var_os(CHILD_PATH) {
            // Only this dedicated child changes its signal and resource limits.
            unsafe {
                assert_ne!(libc::signal(libc::SIGXFSZ, libc::SIG_IGN), libc::SIG_ERR);
                let limit = libc::rlimit {
                    rlim_cur: 5,
                    rlim_max: 5,
                };
                assert_eq!(libc::setrlimit(libc::RLIMIT_FSIZE, &limit), 0);
            }
            let error = write(Path::new(&path), b"replacement longer than five bytes").unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::FileTooLarge, "{error:?}");
            // A stale --exact selector must not pass by running zero tests.
            std::process::exit(CHILD_VERIFIED);
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("schema.rs");
        fs::write(&path, b"original valid generated schema").unwrap();
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "output::tests::partial_write_failure_preserves_old_file",
            ])
            .env(CHILD_PATH, &path)
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(CHILD_VERIFIED), "{result:?}");
        assert_eq!(fs::read(&path).unwrap(), b"original valid generated schema");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
