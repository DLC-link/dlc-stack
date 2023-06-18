use crate::DbPool;
use actix_web::web;
use actix_web::web::{Data, Json, Path};
use actix_web::{delete, get, post, put, HttpResponse, Responder};
use dlc_storage_common::models::{ContractRequestParams, NewContract, UpdateContract};
use dlc_storage_reader;
use dlc_storage_writer;
use log::info;

#[get("/contracts")]
pub async fn get_contracts(
    pool: Data<DbPool>,
    request_params: web::Query<ContractRequestParams>,
) -> impl Responder {
    let key = request_params.key.clone();
    if key.is_empty() {
        return HttpResponse::BadRequest().body("Key is required");
    }
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let contracts =
        dlc_storage_reader::get_contracts(&mut conn, request_params.into_inner()).unwrap();
    HttpResponse::Ok().json(contracts)
}

#[get("/contract/{uuid}")]
pub async fn get_contract(pool: Data<DbPool>, uuid: Path<String>) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let result = dlc_storage_reader::get_contract(&mut conn, &uuid.clone());
    info!("get_contract called for uuid: {}", &uuid.into_inner());
    match result {
        Ok(contract) => HttpResponse::Ok().json(contract),
        Err(diesel::result::Error::NotFound) => HttpResponse::NotFound().body("Contract not found"),
        Err(_) => HttpResponse::InternalServerError().body("Internal server error"),
    }
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
