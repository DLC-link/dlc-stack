use diesel::PgConnection;
use dlc_storage_common;
use dlc_storage_common::models::{
    Contract, DeleteContract, DeleteEvent, Event, NewContract, NewEvent, UpdateContract,
    UpdateEvent,
};

pub fn apply_migrations(conn: &mut PgConnection) {
    let _ = dlc_storage_common::run_migrations(conn);
}

pub fn create_contract(
    conn: &mut PgConnection,
    contract: NewContract,
) -> Result<Contract, diesel::result::Error> {
    return dlc_storage_common::create_contract(conn, contract);
}

pub fn update_contract(
    conn: &mut PgConnection,
    contract: UpdateContract,
) -> Result<usize, diesel::result::Error> {
    return dlc_storage_common::update_contract(conn, contract);
}

pub fn delete_contract(
    conn: &mut PgConnection,
    contract: DeleteContract,
) -> Result<usize, diesel::result::Error> {
    return dlc_storage_common::delete_contract(conn, contract);
}

pub fn delete_all_contracts(
    conn: &mut PgConnection,
    ckey: &str,
) -> Result<usize, diesel::result::Error> {
    return dlc_storage_common::delete_all_contracts(conn, ckey);
}

pub fn create_event(
    conn: &mut PgConnection,
    event: NewEvent,
) -> Result<Event, diesel::result::Error> {
    return dlc_storage_common::create_event(conn, event);
}

pub fn update_event(
    conn: &mut PgConnection,
    event: UpdateEvent,
) -> Result<usize, diesel::result::Error> {
    return dlc_storage_common::update_event(conn, event);
}

pub fn delete_event(
    conn: &mut PgConnection,
    event: DeleteEvent,
) -> Result<usize, diesel::result::Error> {
    return dlc_storage_common::delete_event(conn, event);
}

pub fn delete_events(conn: &mut PgConnection, ckey: &str) -> Result<usize, diesel::result::Error> {
    return dlc_storage_common::delete_all_events(conn, ckey);
}
