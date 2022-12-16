use chrono::NaiveDate;
use rand::seq::SliceRandom;
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
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
    path: String,
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
            path: path.to_string(),
            blogentries: entries,
        }
    }

    pub fn reload(&mut self) {
        self.blogentries = Self::new(&self.path).blogentries;
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

#[derive(Deserialize, Debug, Clone)]
pub struct Artifact {
    pub archive_download_url: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Artifacts {
    pub artifacts: Vec<Artifact>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct WorkflowRun {
    pub artifacts_url: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct UpdatePayload {
    pub workflow_run: Option<WorkflowRun>,
}

#[derive(Serialize, Deserialize, Clone)]
struct HeartBeat {
    project_name: Option<String>,
    editor_name: Option<String>,
    hostname: Option<String>,
    language: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Activity {
    started: chrono::NaiveDateTime,
    duration: i32,
    heartbeat: HeartBeat,
}

#[derive(Debug, Serialize)]
pub struct IndexActivity {
    pub active: bool,
    pub seconds: Option<i64>,
    pub minutes: Option<i64>,
    pub hours: Option<i64>,
    pub project_name: Option<String>,
    pub editor_name: Option<String>,
    pub hostname: Option<String>,
    pub language: Option<String>,
}

impl From<Option<Activity>> for IndexActivity {
    fn from(activity: Option<Activity>) -> Self {
        match activity {
            Some(a) => {
                let duration = chrono::Local::now().naive_utc() - a.started;
                let reported_duration = chrono::Duration::seconds(a.duration as i64);

                Self {
                    active: (duration - reported_duration) <= chrono::Duration::seconds(900),
                    seconds: Some(duration.num_seconds() % 60),
                    minutes: Some(duration.num_minutes() % 60),
                    hours: Some(duration.num_hours()),
                    project_name: a.heartbeat.project_name,
                    editor_name: a.heartbeat.editor_name,
                    hostname: a.heartbeat.hostname,
                    language: a.heartbeat.language,
                }
            }
            None => Self {
                active: false,
                seconds: None,
                minutes: None,
                hours: None,
                project_name: None,
                editor_name: None,
                hostname: None,
                language: None,
            },
        }
    }
}
