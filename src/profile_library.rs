use crate::{Profile, ProfileError};
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredProfile {
    pub path: PathBuf,
    pub profile: Profile,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileLibrary {
    pub directory: PathBuf,
    pub profiles: Vec<StoredProfile>,
    pub skipped: Vec<String>,
}

pub fn profile_library_directory() -> io::Result<PathBuf> {
    profile_directory_from(
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
    )
}

pub fn profile_library() -> io::Result<ProfileLibrary> {
    scan_library_at(&profile_library_directory()?)
}

pub fn library_profile(path: &Path) -> Result<StoredProfile, ProfileError> {
    load_library_profile_at(&profile_library_directory()?, path)
}

pub fn rename_library_profile(path: &Path, new_name: &str) -> Result<StoredProfile, ProfileError> {
    rename_library_profile_at(&profile_library_directory()?, path, new_name)
}

fn profile_directory_from(
    xdg_config: Option<OsString>,
    home: Option<OsString>,
) -> io::Result<PathBuf> {
    let base = xdg_config
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            home.map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .map(|path| path.join(".config"))
        })
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "cannot locate the profile library; XDG_CONFIG_HOME and HOME are unavailable",
            )
        })?;
    Ok(base.join("ae5-control").join("profiles"))
}

fn scan_library_at(directory: &Path) -> io::Result<ProfileLibrary> {
    fs::create_dir_all(directory)?;
    let mut profiles = Vec::new();
    let mut skipped = Vec::new();

    for result in fs::read_dir(directory)? {
        let entry = match result {
            Ok(entry) => entry,
            Err(error) => {
                skipped.push(error.to_string());
                continue;
            }
        };
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) => {
                skipped.push(format!("{}: {error}", path.display()));
                continue;
            }
        };
        let is_json = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
        if !file_type.is_file() || !is_json {
            continue;
        }
        match Profile::load(&path) {
            Ok(profile) => profiles.push(StoredProfile { path, profile }),
            Err(error) => skipped.push(format!("{}: {error}", path.display())),
        }
    }

    profiles.sort_by_cached_key(|entry| entry.profile.name.to_lowercase());
    skipped.sort();
    Ok(ProfileLibrary {
        directory: directory.to_owned(),
        profiles,
        skipped,
    })
}

fn load_library_profile_at(directory: &Path, path: &Path) -> Result<StoredProfile, ProfileError> {
    let path = direct_regular_profile_path(directory, path)?;
    let profile = Profile::load(&path)?;
    Ok(StoredProfile { path, profile })
}

fn rename_library_profile_at(
    directory: &Path,
    path: &Path,
    new_name: &str,
) -> Result<StoredProfile, ProfileError> {
    let mut stored = load_library_profile_at(directory, path)?;
    stored.profile.name = new_name.trim().to_owned();
    stored.profile.save_replace(&stored.path)?;
    Ok(stored)
}

fn direct_regular_profile_path(directory: &Path, path: &Path) -> io::Result<PathBuf> {
    fs::create_dir_all(directory)?;
    let directory = fs::canonicalize(directory)?;
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "profile is not a regular file",
        ));
    }
    let path = fs::canonicalize(path)?;
    let is_json = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
    if path.parent() != Some(directory.as_path()) || !is_json {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "profile is not a JSON file directly inside the profile library",
        ));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProfileControl;
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    fn sample_profile(name: &str) -> Profile {
        Profile {
            format_version: 1,
            name: name.to_owned(),
            target: "1102:0012/1102:0051".to_owned(),
            controls: BTreeMap::from([(
                "Output Select".to_owned(),
                ProfileControl {
                    choice: Some("Headphone".to_owned()),
                    ..ProfileControl::default()
                },
            )]),
        }
    }

    #[test]
    fn follows_xdg_config_and_falls_back_to_home() {
        assert_eq!(
            profile_directory_from(Some("/tmp/xdg".into()), Some("/home/test".into())).unwrap(),
            Path::new("/tmp/xdg/ae5-control/profiles")
        );
        assert_eq!(
            profile_directory_from(Some("relative".into()), Some("/home/test".into())).unwrap(),
            Path::new("/home/test/.config/ae5-control/profiles")
        );
        assert!(profile_directory_from(None, None).is_err());
    }

    #[test]
    fn lists_valid_profiles_and_reports_invalid_json() {
        let directory = test_directory();
        let alpha = sample_profile("Alpha");
        let zulu = sample_profile("Zulu");
        zulu.save_new(&directory.join("first.json")).unwrap();
        alpha.save_new(&directory.join("second.JSON")).unwrap();
        fs::write(directory.join("broken.json"), b"{").unwrap();
        fs::write(directory.join("notes.txt"), b"ignored").unwrap();

        let library = scan_library_at(&directory).unwrap();
        assert_eq!(library.directory, directory);
        assert_eq!(
            library
                .profiles
                .iter()
                .map(|entry| entry.profile.name.as_str())
                .collect::<Vec<_>>(),
            ["Alpha", "Zulu"]
        );
        assert_eq!(library.skipped.len(), 1);
        assert!(library.skipped[0].contains("broken.json"));

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn renames_only_regular_profiles_inside_the_library() {
        let directory = test_directory();
        let path = directory.join("headphones.json");
        sample_profile("Headphones").save_new(&path).unwrap();

        let renamed = rename_library_profile_at(&directory, &path, "  Late night  ").unwrap();
        assert_eq!(renamed.profile.name, "Late night");
        assert_eq!(Profile::load(&path).unwrap().name, "Late night");

        let outside_directory = test_directory();
        let outside = outside_directory.join("outside.json");
        sample_profile("Outside").save_new(&outside).unwrap();
        assert!(rename_library_profile_at(&directory, &outside, "No").is_err());
        assert_eq!(Profile::load(&outside).unwrap().name, "Outside");

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            let linked = directory.join("linked.json");
            symlink(&outside, &linked).unwrap();
            assert!(rename_library_profile_at(&directory, &linked, "No").is_err());
            assert_eq!(Profile::load(&outside).unwrap().name, "Outside");
        }

        assert!(rename_library_profile_at(&directory, &path, "   ").is_err());
        assert_eq!(Profile::load(&path).unwrap().name, "Late night");

        fs::remove_dir_all(directory).unwrap();
        fs::remove_dir_all(outside_directory).unwrap();
    }

    fn test_directory() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "ae5-profile-library-test-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        path
    }
}
