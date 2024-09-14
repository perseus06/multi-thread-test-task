use actix_web::{web, App, HttpServer, HttpResponse, Responder, HttpRequest};
use actix_multipart::Multipart;
use futures::StreamExt;
use mongodb::{Client, Collection, Cursor};
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use uuid::Uuid;
use tokio::fs as async_fs;

#[derive(Serialize, Deserialize, Debug, Clone)] // Added Clone here
struct FileMeta {
    file_id: String,
    file_name: String,
    chunks: Vec<String>,
}

// Initialize MongoDB client and collection
async fn init_db() -> Collection<FileMeta> {
    let mongo_uri = "mongodb://localhost:27017";
    let client = Client::with_uri_str(&mongo_uri).await.unwrap();
    let db = client.database("file_db");
    db.collection::<FileMeta>("files")
}

async fn upload_file(mut payload: Multipart) -> impl Responder {
    println!("Starting file upload...");
    let mut file_chunks = vec![];
    let file_id = Uuid::new_v4().to_string(); // Convert UUID to String
    let uploads_path = "./uploads";

    std::fs::create_dir_all(uploads_path).unwrap();
    println!("Created uploads directory");

    let mut filename = "file".to_string(); // Default filename

    while let Some(field_result) = payload.next().await {
        let mut field = match field_result {
            Ok(field) => field,
            Err(e) => {
                eprintln!("Error while processing multipart field: {}", e);
                continue;
            }
        };

        let content_disposition = field.content_disposition();
        let name = content_disposition.get_filename();
        if let Some(file_name) = name {
            filename = file_name.to_string();
        }

        let chunk_file_path = format!("{}/{}_chunk", uploads_path, file_id);
        let mut chunk_file = match File::create(&chunk_file_path) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("Error creating chunk file: {}", e);
                continue;
            }
        };

        while let Some(chunk_result) = field.next().await {
            let data = match chunk_result {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("Error reading chunk: {}", e);
                    continue;
                }
            };
            if let Err(e) = chunk_file.write_all(&data) {
                eprintln!("Error writing chunk data: {}", e);
            }
            file_chunks.push(chunk_file_path.clone());
        }
    }

    let db = init_db().await;
    let new_file = FileMeta {
        file_id: file_id.clone(),
        file_name: filename,
        chunks: file_chunks,
    };

    match db.insert_one(new_file, None).await {
        Ok(_) => println!("File metadata inserted into database"),
        Err(e) => eprintln!("Error inserting file metadata into database: {}", e),
    }

    HttpResponse::Ok().json(format!("File uploaded successfully with id: {}", file_id))
}

async fn get_uploaded_files() -> impl Responder {
    let db = init_db().await;
    let mut cursor: Cursor<FileMeta> = db.find(None, None).await.unwrap();
    let mut files = Vec::new();

    while let Some(doc) = cursor.next().await {
        match doc {
            Ok(file_meta) => files.push(file_meta),
            Err(_) => continue, // Handle error as needed
        }
    }

    HttpResponse::Ok().json(files)
}

async fn download_file(req: HttpRequest) -> impl Responder {
    let file_id_str = req.match_info().get("id").unwrap_or_default();
    let file_id = file_id_str.to_string(); // Use the file ID as a string

    let db = init_db().await;

    let filter = doc! {
        "file_id": file_id.clone() // Clone file_id for the filter
    };

    let file_meta = db.find_one(filter, None).await.unwrap();

    match file_meta {
        Some(file_meta) => {
            let uploads_path = "./uploads";
            let merged_file_path = format!("{}/{}_merged", uploads_path, file_id.clone()); // Clone file_id for the path

            let mut merged_file = File::create(&merged_file_path).unwrap();

            for chunk_path in file_meta.chunks {
                let mut chunk_file = File::open(chunk_path).unwrap();
                let mut buffer = Vec::new();
                chunk_file.read_to_end(&mut buffer).unwrap();
                merged_file.write_all(&buffer).unwrap();
            }

            let file = async_fs::read(merged_file_path).await.unwrap();
            let file_name = file_meta.file_name; // Get the original filename

            HttpResponse::Ok()
                .content_type("application/octet-stream") // You might want to set this dynamically
                .insert_header(("Content-Disposition", format!("attachment; filename=\"{}\"", file_name)))
                .body(file)
        },
        None => HttpResponse::NotFound().body("File not found"),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(web::resource("/upload").route(web::post().to(upload_file)))
            .service(web::resource("/files").route(web::get().to(get_uploaded_files)))
            .service(web::resource("/download/{id}").route(web::get().to(download_file)))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
