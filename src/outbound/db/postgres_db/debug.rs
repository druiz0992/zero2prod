use super::*;

impl PostgresDb {
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}
