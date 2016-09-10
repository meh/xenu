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

use std::io::{self, Read};

extern crate clap;
use clap::{App, Arg};

extern crate image;
use image::{GenericImage, DynamicImage};

extern crate palette;
use palette::{Rgb, Gradient};
use palette::pixel::Srgb;

#[macro_use]
extern crate lazy_static;
extern crate regex;
use regex::Regex;

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
		.arg(Arg::with_name("solid")
			.short("s")
			.long("solid")
			.takes_value(true)
			.required_unless("gradient")
			.required_unless("PATH")
			.help("Make a solid color background."))
		.arg(Arg::with_name("gradient")
			.short("g")
			.long("gradient")
			.multiple(true)
			.takes_value(true)
			.required_unless("PATH")
			.required_unless("solid")
			.help("Define a gradient as background."))
		.arg(Arg::with_name("horizontal")
			.short("-H")
			.long("horizontal")
			.requires("gradient")
			.help("Make an horizontal gradient (this is the default)."))
		.arg(Arg::with_name("vertical")
			.short("V")
			.long("vertical")
			.requires("gradient")
			.help("Make a vertical gradient."))
		.arg(Arg::with_name("PATH")
			.index(1)
			.required_unless("solid")
			.required_unless("gradient")
			.help("Path to the image to use."))
		.arg(Arg::with_name("flip")
			.short("f")
			.long("flip")
			.multiple(true)
			.takes_value(true)
			.help("Flip the loaded image (takes `vertical` and `horizontal`."))
		.get_matches();

	let (connection, screen) = xcb::Connection::connect(matches.value_of("display")).unwrap();
	let setup  = connection.get_setup();
	let screen = setup.roots().nth(screen as usize).unwrap();

	let mut source = if let Some(path) = matches.value_of("PATH") {
		if path != "-" {
			image::open(path)
		}
		else {
			let mut buffer = Vec::new();
			io::stdin().read_to_end(&mut buffer).unwrap();

			image::load_from_memory(&buffer)
		}.expect("failed to open image")
	}
	else if let Some(colors) = matches.values_of("gradient") {
		let mut source   = DynamicImage::new_rgb8(screen.width_in_pixels() as u32, screen.height_in_pixels() as u32);
		let     gradient = Gradient::new(colors.map(|c| color(c).unwrap()));

		if matches.is_present("vertical") {
			for (y, color) in (0 .. source.height()).zip(gradient.take(source.height() as usize)) {
				for x in 0 .. source.width() {
					source.as_mut_rgb8().unwrap().put_pixel(x, y, srgb(color));
				}
			}
		}
		else {
			for (x, color) in (0 .. source.width()).zip(gradient.take(source.width() as usize)) {
				for y in 0 .. source.height() {
					source.as_mut_rgb8().unwrap().put_pixel(x, y, srgb(color));
				}
			}
		}

		source
	}
	else if let Some(solid) = matches.value_of("solid") {
		let mut source = DynamicImage::new_rgb8(screen.width_in_pixels() as u32, screen.height_in_pixels() as u32);
		let     color  = color(solid).unwrap();

		for px in source.as_mut_rgb8().unwrap().pixels_mut() {
			*px = srgb(color);
		}

		source
	}
	else {
		unreachable!();
	};

	// Flip the image as requested.
	if let Some(flip) = matches.values_of("flip") {
		for side in flip {
			match side.to_lowercase().as_ref() {
				"vertically" | "vertical" | "vert" | "v" =>
					source = source.flipv(),

				"horizontally" | "horizontal" | "horiz" | "h" =>
					source = source.fliph(),

				_ => ()
			}
		}
	}

	// If the source image isn't the right size, resize it.
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

lazy_static! {
	static ref HEX_RGB: Regex = Regex::new(r"#([:xdigit:]{2})([:xdigit:]{2})([:xdigit:]{2})").unwrap();
}

fn color(value: &str) -> Option<Rgb> {
	HEX_RGB.captures(value.as_ref()).map(|captures| {
		Rgb::new(
			u8::from_str_radix(captures.at(1).unwrap_or("0"), 16).unwrap_or(0) as f32 / 255.0,
			u8::from_str_radix(captures.at(2).unwrap_or("0"), 16).unwrap_or(0) as f32 / 255.0,
			u8::from_str_radix(captures.at(3).unwrap_or("0"), 16).unwrap_or(0) as f32 / 255.0,
		)
	})
}

fn srgb(value: Rgb<f32>) -> image::Rgb<u8> {
	let pixel = Srgb::from(value);

	image::Rgb { data: [
		(pixel.red * 255.0) as u8,
		(pixel.green * 255.0) as u8,
		(pixel.blue * 255.0) as u8,
	] }
}
