use crate::DbPool;
use actix_web::web::{Data, Json, Path};
use actix_web::{delete, get, post, put, HttpResponse, Responder};
use dlc_storage_common::models::{NewContract, UpdateContract};
use dlc_storage_reader;
use dlc_storage_writer;

#[get("/contracts")]
pub async fn get_contracts(pool: Data<DbPool>) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let contracts = dlc_storage_reader::get_contracts(&mut conn).unwrap();
    HttpResponse::Ok().json(contracts)
}

#[get("/contracts/{uuid}")]
pub async fn get_contract(pool: Data<DbPool>, uuid: Path<String>) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let result = dlc_storage_reader::get_contract(&mut conn, &uuid.into_inner());
    match result {
        Ok(contract) => HttpResponse::Ok().json(contract),
        Err(diesel::result::Error::NotFound) => HttpResponse::NotFound().body("Contract not found"),
        Err(_) => HttpResponse::InternalServerError().body("Internal server error"),
    }
}

#[get("/contracts/state/{state}")]
pub async fn get_contracts_by_state(pool: Data<DbPool>, state: Path<String>) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let contracts =
        dlc_storage_reader::get_contracts_by_state(&mut conn, &state.into_inner()).unwrap();
    HttpResponse::Ok().json(contracts)
}

#[post("/contracts")]
pub async fn create_contract(pool: Data<DbPool>, contract: Json<NewContract>) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let contract = dlc_storage_writer::create_contract(&mut conn, contract.into_inner()).unwrap();
    HttpResponse::Ok().json(contract)
}

#[put("/contracts/{uuid}")]
pub async fn update_contract(
    pool: Data<DbPool>,
    uuid: Path<String>,
    contract: Json<UpdateContract>,
) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let contract =
        dlc_storage_writer::update_contract(&mut conn, &uuid.into_inner(), contract.into_inner())
            .unwrap();
    HttpResponse::Ok().json(contract)
}

#[delete("/contracts/{uuid}")]
pub async fn delete_contract(pool: Data<DbPool>, uuid: Path<String>) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let num_deleted = dlc_storage_writer::delete_contract(&mut conn, &uuid.into_inner()).unwrap();
    HttpResponse::Ok().json(num_deleted)
}

#[delete("/contracts")]
pub async fn delete_contracts(pool: Data<DbPool>) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let num_deleted = dlc_storage_writer::delete_contracts(&mut conn).unwrap();
    HttpResponse::Ok().json(num_deleted)
}
