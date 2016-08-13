// Copyleft (ↄ) meh. <meh@schizofreni.co> | http://meh.schizofreni.co
//
// This file is part of xenu.
//
// xenu is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// xenu is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with xenu.  If not, see <http://www.gnu.org/licenses/>.

extern crate clap;
use clap::{App, Arg};

extern crate xcb;
extern crate xcb_util as xcbu;

fn main() {
	let matches = App::new("kill")
		.version(env!("CARGO_PKG_VERSION"))
		.author("meh. <meh@schizofreni.co>")
		.arg(Arg::with_name("display")
			.short("d")
			.long("display")
			.takes_value(true)
			.help("The display to connect to."))
		.arg(Arg::with_name("frame")
			.short("f")
			.long("frame")
			.help("Do not ignore WM frames."))
		.arg(Arg::with_name("button")
			.short("b")
			.long("button")
			.takes_value(true)
			.help("The mouse button to use (default is 1)."))
		.arg(Arg::with_name("ID")
			.index(1)
			.help("The resource ID."))
		.get_matches();

	let (connection, screen) = xcb::Connection::connect(matches.value_of("display")).unwrap();
	let button               = matches.value_of("button").map(|b| b.parse().unwrap()).unwrap_or(1);
	let frame                = matches.value_of("frame").is_some();
	let id                   = matches.value_of("ID").map(|id| id.parse().unwrap());

	if let Some(mut window) = id.or_else(|| select(&connection, screen, button)) {
		if !frame {
			if let Some(client) = xcbu::misc::client_window(&connection, window) {
				window = client;
			}
		}

		xcb::kill_client(&connection, window);
		connection.flush();
	}
}

fn select(c: &xcb::Connection, screen: i32, button: u8) -> Option<u32> {
	let root   = c.get_setup().roots().nth(screen as usize).unwrap().root();
	let cursor = xcbu::cursor::create_font_cursor(c, xcbu::cursor::PIRATE);
	let status = xcb::grab_pointer(c, false, root, xcb::EVENT_MASK_BUTTON_RELEASE as u16,
		xcb::GRAB_MODE_ASYNC as u8, xcb::GRAB_MODE_ASYNC as u8, 0, cursor, xcb::CURRENT_TIME)
			.get_reply().unwrap().status();

	if status != xcb::GRAB_STATUS_SUCCESS as u8 {
		return None;
	}

	let mut selection = None;

	while let Some(event) = c.wait_for_event() {
		match event.response_type() {
			xcb::BUTTON_RELEASE => {
				let event: &xcb::ButtonReleaseEvent = xcb::cast_event(&event);
				let window = event.child();

				if event.detail() != button {
					break;
				}

				if window == xcb::WINDOW_NONE {
					continue;
				}

				selection = Some(window);
				break;
			}

			_ => ()
		}
	}

	xcb::ungrab_pointer(c, xcb::CURRENT_TIME);

	selection
}
