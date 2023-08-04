use crate::DbPool;
use actix_web::web;
use actix_web::web::{Data, Json, Path};
use actix_web::{delete, get, post, put, HttpResponse, Responder};
use dlc_storage_common::models::{
    ContractRequestParams, DeleteContract, NewContract, UpdateContract,
};
use dlc_storage_reader;
use dlc_storage_writer;
use log::{info, warn};
use serde_json::json;

#[get("/contracts")]
pub async fn get_contracts(
    pool: Data<DbPool>,
    contract_params: web::Query<ContractRequestParams>,
) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    match dlc_storage_reader::get_contracts(&mut conn, contract_params.into_inner()) {
        Ok(contracts) => HttpResponse::Ok().json(contracts),
        Err(e) => {
            warn!("Error getting contracts: {:?}", e);
            HttpResponse::BadRequest().body(e.to_string())
        }
    }
}

#[post("/contracts")]
pub async fn create_contract(
    pool: Data<DbPool>,
    contract_params: Json<NewContract>,
) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    match dlc_storage_writer::create_contract(&mut conn, contract_params.into_inner()) {
        Ok(contract) => {
            info!("Created contract: {:?}", contract);
            HttpResponse::Ok().json(contract)
        }
        Err(e) => {
            warn!("Error creating contract: {:?}", e);
            HttpResponse::BadRequest().body(e.to_string())
        }
    }
}

#[put("/contracts")]
pub async fn update_contract(
    pool: Data<DbPool>,
    contract_params: Json<UpdateContract>,
) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let num_updated =
        match dlc_storage_writer::update_contract(&mut conn, contract_params.into_inner()) {
            Ok(num_updated) => num_updated,
            Err(e) => {
                warn!("Error updating contract: {:?}", e);
                return HttpResponse::BadRequest().body(e.to_string());
            }
        };
    match num_updated {
        0 => HttpResponse::NotFound().body("No contract found"),
        _ => HttpResponse::Ok().json(json!({ "effected_num": num_updated })),
    }
}

#[delete("/contract")]
pub async fn delete_contract(
    pool: Data<DbPool>,
    contract_params: Json<DeleteContract>,
) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let num_deleted =
        match dlc_storage_writer::delete_contract(&mut conn, contract_params.into_inner()) {
            Ok(num_deleted) => num_deleted,
            Err(e) => {
                warn!("Error deleting contract: {:?}", e);
                return HttpResponse::BadRequest().body(e.to_string());
            }
        };
    match num_deleted {
        0 => HttpResponse::NotFound().body("No contract found"),
        _ => HttpResponse::Ok().json(json!({ "effected_num": num_deleted })),
    }
}

#[delete("/contracts/{ckey}")]
pub async fn delete_contracts(pool: Data<DbPool>, ckey: Path<String>) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let num_deleted = dlc_storage_writer::delete_all_contracts(&mut conn, &ckey).unwrap();
    HttpResponse::Ok().json(json!({ "effected_num": num_deleted }))
}
