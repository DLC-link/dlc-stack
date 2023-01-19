use diesel::PgConnection;
use dlc_storage_common;
use dlc_storage_common::models::Contract;
use dlc_storage_common::models::Event;

pub fn get_contracts(conn: &mut PgConnection) -> Result<Vec<Contract>, diesel::result::Error> {
    return dlc_storage_common::get_contracts(conn);
}

pub fn get_contract(
    conn: &mut PgConnection,
    cuuid: &str,
) -> Result<Contract, diesel::result::Error> {
    return dlc_storage_common::get_contract(conn, cuuid);
}

pub fn get_contracts_by_state(
    conn: &mut PgConnection,
    state_input: &str,
) -> Result<Vec<Contract>, diesel::result::Error> {
    return dlc_storage_common::get_contracts_by_state(conn, state_input);
}

pub fn get_events(conn: &mut PgConnection) -> Result<Vec<Event>, diesel::result::Error> {
    return dlc_storage_common::get_all_events(conn);
}

pub fn get_event(conn: &mut PgConnection, cuuid: &str) -> Result<Event, diesel::result::Error> {
    return dlc_storage_common::get_event(conn, cuuid);
}
