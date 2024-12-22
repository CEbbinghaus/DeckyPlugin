use crate::{err::Error, event::EventTrait};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub enum CardEvent {
	Inserted,
	Removed,
	Updated,
}

impl<'a> EventTrait<'a> for CardEvent {
	fn get_event(&self) -> Option<&'a str> {
		Some(match self {
			CardEvent::Inserted => "insert",
			CardEvent::Removed => "remove",
			CardEvent::Updated => "update",
		})
	}
	// fn get_data(&self) -> Option<&'static str> {
	// 	match self {
	// 		CardEvent::Inserted => None,
	// 		CardEvent::Removed => None,
	// 		CardEvent::Updated => Some("Hello, World!"),
	// 	}
	// }
}

fn default_true() -> bool {
	true
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MicroSDCard {
	pub uid: String,
	pub libid: String,

	#[serde(default)]
	pub mount: Option<String>,

	pub name: String,
	#[serde(default)]
	pub position: u32,
	#[serde(default)]
	pub hidden: bool,
}

impl MicroSDCard {
	pub fn merge(&mut self, other: &MicroSDCard) -> Result<(), Error> {
		if self.uid != other.uid {
			return Error::new_res("uid's did not match");
		}

		if self.libid != other.libid {
			return Error::new_res("libid's did not match");
		}

		self.name = other.name.clone();
		self.position = other.position;
		self.hidden = other.hidden;

		Ok(())
	}
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Game {
	pub uid: String,
	pub name: String,
	pub size: u64,

	#[serde(default = "default_true")]
	pub is_steam: bool,
}
