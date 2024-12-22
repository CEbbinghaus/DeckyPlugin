use std::marker::PhantomData;

use actix_web::web::Bytes;
use tracing::warn;

pub(crate) struct Event<'a, T: EventTrait<'a>>(T, PhantomData<&'a T>);

pub(crate) trait EventTrait<'a> {
	fn get_id(&self) -> Option<&'a str> {
		None
	}
	fn get_event(&self) -> Option<&'a str> {
		None
	}
	fn get_data(&self) -> Option<&'a str> {
		None
	}
}

impl<'a, T: EventTrait<'a>> Event<'a, T> {
	pub fn new(val: T) -> Self {
		Event(val, PhantomData)
	}
}

impl<'a, T: EventTrait<'a>> ToString for Event<'a, T> {
	fn to_string(&self) -> String {
		let mut output = "".to_string();

		if let Some(value) = self.0.get_id() {
			output += &format!("id: {}\n", value);
		}
		if let Some(value) = self.0.get_event() {
			output += &format!("event: {}\n", value);
		}
		if let Some(value) = self.0.get_data() {
			output += &format!("data: {}\n", value);
		}

		if !output.is_empty() {
			output += "\n";
		}

		output
	}
}

impl<'a, T: EventTrait<'a>> From<Event<'a, T>> for Bytes {
	fn from(val: Event<'a, T>) -> Self {
		Bytes::from(val.to_string())
	}
}

impl<'a, T: EventTrait<'a>> From<Event<'a, T>> for String {
	fn from(val: Event<'a, T>) -> Self {
		val.to_string()
	}
}

impl<'a, T: EventTrait<'a>> From<T> for Event<'a, T> {
	fn from(value: T) -> Self {
		Event(value, PhantomData)
	}
}
#[derive(Debug)]
pub(crate) struct EventBuilder<'a> {
	id: Option<&'a str>,
	event: Option<&'a str>,
	data: Option<&'a str>,
}

#[allow(dead_code)]
impl<'a> EventBuilder<'a> {
	pub fn new() -> Self {
		EventBuilder {
			id: None,
			event: None,
			data: None,
		}
	}
	pub fn with_id(mut self, id: &'a str) -> Self {
		self.id = Some(id);
		self
	}
	pub fn set_id(&mut self, id: &'a str) {
		self.id = Some(id);
	}
	pub fn with_event(mut self, event: &'a str) -> Self {
		self.event = Some(event);
		self
	}
	pub fn set_event(&mut self, event: &'a str) {
		self.event = Some(event);
	}
	pub fn with_data(mut self, data: &'a str) -> Self {
		self.data = Some(data);
		self
	}
	pub fn set_data(&mut self, data: &'a str) {
		self.data = Some(data);
	}
}

impl<'a> EventTrait<'a> for EventBuilder<'a> {
	fn get_data(&self) -> Option<&'a str> {
		self.data
	}
	fn get_event(&self) -> Option<&'a str> {
		self.event
	}
	fn get_id(&self) -> Option<&'a str> {
		self.id
	}
}

#[derive(Debug)]
pub struct ParsedEvent<'a>(EventBuilder<'a>);

impl<'a> ParsedEvent<'a> {
	pub fn parse(str: &'a str) -> ParsedEvent<'a> {
		let mut event = EventBuilder::new();

		for line in str.split("\n") {
			let (key, mut value) = match line.split_once(":") {
				None => {
					warn!(line = line, "Malformed Event detected");
					continue;
				}
				Some(key) => key,
			};

			if value.is_empty() {
				warn!(key = key, "Event had empty data");
				continue;
			}

			value = value.trim();

			match key {
				"id" => event.set_id(value),
				"event" => event.set_event(value),
				"data" => event.set_data(value),
				_ => {
					warn!(key = key, "Invalid key provided");
					continue;
				}
			};
		}

		ParsedEvent(event)
	}
}

impl<'a> EventTrait<'a> for ParsedEvent<'a> {
	fn get_id(&self) -> Option<&'a str> {
		self.0.get_id()
	}

	fn get_event(&self) -> Option<&'a str> {
		self.0.get_event()
	}

	fn get_data(&self) -> Option<&'a str> {
		self.0.get_data()
	}
}
