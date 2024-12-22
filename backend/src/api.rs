use crate::{
	cfg::CONFIG,
	ds::Store,
	dto::{CardEvent, Game, MicroSDCard},
	env::PACKAGE_VERSION,
	err::Error,
	event::Event,
	sdcard::{get_card_cid, is_card_inserted},
	ws::ws,
};
use actix_web::{
	delete, get,
	http::StatusCode,
	post,
	web::{self, Bytes},
	Either, HttpResponse, HttpResponseBuilder, Responder, Result,
};
use futures::StreamExt;
use serde::Deserialize;
use std::{ops::Deref, sync::Arc};
use tokio::sync::broadcast::Sender;
use tokio_stream::wrappers::BroadcastStream;
use tracing::{instrument, trace};

pub(crate) fn config(cfg: &mut web::ServiceConfig) {
	cfg //
		.service(ws)
		.service(health)
		.service(version)
		.service(listen)
		.service(save)
		.service(get_setting_by_name)
		.service(set_setting_by_name)
		.service(get_current_card)
		.service(get_current_card_id)
		.service(get_current_card_and_games)
		.service(get_games_on_current_card)
		.service(create_card)
		.service(delete_card)
		.service(update_cards)
		.service(list_cards)
		.service(get_card)
		.service(create_game)
		.service(create_games)
		.service(delete_game)
		.service(list_games)
		.service(get_game)
		.service(list_games_for_card)
		.service(list_cards_for_game)
		.service(list_cards_with_games)
		.service(create_link)
		.service(create_links)
		.service(delete_link)
		.service(delete_links);
}

#[get("/version")]
#[instrument]
pub(crate) async fn version() -> impl Responder {
	HttpResponse::Ok().body(PACKAGE_VERSION)
}

#[allow(clippy::async_yields_async)]
#[get("/health")]
#[instrument]
pub(crate) async fn health() -> impl Responder {
	trace!("HTTP GET /health");

	HttpResponse::Ok()
}

#[get("/listen")]
#[instrument]
pub(crate) async fn listen(sender: web::Data<Sender<CardEvent>>) -> Result<HttpResponse> {
	trace!("HTTP GET /listen");

	let event_stream = BroadcastStream::new(sender.subscribe()).map(|res| match res {
		Err(_) => Err(Error::from_str("Subscriber Closed")),
		Ok(value) => Ok(Event::new(value).into()),
	});
	Ok(HttpResponse::Ok()
		.content_type("text/event-stream")
		.streaming(event_stream))
}

#[get("/setting/{name}")]
#[instrument]
pub(crate) async fn get_setting_by_name(name: web::Path<String>) -> Result<impl Responder> {
	trace!("HTTP GET /setting/{name}");

	let result = CONFIG.read().await.get_property(&name)?;
	Ok(result)
}

#[post("/setting/{name}")]
#[instrument]
pub(crate) async fn set_setting_by_name(
	body: Bytes,
	name: web::Path<String>,
) -> Result<impl Responder> {
	trace!("HTTP POST /setting/{name}");

	let value = String::from_utf8(body.to_vec())
		.map_err(|_| Error::from_str("Unable to decode body as utf8"))?;
	CONFIG.write().await.set_property(&name, &value)?;
	Ok(HttpResponse::Ok())
}

#[get("/list")]
#[instrument(skip(datastore))]
pub(crate) async fn list_cards_with_games(datastore: web::Data<Arc<Store>>) -> impl Responder {
	trace!("HTTP GET /list");

	web::Json(datastore.list_cards_with_games())
}

#[get("/list/games/{card_id}")]
#[instrument(skip(datastore))]
pub(crate) async fn list_games_for_card(
	card_id: web::Path<String>,
	datastore: web::Data<Arc<Store>>,
) -> Result<impl Responder> {
	trace!("HTTP GET /list/games/{card_id}");

	match datastore.get_games_on_card(&card_id) {
		Ok(value) => Ok(web::Json(value)),
		Err(err) => Err(actix_web::Error::from(err)),
	}
}

#[get("/list/cards/{game_id}")]
#[instrument(skip(datastore))]
pub(crate) async fn list_cards_for_game(
	game_id: web::Path<String>,
	datastore: web::Data<Arc<Store>>,
) -> Result<impl Responder> {
	trace!("HTTP GET /list/cards/{game_id}");

	match datastore.get_cards_for_game(&game_id) {
		Ok(value) => Ok(web::Json(value)),
		Err(err) => Err(actix_web::Error::from(err)),
	}
}

#[get("/current")]
#[instrument(skip(datastore))]
pub(crate) async fn get_current_card_and_games(
	datastore: web::Data<Arc<Store>>,
) -> Result<Either<impl Responder, impl Responder>> {
	trace!("HTTP GET /current");

	if !is_card_inserted() {
		return Ok(Either::Right(
			HttpResponseBuilder::new(StatusCode::NO_CONTENT)
				.reason("No Card inserted")
				.finish(),
		));
	}

	match get_card_cid() {
		Some(uid) => Ok(Either::Left(web::Json(datastore.get_card_and_games(&uid)?))),
		None => Ok(Either::Right(
			HttpResponseBuilder::new(StatusCode::NO_CONTENT)
				.reason("Card Id could not be resolved")
				.finish(),
		)),
	}
}

#[get("/current/card")]
#[instrument(skip(datastore))]
pub(crate) async fn get_current_card(datastore: web::Data<Arc<Store>>) -> Result<impl Responder> {
	trace!("HTTP GET /current/card");

	if !is_card_inserted() {
		return Err(Error::from_str("No card is inserted").into());
	}

	let uid = get_card_cid().ok_or(Error::from_str("Unable to evaluate Card Id"))?;

	Ok(web::Json(datastore.get_card(&uid)?))
}

#[get("/current/id")]
#[instrument]
pub(crate) async fn get_current_card_id() -> Result<impl Responder> {
	trace!("HTTP GET /current/id");

	if !is_card_inserted() {
		return Err(Error::from_str("No card is inserted").into());
	}

	Ok(get_card_cid().ok_or(Error::from_str("Unable to evaluate Card Id"))?)
}

#[get("/current/games")]
#[instrument(skip(datastore))]
pub(crate) async fn get_games_on_current_card(
	datastore: web::Data<Arc<Store>>,
) -> Result<impl Responder> {
	trace!("HTTP GET /current/games");

	if !is_card_inserted() {
		return Err(Error::from_str("No card is inserted").into());
	}

	let uid = get_card_cid().ok_or(Error::from_str("Unable to evaluate Card Id"))?;

	match datastore.get_games_on_card(&uid) {
		Ok(value) => Ok(web::Json(value)),
		Err(err) => Err(actix_web::Error::from(err)),
	}
}

#[post("/card/{id}")]
#[instrument(skip(datastore))]
pub(crate) async fn create_card(
	id: web::Path<String>,
	body: web::Json<MicroSDCard>,
	datastore: web::Data<Arc<Store>>,
	sender: web::Data<Sender<CardEvent>>,
) -> Result<impl Responder> {
	trace!("HTTP POST /card/{id}");

	if *id != body.uid {
		return Err(Error::from_str("uid did not match id provided").into());
	}

	match datastore.contains_element(&id) {
		// Merge the records allowing us to update all properties
		true => datastore.update_card(&id, move |existing_card| {
			existing_card.merge(body.deref())?;
			Ok(())
		})?,
		// Insert a new card if it doesn't exist
		false => datastore.add_card(id.into_inner(), body.into_inner()),
	}

	_ = sender.send(CardEvent::Updated);
	Ok(HttpResponse::Ok())
}

#[delete("/card/{id}")]
#[instrument(skip(datastore))]
pub(crate) async fn delete_card(
	id: web::Path<String>,
	datastore: web::Data<Arc<Store>>,
	sender: web::Data<Sender<CardEvent>>,
) -> Result<impl Responder> {
	trace!("HTTP DELETE /card/{id}");
	datastore.remove_element(&id)?;

	trace!("Sending Updated event");
	_ = sender.send(CardEvent::Updated);
	Ok(HttpResponse::Ok())
}

#[get("/card/{id}")]
#[instrument(skip(datastore))]
pub(crate) async fn get_card(
	id: web::Path<String>,
	datastore: web::Data<Arc<Store>>,
) -> Result<impl Responder> {
	trace!("HTTP GET /card/{id}");
	Ok(web::Json(datastore.get_card(&id)?))
}

#[post("/cards")]
#[instrument(skip(datastore))]
pub(crate) async fn update_cards(
	body: web::Json<Vec<MicroSDCard>>,
	datastore: web::Data<Arc<Store>>,
	sender: web::Data<Sender<CardEvent>>,
) -> Result<impl Responder> {
	trace!("HTTP POST /cards");

	for card in body.iter() {
		let card = card.to_owned();

		match datastore.contains_element(&card.uid) {
			// Merge the records allowing us to update all properties
			true => datastore.update_card(&card.uid.clone(), move |existing_card| {
				existing_card.merge(&card)?;
				Ok(())
			})?,
			// Insert a new card if it doesn't exist
			false => datastore.add_card(card.uid.clone(), card),
		}
	}

	trace!("Sending Updated event");
	_ = sender.send(CardEvent::Updated);
	Ok(HttpResponse::Ok())
}

#[get("/cards")]
#[instrument(skip(datastore))]
pub(crate) async fn list_cards(datastore: web::Data<Arc<Store>>) -> impl Responder {
	trace!("HTTP GET /cards");
	web::Json(datastore.list_cards())
}

#[post("/game/{id}")]
#[instrument(skip(datastore))]
pub(crate) async fn create_game(
	id: web::Path<String>,
	body: web::Json<Game>,
	datastore: web::Data<Arc<Store>>,
) -> Result<impl Responder> {
	trace!("HTTP POST /game/{id}");

	if *id != body.uid {
		return Err(Error::from_str("uid did not match id provided").into());
	}

	let mut game = body.to_owned();

	if !cfg!(debug_assertions) {
		game.is_steam = false;
	}

	datastore.add_game(id.into_inner(), game);
	Ok(HttpResponse::Ok())
}

#[delete("/game/{id}")]
#[instrument(skip(datastore))]
pub(crate) async fn delete_game(
	id: web::Path<String>,
	datastore: web::Data<Arc<Store>>,
) -> Result<impl Responder> {
	trace!("HTTP DELETE /game/{id}");
	datastore.remove_element(&id)?;

	Ok(HttpResponse::Ok())
}

#[get("/game/{id}")]
#[instrument(skip(datastore))]
pub(crate) async fn get_game(
	id: web::Path<String>,
	datastore: web::Data<Arc<Store>>,
) -> Result<impl Responder> {
	trace!("HTTP GET /game/{id}");
	Ok(web::Json(datastore.get_game(&id)?))
}

#[get("/games")]
#[instrument(skip(datastore))]
pub(crate) async fn list_games(datastore: web::Data<Arc<Store>>) -> impl Responder {
	trace!("HTTP GET /games");
	web::Json(datastore.list_games())
}

#[allow(clippy::async_yields_async)]
#[post("/games")]
#[instrument(skip(datastore))]
pub(crate) async fn create_games(
	body: web::Json<Vec<Game>>,
	datastore: web::Data<Arc<Store>>,
) -> impl Responder {
	trace!("HTTP POST /games");

	for game in body.iter() {
		let mut game = game.to_owned();

		if !cfg!(debug_assertions) {
			game.is_steam = false;
		}

		datastore.add_game(game.uid.clone(), game);
	}

	HttpResponse::Ok()
}

#[derive(Deserialize, Debug)]
pub struct LinkBody {
	card_id: String,
	game_id: String,
}

#[derive(Deserialize, Debug)]
pub struct ManyLinkBody {
	card_id: String,
	game_ids: Vec<String>,
}

#[post("/link")]
#[instrument(skip(datastore))]
pub(crate) async fn create_link(
	body: web::Json<LinkBody>,
	datastore: web::Data<Arc<Store>>,
	sender: web::Data<Sender<CardEvent>>,
) -> Result<impl Responder> {
	trace!("HTTP POST /link");

	datastore.link(&body.game_id, &body.card_id)?;

	trace!("Sending Updated event");
	_ = sender.send(CardEvent::Updated);
	Ok(HttpResponse::Ok())
}

#[post("/linkmany")]
#[instrument(skip(datastore))]
pub(crate) async fn create_links(
	body: web::Json<ManyLinkBody>,
	datastore: web::Data<Arc<Store>>,
	sender: web::Data<Sender<CardEvent>>,
) -> Result<impl Responder> {
	trace!("HTTP POST /linkmany");

	let data = body.into_inner();
	for game_id in data.game_ids.iter() {
		datastore.link(game_id, &data.card_id)?;
	}

	trace!("Sending Updated event");
	_ = sender.send(CardEvent::Updated);
	Ok(HttpResponse::Ok())
}

#[post("/unlink")]
#[instrument(skip(datastore))]
pub(crate) async fn delete_link(
	body: web::Json<LinkBody>,
	datastore: web::Data<Arc<Store>>,
	sender: web::Data<Sender<CardEvent>>,
) -> Result<impl Responder> {
	trace!("HTTP POST /unlink");

	datastore.unlink(&body.game_id, &body.card_id)?;

	trace!("Sending Updated event");
	_ = sender.send(CardEvent::Updated);
	Ok(HttpResponse::Ok())
}

#[post("/unlinkmany")]
#[instrument(skip(datastore))]
pub(crate) async fn delete_links(
	body: web::Json<ManyLinkBody>,
	datastore: web::Data<Arc<Store>>,
	sender: web::Data<Sender<CardEvent>>,
) -> Result<impl Responder> {
	trace!("HTTP POST /unlinkmany");

	let data = body.into_inner();
	for game_id in data.game_ids.iter() {
		datastore.unlink(game_id, &data.card_id)?;
	}

	trace!("Sending Updated event");
	_ = sender.send(CardEvent::Updated);
	Ok(HttpResponse::Ok())
}

#[post("/save")]
#[instrument(skip(datastore))]
pub(crate) async fn save(datastore: web::Data<Arc<Store>>) -> Result<impl Responder> {
	trace!("HTTP POST /save");

	datastore.write_to_file()?;

	Ok(HttpResponse::Ok())
}
