use chrono::NaiveDate;
use rand::seq::SliceRandom;
use regex::Regex;
use serde_derive::Serialize;
use std::sync::LazyLock;

static TITLE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\{%\sblock\stitle\s%\}(.*)\{%\sendblock"#).unwrap());
static DATE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\{%\sblock\sdate\s%\}(.*)\{%\sendblock"#).unwrap());
static DESCRIPTION_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\{%\sblock\sdescription\s%\}(.*)\{%\sendblock"#).unwrap());

#[derive(Serialize, Debug, Clone)]
pub struct BlogEntry {
    title: String,
    description: String,
    date: String,
    path: String,
}

impl BlogEntry {
    fn new(template: &str) -> Self {
        let [title, date, description] = [&TITLE_REGEX, &DATE_REGEX, &DESCRIPTION_REGEX].map(|r| {
            if let Some(m) = r.captures(template).unwrap().get(1) {
                m.as_str().to_string()
            } else {
                String::new()
            }
        });

        Self {
            title,
            date,
            description,
            path: String::new(),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct BlogContext {
    blogentries: Vec<BlogEntry>,
}

impl BlogContext {
    pub fn new(path: &str) -> Self {
        let mut entries = std::fs::read_dir(std::path::Path::new(path))
            .unwrap()
            .map(|file| {
                let content = std::fs::read_to_string(file.as_ref().unwrap().path()).unwrap();
                let mut entry = BlogEntry::new(&content);
                entry.path = format!(
                    "/blog/{}",
                    file.as_ref().unwrap().file_name().to_str().unwrap()
                );
                entry
            })
            .collect::<Vec<_>>();

        entries.sort_by_key(|k| NaiveDate::parse_from_str(&k.date, "%F").unwrap());
        entries.reverse();

        Self {
            blogentries: entries,
        }
    }
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub path: String,
    pub name: String,
}

impl Image {
    pub fn new(path: &str) -> Self {
        Self {
            name: path.rsplit('/').next().unwrap_or_default().to_string(),
            path: path[1..].to_string(), // Strip the redundant "." from the start
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct ImageGallery {
    path: String,
    pub images: Vec<Image>,
}

impl ImageGallery {
    pub fn new(path: &str) -> Self {
        let images = std::fs::read_dir(std::path::Path::new(path))
            .unwrap()
            .filter_map(|file| Some(Image::new(file.ok()?.path().to_str()?)))
            .collect::<Vec<_>>();

        Self {
            path: path.to_string(),
            images,
        }
    }

    pub fn add_image(&mut self, img: Image) {
        self.images.push(img)
    }

    pub fn shuffle(&mut self) {
        let mut rng = rand::thread_rng();
        self.images.shuffle(&mut rng);
    }
}
