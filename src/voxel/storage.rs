use std::path::Path;
use std::sync::Arc;

use bevy::prelude::Resource;
use redb::{ReadOnlyTable, ReadTransaction, Table, TableDefinition, WriteTransaction};

pub const CHUNKS: TableDefinition<[i32; 3], &[u8]> = TableDefinition::new("chunks");

#[derive(Resource, Clone)]
pub struct WorldDatabase {
    pub db: Arc<redb::Database>,
}

impl WorldDatabase {
    pub fn write<R, K, V>(
        &self,
        table: TableDefinition<K, V>,
        f: impl FnOnce(&WriteTransaction, Table<K, V>) -> R,
    ) -> anyhow::Result<R>
    where
        K: redb::Key + 'static,
        V: redb::Value + 'static,
    {
        let txn = self.db.begin_write()?;
        let table = txn.open_table(table)?;
        let ret = f(&txn, table);
        txn.commit()?;
        Ok(ret)
    }

    pub fn read<R, K, V>(
        &self,
        table: TableDefinition<K, V>,
        f: impl FnOnce(&ReadTransaction, ReadOnlyTable<K, V>) -> R,
    ) -> anyhow::Result<R>
    where
        K: redb::Key + 'static,
        V: redb::Value + 'static,
    {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(table)?;
        let ret = f(&txn, table);
        Ok(ret)
    }

    pub fn new(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let db = WorldDatabase {
            db: Arc::new(redb::Database::builder().create(path)?),
        };
        // create tables
        let txn = db.db.begin_write()?;
        txn.open_table(CHUNKS)?;
        txn.commit()?;
        Ok(db)
    }
}
