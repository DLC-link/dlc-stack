pub mod models;
pub mod schema;

use crate::models::*;
use diesel::expression_methods::ExpressionMethods;
use diesel::query_dsl::QueryDsl;
use diesel::RunQueryDsl;
use diesel::{r2d2::Error, PgConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub fn run_migrations(conn: &mut PgConnection) -> Result<(), Error> {
    conn.run_pending_migrations(MIGRATIONS).unwrap();
    Ok(())
}

pub fn get_contracts(conn: &mut PgConnection) -> Result<Vec<Contract>, diesel::result::Error> {
    use crate::schema::contracts::dsl::*;
    let results = contracts.load::<Contract>(conn)?;
    Ok(results)
}

pub fn get_contract(
    conn: &mut PgConnection,
    cuuid: &str,
) -> Result<Contract, diesel::result::Error> {
    use crate::schema::contracts::dsl::*;
    let result = contracts.filter(uuid.eq(cuuid)).first(conn)?;
    Ok(result)
}

pub fn get_contracts_by_state(
    conn: &mut PgConnection,
    state_input: &str,
) -> Result<Vec<Contract>, diesel::result::Error> {
    use crate::schema::contracts::dsl::*;
    let results = contracts
        .filter(state.eq(state_input))
        .load::<Contract>(conn)?;
    Ok(results)
}

pub fn create_contract(
    conn: &mut PgConnection,
    contract: NewContract,
) -> Result<Contract, diesel::result::Error> {
    use crate::schema::contracts::dsl::*;
    let result = diesel::insert_into(contracts)
        .values(&contract)
        .get_result(conn)?;
    Ok(result)
}

pub fn delete_contract(
    conn: &mut PgConnection,
    cuuid: &str,
) -> Result<usize, diesel::result::Error> {
    use crate::schema::contracts::dsl::*;
    let num_deleted = diesel::delete(contracts.filter(uuid.eq(cuuid))).execute(conn)?;
    Ok(num_deleted)
}

pub fn delete_all_contracts(conn: &mut PgConnection) -> Result<usize, diesel::result::Error> {
    use crate::schema::contracts::dsl::*;
    let num_deleted = diesel::delete(contracts).execute(conn)?;
    Ok(num_deleted)
}

pub fn update_contract(
    conn: &mut PgConnection,
    cuuid: &str,
    contract: UpdateContract,
) -> Result<usize, diesel::result::Error> {
    use crate::schema::contracts::dsl::*;
    let num_updated = diesel::update(contracts.filter(uuid.eq(cuuid)))
        .set(&contract)
        .execute(conn)?;
    Ok(num_updated)
}

pub fn create_event(
    conn: &mut PgConnection,
    event: NewEvent,
) -> Result<Event, diesel::result::Error> {
    use crate::schema::events::dsl::*;
    let result = diesel::insert_into(events)
        .values(&event)
        .get_result(conn)?;
    Ok(result)
}

pub fn update_event(
    conn: &mut PgConnection,
    eid: &str,
    event: UpdateEvent,
) -> Result<usize, diesel::result::Error> {
    use crate::schema::events::dsl::*;
    let num_updated = diesel::update(events.filter(event_id.eq(eid)))
        .set(&event)
        .execute(conn)?;
    Ok(num_updated)
}

pub fn get_event(conn: &mut PgConnection, eid: &str) -> Result<Event, diesel::result::Error> {
    use crate::schema::events::dsl::*;
    let result = events.filter(event_id.eq(eid)).first(conn)?;
    Ok(result)
}

pub fn get_all_events(conn: &mut PgConnection) -> Result<Vec<Event>, diesel::result::Error> {
    use crate::schema::events::dsl::*;
    let results = events.load::<Event>(conn)?;
    Ok(results)
}

pub fn delete_event(conn: &mut PgConnection, eid: &str) -> Result<usize, diesel::result::Error> {
    use crate::schema::events::dsl::*;
    let num_deleted = diesel::delete(events.filter(event_id.eq(eid))).execute(conn)?;
    Ok(num_deleted)
}

pub fn delete_all_events(conn: &mut PgConnection) -> Result<usize, diesel::result::Error> {
    use crate::schema::events::dsl::*;
    let num_deleted = diesel::delete(events).execute(conn)?;
    Ok(num_deleted)
}
