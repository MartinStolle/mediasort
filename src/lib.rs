#[macro_use]
extern crate log;

use clap::Parser;
use exif::{In, Tag};
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::{env, error::Error, fs};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// the name of the directory to parse
    #[arg(short, long)]
    folder: String,

    /// Should the directory be parsed recursively
    #[arg(short, long, default_value_t = true)]
    recursive: bool,
}

#[derive(Debug)]
pub struct MediaConfig {
    pub source: String,
    pub target: PathBuf,
    files: HashMap<String, String>,
}

impl MediaConfig {
    pub fn new(source: String, target: PathBuf) -> Self {
        Self {
            source,
            target,
            files: HashMap::new(),
        }
    }

    pub fn copy_media_files(&mut self) -> Result<(), Box<dyn Error>> {
        self.find_all_media_files(None, true)?;
        info!("Found {} files", self.files.len());
        let mut copied_files = 0;
        for (source, target) in self.files.iter() {
            match copy_file(source, target) {
                Ok(true) => copied_files += 1,
                Ok(false) => (),
                Err(e) => error!("Error copying file: {}", e),
            }
        }
        info!("Copied {}/{} files", copied_files, self.files.len());
        Ok(())
    }

    fn find_all_media_files(
        &mut self,
        path: Option<&str>,
        recursive: bool,
    ) -> Result<(), Box<dyn Error>> {
        let path = path.unwrap_or(&self.source);
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && recursive {
                self.find_all_media_files(Some(path.to_str().unwrap()), true)?;
            } else if path.is_file() && is_media_file(&path) {
                let sourcepath = &path.to_str().unwrap();
                if let Some(targetpath) = smartphone_file(sourcepath) {
                    self.files
                        .insert(sourcepath.to_string(), targetpath.to_owned());
                } else if let Some(targetpath) = read_jpg_exif(sourcepath) {
                    self.files
                        .insert(sourcepath.to_string(), targetpath.to_owned());
                }
            }
        }
        Ok(())
    }
}

fn is_media_file(path: &Path) -> bool {
    let ext = path.extension();

    match ext {
        None => false,
        Some(file_ext) => matches!(
            String::from(file_ext.to_str().unwrap())
                .to_lowercase()
                .as_str(),
            "jpg" | "jpeg" | "mp4" | "png"
        ),
    }
}

pub fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let home = env::var("HOME")?;
    let target = Path::new(&home).join("Pictures");
    MediaConfig::new(args.folder, target).copy_media_files()?;
    Ok(())
}

// Copy file from one directory to another
fn copy_file(from: &str, to: &str) -> Result<bool, Box<dyn Error>> {
    let abs_path = Path::new(&to);
    let parent = abs_path.parent().unwrap();
    create_dir(parent.to_str().unwrap())?;
    if abs_path.exists() {
        warn!("Skipping File {}, already exists", to);
        return Ok(false);
    }
    info!("Copy file {} to {}", from, abs_path.to_str().unwrap());
    fs::copy(from, to)?;
    Ok(true)
}

// Create directory, if it does not exist
fn create_dir(path: &str) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(path)?;
    Ok(())
}

// Read date from smartphone image or video filename
fn smartphone_file(filename: &str) -> Option<String> {
    lazy_static! {
        static ref RE: Regex = Regex::new(
            r"(?x)
  (?:IMG|VID)_
  (?P<y>\d{4}) # the year
  (?P<m>\d{2}) # the month
  (?P<d>\d{2}) # the day
  _(\d{6}).(?:jpg|mp4)
"
        )
        .unwrap();
    };

    RE.captures(filename)
        .map(|cap| format!("{}/{}/{}/{}", &cap["y"], &cap["m"], &cap["d"], &cap[0]))
}

fn read_jpg_exif(filename: &str) -> Option<String> {
    // filename needs to end with .jpg or .png
    if !filename.to_lowercase().ends_with(".jpg") && !filename.to_lowercase().ends_with(".png") {
        return None;
    }
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r"(?P<y>\d{4})-(?P<m>\d{2})-(?P<d>\d{2})\s+(?:\d|:){8}").unwrap();
    };
    let file = File::open(filename).unwrap_or_else(|_| panic!("Could not open file {}", filename));
    let mut bufreader = std::io::BufReader::new(&file);
    let exifreader = exif::Reader::new();
    let exif = exifreader.read_from_container(&mut bufreader).unwrap();
    let datetime = match exif.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
        Some(field) => RE
            .captures(field.display_value().to_string().as_str())
            .map(|cap| {
                format!(
                    "{}/{}/{}/{}",
                    &cap["y"],
                    &cap["m"],
                    &cap["d"],
                    Path::new(filename)
                        .file_name()
                        .expect("no filename")
                        .to_str()
                        .unwrap()
                )
            }),
        _ => Some(String::from("no exif data")),
    };
    datetime
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    macro_rules! test_case {
        ($fname:expr) => {
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/", $fname)
        };
    }

    #[test]
    fn test_is_media_file() {
        let list_of_media_files = vec!["jpg", "jpeg", "mp4", "png", "JPG", "JPEG", "MP4", "PNG"];
        for media_file in list_of_media_files {
            let filename = format!("test.{}", media_file);
            assert_eq!(
                true,
                is_media_file(Path::new(&filename)),
                "File should be a media file {}",
                filename
            );
        }
    }

    #[test]
    fn test_read_jpg_exif() {
        let filename = test_case!("test_image.JPG");
        assert_eq!(
            Some(String::from(format!("2022/12/17/test_image.JPG"))),
            read_jpg_exif(filename)
        );
    }

    #[test]
    fn read_no_smartphone_image() {
        let filename = "no_match.jpg";
        assert_eq!(None, smartphone_file(filename));
    }
    #[test]
    fn read_smartphone_video() {
        let filename = "VID_20221220_170102.jpg";
        assert_eq!(
            Some(String::from(format!("2022/12/20/{filename}"))),
            smartphone_file(filename)
        );
    }
    #[test]
    fn read_smartphone_image() {
        let filename = "IMG_20230115_102911.jpg";
        assert_eq!(
            Some(String::from(format!("2023/01/15/{filename}"))),
            smartphone_file(filename)
        );
    }

    #[test]
    fn find_all_media_files_recursive() {
        let tmpdir = TempDir::new().unwrap();
        let test_images = tmpdir.path().join("test_images");
        //let target_images = tmpdir.path().join("target_images");
        create_dir(test_images.to_str().unwrap()).unwrap();
        let test_media_files = [
            "IMG_20210130_000001.jpg",
            "IMG_20210130_000002.jpg",
            "VID_20210130_000003.mp4",
        ];
        for file in test_media_files.iter() {
            fs::File::create(test_images.join(file)).expect("Just create the test files");
        }

        let mut mediaconfig = MediaConfig::new(
            test_images.to_str().unwrap().to_string(),
            tmpdir.path().join("target_images"),
        );
        mediaconfig
            .find_all_media_files(Some(tmpdir.path().to_str().unwrap()), true)
            .expect("Everything works as intended");
        assert_eq!(test_media_files.len(), mediaconfig.files.len());

        let targets: Vec<String> = mediaconfig.files.into_values().collect();
        for file in test_media_files.iter() {
            assert!(targets.contains(&String::from(format!("2021/01/30/{file}"))));
        }

        tmpdir.close().expect("Remove test directory");
    }
}
