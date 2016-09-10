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

extern crate image;
use image::GenericImage;

extern crate xcb;
extern crate xcb_util as xcbu;

const ATOMS: &'static [&'static str] = &[
	"_XROOTPMAP_ID",      // hsetroot
	"_XSETROOT_ID",       // xsetroot
	"_XENU_BACKGROUND_ID" // meh
];

fn main() {
	let matches = App::new("xenu-background")
		.version(env!("CARGO_PKG_VERSION"))
		.about("Xenu is artistic, and will paint a background on your screen.")
		.arg(Arg::with_name("display")
			.short("d")
			.long("display")
			.takes_value(true)
			.help("The display to connect to."))
		.arg(Arg::with_name("PATH")
			.index(1)
			.required(true)
			.help("Path to the image to use."))
		.get_matches();

	let path = matches.value_of("PATH").unwrap();

	let (connection, screen) = xcb::Connection::connect(matches.value_of("display")).unwrap();
	let setup  = connection.get_setup();
	let screen = setup.roots().nth(screen as usize).unwrap();

	// Open the given path and resize it to fit the screen.
	let mut source = image::open(path).expect("failed to open image");

	if source.width() != screen.width_in_pixels() as u32 || source.height() != screen.height_in_pixels() as u32 {
		source = source.resize(screen.width_in_pixels() as u32, screen.height_in_pixels() as u32,
			image::FilterType::Lanczos3);
	}

	clean(&connection, &screen);
	set(&connection, &screen, source.to_rgb());

	connection.flush();
}

/// Set the background for the screen from the given image.
fn set<T: GenericImage<Pixel = image::Rgb<u8>>>(c: &xcb::Connection, screen: &xcb::Screen, source: T) {
	// Create a shared image to store the pixels.
	let mut image = xcbu::image::shm::create(c, screen.root_depth(),
		screen.width_in_pixels(), screen.height_in_pixels())
			.expect("could not create image");

	// Fill in the pixels from the source image.
	for (x, y, px) in source.pixels() {
		image.put(x, y,
			((px[0] as u32) << 16) |
			((px[1] as u32) << 8)  |
			((px[2] as u32) << 0));
	}

	// Create the pixmap that will store the background.
	let pixmap = c.generate_id();
	xcb::create_pixmap_checked(c, screen.root_depth(), pixmap, screen.root(),
		screen.width_in_pixels(), screen.height_in_pixels())
			.request_check().expect("could not create pixmap");

	// Create the useless graphics context.
	let context = c.generate_id();
	xcb::create_gc_checked(c, context, screen.root(), &[])
		.request_check().expect("could not create context");

	// Push the shared image into the pixmap.
	xcbu::image::shm::put(c, pixmap, context, &image, 0, 0, 0, 0,
		screen.width_in_pixels(), screen.height_in_pixels(), false)
			.expect("could not draw image");

	// Set the created pixmap as root background.
	xcb::change_window_attributes_checked(c, screen.root(), &[
		(xcb::CW_BACK_PIXMAP, pixmap)])
			.request_check().expect("could not change attributes");

	// Clear the root window so the background image is refreshed.
	xcb::clear_area_checked(c, true, screen.root(), 0, 0,
		screen.width_in_pixels(), screen.height_in_pixels())
			.request_check().expect("could not clear root window");

	// Set background ID.
	for atom in ATOMS {
		xcb::change_property(c, xcb::PROP_MODE_REPLACE as u8, screen.root(),
			xcb::intern_atom(c, false, atom).get_reply().expect("failed to intern atom").atom(),
			xcb::ATOM_PIXMAP, 32, &[pixmap]);
	}
}

/// Clean up any previously created resources.
fn clean(c: &xcb::Connection, screen: &xcb::Screen) {
	// Fetch any previously set pixmap IDs.
	let ids = ATOMS.iter().map(|atom| {
		let reply = xcb::get_property(c, false, screen.root(),
			xcb::intern_atom(c, false, atom).get_reply().expect("failed to intern atom").atom(),
			xcb::ATOM_PIXMAP, 0, 1).get_reply();

		match reply {
			Ok(ref reply) if reply.type_() == xcb::ATOM_PIXMAP =>
				Some(reply.value()[0]),

			_ =>
				None
		}
	}).collect::<Vec<Option<xcb::Pixmap>>>();

	// If all are set and equal, kill it with fire.
	if ids.iter().all(Option::is_some) && ids.iter().all(|id| id == ids.first().unwrap()) {
		xcb::kill_client(c, ids.first().unwrap().unwrap());
	}

	// Kill any temporary resources.
	xcb::kill_client(c, xcb::KILL_ALL_TEMPORARY);
	xcb::set_close_down_mode(c, xcb::CLOSE_DOWN_RETAIN_TEMPORARY as u8);
}
