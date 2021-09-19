use wildmatch::WildMatch;

use crate::common::FsEntry;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Eq)]
pub struct InstallTask {
    pub source_path: String,
    pub target_path: String,
}

impl Ord for InstallTask {
    fn cmp(&self, other: &Self) -> Ordering {
        self.source_path.cmp(&other.source_path)
    }
}

impl PartialOrd for InstallTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for InstallTask {
    fn eq(&self, other: &Self) -> bool {
        self.source_path == other.source_path
    }
}

fn dir_contains(entry_list: &Vec<FsEntry>, dir: &str, entries: &Vec<&str>) -> bool {
    entry_list.iter().any(|i| {
        for entry in entries {
            let path = format!("{}{}{}", dir, std::path::MAIN_SEPARATOR, entry);
            if WildMatch::new(path.as_ref()).is_match(i.path.as_ref()) {
                return true;
            }
        }
        return false;
    })
}

#[derive(Debug, PartialEq)]
enum ContentType {
    Car,
    Track,
    Unknown,
}

fn get_directory_name_from_entry(entry: &FsEntry) -> String {
    Path::new(&entry.path)
        .parent()
        .unwrap()
        .display()
        .to_string()
}

fn determine_content_type(entry_list: &Vec<FsEntry>, dir: &str) -> ContentType {
    if dir_contains(
        entry_list,
        dir,
        &vec!["animations*", "collider.kn5", "driver_base_pos.knh"],
    ) {
        ContentType::Car
    } else if dir_contains(entry_list, dir, &vec!["ai", "layout_*", "models*.ini"]) {
        ContentType::Track
    } else {
        ContentType::Unknown
    }
}

pub struct Mod {
    mod_type: ContentType,
    path: String,
}

pub fn find_mods(entry_list: &Vec<FsEntry>) -> Vec<Mod> {
    let mut mod_dirs: HashSet<String> = HashSet::new();

    let kn5_model_list: Vec<&FsEntry> = entry_list
        .iter()
        .filter(|&p| {
            p.is_file
                && WildMatch::new("*.kn5")
                    .is_match(Path::new(&p.path).file_name().unwrap().to_str().unwrap())
        })
        .collect();

    for model in kn5_model_list {
        let dir_path = get_directory_name_from_entry(model);
        mod_dirs.insert(dir_path);
    }

    let mut mods = vec![];
    for dir in mod_dirs {
        let mod_type = determine_content_type(entry_list, dir.as_ref());
        mods.push(Mod {
            mod_type: mod_type,
            path: dir,
        });
    }

    mods
}

fn extension_dir_is_mod_dir(entry_list: &Vec<FsEntry>, entry: &FsEntry) -> bool {
    let dir = Path::new(&entry.path).parent().unwrap();
    determine_content_type(entry_list, dir.to_str().unwrap()) != ContentType::Unknown
}

pub fn determine_install_tasks(entry_list: &Vec<FsEntry>) -> Result<Vec<InstallTask>, &str> {
    let content_dirs: Vec<&FsEntry> = entry_list
        .iter()
        .filter(|&p| !p.is_file && Path::new(&p.path).file_name().unwrap() == "content")
        .collect();
    let extension_dirs: Vec<&FsEntry> = entry_list
        .iter()
        .filter(|&p| {
            let is_extension_dir =
                !p.is_file && Path::new(&p.path).file_name().unwrap() == "extension";
            if !is_extension_dir {
                return false;
            }
            !extension_dir_is_mod_dir(entry_list, p)
        })
        .collect();

    let content_dir_count = content_dirs.len();
    let extension_dir_count = extension_dirs.len();

    if content_dir_count > 1 || extension_dir_count > 1 {
        return Err("Multiple content or extension dirs found");
    }

    let mut ret: Vec<InstallTask> = vec![];

    if content_dir_count == 1 {
        let task = InstallTask {
            source_path: content_dirs[0].path.clone(),
            target_path: "".to_string(),
        };
        ret.push(task);
        if extension_dir_count == 1 {
            let content_parent = Path::new(&content_dirs[0].path).parent().unwrap();
            let extension_parent = Path::new(&extension_dirs[0].path).parent().unwrap();
            if *content_parent == *extension_parent {
                let task = InstallTask {
                    source_path: extension_dirs[0].path.clone(),
                    target_path: "".to_string(),
                };
                ret.push(task);
            }
        }
    }
    if ret.len() > 0 {
        return Ok(ret);
    }

    for ac_mod in find_mods(entry_list) {
        match ac_mod.mod_type {
            ContentType::Car => ret.push(InstallTask {
                source_path: ac_mod.path,
                target_path: "content/cars".to_string(),
            }),
            ContentType::Track => ret.push(InstallTask {
                source_path: ac_mod.path,
                target_path: "content/tracks".to_string(),
            }),
            ContentType::Unknown => println!("Failed to determine content type for {}", ac_mod.path),
        }
    }

    Ok(ret)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vec_equal<T: PartialEq>(a: &Vec<T>, b: &Vec<T>) -> bool {
        let matching = a.iter().zip(b.iter()).filter(|&(a, b)| a == b).count();
        matching == a.len() && matching == b.len()
    }

    #[test]
    fn test_vec_equal() {
        assert!(vec_equal(&vec![1, 2, 3], &vec![1, 2, 3]));
        assert!(vec_equal(&Vec::<u32>::new(), &Vec::<u32>::new()));
        assert!(!vec_equal(&vec![1, 2], &vec![1, 2, 3]));
    }

    #[test]
    fn test_determine_content_type() {
        let unknown_content_entries: Vec<FsEntry> = vec![
            FsEntry {
                path: "/unknown_content".to_string(),
                is_file: false,
            },
            FsEntry {
                path: "/unknown_content/whatever".to_string(),
                is_file: true,
            },
            FsEntry {
                path: "/some_car".to_string(),
                is_file: false,
            },
            FsEntry {
                path: "/some_car/driver_base_pos.knh".to_string(),
                is_file: true,
            },
            FsEntry {
                path: "/some_car/some_car.kn5".to_string(),
                is_file: true,
            },
            FsEntry {
                path: "/another_car".to_string(),
                is_file: false,
            },
            FsEntry {
                path: "/another_car/animations".to_string(),
                is_file: false,
            },
            FsEntry {
                path: "/another_car/data.acd".to_string(),
                is_file: true,
            },
            FsEntry {
                path: "/some_track".to_string(),
                is_file: false,
            },
            FsEntry {
                path: "/some_track/models.ini".to_string(),
                is_file: true,
            },
            FsEntry {
                path: "/some_track/some_track.kn5".to_string(),
                is_file: true,
            },
        ];
        assert_eq!(ContentType::Unknown, determine_content_type(&unknown_content_entries, "/unknown_content"));
        assert_eq!(ContentType::Car, determine_content_type(&unknown_content_entries, "/some_car"));
        assert_eq!(ContentType::Car, determine_content_type(&unknown_content_entries, "/another_car"));
        assert_eq!(ContentType::Track, determine_content_type(&unknown_content_entries, "/some_track"));
    }

    #[test]
    fn test_determine_install_tasks_unknown_mod_content() {
        let entries: Vec<FsEntry> = vec![
            FsEntry {
                path: "/some_super_car".to_string(),
                is_file: false,
            },
            FsEntry {
                path: "/some_super_car/whatever".to_string(),
                is_file: true,
            },
        ];
        let expected: Vec<InstallTask> = vec![];
        let tasks = determine_install_tasks(&entries);

        assert!(tasks.is_ok());
        assert!(vec_equal(&expected, &tasks.unwrap()));
    }

    #[test]
    fn test_determine_install_tasks_one_content_dir() {
        let simple_mod_entries: Vec<FsEntry> = vec![
            FsEntry {
                path: "/tmp/unpacked/content".to_string(),
                is_file: false,
            },
            FsEntry {
                path: "/tmp/unpacked/content/cars".to_string(),
                is_file: false,
            },
            FsEntry {
                path: "/tmp/unpacked/content/cars/aaa_super_fast".to_string(),
                is_file: false,
            },
            FsEntry {
                path: "/tmp/unpacked/content/cars/aaa_super_fast/animations".to_string(),
                is_file: false,
            },
            FsEntry {
                path: "/tmp/unpacked/content/cars/aaa_super_fast/data.acd".to_string(),
                is_file: true,
            },
        ];

        let expected: Vec<InstallTask> = vec![InstallTask {
            source_path: "/tmp/unpacked/content".to_string(),
            target_path: "".to_string(),
        }];
        let tasks = determine_install_tasks(&simple_mod_entries);

        assert!(tasks.is_ok());
        assert!(vec_equal(&expected, &tasks.unwrap()));
    }

    #[test]
    fn test_determine_install_tasks_one_content_dir_and_one_extension_dir() {
        let simple_mod_entries: Vec<FsEntry> = vec![
            FsEntry {
                path: "/tmp/unpacked/content".to_string(),
                is_file: false,
            },
            FsEntry {
                path: "/tmp/unpacked/extension".to_string(),
                is_file: false,
            },
        ];

        let expected: Vec<InstallTask> = vec![
            InstallTask {
                source_path: "/tmp/unpacked/content".to_string(),
                target_path: "".to_string(),
            },
            InstallTask {
                source_path: "/tmp/unpacked/extension".to_string(),
                target_path: "".to_string(),
            },
        ];
        let tasks = determine_install_tasks(&simple_mod_entries);

        assert!(tasks.is_ok());
        assert!(vec_equal(&expected, &tasks.unwrap()));
    }

    #[test]
    fn test_determine_install_tasks_multiple_content_dirs() {
        let simple_mod_entries: Vec<FsEntry> = vec![
            FsEntry {
                path: "/tmp/unpacked/aaa/content".to_string(),
                is_file: false,
            },
            FsEntry {
                path: "/tmp/unpacked/bbb/content".to_string(),
                is_file: false,
            },
        ];

        let tasks = determine_install_tasks(&simple_mod_entries);

        assert!(tasks.is_err());
    }

    #[test]
    fn test_determine_install_tasks_multiple_extension_dirs() {
        let simple_mod_entries: Vec<FsEntry> = vec![
            FsEntry {
                path: "/tmp/unpacked/aaa/extension".to_string(),
                is_file: false,
            },
            FsEntry {
                path: "/tmp/unpacked/bbb/extension".to_string(),
                is_file: false,
            },
        ];

        let tasks = determine_install_tasks(&simple_mod_entries);

        assert!(tasks.is_err());
    }

    #[test]
    fn test_determine_install_tasks_one_car_dir() {
        let simple_mod_entries: Vec<FsEntry> = vec![
            FsEntry {
                path: "/tmp/unpacked/some_car".to_string(),
                is_file: false,
            },
            FsEntry {
                path: "/tmp/unpacked/some_car/driver_base_pos.knh".to_string(),
                is_file: true,
            },
            FsEntry {
                path: "/tmp/unpacked/some_car/some_car.kn5".to_string(),
                is_file: true,
            },
        ];

        let expected: Vec<InstallTask> = vec![InstallTask {
            source_path: "/tmp/unpacked/some_car".to_string(),
            target_path: "content/cars".to_string(),
        }];
        let tasks = determine_install_tasks(&simple_mod_entries);

        assert!(tasks.is_ok());
        assert!(vec_equal(&expected, &tasks.unwrap()));
    }

    #[test]
    fn test_determine_install_tasks_one_car_dir_one_track_dir() {
        let simple_mod_entries: Vec<FsEntry> = vec![
            FsEntry {
                path: "/tmp/unpacked/some_car".to_string(),
                is_file: false,
            },
            FsEntry {
                path: "/tmp/unpacked/some_car/driver_base_pos.knh".to_string(),
                is_file: true,
            },
            FsEntry {
                path: "/tmp/unpacked/some_car/some_car.kn5".to_string(),
                is_file: true,
            },
            FsEntry {
                path: "/tmp/unpacked/nested/whatever/some_track".to_string(),
                is_file: false,
            },
            FsEntry {
                path: "/tmp/unpacked/nested/whatever/some_track/models.ini".to_string(),
                is_file: true,
            },
            FsEntry {
                path: "/tmp/unpacked/nested/whatever/some_track/some_track.kn5".to_string(),
                is_file: true,
            },
        ];

        let expected: Vec<InstallTask> = vec![
            InstallTask {
                source_path: "/tmp/unpacked/nested/whatever/some_track".to_string(),
                target_path: "content/tracks".to_string(),
            },
            InstallTask {
                source_path: "/tmp/unpacked/some_car".to_string(),
                target_path: "content/cars".to_string(),
            },
        ];
        let tasks = determine_install_tasks(&simple_mod_entries);

        assert!(tasks.is_ok());
        let mut tasks = tasks.unwrap();
        tasks.sort();
        assert!(vec_equal(&expected, &tasks));
    }
}
