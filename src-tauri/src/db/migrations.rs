use super::Migration;

pub static PRODUCT: [Migration; 1] = [Migration {
    version: 1,
    name: "initial",
    sql: include_str!("../../migrations/0001_initial.sql"),
}];
