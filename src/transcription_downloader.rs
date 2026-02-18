//! Manages the download of asset files for transcription courses.
//!
//! Transcription courses include references to external assets. Manually downloading them is a
//! cumbersome process, so this module automates the process.

use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{bail, Result};
use sha1::{Digest, Sha1};
use ustr::Ustr;

use trane::{
    data::{
        course_generator::transcription::{TranscriptionLink, TranscriptionPreferences},
        ExerciseAsset, ExerciseManifest,
    },
    error::TranscriptionDownloaderError,
};

/// Extracts the transcription link from an exercise manifest.
fn extract_transcription_link(manifest: &ExerciseManifest) -> Option<TranscriptionLink> {
    match &manifest.exercise_asset {
        ExerciseAsset::TranscriptionAsset { external_link, .. } => external_link.clone(),
        _ => None,
    }
}

/// Gets the transcription link for an exercise.
fn get_transcription_link(
    exercise_id: Ustr,
    get_exercise_manifest: &(impl Fn(Ustr) -> Option<ExerciseManifest> + Send + Sync),
) -> Option<TranscriptionLink> {
    get_exercise_manifest(exercise_id).and_then(|manifest| extract_transcription_link(&manifest))
}

/// Downloads transcription assets to local storage.
pub struct LocalTranscriptionDownloader {
    /// Preferences for transcription courses.
    pub preferences: TranscriptionPreferences,
}

impl LocalTranscriptionDownloader {
    /// Gets the name of the directory where the asset should be downloaded.
    fn download_dir_name(link: &TranscriptionLink) -> String {
        let TranscriptionLink::YouTube(input) = link;
        let mut hasher = Sha1::new();
        hasher.update(input.as_bytes());
        let hash = hasher.finalize();
        hex::encode(hash)
    }

    /// Gets the name of the file to which download the asset.
    fn download_file_name(link: &TranscriptionLink) -> String {
        match link {
            TranscriptionLink::YouTube(_) => "audio.m4a".to_string(),
        }
    }

    /// Generates a download path relative to the root download directory.
    fn rel_download_path(link: &TranscriptionLink) -> PathBuf {
        Path::new(&Self::download_dir_name(link)).join(Self::download_file_name(link))
    }

    /// Gets the full path to the asset file with the download directory prepended.
    fn full_download_path(&self, link: &TranscriptionLink) -> Option<PathBuf> {
        self.preferences
            .download_path
            .as_ref()
            .map(|download_path| Path::new(download_path).join(Self::rel_download_path(link)))
    }

    /// Gets the full path to the asset file with the alias directory prepended.
    fn full_alias_path(&self, link: &TranscriptionLink) -> Option<PathBuf> {
        self.preferences
            .download_path_alias
            .as_ref()
            .map(|path_alias| Path::new(path_alias).join(Self::rel_download_path(link)))
    }

    /// Verifies that a binary is installed. The argument should be something simple, like a version
    /// flag, that will exit quickly.
    fn verify_binary(name: &str, arg: &str) -> Result<()> {
        let status = Command::new(name)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .arg(arg)
            .status();
        let Ok(status) = status else {
            bail!("command \"{name}\" cannot be found");
        };
        if !status.success() {
            bail!("command \"{name}\" failed");
        }
        Ok(())
    }

    /// Checks that the prerequisites to use the downloader are met.
    fn check_prerequisites(&self) -> Result<()> {
        // Check yt-dlp is installed.
        Self::verify_binary("yt-dlp", "--version")?;

        // Check the download path is valid.
        let Some(download_path) = self.preferences.download_path.as_ref() else {
            bail!("transcription download path is not set");
        };
        let download_path = Path::new(download_path);
        if !download_path.exists() {
            bail!("transcription download path does not exist");
        }
        Ok(())
    }

    /// Helper function to download an asset.
    fn download_asset_helper(
        &self,
        exercise_id: Ustr,
        force_download: bool,
        get_exercise_manifest: &(impl Fn(Ustr) -> Option<ExerciseManifest> + Send + Sync),
    ) -> Result<()> {
        // Check if the asset has already been downloaded.
        self.check_prerequisites()?;
        let Some(link) = get_transcription_link(exercise_id, get_exercise_manifest) else {
            return Ok(());
        };
        let download_path = self.full_download_path(&link).unwrap();
        if download_path.exists() && !force_download {
            return Ok(());
        }

        // Create a temporary directory, download the asset, and copy it to the final location.
        let temp_dir = tempfile::tempdir()?;
        match link {
            TranscriptionLink::YouTube(yt_link) => {
                let temp_file = temp_dir.path().join("audio.m4a");
                let output = Command::new("yt-dlp")
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::piped())
                    .arg("--enable-file-urls")
                    .arg("--extract-audio")
                    .arg("--audio-format")
                    .arg("m4a")
                    .arg("--output")
                    .arg(temp_file.to_str().unwrap())
                    .arg(&yt_link)
                    .output()?;
                if !output.status.success() {
                    let err = String::from_utf8_lossy(&output.stderr);
                    bail!("yt-dlp failed to download audio from URL {yt_link}: {err}");
                }
                std::fs::create_dir_all(download_path.parent().unwrap())?;
                std::fs::copy(temp_file, &download_path)?;
            }
        }
        Ok(())
    }

    /// Checks if the given asset has been downloaded.
    pub fn is_transcription_asset_downloaded(
        &self,
        exercise_id: Ustr,
        get_exercise_manifest: &(impl Fn(Ustr) -> Option<ExerciseManifest> + Send + Sync),
    ) -> bool {
        if self.preferences.download_path.is_none() {
            return false;
        }
        let Some(link) = get_transcription_link(exercise_id, get_exercise_manifest) else {
            return false;
        };
        let download_path = self.full_download_path(&link).unwrap();
        download_path.exists()
    }

    /// Downloads the given asset.
    pub fn download_transcription_asset(
        &self,
        exercise_id: Ustr,
        force_download: bool,
        get_exercise_manifest: &(impl Fn(Ustr) -> Option<ExerciseManifest> + Send + Sync),
    ) -> Result<(), TranscriptionDownloaderError> {
        self.download_asset_helper(exercise_id, force_download, get_exercise_manifest)
            .map_err(|e| TranscriptionDownloaderError::DownloadAsset(exercise_id, e))
    }

    /// Returns the download path for the given asset.
    pub fn transcription_download_path(
        &self,
        exercise_id: Ustr,
        get_exercise_manifest: &(impl Fn(Ustr) -> Option<ExerciseManifest> + Send + Sync),
    ) -> Option<PathBuf> {
        let link = get_transcription_link(exercise_id, get_exercise_manifest)?;
        self.full_download_path(&link)
    }

    /// Returns the download path alias for the given asset.
    pub fn transcription_download_path_alias(
        &self,
        exercise_id: Ustr,
        get_exercise_manifest: &(impl Fn(Ustr) -> Option<ExerciseManifest> + Send + Sync),
    ) -> Option<PathBuf> {
        let link = get_transcription_link(exercise_id, get_exercise_manifest)?;
        self.full_alias_path(&link)
    }
}

#[cfg(test)]
mod test {
    use std::path::{self, Path};
    use ustr::Ustr;

    use super::*;
    use trane::data::{
        course_generator::transcription::{TranscriptionLink, TranscriptionPreferences},
        BasicAsset, ExerciseAsset, ExerciseManifest, ExerciseType,
    };

    // Test link to a real YouTube video: Margaret Glaspy and Julian Lage perform “Best Behavior”.
    const YT_LINK: &str = "https://www.youtube.com/watch?v=p4LgzLjF4xE";

    // A local copy of the file above to avoid using the network in tests.
    const LOCAL_FILE: &str = "../trane/testdata/test_audio.m4a";

    fn build_manifest(link: Option<TranscriptionLink>) -> ExerciseManifest {
        ExerciseManifest {
            exercise_asset: ExerciseAsset::TranscriptionAsset {
                content: "content".to_string(),
                external_link: link,
            },
            id: Ustr::from("exercise_id"),
            lesson_id: Ustr::from("lesson_id"),
            course_id: Ustr::from("course_id"),
            name: "Exercise Name".to_string(),
            description: None,
            exercise_type: ExerciseType::Procedural,
        }
    }

    fn build_resolver(
        link: Option<TranscriptionLink>,
    ) -> impl Fn(Ustr) -> Option<ExerciseManifest> {
        let manifest = build_manifest(link);
        move |_id: Ustr| Some(manifest.clone())
    }

    /// Verifies extracting the link from a valid exercise manifest.
    #[test]
    fn test_extract_link() {
        // Transcription asset with no link.
        let mut manifest = build_manifest(None);
        assert!(extract_transcription_link(&manifest).is_none());

        // Transcription asset with a link.
        manifest.exercise_asset = ExerciseAsset::TranscriptionAsset {
            content: "content".to_string(),
            external_link: Some(TranscriptionLink::YouTube(YT_LINK.into())),
        };
        assert_eq!(
            TranscriptionLink::YouTube(YT_LINK.into()),
            extract_transcription_link(&manifest).unwrap()
        );

        // Other type of asset.
        manifest.exercise_asset = ExerciseAsset::BasicAsset(BasicAsset::InlinedAsset {
            content: "content".to_string(),
        });
        assert!(extract_transcription_link(&manifest).is_none());
    }

    /// Verifies that exercises with no links are marked as not downloaded.
    #[test]
    fn test_is_downloaded_no_link() {
        let downloader = LocalTranscriptionDownloader {
            preferences: TranscriptionPreferences::default(),
        };
        let resolver = build_resolver(None);
        assert!(!downloader.is_transcription_asset_downloaded(Ustr::from("exercise"), &resolver));
    }

    /// Verifies that exercises that have not been downloaded are marked as such.
    #[test]
    fn test_is_downloaded_no_download() {
        let downloader = LocalTranscriptionDownloader {
            preferences: TranscriptionPreferences::default(),
        };
        let resolver = build_resolver(Some(TranscriptionLink::YouTube(YT_LINK.into())));
        assert!(!downloader.is_transcription_asset_downloaded(Ustr::from("exercise"), &resolver));
    }

    /// Verifies that downloading an asset fails if there's no download path set.
    #[test]
    fn test_download_asset_no_path_set() {
        let downloader = LocalTranscriptionDownloader {
            preferences: TranscriptionPreferences {
                instruments: vec![],
                download_path: None,
                download_path_alias: None,
            },
        };
        let resolver = build_resolver(Some(TranscriptionLink::YouTube(YT_LINK.into())));
        assert!(downloader
            .download_transcription_asset(Ustr::from("exercise"), false, &resolver)
            .is_err());
    }

    /// Verifies that downloading an asset fails if the download path does not exist.
    #[test]
    fn test_download_asset_missing_dir() {
        let downloader = LocalTranscriptionDownloader {
            preferences: TranscriptionPreferences {
                instruments: vec![],
                download_path: Some("/some/missing/dir".to_string()),
                download_path_alias: None,
            },
        };
        let resolver = build_resolver(Some(TranscriptionLink::YouTube(YT_LINK.into())));
        assert!(!downloader.is_transcription_asset_downloaded(Ustr::from("exercise"), &resolver));
        assert!(downloader
            .download_transcription_asset(Ustr::from("exercise"), false, &resolver)
            .is_err());
    }

    /// Verifies downloading an exercise with no link.
    #[test]
    fn test_download_asset_no_link() {
        let temp_dir = tempfile::tempdir().unwrap();
        let downloader = LocalTranscriptionDownloader {
            preferences: TranscriptionPreferences {
                instruments: vec![],
                download_path: Some(temp_dir.path().to_str().unwrap().to_string()),
                download_path_alias: None,
            },
        };
        let resolver = build_resolver(None);
        assert!(!downloader.is_transcription_asset_downloaded(Ustr::from("exercise"), &resolver));
        downloader
            .download_transcription_asset(Ustr::from("exercise"), false, &resolver)
            .unwrap();
        assert!(!downloader.is_transcription_asset_downloaded(Ustr::from("exercise"), &resolver));
    }

    /// Verifies downloading a valid asset.
    #[test]
    fn test_download_valid_asset() {
        let temp_dir = tempfile::tempdir().unwrap();
        let local_path = path::absolute(Path::new(LOCAL_FILE)).unwrap();
        let file_link = format!("file://{}", local_path.to_str().unwrap());
        let downloader = LocalTranscriptionDownloader {
            preferences: TranscriptionPreferences {
                instruments: vec![],
                download_path: Some(temp_dir.path().to_str().unwrap().to_string()),
                download_path_alias: None,
            },
        };
        let resolver = build_resolver(Some(TranscriptionLink::YouTube(file_link)));
        assert!(!downloader.is_transcription_asset_downloaded(Ustr::from("exercise"), &resolver));
        downloader
            .download_transcription_asset(Ustr::from("exercise"), false, &resolver)
            .unwrap();
        assert!(downloader.is_transcription_asset_downloaded(Ustr::from("exercise"), &resolver));

        // The asset won't be redownloaded if it already exists.
        downloader
            .download_transcription_asset(Ustr::from("exercise"), false, &resolver)
            .unwrap();
        assert!(downloader.is_transcription_asset_downloaded(Ustr::from("exercise"), &resolver));

        // Verify re-downloading the asset as well.
        downloader
            .download_transcription_asset(Ustr::from("exercise"), true, &resolver)
            .unwrap();
        assert!(downloader.is_transcription_asset_downloaded(Ustr::from("exercise"), &resolver));
    }

    /// Verifies downloading an invalid asset.
    #[test]
    fn test_download_bad_asset() {
        let temp_dir = tempfile::tempdir().unwrap();
        let downloader = LocalTranscriptionDownloader {
            preferences: TranscriptionPreferences {
                instruments: vec![],
                download_path: Some(temp_dir.path().to_str().unwrap().to_string()),
                download_path_alias: None,
            },
        };
        let resolver = build_resolver(Some(TranscriptionLink::YouTube(
            "https://www.youtube.com/watch?v=badID".into(),
        )));
        assert!(!downloader.is_transcription_asset_downloaded(Ustr::from("exercise"), &resolver));
        assert!(downloader
            .download_transcription_asset(Ustr::from("exercise"), false, &resolver)
            .is_err());
        assert!(!downloader.is_transcription_asset_downloaded(Ustr::from("exercise"), &resolver));
    }

    /// Verifies that the download paths are correctly generated.
    #[test]
    fn test_download_paths() {
        let temp_dir = tempfile::tempdir().unwrap();
        let downloader = LocalTranscriptionDownloader {
            preferences: TranscriptionPreferences {
                instruments: vec![],
                download_path: Some(temp_dir.path().to_str().unwrap().to_string()),
                download_path_alias: Some("C:/Users/username/Music".to_string()),
            },
        };
        let resolver = build_resolver(Some(TranscriptionLink::YouTube(YT_LINK.into())));

        let download_path = downloader
            .transcription_download_path(Ustr::from("exercise"), &resolver)
            .unwrap();
        assert!(download_path.ends_with("audio.m4a"));
        assert!(download_path.starts_with(temp_dir.path()));
        assert_eq!(
            40,
            download_path
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .len()
        );

        let alias_path = downloader
            .transcription_download_path_alias(Ustr::from("exercise"), &resolver)
            .unwrap();
        assert!(alias_path.ends_with("audio.m4a"));
        assert!(alias_path.starts_with("C:/Users/username/Music"));
        assert_eq!(
            40,
            alias_path
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .len()
        );
    }
}
