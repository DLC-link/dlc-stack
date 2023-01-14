use diesel::PgConnection;
use dlc_storage_common;
use dlc_storage_common::models::{
    Contract, Event, NewContract, NewEvent, UpdateContract, UpdateEvent,
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
    cuuid: &str,
    contract: UpdateContract,
) -> Result<usize, diesel::result::Error> {
    return dlc_storage_common::update_contract(conn, cuuid, contract);
}

pub fn delete_contract(
    conn: &mut PgConnection,
    cuuid: &str,
) -> Result<usize, diesel::result::Error> {
    return dlc_storage_common::delete_contract(conn, cuuid);
}

pub fn delete_contracts(conn: &mut PgConnection) -> Result<usize, diesel::result::Error> {
    return dlc_storage_common::delete_all_contracts(conn);
}

pub fn create_event(
    conn: &mut PgConnection,
    event: NewEvent,
) -> Result<Event, diesel::result::Error> {
    return dlc_storage_common::create_event(conn, event);
}

pub fn update_event(
    conn: &mut PgConnection,
    eid: &str,
    event: UpdateEvent,
) -> Result<usize, diesel::result::Error> {
    return dlc_storage_common::update_event(conn, eid, event);
}

pub fn delete_event(conn: &mut PgConnection, eid: &str) -> Result<usize, diesel::result::Error> {
    return dlc_storage_common::delete_contract(conn, eid);
}

pub fn delete_events(conn: &mut PgConnection) -> Result<usize, diesel::result::Error> {
    return dlc_storage_common::delete_all_events(conn);
}
