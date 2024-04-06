use crate::{
    bootloader::{Command, BOOTLOADER_CHANNEL},
    built_info,
    network::SERVER_THREADS,
    utils::{
        debug::{get_functions, get_variables},
        functions::call_function,
        logger::LOGGER_CHANNEL,
        stop,
    },
};
use cyw43::NetDriver;
use defmt::info;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_net::Stack;
use embassy_time::Duration;
use heapless::Vec;
use picoserve::{
    extract::Form,
    io::{Read, Write},
    response::{
        json::Json,
        ws::{Message, SocketRx, SocketTx, WebSocketCallback},
        File, WebSocketUpgrade,
    },
    routing::{get, post, PathRouter},
    Config as ServeConfig, Router, Timeouts,
};
use serde::Serialize;
use static_cell::make_static;

type AppRouter = impl PathRouter;

struct LoggerHandler;

impl WebSocketCallback for LoggerHandler {
    async fn run<R: Read, W: Write<Error = R::Error>>(
        self,
        mut rx: SocketRx<R>,
        mut tx: SocketTx<W>,
    ) -> core::result::Result<(), W::Error> {
        let mut message_buffer = [0; 128];

        loop {
            match select(
                LOGGER_CHANNEL.receive(),
                rx.next_message(&mut message_buffer),
            )
            .await
            {
                Either::First(byte) => {
                    let mut bytes = [0; 128];
                    let mut bytes_end = 128;

                    bytes[0] = byte;

                    for (index, item) in bytes.iter_mut().enumerate().skip(1) {
                        if let Ok(next_byte) = LOGGER_CHANNEL.try_receive() {
                            *item = next_byte;
                        } else {
                            bytes_end = index;
                            break;
                        }
                    }

                    tx.send_binary(&bytes[..bytes_end]).await?;
                }
                Either::Second(message) => match message {
                    Ok(Message::Ping(ping)) => tx.send_pong(ping).await?,
                    Ok(Message::Close(_)) | Err(_) => break,
                    _ => {}
                },
            }
        }

        tx.close(None).await
    }
}

struct UpdateHandler;

impl WebSocketCallback for UpdateHandler {
    async fn run<R: Read, W: Write<Error = R::Error>>(
        self,
        mut rx: SocketRx<R>,
        mut tx: SocketTx<W>,
    ) -> core::result::Result<(), W::Error> {
        let mut message_buffer = [0; 128];

        let mut offset = 0;
        BOOTLOADER_CHANNEL.send(Command::Prepare).await;

        loop {
            match rx.next_message(&mut message_buffer).await {
                Ok(Message::Ping(ping)) => tx.send_pong(ping).await?,
                Ok(Message::Binary(message)) => {
                    BOOTLOADER_CHANNEL
                        .send(Command::WriteChunk {
                            buffer: Vec::from_slice(message).unwrap(),
                            offset,
                        })
                        .await;
                    offset += message.len() as u32;
                }
                Ok(Message::Close(_)) => {
                    BOOTLOADER_CHANNEL.send(Command::Commit).await;
                    BOOTLOADER_CHANNEL.send(Command::Restart).await;
                }
                Err(_) => break,
                _ => {}
            }
        }

        tx.close(None).await
    }
}

#[derive(Serialize)]
struct Info {
    name: &'static str,
    version: &'static str,
    rustc: &'static str,
    git_version: &'static str,
    git_dirty: bool,
    time: &'static str,
}

fn make_router() -> Router<AppRouter> {
    Router::new()
        .route(
            "/",
            get(|| async move { File::html(include_str!("index.html")) }),
        )
        .route(
            "/update",
            get(|upgrade: WebSocketUpgrade| async move {
                stop().await;
                upgrade.on_upgrade(UpdateHandler).with_protocol("messages")
            }),
        )
        .route(
            "/logs",
            get(|upgrade: WebSocketUpgrade| async move {
                upgrade.on_upgrade(LoggerHandler).with_protocol("messages")
            }),
        )
        .route(
            "/api/info",
            get(|| async move {
                Json(Info {
                    name: built_info::PKG_NAME,
                    version: built_info::PKG_VERSION,
                    rustc: built_info::RUSTC_VERSION,
                    git_version: built_info::GIT_VERSION.unwrap_or_default(),
                    git_dirty: built_info::GIT_DIRTY.unwrap_or(true),
                    time: built_info::BUILT_TIME_UTC,
                })
                .into_response()
            }),
        )
        .route(
            "/api/variables",
            get(|| async move { Json(get_variables().await).into_response() }),
        )
        .route(
            "/api/functions",
            get(|| async move { Json(get_functions().await).into_response() }),
        )
        .route(
            "/api/execute",
            post(|Form(func)| async move {
                call_function(func).await;
                "Ok"
            }),
        )
}

#[embassy_executor::task(pool_size = SERVER_THREADS)]
async fn server_task(
    id: usize,
    stack: &'static Stack<NetDriver<'static>>,
    router: &'static Router<AppRouter>,
    config: &'static ServeConfig<Duration>,
) {
    let mut tcp_rx_buffer = [0; 1024];
    let mut tcp_tx_buffer = [0; 1024];
    let mut http_buffer = [0; 2048];

    picoserve::listen_and_serve(
        id,
        router,
        config,
        stack,
        80,
        &mut tcp_rx_buffer,
        &mut tcp_tx_buffer,
        &mut http_buffer,
    )
    .await
}

pub async fn init(spawner: &Spawner, stack: &'static Stack<NetDriver<'static>>) {
    info!("Starting web server on port 80");

    let router = make_static!(make_router());

    let config = make_static!(ServeConfig::new(Timeouts {
        start_read_request: Some(Duration::from_secs(5)),
        read_request: Some(Duration::from_secs(1)),
        write: Some(Duration::from_secs(1)),
    })
    .keep_connection_alive());

    for i in 0..SERVER_THREADS {
        spawner.must_spawn(server_task(i, stack, router, config));
    }
}
