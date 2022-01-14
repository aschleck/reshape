use reshape::migrations::{
    AddIndex, AlterColumn, ColumnBuilder, ColumnChanges, CreateTableBuilder, Migration,
};

mod common;

#[test]
fn alter_column_data() {
    let (mut reshape, mut old_db, mut new_db) = common::setup();

    let create_users_table = Migration::new("create_user_table", None).with_action(
        CreateTableBuilder::default()
            .name("users")
            .primary_key(vec!["id".to_string()])
            .columns(vec![
                ColumnBuilder::default()
                    .name("id")
                    .data_type("INTEGER")
                    .build()
                    .unwrap(),
                ColumnBuilder::default()
                    .name("name")
                    .data_type("TEXT")
                    .build()
                    .unwrap(),
            ])
            .build()
            .unwrap(),
    );
    let uppercase_name = Migration::new("uppercase_name", None).with_action(AlterColumn {
        table: "users".to_string(),
        column: "name".to_string(),
        up: Some("UPPER(name)".to_string()),
        down: Some("LOWER(name)".to_string()),
        changes: ColumnChanges {
            data_type: None,
            nullable: None,
            name: None,
            default: None,
        },
    });

    let first_migrations = vec![create_users_table.clone()];
    let second_migrations = vec![create_users_table.clone(), uppercase_name.clone()];

    // Run first migration, should automatically finish
    reshape.migrate(first_migrations.clone()).unwrap();

    // Update search paths
    old_db
        .simple_query(&reshape::schema_query_for_migration(
            &first_migrations.last().unwrap().name,
        ))
        .unwrap();

    // Insert some test users
    old_db
        .simple_query(
            "
            INSERT INTO users (id, name) VALUES
                (1, 'john Doe'),
                (2, 'jane Doe');
            ",
        )
        .unwrap();

    // Run second migration
    reshape.migrate(second_migrations.clone()).unwrap();
    new_db
        .simple_query(&reshape::schema_query_for_migration(
            &second_migrations.last().unwrap().name,
        ))
        .unwrap();

    // Check that the existing users has the altered data
    let expected = vec!["JOHN DOE", "JANE DOE"];
    assert!(new_db
        .query("SELECT name FROM users ORDER BY id", &[],)
        .unwrap()
        .iter()
        .map(|row| row.get::<_, String>("name"))
        .eq(expected));

    // Insert data using old schema and make sure the new schema gets correct values
    old_db
        .simple_query("INSERT INTO users (id, name) VALUES (3, 'test testsson')")
        .unwrap();
    let result = new_db
        .query_one("SELECT name from users WHERE id = 3", &[])
        .unwrap();
    assert_eq!("TEST TESTSSON", result.get::<_, &str>("name"));

    // Insert data using new schema and make sure the old schema gets correct values
    new_db
        .simple_query("INSERT INTO users (id, name) VALUES (4, 'TEST TESTSSON')")
        .unwrap();
    let result = old_db
        .query_one("SELECT name from users WHERE id = 4", &[])
        .unwrap();
    assert_eq!("test testsson", result.get::<_, &str>("name"));

    reshape.complete().unwrap();
    common::assert_cleaned_up(&mut new_db);
}

#[test]
fn alter_column_set_not_null() {
    let (mut reshape, mut old_db, mut new_db) = common::setup();

    let create_users_table = Migration::new("create_user_table", None).with_action(
        CreateTableBuilder::default()
            .name("users")
            .primary_key(vec!["id".to_string()])
            .columns(vec![
                ColumnBuilder::default()
                    .name("id")
                    .data_type("INTEGER")
                    .build()
                    .unwrap(),
                ColumnBuilder::default()
                    .name("name")
                    .data_type("TEXT")
                    .build()
                    .unwrap(),
            ])
            .build()
            .unwrap(),
    );
    let set_name_not_null = Migration::new("set_name_not_null", None).with_action(AlterColumn {
        table: "users".to_string(),
        column: "name".to_string(),
        up: Some("COALESCE(name, 'TEST_DEFAULT_VALUE')".to_string()),
        down: Some("name".to_string()),
        changes: ColumnChanges {
            data_type: None,
            nullable: Some(false),
            name: None,
            default: None,
        },
    });

    let first_migrations = vec![create_users_table.clone()];
    let second_migrations = vec![create_users_table.clone(), set_name_not_null.clone()];

    // Run first migration, should automatically finish
    reshape.migrate(first_migrations.clone()).unwrap();

    // Update search paths
    old_db
        .simple_query(&reshape::schema_query_for_migration(
            &first_migrations.last().unwrap().name,
        ))
        .unwrap();

    // Insert some test users
    old_db
        .simple_query(
            "
            INSERT INTO users (id, name) VALUES
                (1, 'John Doe'),
                (2, NULL);
            ",
        )
        .unwrap();

    // Run second migration
    reshape.migrate(second_migrations.clone()).unwrap();
    new_db
        .simple_query(&reshape::schema_query_for_migration(
            &second_migrations.last().unwrap().name,
        ))
        .unwrap();

    // Check that existing users got the correct values
    let expected = vec!["John Doe", "TEST_DEFAULT_VALUE"];
    assert!(new_db
        .query("SELECT name FROM users ORDER BY id", &[],)
        .unwrap()
        .iter()
        .map(|row| row.get::<_, String>("name"))
        .eq(expected));

    // Insert data using old schema and make sure the new schema gets correct values
    old_db
        .simple_query("INSERT INTO users (id, name) VALUES (3, NULL)")
        .unwrap();
    let result = new_db
        .query_one("SELECT name from users WHERE id = 3", &[])
        .unwrap();
    assert_eq!("TEST_DEFAULT_VALUE", result.get::<_, &str>("name"));

    // Insert data using new schema and make sure the old schema gets correct values
    new_db
        .simple_query("INSERT INTO users (id, name) VALUES (4, 'Jane Doe')")
        .unwrap();
    let result = old_db
        .query_one("SELECT name from users WHERE id = 4", &[])
        .unwrap();
    assert_eq!("Jane Doe", result.get::<_, &str>("name"));

    reshape.complete().unwrap();
    common::assert_cleaned_up(&mut new_db);
}

#[test]
fn alter_column_rename() {
    let (mut reshape, mut old_db, mut new_db) = common::setup();

    let create_users_table = Migration::new("create_user_table", None).with_action(
        CreateTableBuilder::default()
            .name("users")
            .primary_key(vec!["id".to_string()])
            .columns(vec![
                ColumnBuilder::default()
                    .name("id")
                    .data_type("INTEGER")
                    .build()
                    .unwrap(),
                ColumnBuilder::default()
                    .name("name")
                    .data_type("TEXT")
                    .build()
                    .unwrap(),
            ])
            .build()
            .unwrap(),
    );
    let rename_to_full_name =
        Migration::new("rename_to_full_name", None).with_action(AlterColumn {
            table: "users".to_string(),
            column: "name".to_string(),
            up: None, // up and down are not required when only renaming a column
            down: None,
            changes: ColumnChanges {
                data_type: None,
                nullable: None,
                name: Some("full_name".to_string()),
                default: None,
            },
        });

    let first_migrations = vec![create_users_table.clone()];
    let second_migrations = vec![create_users_table.clone(), rename_to_full_name.clone()];

    // Run first migration, should automatically finish
    reshape.migrate(first_migrations.clone()).unwrap();

    // Update search paths
    old_db
        .simple_query(&reshape::schema_query_for_migration(
            &first_migrations.last().unwrap().name,
        ))
        .unwrap();

    // Insert some test data
    old_db
        .simple_query(
            "
            INSERT INTO users (id, name) VALUES
                (1, 'John Doe'),
                (2, 'Jane Doe');
            ",
        )
        .unwrap();

    // Run second migration
    reshape.migrate(second_migrations.clone()).unwrap();
    new_db
        .simple_query(&reshape::schema_query_for_migration(
            &second_migrations.last().unwrap().name,
        ))
        .unwrap();

    // Check that existing values can be queried using new column name
    let expected = vec!["John Doe", "Jane Doe"];
    assert!(new_db
        .query("SELECT full_name FROM users ORDER BY id", &[],)
        .unwrap()
        .iter()
        .map(|row| row.get::<_, String>("full_name"))
        .eq(expected));

    reshape.complete().unwrap();
    common::assert_cleaned_up(&mut new_db);
}

#[test]
fn alter_column_multiple() {
    let (mut reshape, mut old_db, mut new_db) = common::setup();

    let create_users_table = Migration::new("create_user_table", None).with_action(
        CreateTableBuilder::default()
            .name("users")
            .primary_key(vec!["id".to_string()])
            .columns(vec![
                ColumnBuilder::default()
                    .name("id")
                    .data_type("INTEGER")
                    .build()
                    .unwrap(),
                ColumnBuilder::default()
                    .name("counter")
                    .data_type("INTEGER")
                    .nullable(false)
                    .build()
                    .unwrap(),
            ])
            .build()
            .unwrap(),
    );
    let increment_counter_twice = Migration::new("increment_counter_twice", None)
        .with_action(AlterColumn {
            table: "users".to_string(),
            column: "counter".to_string(),
            up: Some("counter + 1".to_string()),
            down: Some("counter - 1".to_string()),
            changes: ColumnChanges {
                data_type: None,
                nullable: None,
                name: None,
                default: None,
            },
        })
        .with_action(AlterColumn {
            table: "users".to_string(),
            column: "counter".to_string(),
            up: Some("counter + 1".to_string()),
            down: Some("counter - 1".to_string()),
            changes: ColumnChanges {
                data_type: None,
                nullable: None,
                name: None,
                default: None,
            },
        });

    let first_migrations = vec![create_users_table.clone()];
    let second_migrations = vec![create_users_table.clone(), increment_counter_twice.clone()];

    // Run first migration, should automatically finish
    reshape.migrate(first_migrations.clone()).unwrap();

    // Update search paths
    old_db
        .simple_query(&reshape::schema_query_for_migration(
            &first_migrations.last().unwrap().name,
        ))
        .unwrap();

    // Insert some test data
    old_db
        .simple_query(
            "
            INSERT INTO users (id, counter) VALUES
                (1, 0),
                (2, 100);
            ",
        )
        .unwrap();

    // Run second migration
    reshape.migrate(second_migrations.clone()).unwrap();
    new_db
        .simple_query(&reshape::schema_query_for_migration(
            &second_migrations.last().unwrap().name,
        ))
        .unwrap();

    // Check that the existing data has been updated
    let expected = vec![2, 102];
    let results: Vec<i32> = new_db
        .query("SELECT counter FROM users ORDER BY id", &[])
        .unwrap()
        .iter()
        .map(|row| row.get::<_, i32>("counter"))
        .collect();
    assert_eq!(expected, results);

    // Update data using old schema and make sure it was updated for the new schema
    old_db
        .query("UPDATE users SET counter = 50 WHERE id = 1", &[])
        .unwrap();
    let result: i32 = new_db
        .query("SELECT counter FROM users WHERE id = 1", &[])
        .unwrap()
        .iter()
        .map(|row| row.get("counter"))
        .nth(0)
        .unwrap();
    assert_eq!(52, result);

    // Update data using new schema and make sure it was updated for the old schema
    new_db
        .query("UPDATE users SET counter = 50 WHERE id = 1", &[])
        .unwrap();
    let result: i32 = old_db
        .query("SELECT counter FROM users WHERE id = 1", &[])
        .unwrap()
        .iter()
        .map(|row| row.get("counter"))
        .nth(0)
        .unwrap();
    assert_eq!(48, result);

    reshape.complete().unwrap();
    common::assert_cleaned_up(&mut new_db);
}

#[test]
fn alter_column_default() {
    let (mut reshape, mut old_db, mut new_db) = common::setup();

    let create_users_table = Migration::new("create_user_table", None).with_action(
        CreateTableBuilder::default()
            .name("users")
            .primary_key(vec!["id".to_string()])
            .columns(vec![
                ColumnBuilder::default()
                    .name("id")
                    .data_type("INTEGER")
                    .build()
                    .unwrap(),
                ColumnBuilder::default()
                    .name("name")
                    .data_type("TEXT")
                    .nullable(false)
                    .default_value("'DEFAULT'")
                    .build()
                    .unwrap(),
            ])
            .build()
            .unwrap(),
    );
    let change_name_default =
        Migration::new("change_name_default", None).with_action(AlterColumn {
            table: "users".to_string(),
            column: "name".to_string(),
            up: None,
            down: None,
            changes: ColumnChanges {
                data_type: None,
                nullable: None,
                name: None,
                default: Some("'NEW DEFAULT'".to_string()),
            },
        });

    let first_migrations = vec![create_users_table.clone()];
    let second_migrations = vec![create_users_table.clone(), change_name_default.clone()];

    // Run first migration, should automatically finish
    reshape.migrate(first_migrations.clone()).unwrap();

    // Update search paths
    old_db
        .simple_query(&reshape::schema_query_for_migration(
            &first_migrations.last().unwrap().name,
        ))
        .unwrap();

    // Insert a test user
    old_db
        .simple_query(
            "
            INSERT INTO users (id) VALUES (1)
            ",
        )
        .unwrap();

    // Run second migration
    reshape.migrate(second_migrations.clone()).unwrap();
    new_db
        .simple_query(&reshape::schema_query_for_migration(
            &second_migrations.last().unwrap().name,
        ))
        .unwrap();

    // Check that the existing users has the old default value
    let expected = vec!["DEFAULT"];
    assert!(new_db
        .query("SELECT name FROM users", &[],)
        .unwrap()
        .iter()
        .map(|row| row.get::<_, String>("name"))
        .eq(expected));

    // Insert data using old schema and make those get the old default value
    old_db
        .simple_query("INSERT INTO users (id) VALUES (2)")
        .unwrap();
    let result = new_db
        .query_one("SELECT name from users WHERE id = 2", &[])
        .unwrap();
    assert_eq!("DEFAULT", result.get::<_, &str>("name"));

    // Insert data using new schema and make sure it gets the new default value
    new_db
        .simple_query("INSERT INTO users (id) VALUES (3)")
        .unwrap();
    let result = old_db
        .query_one("SELECT name from users WHERE id = 3", &[])
        .unwrap();
    assert_eq!("NEW DEFAULT", result.get::<_, &str>("name"));

    reshape.complete().unwrap();
    common::assert_cleaned_up(&mut new_db);
}

#[test]
fn alter_column_with_index() {
    let (mut reshape, mut db, _) = common::setup();

    let create_users_table = Migration::new("create_user_table", None)
        .with_action(
            CreateTableBuilder::default()
                .name("users")
                .primary_key(vec!["id".to_string()])
                .columns(vec![
                    ColumnBuilder::default()
                        .name("id")
                        .data_type("INTEGER")
                        .build()
                        .unwrap(),
                    ColumnBuilder::default()
                        .name("first_name")
                        .data_type("TEXT")
                        .build()
                        .unwrap(),
                    ColumnBuilder::default()
                        .name("last_name")
                        .data_type("TEXT")
                        .build()
                        .unwrap(),
                ])
                .build()
                .unwrap(),
        )
        .with_action(AddIndex {
            table: "users".to_string(),
            name: "users_name_idx".to_string(),
            columns: vec!["first_name".to_string(), "last_name".to_string()],
            unique: false,
        });
    let uppercase_name = Migration::new("uppercase_name", None).with_action(AlterColumn {
        table: "users".to_string(),
        column: "last_name".to_string(),
        up: Some("UPPER(last_name)".to_string()),
        down: Some("LOWER(last_name)".to_string()),
        changes: ColumnChanges {
            data_type: None,
            nullable: None,
            name: None,
            default: None,
        },
    });

    let first_migrations = vec![create_users_table.clone()];
    let second_migrations = vec![create_users_table.clone(), uppercase_name.clone()];

    // Run first migration, should automatically finish
    reshape.migrate(first_migrations.clone()).unwrap();

    // Run second migration
    reshape.migrate(second_migrations.clone()).unwrap();
    db.simple_query(&reshape::schema_query_for_migration(
        &second_migrations.last().unwrap().name,
    ))
    .unwrap();

    // Complete the second migration which should replace the existing column
    // with the temporary one
    reshape.complete().unwrap();

    // Make sure index still exists
    let result: i64 = db
        .query(
            "
			SELECT COUNT(*)
			FROM pg_catalog.pg_index
			JOIN pg_catalog.pg_class ON pg_index.indexrelid = pg_class.oid
			WHERE pg_class.relname = 'users_name_idx'
			",
            &[],
        )
        .unwrap()
        .first()
        .map(|row| row.get(0))
        .unwrap();
    assert_eq!(1, result, "expected index to still exist");

    common::assert_cleaned_up(&mut db);
}
