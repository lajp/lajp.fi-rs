use diesel::{
    mysql::MysqlConnection,
    prelude::*,
    r2d2::{ConnectionManager, Pool},
};

use std::io::{stdin, stdout, Write};

pub struct Database {
    pool: Pool<ConnectionManager<MysqlConnection>>,
}

use crate::models::*;

impl Database {
    pub fn new() -> Self {
        let database_url = std::env::var("DATABASE_URL").expect("DATBASE_URL must be set");
        let manager = ConnectionManager::<MysqlConnection>::new(&database_url);
        let pool = Pool::builder().build(manager).unwrap();
        Self { pool }
    }

    pub fn get_blog_entries(&self) -> Vec<BlogEntry> {
        use crate::schema::BlogEntries::dsl::*;
        BlogEntries
            .order_by(date.desc())
            .load::<BlogEntry>(&self.pool.get().unwrap())
            .unwrap()
    }

    fn new_blog_entry(&self, entry: NewBlogEntry) {
        diesel::insert_into(crate::schema::BlogEntries::table)
            .values(&entry)
            .execute(&self.pool.get().unwrap())
            .unwrap();
    }

    fn is_duplicate(&self, ipath: &str) -> bool {
        use crate::schema::BlogEntries::dsl::*;
        !BlogEntries
            .filter(path.eq(ipath))
            .load::<BlogEntry>(&self.pool.get().unwrap())
            .unwrap()
            .is_empty()
    }

    pub fn check_for_new_entries(&self) {
        for file in std::fs::read_dir("templates/blog").unwrap() {
            let fspath = file.as_ref().unwrap().path().to_str().unwrap().to_string();
            let path = format!(
                "/{}",
                &fspath[fspath.find("blog").unwrap()..fspath.find('.').unwrap()]
            )
            .to_string();
            if self.is_duplicate(&path) {
                continue;
            }
            println!("New blog entry discovered at: {}", &path);
            print!("Add it to the database? (y/n): ");
            let _ = stdout().flush();
            let mut answer = String::new();
            stdin().read_line(&mut answer).unwrap();
            if !answer.starts_with('y') {
                println!("Skipping entry!");
                continue;
            }
            print!("Enter title: ");
            let _ = stdout().flush();
            let mut title = String::new();
            stdin().read_line(&mut title).unwrap();
            print!("Enter description: ");
            let _ = stdout().flush();
            let mut description = String::new();
            stdin().read_line(&mut description).unwrap();
            use std::time::SystemTime;
            let mut date = chrono::NaiveDateTime::from_timestamp(
                file.as_ref()
                    .unwrap()
                    .metadata()
                    .unwrap()
                    .created()
                    .unwrap()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64,
                0,
            )
            .date();
            println!("The creation date of the file is {}", date);
            print!("Do you want to use that? (y/n): ");
            let _ = stdout().flush();
            answer.clear();
            stdin().read_line(&mut answer).unwrap();
            if !answer.starts_with('y') {
                loop {
                    print!("Enter the alternative date (yyyy-mm-dd): ");
                    let _ = stdout().flush();
                    answer.clear();
                    stdin().read_line(&mut answer).unwrap();
                    match chrono::NaiveDate::parse_from_str(&answer, "%Y-%m-%d\n") {
                        Ok(d) => {
                            date = d;
                            break;
                        }
                        Err(e) => {
                            println!("Invalid date format! {}", e);
                        }
                    }
                }
            }
            self.new_blog_entry(NewBlogEntry {
                title,
                description,
                date,
                path,
            });
        }
    }
}
