#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <db.h>

int l400_bdb_open(const char *path, int db_type, uint32_t open_flags, DB **out_db) {
    DB *db = NULL;
    int ret = db_create(&db, NULL, 0);
    if (ret != 0) {
        return ret;
    }

    ret = db->open(db, NULL, path, NULL, (DBTYPE)db_type, open_flags, 0644);
    if (ret != 0) {
        db->close(db, 0);
        return ret;
    }

    *out_db = db;
    return 0;
}

int l400_bdb_close(DB *db) {
    return db->close(db, 0);
}

int l400_bdb_put(DB *db, const void *key, uint32_t key_len, const void *data, uint32_t data_len) {
    DBT key_dbt, data_dbt;
    memset(&key_dbt, 0, sizeof(DBT));
    memset(&data_dbt, 0, sizeof(DBT));
    key_dbt.data = (void *)key;
    key_dbt.size = key_len;
    data_dbt.data = (void *)data;
    data_dbt.size = data_len;
    return db->put(db, NULL, &key_dbt, &data_dbt, 0);
}

int l400_bdb_get(DB *db, const void *key, uint32_t key_len, void **out_data, uint32_t *out_len) {
    DBT key_dbt, data_dbt;
    memset(&key_dbt, 0, sizeof(DBT));
    memset(&data_dbt, 0, sizeof(DBT));
    key_dbt.data = (void *)key;
    key_dbt.size = key_len;
    data_dbt.flags = DB_DBT_MALLOC;

    int ret = db->get(db, NULL, &key_dbt, &data_dbt, 0);
    if (ret != 0) {
        return ret;
    }

    *out_data = data_dbt.data;
    *out_len = data_dbt.size;
    return 0;
}

int l400_bdb_del(DB *db, const void *key, uint32_t key_len) {
    DBT key_dbt;
    memset(&key_dbt, 0, sizeof(DBT));
    key_dbt.data = (void *)key;
    key_dbt.size = key_len;
    return db->del(db, NULL, &key_dbt, 0);
}

int l400_bdb_cursor_open(DB *db, DBC **out_cursor) {
    return db->cursor(db, NULL, out_cursor, 0);
}

int l400_bdb_cursor_get(DBC *cursor, void **out_key, uint32_t *out_key_len, void **out_data, uint32_t *out_data_len, uint32_t flags) {
    DBT key_dbt, data_dbt;
    memset(&key_dbt, 0, sizeof(DBT));
    memset(&data_dbt, 0, sizeof(DBT));
    key_dbt.flags = DB_DBT_MALLOC;
    data_dbt.flags = DB_DBT_MALLOC;

    int ret = cursor->get(cursor, &key_dbt, &data_dbt, flags);
    if (ret != 0) {
        return ret;
    }

    *out_key = key_dbt.data;
    *out_key_len = key_dbt.size;
    *out_data = data_dbt.data;
    *out_data_len = data_dbt.size;
    return 0;
}

int l400_bdb_cursor_close(DBC *cursor) {
    return cursor->close(cursor);
}

void l400_bdb_free(void *ptr) {
    free(ptr);
}
