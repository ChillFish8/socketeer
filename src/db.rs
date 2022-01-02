use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::sync::Arc;

use scylla::{QueryResult, SessionBuilder};
use scylla::frame::value::ValueList;
use scylla::prepared_statement::PreparedStatement;
use concread::arcache::{ARCache, ARCacheBuilder};

#[derive(Clone)]
pub struct Session(Arc<scylla::Session>, Arc<ARCache<String, PreppedStmt>>);

impl From<scylla::Session> for Session {
    fn from(s: scylla::Session) -> Self {
        let cache = ARCacheBuilder::new()
            .set_size(50, num_cpus::get())
            .build()
            .unwrap();

        Self(Arc::new(s), Arc::new(cache))
    }
}

impl Session {
    #[instrument(skip(self, query), level = "trace")]
    pub async fn query(
        &self,
        query: &str,
        values: impl ValueList + Debug,
    ) -> anyhow::Result<QueryResult> {
        trace!("executing query {}", query);
        let result = self.0.query(query, values).await.map_err(anyhow::Error::from);

        if let Err(ref e) = result {
            error!("failed to execute query: {} due to error: {}", query, e);
        }

        result
    }

    #[instrument(skip(self, query), level = "trace")]
    pub async fn query_prepared(
        &self,
        query: &str,
        values: impl ValueList + Debug,
    ) -> anyhow::Result<QueryResult> {
        {
            let mut reader = self.1.read();
            if let Some(prep) = reader.get(query) {
                trace!("using cached prepared statement: {}", prep.get_statement());
                let result = self.0
                    .execute(prep, values)
                    .await
                    .map_err(anyhow::Error::from);

                if let Err(ref e) = result {
                    error!("failed to execute prepared statement: {} due to error: {}", prep.get_statement(), e);
                }

                return result
            };
        }

        trace!("preparing new statement: {}", query);
        let stmt = self.0.prepare(query)
            .await
            .map_err(anyhow::Error::from);

        if let Err(ref e) = stmt {
            error!(
                "failed to prepare statement: {} due to error: {}",
                query,
                e,
            );
        }

        let stmt = stmt?;

        let result = self.0
            .execute(&stmt, values)
            .await
            .map_err(anyhow::Error::from);

        if let Err(e) = result {
            error!("failed to execute prepared statement: {} due to error: {}", stmt.get_statement(), e);
            return Err(e)
        }

        let mut writer = self.1.write();
        writer.insert(query.to_string(), PreppedStmt::from(stmt));
        writer.commit();

        result
    }
}

#[derive(Clone)]
struct PreppedStmt(PreparedStatement);

impl Debug for PreppedStmt {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "PreparedStmt")
    }
}

impl From<PreparedStatement> for PreppedStmt {
    fn from(v: PreparedStatement) -> Self {
        Self(v)
    }
}

impl Deref for PreppedStmt{
    type Target = PreparedStatement;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


pub async fn connect(node: &str) -> anyhow::Result<Session> {
    let session = SessionBuilder::new()
        .known_node(node)
        .build()
        .await?;

    let _ = session.query("CREATE KEYSPACE spooderfy WITH replication = {'class': 'SimpleStrategy', 'replication_factor' : 1};", &[]).await;
    session.use_keyspace("spooderfy", false).await?;

    create_tables(&session).await?;

    Ok(Session::from(session))
}

async fn create_tables(session: &scylla::Session) -> anyhow::Result<()> {
    let query_block = include_str!("./scripts/tables.cql");

    for query in query_block.split("--") {
        if query.is_empty() {
            continue;
        }

        info!("executing {}", query.replace("\r\n", "").replace("    ", " "));
        session.query(
            query,
            &[]
        ).await?;
    }

    Ok(())
}