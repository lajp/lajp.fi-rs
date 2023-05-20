use crate::models::Visits;
use crate::visitcounter::Visit;

use crate::diesel::prelude::*;

use std::env;

use diesel::{
    pg::PgConnection,
    r2d2::{ConnectionManager, Pool},
};

#[derive(Clone)]
pub struct Database {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl Database {
    pub fn new() -> Self {
        let db_url = env::var("DATABASE_URL").expect("DATABASE_URL not present in env");

        let manager = ConnectionManager::<PgConnection>::new(db_url);
        let pool = Pool::builder()
            .build(manager)
            .expect("Failed to create connection pool");
        Self { pool }
    }

    // TODO: Do proper error handling
    pub async fn new_visit(&self, visit: Visit) -> Result<usize, crate::error::Error> {
        use crate::schema::visits::dsl::*;
        let mut conn = self.pool.get()?;

        Ok(diesel::insert_into(visits)
            .values(&visit)
            .execute(&mut conn)?)
    }

    pub async fn visits_per_path(&self) -> Result<Vec<Visits>, crate::error::Error> {
        use crate::schema::visits::dsl::*;
        let mut conn = self.pool.get()?;

        Ok(visits
            .group_by(path)
            .select((path, diesel::dsl::count(path)))
            .load::<(String, i64)>(&mut conn)?
            .iter()
            .map(|(a, b)| Visits {
                path: a.to_string(),
                visit_count: *b,
            })
            .collect())
    }
}
