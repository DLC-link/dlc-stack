use diesel::PgConnection;
use dlc_storage_common::models::Contract;
use dlc_storage_common::models::ContractRequestParams;
use dlc_storage_common::models::Event;
use dlc_storage_common::models::EventRequestParams;

pub fn get_contracts(
    conn: &mut PgConnection,
    contract_params: ContractRequestParams,
) -> Result<Vec<Contract>, diesel::result::Error> {
    dlc_storage_common::get_contracts(conn, contract_params)
}

pub fn get_events(
    conn: &mut PgConnection,
    event_params: EventRequestParams,
) -> Result<Vec<Event>, diesel::result::Error> {
    dlc_storage_common::get_events(conn, event_params)
}
