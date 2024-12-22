use std::{sync::Arc, time::{Duration, Instant}};

use actix_web::{get, web, Error, HttpRequest, HttpResponse};
use actix_ws::Message;
use futures::StreamExt;
use tokio::{select, sync::broadcast::*, time::interval};
use tracing::{debug, error, info, warn};


use crate::{dto::CardEvent, event::{Event, ParsedEvent}};

/// How often heartbeat pings are sent.
///
/// Should be half (or less) of the acceptable client timeout.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// How long before lack of client response causes a timeout.
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);


#[get("/ws-test")]
async fn ws(req: HttpRequest, body: web::Payload, sender: web::Data<Sender<CardEvent>>) -> Result<HttpResponse, Error> {
	let (response, session, msg_stream) = actix_ws::handle(&req, body)?;

	actix_rt::spawn(async move {
		handle_ws(session, msg_stream, sender.into_inner()).await;
	});

	Ok(response)
}


/// Broadcast text & binary messages received from a client, respond to ping messages, and monitor
/// connection health to detect network issues and free up resources.
pub async fn handle_ws(
	mut session: actix_ws::Session,
	mut msg_stream: actix_ws::MessageStream,
	channel: Arc<Sender<CardEvent>>,
) {
	info!("connected");

	let mut last_heartbeat = Instant::now();
	let mut interval = interval(HEARTBEAT_INTERVAL);

	let mut reciever = channel.subscribe();

	let reason = loop {
		// waits for either `msg_stream` to receive a message from the client, the broadcast channel
		// to send a message, or the heartbeat interval timer to tick, yielding the value of
		// whichever one is ready first
		select! {
			broadcast_msg = reciever.recv() => {
				let msg = match broadcast_msg {
					Ok(msg) => msg,
					Err(error::RecvError::Closed) => break None,
					Err(error::RecvError::Lagged(_)) => continue,
				};

				let res = session.text(Event::new(msg)).await;

				if let Err(err) = res {
					error!("{err}");
					break None;
				}
			}

			// heartbeat interval ticked
			_tick = interval.tick() => {
				// if no heartbeat ping/pong received recently, close the connection
				if Instant::now().duration_since(last_heartbeat) > CLIENT_TIMEOUT {
					info!(
						"client has not sent heartbeat in over {CLIENT_TIMEOUT:?}; disconnecting"
					);

					break None;
				}

				// send heartbeat ping
				let _ = session.ping(b"").await;
			},

			msg = msg_stream.next() => {
				let msg = match msg {
					// received message from WebSocket client
					Some(Ok(msg)) => msg,

					// client WebSocket stream error
					Some(Err(err)) => {
						error!("{err}");
						break None;
					}

					// client WebSocket stream ended
					None => break None
				};

				debug!("msg: {msg:?}");

				match msg {
					Message::Text(data) => {
						last_heartbeat = Instant::now();
						let event = ParsedEvent::parse(&data);
						info!(event = ?event, "received Event");
						let _ = channel.send(CardEvent::Updated);
						// drop client's text messages
					}

					Message::Binary(_) => {
						last_heartbeat = Instant::now();
						// drop client's binary messages
						warn!("received binary message; dropping");
					}

					Message::Close(reason) => {
						break reason;
					}

					Message::Ping(bytes) => {
						last_heartbeat = Instant::now();
						let _ = session.pong(&bytes).await;
					}

					Message::Pong(_) => {
						last_heartbeat = Instant::now();
					}

					Message::Continuation(_) => {
						warn!("no support for continuation frames");
					}

					// no-op; ignore
					Message::Nop => {}
				};
			}
		}
	};

	// attempt to close connection gracefully
	let _ = session.close(reason).await;

	info!("disconnected");
}
