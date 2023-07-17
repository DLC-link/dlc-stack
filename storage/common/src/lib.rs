pub mod models;
pub mod schema;

use crate::models::*;
use diesel::expression_methods::ExpressionMethods;
use diesel::query_dsl::QueryDsl;
use diesel::{r2d2::Error, PgConnection};
use diesel::{BoolExpressionMethods, RunQueryDsl};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use log::warn;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub fn run_migrations(conn: &mut PgConnection) -> Result<(), Error> {
    conn.run_pending_migrations(MIGRATIONS).unwrap();
    Ok(())
}

pub fn get_contracts(
    conn: &mut PgConnection,
    contract_params: ContractRequestParams,
) -> Result<Vec<Contract>, diesel::result::Error> {
    use crate::schema::contracts::dsl::*;
    let mut query = contracts.into_boxed();
    query = query.filter(key.eq(contract_params.key));

    let cstate = contract_params.state.clone();
    if let Some(cstate) = contract_params.state {
        query = query.filter(state.eq(cstate));
    }

    let cuuid = contract_params.uuid.clone();
    if let Some(cuuid) = contract_params.uuid {
        query = query.filter(uuid.eq(cuuid));
    }

    let results = query.load::<Contract>(conn)?;
    Ok(results)
}

pub fn create_contract(
    conn: &mut PgConnection,
    contract: NewContract,
) -> Result<Contract, diesel::result::Error> {
    use crate::schema::contracts::dsl::*;
    match diesel::insert_into(contracts)
        .values(&contract)
        .get_result(conn)
    {
        Ok(result) => Ok(result),
        Err(e) => {
            warn!("Got an error creating contract: {:?}", e);
            Err(e)
        }
    }
}

pub fn delete_contract(
    conn: &mut PgConnection,
    contract: DeleteContract,
) -> Result<usize, diesel::result::Error> {
    use crate::schema::contracts::dsl::*;

    let num_deleted = diesel::delete(
        contracts
            .filter(uuid.eq(contract.uuid))
            .filter(key.eq(contract.key)),
    )
    .execute(conn)?;

    Ok(num_deleted)
}

pub fn delete_all_contracts(
    conn: &mut PgConnection,
    ckey: &str,
) -> Result<usize, diesel::result::Error> {
    use crate::schema::contracts::dsl::*;

    let num_deleted = diesel::delete(contracts.filter(key.eq(ckey))).execute(conn)?;

    Ok(num_deleted)
}

pub fn update_contract(
    conn: &mut PgConnection,
    contract: UpdateContract,
) -> Result<usize, diesel::result::Error> {
    use crate::schema::contracts::dsl::*;
    let update_contract = contract.clone();
    let num_updated = diesel::update(
        contracts
            .filter(uuid.eq(contract.uuid))
            .filter(key.eq(contract.key)),
    )
    .set(&update_contract)
    .execute(conn)?;
    Ok(num_updated)
}

pub fn create_event(
    conn: &mut PgConnection,
    event: NewEvent,
) -> Result<Event, diesel::result::Error> {
    use crate::schema::events::dsl::*;
    match diesel::insert_into(events).values(&event).get_result(conn) {
        Ok(event) => Ok(event),
        Err(e) => {
            warn!("Got an error creating event: {:?}", e);
            Err(e)
        }
    }
}

pub fn update_event(
    conn: &mut PgConnection,
    event: UpdateEvent,
) -> Result<usize, diesel::result::Error> {
    use crate::schema::events::dsl::*;
    let update_event = event.clone();
    match diesel::update(
        events
            .filter(event_id.eq(event.event_id))
            .filter(key.eq(event.key)),
    )
    .set(&update_event)
    .execute(conn)
    {
        Ok(num_updated) => Ok(num_updated),
        Err(e) => {
            warn!("Got an error creating event: {:?}", e);
            Err(e)
        }
    }
}

pub fn get_events(
    conn: &mut PgConnection,
    event: EventRequestParams,
) -> Result<Vec<Event>, diesel::result::Error> {
    use crate::schema::events::dsl::*;
    let mut query = events.into_boxed();
    query = query.filter(key.eq(event.key));

    let cuuid = event.event_id.clone();
    if let Some(cevent_id) = event.event_id {
        query = query.filter(event_id.eq(cevent_id));
    }

    let results = query.load::<Event>(conn)?;
    Ok(results)
}

pub fn delete_event(
    conn: &mut PgConnection,
    event: DeleteEvent,
) -> Result<usize, diesel::result::Error> {
    use crate::schema::events::dsl::*;
    let num_deleted = diesel::delete(
        events
            .filter(event_id.eq(event.event_id))
            .filter(key.eq(event.key)),
    )
    .execute(conn)?;
    Ok(num_deleted)
}

pub fn delete_all_events(
    conn: &mut PgConnection,
    ckey: &str,
) -> Result<usize, diesel::result::Error> {
    use crate::schema::events::dsl::*;
    let num_deleted = diesel::delete(events.filter(key.eq(ckey))).execute(conn)?;
    Ok(num_deleted)
}
