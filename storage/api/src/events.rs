use crate::DbPool;
use actix_web::web::{Data, Json, Path};
use actix_web::{delete, get, post, put, HttpResponse, Responder};
use dlc_storage_common::models::{NewEvent, UpdateEvent};
use dlc_storage_reader;
use dlc_storage_writer;

#[get("/events")]
pub async fn get_events(pool: Data<DbPool>) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let events = dlc_storage_reader::get_events(&mut conn).unwrap();
    HttpResponse::Ok().json(events)
}

#[get("/events/{uuid}")]
pub async fn get_event(pool: Data<DbPool>, uuid: Path<String>) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let result = dlc_storage_reader::get_event(&mut conn, &uuid.into_inner());
    match result {
        Ok(events) => HttpResponse::Ok().json(events),
        Err(diesel::result::Error::NotFound) => HttpResponse::NotFound().body("Event not found"),
        Err(_) => HttpResponse::InternalServerError().body("Internal server error"),
    }
}

#[post("/events")]
pub async fn create_event(pool: Data<DbPool>, event: Json<NewEvent>) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let events = dlc_storage_writer::create_event(&mut conn, event.into_inner()).unwrap();
    HttpResponse::Ok().json(events)
}

#[put("/events/{uuid}")]
pub async fn update_event(
    pool: Data<DbPool>,
    uuid: Path<String>,
    event: Json<UpdateEvent>,
) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let events =
        dlc_storage_writer::update_event(&mut conn, &uuid.into_inner(), event.into_inner())
            .unwrap();
    HttpResponse::Ok().json(events)
}

#[delete("/events/{uuid}")]
pub async fn delete_event(pool: Data<DbPool>, uuid: Path<String>) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let num_deleted = dlc_storage_writer::delete_event(&mut conn, &uuid.into_inner()).unwrap();
    HttpResponse::Ok().json(num_deleted)
}

#[delete("/events")]
pub async fn delete_events(pool: Data<DbPool>) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let num_deleted = dlc_storage_writer::delete_events(&mut conn).unwrap();
    HttpResponse::Ok().json(num_deleted)
}
