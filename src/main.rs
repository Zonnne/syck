use actix_files as fs;
use actix_web::dev::Server;
use actix_web::{
    get, http::header::ContentType, web, App, Error, HttpRequest, HttpResponse, HttpServer, Result,
};
use web_view::*;
use futures::StreamExt;
use futures::{future::ok, stream::once};
use serde::{Deserialize, Serialize};
use std::fs as fss;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use uuid::Uuid;
extern crate ifaces;
use std::net::SocketAddr;
extern crate qrcode_generator;
use actix_multipart::Multipart;
use qrcode_generator::QrCodeEcc;
use urlencoding::decode;
#[derive(Deserialize)]
struct Info {
    raw: String,
}
#[derive(Serialize)]
struct Address {
    url: String,
}
fn get_file_as_byte_vec(filename: &PathBuf) -> Vec<u8> {
    let mut f = File::open(filename).expect("no file found");
    let metadata = fss::metadata(filename).expect("unable to read metadata");
    let mut buffer = vec![0; metadata.len() as usize];
    f.read(&mut buffer).expect("buffer overflow");

    buffer
}
async fn file(mut body: Multipart) -> Result<HttpResponse, Error> {
    let item = body.next().await.unwrap();
    let mut field = item?;
    let or_filename = field.content_disposition().get_filename().unwrap();
    let v: Vec<&str> = or_filename.split(".").collect();

    let exe = std::env::temp_dir();
    let mut filename = Uuid::new_v4().to_hyphenated().to_string();
    filename.push_str(".");
    filename.push_str(v[v.len() - 1]);
    let upload = exe.join("uploads");
    fss::create_dir_all(upload)?;
    let fullpath = Path::new("uploads").join(filename);
    let mut st = String::from("/");
    st.push_str(fullpath.to_str().unwrap());
    let mut file = File::create(exe.join(fullpath)).unwrap();
    while let Some(chunk) = field.next().await {
        let data = chunk.unwrap();
        file.write_all(&data)?;
    }
    Ok(HttpResponse::Ok().json(Address { url: st }))
}
async fn text(info: web::Json<Info>) -> Result<HttpResponse, Error> {
    let exe = std::env::temp_dir();
    let mut filename = Uuid::new_v4().to_hyphenated().to_string();
    filename.push_str(".txt");
    let upload = exe.join("uploads");
    fss::create_dir_all(upload)?;
    let fullpath = Path::new("uploads").join(filename);
    let mut st = String::from("/");
    st.push_str(fullpath.to_str().unwrap());
    let mut file = File::create(exe.join(fullpath)).unwrap();
    file.write_all((info.raw).as_bytes())?;
    Ok(HttpResponse::Ok().json(Address { url: st }))
}
#[get("/uploads/{path}")]
async fn upload(req: HttpRequest) -> Result<HttpResponse, Error> {
    let path = req.match_info().query("path");
    let target = get_upload().join(path);
    println!("{}", target.display());
    let ss = get_file_as_byte_vec(&target);
    let body = once(ok::<_, Error>(web::Bytes::from(ss)));
    Ok(HttpResponse::Ok()
        .insert_header(("Content-Description", "File Transfer"))
        .insert_header(("Content-Transfer-Encoding", "binary"))
        .insert_header((
            "Content-Disposition",
            "attachment; filename=".to_owned() + path,
        ))
        .insert_header(ContentType::octet_stream())
        .streaming(body))
}
#[derive(Serialize)]
struct Addr {
    addresses: Vec<String>,
}
#[get("/api/v1/addresses")]
async fn address() -> Result<HttpResponse, Error> {
    let mut addr = Addr {
        addresses: Vec::new(),
    };
    for iface in ifaces::Interface::get_all().unwrap().into_iter() {
        let s = iface.addr.unwrap();
        match s {
            SocketAddr::V4(s) => {
                let with0 = s.to_string();
                let v: Vec<&str> = with0.split(':').collect();
                addr.addresses.push(v[0].to_string());
            }
            _ => (),
        }
    }
    Ok(HttpResponse::Ok().json(addr))
}
fn get_upload() -> PathBuf {
    let exe = std::env::temp_dir().join("uploads");
    println!("{}", exe.display());
    exe
}
#[get("/api/v1/qrcodes")]
async fn qr2download(req: HttpRequest) -> Result<HttpResponse, Error> {
    let s = req.query_string();
    let path: Vec<&str> = s.split("content=").collect();
    let url = decode(path[1]).unwrap().into_owned();
    let url = url.replace("%2F", "/");

    let data = qrcode_generator::to_png_to_vec_from_str(url, QrCodeEcc::Medium, 256).unwrap();
    Ok(HttpResponse::Ok()
        .insert_header(ContentType::png())
        .body(data))
}
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let server = start_server().await?;
    tokio::join!(server, open_web_app());
    Ok(())
}
async fn start_server() -> Result<Server, std::io::Error> {
    let server = HttpServer::new(|| {
        App::new()
            .route("/api/v1/texts", web::post().to(text))
            .route("/api/v1/files", web::post().to(file))
            .service(qr2download)
            .service(address)
            .service(upload)
            .service(fs::Files::new("/", "./frontend/dist").index_file("index.html"))
    })
    .bind(("127.0.0.1", 8080))?
    .run();
    Ok(server)
}
async fn open_web_app() -> Result<(), web_view::Error> {
    let web = web_view::builder()
        .title("syck")
        .content(Content::Url(format!("http://127.0.0.1:{}", 8080)))
        .size(600, 600)
        .resizable(false)
        .debug(false)
        .user_data(())
        .invoke_handler(|_webview, _arg| Ok(()))
        .run().unwrap();
        Ok(web)
}
